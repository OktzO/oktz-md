//! Port `src/lib/games.js` — sistem foto game (tebak gambar / kuis).
//!
//! Ekspor JS: `FotoGames` class, `games` instance, dan fungsi `pick`.
//! Semua fungsi pub di bawah menamai asal JS-nya di komentar.

use rand::Rng as _;
use std::collections::HashMap;

/// `pick(arr)` (`games.js:53-55`) — ambil satu item acak dari array.
///
/// `Math.floor(Math.random() * arr.length)`. Mengembalikan `None` kalau
/// array kosong (JS mengembalikan `undefined`).
pub fn pick(arr: &[&str]) -> Option<&str> {
    if arr.is_empty() {
        return None;
    }
    let idx = rand::rng().random_range(0..arr.len());
    Some(arr[idx])
}

/// `WIN_MESSAGES` (`games.js:32-39`).
pub const WIN_MESSAGES: [&str; 6] = [
    "🌟 *GG WP! Otakmu encer!*",
    "✨ *KEREN ABIS! Lu emang pinter!*",
    "🎉 *MANTAPPPP! Jawaban sempurna!*",
    "💫 *EPIC! Gak ada lawan lu!*",
    "🏆 *NGERI! Otak lu kayak Google!*",
    "🔥 *LEGEND! Jawab kek gak ada beban!*",
];

/// `TIMEOUT_MESSAGES` (`games.js:41-45`).
pub const TIMEOUT_MESSAGES: [&str; 3] = [
    "⏱️ *Yah telat, waktu habis!*",
    "⏱️ *WAKTU HABIS!*",
    "⏱️ *Telat bro, waktu dah abis!*",
];

/// `SURRENDER_MESSAGES` (`games.js:47-51`).
pub const SURRENDER_MESSAGES: [&str; 3] = [
    "🏳️ *Yahhh nyerah deh...*",
    "🏳️ *MENYERAH!*",
    "🏳️ *Yah sayang banget nyerah...*",
];

// ============================================================================
// Konfigurasi game — padanan `defaults` di `register` (`games.js:63-76`)
// ============================================================================

/// Konfigurasi satu jenis game. Semua field sesuai kunci di JS.
pub struct GameConfig {
    pub data_file: String,
    pub question_field: String,
    pub answer_field: String,
    pub emoji: String,
    pub title: String,
    pub description: String,
    /// Timeout dalam milidetik (JS: `timeout: 60000`).
    pub timeout: i64,
    /// Cooldown dalam detik (JS: `cooldown: 5`).
    pub cooldown: i64,
    pub has_image: bool,
    pub image_field: String,
    pub alias: Vec<String>,
    pub hint_count: i64,
    pub game_type: String,
}

impl GameConfig {
    /// Default untuk `game_type`, padanan objek `defaults` di JS.
    pub fn default_for(game_type: &str) -> Self {
        Self {
            data_file: format!("{game_type}.json"),
            question_field: "soal".to_string(),
            answer_field: "jawaban".to_string(),
            emoji: "🎮".to_string(),
            title: game_type.to_uppercase(),
            description: format!("Game {game_type}"),
            timeout: 60_000,
            cooldown: 5,
            has_image: false,
            image_field: "img".to_string(),
            alias: Vec::new(),
            hint_count: 2,
            game_type: game_type.to_string(),
        }
    }
}

/// `register(gameType, cfg)` (`games.js:62-78`) — daftarkan game; `cfg`
/// menimpa default, lalu `gameType` ditetapkan.
#[must_use]
pub fn register_config(game_type: &str, cfg: GameConfig) -> GameConfig {
    let mut full = GameConfig::default_for(game_type);
    if !cfg.data_file.is_empty() {
        full.data_file = cfg.data_file;
    }
    if !cfg.question_field.is_empty() {
        full.question_field = cfg.question_field;
    }
    if !cfg.answer_field.is_empty() {
        full.answer_field = cfg.answer_field;
    }
    if !cfg.emoji.is_empty() {
        full.emoji = cfg.emoji;
    }
    if !cfg.title.is_empty() {
        full.title = cfg.title;
    }
    if !cfg.description.is_empty() {
        full.description = cfg.description;
    }
    if cfg.timeout > 0 {
        full.timeout = cfg.timeout;
    }
    if cfg.cooldown > 0 {
        full.cooldown = cfg.cooldown;
    }
    full.has_image = cfg.has_image;
    if !cfg.image_field.is_empty() {
        full.image_field = cfg.image_field;
    }
    if !cfg.alias.is_empty() {
        full.alias = cfg.alias;
    }
    if cfg.hint_count > 0 {
        full.hint_count = cfg.hint_count;
    }
    full.game_type = game_type.to_string();
    full
}

// ============================================================================
// FotoGames — padanan class di `games.js:57-328`
// ============================================================================

/// `FotoGames` (`games.js:57-60`) — registry game dengan `HashMap`.
pub struct FotoGames {
    pub registry: HashMap<String, GameConfig>,
}

impl FotoGames {
    /// `constructor()` (`games.js:58-60`) — inisialisasi registry kosong.
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// `register(gameType, cfg)` (`games.js:62-78`).
    pub fn register(&mut self, game_type: &str, cfg: GameConfig) {
        let full = register_config(game_type, cfg);
        self.registry.insert(game_type.to_string(), full);
    }

    /// `get(gameType)` (`games.js:80-82`).
    pub fn get(&self, game_type: &str) -> Option<&GameConfig> {
        self.registry.get(game_type)
    }
}

/// `games` instance (`games.js:330`) — padanan singleton JS.
///
/// `ponytail:` JS memiliki satu instance module-level `games`. Di Rust
/// ini adalah konstruktor; pemanggil yang ingin singleton sama harus
/// membagikan `FotoGames` melalui `Arc<Mutex<_>>` atau state router.
pub fn games() -> FotoGames {
    FotoGames::new()
}

// ============================================================================
// Helper murni dari createHandler / answerHandler (`games.js:186-296`)
// ============================================================================

/// Hasil perhitungan hadiah kemenangan.
pub struct RewardResult {
    pub total_limit: i64,
    pub total_balance: i64,
    pub total_exp: i64,
    pub bonus_text: String,
}

/// Hitung total hadiah dari config + fast answer bonus.
///
/// Padanan cabang `cfg.rewards` + `checkFastAnswer` di
/// `games.js:222-247`. `rewards` = `None` berarti `false`/`null` (tidak ada
/// hadiah), `Some` berarti objek rewards dari config.
pub fn calculate_rewards(
    rewards: Option<&RewardConfig>,
    fast: Option<&FastBonus>,
) -> RewardResult {
    let mut total_limit = 0;
    let mut total_balance = 0;
    let mut total_exp = 0;

    if let Some(r) = rewards {
        total_limit = r.limit.max(r.energi).max(0);
        total_balance = r.koin.max(r.balance).max(0);
        total_exp = r.exp.max(0);
    }

    let mut bonus_text = String::new();
    if let Some(f) = fast {
        total_limit += f.bonus.limit;
        total_balance += f.bonus.koin;
        total_exp += f.bonus.exp;
        bonus_text = format!(
            "\n\n{}\n⚡ *BONUS KILAT:* +{} Limit, +{} Koin\n⏱️ Waktu: *{:.1}s*",
            f.praise,
            f.bonus.limit,
            f.bonus.koin,
            f.elapsed_ms as f64 / 1000.0
        );
    }

    RewardResult {
        total_limit,
        total_balance,
        total_exp,
        bonus_text,
    }
}

/// Konfigurasi rewards dari `cfg.rewards` (`games.js:222-233`).
pub struct RewardConfig {
    pub limit: i64,
    pub energi: i64,
    pub koin: i64,
    pub balance: i64,
    pub exp: i64,
}

/// Hasil `checkFastAnswer` + bonus (`games.js:237-247`).
pub struct FastBonus {
    pub praise: String,
    pub bonus: RewardConfig,
    /// Waktu yang telah berlalu dalam milidetik.
    pub elapsed_ms: i64,
}

/// Bangun teks kemenangan, padanan `games.js:258-270`.
pub fn build_win_text(
    winner_number: &str,
    answer: &str,
    attempts: i64,
    total_limit: i64,
    total_balance: i64,
    total_exp: i64,
    bonus_text: &str,
) -> String {
    let mut text = pick(&WIN_MESSAGES)
        .unwrap_or("🎉 *Kemenangan!*")
        .to_string();
    text.push_str(&format!("\n\nJawaban: *{answer}*\n"));
    text.push_str(&format!("Pemenang: *@{winner_number}*\n"));
    text.push_str(&format!("Percobaan: *{attempts}x*\n\n"));

    if total_limit > 0 || total_balance > 0 || total_exp > 0 {
        let mut parts: Vec<&str> = Vec::new();
        if total_limit > 0 {
            parts.push(&format!("+{total_limit} Limit"));
        }
        if total_balance > 0 {
            parts.push(&format!("+{total_balance} Koin"));
        }
        if total_exp > 0 {
            parts.push(&format!("+{total_exp} EXP"));
        }
        text.push_str(&format!("🎁 {}", parts.join(", ")));
    }
    text.push_str(bonus_text);
    text
}

/// Bangun teks waktu habis, padanan `games.js:179-182`.
pub fn build_timeout_text(answer: &str) -> String {
    let mut text = pick(&TIMEOUT_MESSAGES)
        .unwrap_or("⏱️ Waktu habis!")
        .to_string();
    text.push_str(&format!("\n\nJawaban: *{answer}*\n\n"));
    text.push_str("_Gak ada yang bisa jawab nih~_");
    text
}

/// Bangun teks menyerah, padanan `games.js:198-200`.
pub fn build_surrender_text(winner_number: &str, answer: &str) -> String {
    let mut text = pick(&SURRENDER_MESSAGES)
        .unwrap_or("🏳️ Nyerah!")
        .to_string();
    text.push_str(&format!("\n\nJawaban: *{answer}*\n\n"));
    text.push_str(&format("_@{winner_number} menyerah_"));
    text
}

/// Bangun teks "hampir" (close), padanan `games.js:276-283`.
pub fn build_close_text(percent: u32, remaining_secs: i64) -> String {
    format!(
        "🔥 *Hampir!* Jawabanmu *{percent}%* mirip!\n_Sisa waktu: *{}*_",
        format_remaining(remaining_secs)
    )
}

/// Bangun teks salah, padanan `games.js:289-292`.
pub fn build_wrong_text(hint: &str, remaining_secs: i64) -> String {
    format!(
        "❌ Belum bener! Hint: *{hint}*\n_Sisa: *{}*_",
        format_remaining(remaining_secs)
    )
}

/// `formatRemainingTime` — sisa waktu dalam detik ditampilkan sebagai
/// `X detik`. Padanan JS yang memakai `formatRemainingTime` dari
/// `./game-data.js`.
pub fn format_remaining(secs: i64) -> String {
    if secs <= 0 {
        "0 detik".to_string()
    } else {
        format!("{secs} detik")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------- pick

    #[test]
    fn pick_mengembalikan_satu_item_dari_array() {
        let arr = ["a", "b", "c"];
        let hasil = pick(&arr).expect("harus ada hasil");
        assert!(arr.iter().any(|&s| s == hasil), "{hasil} harus ada di {arr:?}");
    }

    #[test]
    fn pick_array_kosong_mengembalikan_none() {
        assert_eq!(pick(&[] as &[&str]), None);
    }

    // ------------------------------------------------- GameConfig defaults

    #[test]
    fn default_config_sama_dengan_js() {
        let cfg = GameConfig::default_for("tebakan");
        assert_eq!(cfg.data_file, "tebakan.json");
        assert_eq!(cfg.question_field, "soal");
        assert_eq!(cfg.answer_field, "jawaban");
        assert_eq!(cfg.emoji, "🎮");
        assert_eq!(cfg.title, "TEBAKAN");
        assert_eq!(cfg.description, "Game tebakan");
        assert_eq!(cfg.timeout, 60_000);
        assert_eq!(cfg.cooldown, 5);
        assert!(!cfg.has_image);
        assert_eq!(cfg.image_field, "img");
        assert_eq!(cfg.alias, Vec::<String>::new());
        assert_eq!(cfg.hint_count, 2);
        assert_eq!(cfg.game_type, "tebakan");
    }

    // ------------------------------------------------- register_config / FotoGames

    #[test]
    fn register_config_menimpa_default() {
        let mut cfg = GameConfig::default_for("tebakan");
        cfg.timeout = 120_000;
        cfg.cooldown = 10;
        cfg.hint_count = 3;
        let full = register_config("tebakan", cfg);
        assert_eq!(full.timeout, 120_000);
        assert_eq!(full.cooldown, 10);
        assert_eq!(full.hint_count, 3);
        // Field yang tidak di-override tetap default.
        assert_eq!(full.data_file, "tebakan.json");
        assert_eq!(cfg.emoji, "🎮");
    }

    #[test]
    fn foto_games_register_dan_get() {
        let mut fg = FotoGames::new();
        let cfg = GameConfig::default_for("suitek");
        fg.register("suitek", cfg);
        assert!(fg.get("suitek").is_some());
        assert_eq!(fg.get("suitek").unwrap().game_type, "suitek");
        assert!(fg.get("tidakAda").is_none());
    }

    // ------------------------------------------------- calculate_rewards

    #[test]
    fn calculate_rewards_tanpa_rewards_adalah_nol() {
        let r = calculate_rewards(None, None);
        assert_eq!(r.total_limit, 0);
        assert_eq!(r.total_balance, 0);
        assert_eq!(r.total_exp, 0);
        assert_eq!(r.bonus_text, "");
    }

    #[test]
    fn calculate_rewards_dengan_rewards() {
        let rewards = RewardConfig {
            limit: 100,
            energi: 0,
            koin: 50,
            balance: 0,
            exp: 200,
        };
        let r = calculate_rewards(Some(&rewards), None);
        assert_eq!(r.total_limit, 100);
        assert_eq!(r.total_balance, 50);
        assert_eq!(r.total_exp, 200);
        assert_eq!(r.bonus_text, "");
    }

    #[test]
    fn calculate_rewards_dengan_fast_bonus() {
        let rewards = RewardConfig {
            limit: 100,
            energi: 0,
            koin: 50,
            balance: 0,
            exp: 200,
        };
        let fast = FastBonus {
            praise: "Cepat banget!".to_string(),
            bonus: RewardConfig {
                limit: 10,
                energi: 0,
                koin: 5,
                balance: 0,
                exp: 20,
            },
            elapsed_ms: 2500,
        };
        let r = calculate_rewards(Some(&rewards), Some(&fast));
        assert_eq!(r.total_limit, 110);
        assert_eq!(r.total_balance, 55);
        assert_eq!(r.total_exp, 220);
        assert!(r.bonus_text.contains("BONUS KILAT"));
        assert!(r.bonus_text.contains("+10 Limit"));
        assert!(r.bonus_text.contains("+5 Koin"));
        assert!(r.bonus_text.contains("2.5s"));
    }

    // ------------------------------------------------- build_win_text

    #[test]
    fn build_win_text_mengandung_jawaban_dan_pemenang() {
        let text = build_win_text("628111", "kucing", 3, 100, 50, 200, "");
        assert!(text.contains("Jawaban: *kucing*"));
        assert!(text.contains("Pemenang: *@628111*"));
        assert!(text.contains("Percobaan: *3x*"));
        assert!(text.contains("+100 Limit"));
        assert!(text.contains("+50 Koin"));
        assert!(text.contains("+200 EXP"));
    }

    #[test]
    fn build_win_text_dengan_bonus() {
        let text = build_win_text("628111", "kucing", 1, 0, 0, 0, "\n⚡ *BONUS KILAT:* +5 Limit");
        assert!(text.contains("BONUS KILAT"));
    }

    // ------------------------------------------------- build_timeout_text

    #[test]
    fn build_timeout_text_mengandung_jawaban() {
        let text = build_timeout_text("rahasia");
        assert!(text.contains("Jawaban: *rahasia*"));
        assert!(text.contains("Gak ada yang bisa jawab nih~"));
    }

    // ------------------------------------------------- build_surrender_text

    #[test]
    fn build_surrender_text_mengandung_pemenang_dan_jawaban() {
        let text = build_surrender_text("628111", "rahasia");
        assert!(text.contains("Jawaban: *rahasia*"));
        assert!(text.contains("@628111 menyerah"));
    }

    // ------------------------------------------------- build_close_text

    #[test]
    fn build_close_text_mengandung_persentase_dan_waktu() {
        let text = build_close_text(75, 30);
        assert!(text.contains("Hampir!"));
        assert!(text.contains("*75%* mirip"));
        assert!(text.contains("30 detik"));
    }

    // ------------------------------------------------- build_wrong_text

    #[test]
    fn build_wrong_text_mengandung_hint_dan_waktu() {
        let text = build_wrong_text("huruf K", 15);
        assert!(text.contains("Belum bener!"));
        assert!(text.contains("Hint: *huruf K*"));
        assert!(text.contains("15 detik"));
    }

    // ------------------------------------------------- format_remaining

    #[test]
    fn format_remaining_nol_dan_negatif() {
        assert_eq!(format_remaining(0), "0 detik");
        assert_eq!(format_remaining(-5), "0 detik");
    }

    #[test]
    fn format_remaining_positif() {
        assert_eq!(format_remaining(42), "42 detik");
    }
}