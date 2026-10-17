//! `plugins/search/film.js` — cari film lewat `api.neoxr.eu`.
//!
//! `ponytail:` JS mengirim daftar lewat `sock.sendButton` dengan picker
//! `single_select` dan thumbnail `getAssetBuffer("foto")`. `Reply::Interactive`
//! hanya membawa teksnya, jadi tombol (judul, deskripsi, `id` untuk `filmget`)
//! dan thumbnail tidak ikut — persis teks `film.js:49-57`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{error_template, js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `film.js:37` — endpoint pencarian film.
pub(crate) fn api_url(query: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/film?q={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(query)
    )
}

/// `film.js:24-31` — kartu saat query kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎬 *ꜰɪʟᴍ sᴇᴀʀᴄʜ*\n\n\
         > Cari dan nonton film online\n\n\
         *Format:*\n\
         > \\`{prefix}film <judul>\\`\n\n\
         *Contoh:*\n\
         > \\`{prefix}film civil war\\`"
    )
}

/// `film.js:42-44` — tidak ada hasil.
pub(crate) fn not_found(query: &str) -> String {
    format!("❌ *ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> Film \"{query}\" tidak ditemukan")
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

/// `film.js:49-57` — daftar film. Dipanggil dengan `films` yang sudah di-`slice(0, 10)`.
pub(crate) fn format_list(films: &[Value], query: &str) -> String {
    let mut text = String::from("🎬 *ʜᴀsɪʟ ᴘᴇɴᴄᴀʀɪᴀɴ*\n\n");
    text.push_str(&format!(
        "> Ditemukan *{}* film untuk \"{query}\"\n\n",
        films.len()
    ));
    for (i, f) in films.iter().enumerate() {
        text.push_str(&format!("*{}. {}*\n", i + 1, interp(f.get("title"))));
        text.push_str(&format!(
            "> ⭐ {} | 📺 {} | 📅 {}\n\n",
            interp(f.get("rating")),
            interp(f.get("quality")),
            interp(f.get("release"))
        ));
    }
    text.push_str("> _Pilih film dari list di bawah_");
    text
}

/// `!data?.data?.length` (`film.js:40`) — array/string tak kosong lolos.
pub(crate) fn has_items(data: Option<&Value>) -> bool {
    match data {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::String(s)) => !s.is_empty(),
        _ => false,
    }
}

#[plugin(
    name = "film",
    alias = ["nonton", "lk21"],
    category = "search",
    description = "Cari film dan nonton online",
    usage = ".film <judul>",
    example = ".film civil war",
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
pub fn film(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim().to_string();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(resp) = ctx.http.get(&api_url(&query)).ok() else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    if !js_truthy(data.get("status")) || !has_items(data.get("data")) {
        return Reply::text(not_found(&query)).with_reaction("❌");
    }
    // String non-kosong lolos guard lalu `films.forEach` melempar TypeError
    // di JS -> `catch` -> template error.
    let Some(films) = data.get("data").and_then(Value::as_array) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let films = &films[..films.len().min(10)];

    Reply::Interactive {
        body: format_list(films, &query),
    }
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
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
            command: "film".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("film").unwrap().handler)(ctx)
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

    const DUA_FILM: &str = r#"{
        "status": true,
        "data": [
            {"title": "Civil War", "rating": "8.1", "quality": "HD", "release": "2024", "url": "https://tv.neoxr.eu/film/civil-war"},
            {"title": "Dune", "rating": "8.5", "quality": "FHD", "release": "2021", "url": "https://tv.neoxr.eu/film/dune"}
        ]
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("film").unwrap();
        assert_eq!(p.meta.name, "film");
        assert_eq!(p.meta.alias, ["nonton", "lk21"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari film dan nonton online");
        assert_eq!(p.meta.usage, ".film <judul>");
        assert_eq!(p.meta.example, ".film civil war");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        // `pluginConfig` JS tidak menyebut flag permission sama sekali.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["nonton", "lk21"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "film", "alias {a}");
        }
    }

    #[test]
    fn a_blank_title_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn two_films_render_the_numbered_list() {
        let http = Stub::new(DUA_FILM);
        let r = run(http.clone(), "civil war");
        let body = body_of(&r);
        assert!(
            body.starts_with("🎬 *ʜᴀsɪʟ ᴘᴇɴᴄᴀʀɪᴀɴ*\n\n> Ditemukan *2* film untuk \"civil war\"\n\n"),
            "{body}"
        );
        assert!(
            body.contains("*1. Civil War*\n> ⭐ 8.1 | 📺 HD | 📅 2024\n\n"),
            "{body}"
        );
        assert!(
            body.contains("*2. Dune*\n> ⭐ 8.5 | 📺 FHD | 📅 2021\n\n"),
            "{body}"
        );
        assert!(body.ends_with("> _Pilih film dari list di bawah_"), "{body}");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(r.without_mutations(), Reply::Interactive { .. }));
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/film?q=civil%20war&apikey="
        );
    }

    #[test]
    fn only_ten_films_are_listed_and_counted() {
        let films: Vec<Value> = (0..14)
            .map(|i| json!({"title": format!("Film {i}")}))
            .collect();
        let body = format!("{{\"status\": true, \"data\": {}}}", json!(films));
        let out = body_of(&run(Stub::new(&body), "film"));
        assert!(out.contains("> Ditemukan *10* film"), "{out}");
        assert!(out.contains("*10. Film 9*"), "{out}");
        assert!(!out.contains("Film 10"), "film ke-11 terpotong: {out}");
    }

    #[test]
    fn a_missing_field_is_interpolated_as_undefined_like_js() {
        let body = r#"{"status": true, "data": [{"title": "Tanpa Data"}]}"#;
        let out = body_of(&run(Stub::new(body), "x"));
        assert!(
            out.contains("*1. Tanpa Data*\n> ⭐ undefined | 📺 undefined | 📅 undefined\n\n"),
            "{out}"
        );
    }

    #[test]
    fn status_false_or_an_empty_list_reports_not_found() {
        for body in [
            r#"{"status": false, "data": [{"title": "A"}]}"#,
            r#"{"status": true, "data": []}"#,
            r#"{"status": true}"#,
            "bukan json",
        ] {
            let r = run(Stub::new(body), "film lngk");
            assert_eq!(body_of(&r), not_found("film lngk"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn a_non_array_payload_hits_the_catch_branch_like_the_foreach_type_error() {
        let r = run(Stub::new(r#"{"status": true, "data": "x"}"#), "q");
        assert_eq!(body_of(&r), error_template(".", "film"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "q");
        assert!(body_of(&r).contains("command `.film`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_title_is_encoded_like_encode_uri_component() {
        let http = Stub::new(DUA_FILM);
        let _ = run(http.clone(), "film 100% & co's");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/film?q=film%20100%25%20%26%20co's&apikey="
        );
    }

    #[test]
    fn a_very_long_title_is_sent_whole() {
        let http = Stub::new(DUA_FILM);
        let _ = run(http.clone(), &"a".repeat(4_000));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }
}