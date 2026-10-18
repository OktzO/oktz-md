//! `plugins/game/family100.js` — tebak jawaban teratas survei.
//!
//! `ponytail:` `family100AnswerHandler` bukan command; router global yang
//! memanggilnya (sama seperti `kyubigameAnswerHandler` di `kyubigame.js`).
//! Yang dipindah di sini `handler`.
//!
//! Bank soal `src/data/family100.json` di-`include_str!` apa adanya, bukan
//! disalin jadi konstanta Rust: 7152 soal / 1,6 MB jauh lebih besar dari file
//! plugin mana pun, dan menyalinnya jadi array `const` berarti data punya dua
//! sumber yang bisa melenceng diam-diam.
//!
//! `ponytail:` `setSessionTimer` butuh event loop, sedangkan `Reply` tidak
//! punya timer. Sesi yang lewat 120 detik dianggap habis saat dibaca
//! berikutnya — efek yang sama dengan sweeper 5-menit di `game-data.js`,
//! tanpa timer yang benar-benar memanggil `endSession`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value};

/// `createSession(..., 120000)` (`family100.js:84`).
pub(crate) const SESSION_TIMEOUT_MS: i64 = 120_000;

/// `gameSessions` di `game-data.js:7` — satu map bersama untuk semua game,
/// di-key `chatId`. Nama key dipertahankan supaya plugin game lain yang juga
/// memakainya tidak membuat sesi kedua untuk chat yang sama.
pub(crate) const SESSIONS_KEY: &str = "gameSessions";

const BANK: &str = include_str!("../../../../../src/data/family100.json");

/// Satu baris `src/data/family100.json`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Question {
    pub soal: String,
    pub jawaban: Vec<String>,
}

fn bank() -> &'static Vec<Question> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<Question>> = OnceLock::new();
    CACHE.get_or_init(|| {
        serde_json::from_str::<Vec<Value>>(BANK)
            .map(|rows| {
                rows.iter()
                    .map(|row| Question {
                        soal: row.get("soal").and_then(Value::as_str).unwrap_or("").to_string(),
                        jawaban: row
                            .get("jawaban")
                            .and_then(Value::as_array)
                            .map(|a| {
                                a.iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// `getRandomItem("family100.json")` — `None` kalau bank kosong
/// (`family100.js:59-63`).
pub(crate) fn random_question() -> Option<&'static Question> {
    use rand::Rng as _;
    let b = bank();
    if b.is_empty() {
        return None;
    }
    b.get(rand::rng().random_range(0..b.len()))
}

/// `formatRemainingTime` (`game-data.js:189`).
pub(crate) fn format_remaining_time(seconds: i64) -> String {
    if seconds <= 0 {
        return String::from("0 detik");
    }
    if seconds < 60 {
        return format!("{seconds} detik");
    }
    format!("{}m {}s", seconds / 60, seconds % 60)
}

/// `getRemainingTime` (`game-data.js:181`) — detik, `Math.ceil`, tidak negatif.
pub(crate) fn remaining_time(end_time: i64, now: i64) -> i64 {
    (((end_time - now) as f64) / 1000.0).ceil().max(0.0) as i64
}

fn session_of(ctx: &Ctx) -> Option<Value> {
    ctx.session_get(SESSIONS_KEY)?.get(&ctx.chat).cloned()
}

/// `family100.js:43-53` — kartu "sesi masih jalan".
pub(crate) fn in_progress(session: &Value, remaining_secs: i64) -> String {
    let question = session.get("question");
    let soal = question
        .and_then(|q| q.get("soal"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let total = question
        .and_then(|q| q.get("jawaban"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let answered: Vec<String> = session
        .get("answered")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut text = format!("Wah, sesi Family 100 masih jalan nih kak! 😱✨\n\n*{soal}*\n\n");
    text.push_str(&format!(
        "Terjawab: *{} dari {total}*\n",
        answered.len()
    ));
    for (i, ans) in answered.iter().enumerate() {
        text.push_str(&format!("{}. ✅ {ans}\n", i + 1));
    }
    for i in answered.len()..total {
        text.push_str(&format!("{}. ❓ ???\n", i + 1));
    }
    text.push_str(&format!(
        "\nSisa waktu: *{}* ⏳\nBuruan di-reply pesannya buat jawab! 🔥",
        format_remaining_time(remaining_secs)
    ));
    text
}

/// `family100.js:67-75` — kartu soal baru.
pub(crate) fn question_card(question: &Question) -> String {
    let mut text = format!(
        "Waktunya main *FAMILY 100*! 🎉✨\n\n*Pertanyaan:* {}\n\nTotal Jawaban: *{}* 📝\n",
        question.soal,
        question.jawaban.len()
    );
    for i in 1..=question.jawaban.len() {
        text.push_str(&format!("{i}. ❓ ???\n"));
    }
    text.push_str(
        "\nWaktu kamu cuman *120 detik* aja ya! ⏱️\n\
         Hadiahnya? Random *EXP* & *Koin* buat setiap jawaban bener! 🎁💸\n\n\
         Cara main: langsung *reply pesan ini* dengan jawabanmu, atau reply dengan kata *nyerah* kalau udah pusing banget 🏳️😂",
    );
    text
}

/// `family100.js:61` — bank soal kosong.
pub(crate) const NO_QUESTION: &str = "Yah maaf banget kak, soal gamenya lagi kosong nih 😭💔";

#[plugin(
    name = "family100",
    alias = ["f100"],
    category = "game",
    description = "Survey says! Tebak jawaban teratas survei",
    usage = ".family100",
    example = ".family100",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn family100(ctx: Ctx) -> Reply {
    let now = crate::common::now_ms();
    if let Some(session) = session_of(&ctx)
        && session.get("gameType").and_then(Value::as_str) == Some("family100")
    {
        let end_time = session.get("endTime").and_then(Value::as_i64).unwrap_or(0);
        let remaining = remaining_time(end_time, now);
        if remaining > 0 {
            return Reply::text(in_progress(&session, remaining));
        }
        // Lewat waktu. Di JS `setSessionTimer` yang memanggil `endSession`;
        // tanpa timer, sesi dibuang di sini.
        ctx.session_take(SESSIONS_KEY);
    }

    let Some(question) = random_question() else {
        return Reply::text(NO_QUESTION);
    };

    let chat = ctx.chat.clone();
    ctx.session_update(SESSIONS_KEY, |sessions| {
        // `session_update` memberi `Value::Null` untuk key baru, sedangkan JS
        // menulis ke objek `gameSessions` yang sudah ada.
        if !sessions.is_object() {
            *sessions = serde_json::json!({});
        }
        let entry = sessions
            .as_object_mut()
            .expect("baru saja diganti objek")
            .entry(chat.clone())
            .or_insert(Value::Null);
        *entry = serde_json::json!({
            "chatId": chat,
            "gameType": "family100",
            "question": { "soal": question.soal, "jawaban": question.jawaban },
            "answered": [],
            "answeredBy": {},
            "startTime": now,
            "endTime": now + SESSION_TIMEOUT_MS,
        });
    });

    Reply::text(question_card(question))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAT: &str = "120363@g.us";
    const SENDER: &str = "628111@s.whatsapp.net";

    fn base_ctx() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "family100".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            is_group: true,
            ..Default::default()
        }
    }

    /// `Ctx` dengan sesi yang sudah ada, untuk menguji cabang "masih jalan".
    fn ctx_with(sessions: Value) -> Ctx {
        let ctx = base_ctx();
        ctx.session_set(SESSIONS_KEY, sessions);
        ctx
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("family100").unwrap().handler)(ctx)
    }

    /// Dimiliki (bukan `&str`) supaya bisa dipakai langsung pada balasan
    /// temporer tanpa pinjaman yang lebih panjang dari statement.
    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("family100").unwrap();
        assert_eq!(p.meta.name, "family100");
        assert_eq!(p.meta.alias, ["f100"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(
            p.meta.description,
            "Survey says! Tebak jawaban teratas survei"
        );
        assert_eq!(p.meta.usage, ".family100");
        assert_eq!(p.meta.example, ".family100");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_alias_f100_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("f100").unwrap().meta.name,
            "family100"
        );
    }

    #[test]
    fn the_bank_is_the_original_json_not_a_trimmed_copy() {
        assert!(bank().len() > 1000, "bank soal terpotong: {}", bank().len());
        let q = &bank()[0];
        assert_eq!(q.soal, "Hotel Bintang 5 Jakarta");
        assert_eq!(
            q.jawaban,
            [
                "sheraton",
                "the westin",
                "kempinski",
                "the langham",
                "borobudur",
                "hilton"
            ]
        );
    }

    #[test]
    fn a_fresh_game_posts_the_question_and_writes_a_120_second_session() {
        let ctx = base_ctx();
        let before = crate::common::now_ms();
        let r = run(ctx.clone());
        let t = text_of(&r);
        assert!(t.starts_with("Waktunya main *FAMILY 100*"), "{t}");
        assert!(t.contains("120 detik"), "{t}");
        assert!(r.mutations().is_empty(), "state ditulis lewat session, bukan mutasi");

        let s = ctx.session_get(SESSIONS_KEY).unwrap()[CHAT].clone();
        assert_eq!(s["gameType"], serde_json::json!("family100"));
        assert_eq!(s["chatId"], serde_json::json!(CHAT));
        assert_eq!(s["answered"], serde_json::json!([]));
        assert_eq!(s["answeredBy"], serde_json::json!({}));
        let end = s["endTime"].as_i64().unwrap();
        let start = s["startTime"].as_i64().unwrap();
        assert!(
            start >= before && start <= crate::common::now_ms(),
            "startTime {start} bukan sekarang"
        );
        assert_eq!(end - start, SESSION_TIMEOUT_MS, "deadline harus 120 detik");
        // Kartu yang dikirim harus soal yang benar-benar tersimpan.
        assert!(
            t.contains(s["question"]["soal"].as_str().unwrap()),
            "kartu dan sesi harus soal yang sama: {t}"
        );
        assert_eq!(
            s["question"]["jawaban"].as_array().unwrap().len(),
            t.matches("❓ ???").count(),
            "jumlah baris kartu harus sama dengan jumlah jawaban"
        );
    }

    #[test]
    fn a_second_call_shows_the_running_session_instead_of_a_new_question() {
        let now = crate::common::now_ms();
        let t = text_of(&run(ctx_with(serde_json::json!({
            CHAT: {
                "gameType": "family100",
                "question": { "soal": "Hotel Bintang 5 Jakarta", "jawaban": ["sheraton", "hilton"] },
                "answered": ["sheraton"],
                "answeredBy": { "sheraton": SENDER },
                "startTime": now,
                "endTime": now + SESSION_TIMEOUT_MS,
            }
        }))));
        assert!(t.starts_with("Wah, sesi Family 100 masih jalan"), "{t}");
        assert!(t.contains("Hotel Bintang 5 Jakarta"), "{t}");
        assert!(t.contains("Terjawab: *1 dari 2*"), "{t}");
        assert!(t.contains("1. ✅ sheraton"), "{t}");
        assert!(t.contains("2. ❓ ???"), "{t}");
        assert!(
            t.contains("Sisa waktu: *1m 59s*") || t.contains("Sisa waktu: *2m 0s*"),
            "sisa waktu harus dihitung dari deadline: {t}"
        );
    }

    #[test]
    fn a_session_from_another_game_does_not_block_a_new_family100() {
        let now = crate::common::now_ms();
        let t = text_of(&run(ctx_with(serde_json::json!({
            CHAT: {
                "gameType": "tictactoe",
                "question": { "soal": "?", "jawaban": [] },
                "startTime": now,
                "endTime": now + SESSION_TIMEOUT_MS,
            }
        }))));
        assert!(t.starts_with("Waktunya main *FAMILY 100*"), "{t}");
    }

    #[test]
    fn an_expired_session_is_replaced_by_a_new_one() {
        let now = crate::common::now_ms();
        let ctx = ctx_with(serde_json::json!({
            CHAT: {
                "gameType": "family100",
                "question": { "soal": "Sesi Lama", "jawaban": ["x"] },
                "answered": ["x"],
                "startTime": now - SESSION_TIMEOUT_MS - 1_000,
                "endTime": now - 1_000,
            }
        }));
        let __balasan = run(ctx.clone());
        let t = text_of(&__balasan);
        assert!(t.starts_with("Waktunya main *FAMILY 100*"), "{t}");
        assert!(!t.contains("Sesi Lama"), "sesi kedaluwarsa tidak boleh tampil");
        assert_ne!(
            ctx.session_get(SESSIONS_KEY).unwrap()[CHAT]["question"]["soal"],
            serde_json::json!("Sesi Lama")
        );
    }

    // `createSession` (game-data.js:149) hanya membersihkan timer sesi lama lalu
    // `gameSessions.set(chatId, session)` tanpa mengecek `gameType` — sesi game
    // lain memang DITIMPA, bukan dipertahankan.
    #[test]
    fn a_running_session_of_another_game_is_replaced_by_the_new_one() {
        let now = crate::common::now_ms();
        let ctx = ctx_with(serde_json::json!({
            CHAT: {
                "gameType": "tictactoe",
                "question": { "soal": "?", "jawaban": [] },
                "startTime": now,
                "endTime": now + SESSION_TIMEOUT_MS,
            }
        }));
        let __balasan = run(ctx.clone());
        let t = text_of(&__balasan);
        assert!(t.starts_with("Waktunya main *FAMILY 100*"), "{t}");
        let s = ctx.session_get(SESSIONS_KEY).unwrap()[CHAT].clone();
        assert_eq!(s["gameType"], serde_json::json!("family100"), "sesi lama harus tertimpa");
        assert_ne!(s["question"]["soal"], serde_json::json!("?"), "soal baru harus tersimpan");
    }

    #[test]
    fn remaining_time_rounds_up_and_never_goes_negative() {
        assert_eq!(remaining_time(1_000, 0), 1, "999ms harus 1 detik");
        assert_eq!(remaining_time(120_000, 0), 120);
        assert_eq!(remaining_time(0, 0), 0);
        assert_eq!(remaining_time(0, 5_000), 0, "kedaluwarsa tidak boleh negatif");
    }

    #[test]
    fn the_remaining_time_labels_follow_the_js_bands() {
        assert_eq!(format_remaining_time(0), "0 detik");
        assert_eq!(format_remaining_time(1), "1 detik");
        assert_eq!(format_remaining_time(59), "59 detik");
        assert_eq!(format_remaining_time(60), "1m 0s");
        assert_eq!(format_remaining_time(120), "2m 0s");
        assert_eq!(format_remaining_time(125), "2m 5s");
    }

    #[test]
    fn the_question_card_lists_one_row_per_answer() {
        let t = question_card(&Question {
            soal: "Uji".into(),
            jawaban: vec!["a".into(), "b".into(), "c".into()],
        });
        assert!(t.contains("Total Jawaban: *3* 📝"), "{t}");
        assert!(t.contains("1. ❓ ???\n2. ❓ ???\n3. ❓ ???\n"), "{t}");
        assert_eq!(t.matches("❓ ???").count(), 3, "{t}");
    }
}