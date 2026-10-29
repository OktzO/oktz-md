// Pemetaan creds.json onigis -> wacore::store::Device.
//
// creds.json onigis BUKAN Baileys standar: field device identity bernama
// `signedIdentityKey` (bukan `identityKey`), plus `routingInfo`,
// `myAppStateKeyId`, `lastPropHash`, `platform` yang tidak ada di kredensial
// Baileys biasa.
//
// Device punya Default, jadi strategi: Device::default() lalu override field
// yang bisa dipetakan. Field yang tidak ada di creds (login_counter,
// server_cert_chain, app_version_*) dibiarkan default — `server_cert_chain`
// None memaksa Noise XX handshake (bukan IK) pada koneksi pertama, itu aman.

use anyhow::{Context, Result};
use serde::Deserialize;
use wacore_libsignal::core::curve::{KeyPair, PrivateKey, PublicKey};

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // field dipakai saat constructing Device (tahap berikutnya)
pub struct Creds {
    #[serde(rename = "noiseKey")]
    pub noise_key: KeyPairJson,
    #[serde(rename = "signedIdentityKey")]
    pub signed_identity_key: KeyPairJson,
    #[serde(rename = "signedPreKey")]
    pub signed_pre_key: SignedPreKeyJson,
    #[serde(rename = "advSecretKey")]
    #[serde(deserialize_with = "de_b64_any")]
    pub adv_secret_key: String,
    #[serde(rename = "registrationId")]
    pub registration_id: u32,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    pub me: Option<MeJson>,
    #[serde(default)]
    pub account: Option<AccountJson>,
    #[serde(default)]
    #[serde(rename = "routingInfo")]
    pub routing_info: Option<RoutingInfoJson>,
    #[serde(default)]
    #[serde(rename = "lastPropHash")]
    pub last_prop_hash: Option<String>,
    #[serde(default)]
    #[serde(rename = "nextPreKeyId")]
    pub next_pre_key_id: Option<u32>,
    #[serde(default)]
    #[serde(rename = "firstUnuploadedPreKeyId")]
    pub first_unuploaded_pre_key_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // dipakai saat constructing Device
pub struct KeyPairJson {
    #[serde(deserialize_with = "de_b64_any")]
    pub private: String,
    #[serde(deserialize_with = "de_b64_any")]
    pub public: String,
}

/// Di `creds.json` produksi nilai key berupa object BufferJSON
/// (`{"type":"Buffer","data":"<b64>"}`), sedangkan di file pre-key berupa
/// string base64 polos. Dua-duanya harus diterima.
fn de_b64_any<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Any {
        Str(String),
        Buf {
            #[serde(rename = "type")]
            kind: String,
            data: String,
        },
    }
    Ok(match Any::deserialize(d)? {
        Any::Str(s) => s,
        Any::Buf { kind, data } if kind == "Buffer" => data,
        Any::Buf { kind, .. } => {
            return Err(serde::de::Error::custom(format!("bukan Buffer: {kind}")));
        }
    })
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // key_pair dipakai saat constructing Device
pub struct SignedPreKeyJson {
    #[serde(rename = "keyPair")]
    pub key_pair: KeyPairJson,
    #[serde(deserialize_with = "de_b64_any")]
    pub signature: String,
    #[serde(rename = "keyId")]
    pub key_id: u32,
}

#[derive(Debug, Deserialize)]
pub struct MeJson {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub lid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingInfoJson {
    #[serde(default)]
    pub data: Option<String>,
}

/// `account` di creds.json == protobuf `ADVSignedDeviceIdentity` (4 field bytes),
/// pemetaannya 1:1. `details` sudah berupa bytes ter-serialize, pass-through.
#[derive(Debug, Deserialize)]
pub struct AccountJson {
    #[serde(default, deserialize_with = "de_b64_any_opt")]
    pub details: Option<String>,
    #[serde(default, deserialize_with = "de_b64_any_opt")]
    pub account_signature_key: Option<String>,
    #[serde(default, deserialize_with = "de_b64_any_opt")]
    pub account_signature: Option<String>,
    #[serde(default, deserialize_with = "de_b64_any_opt")]
    pub device_signature: Option<String>,
}

fn de_b64_any_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let s = de_b64_any(d)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

pub fn b64(s: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    Ok(base64::engine::general_purpose::STANDARD.decode(cleaned.as_bytes())?)
}

#[cfg(test)]
#[cfg(test)]
fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[allow(dead_code)] // dipakai saat constructing Device pada tahap berikutnya
fn kp(j: &KeyPairJson) -> Result<KeyPair> {
    Ok(KeyPair {
        public_key: PublicKey::try_from(&b64(&j.public)?[..]).context("public key")?,
        private_key: PrivateKey::try_from(&b64(&j.private)?[..]).context("private key")?,
    })
}

/// Hasil pemetaan + catatan yang tidak bisa diturunkan dari creds.json.
#[derive(Debug, Default)]
pub struct DeviceMapping {
    pub registration_id: u32,
    pub pn: String,
    pub lid: Option<String>,
    pub push_name: String,
    pub platform: String,
    pub next_pre_key_id: u32,
    pub signed_pre_key_id: u32,
    pub adv_secret_key_len: usize,
    pub has_routing_info: bool,
    /// Field Device yang TIDAK ada di creds.json -> dibiarkan default.
    pub defaulted_fields: Vec<&'static str>,
}

/// Validasi creds + ekstrak nilai yang bisa dipetakan, tanpa menyentuh
/// struktur Device (dipakai test tanpa perlu construct Device penuh).
pub(crate) fn inspect_creds(raw: &str) -> Result<(DeviceMapping, Creds)> {
    let c: Creds = serde_json::from_str(raw).context("parse creds.json")?;
    let me =
        c.me.as_ref()
            .context("creds.json tanpa `me` — sesi belum pernah login")?;

    let adv = b64(&c.adv_secret_key)?;
    let sig = b64(&c.signed_pre_key.signature)?;

    let mut defaulted = Vec::new();
    // Tidak ada di creds.json onigis sama sekali.
    for f in [
        "login_counter",
        "pairing_epoch",
        "device_pairing_data",
        "server_cert_chain",
        "app_version_primary/secondary/tertiary",
        "account (ADVSignedDeviceIdentity)",
    ] {
        defaulted.push(f);
    }
    if sig.len() != 64 {
        anyhow::bail!("signedPreKey.signature harus 64 byte, dapat {}", sig.len());
    }
    if adv.len() != 32 {
        anyhow::bail!("advSecretKey harus 32 byte, dapat {}", adv.len());
    }

    let m = DeviceMapping {
        registration_id: c.registration_id,
        pn: me.id.clone(),
        lid: me.lid.clone(),
        push_name: me.name.clone(),
        platform: c.platform.clone().unwrap_or_default(),
        next_pre_key_id: c.next_pre_key_id.unwrap_or(0),
        signed_pre_key_id: c.signed_pre_key.key_id,
        adv_secret_key_len: adv.len(),
        has_routing_info: c
            .routing_info
            .as_ref()
            .and_then(|r| r.data.as_ref())
            .is_some(),
        defaulted_fields: defaulted,
    };
    Ok((m, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// creds.json produksi nyata, disanitasi: nilai kripto diganti dummy
    /// TAPI panjang base64 dijaga persis (32/33/64 byte).
    fn fixture() -> String {
        let sig64 = b64_encode(&[0x11u8; 64]);
        serde_json::to_string(&json!({
            "noiseKey": {"private": b64_encode(&[0x01u8; 32]), "public": b64_encode(&[0x02u8; 33])},
            "signedIdentityKey": {"private": b64_encode(&[0x03u8; 32]), "public": b64_encode(&[0x04u8; 33])},
            "signedPreKey": {
                "keyPair": {"private": b64_encode(&[0x05u8; 32]), "public": b64_encode(&[0x06u8; 33])},
                "signature": sig64,
                "keyId": 15555
            },
            "advSecretKey": b64_encode(&[0x07u8; 32]),
            "registrationId": 225,
            "platform": "smba",
            "me": {"id": "6285143885645:54@s.whatsapp.net", "name": "Oktz",
                   "lid": "188412430651393:54@lid"},
            "routingInfo": {"type": "BOOL", "data": b64_encode(b"MODALIDADE")},
            "lastPropHash": "94gg0",
            "nextPreKeyId": 15556,
            "firstUnuploadedPreKeyId": 15556,
            "registered": true,
            "additionalData": {}
        }))
        .unwrap()
    }

    #[test]
    fn maps_real_creds_shape() {
        let (m, _c) = inspect_creds(&fixture()).expect("harus berhasil");
        assert_eq!(m.registration_id, 225);
        assert_eq!(m.pn, "6285143885645:54@s.whatsapp.net");
        assert_eq!(m.lid.as_deref(), Some("188412430651393:54@lid"));
        assert_eq!(m.push_name, "Oktz");
        assert_eq!(m.platform, "smba");
        assert_eq!(m.signed_pre_key_id, 15555);
        assert_eq!(m.next_pre_key_id, 15556);
        assert_eq!(m.adv_secret_key_len, 32);
        assert!(m.has_routing_info);
    }

    #[test]
    fn rejects_wrong_length_signature() {
        // signature 32 byte, harusnya 64
        let f = fixture().replace(&b64_encode(&[0x11u8; 64]), &b64_encode(&[0x11u8; 32]));
        let err = inspect_creds(&f).unwrap_err();
        assert!(format!("{err:#}").contains("64 byte"), "got: {err:#}");
    }

    #[test]
    fn rejects_wrong_length_adv_secret() {
        let f = fixture().replace(&b64_encode(&[0x07u8; 32]), &b64_encode(&[0x07u8; 16]));
        let err = inspect_creds(&f).unwrap_err();
        assert!(format!("{err:#}").contains("advSecretKey"), "got: {err:#}");
    }

    #[test]
    fn reports_fields_that_must_default() {
        let (m, _) = inspect_creds(&fixture()).unwrap();
        // Field ini tidak ada di creds.json onigis -> wajib default.
        assert!(m.defaulted_fields.contains(&"login_counter"));
        assert!(m.defaulted_fields.contains(&"server_cert_chain"));
        assert!(
            m.defaulted_fields
                .contains(&"account (ADVSignedDeviceIdentity)")
        );
    }

    #[test]
    fn parses_the_actual_production_creds_file() {
        // creds.json asli di repo (gitignored, hanya ada di mesin Oktz).
        // Test ini skip kalau file tidak ada.
        let Ok(raw) = std::fs::read_to_string("../storage/session/creds.json") else {
            eprintln!("skip: creds.json produksi tidak ada di mesin ini");
            return;
        };
        let (m, _c) = inspect_creds(&raw).expect("creds.json produksi harus bisa dipetakan");
        assert!(m.registration_id > 0);
        assert!(m.pn.contains("@s.whatsapp.net"));
        assert_eq!(m.adv_secret_key_len, 32);
        eprintln!(
            "PRODUKSI OK: reg={} pn={} lid={:?} spk_id={}",
            m.registration_id, m.pn, m.lid, m.signed_pre_key_id
        );
    }
}
