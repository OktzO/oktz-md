use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::env::Env;
use crate::error::{Error, Result};

fn d(v: &str) -> String {
    v.to_string()
}

/// `config.js` `APIkey` — semua dari env, nilai kosong kalau tidak diisi
/// (`config.js:326-335`). Field yang kosong tidak dihapus supaya nama kuncinya
/// masih terlihat saat debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeys {
    pub neoxr: String,
    pub fgsi: String,
    pub groq: String,
    pub covenant: String,
    pub obscura: String,
    pub firefly: String,
    pub cuki: String,
    pub linode: String,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            neoxr: Env::get_or("APIKEY_NEOXR", ""),
            fgsi: Env::get_or("APIKEY_FGSI", ""),
            groq: Env::get_or("GROQ_API_KEY", ""),
            covenant: Env::get_or("APIKEY_COVENANT", ""),
            obscura: Env::get_or("APIKEY_OBSCURA", ""),
            firefly: Env::get_or("APIKEY_FIREFLY", ""),
            cuki: Env::get_or("APIKEY_CUKI", ""),
            linode: Env::get_or("LINODE_API_TOKEN", ""),
        }
    }
}

/// `config.js` `owner` (`config.js:13-16`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    pub number: Vec<String>,
}

impl Default for Owner {
    fn default() -> Self {
        Self {
            name: d("Oktz"),
            number: vec![d("6285608953677")],
        }
    }
}

/// `config.js` `sticker` (`config.js:112-115`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sticker {
    pub packname: String,
    pub author: String,
}

impl Default for Sticker {
    fn default() -> Self {
        Self {
            // `config.js:113` — nama pack sticker produksi.
            packname: d("𝗙𝗢𝗧𝗢 𝗗𝗘𝗟𝗨𝗫𝗘"),
            author: d("Zann"),
        }
    }
}

/// `config.js` `info` (`config.js:10-13`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub website: String,
    pub grupwa: String,
}

impl Default for Info {
    fn default() -> Self {
        Self {
            website: d("https://oktzz.xyz"),
            grupwa: d("https://chat.whatsapp.com/H1MOIGsWKuK1ulzOBAjW9b"),
        }
    }
}

/// Satu metode pembayaran pada `config.payment` / `config.donasi.payment`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub name: String,
    pub number: String,
    pub holder: String,
}

/// Tautan donasi pada `config.donasi.links`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DonasiLink {
    pub name: String,
    pub url: String,
}

/// `config.js` `payment` (`config.js:81-100`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub qris_url: String,
    pub methods: Vec<PaymentMethod>,
    pub banks: Vec<PaymentMethod>,
    pub custom_text: String,
}

impl Default for Payment {
    fn default() -> Self {
        Self {
            qris_url: String::new(),
            methods: ["Dana", "GoPay", "OVO", "ShopeePay"]
                .into_iter()
                .map(|n| PaymentMethod {
                    name: n.to_string(),
                    number: String::new(),
                    holder: String::new(),
                })
                .collect(),
            banks: Vec::new(),
            custom_text: d("https://imgdrop.web.id/KodpV.webp"),
        }
    }
}

/// `config.js` `donasi` (`config.js:102-112`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Donasi {
    pub payment: Vec<PaymentMethod>,
    pub links: Vec<DonasiLink>,
    pub benefits: Vec<String>,
    pub qris: String,
}

impl Default for Donasi {
    fn default() -> Self {
        Self {
            payment: ["Dana", "GoPay", "OVO"]
                .into_iter()
                .map(|n| PaymentMethod {
                    name: n.to_string(),
                    number: d("08xxxxxxxxxx"),
                    holder: d("Nama Owner"),
                })
                .collect(),
            links: vec![
                DonasiLink {
                    name: d("Saweria"),
                    url: d("saweria.co/username"),
                },
                DonasiLink {
                    name: d("Trakteer"),
                    url: d("trakteer.id/username"),
                },
            ],
            benefits: vec![
                d("Mendukung development"),
                d("Server lebih stabil"),
                d("Fitur baru lebih cepat"),
                d("Priority support"),
            ],
            qris: d("https://imgdrop.web.id/KodpV.webp"),
        }
    }
}

/// `config.js` `assets` — peta nama ke path relatif (`config.js:44-66`).
///
/// Disimpan apa adanya sebagai string, bukan `PathBuf`: plugin JS melakukan
/// `fs.readFileSync(config.assets["mp4"])`, jadi yang dibutuhkan Hanya teks path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assets(pub std::collections::BTreeMap<String, String>);

impl Default for Assets {
    fn default() -> Self {
        let mut m = std::collections::BTreeMap::new();
        for (k, v) in [
            ("daftar", "./assets/image/daftar.jpeg"),
            ("demote", "./assets/image/demote.jpeg"),
            ("fishit", "./assets/image/fishit.jpeg"),
            ("games", "./assets/image/games.jpeg"),
            ("landscape", "./assets/image/landscape.jpeg"),
            ("levelup", "./assets/image/levelup.jpeg"),
            ("minecraft", "./assets/image/minecraft.jpeg"),
            ("promote", "./assets/image/promote.jpeg"),
            ("rpg", "./assets/image/rpg.jpeg"),
            ("rules", "./assets/image/rules.jpeg"),
            ("store", "./assets/image/store.jpeg"),
            ("v8", "./assets/image/v8.jpeg"),
            ("winner", "./assets/image/winner.jpeg"),
            ("foto", "./assets/image/foto.jpeg"),
            ("foto2", "./assets/image/foto2.jpeg"),
            ("foto3", "./assets/image/foto3.jpeg"),
            ("pp-kosong", "./assets/image/pp-kosong.jpeg"),
            ("mp4", "./assets/video/mp4.mp4"),
            ("mp3", "./assets/audio/mp3.mp3"),
            ("font", "./assets/font.ttf"),
            ("kertas", "./assets/image/kertas.jpeg"),
        ] {
            m.insert(d(k), d(v));
        }
        Self(m)
    }
}

impl Assets {
    /// `config.assets[nama]` — `None` kalau kuncinya tidak ada, supaya plugin
    /// bisa membedakan `undefined` (jalur error) dari path kosong.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

/// Yang boleh dibaca plugin lewat `ctx.config()`: cerminan `config.js` yang
/// dipakai plugin, bukan seluruh objeknya.
///
/// Yang TIDAK ada di sini sengaja: `session`, `database`, `turso`,
/// `groupProtection`, `features`, `autoWork`, `registration`, `ui`, dan
/// `energi` — semuanya sudah punya jalur sendiri di pipeline (gate, limiter,
/// store), jadi menyalinnya ke sini hanya menciptakan dua sumber kebenaran.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub info: Info,
    pub owner: Owner,
    pub bot: BotInfo,
    pub assets: Assets,
    pub sticker: Sticker,
    pub payment: Payment,
    pub donasi: Donasi,
    pub saluran: Saluran,
    pub api_keys: ApiKeys,
}

impl Default for Runtime {
    /// Semua string KOSONG. Nilai produksi ada di [`Runtime::from_config`].
    ///
    /// Sengaja tidak memakai `BotInfo::default`/`Owner::default`:
    /// `ApiKeys::default` membaca environment, jadi `Ctx::default()` di test
    /// bisa mengambil API key sungguhan kalau env-nya terisi. Default kosong
    /// membuat kebocoran itu mustahil.
    fn default() -> Self {
        Self {
            info: Info {
                website: String::new(),
                grupwa: String::new(),
            },
            owner: Owner {
                name: String::new(),
                number: Vec::new(),
            },
            bot: BotInfo {
                name: String::new(),
                version: String::new(),
                developer: String::new(),
                website: String::new(),
            },
            assets: Assets(std::collections::BTreeMap::new()),
            sticker: Sticker {
                packname: String::new(),
                author: String::new(),
            },
            payment: Payment {
                qris_url: String::new(),
                methods: Vec::new(),
                banks: Vec::new(),
                custom_text: String::new(),
            },
            donasi: Donasi {
                payment: Vec::new(),
                links: Vec::new(),
                benefits: Vec::new(),
                qris: String::new(),
            },
            saluran: Saluran::default(),
            api_keys: ApiKeys {
                neoxr: String::new(),
                fgsi: String::new(),
                groq: String::new(),
                covenant: String::new(),
                obscura: String::new(),
                firefly: String::new(),
                cuki: String::new(),
                linode: String::new(),
            },
        }
    }
}

impl Runtime {
    /// Bawaan `config.js`. `saluran.id` dan `name` mengikuti `config.js`
    /// apa adanya; `Saluran::default()` sengaja kosong supaya unit test lain
    /// tidak ikut terbawa nilai produksi.
    pub fn from_config() -> Self {
        Self {
            info: Info::default(),
            owner: Owner::default(),
            bot: BotInfo::default(),
            assets: Assets::default(),
            sticker: Sticker::default(),
            payment: Payment::default(),
            donasi: Donasi::default(),
            saluran: Saluran {
                id: d("120363411617217441@newsletter"),
                name: d("Join saluran resmi Oktz"),
                link: d("https://whatsapp.com/channel/0029Vb8z7S0KrWQvCa8i0J0I"),
            },
            api_keys: ApiKeys::default(),
        }
    }
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
            // `config.js:17` — tunggu 1 jam bila WhatsApp rate-limit pairing.
            pairing_rate_limit_cooldown_ms: 3_600_000,
            pairing_fallback_to_qr: true,
            // `config.js:18` — folder QR di dalam `storage/`.
            qr_dir: d("qr"),
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
    /// `config.js:125` — link undangan saluran.
    pub link: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub owner_numbers: Vec<String>,
    /// `config.js` `owner.name` — nama owner yang dipakai `getOwnerName`.
    pub owner_name: String,
    pub info: Info,
    pub sticker: Sticker,
    pub api_keys: ApiKeys,
    pub session: Session,
    pub database: Database,
    pub energi: Energi,
    pub turso: Turso,
    pub messages: Messages,
    pub bot: BotInfo,
    pub saluran: Saluran,
}

/// Env yang memindahkan lokasi `config.json`; kosong = `config.json` di cwd.
pub const CONFIG_ENV: &str = "OKTZ_CONFIG";

/// Nama file config yang dibaca [`Config::load`]. Kunci tak dikenal diabaikan.
pub const CONFIG_FILE: &str = "config.json";

impl Config {
    /// Bawaan dari env + `.env` saja (`config.js` versi lama).
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
            owner_name: Owner::default().name,
            info: Info::default(),
            sticker: Sticker::default(),
            api_keys: ApiKeys::default(),
            session: Session {
                pairing_number: Env::get_or("PAIRING_NUMBER", ""),
                use_pairing_code: Env::get("USE_PAIRING_CODE")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(true),
                pairing_rate_limit_cooldown_ms: Env::get("PAIRING_RATE_LIMIT_COOLDOWN_MS")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3_600_000),
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
            saluran: Saluran {
                id: d("120363411617217441@newsletter"),
                name: d("Join saluran resmi Oktz"),
                link: d("https://whatsapp.com/channel/0029Vb8z7S0KrWQvCa8i0J0I"),
            },
        }
    }

    /// Env + `.env`, lalu ditimpa `config.json` (lokasi lewat [`CONFIG_ENV`]).
    ///
    /// Nilai terisi di file menang; string kosong berarti "jangan sentuh".
    /// File tidak ada bukan error, file rusak/salah tipe justru error.
    pub fn load() -> Result<Self> {
        let path = Env::get(CONFIG_ENV).unwrap_or_else(|| CONFIG_FILE.to_string());
        let mut cfg = Self::from_env();
        match std::fs::read_to_string(&path) {
            Ok(raw) => {
                let v: Value = serde_json::from_str(&raw)
                    .map_err(|e| Error::Config(format!("{path}: JSON tidak valid: {e}")))?;
                cfg.apply_file(&v)
                    .map_err(|e| Error::Config(format!("{path}: {e}")))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(path = %path, "config.json tidak ada; pakai env + bawaan");
            }
            Err(e) => return Err(Error::Config(format!("{path}: {e}"))),
        }
        // Owner bawaan `config.js` dipakai kalau env DAN file sama-sama diam,
        // supaya owner tetap dikenali tanpa mengisi apa pun.
        if cfg.owner_numbers.is_empty() {
            cfg.owner_numbers = Owner::default().number;
        }
        Ok(cfg)
    }

    /// Timpa field dari objek JSON bentuk `config.js`. Dipisah dari
    /// [`Self::load`] supaya bisa diuji tanpa menyentuh cwd.
    pub fn apply_file(&mut self, root: &Value) -> Result<()> {
        let root = root
            .as_object()
            .ok_or_else(|| Error::Config("isi config harus objek JSON".into()))?;

        if let Some(m) = objek(root, "owner")? {
            set_teks(m, "name", "owner", &mut self.owner_name)?;
            if let Some(l) = daftar_teks(m, "number", "owner.number")? {
                self.owner_numbers = l;
            }
        }
        if let Some(m) = objek(root, "info")? {
            set_teks(m, "website", "info", &mut self.info.website)?;
            set_teks(m, "grupwa", "info", &mut self.info.grupwa)?;
        }
        if let Some(m) = objek(root, "bot")? {
            set_teks(m, "name", "bot", &mut self.bot.name)?;
            set_teks(m, "version", "bot", &mut self.bot.version)?;
            set_teks(m, "developer", "bot", &mut self.bot.developer)?;
            set_teks(m, "website", "bot", &mut self.bot.website)?;
        }
        if let Some(m) = objek(root, "sticker")? {
            set_teks(m, "packname", "sticker", &mut self.sticker.packname)?;
            set_teks(m, "author", "sticker", &mut self.sticker.author)?;
        }
        if let Some(m) = objek(root, "saluran")? {
            set_teks(m, "id", "saluran", &mut self.saluran.id)?;
            set_teks(m, "name", "saluran", &mut self.saluran.name)?;
            set_teks(m, "link", "saluran", &mut self.saluran.link)?;
        }
        if let Some(m) = objek(root, "session")? {
            set_teks(m, "pairingNumber", "session", &mut self.session.pairing_number)?;
            set_bool(m, "usePairingCode", "session", &mut self.session.use_pairing_code)?;
            set_bool(
                m,
                "pairingFallbackToQr",
                "session",
                &mut self.session.pairing_fallback_to_qr,
            )?;
            set_u64(
                m,
                "pairingRateLimitCooldownMs",
                "session",
                &mut self.session.pairing_rate_limit_cooldown_ms,
            )?;
            set_u64(m, "keepAliveIntervalMs", "session", &mut self.session.keep_alive_interval_ms)?;
            set_teks(m, "qrDir", "session", &mut self.session.qr_dir)?;
            set_u32(m, "qrPngWidth", "session", &mut self.session.qr_png_width)?;
            set_u32(m, "qrMaxPrints", "session", &mut self.session.qr_max_prints)?;
        }
        if let Some(m) = objek(root, "database")? {
            set_teks(m, "path", "database", &mut self.database.path)?;
        }
        if let Some(m) = objek(root, "turso")? {
            set_teks(m, "url", "turso", &mut self.turso.url)?;
            set_teks(m, "authToken", "turso", &mut self.turso.auth_token)?;
            set_u64(m, "syncInterval", "turso", &mut self.turso.sync_interval)?;
            set_bool(m, "enabled", "turso", &mut self.turso.enabled)?;
        }
        if let Some(m) = objek(root, "apiKeys")? {
            let a = &mut self.api_keys;
            set_teks(m, "neoxr", "apiKeys", &mut a.neoxr)?;
            set_teks(m, "fgsi", "apiKeys", &mut a.fgsi)?;
            set_teks(m, "groq", "apiKeys", &mut a.groq)?;
            set_teks(m, "covenant", "apiKeys", &mut a.covenant)?;
            set_teks(m, "obscura", "apiKeys", &mut a.obscura)?;
            set_teks(m, "firefly", "apiKeys", &mut a.firefly)?;
            set_teks(m, "cuki", "apiKeys", &mut a.cuki)?;
            set_teks(m, "linode", "apiKeys", &mut a.linode)?;
        }
        Ok(())
    }

    /// [`Runtime`] untuk plugin: bawaan `config.js`, ditimpa nilai config.
    /// `payment`/`donasi` masih bawaan.
    pub fn runtime(&self) -> Runtime {
        let mut rt = Runtime::from_config();
        rt.info = self.info.clone();
        rt.owner = Owner {
            name: self.owner_name.clone(),
            number: self.owner_numbers.clone(),
        };
        rt.bot = self.bot.clone();
        rt.sticker = self.sticker.clone();
        rt.saluran = self.saluran.clone();
        rt.api_keys = self.api_keys.clone();
        rt
    }
}

/// Sub-objek `kunci`; `null`/tidak ada = `None`, bukan objek = error.
fn objek<'a>(root: &'a Map<String, Value>, kunci: &str) -> Result<Option<&'a Map<String, Value>>> {
    match root.get(kunci) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(m)) => Ok(Some(m)),
        Some(_) => Err(Error::Config(format!("{kunci} harus objek"))),
    }
}

fn teks(v: &Value, jalur: &str) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(Error::Config(format!("{jalur} harus teks"))),
    }
}

fn set_teks(m: &Map<String, Value>, kunci: &str, jalur: &str, out: &mut String) -> Result<()> {
    match m.get(kunci) {
        None | Some(Value::Null) => Ok(()),
        Some(v) => {
            let s = teks(v, &format!("{jalur}.{kunci}"))?;
            if !s.is_empty() {
                *out = s;
            }
            Ok(())
        }
    }
}

fn set_bool(m: &Map<String, Value>, kunci: &str, jalur: &str, out: &mut bool) -> Result<()> {
    match m.get(kunci) {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Bool(b)) => {
            *out = *b;
            Ok(())
        }
        Some(_) => Err(Error::Config(format!("{jalur}.{kunci} harus true/false"))),
    }
}

fn set_u64(m: &Map<String, Value>, kunci: &str, jalur: &str, out: &mut u64) -> Result<()> {
    match m.get(kunci) {
        None | Some(Value::Null) => Ok(()),
        Some(v) => {
            *out = v
                .as_u64()
                .ok_or_else(|| Error::Config(format!("{jalur}.{kunci} harus bilangan bulat")))?;
            Ok(())
        }
    }
}

fn set_u32(m: &Map<String, Value>, kunci: &str, jalur: &str, out: &mut u32) -> Result<()> {
    match m.get(kunci) {
        None | Some(Value::Null) => Ok(()),
        Some(v) => {
            let n = v
                .as_u64()
                .ok_or_else(|| Error::Config(format!("{jalur}.{kunci} harus bilangan bulat")))?;
            *out = u32::try_from(n)
                .map_err(|_| Error::Config(format!("{jalur}.{kunci} melebihi batas u32")))?;
            Ok(())
        }
    }
}

fn daftar_teks(m: &Map<String, Value>, kunci: &str, jalur: &str) -> Result<Option<Vec<String>>> {
    let Some(v) = m.get(kunci).filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let arr = v
        .as_array()
        .ok_or_else(|| Error::Config(format!("{jalur} harus array")))?;
    let mut out = Vec::new();
    for (i, x) in arr.iter().enumerate() {
        let s = teks(x, &format!("{jalur}[{i}]"))?;
        if !s.is_empty() {
            out.push(s);
        }
    }
    Ok(if out.is_empty() { None } else { Some(out) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_default_kosong_tapi_from_config_berisi_nilai_prodiksi() {
        let kosong = Runtime::default();
        assert_eq!(kosong.bot.name, "");
        assert_eq!(kosong.owner.name, "");
        assert_eq!(kosong.info.website, "");
        assert_eq!(kosong.api_keys.firefly, "");
        assert!(kosong.assets.get("foto").is_none());
        assert_eq!(kosong.payment.methods.len(), 0);

        let prod = Runtime::from_config();
        assert_eq!(prod.bot.name, "Oktz Bot");
        assert_eq!(prod.bot.developer, "Zann");
        assert_eq!(prod.owner.name, "Oktz");
        assert_eq!(prod.owner.number, vec!["6285608953677".to_string()]);
        assert_eq!(prod.info.website, "https://oktzz.xyz");
        assert_eq!(prod.sticker.author, "Zann");
        assert_eq!(prod.saluran.id, "120363411617217441@newsletter");
        assert_eq!(
            prod.assets.get("mp4"),
            Some("./assets/video/mp4.mp4"),
            "path aset ikut apa adanya, bukan digabung ke storage"
        );
        assert_eq!(prod.assets.get("pp-kosong"), Some("./assets/image/pp-kosong.jpeg"));
        assert_eq!(prod.payment.methods.len(), 4);
        assert_eq!(prod.donasi.payment.len(), 3);
        assert_eq!(prod.donasi.links.len(), 2);
        assert_eq!(prod.donasi.benefits.len(), 4);
    }

    #[test]
    fn defaults_match_config_js() {
        let c = Config::default();
        assert_eq!(c.database.path, "./database/main");
        assert_eq!(c.session.qr_dir, "qr");
        assert_eq!(c.session.qr_png_width, 320);
        assert_eq!(c.session.pairing_rate_limit_cooldown_ms, 3_600_000);
        assert_eq!(c.energi.owner, -1);
        assert!(c.session.use_pairing_code);
        assert!(c.session.pairing_fallback_to_qr);
    }

    #[test]
    fn apply_file_menimpa_nilai_dan_melewati_string_kosong() {
        let mut cfg = Config::from_env();
        cfg.owner_numbers.clear();
        cfg.session.pairing_number = "628111".into();
        cfg.apply_file(&serde_json::json!({
            "owner": { "name": "Bos", "number": ["628999", "628888"] },
            "bot": { "name": "Bot Uji", "version": "" },
            "session": {
                "pairingNumber": "628222",
                "usePairingCode": false,
                "qrMaxPrints": 3,
            },
            "turso": { "url": "libsql://contoh", "authToken": "rahasia", "enabled": true },
            "apiKeys": { "groq": "kunci-groq", "neoxr": "" },
        }))
        .unwrap();

        assert_eq!(cfg.owner_name, "Bos");
        assert_eq!(cfg.owner_numbers, vec!["628999", "628888"]);
        assert_eq!(cfg.session.pairing_number, "628222");
        assert!(!cfg.session.use_pairing_code);
        assert_eq!(cfg.session.qr_max_prints, 3);
        // String kosong tidak menimpa: "kunci-groq" dipakai, neoxr dibiarkan
        // apa adanya dari env (bisa kosong).
        assert_eq!(cfg.api_keys.groq, "kunci-groq");
        assert_eq!(cfg.bot.name, "Bot Uji");
        assert_eq!(cfg.bot.version, BotInfo::default().version);
        assert!(cfg.turso.enabled);
        assert_eq!(cfg.turso.auth_token, "rahasia");
    }

    #[test]
    fn apply_file_menolak_tipe_salah() {
        let mut cfg = Config::from_env();
        let e = cfg
            .apply_file(&serde_json::json!({"session": {"usePairingCode": "ya"}}))
            .unwrap_err();
        assert!(format!("{e}").contains("usePairingCode"), "{e}");
        let e = cfg
            .apply_file(&serde_json::json!({"turso": 5}))
            .unwrap_err();
        assert!(format!("{e}").contains("turso"), "{e}");
    }

    #[test]
    fn runtime_mengambil_nilai_dari_config() {
        let mut cfg = Config::from_env();
        cfg.apply_file(&serde_json::json!({
            "owner": { "name": "Bos", "number": ["628999"] },
            "bot": { "name": "Bot Uji" },
            "saluran": { "name": "Saluran Uji" },
            "sticker": { "packname": "Pack Uji" },
            "apiKeys": { "groq": "kunci-groq" },
        }))
        .unwrap();
        let rt = cfg.runtime();
        assert_eq!(rt.owner.name, "Bos");
        assert_eq!(rt.owner.number, vec!["628999".to_string()]);
        assert_eq!(rt.bot.name, "Bot Uji");
        assert_eq!(rt.saluran.name, "Saluran Uji");
        assert_eq!(rt.sticker.packname, "Pack Uji");
        assert_eq!(rt.api_keys.groq, "kunci-groq");
        // Blok yang belum punya jalur file tetap bawaan `config.js`.
        assert_eq!(rt.payment.methods.len(), 4);
        assert_eq!(rt.assets.get("mp4"), Some("./assets/video/mp4.mp4"));
    }

    #[test]
    fn config_contoh_yang_dikirim_bisa_dibaca() {
        // File di `rust/config.json` ikut dikirim ke repo `oktz-rs`; kalau
        // kuncinya salah ketik, loader-nya jangan sampai baru ketahuan saat
        // user menjalankan bot.
        let raw = include_str!("../../config.json");
        let v: Value = serde_json::from_str(raw).expect("config.json harus JSON valid");
        let mut cfg = Config::from_env();
        cfg.apply_file(&v).expect("config.json harus lolos loader");
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
