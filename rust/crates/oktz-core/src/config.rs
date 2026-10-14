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

/// `config.js` `messages`. Field `snake_case`, nilai verbatim dari JS.
///
/// `private_only` dan `admin_only` sengaja dimulai U+FFFD: itulah yang ada
/// di `config.js` (emoji rusak), dan parity berarti menyalinnya apa adanya.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Messages {
    pub success: String,
    pub error: String,
    pub owner_only: String,
    pub premium_only: String,
    pub group_only: String,
    pub private_only: String,
    pub admin_only: String,
    pub bot_admin_only: String,
    pub cooldown: String,
    pub energi_exceeded: String,
    pub limit_deducted: String,
    pub banned: String,
    pub reject_call: String,
    /// Tidak ada di `config.js` — `middleware.js:123,131` menulisnya langsung.
    pub game_disabled: String,
    pub rpg_disabled: String,
    /// `middleware.js:62` juga menulis langsung, bukan lewat `config.messages`.
    pub partner_only: String,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            success: d("✅ *Berhasil!* Permintaan kamu sudah selesai."),
            error: d("❌ *Error!* Ada masalah pada sistem, coba lagi nanti."),
            owner_only: d("*Akses Ditolak!* Fitur ini khusus untuk Owner bot."),
            premium_only: d(
                "💎 *Premium Only!* Fitur ini khusus member Premium. Ketik *.benefitpremium* untuk info upgrade.",
            ),
            group_only: d("👥 *Group Only!* Fitur ini hanya bisa digunakan di dalam grup."),
            private_only: d(
                "\u{fffd} *Private Only!* Fitur ini hanya bisa digunakan di chat pribadi bot.",
            ),
            admin_only: d(
                "\u{fffd}\u{fe0f} *Admin Only!* Kamu harus jadi Admin grup untuk pakai fitur ini.",
            ),
            bot_admin_only: d(
                "🤖 *Bot Bukan Admin!* Jadikan bot sebagai Admin grup dulu biar bisa kerja.",
            ),
            cooldown: d(
                "🕕 *Tunggu Dulu!* Kamu masih dalam cooldown. Tunggu %time% detik lagi ya.",
            ),
            energi_exceeded: d(
                "⚡ *Energi Habis!* Energi kamu sudah habis. Tunggu reset besok atau beli Premium.",
            ),
            limit_deducted: d("🔋 Limit kau berkurang sebanyak {amount}. Sisa limit: {sisa}"),
            banned: d(
                "🚫 *Kamu Dibanned!* Kamu tidak bisa menggunakan bot ini karena telah melanggar aturan.",
            ),
            reject_call: d("🚫 JANGAN TELPON NOMOR INI WEH"),
            game_disabled: d(
                "🎮 Fitur Game sedang dinonaktifkan di grup ini oleh Admin!",
            ),
            rpg_disabled: d(
                "⚔️ Fitur RPG sedang dinonaktifkan di grup ini oleh Admin!",
            ),
            partner_only: d("🤝 Partner only!"),
        }
    }
}

impl Messages {
    pub fn cooldown_text(&self, secs: u64) -> String {
        self.cooldown.replace("%time%", &secs.to_string())
    }
}

/// `config.js` `bot` + `info.website`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotInfo {
    pub name: String,
    pub version: String,
    pub developer: String,
    pub website: String,
}

impl Default for BotInfo {
    fn default() -> Self {
        Self {
            name: d("Oktz Bot"),
            version: d("3.3.1"),
            developer: d("Zann"),
            website: d("https://oktzz.xyz"),
        }
    }
}

/// `config.js` `saluran`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Saluran {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub owner_numbers: Vec<String>,
    pub session: Session,
    pub database: Database,
    pub energi: Energi,
    pub turso: Turso,
    pub messages: Messages,
    pub bot: BotInfo,
    pub saluran: Saluran,
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
            messages: Messages::default(),
            bot: BotInfo::default(),
            saluran: Saluran::default(),
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

    #[test]
    fn denial_texts_match_config_js_not_the_middleware_fallback() {
        let m = Messages::default();
        // `middleware.js:57` memakai fallback hanya kalau `config.messages`
        // kosong; di produksi nilainya selalu ada, jadi yang tampil ke user
        // adalah string dari `config.js`.
        assert_eq!(m.owner_only, "*Akses Ditolak!* Fitur ini khusus untuk Owner bot.");
        assert_ne!(m.owner_only, "🚫 Owner only!");
        assert_eq!(
            m.energi_exceeded,
            "⚡ *Energi Habis!* Energi kamu sudah habis. Tunggu reset besok atau beli Premium."
        );
    }

    #[test]
    fn cooldown_replaces_the_time_placeholder() {
        let m = Messages::default();
        assert!(m.cooldown_text(30).contains("30"));
        assert!(!m.cooldown_text(30).contains("%time%"));
    }

    #[test]
    fn bot_info_website_matches_config_js() {
        assert_eq!(BotInfo::default().website, "https://oktzz.xyz");
        assert_eq!(BotInfo::default().name, "Oktz Bot");
        assert_eq!(BotInfo::default().version, "3.3.1");
    }
}
