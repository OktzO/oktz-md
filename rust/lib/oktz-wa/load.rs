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

/// Terjemahkan id sesi onigis ke protocol address wacore.
///
/// onigis (`Onigi/lib/Signal/libsignal.js:384`) membangun address sebagai
/// `ProtocolAddress(signalUser, device).toString()`, dengan
/// `signalUser = domainType !== WHATSAPP ? "{user}_{domainType}" : user`.
/// Jadi suffix `_N` BUKAN device — itu penanda domain (WAJIDDomains: 0=PN,
/// 1=LID, 128=hosted, 129=hosted.lid). Field setelah titik terakhir adalah
/// device, di-{0} menjadi suffix `.0` oleh `ProtocolAddress::toString`.
///
/// wacore (`types::jid.rs:79` `write_signal_address_to`) mencari
/// `{user}[:{device}]@{server}` lalu append `.0`, dengan `mapped_server`
/// menerjemahkan `s.whatsapp.net` -> `c.us`. Tanpa terjemahan ini, sesi
/// tersimpan di key yang tidak pernah dibaca wacore.
fn to_address(onigis_id: &str) -> String {
    let (name, device) = match onigis_id.rsplit_once('.') {
        Some((name, device)) => (name, device.parse::<u16>().unwrap_or(0)),
        None => (onigis_id, 0),
    };
    // `rsplit_once` aman: user onigis hanya digit, jadi `_` terakhir selalu
    // pemisah domain — sama seperti `split('_')` di sisi JS.
    let (user, domain) = match name.rsplit_once('_') {
        Some((user, domain)) => (user, domain.parse::<u32>().unwrap_or(0)),
        None => (name, 0),
    };
    let server = match domain {
        1 => "lid",
        128 => "hosted",
        129 => "hosted.lid",
        _ => "c.us",
    };
    if device == 0 {
        format!("{user}@{server}.0")
    } else {
        format!("{user}:{device}@{server}.0")
    }
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
    //
    // Fixtures nyata: `247957890850846_1.0` ada di turso_session_keys.json dan
    // wacore sendiri menulis `247957890850846@lid.0` (lihat `identities` di
    // oktz-smoke.db) — jadi `_1` harus jadi `@lid`, bukan PN.
    #[test]
    fn lid_session_maps_to_lid_server() {
        assert_eq!(to_address("247957890850846_1.0"), "247957890850846@lid.0");
    }

    #[test]
    fn pn_session_maps_to_legacy_c_us() {
        // `mapped_server` di wacore menerjemahkan s.whatsapp.net -> c.us, jadi
        // `@s.whatsapp.net` TIDAK PERNAH dibaca. Lihat jid.rs:16.
        assert_eq!(to_address("628123456789_0.0"), "628123456789@c.us.0");
        assert_eq!(to_address("628123456789.0"), "628123456789@c.us.0");
    }

    #[test]
    fn device_suffix_is_kept_and_moves_into_the_name() {
        // onigis `240406902128794_1.80`: user, domain LID, device 80.
        assert_eq!(
            to_address("240406902128794_1.80"),
            "240406902128794:80@lid.0"
        );
    }

    #[test]
    fn hosted_domains_keep_their_own_server() {
        assert_eq!(to_address("123_128.0"), "123@hosted.0");
        assert_eq!(to_address("123_129.0"), "123@hosted.lid.0");
    }

    /// Bentuk id yang benar-benar muncul di fixture onigis: device 0 (452
    /// dari 514) plus device 1..88 pada PN maupun LID.
    #[test]
    fn every_real_fixture_id_shape_maps_to_a_readable_address() {
        for (user, domain, device) in [
            ("247957890850846", 1, 0),
            ("240406902128794", 1, 80),
            ("86222139600981", 1, 80),
            ("188412430651393", 1, 54),
            ("628123456789", 0, 0),
            ("628123456789", 0, 33),
            ("120363428677398", 1, 12),
        ] {
            let id = format!("{user}_{domain}.{device}");
            let address = to_address(&id);
            assert!(
                address.ends_with(".0") && address.contains('@'),
                "address tidak pernah dibaca wacore: {id} -> {address}"
            );
            let expected = if device == 0 {
                format!("{user}@{}.0", if domain == 1 { "lid" } else { "c.us" })
            } else {
                format!("{user}:{device}@{}.0", if domain == 1 { "lid" } else { "c.us" })
            };
            assert_eq!(address, expected, "id: {id}");
        }
    }
}
