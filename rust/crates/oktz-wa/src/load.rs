// Muat 538 sesi onigis dari dump Turso ke SQLite store sebelum connect,
// supaya pesan lama/offline-queue bisa decrypt (tanpa ini: "Bad Mac" semua).
use anyhow::Result;
use wacore_libsignal::core::curve::PublicKey;
use wacore_libsignal::protocol::{IdentityKey, LegacySessionLocalContext, LegacySessionRecordV1};
use whatsapp_rust::wacore::store::traits::SignalStore;

use crate::session::{ImportStats, import_session_record};

#[derive(Default)]
pub struct LoadReport {
    pub sessions_loaded: usize,
    pub sessions_failed: usize,
    pub records_failed: usize,
    pub skipped_unknown_basekeyrole: usize,
    pub write_failed: usize,
    pub first_error: Option<String>,
}

/// onigis menyimpan id sesi sebagai `<addr>_<n>.<device>` dengan `addr` berupa
/// nomor polos (tanpa @server). wacore mencari key `<user>[:device]@<server>` lalu
/// append `.0` — lihat wacore::types::jid::write_signal_address_to. Tanpa
/// menambah @server, sesi tersimpan di key yang tidak pernah dibaca.
fn to_address(onigis_id: &str) -> String {
    let base = onigis_id.split('_').next().unwrap_or(onigis_id);
    let num = base.split('@').next().unwrap_or(base);
    if num.contains('@') {
        return format!("{base}.0");
    }
    format!("{num}@s.whatsapp.net.0")
}

pub async fn load_into_store<S>(
    store: &S,
    fixture: &str,
    our_identity: &[u8],
    our_registration: u32,
) -> Result<LoadReport, anyhow::Error>
where
    S: SignalStore + ?Sized,
{
    let raw = std::fs::read_to_string(fixture).unwrap_or_default();
    let Ok(dump) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Err(anyhow::anyhow!("fixture sesi tidak terbaca: {fixture}"));
    };
    let rows = dump
        .get("session")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let pk = match PublicKey::try_from(our_identity) {
        Ok(k) => k,
        Err(e) => return Err(anyhow::anyhow!("identity key: {e}")),
    };
    let identity = IdentityKey::new(pk);
    let ctx = LegacySessionLocalContext {
        identity_key: identity,
        registration_id: our_registration,
    };

    let mut batch: Vec<(std::sync::Arc<str>, bytes::Bytes)> = Vec::new();
    let mut rep = LoadReport::default();

    for row in &rows {
        let Some(id) = row.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(data) = row.get("data").and_then(|v| v.as_str()) else {
            continue;
        };

        let mut stats = ImportStats::default();
        let legacy: LegacySessionRecordV1 = match import_session_record(data, &mut stats) {
            Ok(r) => r,
            Err(_) => {
                rep.records_failed += 1;
                continue;
            }
        };
        rep.skipped_unknown_basekeyrole += stats.skipped_unknown_basekeyrole;

        let record = match legacy.into_session_record(ctx) {
            Ok(r) => r,
            Err(e) => {
                rep.sessions_failed += 1;
                if rep.first_error.is_none() {
                    rep.first_error = Some(format!("{e:?}"));
                }
                continue;
            }
        };
        let bytes = match record.serialize() {
            Ok(b) => b,
            Err(_) => {
                rep.sessions_failed += 1;
                continue;
            }
        };

        batch.push((
            std::sync::Arc::from(to_address(id).as_str()),
            bytes::Bytes::from(bytes),
        ));
    }

    // `sessions_loaded` dihitung SETELAH persist sukses; kalau batch write gagal,
    // jangan dilaporkan sebagai termuat.
    let mut persisted = 0usize;
    let mut write_failed = 0usize;
    for chunk in batch.chunks(64) {
        match store.put_sessions_batch(chunk).await {
            Ok(()) => persisted += chunk.len(),
            Err(e) => {
                write_failed += chunk.len();
                if rep.first_error.is_none() {
                    rep.first_error = Some(format!("write batch gagal: {e}"));
                }
            }
        }
    }
    rep.sessions_loaded = persisted;
    rep.write_failed = write_failed;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bug yang ketahuan review: loader menulis key yang tidak pernah dibaca
    // wacore, jadi impor "sukses" tapi tidak berguna.
    #[test]
    fn address_matches_wacore_protocol_address() {
        assert_eq!(to_address("240406902128794_1.0"), "240406902128794@s.whatsapp.net.0");
    }

    #[test]
    fn address_keeps_server_when_present() {
        assert_eq!(to_address("628123@s.whatsapp.net_1.0"), "628123@s.whatsapp.net.0");
    }

    #[test]
    fn lid_address_uses_same_server_suffix() {
        assert_eq!(to_address("188412430651393_1.0"), "188412430651393@s.whatsapp.net.0");
    }

}
