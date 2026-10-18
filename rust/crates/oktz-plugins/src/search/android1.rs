//! `plugins/search/android1.js` — cari APK MOD dari Android1 lewat
//! `api.neoxr.eu`.
//!
//! `ponytail:` JS mengirim kartu `sock.sendButton` dengan picker
//! `single_select` (judul "Pilih APK", footer "📱 Android1 Search", dan
//! thumbnail `getAssetBuffer("foto")`). `Reply::Interactive` hanya membawa
//! teksnya, jadi daftar yang sama terkirim tanpa tombol/thumbnail — persis
//! teks `android1.js:83-105`.
//!
//! `ponytail:` JS menyimpan hasil ke `db.db.data.sessions` (map sessions
//! tingkat-atas, di luar `Mutation` yang tersedia) supaya tombol
//! `android1-get` bisa membacanya. Tidak ada plugin di batch ini yang membaca
//! map itu, jadi tulisannya dilewati.
//!
//! `ponytail:` reaksi pertama (`🔍`) tidak ikut: `Reply::Reaction` hanya
//! membawa satu emoji, dan reaksi terakhir (`✅`/`❌`/`☢`) yang menandai hasil
//! akhir — sama seperti `info/jadwalbola.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey.neoxr` (`config.js`) = `process.env.APIKEY_NEOXR || ""`.
///
/// `ponytail:` `Ctx` tidak membawa config, jadi kunci ikut `config.js` yang
/// di-hardcode — persis `info/jadwalbola.rs`. Begitu `.env` diisi, baris ini
/// ikut stale sampai router menyediakannya lewat `Ctx`.
pub(crate) const NEOXR_APIKEY: &str = "";

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `android1.js:42-47` — satu-satunya endpoint pencarian.
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/an1?q={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(text)
    )
}

/// `android1.js:29-36` — kartu penggunaan saat `m.text` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📱 *ᴀɴᴅʀᴏɪᴅ1 sᴇᴀʀᴄʜ*\n\n\
         ╭┈┈⬡「 📋 *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ* 」\n\
         ┃ 🔍 `{prefix}android1 <query>` - Cari APK\n\
         ╰┈┈⬡\n\n\
         > Contoh:\n\
         `{prefix}android1 Subway Surfer`"
    )
}

/// `android1.js:51` — tidak ada hasil.
pub(crate) fn not_found(text: &str) -> String {
    format!("❌ Tidak ditemukan hasil untuk: `{text}`")
}

/// `te(prefix, command, pushName)` (`src/lib/error.js:3-9`).
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    crate::common::error_template(prefix, command)
}

/// `android1.js:65-74` — caption daftar aplikasi.
///
/// Dipanggil dengan `apps` yang SUDAH di-`slice(0, 10)`, jadi `apps.len()`
/// di baris "aplikasi ditemukan" mengikuti panjang yang sudah dipotong —
/// sama seperti `apps.length` di JS.
pub(crate) fn format_list(apps: &[Value], text: &str) -> String {
    let mut caption = format!("📱 Hasil dari pencarian apk mod *{text}*\n");
    caption.push_str(&format!("*{}* aplikasi ditemukan\n\n", apps.len()));
    for (i, app) in apps.iter().enumerate() {
        caption.push_str(&format!(
            "*{}.* {}\n   ├ 👤 {}\n   └ ⭐ {}/5\n\n",
            i + 1,
            interp(app.get("name")),
            interp(app.get("developer")),
            interp(app.get("rating")),
        ));
    }
    caption.push_str("> Pilih angka untuk download langsung");
    caption
}

#[plugin(
    name = "android1",
    alias = ["an1"],
    category = "search",
    description = "Cari dan download APK MOD dari Android1",
    usage = ".android1 <query>",
    example = ".android1 Subway Surfer",
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
pub fn android1(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(resp) = ctx.http.get(&api_url(text)).ok() else {
        // `catch (err)` — ☢ lalu template error.
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    // Body yang bukan JSON jadi string di axios; `.status`/`.data` undefined
    // dan guard di bawah jatuh ke cabang "tidak ditemukan".
    let data = resp.json().unwrap_or(Value::Null);

    // `!data?.status || !data?.data?.length`: array/string non-kosong lolos.
    let block = data.get("data");
    let length_truthy = match block {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::String(s)) => !s.is_empty(),
        _ => false,
    };
    if !js_truthy(data.get("status")) || !length_truthy {
        return Reply::text(not_found(text)).with_reaction("❌");
    }
    // `data.data` string non-kosong lolos guard lalu `apps.forEach` melempar
    // TypeError di JS -> `catch` -> ☢.
    let Some(apps) = block.and_then(Value::as_array) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let apps = &apps[..apps.len().min(10)];

    Reply::Interactive {
        body: format_list(apps, text),
    }
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
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
            let body = self.body.clone().ok_or_else(|| oktz_plugin_api::HttpError {
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
            command: "android1".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("android1").unwrap().handler)(ctx)
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

    const TWO_APPS: &str = r#"{
        "status": true,
        "data": [
            {"name": "Subway Surfers", "developer": "SYBO", "rating": 4.5, "url": "https://an1.com/a"},
            {"name": "Shadow Fight 2", "developer": "NEKKI", "rating": 4.7, "url": "https://an1.com/b"}
        ]
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("android1").unwrap();
        assert_eq!(p.meta.name, "android1");
        assert_eq!(p.meta.alias, ["an1"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari dan download APK MOD dari Android1");
        assert_eq!(p.meta.usage, ".android1 <query>");
        assert_eq!(p.meta.example, ".android1 Subway Surfer");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("an1").unwrap().meta.name, "android1");
    }

    #[test]
    fn an_empty_query_shows_the_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn two_apps_render_the_numbered_caption_and_the_download_hint() {
        let http = Stub::new(TWO_APPS);
        let r = run(http.clone(), "subway surfer");
        let body = body_of(&r);
        assert!(body.starts_with("📱 Hasil dari pencarian apk mod *subway surfer*\n"));
        assert!(body.contains("*2* aplikasi ditemukan\n\n"), "{body}");
        assert!(
            body.contains("*1.* Subway Surfers\n   ├ 👤 SYBO\n   └ ⭐ 4.5/5\n\n"),
            "{body}"
        );
        assert!(
            body.contains("*2.* Shadow Fight 2\n   ├ 👤 NEKKI\n   └ ⭐ 4.7/5\n\n"),
            "{body}"
        );
        assert!(body.ends_with("> Pilih angka untuk download langsung"), "{body}");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(r.without_mutations().tanpa_reaksi(), Reply::Interactive { .. }));

        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/an1?q=subway%20surfer&apikey="
        );
    }

    #[test]
    fn only_ten_apps_are_listed_and_counted() {
        let apps: Vec<Value> = (0..14)
            .map(|i| serde_json::json!({"name": format!("App {i}"), "developer": "D", "rating": 4}))
            .collect();
        let body = format!("{{\"status\": true, \"data\": {}}}", serde_json::json!(apps));
        let out = body_of(&run(Stub::new(&body), "app"));
        assert!(out.contains("*10* aplikasi ditemukan"), "{out}");
        assert!(out.contains("*10.* App 9"), "{out}");
        assert!(!out.contains("*11.*"), "hasil ke-11 terpotong: {out}");
    }

    #[test]
    fn a_missing_field_is_interpolated_as_undefined_like_js() {
        let body = r#"{"status": true, "data": [{"name": "Cuma Nama"}]}"#;
        let out = body_of(&run(Stub::new(body), "x"));
        assert!(out.contains("*1.* Cuma Nama\n   ├ 👤 undefined\n   └ ⭐ undefined/5\n\n"), "{out}");
    }

    #[test]
    fn status_false_or_an_empty_list_reports_not_found() {
        for body in [
            r#"{"status": false, "data": [{"name": "A"}]}"#,
            r#"{"status": true, "data": []}"#,
            r#"{"status": true}"#,
            r#"bukan json"#,
        ] {
            let r = run(Stub::new(body), "subway surfers & co");
            assert_eq!(body_of(&r), not_found("subway surfers & co"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn a_non_array_payload_hits_the_catch_branch_like_the_foreach_type_error() {
        // `"data": "x"` -> `"x".length` truthy -> `apps.forEach` bukan fungsi.
        let r = run(Stub::new(r#"{"status": true, "data": "x"}"#), "q");
        assert_eq!(body_of(&r), error_reply(".", "android1"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "q");
        assert!(body_of(&r).contains("command `.android1`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_query_is_encoded_like_encode_uri_component() {
        let http = Stub::new(TWO_APPS);
        let _ = run(http.clone(), "pes 2024 & co's");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/an1?q=pes%202024%20%26%20co's&apikey="
        );
    }

    #[test]
    fn a_very_long_query_is_not_truncated_and_does_not_panic() {
        let query = "a".repeat(4_000);
        let http = Stub::new(TWO_APPS);
        let _ = run(http.clone(), &query);
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "query utuh: {}", seen[0].len());
    }

    #[test]
    fn the_list_helper_never_panics_on_strange_values() {
        let apps = [serde_json::json!("bukan objek"), serde_json::json!(null), serde_json::json!({"name": 7})];
        let out = format_list(&apps, "🚀 \u{0} utf8 ñ");
        assert!(out.contains("*3* aplikasi ditemukan"), "{out}");
    }
}
