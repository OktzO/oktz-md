//! `plugins/search/animeapaini.js` — tebak judul anime dari gambar.
//!
//! Media diambil lewat `m.download()` / `m.quoted.download()`, yang di `Ctx`
//! adalah `ctx.download_media()` / `ctx.download_quoted_media()`; unggahan
//! `form.append("image", buffer, { filename, contentType })` jadi
//! `MultipartPart::file`.
//!
//! `ponytail:` `sock.sendMessage({ image: { url }, caption })` dipetakan ke
//! `Reply::image(MediaSource::Url, caption)`. `Reply::Image` tidak menyimpan
//! status kutipan seperti `{ quoted: m }` di JS.
//!
//! `ponytail:` `full_matches` kosong tetap lolos guard (`![]` falsy di JS),
//! lalu `full_matches[0].anilist` melempar TypeError dan masuk `catch` —
//! hasilnya template error, bukan "tidak ditemukan".

use oktz_plugin_api::{
    Ctx, HttpCall, IncomingKind, MediaSource, MultipartPart, QuotedKind, QuotedMessage, Reply,
    plugin,
};
use serde_json::Value;

use crate::common::{error_template, js_truthy};

/// `animeapaini.js:52` — endpoint cek anime.
const ENDPOINT: &str = "https://my.izuka-api.xyz/api/anime/anime-checker";

/// `animeapaini.js:46` — unduhan gambar gagal.
const DOWNLOAD_FAILED: &str = "Maaf, sistem gagal mengunduh gambar yang kamu berikan. Silakan coba kirim ulang gambarnya!";

/// `animeapaini.js:60` — judul tidak ketemu.
const NOT_FOUND: &str = "Maaf, judul anime tidak ditemukan. Coba dengan screenshot adegan yang lebih jelas atau karakter yang lebih spesifik.";

/// `animeapaini.js:25-30` — kartu bantuan.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔍 *ANIME APA INI?*\n\n\
         Fitur cerdas untuk mengetahui judul anime hanya dari sebuah screenshot atau potongan gambar!\n\n\
         *Cara Penggunaan:*\n\
         - Kirim gambar adegan anime dengan caption *{prefix}animeapaini*\n\
         - Atau balas (reply) gambar adegan anime dengan perintah *{prefix}animeapaini*\n\n\
         ⚠️ *Catatan:* Video tidak didukung, hanya gambar/screenshot saja ya!"
    )
}

/// `m.isImage || (m.quoted && m.quoted.type === "imageMessage")`.
pub(crate) fn is_image(ctx: &Ctx) -> bool {
    ctx.media
        .as_ref()
        .is_some_and(|m| m.kind == IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind == QuotedKind::Image)
}

/// Ambil byte gambar sesuai urutan `if (m.quoted && m.quoted.isMedia) … else if
/// (m.isMedia) …` di `animeapaini.js:38-42`.
fn unduh_gambar(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some_and(QuotedMessage::is_media) {
        return ctx.download_quoted_media().ok();
    }
    if ctx.is_media() {
        return ctx.download_media().ok();
    }
    None
}

/// Ambil nilai di jalur titik — padanan `a?.b?.c` yang tidak bisa ditulis
/// dengan rantai `.get()`.
fn dig<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = v;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// `a || b` untuk nilai JSON: yang pertama kalau truthy, kalau tidak yang kedua.
fn or<'a>(a: Option<&'a Value>, b: Option<&'a Value>) -> Option<&'a Value> {
    if js_truthy(a) { a } else { b }
}

/// `parseFloat` — hanya bagian depan yang dibaca (`"92.5%"` -> `92.5`).
/// `None` = `NaN`.
pub(crate) fn parse_float(raw: &str) -> Option<f64> {
    let t = raw.trim_start();
    let mut end = 0;
    for (i, c) in t.char_indices() {
        let ok = c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+'));
        if ok {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    t[..end].parse::<f64>().ok()
}

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
///
/// Sama dengan `search/apkmod.rs` dan `search/android1.rs`; kandidat
/// `common.rs`.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `resObj.similarity ? parseFloat(resObj.similarity) : match.similarity * 100`
/// lalu `isNaN(x) ? resObj.similarity : x.toFixed(2)`.
///
/// `ponytail:` `toFixed(2)` JS membulatkan setengah ke arah luar sedangkan
/// `{:.2}` Rust memakai pembulatan ke genap; angka tepat di tengah bisa
/// berbeda satu digit terakhir. Teks asal (`"92.5%"`) tidak pernah terpengaruh.
pub(crate) fn similarity_of(result: &Value, matched: &Value) -> String {
    let own = result.get("similarity");
    let raw = if js_truthy(own) {
        own.and_then(Value::as_str).and_then(parse_float)
    } else {
        matched.get("similarity").and_then(Value::as_f64).map(|v| v * 100.0)
    };
    match raw {
        Some(v) => format!("{v:.2}"),
        None => interp(own),
    }
}

/// `animeapaini.js:71-77` — teks hasil. Semua fallback diambil dari `match`.
pub(crate) fn format_text(result: &Value, matched: &Value) -> String {
    let mut txt = String::from("🔍 *ANIME DITEMUKAN!*\n\n");
    txt.push_str(&format!(
        "🎬 *Judul Romaji:* {}\n",
        interp(or(result.get("title_romaji"), dig(matched, "anilist.title.romaji")))
    ));
    txt.push_str(&format!(
        "🇯🇵 *Judul Asli:* {}\n",
        interp(or(result.get("title_native"), dig(matched, "anilist.title.native")))
    ));
    txt.push_str(&format!(
        "📺 *Episode:* {}\n",
        interp(or(result.get("episode"), matched.get("episode")))
    ));
    txt.push_str(&format!(
        "📊 *Kemiripan:* {}%\n",
        similarity_of(result, matched)
    ));
    txt.push_str(&format!(
        "🔞 *Dewasa (18+):* {}\n\n",
        if js_truthy(result.get("is_adult")) {
            "Ya"
        } else {
            "Tidak"
        }
    ));

    // `match.anilist.siteUrl || `https://anilist.co/anime/${match.anilist.id}``
    let site = match dig(matched, "anilist.siteUrl").filter(|v| js_truthy(Some(*v))) {
        Some(url) => interp(Some(url)),
        None => format!(
            "https://anilist.co/anime/{}",
            interp(dig(matched, "anilist.id"))
        ),
    };
    txt.push_str(&format!("🔗 *Detail Anilist:*\n{site}"));
    txt
}

/// `resObj.image_preview || match.image` — URL untuk `Reply::image`.
pub(crate) fn image_of(result: &Value, matched: &Value) -> Option<String> {
    or(result.get("image_preview"), matched.get("image"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `!data || !data.status || !data.result || !data.result.full_matches`
/// (`animeapaini.js:58`) — `false` = cabang "tidak ditemukan".
pub(crate) fn guard_ok(data: &Value) -> bool {
    js_truthy(data.get("status"))
        && js_truthy(data.get("result"))
        && js_truthy(data.get("result").and_then(|r| r.get("full_matches")))
}

/// `full_matches[0]` (`animeapaini.js:66`).
///
/// Array kosong (lolos guard di JS) dan bentuk non-array — yang di JS membuat
/// `matches[0].anilist` melempar TypeError dan masuk `catch` — keduanya
/// `None` di sini.
pub(crate) fn first_match(data: &Value) -> Option<&Value> {
    data.get("result")?
        .get("full_matches")?
        .as_array()?
        .first()
}

/// Body multipart — `form.append("image", buffer, { filename: "image.jpg",
/// contentType: "image/jpeg" })`.
pub(crate) fn image_part(buffer: Vec<u8>) -> MultipartPart {
    MultipartPart::file("image", "image.jpg", buffer).with_mimetype("image/jpeg")
}

#[plugin(
    name = "animeapaini",
    alias = [
        "whatanime",
        "animesearch",
        "sauceanime",
        "searchanime",
        "anime-checker",
        "animechecker"
    ],
    category = "search",
    description = "Identifikasi judul anime dari gambar/screenshot",
    usage = ".animeapaini (reply gambar)",
    example = ".animeapaini",
    cooldown = 15,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn animeapaini(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(buffer) = unduh_gambar(&ctx) else {
        return Reply::text(DOWNLOAD_FAILED).with_reaction("❌");
    };

    let Some(resp) = ctx
        .http
        .call(&HttpCall::post(ENDPOINT).multipart(vec![image_part(buffer)]))
        .ok()
    else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let data = resp.json().unwrap_or(Value::Null);
    if !guard_ok(&data) {
        return Reply::text(NOT_FOUND).with_reaction("❌");
    }
    let Some(matched) = first_match(&data) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let result = data.get("result").unwrap_or(&Value::Null);

    let caption = format_text(result, matched);
    match image_of(result, matched) {
        Some(url) => Reply::image(MediaSource::url(url), caption).with_reaction("✅"),
        None => Reply::text(caption).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpError, HttpGet, HttpResponse, IncomingMedia, MediaError, MediaFetch,
        MediaOrigin, NoMediaFetch,
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Api {
        body: Option<String>,
        seen: Mutex<Vec<HttpCall>>,
    }

    impl Api {
        fn new(body: &str) -> Arc<Api> {
            Arc::new(Api {
                body: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Api> {
            Arc::new(Api {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Api {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.clone());
            let body = self.body.clone().ok_or_else(|| HttpError {
                message: "offline".into(),
            })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
    }

    #[derive(Debug)]
    struct Bytes(Vec<u8>);

    impl MediaFetch for Bytes {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            Ok(self.0.clone())
        }
    }

    fn quoted(kind: QuotedKind) -> QuotedMessage {
        QuotedMessage {
            id: "x".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            kind,
            mimetype: Some("image/jpeg".into()),
            file_name: None,
            from_me: false,
        }
    }

    fn ctx_with(api: Arc<Api>, media: bool, quoted: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "animeapaini".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            http: api,
            media: media.then(|| IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            quoted,
            media_fetch: Arc::new(Bytes(vec![0xFF, 0xD8, 0xFF])),
            ..Default::default()
        }
    }

    fn run(api: Arc<Api>, media: bool, quoted: Option<QuotedMessage>) -> Reply {
        (oktz_plugin_api::find("animeapaini").unwrap().handler)(ctx_with(api, media, quoted))
    }

    fn body_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    // Fixture memakai judul berbahasa Rusia, bukan CJK: `title_native` di
    // AniList memang berkode non-Latin, tapi huruf CJK tidak boleh bocor ke
    // file .rs (aturan repo). Sifat multi-byte yang sedang diuji tetap sama.
    const COCOK: &str = r#"{
        "status": true,
        "result": {
            "title_romaji": "Kimi no Na wa.",
            "title_native": "Твоё имя",
            "episode": "1",
            "similarity": "92.5%",
            "is_adult": false,
            "image_preview": "https://cdn/preview.jpg",
            "full_matches": [
                {
                    "similarity": 0.87,
                    "episode": "1",
                    "image": "https://cdn/scene.jpg",
                    "anilist": {"id": 140960, "siteUrl": "https://anilist.co/anime/140960", "title": {"romaji": "Kimi no Na wa.", "native": "Твоё имя"}}
                }
            ]
        }
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("animeapaini").unwrap();
        assert_eq!(p.meta.name, "animeapaini");
        assert_eq!(
            p.meta.alias,
            [
                "whatanime",
                "animesearch",
                "sauceanime",
                "searchanime",
                "anime-checker",
                "animechecker"
            ]
        );
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Identifikasi judul anime dari gambar/screenshot"
        );
        assert_eq!(p.meta.usage, ".animeapaini (reply gambar)");
        assert_eq!(p.meta.example, ".animeapaini");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in [
            "whatanime",
            "animesearch",
            "sauceanime",
            "searchanime",
            "anime-checker",
            "animechecker",
        ] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "animeapaini",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_message_without_an_image_shows_the_help_card() {
        let api = Api::new(COCOK);
        for quoted in [None, Some(quoted(QuotedKind::Video))] {
            let r = run(api.clone(), false, quoted);
            assert_eq!(body_of(&r), help("."));
            assert!(api.seen.lock().unwrap().is_empty(), "tidak ada request");
        }
    }

    #[test]
    fn an_image_or_an_image_reply_is_accepted() {
        assert!(is_image(&ctx_with(Api::empty(), true, None)));
        assert!(is_image(&ctx_with(
            Api::empty(),
            false,
            Some(quoted(QuotedKind::Image))
        )));
        assert!(!is_image(&ctx_with(Api::empty(), false, None)));
        assert!(!is_image(&ctx_with(
            Api::empty(),
            false,
            Some(quoted(QuotedKind::Sticker))
        )));
    }

    #[test]
    fn a_failed_download_reports_the_retry_message() {
        let mut ctx = ctx_with(Api::new(COCOK), true, None);
        ctx.media_fetch = Arc::new(NoMediaFetch);
        let r = (oktz_plugin_api::find("animeapaini").unwrap().handler)(ctx);
        assert_eq!(body_of(&r), DOWNLOAD_FAILED);
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn the_image_is_posted_as_multipart_named_image_jpg() {
        let api = Api::new(COCOK);
        let _ = run(api.clone(), true, None);
        let seen = api.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].url, ENDPOINT);
        let Some(HttpBody::Multipart(parts)) = seen[0].body.as_ref() else {
            panic!("body harus multipart");
        };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "image");
        assert_eq!(parts[0].file_name.as_deref(), Some("image.jpg"));
        assert_eq!(parts[0].mimetype.as_deref(), Some("image/jpeg"));
        assert_eq!(parts[0].body.bytes(), &[0xFF, 0xD8, 0xFF]);
    }

    #[test]
    fn a_quoted_image_is_used_when_the_trigger_has_no_media() {
        let api = Api::new(COCOK);
        let r = run(api.clone(), false, Some(quoted(QuotedKind::Image)));
        let body = body_of(&r);
        assert!(body.contains("🎬 *Judul Romaji:* Kimi no Na wa.\n"), "{body}");
        assert!(body.contains("🇯🇵 *Judul Asli:* Твоё имя\n"), "{body}");
        assert!(body.contains("📺 *Episode:* 1\n"), "{body}");
        assert!(body.contains("📊 *Kemiripan:* 92.50%\n"), "{body}");
        assert!(
            body.ends_with("🔗 *Detail Anilist:*\nhttps://anilist.co/anime/140960"),
            "{body}"
        );
        assert_eq!(api.seen.lock().unwrap().len(), 1);
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn the_caption_falls_back_to_the_match_object() {
        let data = json!({
            "status": true,
            "result": {
                "full_matches": [{
                    "similarity": 0.5,
                    "episode": "3",
                    "anilist": {"id": 42, "title": {"romaji": "Fallback Title", "native": "Nama Fallback"}}
                }]
            }
        });
        let matched = first_match(&data).unwrap();
        let text = format_text(data.get("result").unwrap(), matched);
        assert!(text.starts_with("🔍 *ANIME DITEMUKAN!*\n\n"), "{text}");
        assert!(text.contains("🎬 *Judul Romaji:* Fallback Title\n"), "{text}");
        assert!(text.contains("🇯🇵 *Judul Asli:* Nama Fallback\n"), "{text}");
        assert!(text.contains("📺 *Episode:* 3\n"), "{text}");
        assert!(
            text.contains("📊 *Kemiripan:* 50.00%\n"),
            "`match.similarity * 100`: {text}"
        );
        assert!(text.contains("🔞 *Dewasa (18+):* Tidak\n\n"), "{text}");
        assert!(
            text.ends_with("🔗 *Detail Anilist:*\nhttps://anilist.co/anime/42"),
            "siteUrl kosong -> URL dari id: {text}"
        );
    }

    #[test]
    fn an_adult_flag_flips_only_the_dewasa_line() {
        let data = json!({
            "status": true,
            "result": {"is_adult": 1, "full_matches": [{"anilist": {"id": 1, "siteUrl": "u"}}]}
        });
        let matched = first_match(&data).unwrap();
        assert!(format_text(data.get("result").unwrap(), matched)
            .contains("🔞 *Dewasa (18+):* Ya\n"));
    }

    #[test]
    fn similarity_follows_parse_float_then_to_fixed() {
        assert_eq!(parse_float("92.5%"), Some(92.5));
        assert_eq!(parse_float("  -3.25abc"), Some(-3.25));
        assert_eq!(parse_float("abc"), None);
        assert_eq!(parse_float(""), None);

        assert_eq!(
            similarity_of(&json!({"similarity": "92.5%"}), &json!({})),
            "92.50"
        );
        assert_eq!(
            similarity_of(&json!({"similarity": "bukan angka"}), &json!({})),
            "bukan angka",
            "`parseFloat` NaN -> nilai aslinya ikut tampil"
        );
        assert_eq!(
            similarity_of(&json!({"similarity": 0}), &json!({})),
            "0",
            "0 falsy -> bukan sumber, `match.similarity` juga tidak ada"
        );
        assert_eq!(
            similarity_of(&json!({}), &json!({"similarity": 0.875})),
            "87.50"
        );
    }

    #[test]
    fn a_falsy_status_or_missing_result_reports_not_found() {
        for body in [
            r#"{"status": false, "result": {"full_matches": [{}]}}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "result": {}}"#,
            r#"{"status": true, "result": {"full_matches": null}}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Api::new(body), true, None);
            assert_eq!(body_of(&r), NOT_FOUND, "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn an_empty_match_list_hits_the_catch_branch_like_the_type_error() {
        for body in [
            r#"{"status": true, "result": {"full_matches": []}}"#,
            r#"{"status": true, "result": {"full_matches": "ab"}}"#,
        ] {
            let r = run(Api::new(body), true, None);
            assert_eq!(body_of(&r), error_template(".", "animeapaini"), "{body}");
            assert_eq!(reaction_of(&r), Some("☢"), "{body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Api::empty(), true, None);
        assert!(body_of(&r).contains("command `.animeapaini`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_preview_image_is_sent_as_media() {
        let r = run(Api::new(COCOK), true, None);
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url("https://cdn/preview.jpg"))
        );
    }

    #[test]
    fn without_an_image_source_the_answer_is_plain_text() {
        let data = json!({
            "status": true,
            "result": {"full_matches": [{"anilist": {"id": 1, "siteUrl": "https://anilist.co/anime/1"}, "image": "https://cdn/scene.jpg"}]}
        });
        let result = data.get("result").unwrap();
        let matched = first_match(&data).unwrap();
        assert_eq!(
            image_of(result, matched),
            Some("https://cdn/scene.jpg".to_string()),
            "preview kosong -> pakai `match.image`"
        );
        let r = run(Api::new(&data.to_string()), true, None);
        assert!(!r.has_media());
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            image_of(&json!({}), &json!({})),
            None,
            "tanpa gambar, teks biasa yang terkirim"
        );
    }
}