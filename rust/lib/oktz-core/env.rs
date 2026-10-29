use crate::error::{Error, Result};

/// Pembaca env + .env. Semua secret lewat sini, tidak ada hardcoded.
pub struct Env;

impl Env {
    pub fn load_dotenv() {
        let _ = dotenvy::dotenv();
    }

    pub fn get(key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.is_empty())
    }

    pub fn require(key: &str) -> Result<String> {
        Self::get(key).ok_or_else(|| Error::Env(format!("{key} wajib diisi")))
    }

    pub fn get_or(key: &str, default: &str) -> String {
        Self::get(key).unwrap_or_else(|| default.to_string())
    }

    pub fn require_u64(key: &str) -> Result<u64> {
        let raw = Self::require(key)?;
        raw.parse()
            .map_err(|_| Error::Config(format!("{key} bukan angka: {raw}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Env dibaca process-wide, jadi test serial (tanpa --test-threads) demi
    // tidak saling menimpa variabel yang sama.
    fn with_var(key: &str, val: Option<&str>, f: impl FnOnce()) {
        let prev = std::env::var(key).ok();
        // set_var/remove_var butuh unsafe di edition 2024.
        unsafe {
            match val {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        f();
        unsafe {
            match prev {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn get_returns_none_when_missing_or_empty() {
        with_var("OKTZ_TEST_VAR", None, || {
            assert_eq!(Env::get("OKTZ_TEST_VAR"), None);
        });
        with_var("OKTZ_TEST_VAR", Some(""), || {
            assert_eq!(Env::get("OKTZ_TEST_VAR"), None);
        });
    }

    #[test]
    fn require_errors_when_missing() {
        with_var("OKTZ_TEST_VAR", None, || {
            let e = Env::require("OKTZ_TEST_VAR").unwrap_err();
            assert!(matches!(e, Error::Env(_)));
        });
    }

    #[test]
    fn require_u64_parses() {
        with_var("OKTZ_TEST_NUM", Some("42"), || {
            assert_eq!(Env::require_u64("OKTZ_TEST_NUM").unwrap(), 42);
        });
    }

    #[test]
    fn get_or_falls_back() {
        with_var("OKTZ_TEST_VAR", None, || {
            assert_eq!(Env::get_or("OKTZ_TEST_VAR", "dflt"), "dflt");
        });
    }
}
