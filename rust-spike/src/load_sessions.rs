// Muat 538 sesi onigis dari dump Turso ke SQLite store sebelum connect,
// supaya pesan lama/offline-queue bisa decrypt (tanpa ini: "Bad Mac" semua).
use whatsapp_rust::wacore::store::traits::SignalStore;
use anyhow::Result;
use wacore_libsignal::core::curve::PublicKey;
use wacore_libsignal::protocol::{IdentityKey, LegacySessionLocalContext, LegacySessionRecordV1};

use crate::onigis_session::{import_session_record, ImportStats};

#[derive(Default)]
pub struct LoadReport {
    pub sessions_loaded: usize,
    pub sessions_failed: usize,
    pub records_failed: usize,
    pub skipped_unknown_basekeyrole: usize,
}

/// Address key yang dipakai wacore: `<jid>.<device>` (lihat log run S1:
/// "message from 32328738959591@lid.0")..onigis id = `<jid>_<device>`.
fn to_address(onigis_id: &str) -> String {
    onigis_id.replacen('_', ".", 1)
}

pub async fn load_into_store<S>(store: &S, fixture: &str, our_identity: &[u8], our_registration: u32) -> Result<LoadReport, anyhow::Error>
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

    let pk = match PublicKey::try_from(our_identity) { Ok(k) => k, Err(e) => return Err(anyhow::anyhow!("identity key: {e}")) };
    let identity = IdentityKey::new(pk);
    let ctx = LegacySessionLocalContext {
        identity_key: identity,
        registration_id: our_registration,
    };

    let mut batch: Vec<(std::sync::Arc<str>, bytes::Bytes)> = Vec::new();
    let mut rep = LoadReport::default();

    for row in &rows {
        let Some(id) = row.get("id").and_then(|v| v.as_str()) else { continue };
        let Some(data) = row.get("data").and_then(|v| v.as_str()) else { continue };

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
            Err(_) => {
                rep.sessions_failed += 1;
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

        batch.push((std::sync::Arc::from(to_address(id).as_str()), bytes::Bytes::from(bytes)));
        rep.sessions_loaded += 1;
    }

    // Batch dalam chunks supaya tidak menumpuk semuanya di memori.
    for chunk in batch.chunks(64) {
        if let Err(e) = store.put_sessions_batch(chunk).await {
            eprintln!("gagal menyimpan batch sesi: {e}");
        }
    }
    Ok(rep)
}