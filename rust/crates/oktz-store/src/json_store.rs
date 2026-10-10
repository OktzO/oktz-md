use std::path::{Path, PathBuf};

use oktz_core::error::{Error, Result};
use serde_json::{Map, Value};

pub struct JsonStore {
    path: PathBuf,
    data: Value,
}

impl JsonStore {
    /// Buka file JSON; kalau tidak ada, mulai dari objek kosong.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let data: Value = serde_json::from_str(&raw).map_err(Error::from)?;
            Ok(Self { path, data })
        } else {
            Ok(Self {
                path,
                data: Value::Object(Map::new()),
            })
        }
    }

    fn map_mut(&mut self) -> &mut Map<String, Value> {
        if !self.data.is_object() {
            self.data = Value::Object(Map::new());
        }
        self.data.as_object_mut().expect("sudah object")
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.as_object().and_then(|m| m.get(key))
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.map_mut().insert(key.to_string(), value);
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.as_object_mut().and_then(|m| m.remove(key))
    }

    pub fn all(&self) -> &Value {
        &self.data
    }

    /// Tulis atomik: temp lalu rename (sama seperti `database.js:49`).
    pub fn write(&self) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let body = serde_json::to_string(&self.data).map_err(Error::from)?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpfile(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("oktz-store-test-{name}.json"));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn missing_file_starts_empty() {
        let s = JsonStore::open(tmpfile("missing")).unwrap();
        assert_eq!(s.all(), &Value::Object(Map::new()));
    }

    #[test]
    fn set_get_remove() {
        let mut s = JsonStore::open(tmpfile("crud")).unwrap();
        s.set("a", Value::from(1));
        assert_eq!(s.get("a"), Some(&Value::from(1)));
        s.remove("a");
        assert_eq!(s.get("a"), None);
    }

    // Review Focus #1: field yang tidak dikenal harus bertahan utuh.
    #[test]
    fn unknown_fields_survive_read_modify_write() {
        let path = tmpfile("unknown-fields");
        std::fs::write(
            &path,
            r#"{"known":1,"fieldBaru":{"nested":[1,2,{"x":"y"}]},"tail":"keep"}"#,
        )
        .unwrap();

        let mut s = JsonStore::open(&path).unwrap();
        s.set("known", Value::from(99));
        s.write().unwrap();

        let after = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(v["fieldBaru"]["nested"][2]["x"], "y", "field asing hilang");
        assert_eq!(v["tail"], "keep");
        assert_eq!(v["known"], 99);
    }

    // Review Focus #3: angka tidak jadi float.
    #[test]
    fn integers_stay_integers_on_disk() {
        let path = tmpfile("ints");
        let mut s = JsonStore::open(&path).unwrap();
        s.set("count", Value::from(42));
        s.write().unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, r#"{"count":42}"#);
        assert!(!after.contains("42.0"));
    }

    #[test]
    fn write_is_byte_identical_to_lowdb_format() {
        let path = tmpfile("bytefmt");
        let mut s = JsonStore::open(&path).unwrap();
        s.set("z", Value::from(1));
        s.set("a", Value::from(2));
        s.write().unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            after, r#"{"z":1,"a":2}"#,
            "urutan + format harus sama lowdb"
        );
    }

    #[test]
    fn tmp_file_is_cleaned_after_write() {
        let path = tmpfile("tmpclean");
        let s = JsonStore::open(&path).unwrap();
        s.write().unwrap();
        let tmp = path.with_extension("json.tmp");
        assert!(!tmp.exists(), "file tmp tidak boleh tertinggal");
    }
}
