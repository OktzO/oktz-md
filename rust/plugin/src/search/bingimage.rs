//! `plugins/search/bingimage.js` — cari artwork lewat `api-faa.my.id`.
//!
//! `ponytail:` hasil pencarian dikirim sebagai `albumMessage` (`sock.sendMessage`),
//! dan `Reply` hanya punya satu sumber media per balasan — tidak ada padanan
//! album. Yang dipindah: guard kata kunci, daftar URL hasil, unduhan tiap
//! gambar (termasuk kegagalan unduhan yang di JS masuk `catch`), dan reaksi.
//! Balasan tanpa media yang berhasil dikirim apa adanya — tidak ada teks lain
//! di JS untuk album itu.
//!
//! `ponytail:` `fotoApi.apiFaa.get("/faa/google-image", …)` adalah
//! `src/lib/apimanager.js` yang menambah `User-Agent` bawaan dan parameter
//! query; URL-nya ditulis langsung di sini dengan bentuk yang sama.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `bingimage.js:27` — guard kata kunci.
pub(crate) fn no_query(prefix: &str) -> String {
    format!("❌ *Masukkan kata kunci pencarian!*\n\n> Contoh: {prefix}carigambar rem")
}

/// `bingimage.js:45` — `status` falsy.
pub(crate) fn not_found(query: &str) -> String {
    format!("❌ *Tidak ditemukan hasil untuk:* {query}")
}

/// `bingimage.js:34-41` — `apiFaa` provider `baseURL` + endpoint + params.
pub(crate) fn api_url(query: &str) -> String {
    format!(
        "https://api-faa.my.id/faa/google-image?query={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(query)
    )
}

/// `data.result` sebagai daftar URL — `results.map(url => …)` butuh array;
/// bentuk lain membuat `.map` melempar TypeError dan masuk `catch` (`☢`).
pub(crate) fn result_urls(data: &Value) -> Option<Vec<String>> {
    data.get("result")?
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_string))
        .collect()
}

#[plugin(
    name = "bingimage",
    alias = ["imagesearch", "carigambar", "bingimg"],
    category = "search",
    description = "Cari artwork di Pixiv",
    usage = ".carigambar <query>",
    example = ".carigambar rem",
    cooldown = 5,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bingimage(ctx: Ctx) -> Reply {
    let query = ctx.text.clone();

    if query.is_empty() {
        return Reply::text(no_query(&ctx.prefix));
    }

    let error = crate::common::error_template(&ctx);
    let Some(resp) = ctx.http.get(&api_url(&query)).ok() else {
        return Reply::text(error).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    if !crate::common::js_truthy(data.get("status")) {
        return Reply::text(not_found(&query)).with_reaction("❌");
    }

    // `results.map(...)` di dalam `Promise.all`: bentuk lain bukan array ->
    // TypeError -> `catch` -> template error.
    let Some(urls) = result_urls(&data) else {
        return Reply::text(error).with_reaction("☢");
    };

    // `axios.get(url, { responseType: "arraybuffer" })` untuk tiap hasil. Satu
    // gagal saja sudah membuat `Promise.all` menolak -> `catch`.
    for url in &urls {
        if ctx.http.get(url).is_err() {
            return Reply::text(error).with_reaction("☢");
        }
    }

    // Album tidak bisa dikirim; lihat catatan `ponytail:` di kepala file.
    Reply::None.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub dengan dua tahap: endpoint pencarian (prefix penuh) dan host gambar
    /// apa pun. `gambar_ok` menentukan apakah unduhan gambar berhasil.
    #[derive(Debug)]
    struct Stub {
        cari: Option<String>,
        gambar_ok: bool,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(cari: &str) -> Arc<Stub> {
            Arc::new(Stub {
                cari: Some(cari.to_string()),
                gambar_ok: true,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline(cari: &str) -> Arc<Stub> {
            Arc::new(Stub {
                cari: Some(cari.to_string()),
                gambar_ok: false,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                cari: None,
                gambar_ok: true,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = if url.starts_with("https://api-faa.my.id/faa/google-image") {
                self.cari.clone()
            } else if self.gambar_ok {
                Some("PNG".to_string())
            } else {
                None
            };
            let body = body.ok_or_else(|| oktz_plugin_api::HttpError {
                message: "offline".into(),
            })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: Some("application/octet-stream".into()),
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
            command: "carigambar".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("bingimage").unwrap().handler)(ctx)
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

    /// Balasan di dalam pembungkus reaksi — `is_none()` tidak melihat ke
    /// dalam `Reaction`, jadi harus diturunkan sendiri.
    fn inner_of(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            other => other,
        }
    }

    const DUA_HASIL: &str = r#"{"status": true, "result": ["https://img.example/a.png", "https://img.example/b.png"]}"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bingimage").unwrap();
        assert_eq!(p.meta.name, "bingimage");
        assert_eq!(p.meta.alias, ["imagesearch", "carigambar", "bingimg"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari artwork di Pixiv");
        assert_eq!(p.meta.usage, ".carigambar <query>");
        assert_eq!(p.meta.example, ".carigambar rem");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["imagesearch", "carigambar", "bingimg"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "bingimage",
                "alias {a}"
            );
        }
    }

    #[test]
    fn an_empty_query_shows_the_hint_without_a_request() {
        let http = Stub::empty();
        // `m.text` tidak di-`trim` di JS, jadi spasi saja TIDAK kosong falsy.
        assert_eq!(body_of(&run(http.clone(), "")), no_query("."));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn every_result_url_is_downloaded_before_the_reply() {
        let http = Stub::new(DUA_HASIL);
        let r = run(http.clone(), "rem");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 3, "1 cari + 2 gambar: {seen:?}");
        assert_eq!(
            seen[0],
            "https://api-faa.my.id/faa/google-image?query=rem&apikey="
        );
        assert_eq!(seen[1], "https://img.example/a.png");
        assert_eq!(seen[2], "https://img.example/b.png");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(
            matches!(inner_of(&r), Reply::None),
            "album tidak punya padanan Reply, jadi tidak ada media/text"
        );
    }

    #[test]
    fn a_falsy_status_reports_not_found_without_downloading() {
        for body in [
            r#"{"status": false, "result": ["https://img.example/a.png"]}"#,
            r#"{"result": ["https://img.example/a.png"]}"#,
            "bukan json",
        ] {
            let http = Stub::new(body);
            let r = run(http.clone(), "rem");
            assert_eq!(body_of(&r), not_found("rem"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
            assert_eq!(http.seen.lock().unwrap().len(), 1, "{body}");
        }
    }

    #[test]
    fn a_result_that_is_not_an_array_hits_the_catch_branch() {
        for body in [
            r#"{"status": true, "result": "bukan array"}"#,
            r#"{"status": true, "result": {"a": 1}}"#,
            r#"{"status": true, "result": [1, 2]}"#,
            r#"{"status": true}"#,
        ] {
            let r = run(Stub::new(body), "rem");
            assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "carigambar"), "{body}");
            assert_eq!(reaction_of(&r), Some("☢"), "{body}");
        }
    }

    #[test]
    fn one_failed_image_download_rejects_the_whole_promise_all() {
        let http = Stub::offline(DUA_HASIL);
        let r = run(http.clone(), "rem");
        assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "carigambar"));
        assert_eq!(reaction_of(&r), Some("☢"));
        assert_eq!(http.seen.lock().unwrap().len(), 2, "berhenti di gambar pertama");
    }

    #[test]
    fn a_transport_failure_on_the_search_reports_the_error_template() {
        let http = Stub::empty();
        let r = run(http, "rem");
        assert!(body_of(&r).contains("command `.carigambar`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn an_empty_result_list_downloads_nothing_and_still_reacts_ok() {
        let http = Stub::new(r#"{"status": true, "result": []}"#);
        let r = run(http.clone(), "rem");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen, ["https://api-faa.my.id/faa/google-image?query=rem&apikey="]);
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(inner_of(&r), Reply::None));
    }

    #[test]
    fn the_query_is_encoded_like_encode_uri_component() {
        let http = Stub::new(r#"{"status": true, "result": []}"#);
        let _ = run(http.clone(), "rem 01 & 'x'");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api-faa.my.id/faa/google-image?query=rem%2001%20%26%20'x'&apikey="
        );
    }

    #[test]
    fn a_very_long_query_is_sent_whole() {
        let http = Stub::new(r#"{"status": true, "result": []}"#);
        let _ = run(http.clone(), &"a".repeat(4_000));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }
}