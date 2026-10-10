//! Kompatibilitas byte-level dengan LowDB (JS).
//!
//! `src/lib/database.js:49` menulis `fs.writeFileSync(tmp, JSON.stringify(obj))`
//! — tanpa indent, key berurutan sesuai objek. `serde_json` dengan
//! `preserve_order` (lihat Cargo.toml) menghasilkan urutan identik.

use serde_json::Value;

/// Parse lalu serialize ulang. Output == input untuk file yang ditulis lowdb.
pub fn roundtrip_bytes(raw: &str) -> Result<String, serde_json::Error> {
    let v: Value = serde_json::from_str(raw)?;
    serde_json::to_string(&v)
}

pub fn parse(raw: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(raw)
}

pub fn stringify(v: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_object_roundtrips_identical() {
        let raw = r#"{"a":1,"b":2,"c":{"d":"e"}}"#;
        assert_eq!(roundtrip_bytes(raw).unwrap(), raw);
    }

    #[test]
    fn key_order_is_preserved_not_sorted() {
        let raw = r#"{"zebra":1,"alpha":2,"middle":3}"#;
        assert_eq!(roundtrip_bytes(raw).unwrap(), raw);
    }

    #[test]
    fn nested_key_order_preserved() {
        let raw = r#"{"outer":{"z":1,"a":2}}"#;
        assert_eq!(roundtrip_bytes(raw).unwrap(), raw);
    }

    #[test]
    fn unicode_and_escapes_preserved() {
        let raw = r#"{"nama":"Oktz 🚀","s":"a\"b\\c\nd"}"#;
        assert_eq!(roundtrip_bytes(raw).unwrap(), raw);
    }

    #[test]
    fn integers_do_not_become_floats() {
        let raw = r#"{"i":42,"neg":-7,"zero":0}"#;
        assert_eq!(roundtrip_bytes(raw).unwrap(), raw);
    }

    #[test]
    fn roundtrips_real_lowdb_file() {
        // File produksi lowdb; hanya jalan kalau ada (test skip kalau tidak).
        let Ok(raw) = std::fs::read_to_string("../../../database/main/settings.json") else {
            eprintln!("skip: settings.json tidak ada di mesin ini");
            return;
        };
        let out = roundtrip_bytes(&raw).expect("roundtrip lowdb");
        assert_eq!(out, raw, "lowdb settings.json harus roundtrip byte-identik");
    }
}
