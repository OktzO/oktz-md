//! `plugins/anime/topanime.js` — 10 karakter anime terpopuler dari izuka-api.
//!
//! Balasan berupa gambar (`list[0].image`) dengan caption daftar; kalau entri
//! pertama tidak punya gambar, daftar polos yang dikirim.
//!
//! `ponytail:` JS mengirim gambar dengan `{ quoted: m }`. `Reply::Image` tidak
//! punya penanda kutipan, jadi pipeline yang menentukan.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;

/// Endpoint tunggal (`topanime.js:22`).
const ENDPOINT: &str = "https://my.izuka-api.xyz/api/anime/top-anime";

/// `Math.min(list.length, 10)` (`topanime.js:35`).
const MAX_ITEMS: usize = 10;

/// Kalimat saat data tidak lengkap (`topanime.js:27`).
const NO_DATA: &str = "Maaf, sistem gagal mengambil daftar Top Anime.";

/// Kalimat saat request-nya sendiri gagal (`topanime.js:58`).
const SERVER_ERROR: &str = "Maaf, terjadi kesalahan saat menghubungi server. Silakan coba lagi.";

/// `${nilai}` di JS: string apa adanya, sisanya JSON kompak.
fn interp(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "undefined".to_string(),
    }
}

/// `topanime.js:32-45`.
fn render_list(list: &[Value]) -> String {
    let mut txt = String::from("🏆 *TOP ANIME CHARACTERS (WAIFU LIST)* 🏆\n\n");
    txt.push_str("Berikut adalah daftar karakter terpopuler dan paling dicintai saat ini:\n\n");

    let max_items = list.len().min(MAX_ITEMS);
    for character in list.iter().take(max_items) {
        txt.push_str(&format!(
            "*{} - {}*\n",
            interp(character.get("rank")),
            interp(character.get("name"))
        ));
        txt.push_str(&format!(
            "- *Jepang:* {}\n",
            interp(character.get("japanese"))
        ));
        txt.push_str(&format!(
            "- *Anime:* {}\n",
            interp(character.get("anime"))
        ));
        txt.push_str(&format!(
            "- *Favorit:* {} | *Votes:* {}\n\n",
            interp(character.get("favorites")),
            interp(character.get("votes"))
        ));
    }

    txt.push_str(&format!("_Menampilkan {max_items} karakter teratas._"));
    txt
}

#[plugin(
    name = "topanime",
    alias = ["top-anime", "waifutop"],
    category = "anime",
    description = "Melihat daftar karakter anime / waifu terpopuler",
    usage = ".topanime",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn topanime(ctx: Ctx) -> Reply {
    let Ok(res) = ctx.http.get(ENDPOINT) else {
        return Reply::text(SERVER_ERROR).with_reaction("❌");
    };
    let Some(data) = res.json() else {
        return Reply::text(SERVER_ERROR).with_reaction("❌");
    };

    // `!data.result.result` harus ada DAN berupa array; kalau bukan array,
    // `list[0].image` di JS melempar TypeError dan jatuh ke `catch`.
    if !js_truthy(data.get("status")) || !js_truthy(data.get("result")) {
        return Reply::text(NO_DATA).with_reaction("❌");
    }
    let Some(list) = data
        .get("result")
        .and_then(|r| r.get("result"))
        .and_then(Value::as_array)
    else {
        return Reply::text(SERVER_ERROR).with_reaction("❌");
    };

    let txt = render_list(list);
    // `list[0].image` truthy -> kirim gambar; selain itu teks polos.
    let image = list
        .first()
        .and_then(|c| c.get("image"))
        .filter(|v| js_truthy(Some(*v)))
        .and_then(Value::as_str);
    match image {
        Some(url) => Reply::image(MediaSource::url(url), txt).with_reaction("✅"),
        None => Reply::text(txt).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<String>,
        seen: Mutex<Vec<String>>,
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
                return Err(HttpError {
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
            self.seen.lock().unwrap().push(url.into());
            self.respond()
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
    }

    fn run(http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "topanime".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("topanime").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    fn emoji(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    /// Satu entri dengan field lengkap supaya render bisa diuji terpisah.
    fn entry(rank: i64, name: &str, image: Option<&str>) -> Value {
        json!({
            "rank": rank,
            "name": name,
            "japanese": format!("{name} JP"),
            "anime": format!("Anime {name}"),
            "favorites": rank * 1000,
            "votes": rank * 10,
            "image": image,
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("topanime").unwrap();
        assert_eq!(p.meta.name, "topanime");
        assert_eq!(p.meta.alias, ["top-anime", "waifutop"]);
        assert_eq!(p.meta.category, "anime");
        assert_eq!(
            p.meta.description,
            "Melihat daftar karakter anime / waifu terpopuler"
        );
        assert_eq!(p.meta.usage, ".topanime");
        assert_eq!(p.meta.example, "", "JS tidak punya example");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["topanime", "top-anime", "waifutop"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("topanime"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn a_normal_list_becomes_an_image_with_the_caption() {
        let payload = json!({
            "status": true,
            "result": { "result": [
                entry(1, "Gojo", Some("https://cdn.example/gojo.png")),
                entry(2, "Yuji", Some("https://cdn.example/yuji.png")),
            ]}
        });
        let r = run(Stub::ok(&payload.to_string()));
        let Some(MediaSource::Url { url, .. }) = r.media_source() else {
            panic!("gambar entri pertama harus jadi media URL");
        };
        assert_eq!(url, "https://cdn.example/gojo.png");
        assert_eq!(emoji(&r), "✅");

        let t = body(&r);
        assert!(t.starts_with("🏆 *TOP ANIME CHARACTERS (WAIFU LIST)* 🏆\n\n"));
        assert!(t.contains("*1 - Gojo*"), "{t}");
        assert!(t.contains("- *Jepang:* Gojo JP"), "{t}");
        assert!(t.contains("- *Favorit:* 1000 | *Votes:* 10"), "{t}");
        assert!(t.ends_with("_Menampilkan 2 karakter teratas._"), "{t}");
    }

    #[test]
    fn at_most_ten_entries_are_rendered() {
        let list: Vec<Value> = (1..=15)
            .map(|i| entry(i, &format!("C{i}"), None))
            .collect();
        let txt = render_list(&list);
        assert!(txt.contains("*10 - C10*"), "entri ke-10 ikut");
        assert!(!txt.contains("*11 - C11*"), "entri ke-11 harus terpotong");
        assert!(txt.ends_with("_Menampilkan 10 karakter teratas._"), "{txt}");
    }

    #[test]
    fn a_list_without_a_leading_image_falls_back_to_plain_text() {
        let payload = json!({
            "status": true,
            "result": { "result": [entry(1, "Gojo", None)] }
        });
        let r = run(Stub::ok(&payload.to_string()));
        assert!(!r.has_media(), "tanpa gambar tidak boleh jadi Reply::Image");
        assert!(body(&r).contains("*1 - Gojo*"));
        assert_eq!(emoji(&r), "✅");
    }

    #[test]
    fn an_empty_list_still_renders_the_frame() {
        let payload = json!({ "status": true, "result": { "result": [] } });
        let r = run(Stub::ok(&payload.to_string()));
        assert!(!r.has_media());
        assert!(body(&r).ends_with("_Menampilkan 0 karakter teratas._"));
    }

    #[test]
    fn missing_or_falsy_status_reports_the_gagal_branch() {
        for payload in [
            json!({ "status": false, "result": { "result": [] } }),
            json!({ "status": true }),
            json!({ "status": true, "result": {} }),
            json!({ "status": true, "result": { "result": null } }),
        ] {
            let r = run(Stub::ok(&payload.to_string()));
            assert_eq!(body(&r), NO_DATA, "{payload}");
            assert_eq!(emoji(&r), "❌");
        }
    }

    #[test]
    fn an_unusable_shape_or_a_dead_server_reports_the_server_error() {
        // `result.result` bukan array: di JS `list[0].image` melempar.
        let r = run(Stub::ok(r#"{"status":true,"result":{"result":{"a":1}}}"#));
        assert_eq!(body(&r), SERVER_ERROR);
        assert_eq!(emoji(&r), "❌");

        let r = run(Stub::offline());
        assert_eq!(body(&r), SERVER_ERROR);

        let r = run(Stub::ok("bukan json"));
        assert_eq!(body(&r), SERVER_ERROR);
    }

    #[test]
    fn missing_fields_render_as_undefined_like_template_literal() {
        let txt = render_list(&[json!({})]);
        assert!(txt.contains("*undefined - undefined*"), "{txt}");
    }
}