//! Pengait runtime untuk [`crate::pairing`].
//!
//! Dua hal yang tadinya tidak pernah dipanggil dari mana pun:
//!
//! * `pair_code_flow` / `pair_qr_flow` — alur linking device lewat pairing code
//!   dan QR, di-build di atas `Client::pair_with_code` dan `Event::PairingQrCode`.
//! * `FalseLogoutGuard` — state machine `pairing::classify_logout_401` +
//!   `pairing::classify_close` yang sebelumnya cuma function mati.
//!
//! Batas policy: tidak ada ambang baru di sini. Semua angka keputusan diambil
//! dari `crate::pairing` (`FALSE_LOGOUT_GRACE_MS`, `FALSE_LOGOUT_RETRIES`).
//! Modul ini hanya menjadwalkannya ke event yang benar dari whatsapp-rust.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{Value, json};
use whatsapp_rust::prelude::{Bot, Client, Event, EventKind, SqliteStore};
use whatsapp_rust::types::events::LoggedOut;

use crate::pairing::{
    self, CloseAction, FALSE_LOGOUT_GRACE_MS, FALSE_LOGOUT_RETRIES, LogoutTrust,
    normalize_pairing_number,
};

/// Milidetik sejak epoch; sumber tunggal waktu untuk seluruh modul ini.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Status code yang dipakai `pairing::classify_close` / `classify_logout_401`
/// untuk satu event logout.
///
/// whatsapp-rust sudah mengaburkan status ke dalam enum
/// `ConnectFailureReason`, jadi angka aslinya dibaca lagi dari `raw`:
/// `<stream:error code="401">` menyimpan `code`. Tanpa stanza (logout lokal)
/// satu-satunya petunjuk adalah enum itu sendiri, dan `ConnectFailureReason::LoggedOut`
/// persis `#[wire = 401]` (wacore src/types/events.rs:1482). Enum lain tidak
/// punya accessor wire publik, dan memang tidak perlu: `classify_close`
/// memperlakukan semua non-401 sebagai `Reconnect`, jadi bentuk yang sama
/// (0) sudah benar tanpa menyalin tabel `#[wire = ...]` jadi sumber kebenaran
/// kedua.
pub fn logout_status_code(info: &LoggedOut) -> u16 {
    if let Some(raw) = &info.raw
        && raw.tag == "stream:error"
        && let Some(code) = raw.as_node_ref().get_attr("code").map(|v| v.as_str())
        && let Ok(n) = code.parse::<u16>()
    {
        return n;
    }
    if let whatsapp_rust::wacore::types::events::ConnectFailureReason::LoggedOut = info.reason {
        return 401;
    }
    0
}

/// Keputusan runtime untuk satu `Event::LoggedOut`.
///
/// Susunannya sama dengan `src/connection.js:978-995`: `classify_close` dulu,
/// dan `classify_logout_401` hanya dipercaya kalau `classify_close` sudah
/// bilang "bukan reconnect" (yaitu memang 401).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutDecision {
    /// Bukan 401 — whatsapp-rust yang reconnect sendiri, kita tidak melakukan apa-apa.
    Reconnect,
    /// 401 palsu: sesi dipertahankan, reconnect setelah grace.
    Retry,
    /// Logout originals dipercaya; `CloseAction` yang harus dijalankan pemanggil.
    Apply(CloseAction),
}

/// Versi murni [`LogoutDecision`] — tanpa client, tanpa I/O, tanpa jam.
pub fn decide_logout(
    status_code: u16,
    creds: &Value,
    connected_at: Option<u64>,
    now: u64,
    retries: u32,
) -> LogoutDecision {
    let action = pairing::classify_close(status_code, creds);
    if action == CloseAction::Reconnect {
        return LogoutDecision::Reconnect;
    }
    match pairing::classify_logout_401(status_code, creds, connected_at, now, retries) {
        LogoutTrust::Retry => LogoutDecision::Retry,
        LogoutTrust::Trust => LogoutDecision::Apply(action),
    }
}

/// Menyimpan dokumen creds bersama + jam connect terakhir + penghitung retry,
/// lalu menjalankan [`decide_logout`] terhadap event yang benar.
///
/// Meng-held bersama karena whatsapp-rust tidak punya API "tunda logout":
/// pada `<stream:error code="401">` `enable_auto_reconnect` di-clear dan
/// supervision loop di-break sebelum handler mana pun jalan
/// (whatsapp-rust-0.7.0 src/client/node_io.rs:1789-1801 dan
/// src/client/lifecycle.rs:637-641). Yang bisa kita lakukan adalah
/// menghidupkan kembali flag itu dan menjalankan ulang loop-nya.
pub struct FalseLogoutGuard {
    creds: std::sync::Mutex<Value>,
    connected_at: AtomicU64,
    retries: AtomicU32,
    /// Dipanggil kalau logout dipercaya dan action-nya `PurgeSession`.
    /// Penghapusan folder sesi milik `crate::session`, jadi pemanggil yang
    /// memutuskan — guard hanya melaporkan.
    on_purge: Option<Arc<dyn Fn(CloseAction) + Send + Sync>>,
}

impl FalseLogoutGuard {
    pub fn new(creds: Value) -> Self {
        Self {
            creds: std::sync::Mutex::new(creds),
            connected_at: AtomicU64::new(0),
            retries: AtomicU32::new(0),
            on_purge: None,
        }
    }

    pub fn with_purge_handler(mut self, f: Arc<dyn Fn(CloseAction) + Send + Sync>) -> Self {
        self.on_purge = Some(f);
        self
    }

    /// Dipanggil tiap `Event::Connected`; ini yang memulai grace window.
    pub fn mark_connected(&self, now: u64) {
        self.connected_at.store(now, Ordering::Relaxed);
    }

    pub fn retries(&self) -> u32 {
        self.retries.load(Ordering::Relaxed)
    }

    pub fn creds(&self) -> std::sync::MutexGuard<'_, Value> {
        self.creds.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Keputusan tanpa menyentuh runtime — dipakai tes dan oleh [`Self::handle`].
    pub fn decide(&self, status_code: u16, now: u64) -> LogoutDecision {
        let creds = self.creds();
        decide_logout(
            status_code,
            &creds,
            Some(self.connected_at.load(Ordering::Relaxed)).filter(|t| *t != 0),
            now,
            self.retries.load(Ordering::Relaxed),
        )
    }

    /// Titik masuk runtime: dipanggil dari handler `Event::LoggedOut`.
    ///
    /// Efek sampingnya sudah terjadi di sini; nilai yang dikembalikan supaya
    /// pemanggil bisa mencatatnya.
    pub fn handle(&self, info: &LoggedOut, client: &Arc<Client>, now: u64) -> LogoutDecision {
        let decision = self.handle_logged_out(info, now);
        match decision {
            LogoutDecision::Retry => rearm_after_grace(Arc::clone(client)),
            LogoutDecision::Apply(_) => {
                // Flag sudah di-clear oleh `handle_stream_error`; store ulang
                // hanya memastikan tidak ada jalur implisit yang menghidupkannya.
                client.enable_auto_reconnect.store(false, Ordering::Relaxed);
            }
            LogoutDecision::Reconnect => {}
        }
        decision
    }

    /// [`Self::handle`] tanpa bagian yang butuh `Arc<Client>`.
    ///
    /// Pemisahan ini bukan beautification: `on_purge` — satu-satunya efek
    /// samping yang bisa menghapus state — hanya bisa diuji kalau cabangnya
    /// bisa dipanggil tanpa koneksi. Sisanya (`rearm_after_grace`, flag
    /// auto-reconnect) justru butuh client sungguhan.
    pub fn handle_logged_out(&self, info: &LoggedOut, now: u64) -> LogoutDecision {
        let status = logout_status_code(info);
        match self.decide(status, now) {
            LogoutDecision::Reconnect => LogoutDecision::Reconnect,
            LogoutDecision::Retry => {
                let n = self.retries.fetch_add(1, Ordering::SeqCst) + 1;
                tracing::warn!(
                    status,
                    attempt = n,
                    max = FALSE_LOGOUT_RETRIES,
                    "401 belum dianggap logout — sesi dipertahankan, reconnect setelah grace"
                );
                LogoutDecision::Retry
            }
            LogoutDecision::Apply(action) => {
                self.retries.store(0, Ordering::Relaxed);
                match action {
                    CloseAction::ResetPairing => {
                        let mut creds = self.creds();
                        pairing::reset_pairing_creds(&mut creds);
                    }
                    CloseAction::PurgeSession | CloseAction::Reconnect => {
                        if let Some(f) = &self.on_purge {
                            f(action);
                        }
                    }
                }
                LogoutDecision::Apply(action)
            }
        }
    }
}

// ───────────────────────────── purge artefak sesi ─────────────────────────────
//
// Padanan `src/connection.js:1027-1055`. Bukti bentuknya:
//
// ```js
// if (fs.existsSync(sessionPath)) {
//   pruneBrokenSessions(storageRoot, path.basename(sessionPath), BROKEN_SESSION_KEEP);
//   fs.renameSync(sessionPath, `${sessionPath}.broken-${Date.now()}`);
// }
// if (TURSO_ENABLED) { await deleteTursoSession("main"); }
// ```
//
// Tiga hal yang harus selalu diingat soal kode ini:
//
// 1. **Rename, bukan hapus.** 401 yang dipercaya bisa tetap transien, dan isi
//    folder `.broken-*` adalah satu-satunya salinan sesi lama. Menghapus akan
//    membuang satu-satunya jalur pemulihan.
// 2. **Prune SEBELUM rename, dan tidak pernah menghapus semua.** Baris
//    `pruneBrokenSessions` muncul sebelum `renameSync` tepat supaya
//    `excess = entries.length - keep` dihitung terhadap karantina yang masih
//    ada — kalau ur dibalik, satu-satunya salinan ikut hilang.
// 3. **Cooldown tidak boleh ikut terhapus.** `storageRoot` holding
//    `.pairing-cooldown` sengaja DI LUAR `sessionPath`
//    (`src/connection.js:624-627`), supaya purge tidak jadi cara untuk melewati cooldown.

/// Env yang menimpa lokasi db sesi untuk [`PurgePlan`].
///
/// Sengaja punya env sendiri, terpisah dari `OKTZ_DB_DIR`: yang di-purge hanya
/// artefak sesi yang sedang dipakai proses ini. `OKTZ_DB_DIR` bisa menunjuk
/// root yang dipakai beberapa db (pairing, smoke, send) dan salah satu saja
/// yang di-purge = kehilangan device row yang tidak diminta.
pub const SESSION_DB_ENV: &str = "OKTZ_SESSION_DB";

/// Suffix sidecar SQLite yang harus ikut pindah bersama db-nya.
///
/// `SqliteStore` menjalankan `PRAGMA journal_mode = WAL` saat boot
/// (`whatsapp-rust-sqlite-storage-0.7.0/src/sqlite_store.rs:523`), jadi
/// `-wal`/`-shm` melekat ke nama db. Kalau db di-rename tapi sidecar-nya
/// dibiarkan, boot berikutnya membuka db BARU di depan `-wal` milik db LAMA dan
/// SQLite akan memutar ulang log itu ke db kosong — bukan sekadar sisa file,
/// tapi rekamrapat yang salah.
const DB_SIDECARS: [&str; 3] = ["-wal", "-shm", "-journal"];

/// Path db sesi hasil [`prepare_store`](crate::bot::prepare_store) terakhir.
///
/// Dipakai karena `SqliteStore` tidak mengekspos path-nya
/// (`database_path` = `pub(crate)` di
/// `whatsapp-rust-sqlite-storage-0.7.0/src/sqlite_store.rs:126`), sementara
/// `connect_device` hanya menerima store-nya. Satu proses = satu store aktif,
/// jadi "path terakhir yang disiapkan" == "store yang sedang terhubung".
static LAST_STORE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

/// Catat tempat store sesi dibuat. Dipanggil oleh
/// [`crate::bot::prepare_store`] — satu-satunya tempat yang benar-benar
/// menulis device row.
pub fn note_store_path(path: &Path) {
    let slot = LAST_STORE.get_or_init(|| Mutex::new(None));
    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.to_path_buf());
}

/// Path store yang dicatat terakhir, kalau ada.
pub fn remembered_store_path() -> Option<PathBuf> {
    LAST_STORE
        .get()
        .and_then(|s| s.lock().unwrap_or_else(|e| e.into_inner()).clone())
}

/// Yang harus dihapus saat logout dipercaya.
///
/// Kosong berarti "tidak ada yang bisa dihapus" dan itu dilaporkan sebagai
/// warning, bukan diperlakukan sebagai purge sukses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PurgePlan {
    /// Db SQLite: device row + session ratchet + prekey. Padanan
    /// `sessionPath` JS.
    pub store_db: Option<PathBuf>,
    /// `creds.json`. Di JS file ini BERADA DI DALAM `sessionPath`, jadi ikut
    /// ter-rename bersama folder; di Rust ia terpisah, jadi di-decompose
    /// eksplisit.
    pub creds: Option<PathBuf>,
    pub keep: usize,
}

impl PurgePlan {
    /// Susun plan dari keadaan runtime: env dulu, lalu store yang dicatat
    /// [`prepare_store`](crate::bot::prepare_store), lalu
    /// [`crate::bot::CREDS_ENV`] untuk creds.
    pub fn resolve() -> Self {
        Self {
            // `.trim()` + filter, sama seperti `bot::resolve_creds_path` dan
            // `bot::settings_path`: `Env::get` hanya menyaring string kosong,
            // jadi env berisi spasi akan jadi path `"   "` yang menunjuk
            // direktori tak terduga.
            store_db: oktz_core::env::Env::get(SESSION_DB_ENV)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .or_else(remembered_store_path),
            creds: crate::bot::resolve_creds_path(
                None,
                oktz_core::env::Env::get(crate::bot::CREDS_ENV).as_deref(),
            ),
            keep: pairing::BROKEN_SESSION_KEEP,
        }
    }

    /// Plan kosong dari eksplisit — dipakai tes dan caller yang sudah tahu
    /// semua jalannya.
    pub fn new(store_db: Option<PathBuf>, creds: Option<PathBuf>) -> Self {
        Self {
            store_db,
            creds,
            keep: pairing::BROKEN_SESSION_KEEP,
        }
    }
}

/// Hasil purge. Setiap field adalah artefak yang benar-benar tersentuh, jadi
/// bisa di-assert dan bukan "seharusnya sudah dihapus".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PurgeReport {
    /// Berapa karantina `.broken-*` lama yang dibuang oleh prune.
    pub pruned: usize,
    /// Db sesi yang di-rename ke `.broken-<now>`.
    pub quarantined_store: Option<PathBuf>,
    /// Sidecar yang ikut pindah (satu per entri yang berhasil).
    pub quarantined_sidecars: Vec<PathBuf>,
    /// `creds.json` yang di-rename ke `.broken-<now>`.
    pub quarantined_creds: Option<PathBuf>,
    /// Kegagalan yang tidak menggagalkan yang lain — JS juga menelan error
    /// (`} catch (e) { }`, src/connection.js:1046), jadi logout yang sudah
    /// dipercaya TIDAK boleh gagal total hanya karena satu path bermasalah.
    pub errors: Vec<String>,
}

/// Namakan artefak yang tidak ada lagi sebagai `<path>.broken-<now_ms>`.
///
/// Timestamp milliseconds sama dengan `Date.now()` di JS, dan `prune`/
/// `pruneBrokenSessions`antikjub mengurutkan nama secara leksikografis —
/// lebarnya sudah tetap selama ini (13 digit) sejak 2001.
fn quarantine_name(path: &Path, now: u64) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".broken-{now}"));
    path.with_file_name(name)
}

/// Pindahkan satu artefak ke dalam folder karantina.
///
/// Rename yang gagal (Windows mengunci file yang sedang dibuka, sticky bit,
/// device penuh) TIDAK dihapus — artefak yang masih ada lebih berguna daripada
/// artefak yang hilang diam-diam.
fn move_into(path: &Path, dest_dir: &Path, errors: &mut Vec<String>) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let dest = dest_dir.join(path.file_name().unwrap_or_default());
    match std::fs::rename(path, &dest) {
        Ok(()) => Some(dest),
        Err(e) => {
            errors.push(format!("gagal karantina {}: {e}", path.display()));
            None
        }
    }
}

/// Pindahkan satu artefak ke karantina datar. `Some(path_karantina)` kalau
/// benar-benar pindah.
///
/// Dipakai untuk `creds.json`, yang di JS ikut ter-rename karena berada DI DALAM
/// `sessionPath`; di Rust ia berdiri sendiri sehingga karantinanya juga datar.
fn quarantine(path: &Path, now: u64, errors: &mut Vec<String>) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let dest = quarantine_name(path, now);
    match std::fs::rename(path, &dest) {
        Ok(()) => Some(dest),
        Err(e) => {
            errors.push(format!("gagal karantina {}: {e}", path.display()));
            None
        }
    }
}

/// Kerjakan purge sesuai `src/connection.js:1031-1055`.
///
/// Murni filesystem + jam yang dikunci lewat `now`, jadi seluruhnya bisa
/// diuji tanpa koneksi dan tanpa `/var/lib` milik orang lain.
pub fn purge_session(plan: &PurgePlan, now: u64) -> PurgeReport {
    let mut r = PurgeReport::default();

    // `if (fs.existsSync(sessionPath))` di JS. Prune dan rename hanya jalan
    // kalau artefak sesi benar-benar ada — prune tanpa rename hanya menghapus
    // karantina orang lain tanpa menambahkan yang baru.
    if let Some(db) = plan.store_db.as_deref().filter(|p| p.exists()) {
        // Karantina = DIRECTORY `<db>.broken-<now>/`, persis seperti JS yang
        // me-rename satu FOLDER. Bentuk direktori bukan sekadar formalitas:
        // sidecar punya prefiks nama yang sama dengan db-nya, jadi kalau
        // dipipihkan sebagai `<db>.broken-<now>-wal`, `prune_broken_sessions`
        // menghitung db dan sidecar sebagai dua entri terpisah — lalu bisa
        // membuang sidecar-nya sambil menyimpan db-nya (frame yang belum
        // checkpoint hilang) atau sebaliknya (orphan nyaring). Satu directory
        // = satu entri prune = satu karantina yang utuh.
        let parent = db.parent().filter(|p| !p.as_os_str().is_empty());
        let base = db
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Urutan WAJIB prune-lalu-rename (lihat catatan modul): prune menghitung
        // `excess = n - keep` dari karantina yang masih ADA. Kalau folder
        // karantina baru dibuat lebih dulu, ikut terhitung dan bot membuang satu
        // backup ekstra tanpa dapat balasan — persis kesalahan yang kode JS
        // komentari soal di src/connection.js:1033-1035.
        if let Some(parent) = parent {
            r.pruned = pairing::prune_broken_sessions(parent, &base, plan.keep);
        }

        let dest_dir = quarantine_name(db, now);
        match std::fs::create_dir(&dest_dir) {
            Ok(()) => {
                r.quarantined_store = move_into(db, &dest_dir, &mut r.errors);
                // Sidecar ikut pindah, bukan dihapus: `-wal` masih berisi frame
                // yang belum checkpoint, jadi menghapusnya membuang state yang
                // tidak ada di db mana pun.
                for suffix in DB_SIDECARS {
                    let side = PathBuf::from(format!("{}{suffix}", db.display()));
                    if let Some(moved) = move_into(&side, &dest_dir, &mut r.errors) {
                        r.quarantined_sidecars.push(moved);
                    }
                }
            }
            Err(e) => r
                .errors
                .push(format!("gagal buat karantina {}: {e}", dest_dir.display())),
        }
    }

    // Creds TIDAK digate oleh keberadaan db: tidak seperti JS, di sini creds
    // berdiri sendiri, dan tertinggalkan yang berbahaya. Kalau creds
    // tetap di tempat, `prepare_store` akan membangun device row
    // dari identitas yang persis sama yang barusan ditolak WA — dan 401-nya
    // berulang selamanya tanpa chance untuk re-pair.
    if let Some(creds) = plan.creds.as_deref() {
        r.quarantined_creds = quarantine(creds, now, &mut r.errors);
    }

    r
}

/// Handler yang dipasang ke [`FalseLogoutGuard::with_purge_handler`].
///
/// Hanya `PurgeSession` yang menghapus apa pun. `ResetPairing` sengaja
/// tidak menyentuh disk (src/connection.js:1003-1026) dan `Reconnect` cuma
/// sampai ke sini lewat match gabung di guard, jadi keduanya dicatat lalu
/// diabaikan — lebih aman daripada diam-diam menganggapnya purge.
pub fn purge_handler(plan: PurgePlan) -> Arc<dyn Fn(CloseAction) + Send + Sync> {
    Arc::new(move |action| match action {
        CloseAction::PurgeSession => {
            let report = purge_session(&plan, now_ms());
            if report.quarantined_store.is_none() && report.quarantined_creds.is_none() {
                tracing::warn!(
                    "logout dipercaya tapi tidak ada artefak sesi yang bisa dipurge — \
                     set {SESSION_DB_ENV} kalau lokasi store tidak ketahu"
                );
            }
            tracing::info!(
                pruned = report.pruned,
                store = ?report.quarantined_store,
                sidecars = report.quarantined_sidecars.len(),
                creds = ?report.quarantined_creds,
                errors = report.errors.len(),
                "sesi di-purge setelah logout dipercaya; pair ulang dibutuhkan"
            );
        }
        other => {
            tracing::warn!(?other, "aksi logout tanpa purge — state sesi tidak dihapus");
        }
    })
}

/// Kembalikan client ke "boleh reconnect" setelah grace window.
///
/// Dua hal harus dibetulkan, karena keduanya sudah terjadi sebelum handler kita
/// jalan:
///
/// 1. `enable_auto_reconnect` di-clear oleh `handle_stream_error`, lalu
///    supervision loop `break` + `stop_supervision_loop()`. Keduanya `pub(crate)`,
///    tidak ada callback pra-break.
/// 2. Jadi kita set flag-nya lagi dan panggil lagi `Client::run()` (publik).
///    Kalau ternyata loop belum sempat break, `run()` balik sendiri dengan
///    warning "already running" dan loop yang sekarang membaca flag `true`
///    langsung lanjut — dua jalur itu menutup race-nya.
///
/// Delay `FALSE_LOGOUT_GRACE_MS` supaya percobaan berikutnya jatuh di dalam
/// grace window, persis seperti ritme retry JS.
fn rearm_after_grace(client: Arc<Client>) {
    client.enable_auto_reconnect.store(true, Ordering::Relaxed);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(FALSE_LOGOUT_GRACE_MS)).await;
        tracing::info!("grace habis — menjalankan ulang supervision loop");
        client.run().await;
    });
}

/// Pasang guard sebagai handler `Connected` + `LoggedOut` pada `BotBuilder`.
///
/// Panggil sebelum `with_backend` supaya tipe builder tetap generic.
pub fn with_logout_guard<B, T, H, R>(
    builder: whatsapp_rust::prelude::BotBuilder<B, T, H, R>,
    guard: Arc<FalseLogoutGuard>,
) -> whatsapp_rust::prelude::BotBuilder<B, T, H, R> {
    builder.on_event_for(
        &[EventKind::Connected, EventKind::LoggedOut],
        move |event, client| {
            let fut = match &*event {
                Event::Connected(_) => {
                    guard.mark_connected(now_ms());
                    None
                }
                Event::LoggedOut(info) => {
                    let info = info.clone();
                    let g = Arc::clone(&guard);
                    Some(async move {
                        let decision = g.handle(&info, &client, now_ms());
                        tracing::info!(?decision, "logout diproses pairing guard");
                    })
                }
                _ => None,
            };
            async move {
                if let Some(f) = fut {
                    f.await;
                }
            }
        },
    )
}
// ───────────────────────────── alur pairing CLI ─────────────────────────────

/// `connect_device` yang memasang [`FalseLogoutGuard`] DAN handler purge-nya.
///
/// Handler purge dipasang DI SINA, di titik paling dalam yang dilewati semua
/// jalur — termasuk `oktz-bin` yang memanggil fungsi ini langsung untuk
/// `smoke`/`send`, dan `crate::bot::connect_device` yang dipakai produksi.
/// Dulu guard-nya saja yang terpasang: `on_purge` selalu `None`, jadi
/// `CloseAction::PurgeSession` sudah dilaporkan tapi tidak pernah menghapus
/// apa pun.
///
/// Plan diambil dari [`PurgePlan::resolve`], bukan dari argumen, karena
/// `SqliteStore` tidak mengekspos path-nya (lihat [`note_store_path`]).
pub async fn connect_device_guarded<F, Fut>(
    store: SqliteStore,
    creds: Value,
    on_ready: F,
) -> anyhow::Result<(whatsapp_rust::prelude::BotHandle, Arc<FalseLogoutGuard>)>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    connect_device_guarded_with_purge(store, creds, PurgePlan::resolve(), on_ready).await
}

/// Seperti [`connect_device_guarded`], tapi plan-nya diberikan pemanggil.
///
/// Berdiri sendiri supaya jalur produksi bisa mengirim plan yang sudah lengkap,
/// dan supaya tes bisa memasang plan ke temp dir tanpa menyentuh env yang
/// process-wide.
pub async fn connect_device_guarded_with_purge<F, Fut>(
    store: SqliteStore,
    creds: Value,
    plan: PurgePlan,
    on_ready: F,
) -> anyhow::Result<(whatsapp_rust::prelude::BotHandle, Arc<FalseLogoutGuard>)>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let guard = Arc::new(
        FalseLogoutGuard::new(creds).with_purge_handler(purge_handler(plan)),
    );
    let bot = with_logout_guard(Bot::builder(), Arc::clone(&guard))
        .with_backend(store)
        .on_connected(on_ready)
        .build()
        .await?;
    Ok((bot.spawn(), guard))
}

const SOCKET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Store baru untuk linking: device row dibikin `PersistenceManager::new`, jadi
/// store lama yang sudah ter-link harus dibuang — kalau tidak, bot tidak akan
/// pernah melihat QR.
async fn fresh_store(db_path: &str) -> anyhow::Result<SqliteStore> {
    let db = std::path::PathBuf::from(db_path);
    if let Some(parent) = db.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&db);
    Ok(SqliteStore::new(
        db.to_str()
            .ok_or_else(|| anyhow::anyhow!("path db tidak utf8: {}", db.display()))?,
    )
    .await?)
}

/// Pump dua channel sampai ada hasil akhir atau `hold_secs` habis.
///
/// Event QR/code lewat channel sendiri supaya payload yang sudah terbit tetap
/// tercetak walau pairing-nya gagal belakangan.
async fn wait_for_pairing(
    done: &std::sync::mpsc::Receiver<Value>,
    codes: &std::sync::mpsc::Receiver<String>,
    hold_secs: u64,
) -> Option<Value> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs);
    loop {
        while let Ok(line) = codes.try_recv() {
            println!("{line}");
        }
        if let Ok(v) = done.try_recv() {
            return Some(v);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// Alur pairing-by-code: minta kode 8 karakter ke server, lalu tunggu
/// konfirmasi dari HP.
pub async fn pair_code_flow(db_path: &str, phone: &str, hold_secs: u64) -> anyhow::Result<Value> {
    let Some(phone) = normalize_pairing_number(phone) else {
        anyhow::bail!("nomor tidak punya digit: {phone:?}");
    };

    let store = fresh_store(db_path).await?;
    let (code_tx, code_rx) = std::sync::mpsc::channel::<String>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<Value>();

    let bot = Bot::builder()
        .with_backend(store)
        .on_event_for(
            &[
                EventKind::PairingCode,
                EventKind::PairSuccess,
                EventKind::PairError,
            ],
            {
                let code_tx = code_tx.clone();
                let done_tx = done_tx.clone();
                let phone = phone.clone();
                move |event, _client| {
                    let notice = match &*event {
                        Event::PairingCode(pc) => Some((
                            true,
                            json!({
                                "phase": "CODE",
                                "code": pc.code,
                                "timeout_secs": pc.timeout.as_secs(),
                                "phone": phone,
                                "instruction": "WhatsApp > Linked Devices > Link a Device > Link with phone number instead",
                            }),
                        )),
                        Event::PairSuccess(ps) => Some((
                            false,
                            json!({
                                "phase": "PAIRED",
                                "jid": ps.id.to_string(),
                                "lid": ps.lid.to_string(),
                            }),
                        )),
                        Event::PairError(e) => {
                            Some((false, json!({"phase": "PAIR_ERROR", "error": e.error})))
                        }
                        _ => None,
                    };
                    let fut = notice.map(|(is_code, payload)| {
                        let code_tx = code_tx.clone();
                        let done_tx = done_tx.clone();
                        async move {
                            if is_code {
                                let _ = code_tx.send(payload.to_string());
                            } else {
                                let _ = done_tx.send(payload);
                            }
                        }
                    });
                    async move {
                        if let Some(f) = fut {
                            f.await;
                        }
                    }
                }
            },
        )
        .build()
        .await?;

    let handle = bot.spawn();
    let client = handle.client();

    // Socket harus siap (belum login) sebelum IQ pairing dikirim.
    if let Err(e) = client.wait_for_socket(SOCKET_TIMEOUT).await {
        handle.abort();
        anyhow::bail!("socket tidak siap dalam {}s: {e}", SOCKET_TIMEOUT.as_secs());
    }

    let opts = whatsapp_rust::pair_code::PairCodeOptions {
        phone_number: phone.clone(),
        ..Default::default()
    };
    // `pair_with_code` sudah mencek event `PairingCode` yang membawa kode-nya,
    // jadi tidak ada println kedua di sini.
    let code = match client.pair_with_code(opts).await {
        Ok(c) => c,
        Err(e) => {
            handle.abort();
            anyhow::bail!("pair_with_code gagal: {e}");
        }
    };
    tracing::info!(code, "pairing code terbit");

    let outcome = wait_for_pairing(&done_rx, &code_rx, hold_secs).await;
    handle.abort();

    match outcome {
        Some(v) => Ok(v),
        None => anyhow::bail!("pairing tidak selesai dalam {hold_secs}s (kode tidak dikonfirmasi)"),
    }
}

/// Alur pairing via QR: print payload string yang harus di-render jadi gambar QR
/// dan dipindai dari HP.
pub async fn pair_qr_flow(db_path: &str, hold_secs: u64) -> anyhow::Result<Value> {
    let store = fresh_store(db_path).await?;
    let (code_tx, code_rx) = std::sync::mpsc::channel::<String>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<Value>();

    let bot = Bot::builder()
        .with_backend(store)
        .on_event_for(
            &[
                EventKind::PairingQrCode,
                EventKind::PairingQrCodesExhausted,
                EventKind::PairSuccess,
                EventKind::PairError,
            ],
            {
                let done_tx = done_tx.clone();
                move |event, _client| {
                    let notice = match &*event {
                        Event::PairingQrCode(qr) => Some((
                            true,
                            json!({
                                "phase": "QR",
                                "code": qr.code,
                                "timeout_secs": qr.timeout.as_secs(),
                            }),
                        )),
                        Event::PairingQrCodesExhausted(ex) => Some((
                            false,
                            json!({"phase": "QR_EXPIRED", "disconnected": ex.disconnected}),
                        )),
                        Event::PairSuccess(ps) => Some((
                            false,
                            json!({
                                "phase": "PAIRED",
                                "jid": ps.id.to_string(),
                                "lid": ps.lid.to_string(),
                            }),
                        )),
                        Event::PairError(e) => {
                            Some((false, json!({"phase": "PAIR_ERROR", "error": e.error})))
                        }
                        _ => None,
                    };
                    let fut = notice.map(|(is_code, payload)| {
                        let code_tx = code_tx.clone();
                        let done_tx = done_tx.clone();
                        async move {
                            if is_code {
                                let _ = code_tx.send(payload.to_string());
                            } else {
                                let _ = done_tx.send(payload);
                            }
                        }
                    });
                    async move {
                        if let Some(f) = fut {
                            f.await;
                        }
                    }
                }
            },
        )
        .build()
        .await?;

    let handle = bot.spawn();
    let client = handle.client();
    if let Err(e) = client.wait_for_socket(SOCKET_TIMEOUT).await {
        handle.abort();
        anyhow::bail!("socket tidak siap dalam {}s: {e}", SOCKET_TIMEOUT.as_secs());
    }

    let outcome = wait_for_pairing(&done_rx, &code_rx, hold_secs).await;
    handle.abort();

    match outcome {
        Some(v) => Ok(v),
        None => anyhow::bail!("pairing QR tidak selesai dalam {hold_secs}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;
    use whatsapp_rust::NodeBuilder;
    use whatsapp_rust::wacore::types::events::ConnectFailureReason;

    fn authed() -> Value {
        json!({"account": {"details": "x"}, "registered": true, "additionalData": {}})
    }

    fn stream_error(code: &str) -> LoggedOut {
        LoggedOut::builder()
            .on_connect(false)
            .reason(ConnectFailureReason::LoggedOut)
            .raw(
                NodeBuilder::new("stream:error")
                    .attrs([("code", code.to_string())])
                    .build(),
            )
            .build()
    }

    #[test]
    fn status_code_read_from_stream_error_attr() {
        assert_eq!(logout_status_code(&stream_error("401")), 401);
        assert_eq!(logout_status_code(&stream_error("516")), 516);
        assert_eq!(logout_status_code(&stream_error("440")), 440);
    }

    #[test]
    fn status_code_falls_back_to_reason_wire_code() {
        // Tanpa `raw` (logout lokal) satu-satunya petunjuk adalah enum.
        let local = LoggedOut::builder()
            .on_connect(false)
            .reason(ConnectFailureReason::LoggedOut)
            .build();
        assert_eq!(logout_status_code(&local), 401);

        // Non-401 sengaja dipetakan ke 0: classify_close memperlakukannya
        // sebagai Reconnect, dan wacore tidak mengekspos tabel wire-nya.
        for reason in [
            ConnectFailureReason::AccountLocked,
            ConnectFailureReason::UnknownLogout,
            ConnectFailureReason::ServiceUnavailable,
            ConnectFailureReason::Unknown(418),
        ] {
            let e = LoggedOut::builder().on_connect(true).reason(reason).build();
            assert_eq!(logout_status_code(&e), 0, "{reason:?}");
        }
    }

    #[test]
    fn transient_401_inside_grace_retries() {
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &authed(), Some(t0), t0 + 5_000, 0),
            LogoutDecision::Retry
        );
    }

    #[test]
    fn second_grace_retry_is_still_allowed() {
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &authed(), Some(t0), t0 + 30_000, 1),
            LogoutDecision::Retry
        );
    }

    #[test]
    fn logout_401_after_grace_window_is_trusted() {
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &authed(), Some(t0), t0 + FALSE_LOGOUT_GRACE_MS + 1, 0),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
    }

    #[test]
    fn logout_401_with_retries_exhausted_is_trusted() {
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &authed(), Some(t0), t0 + 1_000, FALSE_LOGOUT_RETRIES),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
    }

    #[test]
    fn unauthenticated_401_is_a_real_logout() {
        // Tidak pernah ter-link: 401 bukan "palsu", tidak ada grace.
        let fresh = json!({"additionalData": {}});
        assert_eq!(
            decide_logout(401, &fresh, Some(1_000_000), 1_005_000, 0),
            LogoutDecision::Apply(CloseAction::ResetPairing)
        );
    }

    #[test]
    fn pairing_pending_401_still_gets_the_grace_window() {
        // `classifyLogout401` (dan padanannya di src/connection.js:978-995)
        // tidak pernah mengecek `pairingPending` — hanya `isAuthenticated`.
        // Creds hasil pairing lama masih punya `registered: true`, jadi 401
        // pertama di dalam grace tetap Retry. Guard ini mengunci urutan itu,
        // supaya "tambahkan cek pairingPending" tidak diam-diam mengubah policy.
        let mut c = authed();
        pairing::mark_pairing_pending(&mut c);
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &c, Some(t0), t0 + 5_000, 0),
            LogoutDecision::Retry
        );
    }

    #[test]
    fn pairing_pending_401_after_grace_resets_pairing_not_session() {
        // Inilah gunanya cabang ResetPairing: begitu grace habis, 401 yang
        // sekarang dipercaya me-reset creds pairing dan TIDAK menghapus folder
        // sesi (src/connection.js:1002-1024).
        let mut c = authed();
        pairing::mark_pairing_pending(&mut c);
        let t0 = 1_000_000;
        assert_eq!(
            decide_logout(401, &c, Some(t0), t0 + FALSE_LOGOUT_GRACE_MS + 1, 0),
            LogoutDecision::Apply(CloseAction::ResetPairing)
        );
    }

    #[test]
    fn non_401_close_is_left_to_the_library() {
        assert_eq!(
            decide_logout(516, &authed(), Some(1_000_000), 1_005_000, 0),
            LogoutDecision::Reconnect
        );
        assert_eq!(
            decide_logout(440, &authed(), Some(1_000_000), 1_005_000, 0),
            LogoutDecision::Reconnect
        );
    }

    #[test]
    fn guard_decides_retry_inside_grace() {
        let g = FalseLogoutGuard::new(authed());
        let t0 = 1_000_000;
        g.mark_connected(t0);
        assert_eq!(g.decide(401, t0 + 1_000), LogoutDecision::Retry);
        assert_eq!(
            g.decide(401, t0 + 2_000),
            LogoutDecision::Retry,
            "decide() tidak boleh menaikkan penghitung; handle() yang incremented"
        );
        assert_eq!(g.retries(), 0);
    }

    #[test]
    fn guard_has_no_grace_before_first_connected() {
        // Tanpa `mark_connected` tidak ada grace window — 401 langsung dipercaya.
        let g = FalseLogoutGuard::new(authed());
        assert_eq!(
            g.decide(401, 1_000_000),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
    }

    #[test]
    fn guard_ignores_grace_once_window_expired() {
        let g = FalseLogoutGuard::new(authed());
        g.mark_connected(1_000_000);
        assert_eq!(
            g.decide(401, 1_000_000 + FALSE_LOGOUT_GRACE_MS + 1),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
    }

    #[test]
    fn guard_creds_drive_the_pairing_pending_branch() {
        let mut c = json!({"registered": true, "additionalData": {}});
        pairing::mark_pairing_pending(&mut c);
        let g = FalseLogoutGuard::new(c);
        let t0 = 1_000_000;
        g.mark_connected(t0);

        assert_eq!(g.decide(401, t0 + 1_000), LogoutDecision::Retry);
        assert_eq!(
            g.decide(401, t0 + FALSE_LOGOUT_GRACE_MS + 1),
            LogoutDecision::Apply(CloseAction::ResetPairing),
            "grace habis → ResetPairing, bukan PurgeSession"
        );
        {
            let mut creds = g.creds();
            pairing::reset_pairing_creds(&mut creds);
            assert!(!pairing::is_pairing_pending(&creds));
            assert!(!pairing::is_authenticated(&creds));
        }
    }

    #[test]
    fn normalize_number_is_reachable_from_the_cli_path() {
        assert_eq!(
            normalize_pairing_number("+62 856-0895-3677"),
            Some("6285608953677".into())
        );
        assert_eq!(normalize_pairing_number("abc"), None);
    }

    // ---------- on_purge: Dipanggil atau tidak ----------

    /// Guard dengan pencatat, plus plan yang menunjuk file di temp dir.
    fn guarded(dir: &std::path::Path) -> (FalseLogoutGuard, Arc<Mutex<Vec<CloseAction>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let plan = PurgePlan::new(Some(dir.join("oktz-run.db")), Some(dir.join("creds.json")));
        let guard = FalseLogoutGuard::new(authed()).with_purge_handler(Arc::new(move |a| {
            // Handler sungguhan ikut dijalankan supaya tes ini benar-benar
            // melewati batas hook→filesystem, bukan cuma counter.
            let _ = purge_session(&plan, 1_700_000_000_000);
            sink.lock().unwrap().push(a);
        }));
        (guard, seen)
    }

    fn seed(dir: &std::path::Path) {
        std::fs::write(dir.join("oktz-run.db"), b"sqlite").unwrap();
        std::fs::write(dir.join("oktz-run.db-wal"), b"wal").unwrap();
        std::fs::write(dir.join("creds.json"), r#"{"registered":true}"#).unwrap();
    }

    /// Direktori unik per test — `std::env::temp_dir()` dipakai bersama, jadi
    /// nama harus membawa nama test-nya sendiri.
    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("oktz-purge-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn purge_handler_runs_on_a_trusted_logout_and_removes_the_session() {
        let dir = scratch("trusted");
        seed(&dir);
        let (guard, seen) = guarded(&dir);
        let t0 = 1_000_000;
        guard.mark_connected(t0);

        // 401 setelah grace habis = logout asli (src/connection.js:978-1031).
        assert_eq!(
            guard.handle_logged_out(&stream_error("401"), t0 + FALSE_LOGOUT_GRACE_MS + 1),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
        assert_eq!(*seen.lock().unwrap(), vec![CloseAction::PurgeSession]);

        // Artefak hilang dari tempat asalnya, dan ada di karantina.
        assert!(!dir.join("oktz-run.db").exists(), "db harus pindah");
        assert!(!dir.join("oktz-run.db-wal").exists(), "sidecar harus ikut");
        assert!(!dir.join("creds.json").exists(), "creds harus ikut");
        let q = dir.join("oktz-run.db.broken-1700000000000");
        assert!(q.is_dir(), "karantina = satu directory, satu entri prune");
        assert!(q.join("oktz-run.db").exists());
        assert!(q.join("oktz-run.db-wal").exists());
        assert!(dir.join("creds.json.broken-1700000000000").exists());
        assert!(!dir.join(".pairing-cooldown").exists(), "cooldown tidak lahir");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_transient_401_inside_grace_never_touches_disk() {
        let dir = scratch("transient");
        seed(&dir);
        let (guard, seen) = guarded(&dir);
        let t0 = 1_000_000;
        guard.mark_connected(t0);

        assert_eq!(
            guard.handle_logged_out(&stream_error("401"), t0 + 1_000),
            LogoutDecision::Retry
        );
        assert!(seen.lock().unwrap().is_empty(), "hook tidak boleh jalan");
        assert!(dir.join("oktz-run.db").exists(), "sesi sah harus utuh");
        assert!(dir.join("oktz-run.db-wal").exists());
        assert!(dir.join("creds.json").exists());

        // Retry kedua juga masih di dalam grace → tetap tanpa purge.
        assert_eq!(
            guard.handle_logged_out(&stream_error("401"), t0 + 2_000),
            LogoutDecision::Retry
        );
        assert!(seen.lock().unwrap().is_empty());
        assert!(dir.join("oktz-run.db").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn retries_exhausted_trusts_the_logout_and_purges() {
        let dir = scratch("exhausted");
        seed(&dir);
        let (guard, seen) = guarded(&dir);
        let t0 = 1_000_000;
        guard.mark_connected(t0);
        guard.handle_logged_out(&stream_error("401"), t0 + 1_000);
        guard.handle_logged_out(&stream_error("401"), t0 + 2_000);
        assert!(seen.lock().unwrap().is_empty());

        // Grace masih aktif tapi anggaran retry habis → percaya (src/connection.js:998).
        assert_eq!(
            guard.handle_logged_out(&stream_error("401"), t0 + 3_000),
            LogoutDecision::Apply(CloseAction::PurgeSession)
        );
        assert_eq!(*seen.lock().unwrap(), vec![CloseAction::PurgeSession]);
        assert!(!dir.join("oktz-run.db").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_non_401_close_reconnects_without_purging() {
        let dir = scratch("non401");
        seed(&dir);
        let (guard, seen) = guarded(&dir);
        let t0 = 1_000_000;
        guard.mark_connected(t0);
        assert_eq!(
            guard.handle_logged_out(&stream_error("440"), t0 + FALSE_LOGOUT_GRACE_MS + 1),
            LogoutDecision::Reconnect
        );
        assert!(seen.lock().unwrap().is_empty(), "440 = sesi sah");
        assert!(dir.join("oktz-run.db").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pairing_pending_401_resets_creds_and_leaves_the_session_alone() {
        let dir = scratch("resetpairing");
        seed(&dir);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let plan = PurgePlan::new(Some(dir.join("oktz-run.db")), Some(dir.join("creds.json")));
        let mut c = authed();
        pairing::mark_pairing_pending(&mut c);
        let guard = FalseLogoutGuard::new(c).with_purge_handler(Arc::new(move |a| {
            let _ = purge_session(&plan, 1_700_000_000_000);
            sink.lock().unwrap().push(a);
        }));
        let t0 = 1_000_000;
        guard.mark_connected(t0);
        assert_eq!(
            guard.handle_logged_out(&stream_error("401"), t0 + FALSE_LOGOUT_GRACE_MS + 1),
            LogoutDecision::Apply(CloseAction::ResetPairing)
        );
        assert!(seen.lock().unwrap().is_empty(), "reset-pairing tidak boleh purge");
        assert!(dir.join("oktz-run.db").exists(), "src/connection.js:1006-1007");
        assert!(dir.join("creds.json").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_installed_handler_ignores_actions_that_are_not_a_purge() {
        let dir = scratch("handler-guard");
        seed(&dir);
        let h = purge_handler(PurgePlan::new(
            Some(dir.join("oktz-run.db")),
            Some(dir.join("creds.json")),
        ));
        for a in [CloseAction::Reconnect, CloseAction::ResetPairing] {
            h(a);
            assert!(dir.join("oktz-run.db").exists(), "{a:?} tidak boleh menghapus apa pun");
            assert!(dir.join("creds.json").exists(), "{a:?}");
        }
        h(CloseAction::PurgeSession);
        assert!(!dir.join("oktz-run.db").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---------- purge_session: artefak yang benar-benar tersentuh ----------

    #[test]
    fn prune_runs_before_the_rename_so_the_previous_backup_survives() {
        let dir = scratch("prune");
        seed(&dir);
        let now = 1_700_000_000_000u64;
        // Empat karantina lama; `keep` = 3 jadi yang tertua yang dibuang.
        // Kalau prune dilakukan SESUDAH rename, karantina yang baru ikut
        // terhitung dan bot kehilangan satu-satunya salinan sesi lama.
        for ts in [100u64, 200, 300, 400] {
            let d = dir.join(format!("oktz-run.db.broken-{ts}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("oktz-run.db"), "x").unwrap();
            std::fs::write(d.join("oktz-run.db-wal"), "w").unwrap();
        }

        let r = purge_session(
            &PurgePlan::new(Some(dir.join("oktz-run.db")), Some(dir.join("creds.json"))),
            now,
        );
        assert_eq!(r.pruned, 1);
        assert!(!dir.join("oktz-run.db.broken-100").exists(), "tertua dibuang");
        for ts in [200u64, 300, 400] {
            assert!(dir.join(format!("oktz-run.db.broken-{ts}")).exists(), "ts {ts} harus bertahan");
        }
        // Karantina yang selamat tetap utuh: db DAN sidecar-nya sepadan.
        for ts in [200u64, 300, 400] {
            let d = dir.join(format!("oktz-run.db.broken-{ts}"));
            assert!(d.join("oktz-run.db").exists(), "db ts {ts}");
            assert!(d.join("oktz-run.db-wal").exists(), "sidecar ts {ts} ikut, bukan dibuang");
        }
        assert!(dir.join(format!("oktz-run.db.broken-{now}")).is_dir());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn every_sidecar_rides_inside_the_quarantine_directory() {
        // Kalau sidecar dipipihkan sebagai `<db>.broken-<now>-wal`, ia jadi
        // entri prune tersendiri dan bisa dibuang while db-nya disimpan —
        // frame yang belum checkpoint hilang dari karantina yang tersisa.
        let dir = scratch("sidecars");
        seed(&dir);
        for s in ["-wal", "-shm", "-journal"] {
            std::fs::write(format!("{}{s}", dir.join("oktz-run.db").display()), b"x").unwrap();
        }
        let now = 1_700_000_000_000u64;
        let r = purge_session(&PurgePlan::new(Some(dir.join("oktz-run.db")), None), now);
        assert_eq!(r.quarantined_sidecars.len(), 3);
        let q = dir.join(format!("oktz-run.db.broken-{now}"));
        for s in ["-wal", "-shm", "-journal"] {
            assert!(q.join(format!("oktz-run.db{s}")).exists(), "sidecar {s}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_store_skips_prune_and_leaves_the_existing_backups_alone() {
        // JS: `if (fs.existsSync(sessionPath))` — tanpa store tidak ada yang
        // dipurge, dan prune tanpa rename hanya membuang karantina orang lain.
        let dir = scratch("no-store");
        std::fs::create_dir_all(dir.join("oktz-run.db.broken-100")).unwrap();
        std::fs::create_dir_all(dir.join("oktz-run.db.broken-200")).unwrap();
        let r = purge_session(
            &PurgePlan::new(Some(dir.join("tidak-ada.db")), None),
            1_700_000_000_000,
        );
        assert_eq!(r.pruned, 0);
        assert_eq!(r.quarantined_store, None);
        assert!(dir.join("oktz-run.db.broken-100").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn creds_are_quarantined_even_when_the_store_is_already_gone() {
        // Tidak seperti JS (creds ada DI DALAM folder sesi), di Rust creds
        // berdiri sendiri. Membiarkannya = `prepare_store` membangun device row
        // dari identitas yang persis ditolak WA, dan 401-nya berulang.
        let dir = scratch("creds-only");
        std::fs::write(dir.join("creds.json"), r#"{"registered":true}"#).unwrap();
        let now = 1_700_000_000_000u64;
        let r = purge_session(&PurgePlan::new(None, Some(dir.join("creds.json"))), now);
        assert_eq!(r.quarantined_store, None);
        assert_eq!(
            r.quarantined_creds,
            Some(dir.join(format!("creds.json.broken-{now}")))
        );
        assert!(!dir.join("creds.json").exists());
        assert!(r.errors.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_cooldown_marker_is_never_inside_the_purge_scope() {
        // src/connection.js:624-627 — cooldown sengaja di luar `sessionPath`
        // supaya purge tidak jadi cara untuk melewati cooldown.
        let dir = scratch("cooldown");
        seed(&dir);
        std::fs::write(dir.join(".pairing-cooldown"), r#"{"until":123}"#).unwrap();
        purge_session(
            &PurgePlan::new(Some(dir.join("oktz-run.db")), Some(dir.join("creds.json"))),
            1_700_000_000_000,
        );
        assert!(dir.join(".pairing-cooldown").exists(), "cooldown harus utuh");
        assert_eq!(pairing::read_cooldown(&dir), 123);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn purging_nothing_is_an_empty_report_not_an_error() {
        let dir = scratch("empty");
        let r = purge_session(
            &PurgePlan::new(Some(dir.join("nope.db")), Some(dir.join("nope.json"))),
            1_700_000_000_000,
        );
        assert_eq!(r, PurgeReport::default());
    }

    #[test]
    fn an_unwritable_target_is_reported_instead_of_silently_dropped() {
        // Artefak yang masih ada lebih berguna daripada artefak yang hilang
        // diam-diam, jadi rename gagal = error tercatat, bukan dihapus.
        let dir = scratch("readonly");
        seed(&dir);
        let ro = dir.join("ro");
        std::fs::create_dir_all(&ro).unwrap();
        let db = ro.join("oktz-run.db");
        std::fs::write(&db, b"sqlite").unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o500)).unwrap();

        let r = purge_session(&PurgePlan::new(Some(db.clone()), None), 1_700_000_000_000);
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(r.quarantined_store, None, "tidak boleh pindah");
        assert!(db.exists(), "file asli harus masih ada");
        assert_eq!(r.errors.len(), 1, "{}", r.errors.join(" | "));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Path yang dipakai dua tes yang menyentuh `LAST_STORE`. `cargo test`
/// menjalankan tes dalam satu proses secara paralel, jadi keduanya harus
/// mencatat path yang sama — kalau tidak, assertions saling menimpa.
const SHARED_STORE: &str = "oktz-run.db";

#[test]
    fn the_remembered_store_path_round_trips() {
        // `prepare_store` mencatatnya; ini yang membuat purge bekerja tanpa
        // `SqliteStore` mengekspos path-nya.
        let dir = scratch("remember");
        note_store_path(&dir.join(SHARED_STORE));
        assert_eq!(remembered_store_path(), Some(dir.join(SHARED_STORE)));
        assert_eq!(PurgePlan::resolve().keep, pairing::BROKEN_SESSION_KEEP);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_empty_session_db_env_means_absent_not_empty_path() {
        let dir = scratch("env");
        note_store_path(&dir.join(SHARED_STORE));
        unsafe { std::env::set_var(SESSION_DB_ENV, "   ") };
        let plan = PurgePlan::resolve();
        unsafe { std::env::remove_var(SESSION_DB_ENV) };
        assert_eq!(
            plan.store_db,
            Some(dir.join(SHARED_STORE)),
            "env kosong harus jatuh ke store yang dicatat, bukan jadi path kosong"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
