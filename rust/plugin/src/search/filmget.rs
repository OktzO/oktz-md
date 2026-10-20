//! `plugins/search/filmget.js` — ambil detail film dari `api.neoxr.eu`.
//!
//! `ponytail:` JS mengirim `sock.sendMessage` dengan `footer` +
//! `contextInfo: saluranCtx()` + `interactiveButtons` cta_url. Saluran ctx
//! membaca `config.saluran`/`config.bot.name` dan `Reply` tidak punya context
//! info, jadi `Reply::Interactive` hanya membawa teksnya.
//!
//! `ponytail:` `axios.get(film.thumbnail, { responseType: "arraybuffer" })`
//! dibungkus `try {} catch {}` — hasilnya TIDAK dipakai apa pun, cuma memaksa
//! thumbnail terunduh. Request itu tetap dilakukan di sini supaya urutan dan
//! jumlahnya sama.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `filmget.js:37` — endpoint detail film.
pub(crate) fn api_url(url: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/film-get?url={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(url)
    )
}

/// `filmget.js:25-31` — kartu saat URL bukan `neoxr.eu`.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎬 *ꜰɪʟᴍ ᴅᴇᴛᴀɪʟ*\n\n\
         > Ambil detail film dari URL\n\n\
         *Format:*\n\
         > `{prefix}filmget <url>`\n\n\
         > Gunakan `{prefix}film <judul>` untuk cari film dulu"
    )
}

/// `filmget.js:42` — film tidak ada.
pub(crate) const NOT_FOUND: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Film tidak ditemukan";

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

/// `data.stream || []` (`filmget.js:46`).
pub(crate) fn streams_of(data: &Value) -> &[Value] {
    data.get("stream").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// `data.download || []` (`filmget.js:47`).
pub(crate) fn downloads_of(data: &Value) -> &[Value] {
    data.get("download")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `filmget.js:58-85` — teks info film.
pub(crate) fn format_text(film: &Value, streams: &[Value], downloads: &[Value]) -> String {
    let mut text = format!("🎬 *{}*\n\n", dash(film.get("title"), "Film"));
    text.push_str("╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n");
    for (label, key) in [
        ("Rating", "rating"),
        ("Quality", "quality"),
        ("Duration", "duration"),
        ("Release", "release"),
        ("Genre", "tags"),
        ("Director", "director"),
        ("Actors", "actors"),
    ] {
        text.push_str(&format!("┃ {label}: {}\n", dash(film.get(key), "-")));
    }
    text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    text.push_str("📝 *Synopsis:*\n");
    text.push_str(&format!("> {}\n\n", dash(film.get("synopsis"), "-")));

    if !streams.is_empty() {
        text.push_str("▶️ *Streaming:*\n");
        for (i, s) in streams.iter().enumerate() {
            text.push_str(&format!("> {}. {}\n", i + 1, interp(s.get("server"))));
        }
        text.push('\n');
    }

    if !downloads.is_empty() {
        text.push_str("📥 *Download:*\n");
        for (i, d) in downloads.iter().enumerate() {
            text.push_str(&format!("> {}. {}\n", i + 1, interp(d.get("provider"))));
        }
    }

    text
}

/// `film.title || "Film"` dan `film.x || "-"` — nilai falsy (termasuk `""`)
/// memakai cadangan, bukan dirender apa adanya.
fn dash(v: Option<&Value>, fallback: &str) -> String {
    match v {
        Some(value) if js_truthy(Some(value)) => interp(Some(value)),
        _ => fallback.to_string(),
    }
}

#[plugin(
    name = "filmget",
    alias = ["getfilm", "filmdetail", "filminfo"],
    category = "search",
    description = "Ambil detail film",
    usage = ".filmget <url>",
    example = ".filmget https://tv.neoxr.eu/film/civil-war-2024",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn filmget(ctx: Ctx) -> Reply {
    let url = ctx.args.first().map(|a| a.trim()).unwrap_or("");

    if url.is_empty() || !url.contains("neoxr.eu") {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(resp) = ctx.http.get(&api_url(url)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    if !js_truthy(data.get("status")) || !js_truthy(data.get("data")) {
        return Reply::text(NOT_FOUND).with_reaction("❌");
    }
    let film = data.get("data").unwrap_or(&Value::Null);
    let streams = streams_of(&data);
    let downloads = downloads_of(&data);

    // `try { await axios.get(film.thumbnail, …) } catch {}` — hasilnya
    // dibuang, jadi kegagalan di sini tidak menggerakkan apa pun.
    if let Some(thumb) = film.get("thumbnail").filter(|v| js_truthy(Some(*v))).and_then(Value::as_str) {
        let _ = ctx.http.get(thumb);
    }

    Reply::interactive(format_text(film, streams, downloads))
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub dengan pencocokanAwalan: endpoint film-get, lalu thumbnail.
    #[derive(Debug)]
    struct Stub {
        pages: Vec<(String, Option<String>)>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(pages: &[(&str, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                pages: pages
                    .iter()
                    .map(|(u, b)| ((*u).to_string(), Some((*b).to_string())))
                    .collect(),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                pages: vec![],
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = self
                .pages
                .iter()
                .find(|(prefix, _)| url.starts_with(prefix.as_str()))
                .and_then(|(_, b)| b.clone())
                .ok_or_else(|| oktz_plugin_api::HttpError {
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

    const FILM: &str = "https://api.neoxr.eu/api/film-get";
    const THUMB: &str = "https://tv.neoxr.eu/img/civil.jpg";

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "filmget".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("filmget").unwrap().handler)(ctx)
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

    const DETAIL: &str = r#"{
        "status": true,
        "data": {
            "title": "Civil War",
            "rating": "8.1",
            "quality": "HD",
            "duration": "1j 49m",
            "release": "2024",
            "tags": "Action, Drama",
            "director": "Alex Garland",
            "actors": "Cailee Spaeny, Kirsten Dunst",
            "synopsis": "Juru reporter invoke rival.",
            "thumbnail": "https://tv.neoxr.eu/img/civil.jpg"
        },
        "stream": [{"server": "Srv1", "url": "https://s1/a.m3u8"}, {"server": "Srv2", "url": "https://s2/a.m3u8"}],
        "download": [{"provider": "Pixeldrain", "url": "https://p/a"}]
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("filmget").unwrap();
        assert_eq!(p.meta.name, "filmget");
        assert_eq!(p.meta.alias, ["getfilm", "filmdetail", "filminfo"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Ambil detail film");
        assert_eq!(p.meta.usage, ".filmget <url>");
        assert_eq!(
            p.meta.example,
            ".filmget https://tv.neoxr.eu/film/civil-war-2024"
        );
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        // `pluginConfig` JS tidak menyebut flag permission sama sekali.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["getfilm", "filmdetail", "filminfo"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "filmget",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_missing_or_foreign_url_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   ", "https://lk21.com/x", "neoxr eu"] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_info_card_lists_streaming_and_download_rows() {
        let http = Stub::new(&[(FILM, DETAIL), (THUMB, "JPG")]);
        let r = run(http.clone(), "https://tv.neoxr.eu/film/civil-war-2024");
        let body = body_of(&r);
        assert!(body.starts_with("🎬 *Civil War*\n\n╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n"), "{body}");
        assert!(body.contains("┃ Rating: 8.1\n┃ Quality: HD\n┃ Duration: 1j 49m\n"), "{body}");
        assert!(body.contains("┃ Genre: Action, Drama\n┃ Director: Alex Garland\n"), "{body}");
        assert!(body.contains("┃ Actors: Cailee Spaeny, Kirsten Dunst\n╰┈┈┈┈┈┈┈┈⬡\n\n"), "{body}");
        assert!(body.contains("📝 *Synopsis:*\n> Juru reporter invoke rival.\n\n"), "{body}");
        assert!(body.contains("▶️ *Streaming:*\n> 1. Srv1\n> 2. Srv2\n\n"), "{body}");
        assert!(body.contains("📥 *Download:*\n> 1. Pixeldrain\n"), "{body}");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(r.without_mutations().tanpa_reaksi(), Reply::Interactive { .. }));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2, "film-get lalu thumbnail: {seen:?}");
        assert_eq!(
            seen[0],
            "https://api.neoxr.eu/api/film-get?url=https%3A%2F%2Ftv.neoxr.eu%2Ffilm%2Fcivil-war-2024&apikey="
        );
        assert_eq!(seen[1], THUMB);
    }

    #[test]
    fn missing_fields_fall_back_to_a_dash() {
        let film = serde_json::json!({});
        let text = format_text(&film, &[], &[]);
        assert!(text.starts_with("🎬 *Film*\n\n"), "{text}");
        assert!(text.contains("┃ Rating: -\n┃ Quality: -\n"), "{text}");
        assert!(text.contains("> -\n\n"), "{text}");
        assert!(!text.contains("Streaming"), "{text}");
        assert!(!text.contains("Download"), "{text}");
    }

    #[test]
    fn a_falsy_status_or_missing_data_reports_not_found() {
        for body in [
            r#"{"status": false, "data": {"title": "A"}}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "data": null}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Stub::new(&[(FILM, body)]), "https://tv.neoxr.eu/film/x");
            assert_eq!(body_of(&r), NOT_FOUND, "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn a_failing_thumbnail_request_does_not_break_the_reply() {
        // `try { … } catch {}` di `filmget.js:50-56`.
        let http = Stub::new(&[(FILM, DETAIL)]);
        let r = run(http.clone(), "https://tv.neoxr.eu/film/civil-war-2024");
        assert!(body_of(&r).starts_with("🎬 *Civil War*"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(http.seen.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_transport_failure_on_the_endpoint_reports_the_error_template() {
        let r = run(Stub::empty(), "https://tv.neoxr.eu/film/civil-war");
        assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "filmget"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_stream_and_download_lists_default_to_empty_arrays() {
        let data = serde_json::json!({"stream": "bukan array", "download": {"a": 1}});
        assert!(streams_of(&data).is_empty());
        assert!(downloads_of(&data).is_empty());
    }

    #[test]
    fn a_very_long_url_is_sent_whole() {
        let long = format!("https://tv.neoxr.eu/film/{}", "a".repeat(4_000));
        let http = Stub::new(&[(FILM, r#"{"status": true, "data": {"title": "X"}}"#)]);
        let _ = run(http.clone(), &long);
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }
}