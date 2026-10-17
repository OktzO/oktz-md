//! `plugins/search/pixiv.js` — cari artwork di Pixiv lewat `api.neoxr.eu`.
//!
//! `ponytail:` `config.APIkey?.neoxr` tidak ada di `Ctx`, jadi kunci dikunci
//! `""` — pola `search::android1` dan `search::mangatoon`. Query `apikey=`
//! tetap dikirim apa adanya, sama seperti `|| ''` di JS.
//!
//! `ponytail:` `contextInfo.forwardedNewsletterMessageInfo` +
//! `sock.sendMessage` tidak ada padanannya di `Reply`; yang dikirim hanya teks
//! caption-nya. `config.saluran` / `config.bot.name` juga tidak ada di `Ctx`,
//! dan tanpa newsletter info nilainya juga tidak terpakai.
//!
//! `ponytail:` `error.response?.status === 403` tidak bisa dibedakan: `HttpError`
//! hanya membawa `message`, bukan status. Cabang 403 karena itu tidak
//! terjangkau; teksnya tetap ditulis sebagai fungsi murni.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{error_template, js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''`.
const NEOXR_APIKEY: &str = "";

/// `pixiv.js:26` — guard kata kunci kosong.
pub(crate) fn butuh_query() -> String {
    "❌ *Masukkan kata kunci pencarian!*\n\n> Contoh: .pixiv rem".to_string()
}

/// `pixiv.js:84` — khusus status 403. Tidak terjangkau tanpa status di
/// `HttpError`.
#[allow(dead_code)]
pub(crate) fn api_key_tidak_valid() -> String {
    "❌ *API Key tidak valid atau limit tercapai*".to_string()
}

/// `pixiv.js:33` — endpoint + kunci.
pub(crate) fn api_url(query: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/pixiv-search?q={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(query)
    )
}

/// `${value}` di JS.
fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `data.data.slice(0, 10)` (`pixiv.js:43`).
pub(crate) fn artworks(data: &Value) -> Vec<Value> {
    data.get("data")
        .and_then(Value::as_array)
        .map(|a| a.iter().take(10).cloned().collect())
        .unwrap_or_default()
}

/// `pixiv.js:48-61` — caption. Label AI dan NSFW menempel ke judul tanpa spasi,
/// persis seperti di JS.
pub(crate) fn caption(results: &[Value], query: &str) -> String {
    let mut cap = String::from("🎨 *ᴘɪxɪᴠ sᴇᴀʀᴄʜ*\n");
    cap.push_str(&format!("📝 *ᴋᴜᴇʀʛ:* {query}\n"));
    cap.push_str(&format!(
        "📊 *ʜᴀsɪʟ:* {} artwork\n\n",
        results.len()
    ));
    for (i, art) in results.iter().enumerate() {
        let ai_label = if art.get("aiType").and_then(Value::as_i64) == Some(2) {
            " 🤖"
        } else {
            ""
        };
        // `xRestrict > 0` memakai OPERATOR ">": string angka ikut dikoersi
        // (`"1" > 0` = true di JS), berbeda dari `===` yang dipakai `aiType`.
        let nsfw = if number_at_least_one(art.get("xRestrict")) {
            " 🔞"
        } else {
            ""
        };
        cap.push_str(&format!(
            "*{}.* {}{ai_label}{nsfw}\n   👤 {}\n   📐 {}x{} • 📄 {} page\n   🔗 {}\n\n",
            i + 1,
            interp(art.get("title")),
            interp(art.get("userName")),
            interp(art.get("width")),
            interp(art.get("height")),
            interp(art.get("pageCount")),
            interp(art.get("url"))
        ));
    }
    cap.push_str("> 🎨 Powered by Pixiv");
    cap
}

/// `n > 0` — `ToNumber` JS: string angka ikut, `"abc"` jadi `NaN` (false),
/// `null` jadi `0` (false), `true` jadi `1` (true).
fn number_at_least_one(value: Option<&Value>) -> bool {
    let n = match value {
        None => return false,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Some(Value::Bool(b)) => i32::from(*b) as f64,
        Some(Value::Null) => 0.0,
        Some(Value::Array(_)) | Some(Value::Object(_)) => f64::NAN,
    };
    n.is_finite() && n > 0.0
}

/// `!data.status || !data.data || data.data.length === 0` (`pixiv.js:38`).
pub(crate) fn kosong(data: &Value) -> bool {
    !js_truthy(data.get("status"))
        || data
            .get("data")
            .map_or(true, |d| !js_truthy(Some(d)))
        || artworks(data).is_empty()
}

#[plugin(
    name = "pixiv",
    alias = ["pixivsearch", "caripixiv"],
    category = "search",
    description = "Cari artwork di Pixiv",
    usage = ".pixiv <query>",
    example = ".pixiv rem",
    cooldown = 5,
    energi = 2,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pixiv(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim().to_string();

    if query.is_empty() {
        return Reply::text(butuh_query());
    }

    // `await m.react("🔍")` di `:30` sudah jalan sebelum request.
    let Ok(res) = ctx.http.get(&api_url(&query)) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let data = res.json().unwrap_or(Value::Null);
    if kosong(&data) {
        return Reply::text(format!("❌ *Tidak ditemukan hasil untuk:* {query}"))
            .with_reaction("❌");
    }

    let results = artworks(&data);

    // `await m.react("🎨")` di `:63` menggantikan `🔍` sebelum mengirim.
    Reply::text(caption(&results, &query)).with_reaction("🎨")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        body: Option<Value>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn with(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let v = self
                .body
                .clone()
                .ok_or_else(|| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(v.to_string().into_bytes()),
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
            command: "pixiv".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("pixiv").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pixiv").unwrap();
        assert_eq!(p.meta.name, "pixiv");
        assert_eq!(p.meta.alias, ["pixivsearch", "caripixiv"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari artwork di Pixiv");
        assert_eq!(p.meta.usage, ".pixiv <query>");
        assert_eq!(p.meta.example, ".pixiv rem");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pixivsearch", "caripixiv"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pixiv");
        }
    }

    #[test]
    fn an_empty_query_asks_for_a_keyword() {
        let http = Stub::offline();
        for text in ["", "   ", "\t\n"] {
            assert_eq!(
                text_of(&run(http.clone(), text)),
                "❌ *Masukkan kata kunci pencarian!*\n\n> Contoh: .pixiv rem",
                "input {text:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
        assert_eq!(reaction_of(&run(Stub::offline(), "")), None);
    }

    #[test]
    fn a_hit_uses_the_artist_palette_reaction() {
        let http = Stub::with(json!({
            "status": true,
            "data": [ { "title": "Fanart", "userName": "Seniman", "width": 1000,
                        "height": 1500, "pageCount": 2, "url": "https://p/1" } ],
        }));
        let r = run(http.clone(), "rem");
        let t = text_of(&r);
        assert_eq!(reaction_of(&r), Some("🎨"), "🔍 digantikan 🎨");
        assert!(t.starts_with("🎨 *ᴘɪxɪᴠ sᴇᴀʀᴄʜ*\n📝 *ᴋᴜᴇʀʛ:* rem\n📊 *ʜᴀsɪʟ:* 1 artwork\n\n"), "{t}");
        assert!(t.contains("*1.* Fanart\n   👤 Seniman\n   📐 1000x1500 • 📄 2 page\n   🔗 https://p/1\n\n"), "{t}");
        assert!(t.ends_with("> 🎨 Powered by Pixiv"), "{t}");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/pixiv-search?q=rem&apikey="
        );
    }

    #[test]
    fn ai_and_nsfw_badges_attach_without_a_space() {
        let body = json!({
            "status": true,
            "data": [
                { "title": "AI", "aiType": 2, "xRestrict": 1 },
                { "title": "AI saja", "aiType": 2, "xRestrict": 0 },
                { "title": "NSFW saja", "xRestrict": 3 },
                { "title": "Bersih", "xRestrict": 0 },
            ],
        });
        let t = caption(&artworks(&body), "q");
        assert!(t.contains("*1.* AI 🤖 🔞\n"), "{t}");
        assert!(t.contains("*2.* AI saja 🤖\n"), "{t}");
        assert!(t.contains("*3.* NSFW saja 🔞\n"), "{t}");
        assert!(t.contains("*4.* Bersih\n"), "{t}");
    }

    #[test]
    fn only_ten_artworks_are_listed() {
        let data: Vec<Value> = (0..13)
            .map(|i| json!({ "title": format!("Art {i}"), "url": format!("https://p/{i}") }))
            .collect();
        let body = json!({ "status": true, "data": data });
        let t = caption(&artworks(&body), "q");
        assert!(t.contains("📊 *ʜᴀsɪʟ:* 10 artwork\n"), "{t}");
        assert!(t.contains("*10.* Art 9\n"), "{t}");
        assert!(!t.contains("Art 10"), "hasil ke-11 terpotong: {t}");
    }

    #[test]
    fn every_empty_or_broken_shape_reports_no_result() {
        for body in [
            json!({ "status": false, "data": [ { "title": "x" } ] }),
            json!({ "status": true, "data": [] }),
            json!({ "status": true }),
            json!({}),
            json!(null),
            json!("bukan json"),
        ] {
            assert!(kosong(&body), "body {body}");
            let r = run(Stub::with(body), "rem");
            assert_eq!(
                text_of(&r),
                "❌ *Tidak ditemukan hasil untuk:* rem",
                "body harus menolak"
            );
            assert_eq!(reaction_of(&r), Some("❌"));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "rem");
        assert_eq!(text_of(&r), error_template(".", "pixiv"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_403_text_exists_even_though_the_branch_is_unreachable() {
        assert_eq!(
            api_key_tidak_valid(),
            "❌ *API Key tidak valid atau limit tercapai*"
        );
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "%zz"] {
            let r = run(Stub::with(json!({ "status": true, "data": [] })), input);
            assert!(text_of(&r).contains("Tidak ditemukan"), "input {input:?}");
        }
        let panjang = "a".repeat(5000);
        let r = run(Stub::offline(), &panjang);
        assert_eq!(text_of(&r), error_template(".", "pixiv"));
    }

    #[test]
    fn missing_artwork_fields_print_undefined() {
        let body = json!({ "status": true, "data": [ {} ] });
        let t = caption(&artworks(&body), "q");
        assert!(t.contains("*1.* undefined\n   👤 undefined\n   📐 undefinedxundefined"), "{t}");
    }
}