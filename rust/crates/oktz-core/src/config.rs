use serde::{Deserialize, Serialize};

use crate::env::Env;

fn d(v: &str) -> String {
    v.to_string()
}

/// Mirror config.js (nilai default sama persis; env override bila ada).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub pairing_number: String,
    pub use_pairing_code: bool,
    pub pairing_rate_limit_cooldown_ms: u64,
    pub pairing_fallback_to_qr: bool,
    pub qr_dir: String,
    pub qr_png_width: u32,
    pub qr_max_prints: u32,
    pub keep_alive_interval_ms: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            pairing_number: String::new(),
            use_pairing_code: true,
            pairing_rate_limit_cooldown_ms: 900_000,
            pairing_fallback_to_qr: true,
            qr_dir: d("storage/qr"),
            qr_png_width: 320,
            qr_max_prints: 5,
            keep_alive_interval_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub path: String,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            path: d("./database/main"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Energi {
    pub enabled: bool,
    pub default: i64,
    pub premium: i64,
    pub owner: i64,
}

impl Default for Energi {
    fn default() -> Self {
        Self {
            enabled: true,
            default: 1,
            premium: 0,
            owner: -1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turso {
    pub url: String,
    pub auth_token: String,
    pub sync_interval: u64,
    pub enabled: bool,
}

impl Default for Turso {
    fn default() -> Self {
        Self {
            url: String::new(),
            auth_token: String::new(),
            sync_interval: 5000,
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub owner_numbers: Vec<String>,
    pub session: Session,
    pub database: Database,
    pub energi: Energi,
    pub turso: Turso,
}

impl Config {
    pub fn from_env() -> Self {
        Env::load_dotenv();
        Self {
            owner_numbers: Env::get("OWNER_NUMBERS")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            session: Session {
                pairing_number: Env::get_or("PAIRING_NUMBER", ""),
                use_pairing_code: Env::get("USE_PAIRING_CODE")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(true),
                pairing_rate_limit_cooldown_ms: Env::get("PAIRING_RATE_LIMIT_COOLDOWN_MS")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(900_000),
                ..Session::default()
            },
            database: Database {
                path: Env::get_or("DATABASE_PATH", "./database/main"),
            },
            energi: Energi::default(),
            turso: Turso {
                url: Env::get_or("TURSO_URL", ""),
                auth_token: Env::get_or("TURSO_AUTH_TOKEN", ""),
                sync_interval: 5000,
                enabled: Env::get("TURSO_URL").is_some(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_config_js() {
        let c = Config::default();
        assert_eq!(c.database.path, "./database/main");
        assert_eq!(c.session.qr_dir, "storage/qr");
        assert_eq!(c.session.qr_png_width, 320);
        assert_eq!(c.session.pairing_rate_limit_cooldown_ms, 900_000);
        assert_eq!(c.energi.owner, -1);
        assert!(c.session.use_pairing_code);
        assert!(c.session.pairing_fallback_to_qr);
    }

    #[test]
    fn owner_numbers_parses_comma_list() {
        let c = Config {
            owner_numbers: vec!["6285608953677".into(), "628123".into()],
            ..Config::default()
        };
        assert_eq!(c.owner_numbers.len(), 2);
        assert_eq!(c.owner_numbers[0], "6285608953677");
    }

    #[test]
    fn turso_disabled_without_url() {
        let c = Config::default();
        assert!(!c.turso.enabled);
    }
}
