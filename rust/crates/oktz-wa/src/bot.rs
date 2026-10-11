use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use whatsapp_rust::prelude::*;

use crate::creds;
use crate::pairing_run::{self, FalseLogoutGuard};

/// Env yang memberi tahu jalur produksi di mana `creds.json` berada.
///
/// [`connect_device`] tidak menerima creds (signedanya dijaga supaya tidak
/// pecah), dan tanpa snapshot guard tidak bisa membedakan "sesi sah yang
/// dapat 401 palsu" dari "belum pernah ter-link". Tanpa env ini guard
/// menerima `Value::Null` → `pairing::is_authenticated` false → arahnya
/// selalu "percaya logout". Itu konservatif (tidak pernah menahan logout
/// asli) tapi berarti grace window tidak aktif; caller yang punya file creds
/// sebaiknya memakai [`connect_device_guarded`] langsung.
pub const CREDS_ENV: &str = "OKTZ_CREDS";

fn jid(s: &str) -> Result<Jid> {
    s.parse::<Jid>()
        .map_err(|e| anyhow::anyhow!("JID {s}: {e}"))
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
            public_key: PublicKey::try_from(&pb[..]).map_err(|e| {
                anyhow::anyhow!(
                    "{label}.public (len={} first={:#x}): {e}",
                    pb.len(),
                    pb.first().unwrap_or(&0)
                )
            })?,
            private_key: PrivateKey::try_from(&vb[..]).map_err(|e| {
                anyhow::anyhow!(
                    "{label}.private (len={} first={:#x}): {e}",
                    vb.len(),
                    vb.first().unwrap_or(&0)
                )
            })?,
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
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(creds::b64(s)?))
            }
        };
        d.account = Some(std::sync::Arc::new(
            whatsapp_rust::waproto::whatsapp::ADVSignedDeviceIdentity {
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
            },
        ));
    }
    Ok(d)
}
/// Buka store sqlite, simpan device, muat sesi onigis, lalu siap connect.
pub async fn prepare_store(
    creds_path: &str,
    db_path: &str,
    session_fixture: Option<&str>,
) -> Result<SqliteStore> {
    let raw = std::fs::read_to_string(creds_path).with_context(|| format!("baca {creds_path}"))?;
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
            write_failed = report.write_failed,
            "sesi ratchet dimuat (hanya yang benar-benar tersimpan)"
        );
    }
    Ok(store)
}

/// Path creds untuk guard: argumen eksplisit menang atas env.
///
/// Murni supaya bisa diuji tanpa menyentuh filesystem maupun `std::env`
/// (yang process-wide, jadi bisa saling menimpa antar-test).
pub fn resolve_creds_path(explicit: Option<&str>, env: Option<&str>) -> Option<PathBuf> {
    [explicit, env]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Snapshot creds untuk [`FalseLogoutGuard`].
///
/// Kegagalan baca TIDAK boleh menggagalkan sesi: guard hanya melindungi
/// keputusan logout, dan creds yang tak terbaca membuat
/// `pairing::is_authenticated` false, yang arahnya selalu "percaya logout" —
/// lebih konservatif daripada diam-diam menganggap sesi sah.
///
/// # Kenapa tidak ada write-back (sudah diperiksa, bukan asumsi)
///
/// Creds di sini read-only untuk runtime. Tidak ada helper yang bisa menulis
/// balik, dan mengarang satu akan lebih buruk daripada tidak menulis:
///
/// * whatsapp-rust 0.7.0 tidak mengenal konsep creds/authState sama sekali
///   (`grep -ri "authstate\|creds\.json" src/` di crate itu kosong). Device
///   state yang dirotasi library disimpan lewat `PersistenceManager` →
///   `DeviceStore::save` ke SQLite, dengan background saver 30 detik
///   (`src/bot.rs:1394`). Jadi "menyalin creds JS balik ke disk" tidak punya
///   padanan Rust.
/// * `oktz_store` tidak punya helper creds; `JsonStore` itu key-value generik
///   dan `oktz-wa` bahkan tidak depend ke `oktz-store` sama sekali.
/// * Yang paling penting: rewrite `creds.json` dari snapshot in-memory
///   akan menimpa prekey/advSecretKey yang SUDAH dirotasi library ke SQLite
///   dengan nilai basi — key rollback senyap yang menetap. JS aman di sini
///   karena `sock.authState.creds` adalah objek yang sama persis dengan yang
///   diputar library, bukan salinan.
///
/// Yang hilang akibat ini, dan hanya ini: `pairing::reset_pairing_creds` pada
/// cabang `Apply(ResetPairing)` tidak bertahan lintas restart. Dampaknya
/// kecil — pemanggil wajib pairing ulang, dan `pair_code_flow` sudah
/// membangun store dari nol.
pub fn load_guard_creds(path: Option<&str>) -> serde_json::Value {
    let env = oktz_core::env::Env::get(CREDS_ENV);
    resolve_creds_path(path, env.as_deref())
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(serde_json::Value::Null)
}

/// Jalur produksi: `connect_device` + [`FalseLogoutGuard`] terpasang.
///
/// Pemasangan guard-nya milik [`crate::pairing_run`]; ini titik masuk
/// bernama supaya semua caller punya satu jalan yang jelas.
pub async fn connect_device_guarded<F, Fut>(
    store: SqliteStore,
    creds: serde_json::Value,
    on_ready: F,
) -> Result<(BotHandle, Arc<FalseLogoutGuard>)>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    pairing_run::connect_device_guarded(store, creds, on_ready).await
}

/// Boot bot dan pump handle; `on_ready` dipanggil setelah connect.
///
/// Guard 401 sudah terpasang di setiap jalur ini — termasuk jalur produksi.
/// Creds diambil dari [`CREDS_ENV`] karena signature lama tidak punya
/// tempat menerima creds dan tidak boleh pecah.
///
/// # Batas urutan (bukan jaminan)
///
/// whatsapp-rust 0.7.0 tidak punya hook pra-logout. Pada
/// `<stream:error code="401">` `handle_stream_error` sudah men-clear
/// `enable_auto_reconnect` dan menandai `should_disconnect` **sebelum**
/// `Event::LoggedOut` keluar ke handler mana pun
/// (whatsapp-rust-0.7.0 `src/client/node_io.rs:1789-1801`), dan supervision
/// loop sudah `break` + `stop_supervision_loop()` kalau flag itu masih
/// `false` saat dibaca (`src/client/lifecycle.rs:641-644`). Jadi guard ini
/// mitigasi (menghidupkan lagi flag dan memanggil ulang `Client::run()`),
/// bukan penjamin: ada jendela mati selama grace di mana socket sudah turun.
///
/// Yang sudah diperiksa dan sengaja TIDAK dipakai untuk "memperbaiki" urutan:
/// [`Bot::with_event_handler`] (raw handler) disubscribe langsung ke bus
/// (`src/bot.rs:596-598`) dan `CoreEventBus::dispatch` memanggil
/// `handle_event` secara sinkron (`wacore src/types/events.rs:609-621`), jadi
/// handler-nya jalan inline tepat setelah flag ter-clear — tidak sebelum.
/// `EventDelivery::Ordered` juga tidak menolong: mailbox-nya digerakkan task
/// terpisah (`src/bot.rs:290-332`), jadi tetap bisa telat. Inline membuat
/// keputusan guard terurut terhadap event-nya, tetapi sekaligus membuat
/// `enable_auto_reconnect=true` pra-grace di [`crate::pairing_run`] langsung
/// terbaca oleh loop, jadi reconnect-nya seketika dan anggaran 2 retry
/// terkuras dalam hitungan detik. Reconnect yang tertunda grace adalah
/// perilaku yang benar, jadi wiring yang sudah direview itu dibiarkan apa
/// adanya.
pub async fn connect_device<F, Fut>(store: SqliteStore, on_ready: F) -> Result<BotHandle>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (handle, _guard) = connect_device_guarded(store, load_guard_creds(None), on_ready).await?;
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing;

    fn tmpfile(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("oktz-bot-test-{name}.json"));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn explicit_creds_path_beats_env() {
        assert_eq!(
            resolve_creds_path(Some("/a/creds.json"), Some("/b/creds.json")),
            Some(PathBuf::from("/a/creds.json"))
        );
    }

    #[test]
    fn env_is_the_fallback_and_blank_means_absent() {
        assert_eq!(
            resolve_creds_path(None, Some("/b/creds.json")),
            Some(PathBuf::from("/b/creds.json"))
        );
        assert_eq!(resolve_creds_path(None, None), None);
        assert_eq!(resolve_creds_path(None, Some("   ")), None);
        assert_eq!(
            resolve_creds_path(Some(""), Some("/b/creds.json")),
            Some(PathBuf::from("/b/creds.json")),
            "argumen kosong harus jatuh ke env, bukan jadi path invalid"
        );
    }

    #[test]
    fn load_guard_creds_reads_the_authenticated_markers() {
        let p = tmpfile("creds-ok");
        std::fs::write(
            &p,
            r#"{"registered": true, "account": {"details": "x"}, "additionalData": {}}"#,
        )
        .unwrap();
        let c = load_guard_creds(Some(p.to_str().unwrap()));
        assert!(pairing::is_authenticated(&c));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn unreadable_creds_yield_null_so_logout_is_trusted() {
        // Arah konservatif: creds tak terbaca = "belum pernah ter-link",
        // jadi 401 langsung dipercaya. Guard TIDAK boleh gagal start.
        let c = load_guard_creds(Some("/definitely/not/here/creds.json"));
        assert!(c.is_null());
        assert!(!pairing::is_authenticated(&c));
    }

    #[test]
    fn production_path_reads_creds_from_env() {
        // `connect_device` memanggil `load_guard_creds(None)`, jadi cabang env
        // adalah satu-satunya yang dipakai jalur produksi. Diuji lewat env
        // sungguhan (bukan helper murni) supaya nama env-nya ikut terkunci —
        // kalau `CREDS_ENV` salah ketik, tes ini yang menangkapnya.
        // Aman: tidak ada tes lain di workspace yang menyentuh var ini.
        let p = tmpfile("creds-env");
        std::fs::write(
            &p,
            r#"{"registered": true, "account": {"details": "x"}, "additionalData": {}}"#,
        )
        .unwrap();
        let path = p.to_str().unwrap();

        unsafe { std::env::set_var(CREDS_ENV, path) };
        let from_env = load_guard_creds(None);
        unsafe { std::env::remove_var(CREDS_ENV) };

        assert!(
            pairing::is_authenticated(&from_env),
            "{} tidak dibaca sebagai creds ter-autentikasi",
            CREDS_ENV
        );
        std::fs::remove_file(&p).ok();
    }
}
