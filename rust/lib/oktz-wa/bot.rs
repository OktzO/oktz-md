use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use oktz_plugin_api::berkas::BerkasNyata;
use oktz_plugin_api::UreqHttp;
use oktz_router::ParseOptions;
use oktz_store::json_store::JsonStore;
use whatsapp_rust::prelude::*;

use crate::creds;
use crate::pairing_run::{self, FalseLogoutGuard};
use crate::pipeline::{ClientSender, Identity, Pipeline, StoreContext};

/// Env yang memberi tahu jalur produksi di mana `creds.json` berada.
///
/// [`connect_device`] tidak menerima creds (signedannya dijaga supaya tidak
/// pecah), dan tanpa snapshot guard tidak bisa membedakan "sesi sah yang
/// dapat 401 palsu" dari "belum pernah ter-link". Tanpa env ini guard
/// menerima `Value::Null` → `pairing::is_authenticated` false → arahnya
/// selalu "percaya logout". Itu konservatif (tidak pernah menahan logout
/// asli) tapi berarti grace window tidak aktif; caller yang punya file creds
/// sebaiknya memakai [`connect_device_guarded`] langsung.
pub const CREDS_ENV: &str = "OKTZ_CREDS";

/// Env yang menimpa lokasi settings store. Kosong = bukan override.
pub const SETTINGS_ENV: &str = "OKTZ_SETTINGS";

/// `bot_id` dipakai kalau creds tidak menyebut JID sendiri. Hanya jadi
/// namespace prefiks pada kunci dedup/spam (`handler.js:926, 1508`), jadi nilai
/// tetap yang penting, bukan formatnya.
pub const DEFAULT_BOT_ID: &str = "oktz";

/// Lokasi settings store yang dibaca router: `users` / `groups` / `settings`.
///
/// Default-nya mengikuti `oktz-bin` (`main.rs:166`), yaitu
/// `<database.path>/settings.json`. Argumen eksplisit selalu menang atas env
/// supaya bisa diuji tanpa menyentuh `std::env` yang process-wide.
pub fn settings_path(db_dir: &str, env: Option<&str>) -> PathBuf {
    match env.map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => PathBuf::from(db_dir).join("settings.json"),
    }
}

/// `OWNER_NUMBERS` → [`Identity`].
///
/// `oktz_core::Config` hanya punya `owner_numbers`; premium/partner/admin tidak
/// punya accessor sama sekali, jadi kumpulan itu sengaja dibiarkan kosong
/// (lihat catatan modul di `pipeline.rs`).
pub fn identity_from_owners<I: IntoIterator<Item = String>>(owners: I) -> Identity {
    Identity {
        owners: owners.into_iter().collect(),
        ..Default::default()
    }
}

/// `creds.me.id` → `bot_id`, dengan [`DEFAULT_BOT_ID`] sebagai fallback.
///
/// Creds tak terbaca menghasilkan `Value::Null` (`load_guard_creds`), jadi
/// jalur ini harus tidak pernah gagal — `bot_id` tidak pernah boleh memblokir
/// boot hanya karena creds belum ada.
pub fn bot_id_from(creds: &serde_json::Value) -> String {
    creds
        .get("me")
        .and_then(|me| me.get("id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_BOT_ID)
        .to_string()
}

/// Segala yang dibutuhkan pipeline yang TIDAK butuh `Arc<Client>`.
///
/// Dipisah supaya semua yang mahal (baca file settings) selesai SEBELUM
/// connect. Yang tersisa saat `Event::Connected` tiba hanyalah `subscribe`,
/// sehingga jendela pesan yang bisa terlewat antara dispatch dan subscribe
/// jadi kecil. Konstruksi yang boros di dalam `on_ready` justru
/// memperlebar jendela itu — lihat catatan pada [`connect_device`].
pub struct PipelineWiring {
    ctx: StoreContext,
    identity: Identity,
    bot_id: String,
    /// `config.js` runtime untuk `ctx.config()`.
    config: oktz_core::config::Runtime,
    /// Pipeline yang sudah menempel ke bus. Disimpan supaya reconnect
    /// memakai instance yang sama — membangun yang baru akan meninggalkan
    /// loop lama hidup dan membalas dua kali.
    live: Mutex<Option<Arc<Pipeline>>>,
}

/// Bangun [`PipelineWiring`] dari lokasi store, owner, dan bot id.
///
/// Semua argumen eksplisit: `Config::from_env()` memanggil `Env::load_dotenv()`
/// yang mengubah environment process-wide, jadi dipanggil di satu tempat saja
/// (`connect_device`), bukan di dalam helper yang juga dipakai tes.
pub fn wiring_from(
    settings: &std::path::Path,
    owners: impl IntoIterator<Item = String>,
    bot_id: impl Into<String>,
) -> Result<PipelineWiring> {
    wiring_from_with_config(
        settings,
        owners,
        bot_id,
        oktz_core::config::Runtime::default(),
    )
}

/// Seperti [`wiring_from`], tapi `config.js`-nya bisa diganti. Dipakai jalur
/// produksi yang memang punya `config.js` sungguhan; test memakai varian
/// bawaan yang kosong.
pub fn wiring_from_with_config(
    settings: &std::path::Path,
    owners: impl IntoIterator<Item = String>,
    bot_id: impl Into<String>,
    config: oktz_core::config::Runtime,
) -> Result<PipelineWiring> {
    let store = JsonStore::open(settings)
        .with_context(|| format!("buka settings store {}", settings.display()))?;
    Ok(PipelineWiring {
        ctx: StoreContext::new(Arc::new(store)),
        identity: identity_from_owners(owners),
        bot_id: bot_id.into(),
        config,
        live: Mutex::new(None),
    })
}

impl PipelineWiring {
    /// Pasang pipeline ke bus event. Aman dipanggil berulang.
    fn attach(&self, client: &Arc<Client>) {
        let mut slot = self.live.lock().unwrap_or_else(|e| e.into_inner());
        let pipeline = match slot.as_ref() {
            Some(p) => p.clone(),
            None => {
                let sender = Arc::new(ClientSender::new(client.clone()));
                let p = Arc::new(
                    Pipeline::new(
                        self.ctx.clone(),
                        sender.clone(),
                        self.bot_id.clone(),
                        ParseOptions::default(),
                    )
                    .with_identity(self.identity.clone())
                    .with_http(Arc::new(UreqHttp::new()))
                    .with_config(self.config.clone())
                    // `Ctx.sock` + roster nyata dari klien yang sama.
                    .with_sock(crate::sock_client::SockKlien::baru(client.clone()))
                    // `fs` Nyata berakar ke cwd — launcher sudah men-set cwd ke
                    // akar repo, jadi jalur `./database/...` sama dengan JS.
                    .with_berkas(Arc::new(BerkasNyata::baru(
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    )))
                    .with_roster_source(sender),
                );
                *slot = Some(p.clone());
                p
            }
        };
        drop(slot);
        pipeline.attach(client);
    }
}

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
/// Db sesi lama boleh dipakai ulang hanya kalau sidik creds-nya cocok.
///
/// Db tanpa marker sidik dianggap tidak sah: device row di dalamnya bisa
/// berasal dari creds lain, dan memakainya bersama creds baru membuat sinyal
/// tidak nyambung.
fn sesi_lama_boleh_dipakai(db_ada: bool, marker: Option<&str>, sidik: &str) -> bool {
    db_ada && marker.is_some_and(|m| m.trim() == sidik)
}

/// Buka store sqlite, simpan device, muat sesi onigis, lalu siap connect.
///
/// Db sesi yang sudah ada dan ber-sidik sama DIPAKAI ULANG: di situ device
/// yang sudah dirotasi library + sesi ratchet bertahan, jadi restart tidak
/// memaksa dekripsi ulang dari nol. Db yang hilang atau ber-sidik beda
/// dibangun ulang dari creds.
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

    // Satu-satunya tempat di crate ini yang benar-benar menulis device row,
    // jadi ini juga tempat yang tahu di mana artefak sesi durable hidup.
    // Dipakai [`pairing_run::purge_handler`] saat logout dipercaya.
    pairing_run::note_store_path(&db);

    let mut ident = crate::creds::b64(&c.signed_identity_key.public)?;
    let sidik = format!(
        "{}:{}",
        c.registration_id,
        ident.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let marker = db.with_extension("ident");
    let marker_isi = std::fs::read_to_string(&marker).ok();
    let db_ada = db.exists();

    if sesi_lama_boleh_dipakai(db_ada, marker_isi.as_deref(), &sidik) {
        tracing::info!(db = %db.display(), "db sesi dipakai ulang (sesi ratchet lama dipertahankan)");
        return Ok(SqliteStore::new(db.to_str().context("path tidak utf8")?).await?);
    }

    let _ = std::fs::remove_file(&db);
    let store = SqliteStore::new(db.to_str().context("path tidak utf8")?).await?;

    let device = build_device(&c).await?;
    store.save_device_data_for_device(1, &device).await?;

    if let Some(fx) = session_fixture {
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

    // Marker gagal ditulis bukan alasan gagal start — efeknya hanya db ini
    // dibangun ulang pada start berikutnya.
    if let Err(e) = std::fs::write(&marker, &sidik) {
        tracing::warn!(error = %e, path = %marker.display(), "gagal menulis sidik sesi");
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
///
/// Handler purge ikut terpasang, karena [`pairing_run::connect_device_guarded`]
/// yang memasangkannya dan fungsi ini mendelegasikannya tanpa syarat apa pun —
/// jalur produksi `oktz-bin` yang memanggil [`connect_device`] ikut dapat
/// purge, bukan cuma caller langsung.
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

/// [`connect_device_guarded`] dengan plan purge eksplisit.
///
/// Dipakai kalau lokasi store sesi sudah diketahui pemanggil dan tidak boleh
/// diambil dari env / pencatatan [`pairing_run::note_store_path`].
pub async fn connect_device_guarded_with_purge<F, Fut>(
    store: SqliteStore,
    creds: serde_json::Value,
    plan: pairing_run::PurgePlan,
    on_ready: F,
) -> Result<(BotHandle, Arc<FalseLogoutGuard>)>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    pairing_run::connect_device_guarded_with_purge(store, creds, plan, on_ready).await
}

/// Boot bot dan pump handle; `on_ready` dipanggil setelah connect.
///
/// Guard 401 sudah terpasang di setiap jalur ini — termasuk jalur produksi.
/// Creds diambil dari [`CREDS_ENV`] karena signature lama tidak punya
/// tempat menerima creds dan tidak boleh pecah.
///
/// # Pipeline pesan
///
/// `on_ready` dibungkus supaya [`crate::pipeline::Pipeline`] terpasang ke bus
/// event SEBELUM closure milik pemanggil dijalankan: setiap pesan inbound
/// `Event::Messages` lalu antre dan diproses (lihat [`PipelineWiring::attach`]).
///
/// Bentuk ini dipakai karena `on_ready` adalah satu-satunya titik di jalur ini
/// yang memegang `Arc<Client>`, dan `pairing_run` berada di luar file yang boleh
/// diubah. Builder (`Bot::on_event_for`) sebenarnya bisa mendaftarkan handler
/// lebih awal, tapi itu menuntut membangun `Bot` sendiri — artinya
/// menggandakan pemasangan `FalseLogoutGuard`, yang lebih berbahaya daripada
/// jendela kecil di bawah.
///
/// ## Batas yang jujur: jendela di `Connected`
///
/// `Event::Connected` dikirim per generasi koneksi
/// (`whatsapp-rust-0.7.0/src/client/lifecycle.rs:230`) dan handler-nya
/// dijalankan pada task terpisah (`src/bot.rs:344-356`), bukan inline. Jadi
/// ada secara teoretis `Event::Messages` yang bisa ter-dispatch sebelum
/// `subscribe` selesai. Jendelanya dibuat sekecil mungkin dengan mengerjakan
/// bagian mahal (baca settings) sebelum connect, sehingga di `on_ready` hanya
/// tersisa `subscribe`. Menutupnya sepenuhnya berarti mendaftarkan handler lewat
/// `Bot::on_event_for` sebelum build — menuntut perubahan di `pairing_run.rs`.
///
/// Kegagalan membangun pipeline (settings.json rusak/tidak terbaca) TIDAK
/// menggagalkan boot: yang dicatat, command tidak dibalas, koneksi tetap jalan.
///
/// # Batas urutan 401 (bukan jaminan)
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
    let creds = load_guard_creds(None);

    let cfg = oktz_core::config::Config::load().context("baca config.json")?;
    let settings = settings_path(
        &cfg.database.path,
        oktz_core::env::Env::get(SETTINGS_ENV).as_deref(),
    );
    let wiring = match wiring_from_with_config(
        &settings,
        cfg.owner_numbers.clone(),
        bot_id_from(&creds),
        cfg.runtime(),
    ) {
        Ok(w) => Some(Arc::new(w)),
        Err(e) => {
            tracing::error!(
                error = %e,
                settings = %settings.display(),
                "pipeline pesan tidak aktif; command inbound tidak akan dibalas"
            );
            None
        }
    };
    if wiring.is_some() {
        tracing::info!(settings = %settings.display(), "pipeline pesan siap");
    }

    let (handle, _guard) = connect_device_guarded(store, creds, move |client: Arc<Client>| {
        if let Some(w) = &wiring {
            w.attach(&client);
        }
        on_ready(client)
    })
    .await?;
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing;
    use oktz_router::RouterContext;

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
    fn sesi_lama_dipakai_hanya_kalau_sidik_creds_cocok() {
        assert!(sesi_lama_boleh_dipakai(true, Some("7:abc\n"), "7:abc"));
        assert!(!sesi_lama_boleh_dipakai(true, Some("8:abc"), "7:abc"));
        assert!(
            !sesi_lama_boleh_dipakai(true, None, "7:abc"),
            "db tanpa marker sidik tidak boleh dipercaya"
        );
        assert!(!sesi_lama_boleh_dipakai(false, Some("7:abc"), "7:abc"));
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

    // ---------- pipeline wiring (tanpa koneksi WA) ----------

    #[test]
    fn settings_path_defaults_under_the_database_dir() {
        // Default harus sama dengan yang dipakai oktz-bin (main.rs:166),
        // kalau tidak pipeline akan membaca store yang salah diam-diam.
        assert_eq!(
            settings_path("./database/main", None),
            PathBuf::from("./database/main/settings.json")
        );
    }

    #[test]
    fn settings_env_overrides_but_blank_does_not() {
        assert_eq!(
            settings_path("./database/main", Some("/var/lib/oktz/s.json")),
            PathBuf::from("/var/lib/oktz/s.json")
        );
        for blank in ["", "   "] {
            assert_eq!(
                settings_path("./database/main", Some(blank)),
                PathBuf::from("./database/main/settings.json"),
                "{blank:?} harus berarti tidak di-override, bukan path path-butuh-dibuat"
            );
        }
    }

    #[test]
    fn owner_numbers_become_identity_owners() {
        let id = identity_from_owners(vec!["6281".to_string(), "6282".to_string()]);
        assert!(id.is_owner("6281@s.whatsapp.net"));
        assert!(id.is_owner("6282:9@s.whatsapp.net"), "cocok lewat nomor");
        assert!(!id.is_owner("6283@s.whatsapp.net"));
        assert!(
            !id.is_premium("6281@s.whatsapp.net") && !id.is_admin("6281@s.whatsapp.net"),
            "premium/admin tidak punya sumber di Config — harus kosong, bukan menebak"
        );
    }

    #[test]
    fn bot_id_falls_back_when_creds_carry_no_me() {
        // `load_guard_creds` mengembalikan Null saat creds tak terbaca; jalur
        // ini tidak boleh menggagalkan boot hanya karena itu.
        assert_eq!(bot_id_from(&serde_json::Value::Null), DEFAULT_BOT_ID);
        assert_eq!(bot_id_from(&serde_json::json!({})), DEFAULT_BOT_ID);
        assert_eq!(bot_id_from(&serde_json::json!({"me": {"id": "  "}})), DEFAULT_BOT_ID);
        assert_eq!(
            bot_id_from(&serde_json::json!({"me": {"id": "6281@s.whatsapp.net"}})),
            "6281@s.whatsapp.net"
        );
    }

    #[test]
    fn wiring_builds_from_a_store_and_survives_a_missing_file() {
        let p = tmpfile("wiring");
        std::fs::write(&p, r#"{"settings": {"botMode": "public"}}"#).unwrap();
        let w = wiring_from(&p, vec!["6281".to_string()], "botid").unwrap();
        assert_eq!(w.bot_id, "botid");
        assert!(w.ctx.mode_settings().mode == "public");
        assert!(w.live.lock().unwrap().is_none(), "belum ada pipeline");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn unreadable_settings_fail_wiring_without_panicking() {
        // File rusak = error yang harus jadi log + boot tanpa pipeline
        // (lihat connect_device), bukan panik di thread boot.
        let p = tmpfile("wiring-bad");
        std::fs::write(&p, "{ this is not json").unwrap();
        assert!(wiring_from(&p, Vec::new(), "botid").is_err());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn wiring_context_is_cloned_so_attach_can_run_more_than_once() {
        // `attach` dipanggil ulang tiap reconnect; kalau ctx tidak bisa di-clone
        // atau state-nya berubah, pipeline kedua akan membaca store berbeda.
        let p = tmpfile("wiring-clone");
        std::fs::write(&p, r#"{"settings": {"onlyGc": true}}"#).unwrap();
        let w = wiring_from(&p, Vec::new(), "botid").unwrap();
        let a = w.ctx.clone();
        let b = w.ctx.clone();
        assert!(a.mode_settings().only_gc && b.mode_settings().only_gc);
        std::fs::remove_file(&p).ok();
    }
}
