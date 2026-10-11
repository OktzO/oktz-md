use anyhow::{Context, Result};
use whatsapp_rust::prelude::*;

use crate::creds;

fn jid(s: &str) -> Result<Jid> {
    s.parse::<Jid>().map_err(|e| anyhow::anyhow!("JID {s}: {e}"))
}

pub async fn build_device(c: &creds::Creds) -> Result<whatsapp_rust::wacore::store::Device> {
    use wacore_libsignal::core::curve::{KeyPair, PrivateKey, PublicKey};
    // onigis/Baileys menyimpan public key Curve25519 sebagai 32 byte POLOS.
// libsignal/wacore memakai bentuk kanonik 33 byte = 0x05 (KeyType::Djb) + 32.
// Kalau sudah 33 byte, biarkan apa adanya.
const DJB_PREFIX: u8 = 0x05;

fn kp(j: &creds::KeyPairJson, label: &str) -> Result<KeyPair> {
        let mut pb = creds::b64(&j.public)?;
        let vb = creds::b64(&j.private)?;
        if pb.len() == 32 {
            pb.insert(0, DJB_PREFIX);
        }
        Ok(KeyPair {
            public_key: PublicKey::try_from(&pb[..])
                .map_err(|e| anyhow::anyhow!("{label}.public (len={} first={:#x}): {e}", pb.len(), pb.first().unwrap_or(&0)))?,
            private_key: PrivateKey::try_from(&vb[..])
                .map_err(|e| anyhow::anyhow!("{label}.private (len={} first={:#x}): {e}", vb.len(), vb.first().unwrap_or(&0)))?,
        })
    }
    fn arr<const N: usize>(b: Vec<u8>, what: &str) -> Result<[u8; N]> {
        b.try_into()
            .map_err(|v: Vec<u8>| anyhow::anyhow!("{what} harus {N} byte, dapat {}", v.len()))
    }

    let me = c.me.as_ref().context("creds tanpa `me`")?;
    let mut d = whatsapp_rust::wacore::store::Device {
        pn: Some(jid(&me.id)?),
        lid: me.lid.as_deref().map(jid).transpose()?,
        registration_id: c.registration_id,
        noise_key: kp(&c.noise_key, "noiseKey")?,
        identity_key: kp(&c.signed_identity_key, "signedIdentityKey")?,
        signed_pre_key: kp(&c.signed_pre_key.key_pair, "signedPreKey.keyPair")?,
        signed_pre_key_id: c.signed_pre_key.key_id,
        signed_pre_key_signature: arr(creds::b64(&c.signed_pre_key.signature)?, "signature")?,
        adv_secret_key: arr(creds::b64(&c.adv_secret_key)?, "advSecretKey")?,
        push_name: me.name.clone(),
        next_pre_key_id: c.next_pre_key_id.unwrap_or(0),
        first_unupload_pre_key_id: c.first_unuploaded_pre_key_id.unwrap_or(0),
        props_hash: c.last_prop_hash.clone(),
        ..Default::default()
    };
    if let Some(ri) = c.routing_info.as_ref().and_then(|r| r.data.as_ref()) {
        d.edge_routing_info = Some(creds::b64(ri)?);
    }

    // ADVSignedDeviceIdentity — tanpa ini PDO/retry receipt dan decrypt pesan
    // grup yang gagal decrypt akan bermasalah.
    if let Some(a) = c.account.as_ref() {
        let nb = |s: &str| -> Result<Option<Vec<u8>>> {
            if s.is_empty() { Ok(None) } else { Ok(Some(creds::b64(s)?)) }
        };
        d.account = Some(std::sync::Arc::new(whatsapp_rust::waproto::whatsapp::ADVSignedDeviceIdentity {
            details: nb(a.details.as_deref().unwrap_or(""))?,
            account_signature_key: nb(a.account_signature_key.as_deref().unwrap_or(""))?,
            account_signature: match &a.account_signature {
                Some(s) => nb(s)?,
                None => None,
            },
            device_signature: match &a.device_signature {
                Some(s) => nb(s)?,
                None => None,
            },
        }));
    }
    Ok(d)
}
/// Buka store sqlite, simpan device, muat sesi onigis, lalu siap connect.
pub async fn prepare_store(
    creds_path: &str,
    db_path: &str,
    session_fixture: Option<&str>,
) -> Result<SqliteStore> {
    let raw = std::fs::read_to_string(creds_path)
        .with_context(|| format!("baca {creds_path}"))?;
    let (_m, c) = crate::creds::inspect_creds(&raw)?;

    let db = std::path::PathBuf::from(db_path);
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&db);
    let store = SqliteStore::new(db.to_str().context("path tidak utf8")?).await?;

    let device = build_device(&c).await?;
    store.save_device_data_for_device(1, &device).await?;

    if let Some(fx) = session_fixture {
        let mut ident = crate::creds::b64(&c.signed_identity_key.public)?;
        if ident.len() == 32 {
            ident.insert(0, 0x05);
        }
        let report = crate::load::load_into_store(&store, fx, &ident, c.registration_id).await?;
        tracing::info!(
            loaded = report.sessions_loaded,
            failed = report.sessions_failed,
            "sesi ratchet dimuat"
        );
    }
    Ok(store)
}

/// Boot bot dan pump handle; `on_ready` dipanggil setelah connect.
pub async fn connect_device<F, Fut>(
    store: SqliteStore,
    on_ready: F,
) -> Result<BotHandle>
where
    F: Fn(std::sync::Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let bot = Bot::builder().with_backend(store).on_connected(on_ready).build().await?;
    Ok(bot.spawn())
}
