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

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

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
                rearm_after_grace(Arc::clone(client));
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
                // Flag sudah di-clear oleh `handle_stream_error`; store ulang
                // hanya memastikan tidak ada jalur implisit yang menghidupkannya.
                client.enable_auto_reconnect.store(false, Ordering::Relaxed);
                LogoutDecision::Apply(action)
            }
        }
    }
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

/// `connect_device` yang memasang [`FalseLogoutGuard`].
///
/// Sama dengan `crate::bot::connect_device` (yang tidak bisa disentuh di sini —
/// pemiliknya agent lain), plus handler `Connected`/`LoggedOut` dari guard.
/// Dipakai untuk setiap sesi yang benar-benar ter-authenticate; tanpa itu,
/// `pairing::classify_logout_401` tidak akan pernah dipanggil.
pub async fn connect_device_guarded<F, Fut>(
    store: SqliteStore,
    creds: Value,
    on_ready: F,
) -> anyhow::Result<(whatsapp_rust::prelude::BotHandle, Arc<FalseLogoutGuard>)>
where
    F: Fn(Arc<Client>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let guard = Arc::new(FalseLogoutGuard::new(creds));
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
}
