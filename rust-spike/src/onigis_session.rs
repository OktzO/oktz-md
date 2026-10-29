// Importer sesi onigis/Baileys -> SessionRecord wacore (whatsapp-rust).
//
// Data sumber (Turso `session_keys` scope `main` + `storage/session/*.json`).
// Semua field kripto disimpan sebagai base64 string di JSON — BUKAN binary
// protobuf. `LegacySessionV1` di wacore-libsignal memetakan 1:1 ke JSON ini,
// jadi tidak ada masalah versi protobuf: kita rekonstruksi dari crypto material.
//
// Peta onigis JSON -> LegacySessionV1:
//   registrationId                         -> registration_id
//   currentRatchet.ephemeralKeyPair.pubKey  -> ratchet.key_pair.public
//   currentRatchet.ephemeralKeyPair.privKey  -> ratchet.key_pair.private
//   currentRatchet.lastRemoteEphemeralKey   -> ratchet.last_remote_ephemeral_key
//   currentRatchet.previousCounter          -> ratchet.previous_counter
//   currentRatchet.rootKey                  -> ratchet.root_key
//   indexInfo.baseKey                       -> index.base_key
//   indexInfo.baseKeyType                   -> index.base_key_role   (JIKA 0/3/4!)
//   indexInfo.closed                        -> index.disposition     (-1=Current, >=0=Archived)
//   indexInfo.used                          -> index.used_at_ms
//   indexInfo.created                       -> index.created_at_ms
//   indexInfo.remoteIdentityKey             -> index.remote_identity_key
//   _chains.<ratchetKey>.chainType          -> chains[].role          (1=Sending,2=Receiving;0=PREKEY?)
//   _chains.<ratchetKey>.chainKey.counter   -> chains[].chain_key.counter
//   _chains.<ratchetKey>.chainKey.key       -> chains[].chain_key.key
//   _chains.<ratchetKey>.messageKeys        -> chains[].message_keys
//
// CATATAN EDGE CASE nyata di data Onigi (lihat REPORT):
//   - `baseKeyType` bisa 0 (UNKNOWN), 3 (BOUND_TO_PEER), 4 (BOUND_TO_SELF),
//     sedangkan LegacySessionBaseKeyRoleV1 cuma menerima 1 (Local) / 2 (Remote).
//   - `chainType` bisa 0 (PREKEY) yang tidak punya padanan di chain role V1
//     (cuma Sending/Receiving).

use std::collections::HashMap;

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Deserialize;
use wacore_libsignal::protocol::{
    LegacyIndexedSessionV1, LegacySessionBaseKeyRoleV1, LegacySessionChainCounterV1,
    LegacySessionChainKeyV1, LegacySessionChainRoleV1, LegacySessionChainV1,
    LegacySessionDispositionV1, LegacySessionIndexV1, LegacySessionKeyPairV1,
    LegacySessionMessageKeyV1, LegacySessionRatchetV1, LegacySessionRecordV1, LegacySessionV1,
};

// ---------- Bentuk JSON onigis (cukup untuk parse; abaikan field lain) ----------

#[derive(Debug, Default, Deserialize)]
struct OnigisRoot {
    #[serde(default)]
    _sessions: HashMap<String, OnigisSession>,
    #[serde(default)]
    _version: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct OnigisSession {
    #[serde(rename = "registrationId")]
    registration_id: u32,
    #[serde(rename = "currentRatchet")]
    current_ratchet: OnigisRatchet,
    #[serde(rename = "indexInfo")]
    index_info: OnigisIndex,
    #[serde(default)]
    _chains: HashMap<String, OnigisChain>,
}

#[derive(Debug, Default, Deserialize)]
struct OnigisRatchet {
    #[serde(rename = "ephemeralKeyPair")]
    ephemeral_key_pair: OnigisKeyPair,
    #[serde(rename = "lastRemoteEphemeralKey")]
    last_remote_ephemeral_key: String,
    #[serde(rename = "previousCounter")]
    previous_counter: i64,
    #[serde(rename = "rootKey")]
    root_key: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct OnigisKeyPair {
    #[serde(rename = "pubKey")]
    pub_key: String,
    #[serde(rename = "privKey")]
    priv_key: String,
}

#[derive(Debug, Default, Deserialize)]
struct OnigisIndex {
    #[serde(rename = "baseKey")]
    base_key: String,
    #[serde(rename = "baseKeyType")]
    base_key_type: i64,
    #[serde(rename = "closed")]
    closed: i64,
    #[serde(rename = "used")]
    used: u64,
    #[serde(rename = "created")]
    created: u64,
    #[serde(rename = "remoteIdentityKey")]
    remote_identity_key: String,
}

#[derive(Debug, Default, Deserialize)]
struct OnigisChain {
    #[serde(rename = "chainKey")]
    chain_key: OnigisChainKey,
    #[serde(rename = "chainType")]
    chain_type: i64,
    #[serde(default)]
    #[serde(rename = "messageKeys")]
    message_keys: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct OnigisChainKey {
    counter: i64,
    key: Option<String>,
}

// ---------- BufferJSON (yang muncul di device-list/pre-key/tctoken) ----------

#[derive(Debug, Default, Deserialize)]
pub struct BufferJson {
    #[serde(rename = "type")]
    kind: String,
    data: String, // base64
}

impl BufferJson {
    pub fn decode(&self) -> Result<Bytes> {
        anyhow::ensure!(self.kind == "Buffer", "bukan BufferJSON: {}", self.kind);
        base64_decode(&self.data)
    }
}

/// Decode base64 toleran whitespace (beberapa nilai onigis beregak/newline).
// onigis simpan public key Curve25519 32 byte polos; wacore/libsignal
// butuh bentuk kanonik 33 byte = 0x05 (KeyType::Djb) + 32.
const DJB_PREFIX: u8 = 0x05;

fn pub_key_b64(s: &str) -> Result<Bytes> {
    let mut v: Vec<u8> = base64_decode(s)?.to_vec();
    if v.len() == 32 {
        v.insert(0, DJB_PREFIX);
    }
    Ok(Bytes::from(v))
}

fn base64_decode(s: &str) -> Result<Bytes> {
    use base64::Engine;
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let v = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .context("base64 decode gagal")?;
    Ok(Bytes::from(v))
}

// ---------- Statistik & error mapping ----------

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ImportStats {
    pub total_sessions: usize,
    pub imported_sessions: usize,
    pub skipped_prekey_chains: usize,
    pub skipped_unknown_basekeyrole: usize,
    pub skipped_other: usize,
    pub errors: Vec<String>,
}

// Politika `baseKeyType`: onigis pakai enum KeyType Baileys
// (0=UNKNOWN,1=OUR_OWN,2=OUR_DEVICE,3=BOUND_TO_PEER,4=BOUND_TO_SELF),
// sedangkan V1 cuma punya Local(1)/Remote(2).
//
// Diverifikasi di wacore-libsignal-0.7.0 legacy_session.rs:798 — satu-satunya
// pemakaian `base_key_role` adalah validasi "Remote + pending_pre_key.is_some()
// -> error". Importer kita selalu pending_pre_key=None, jadi check itu tidak
// pernah aktif. Maka mapping nilai tak dikenal ke Local **aman**: nol dampak
// ke decrypt, dan 85/538 sesi (16%) yang ada di data produksi tetap terpakai.
// Kalau suatu saat pending_pre_key mulai diisi, keputusan ini harus diulang.
fn map_base_key_role(onigis: i64, stats: &mut ImportStats) -> Option<LegacySessionBaseKeyRoleV1> {
    match onigis {
        1 => Some(LegacySessionBaseKeyRoleV1::Local),
        2 => Some(LegacySessionBaseKeyRoleV1::Remote),
        0 | 3 | 4 => {
            stats.skipped_unknown_basekeyrole += 1;
            Some(LegacySessionBaseKeyRoleV1::Local)
        }
        _ => None,
    }
}

fn map_chain_role(chain_type: i64) -> Option<LegacySessionChainRoleV1> {
    match chain_type {
        1 => Some(LegacySessionChainRoleV1::Sending),
        2 => Some(LegacySessionChainRoleV1::Receiving),
        // 0 = PREKEY: tidak ada chain role V1. sesi prekey sudah selesai
        //_negative; chain prekey tidak perlu untuk decrypt lanjutan.
        _ => None,
    }
}

/// Bangun satu `LegacyIndexedSessionV1` dari satu entri onigis.
fn convert_one(
    _base_key: &str,
    s: &OnigisSession,
    stats: &mut ImportStats,
) -> Result<LegacyIndexedSessionV1> {
    let base_key_bytes = pub_key_b64(&s.index_info.base_key)?;

    let role = map_base_key_role(s.index_info.base_key_type, stats).ok_or_else(|| {
        anyhow::anyhow!(
            "baseKeyRole tidak dipetakan: {}",
            s.index_info.base_key_type
        )
    })?;

    let disposition = LegacySessionDispositionV1::from_closed_timestamp(s.index_info.closed)
        .context("closed timestamp")?;

    // chains: sort by ratchet_key supaya deterministik (from_indexed_sessions
    // akan sort ulang by base_key, tapi stabilitas aids debugging).
    let mut chains = Vec::with_capacity(s._chains.len());
    for (ratchet_key, c) in &s._chains {
        let Some(role) = map_chain_role(c.chain_type) else {
            stats.skipped_prekey_chains += 1;
            continue;
        };
        let ratchet_bytes = pub_key_b64(ratchet_key)?;
        let counter = LegacySessionChainCounterV1::new(c.chain_key.counter).context("counter")?;
        let key_bytes = match &c.chain_key.key {
            Some(k) => Some(base64_decode(k)?),
            None => None,
        };
        // messageKeys: map {index -> {seed: base64}} (umumnya kosong).
        let mut message_keys = Vec::new();
        for (idx, v) in &c.message_keys {
            let Ok(idx_n) = idx.parse::<u32>() else {
                continue;
            };
            let seed_b64 = v.get("seed").and_then(|s| s.as_str()).unwrap_or_default();
            let Ok(seed) = base64_decode(seed_b64) else {
                continue;
            };
            if seed.len() != 32 {
                continue;
            }
            message_keys.push(LegacySessionMessageKeyV1 { index: idx_n, seed });
        }

        chains.push(LegacySessionChainV1 {
            ratchet_key: ratchet_bytes,
            role,
            chain_key: LegacySessionChainKeyV1 {
                counter,
                key: key_bytes,
            },
            message_keys,
        });
    }

    let session = LegacySessionV1 {
        registration_id: s.registration_id,
        ratchet: LegacySessionRatchetV1 {
            key_pair: LegacySessionKeyPairV1 {
                public: pub_key_b64(&s.current_ratchet.ephemeral_key_pair.pub_key)?,
                private: base64_decode(&s.current_ratchet.ephemeral_key_pair.priv_key)?,
            },
            last_remote_ephemeral_key: pub_key_b64(&s.current_ratchet.last_remote_ephemeral_key)?,
            previous_counter: s.current_ratchet.previous_counter,
            root_key: base64_decode(&s.current_ratchet.root_key)?,
        },
        index: LegacySessionIndexV1 {
            base_key: base_key_bytes.clone(),
            base_key_role: role,
            disposition,
            used_at_ms: s.index_info.used,
            created_at_ms: s.index_info.created,
            remote_identity_key: pub_key_b64(&s.index_info.remote_identity_key)?,
        },
        chains,
        // pending_pre_key: tidak ada di serialisasi onigis. None aman — sesi
        // sudah selesai prekey (chainType tidak ada PREKEY).
        pending_pre_key: None,
    };

    Ok(LegacyIndexedSessionV1 {
        index_key: base_key_bytes,
        session,
    })
}

/// Parse satu string JSON onigis (yang mungkin double-encoded) lalu konversi
/// seluruh sesi di dalamnya ke satu `LegacySessionRecordV1`.
pub fn import_session_record(
    raw_data: &str,
    stats: &mut ImportStats,
) -> Result<LegacySessionRecordV1> {
    // Kolom data di Turso = JSON-string yang berisi JSON-string (double-encoded),
    // jadi butuh 2 parse.-smoke test single-level juga boleh (1 parse jalan,
    // kedua parse tidak dipicu karena bukan String lagi... tapi data asli 2-level).
    // Loop defensif: parse sampai bukan String atau mentok.
    let mut value: serde_json::Value = serde_json::from_str(raw_data).context("parse JSON luar")?;
    let mut hops = 0;
    while let serde_json::Value::String(inner) = &value {
        if hops >= 4 {
            break;
        }
        value = serde_json::from_str(inner).context("parse JSON dalam")?;
        hops += 1;
    }
    let root: OnigisRoot =
        serde_json::from_value(value).context("bentuk OnigisRoot tidak dikenali")?;

    let mut indexed = Vec::with_capacity(root._sessions.len());
    for (base_key, s) in &root._sessions {
        stats.total_sessions += 1;
        match convert_one(base_key, s, stats) {
            Ok(entry) => indexed.push(entry),
            Err(e) => {
                let msg = format!("session {base_key}: {e:#}");
                if msg.contains("baseKeyRole") {
                    stats.skipped_unknown_basekeyrole += 1;
                } else {
                    stats.skipped_other += 1;
                }
                if stats.errors.len() < 20 {
                    stats.errors.push(msg);
                }
            }
        }
    }

    let record =
        LegacySessionRecordV1::from_indexed_sessions(indexed).context("validasi SessionRecord")?;
    stats.imported_sessions = record.indexed_sessions().len();
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn wrap(inner: serde_json::Value) -> String {
        serde_json::to_string(&serde_json::to_string(&inner).unwrap()).unwrap()
    }

    // Nilai kripto di bawah diambil apa adanya dari dump Turso produksi —
    // semua base64 valid. JANGAN pakai placeholder pendek ("AAAA"/"PUB"):
    // base64 butuh panjang kelipatan 4, placeholder akan gagal decode dan
    // sesi ter-skip (test jadi hijau tapi salah).
    const PUB: &str = "BeT3xxeHDkC1rdKE+cd8ZvdI7bU7RubLxRnDgBjMaSdg";
    const PRIV: &str = "NbiXHQ3x4fdQRBCD5PwV7PBCAt/OKijY+8CH5GOsfvo=";
    const LREK: &str = "BasgyNM90BAKgsYxCzpECRCSsJG3V2aZGOLCynigRNF8";
    const ROOT: &str = "JhJ9pQt3YXx6By+02V59eKa+jb1OJ4VvlxMsCcgzC7M=";
    const BKEY: &str = "26DlReHF05u30c7hRwp+nL/CNR62JiMPdaZZDLyo0hI=";
    const RID: &str = "BSYSZJtDdy++0q/W3Qd/Sz2M+u2QBjeFqpawd8SRRIt1";

    fn session_json(base_key_type: i64, chains: serde_json::Value) -> serde_json::Value {
        json!({"_sessions": {"BK": {
            "registrationId": 892825949,
            "currentRatchet": {
                "ephemeralKeyPair": {"pubKey": PUB, "privKey": PRIV},
                "lastRemoteEphemeralKey": LREK,
                "previousCounter": 0,
                "rootKey": ROOT
            },
            "indexInfo": {
                "baseKey": BKEY,
                "baseKeyType": base_key_type,
                "closed": -1,
                "used": 1791111010852u64,
                "created": 1791111010852u64,
                "remoteIdentityKey": RID
            },
            "_chains": chains
        }}, "version": 3})
    }

    #[test]
    fn parses_a_real_session() {
        let chains = json!({PUB: {
            "chainKey": {"counter": 0, "key": "ckEoDXdLPWLcPf24serE5iU8zilj0u8QHVNXpmK5sn8="},
            "chainType": 1,
            "messageKeys": {}
        }});
        let outer = wrap(session_json(1, chains));

        let mut stats = ImportStats::default();
        let rec = import_session_record(&outer, &mut stats).expect("impor harus sukses");
        assert_eq!(stats.total_sessions, 1, "errors: {:?}", stats.errors);
        assert_eq!(stats.imported_sessions, 1, "errors: {:?}", stats.errors);
        assert_eq!(stats.skipped_prekey_chains, 0);
        let s = &rec.indexed_sessions()[0].session;
        assert_eq!(s.registration_id, 892825949);
        assert_eq!(s.index.disposition, LegacySessionDispositionV1::Current);
        assert_eq!(s.chains.len(), 1);
        assert_eq!(s.chains[0].role, LegacySessionChainRoleV1::Sending);
        assert_eq!(s.ratchet.root_key.len(), 32, "rootKey harus 32 byte");
        assert_eq!(s.index.remote_identity_key.len(), 33);
    }

    #[test]
    fn base_key_type_zero_is_mapped_to_local_not_dropped() {
        let outer = wrap(session_json(0, json!({})));
        let mut stats = ImportStats::default();
        let rec = import_session_record(&outer, &mut stats).expect("tidak boleh gagal total");
        assert_eq!(
            stats.skipped_unknown_basekeyrole, 1,
            "nilai tak dikenal harus tercatat"
        );
        assert_eq!(stats.imported_sessions, 1, "sesi TIDAK boleh dibuang");
        assert_eq!(
            rec.indexed_sessions()[0].session.index.base_key_role,
            LegacySessionBaseKeyRoleV1::Local
        );
    }

    #[test]
    fn prekey_chain_type_zero_is_skipped() {
        let chains = json!({PUB: {
            "chainKey": {"counter": 0, "key": "ckEoDXdLPWLcPf24serE5iU8zilj0u8QHVNXpmK5sn8="},
            "chainType": 0,
            "messageKeys": {}
        }});
        let outer = wrap(session_json(1, chains));
        let mut stats = ImportStats::default();
        let rec = import_session_record(&outer, &mut stats).expect("sesi harus tetap impor");
        assert_eq!(stats.skipped_prekey_chains, 1);
        assert_eq!(rec.indexed_sessions()[0].session.chains.len(), 0);
    }

    #[test]
    fn buffer_json_decodes() {
        let bj: BufferJson = serde_json::from_str(r#"{"type":"Buffer","data":"AQID"}"#).unwrap();
        assert_eq!(bj.decode().unwrap(), vec![1, 2, 3]);
    }
}
