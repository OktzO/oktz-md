//! `plugins/download/threaddl.js` — unduh foto dan video dari Threads.
//!
//! `ponytail:` `he.decode` dari paket npm `he` punya tabel ~2000 entitas.
//! Yang dipindah di sini subset yang lazim muncul pada teks Threads plus
//! semua entitas numerik (`&#123;` / `&#x7B;`); entitas di luar subset itu
//! dibiarkan apa adanya — sama seperti `he.decode` pada entity yang tidak
//! dikenal.
//!
//! `ponytail:` axios di sini memakai `validateStatus: () => true`, jadi status
//! 4xx/5xx tidak melempar. `HttpGet` justru mengembalikan `Err` di atas 399,
//! sehingga 4xx/5xx sekarang masuk ke cabang `catch` (`☢`), bukan ke cabang
//! "gagal mengambil data" (`❌`). Perbaiki di `HttpGet` kalau bedanya penting.
//!
//! `ponytail:` `Reply` tidak punya variant album maupun video. Postinggan
//! banyak media hanya mengirim teks caption-nya; satu video juga hanya teks
//! caption; satu foto masih terkirim sebagai gambar.

use oktz_plugin_api::{Ctx, HttpGet, MediaSource, Reply, plugin};
use serde_json::Value;

use super::facebookdl::axios_query;
use crate::common::js_truthy_value;

/// `threaddl.js:4` — worker pihak ketiga.
pub(crate) const BASE_URL: &str =
    "https://workers-playground-cool-wood-c008.accoutydusra.workers.dev";

/// `threaddl.js:60` — link kosong atau tidak memuat kata "threads".
pub(crate) const BAD_LINK: &str = "❌ *Waduh, Link Threads-nya mana nih?*\n\nKamu harus memasukkan tautan (link) dari postingan Threads yang ingin diunduh. Pastikan linknya benar ya! \n\nContoh: `.tdl https://www.threads.net/@zuck/post/xxx`";

/// `threaddl.js:93` — respons gagal, bukan melempar.
pub(crate) fn failed(reason: &str) -> String {
    format!(
        "⚠️ *Aduh, gagal mengambil data dari Threads!*\n\n\
         Mungkin postingan ini bersifat privat, sudah dihapus, atau link yang kamu berikan kurang tepat.\n\n\
         Alasan sistem: {reason}"
    )
}

/// `threaddl.js:129` — cabang `catch`.
pub(crate) const SYSTEM_FAILED: &str = "😔 *Sepertinya ada gangguan di sistemku.* \n\nTerjadi kesalahan fatal saat mencoba memproses link Threads tersebut. Silakan coba lagi nanti ya!";

/// Entitas bernama yang dipakai teks Threads. Sisanya dibiarkan apa adanya.
const ENTITIES: [(&str, char); 28] = [
    ("&amp;", '&'),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&apos;", '\''),
    ("&nbsp;", ' '),
    ("&hellip;", '…'),
    ("&mdash;", '—'),
    ("&ndash;", '–'),
    ("&lsquo;", '‘'),
    ("&rsquo;", '’'),
    ("&ldquo;", '“'),
    ("&rdquo;", '”'),
    ("&middot;", '·'),
    ("&bull;", '•'),
    ("&copy;", '©'),
    ("&reg;", '®'),
    ("&trade;", '™'),
    ("&deg;", '°'),
    ("&laquo;", '«'),
    ("&raquo;", '»'),
    ("&eacute;", 'é'),
    ("&egrave;", 'è'),
    ("&agrave;", 'à'),
    ("&ccedil;", 'ç'),
    ("&uuml;", 'ü'),
    ("&ouml;", 'ö'),
    ("&auml;", 'ä'),
];

/// `he.decode` — subset, lihat catatan modul.
pub(crate) fn he_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            let next = raw[i..]
                .find('&')
                .map_or(bytes.len(), |p| i + p);
            out.push_str(&raw[i..next]);
            i = next;
            continue;
        }
        let semi = match raw[i..].find(';') {
            Some(p) => i + p,
            None => {
                out.push('&');
                i += 1;
                continue;
            }
        };
        let entity = &raw[i..semi];
        // `&#39;` dan `&#x27;` — `entity` dipotong sebelum `;`.
        let numeric = entity.strip_prefix('#').and_then(|rest| {
            let (digits, radix) = match rest.strip_prefix(['x', 'X']) {
                Some(hex) => (hex, 16),
                None => (rest, 10),
            };
            u32::from_str_radix(digits, radix).ok().and_then(char::from_u32)
        });
        if let Some(c) = numeric {
            out.push(c);
        } else if let Some((_, c)) = ENTITIES.iter().find(|(name, _)| *name == entity) {
            out.push(*c);
        } else {
            out.push_str(entity);
        }
        i = semi + 1;
    }
    out
}

/// `threaddl.js:6-8` — `cleanText`.
pub(crate) fn clean_text(raw: Option<&Value>) -> String {
    // `String(text || "")`: null/undefined/number 0/string kosong semua jadi "".
    let text = match raw {
        Some(Value::String(s)) if !s.is_empty() => s.as_str(),
        _ => "",
    };
    let decoded = he_decode(text).replace('\r', "");
    // `/[ \t]+\n/` lalu `/\n{3,}/`.
    let mut collapsed = String::with_capacity(decoded.len());
    let mut chars = decoded.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ' ' || c == '\t' {
            let mut blanks = String::new();
            blanks.push(c);
            while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                blanks.push(chars.next().expect("peek Some berarti ada karakter"));
            }
            if chars.peek() == Some(&'\n') {
                chars.next();
                collapsed.push('\n');
            } else {
                collapsed.push_str(&blanks);
            }
            continue;
        }
        collapsed.push(c);
    }
    let mut squeezed = String::with_capacity(collapsed.len());
    let mut run = 0usize;
    for c in collapsed.chars() {
        if c == '\n' {
            run += 1;
            continue;
        }
        if run > 0 {
            for _ in 0..run.min(2) {
                squeezed.push('\n');
            }
            run = 0;
        }
        squeezed.push(c);
    }
    for _ in 0..run.min(2) {
        squeezed.push('\n');
    }
    squeezed.trim().to_string()
}

/// Satu media hasil `normalizeResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Item {
    Video { quality: Option<String>, url: String },
    Image { url: String },
}

/// `threaddl.js:10-19` — `uniqueByUrl`.
fn unique_by_url(list: Option<&Vec<Value>>) -> Vec<&Value> {
    let Some(list) = list else {
        return Vec::new();
    };
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for item in list {
        let Some(url) = item.get("url").filter(|u| js_truthy_value(u)).and_then(Value::as_str)
        else {
            continue;
        };
        if seen.contains(&url) {
            continue;
        }
        seen.push(url);
        out.push(item);
    }
    out
}

/// `threaddl.js:21-42` — `normalizeResult`: video dulu, baru gambar.
pub(crate) fn normalize_result(data: &Value) -> Vec<Item> {
    let mut out = Vec::new();
    for item in unique_by_url(data.get("video").and_then(|v| v.get("qualities"))) {
        out.push(Item::Video {
            quality: item
                .get("quality")
                .filter(|q| js_truthy_value(q))
                .and_then(Value::as_str)
                .map(str::to_string),
            url: item
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        });
    }
    for item in unique_by_url(data.get("images").and_then(|v| v.get("urls"))) {
        out.push(Item::Image {
            url: item
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        });
    }
    out
}

/// `threaddl.js:96-104`.
pub(crate) fn caption(info: &Value, count: usize) -> String {
    let author = info
        .get("author")
        .filter(|a| js_truthy_value(a))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    let text = {
        let a = clean_text(info.get("title"));
        if a.is_empty() {
            let b = clean_text(info.get("description"));
            if b.is_empty() {
                "Tidak ada deskripsi.".to_string()
            } else {
                b
            }
        } else {
            a
        }
    };
    format!(
        "✨ *THREADS DOWNLOADER* ✨\n\n\
         Halo! Ini hasil unduhan Threads yang kamu minta:\n\n\
         👤 *Pembuat*: {author}\n\
         📝 *Teks Postingan*: {text}\n\
         📊 *Jumlah File Media*: {count} file\n\n\
         *Semoga bermanfaat ya!* Jangan lupa mampir lagi kalau mau download yang lain. 🚀"
    )
}

#[plugin(
    name = "threaddl",
    alias = ["tdl", "threads", "threadsdl"],
    category = "download",
    description = "Download foto dan video dari postingan Threads tanpa repot!",
    usage = ".tdl <url>",
    example = ".tdl https://www.threads.net/@xxx/post/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn threaddl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() || !url.to_lowercase().contains("threads") {
        return Reply::text(BAD_LINK);
    }

    let target = format!("{BASE_URL}?url={}&action=info", axios_query(url));
    let Ok(response) = ctx.http.get(&target) else {
        return Reply::text(SYSTEM_FAILED).with_reaction("☢");
    };
    let Some(data) = response.json() else {
        return Reply::text(SYSTEM_FAILED).with_reaction("☢");
    };

    let info = data.get("data").unwrap_or(&Value::Null);
    let result = normalize_result(info);

    let success_ok = data.get("success").and_then(Value::as_bool) == Some(true);
    if !success_ok || result.is_empty() {
        let reason = data
            .get("message")
            .filter(|m| js_truthy_value(m))
            .and_then(Value::as_str)
            .or_else(|| {
                data.get("error")
                    .filter(|e| js_truthy_value(e))
                    .and_then(Value::as_str)
            })
            .unwrap_or("Tidak diketahui");
        return Reply::text(failed(reason)).with_reaction("❌");
    }

    let text = caption(info, result.len());
    if result.len() > 1 {
        // `threaddl.js:115-117` — teks dulu lalu album. `Reply` tidak punya
        // album, jadi yang terkirim hanya teksnya.
        return Reply::text(text).with_reaction("✅");
    }
    match &result[0] {
        Item::Image { url } => {
            Reply::image(MediaSource::url(url.clone()), text).with_reaction("✅")
        }
        // `threaddl.js:110-111` — video tidak punya variant di `Reply`.
        Item::Video { .. } => Reply::text(text).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .body
                .clone()
                .map_err(|()| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "tdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("threaddl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("threaddl").unwrap();
        assert_eq!(p.meta.name, "threaddl");
        assert_eq!(p.meta.alias, ["tdl", "threads", "threadsdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(
            p.meta.description,
            "Download foto dan video dari postingan Threads tanpa repot!"
        );
        assert_eq!(p.meta.usage, ".tdl <url>");
        assert_eq!(p.meta.example, ".tdl https://www.threads.net/@xxx/post/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tdl", "threads", "threadsdl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "threaddl");
        }
    }

    #[test]
    fn a_link_without_the_word_threads_is_refused_without_any_request() {
        let http = Stub::body(json!({ "success": true }));
        for text in ["", "   ", "https://twitter.com/x/status/1"] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), BAD_LINK, "input {text:?}");
            assert!(reaction_of(&r).is_none(), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn he_decode_covers_named_decimal_and_hex_entities() {
        assert_eq!(he_decode("a &amp; b"), "a & b");
        assert_eq!(he_decode("&lt;p&gt;"), "<p>");
        assert_eq!(he_decode("&#39;&#x27;"), "''");
        assert_eq!(he_decode("a&#9731;b"), "a☃b");
        assert_eq!(he_decode("&hellip;"), "…");
        assert_eq!(he_decode("&tidakdikenal;"), "&tidakdikenal;");
        assert_eq!(he_decode("100% & Counting"), "100% & Counting");
        assert_eq!(he_decode("&"), "&");
        assert_eq!(he_decode("&#xZZ;"), "&#xZZ;");
    }

    #[test]
    fn clean_text_strips_carriage_returns_and_collapses_blank_runs() {
        assert_eq!(
            clean_text(Some(&json!("halo\r\ndunia"))),
            "halo\ndunia",
            "\\r dihapus total"
        );
        assert_eq!(
            clean_text(Some(&json!("a \t \nb"))),
            "a\nb",
            "spasi dan tab tepat sebelum newline jadi satu newline"
        );
        assert_eq!(
            clean_text(Some(&json!("a\n   b"))),
            "a\n   b",
            "spasi setelah newline tidak disentuh"
        );
        assert_eq!(
            clean_text(Some(&json!("a\n\n\n\n\nb"))),
            "a\n\nb",
            "tiga newline atau lebih jadi dua"
        );
        assert_eq!(clean_text(Some(&json!("   \n  "))), "");
        assert_eq!(clean_text(Some(&Value::Null)), "");
        assert_eq!(clean_text(None), "");
        assert_eq!(clean_text(Some(&json!("a &amp; b"))), "a & b");
    }

    #[test]
    fn duplicate_and_url_less_items_are_dropped() {
        let info = json!({
            "video": { "qualities": [
                { "quality": "hd", "url": "https://cdn/v.mp4" },
                { "quality": "sd", "url": "https://cdn/v.mp4" },
                { "quality": "hd" },
                { "quality": "sd", "url": "" }
            ]},
            "images": { "urls": [
                { "url": "https://cdn/a.jpg" },
                { "url": "https://cdn/a.jpg" },
                { "url": "https://cdn/b.jpg" }
            ]}
        });
        assert_eq!(
            normalize_result(&info),
            vec![
                Item::Video {
                    quality: Some("hd".into()),
                    url: "https://cdn/v.mp4".into()
                },
                Item::Image {
                    url: "https://cdn/a.jpg".into()
                },
                Item::Image {
                    url: "https://cdn/b.jpg".into()
                },
            ]
        );
    }

    #[test]
    fn video_comes_first_and_images_second() {
        let info = json!({
            "images": { "urls": [{ "url": "https://cdn/a.jpg" }] },
            "video": { "qualities": [{ "url": "https://cdn/v.mp4" }] }
        });
        let out = normalize_result(&info);
        assert_eq!(
            out,
            vec![
                Item::Video {
                    quality: None,
                    url: "https://cdn/v.mp4".into()
                },
                Item::Image {
                    url: "https://cdn/a.jpg".into()
                },
            ]
        );
    }

    #[test]
    fn missing_containers_normalize_to_an_empty_list() {
        assert!(normalize_result(&json!({})).is_empty());
        assert!(normalize_result(&Value::Null).is_empty());
        assert!(normalize_result(&json!({ "video": {}, "images": {} })).is_empty());
    }

    #[test]
    fn the_caption_uses_the_js_fallback_chain() {
        let info = json!({ "author": "zuck", "title": "Halo", "description": "Cadangan" });
        let text = caption(&info, 2);
        assert!(text.contains("👤 *Pembuat*: zuck"), "{text}");
        assert!(text.contains("📝 *Teks Postingan*: Halo"), "{text}");
        assert!(text.contains("📊 *Jumlah File Media*: 2 file"), "{text}");

        let tanpa_judul = json!({ "author": "", "description": "  Hanya deskripsi  " });
        let text = caption(&tanpa_judul, 0);
        assert!(text.contains("👤 *Pembuat*: Unknown"), "{text}");
        assert!(
            text.contains("📝 *Teks Postingan*: Hanya deskripsi"),
            "{text}"
        );

        let kosong = caption(&json!({}), 1);
        assert!(kosong.contains("📝 *Teks Postingan*: Tidak ada deskripsi."), "{kosong}");
    }

    #[test]
    fn the_query_string_keeps_the_param_order_of_the_js() {
        let http = Stub::body(json!({ "success": true, "data": { "images": { "urls": [] } } }));
        let _ = run(http.clone(), "https://www.threads.net/@zuck/post/1");
        assert_eq!(
            http.seen.lock().unwrap().clone(),
            vec![
                "https://workers-playground-cool-wood-c008.accoutydusra.workers.dev\
                 ?url=https:%2F%2Fwww.threads.net%2F%40zuck%2Fpost%2F1&action=info"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_single_photo_is_sent_as_an_image_with_the_caption() {
        let r = run(
            Stub::body(json!({
                "success": true,
                "data": { "author": "zuck", "images": { "urls": [{ "url": "https://cdn/a.jpg" }] } }
            })),
            "https://www.threads.net/@zuck/post/1",
        );
        assert!(text_of(&r).starts_with("✨ *THREADS DOWNLOADER* ✨"), "{}", text_of(&r));
        assert_eq!(r.media_source().map(|m| m.label()), Some("url"));
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn a_single_video_degrades_to_the_caption_text() {
        let r = run(
            Stub::body(json!({
                "success": true,
                "data": { "video": { "qualities": [{ "url": "https://cdn/v.mp4" }] } }
            })),
            "https://www.threads.net/@zuck/post/1",
        );
        assert!(text_of(&r).starts_with("✨ *THREADS DOWNLOADER* ✨"), "{}", text_of(&r));
        assert!(r.media_source().is_none(), "video tidak punya media source");
    }

    #[test]
    fn several_media_send_only_the_caption_text() {
        let r = run(
            Stub::body(json!({
                "success": true,
                "data": { "images": { "urls": [
                    { "url": "https://cdn/a.jpg" },
                    { "url": "https://cdn/b.jpg" }
                ] } }
            })),
            "https://www.threads.net/@zuck/post/1",
        );
        assert!(text_of(&r).contains("2 file"), "{}", text_of(&r));
        assert!(r.media_source().is_none(), "album tidak punya media source");
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn a_failed_response_reports_the_system_reason() {
        for (body, alasan) in [
            (json!({ "success": false, "message": "Postingan privat" }), "Postingan privat"),
            (json!({ "success": false, "error": "Dihapus" }), "Dihapus"),
            (json!({ "success": false }), "Tidak diketahui"),
            (json!({ "success": true, "data": {} }), "Tidak diketahui"),
            (json!({ "success": "true", "data": { "images": { "urls": [{ "url": "u" }] } } }), "Tidak diketahui"),
        ] {
            let r = run(Stub::body(body.clone()), "https://www.threads.net/@zuck/post/1");
            assert_eq!(text_of(&r), failed(alasan), "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn a_transport_failure_or_bad_json_reports_the_system_error() {
        for http in [Stub::offline(), Stub::body(json!("bukan objek"))] {
            let r = run(http, "https://www.threads.net/@zuck/post/1");
            assert_eq!(text_of(&r), SYSTEM_FAILED);
            assert_eq!(reaction_of(&r), Some("☢"));
        }
    }
}