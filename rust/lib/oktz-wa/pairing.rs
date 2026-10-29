//! Logika pairing 1:1 dengan `src/lib/pairing-state.js`.
//! WA mengirim 401 palsu sesaat setelah handshake — percaya langsung = hapus
//! sesi sah. Referensi: zaileys retry 2x sebelum believe a logout.

use serde_json::Value;
use std::path::Path;

pub const FALSE_LOGOUT_GRACE_MS: u64 = 60 * 1000;
pub const FALSE_LOGOUT_RETRIES: u32 = 2;
pub const BROKEN_SESSION_KEEP: usize = 3;

const COOLDOWN_FILE: &str = ".pairing-cooldown";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    ResetPairing,
    PurgeSession,
    Reconnect,
}

pub fn is_pairing_pending(creds: &Value) -> bool {
    creds
        .get("additionalData")
        .and_then(|d| d.get("pairingPending"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn mark_pairing_pending(creds: &mut Value) {
    if !creds.get("additionalData").is_some_and(Value::is_object) {
        creds["additionalData"] = Value::Object(serde_json::Map::new());
    }
    creds["additionalData"]["pairingPending"] = Value::Bool(true);
}

pub fn clear_pairing_pending(creds: &mut Value) {
    if let Some(d) = creds
        .get_mut("additionalData")
        .and_then(Value::as_object_mut)
    {
        d.remove("pairingPending");
    }
}

pub fn reset_pairing_creds(creds: &mut Value) {
    if let Some(m) = creds.as_object_mut() {
        for k in ["me", "pairingCode", "account"] {
            m.remove(k);
        }
        // JS menulis `creds.signalIdentities = []` tanpa syarat
        // (src/lib/pairing-state.js:32). Hanya mengosongkan array yang sudah
        // ada berarti creds hasil pairing/migrasi yang belum punya field ini
        // tetap tanpa identitas — dan identitas lama masih terbaca.
        m.insert("signalIdentities".into(), Value::Array(Vec::new()));
        m.insert("registered".into(), Value::Bool(false));
    }
    clear_pairing_pending(creds);
}

/// `account` ada (QR) atau `registered` true (kode) =_device pernah ter-link.
pub fn is_authenticated(creds: &Value) -> bool {
    creds.get("account").is_some_and(|v| !v.is_null())
        || creds
            .get("registered")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

pub fn classify_close(status_code: u16, creds: &Value) -> CloseAction {
    if status_code == 401 {
        if is_pairing_pending(creds) {
            return CloseAction::ResetPairing;
        }
        return if is_authenticated(creds) {
            CloseAction::PurgeSession
        } else {
            CloseAction::ResetPairing
        };
    }
    CloseAction::Reconnect
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutTrust {
    Retry,
    Trust,
}

pub fn classify_logout_401(
    status_code: u16,
    creds: &Value,
    connected_at: Option<u64>,
    now: u64,
    retries: u32,
) -> LogoutTrust {
    if status_code != 401 {
        return LogoutTrust::Trust;
    }
    if !is_authenticated(creds) {
        return LogoutTrust::Trust;
    }
    if let Some(t) = connected_at
        && now.saturating_sub(t) < FALSE_LOGOUT_GRACE_MS
        && retries < FALSE_LOGOUT_RETRIES
    {
        return LogoutTrust::Retry;
    }
    LogoutTrust::Trust
}

/// Port `isRateLimitError` (src/lib/pairing-state.js:104). JS membaca EMPAT
/// sumber; sebelumnya Rust hanya mencocokkan teks message, jadi 429 yang
/// datang lewat `output.statusCode` (bentuk yang biasa dikirim WA/Baileys)
/// lolos dan bot retry tanpa cooldown.
pub fn is_rate_limit_error(err: &Value) -> bool {
    if err.is_null() {
        return false;
    }
    // (1) message berisi "rate-overlimit" atau "429"
    let msg = err.get("message").and_then(Value::as_str).unwrap_or("");
    if msg.contains("rate-overlimit") || msg.contains("429") {
        return true;
    }
    // (2) error.output.statusCode  (3) error.data  (4) error.data.statusCode
    ["/output/statusCode", "/data", "/data/statusCode"]
        .iter()
        .any(|p| err.pointer(p).and_then(Value::as_i64) == Some(429))
}

fn cooldown_path(dir: &Path) -> std::path::PathBuf {
    dir.join(COOLDOWN_FILE)
}

pub fn read_cooldown(dir: &Path) -> u64 {
    std::fs::read_to_string(cooldown_path(dir))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.get("until").and_then(Value::as_u64))
        .unwrap_or(0)
}

pub fn write_cooldown(dir: &Path, until: u64) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(cooldown_path(dir), format!("{{\"until\":{until}}}"))
}

pub fn clear_cooldown(dir: &Path) {
    let _ = std::fs::remove_file(cooldown_path(dir));
}

pub fn cooldown_remaining_ms(dir: &Path, now: u64) -> u64 {
    let until = read_cooldown(dir);
    if until == 0 || until <= now {
        clear_cooldown(dir);
        return 0;
    }
    until - now
}

/// Buang folder `*.broken-*` paling lama, sisakan `keep`.
pub fn prune_broken_sessions(storage_root: &Path, session_base: &str, keep: usize) -> usize {
    let prefix = format!("{session_base}.broken-");
    let Ok(rd) = std::fs::read_dir(storage_root) else {
        return 0;
    };
    let mut hits: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(&prefix))
        .collect();
    hits.sort();
    let mut removed = 0;
    while hits.len() > keep {
        let victim = hits.remove(0);
        if std::fs::remove_dir_all(storage_root.join(&victim)).is_ok() {
            removed += 1;
        } else {
            let _ = std::fs::remove_file(storage_root.join(&victim));
            removed += 1;
        }
    }
    removed
}

pub fn normalize_pairing_number(input: &str) -> Option<String> {
    let d: String = input.chars().filter(char::is_ascii_digit).collect();
    (!d.is_empty()).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn authed() -> Value {
        json!({"account": {"details": "x"}, "registered": true, "additionalData": {}})
    }

    #[test]
    fn pending_pairing_401_resets_not_purges() {
        let mut c = authed();
        mark_pairing_pending(&mut c);
        assert!(is_pairing_pending(&c));
        assert_eq!(classify_close(401, &c), CloseAction::ResetPairing);
    }

    #[test]
    fn authed_401_purges() {
        assert_eq!(classify_close(401, &authed()), CloseAction::PurgeSession);
    }

    #[test]
    fn unauthed_401_resets() {
        let c = json!({"additionalData": {}});
        assert_eq!(classify_close(401, &c), CloseAction::ResetPairing);
    }

    #[test]
    fn non_401_reconnects() {
        assert_eq!(classify_close(440, &authed()), CloseAction::Reconnect);
    }

    #[test]
    fn fresh_401_within_grace_is_retried() {
        let t0 = 1_000_000;
        assert_eq!(
            classify_logout_401(401, &authed(), Some(t0), t0 + 5_000, 0),
            LogoutTrust::Retry
        );
    }

    #[test]
    fn retries_exhausted_becomes_trust() {
        let t0 = 1_000_000;
        assert_eq!(
            classify_logout_401(401, &authed(), Some(t0), t0 + 1_000, 2),
            LogoutTrust::Trust
        );
    }

    #[test]
    fn grace_expired_is_trust() {
        let t0 = 1_000_000;
        assert_eq!(
            classify_logout_401(401, &authed(), Some(t0), t0 + FALSE_LOGOUT_GRACE_MS + 1, 0),
            LogoutTrust::Trust
        );
    }

    #[test]
    fn reset_clears_identity_and_flag() {
        let mut c = authed();
        mark_pairing_pending(&mut c);
        reset_pairing_creds(&mut c);
        assert!(!is_pairing_pending(&c));
        assert!(!is_authenticated(&c));
        assert!(c.get("me").is_none());
    }

    #[test]
    fn reset_creates_signal_identities_when_absent() {
        // creds hasil pairing/migrasi belum tentu punya `signalIdentities`.
        // JS (src/lib/pairing-state.js:32) menulis `creds.signalIdentities = []`
        // tanpa syarat ada/tidaknya field — kalau tidak dibuat, identitas
        // Signal yang lama masih terbaca dan tidak dibangun ulang.
        let mut c = json!({"registered": true, "me": {"id": "6285:1@s.whatsapp.net"}});
        assert!(
            c.get("signalIdentities").is_none(),
            "fixture memang tidak punya field"
        );
        reset_pairing_creds(&mut c);
        assert_eq!(
            c.get("signalIdentities"),
            Some(&json!([])),
            "reset harus selalu menghasilkan signalIdentities = []"
        );
    }

    #[test]
    fn reset_clears_existing_signal_identities() {
        let mut c = json!({"signalIdentities": [
            {"identifier": {"name": "6285:1@s.whatsapp.net"}, "accountSignatureKey": "AAA"}
        ]});
        reset_pairing_creds(&mut c);
        assert_eq!(c.get("signalIdentities"), Some(&json!([])));
    }

    #[test]
    fn rate_limit_detected() {
        // Empat assert yang sama seperti sebelumnya, sekarang lewat bentuk
        // objek error yang sama dengan JS.
        assert!(is_rate_limit_error(&json!({"message": "rate-overlimit"})));
        assert!(is_rate_limit_error(&json!({"message": "boom 429"})));
        assert!(is_rate_limit_error(&json!({"message": "x", "output": {"statusCode": 429}})));
        assert!(!is_rate_limit_error(&json!({"message": "ok", "output": {"statusCode": 200}})));
    }

    #[test]
    fn rate_limit_from_message_text() {
        assert!(is_rate_limit_error(&json!({"message": "rate-overlimit"})));
        assert!(is_rate_limit_error(&json!({"message": "boom 429"})));
    }

    #[test]
    fn rate_limit_from_output_status_code() {
        // `error.output.statusCode` — bentuk yang dikirim WA/Baileys saat
        // rate limit, tanpa teks "429" di message.
        assert!(is_rate_limit_error(
            &json!({"message": "send failed", "output": {"statusCode": 429}})
        ));
    }

    #[test]
    fn rate_limit_from_data_number() {
        assert!(is_rate_limit_error(&json!({"message": "send failed", "data": 429})));
    }

    #[test]
    fn rate_limit_from_data_status_code() {
        assert!(is_rate_limit_error(
            &json!({"message": "send failed", "data": {"statusCode": 429}})
        ));
    }

    #[test]
    fn not_rate_limit_without_any_signal() {
        assert!(!is_rate_limit_error(&json!({"message": "connection reset"})));
        assert!(!is_rate_limit_error(&json!({"output": {"statusCode": 500}})));
        assert!(!is_rate_limit_error(&json!({"data": {"statusCode": 500}})));
        assert!(!is_rate_limit_error(&json!({})));
        assert!(!is_rate_limit_error(&Value::Null));
    }

    #[test]
    fn cooldown_roundtrip() {
        let dir = std::env::temp_dir().join("oktz-pairing-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_cooldown(&dir, 5_000).unwrap();
        assert_eq!(read_cooldown(&dir), 5_000);
        assert_eq!(cooldown_remaining_ms(&dir, 1_000), 4_000);
        assert_eq!(
            cooldown_remaining_ms(&dir, 9_000),
            0,
            "kedaluwarsa dibersihkan"
        );
        assert!(!dir.join(COOLDOWN_FILE).exists());
    }

    #[test]
    fn normalize_number() {
        assert_eq!(
            normalize_pairing_number("+62 856-0895-3677"),
            Some("6285608953677".into())
        );
        assert_eq!(normalize_pairing_number("abc"), None);
    }

    #[test]
    fn prune_keeps_newest() {
        let dir = std::env::temp_dir().join("oktz-broken-test");
        let _ = std::fs::remove_dir_all(&dir);
        for i in 1..=5 {
            std::fs::create_dir_all(dir.join(format!("sess.broken-{i}"))).unwrap();
        }
        let removed = prune_broken_sessions(&dir, "sess", BROKEN_SESSION_KEEP);
        assert_eq!(removed, 2);
        let left = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(left, BROKEN_SESSION_KEEP);
    }
}
