// Tes koneksi nyata ke server WhatsApp memakai device yang diimpor dari
// creds.json onigis. TIDAK mengirim pesan, TIDAK pairing — hanya connect,
// ukur RSS, lalu disconnect bersih.
use anyhow::{Context, Result};
use whatsapp_rust::prelude::*;


use crate::creds_map;
use crate::rss;

pub async fn run(creds_path: &str, hold_secs: u64) -> Result<()> {
    let raw = std::fs::read_to_string(creds_path)
        .map_err(|e| anyhow::anyhow!("baca {creds_path}: {e}"))?;
    let (_m, c) = creds_map::inspect_creds(&raw)?;
    let device = build_device_pub(&c).await?;
    println!(
        "{}",
        serde_json::json!({"phase":"device_built","rss_kb":rss::current_kb()})
    );

    let db = std::env::temp_dir().join("oktz-spike-connect.db");
    let _ = std::fs::remove_file(&db);
    let store = SqliteStore::new(db.to_str().unwrap()).await?;
    store.save_device_data_for_device(1, &device).await?;
    println!(
        "{}",
        serde_json::json!({"phase":"device_saved","rss_kb":rss::current_kb()})
    );

    let connected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let logged_out = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let c1 = connected.clone();
    let c2 = logged_out.clone();

    let msgs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let m1 = msgs.clone();


    let bot = Bot::builder()
        .with_backend(store)
        .on_message(move |ctx| {
            let m1 = m1.clone();
            async move {
                let n = m1.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                let sender = ctx.info.source.sender.to_string();
                let text = ctx.message.text_content().unwrap_or("<bukan teks>");
                println!(
                    "{}",
                    serde_json::json!({
                        "phase": "MESSAGE",
                        "seq": n,
                        "sender": sender,

                        "text": text,
                        "rss_kb": rss::current_kb(),
                    })
                );
            }
        })
        .on_connected(move |_client| {
            let c1 = c1.clone();
            async move {
                println!(
                    "{}",
                    serde_json::json!({"phase":"CONNECTED","rss_kb":rss::current_kb()})
                );
                c1.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .on_logged_out(move |info| {
            let c2 = c2.clone();
            async move {
                println!(
                    "{}",
                    serde_json::json!({"phase":"LOGGED_OUT","reason":format!("{info:?}")})
                );
                c2.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .build()
        .await?;

    let mut handle = bot.spawn();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs);
    let mut last_rss = 0u64;
    while tokio::time::Instant::now() < deadline {
        if logged_out.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let r = rss::current_kb();
        if r != last_rss {
            last_rss = r;
            println!(
                "{}",
                serde_json::json!({"phase":"waiting","rss_kb":r})
            );
        }
    }

    let ok = connected.load(std::sync::atomic::Ordering::SeqCst);
    let out = std::sync::atomic::Ordering::SeqCst;
    println!(
        "{}",
        serde_json::json!({
            "phase":"result",
            "connected": ok,
            "logged_out": logged_out.load(out),
            "messages": msgs.load(std::sync::atomic::Ordering::SeqCst),
            "rss_kb": rss::current_kb(),
        })
    );
    handle.shutdown().await;
    Ok(())
}

fn jid(s: &str) -> Result<Jid> {
    s.parse::<Jid>().map_err(|e| anyhow::anyhow!("JID {s}: {e}"))
}

pub async fn build_device_pub(c: &creds_map::Creds) -> Result<whatsapp_rust::wacore::store::Device> {
    use wacore_libsignal::core::curve::{KeyPair, PrivateKey, PublicKey};
    // onigis/Baileys menyimpan public key Curve25519 sebagai 32 byte POLOS.
// libsignal/wacore memakai bentuk kanonik 33 byte = 0x05 (KeyType::Djb) + 32.
// Kalau sudah 33 byte, biarkan apa adanya.
const DJB_PREFIX: u8 = 0x05;

fn kp(j: &creds_map::KeyPairJson, label: &str) -> Result<KeyPair> {
        let mut pb = creds_map::b64(&j.public)?;
        let vb = creds_map::b64(&j.private)?;
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
    let mut d = whatsapp_rust::wacore::store::Device::default();

    d.pn = Some(jid(&me.id)?);
    d.lid = me.lid.as_deref().map(jid).transpose()?;
    d.registration_id = c.registration_id;
    d.noise_key = kp(&c.noise_key, "noiseKey")?;
    d.identity_key = kp(&c.signed_identity_key, "signedIdentityKey")?;
    d.signed_pre_key = kp(&c.signed_pre_key.key_pair, "signedPreKey.keyPair")?;
    d.signed_pre_key_id = c.signed_pre_key.key_id;
    d.signed_pre_key_signature = arr(creds_map::b64(&c.signed_pre_key.signature)?, "signature")?;
    d.adv_secret_key = arr(creds_map::b64(&c.adv_secret_key)?, "advSecretKey")?;
    d.push_name = me.name.clone();
    d.next_pre_key_id = c.next_pre_key_id.unwrap_or(0);
    d.first_unupload_pre_key_id = c.first_unuploaded_pre_key_id.unwrap_or(0);
    d.props_hash = c.last_prop_hash.clone();
    if let Some(ri) = c.routing_info.as_ref().and_then(|r| r.data.as_ref()) {
        d.edge_routing_info = Some(creds_map::b64(ri)?);
    }

    // ADVSignedDeviceIdentity — tanpa ini PDO/retry receipt dan decrypt pesan
    // grup yang gagal decrypt akan bermasalah.
    if let Some(a) = c.account.as_ref() {
        let nb = |s: &str| -> Result<Option<Vec<u8>>> {
            if s.is_empty() { Ok(None) } else { Ok(Some(creds_map::b64(s)?)) }
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