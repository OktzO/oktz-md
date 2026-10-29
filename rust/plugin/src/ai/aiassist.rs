//! `plugins/ai/aiassist.js` — chat dengan AI Assistant (backend UnlimitedAI).
//!
//! File ini juga menjadi rumah helper `UnlimitedAI` untuk plugin sekelas di
//! direktori ini (`jokowi-ai`, `kobo-ai`, `prabowo-ai` memanggil fungsi yang
//! sama dengan karakter berbeda). Helper-nya belum ada di `common.rs` —
//! laporan batch menyebut pemindahannya ke sana.
//!
//! `ponytail:` `HttpGet` tidak punya setter header per-request, jadi header
//! `cookie`, `sec-ch-ua*`, `origin`, dan `referer` yang dikirim JS tidak ikut.
//! akibatnya `!response.ok` (403/500) tidak pernah terlihat sebagai `status:false`
//! seperti di JS — `UreqHttp` mengubahnya jadi `Err` lebih dulu, jadi yang tampil
//! template error `catch`, bukan `❌ *AI Assistant Error*`. Isi `chatId`,
//! `deviceId`, `selectedChatModel`, dan `locale` tetap sama.

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::{Value};

use crate::common::{JAKARTA_OFFSET_SECS, jakarta_parts, now_ms, roll};

/// `API` (`src/scraper/unlimitedai.js:3`).
const ENDPOINT: &str = "https://app.unlimitedai.chat/api/chat";

/// Batas potong `reply.slice(0, 4096)` (`aiassist.js:45`).
const MAX_LEN: usize = 4096;

/// Kegagalan `UnlimitedAI` — dua cabang yang di JS terpisah.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AiFailure {
    /// `fetch` melempar (JS: masuk `catch`) — di sini juga `post_json` `Err`,
    /// karena `UreqHttp` tidak mengembalikan body untuk status >= 400.
    Transport,
    /// `!response.ok` — JS dapat `status:false` plus teks body sebagai `error`.
    Rejected(String),
}

pub(crate) type AiAnswer = Result<String, AiFailure>;

/// `reply.length > 4096 ? reply.slice(0, 4096) + "..." : reply`.
///
/// `ponytail:` `slice` JS menghitung UTF-16 code unit, `chars()` menghitung
/// code point. Bedanya cuma terlihat kalau 4096 batasnya jatuh di tengah emoji.
pub(crate) fn clip(s: &str) -> String {
    if s.chars().count() > MAX_LEN {
        let head: String = s.chars().take(MAX_LEN).collect();
        format!("{head}...")
    } else {
        s.to_string()
    }
}

/// `crypto.randomUUID()` — 32 hex acak dengan bit versi `4` dan varian `a`.
pub(crate) fn uuid_v4() -> String {
    let mut hex = String::with_capacity(32);
    for _ in 0..32 {
        // `from_digit(.., 16)` selalu `Some` untuk rentang 0..16.
        hex.push(char::from_digit(roll(16), 16).unwrap_or('0'));
    }
    format!(
        "{}-{}-4{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

/// `new Date().toISOString()` — `jakarta_parts` menambah 7 jam, jadi
/// dikurangi dulu supaya yang dihasilkan_parts benar-benar UTC.
fn iso_utc_now() -> String {
    let ms = now_ms();
    let (y, mo, d, h, mi, s) = jakarta_parts(ms - JAKARTA_OFFSET_SECS * 1000);
    format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{:03}Z",
        ms.rem_euclid(1000)
    )
}

/// `CHARACTERS["ai-assistant"].prompt` — nilai yang dipakai
/// `CHARACTERS[character] || CHARACTERS["ai-assistant"]`
/// (`src/scraper/unlimitedai.js:37-63`) saat kunci tidak dikenal.
const AI_ASSISTANT_PROMPT: &str = "Kamu adalah AI Assistant, asisten WhatsApp bot yang ramah, cerdas, dan responsif. Kamu menjawab dalam bahasa Indonesia dengan gaya santai tapi tetap informatif. Kamu ahli dalam teknologi, programming, dan hal-hal umum. Jawab dengan singkat, jelas, dan natural. Gunakan emoji secukupnya untuk membuat percakapan lebih hidup.";


/// (`src/scraper/unlimitedai.js:37-63`) — hanya `prompt` yang dipakai; `name`
/// hanya diisi ke objek hasil yang tidak pernah dibaca plugin mana pun.
pub(crate) fn character_prompt(key: &str) -> &'static str {
    match key {
        "ai-assistant" => "Kamu adalah AI Assistant, asisten WhatsApp bot yang ramah, cerdas, dan responsif. Kamu menjawab dalam bahasa Indonesia dengan gaya santai tapi tetap informatif. Kamu ahli dalam teknologi, programming, dan hal-hal umum. Jawab dengan singkat, jelas, dan natural. Gunakan emoji secukupnya untuk membuat percakapan lebih hidup.",
        "kobo-ai" => "Kamu adalah Kobo Kanaeru, VTuber dari Hololive Indonesia gen 3. Kamu gadis cheerfull, energetic, dan sedikit tsundere. Kamu bicara pakai bahasa Indonesia casual campur bahasa Jawa dan sedikit Jepang. Kamu suka bilang \"DAJOOR!\", \"HMPH!\", dan \"EHE~\". Kamu itu wind shaman yang suka nge-prank dan bikin lelucon. Kamu panggil user \"Kobo-kun\" atau \"Anon\". Kamu suka makan dan sering ngomong soal makanan. Gaya bicaramu imut tapi kadang galak kalau diprank. Kamu jawab dengan gaya Kobo yang asli, jangan kaku.",
        "waguri-ai" => "Kamu adalah Waguri-san, gadis pemalu tapi perhatian dari manga \"The Girl I Like Forgot Her Glasses\". Kamu bicara pelan, lembut, dan sering salah tingkah kalau dipuji. Kamu sering lupa pakai kacamata jadi pandanganmu kadang kabur. Kamu bicara pakai bahasa Indonesia dengan gaya pemalu dan manis, sering pakai \"E-eto...\", \"A-ano...\", dan \"Gomen...\". Kamu sangat perhatian ke orang lain dan suka membantu meski malu-malu. Kamu panggil user \"Kaichou\" atau \"Senpai\". Jawab dengan gaya manis dan sedikit tsundere.",
        "jokowi-ai" => "Kamu adalah Joko Widodo (Jokowi), mantan Presiden RI yang asli Solo. Kamu bicara pakai bahasa Indonesia dengan logat Jawa, sederhana, dan down-to-earth. Kamu suka bilang \"Lha\", \"Nah itu lho\", \"Monggo\", dan \"Sami-sami\". Kamu sering cerita soal pembangunan, infrastruktur, dan pengalaman blusukan. Kamu panggil user \"Mbak\", \"Mas\", atau \"Saudara\". Kamu jawab dengan gaya sederhana tapi bijak, pakai analogi kehidupan sehari-hari. Kamu sering pakai bahasa Jawa halus seperti \"Niku\", \"Nggih\", \"Monggo\". Kamu bangga sama Solo dan sering cerita soal Solo. Jawab dengan gaya Pak Jokowi yang asli, jangan kaku.",
        "prabowo-ai" => "Kamu adalah Prabowo Subianto, Presiden RI dan ketua umum Partai Gerindra. Kamu bicara dengan gaya tegas, patriotik, dan penuh semangat. Kamu suka bilang \"Saudara-saudara!\", \"Ini negeri kita!\", dan \"Kita harus berdaulat!\". Kamu sering bicara soal kedaulatan, kemandirian ekonomi, dan kekuatan bangsa. Kamu panggil user \"Saudara\" atau \"Pemuda\". Kamu sering pakai analogi militer dan strategi. Kamu sangat bangga dengan sawit dan sumber daya alam Indonesia. Kamu bicara dengan nada kuat dan meyakinkan. Kamu kadang pakai bahasa Jawa kasar seperti \"Nduk\", \"Ojo\". Jawab dengan gaya Pak Prabowo yang karismatik dan tegas, jangan kaku.",
        _ => AI_ASSISTANT_PROMPT,
    }
}

/// Parser NDJSON UnlimitedAI (`unlimitedai.js:141-151`).
///
/// `buffer.split("\n")` lalu `lines.pop()`: potongan terakhir TIDAK pernah
/// diproses kalau body tidak diakhiri newline. Ditiru di sini.
pub(crate) fn ndjson_answer(raw: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut answer = String::new();
    for line in lines.iter().take(lines.len() - 1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("type").and_then(Value::as_str) == Some("delta")
            && let Some(d) = v.get("delta").and_then(Value::as_str)
        {
            answer.push_str(d);
        }
    }
    answer
}

/// `UnlimitedAI(prompt, character)` (`src/scraper/unlimitedai.js:60-161`).
pub(crate) fn unlimited_ai(http: &dyn HttpGet, character: &str, prompt: &str) -> AiAnswer {
    let chat_id = uuid_v4();
    let device_id = uuid_v4();
    let system = format!(
        "{}\n\nPertanyaan user: {prompt}",
        character_prompt(character)
    );
    let created = iso_utc_now();
    let body = serde_json::json!({
        "chatId": chat_id,
        "messages": [
            {
                "id": uuid_v4(),
                "role": "user",
                "content": system,
                "parts": [{ "type": "text", "text": system }],
                "createdAt": created,
            },
            {
                "id": uuid_v4(),
                "role": "assistant",
                "content": "",
                "parts": [{ "type": "text", "text": "" }],
                "createdAt": created,
            },
        ],
        "selectedChatModel": "chat-model-reasoning",
        "selectedCharacter": Value::Null,
        "selectedStory": Value::Null,
        "deviceId": device_id,
        "locale": "id",
    });

    let res = http.post_json(ENDPOINT, &body).map_err(|_| AiFailure::Transport)?;
    // `!response.ok` (`unlimitedai.js:150`). Cabang ini praktis tidak terjangkau
    // karena `UreqHttp` sudah `Err` untuk status >= 400 — lihat `ponytail:`.
    if !(200..300).contains(&res.status) {
        return Err(AiFailure::Rejected(res.text()));
    }
    Ok(ndjson_answer(&res.text()))
}

/// `❌ *{judul}*\n\n> {error}` — `error || "Gagal mendapatkan respons"`.
pub(crate) fn failed(title: &str, err: &str) -> String {
    let detail = if err.is_empty() {
        "Gagal mendapatkan respons"
    } else {
        err
    };
    format!("❌ *{title}*\n\n> {detail}")
}

/// `aiassist.js:24-30`.
fn help(prefix: &str) -> String {
    format!(
        "🤖 *AI Assistant*\n\n\
         > Asisten cerdas siap membantu\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}aiassist <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}aiassist Apa itu Node.js?*"
    )
}

#[plugin(
    name = "aiassist",
    alias = ["asisten", "tanya-ai"],
    category = "ai",
    description = "Chat dengan AI Assistant — Asisten bot cerdas",
    usage = ".aiassist <pertanyaan>",
    example = ".aiassist Apa itu Node.js?",
    cooldown = 10,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn aiassist(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match unlimited_ai(&*ctx.http, "ai-assistant", &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("AI Assistant Error", &e)).with_reaction("☢")
        }
        Err(AiFailure::Transport) => Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// HTTP offline: mengembalikan body yang sudah disiapkan, atau gagal total.
    #[derive(Debug)]
    struct Stub {
        resp: Option<String>,
        seen: Mutex<Vec<(String, String)>>,
    }

    impl Stub {
        fn ok(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                resp: None,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn respond(&self) -> Result<HttpResponse, HttpError> {
            let Some(text) = &self.resp else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.clone().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push((url.into(), String::new()));
            self.respond()
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.into(), body.to_string()));
            self.respond()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
    }

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "aiassist".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("aiassist").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("aiassist").unwrap();
        assert_eq!(p.meta.name, "aiassist");
        assert_eq!(p.meta.alias, ["asisten", "tanya-ai"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Chat dengan AI Assistant — Asisten bot cerdas"
        );
        assert_eq!(p.meta.usage, ".aiassist <pertanyaan>");
        assert_eq!(p.meta.example, ".aiassist Apa itu Node.js?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["aiassist", "asisten", "tanya-ai"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("aiassist"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_and_never_calls_the_api() {
        let http = Stub::ok("");
        for text in ["", "   "] {
            let r = run(http.clone(), text);
            assert!(body(&r).contains("Asisten cerdas siap membantu"), "{text:?}");
            assert!(body(&r).contains("*.aiassist <pertanyaan>*"), "{text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn delta_chunks_are_joined_in_order() {
        assert_eq!(
            ndjson_answer(
                "{\"type\":\"delta\",\"delta\":\"Halo \"}\n\
                 {\"type\":\"delta\",\"delta\":\"dunia\"}\n\
                 {\"type\":\"done\"}\n"
            ),
            "Halo dunia"
        );
        // Baris bukan JSON dan baris tanpa `delta` dilewati tanpa menggagalkan.
        assert_eq!(
            ndjson_answer("bukan json\n{\"type\":\"meta\"}\n{\"delta\":\"x\"}\n"),
            ""
        );
    }

    #[test]
    fn the_trailing_partial_line_is_dropped_like_the_js_buffer() {
        // `lines.pop()` di JS membuang potongan terakhir; tanpa newline terakhir
        // delta terakhir hilang.
        assert_eq!(
            ndjson_answer("{\"type\":\"delta\",\"delta\":\"a\"}\n{\"type\":\"delta\",\"delta\":\"b\"}"),
            "a"
        );
    }

    #[test]
    fn a_normal_answer_is_reacted_and_sent_untruncated() {
        let http = Stub::ok("{\"type\":\"delta\",\"delta\":\"Jawabannya 4\"}\n");
        let r = run(http.clone(), "2+2 berapa?");
        assert_eq!(body(&r), "Jawabannya 4");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "harus tepat satu POST");
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["selectedChatModel"], "chat-model-reasoning");
        assert_eq!(sent["locale"], "id");
        assert_eq!(sent["messages"][0]["role"], "user");
        assert!(
            sent["messages"][0]["content"]
                .as_str()
                .unwrap()
                .ends_with("Pertanyaan user: 2+2 berapa?"),
            "prompt user harus appended di akhir system prompt"
        );
    }

    #[test]
    fn an_answer_longer_than_the_limit_is_cut_with_an_ellipsis() {
        let long = "x".repeat(5000);
        let http = Stub::ok(&format!(
            "{{\"type\":\"delta\",\"delta\":\"{long}\"}}\n"
        ));
        let r = run(http.clone(), "panjang");
        let t = body(&r);
        assert!(t.ends_with("..."), "{}", &t[t.len().saturating_sub(10)..]);
        assert_eq!(t.chars().filter(|c| *c == 'x').count(), MAX_LEN);
    }

    #[test]
    fn a_short_answer_is_sent_verbatim() {
        assert_eq!(clip("pendek"), "pendek");
        assert_eq!(
            clip(&"a".repeat(MAX_LEN + 1)).chars().count(),
            MAX_LEN + 3,
            "4096 + '...'"
        );
    }

    #[test]
    fn a_transport_failure_falls_back_to_the_shared_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "aiassist"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn an_empty_error_body_falls_back_to_the_default_wording() {
        assert_eq!(
            failed("AI Assistant Error", ""),
            "❌ *AI Assistant Error*\n\n> Gagal mendapatkan respons"
        );
        assert_eq!(
            failed("AI Assistant Error", "502"),
            "❌ *AI Assistant Error*\n\n> 502"
        );
    }

    #[test]
    fn the_request_carries_the_character_prompt_and_two_messages() {
        let http = Stub::ok("");
        let _ = run(http.clone(), "hai");
        let sent = http.seen.lock().unwrap()[0].1.clone();
        let v: Value = serde_json::from_str(&sent).unwrap();
        assert_eq!(v["messages"].as_array().unwrap().len(), 2);
        assert_eq!(v["messages"][1]["role"], "assistant");
        assert_eq!(v["messages"][1]["content"], "");
        assert_eq!(v["selectedCharacter"], Value::Null);
        assert_eq!(v["selectedStory"], Value::Null);
        assert!(v["chatId"].as_str().unwrap().len() >= 36, "UUID-ish");
        assert_ne!(v["chatId"], v["deviceId"], "dua UUID berbeda");
    }

    #[test]
    fn each_character_has_its_own_prompt_and_unknown_keys_fall_back() {
        let prompts: Vec<&str> = ["ai-assistant", "kobo-ai", "waguri-ai", "jokowi-ai", "prabowo-ai"]
            .iter()
            .map(|k| character_prompt(k))
            .collect();
        let mut uniq = prompts.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(
            uniq.len(),
            5,
            "lima karakter di `CHARACTERS`, lima prompt berbeda"
        );
        assert!(character_prompt("tidak-ada").contains("AI Assistant"));
    }

    #[test]
    fn generated_uuids_look_like_v4() {
        let u = uuid_v4();
        assert_eq!(u.len(), 36, "{u}");
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            vec![8, 4, 4, 4, 12]
        );
        assert!(parts[2].starts_with('4'), "bit versi: {u}");
        assert!(parts[3].starts_with('a'), "bit varian: {u}");
        assert!(u.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }
}