//! `plugins/tools/izen.js` — bypass shortlink / skiplink menggunakan izen.
//!
//! `ponytail:` `m.react("⏳")` dan `m.react("✅")` terjadi pada satu pesan,
//! sementara `Reply::with_reaction` hanya satu emoji — hanya `✅` yang dibawa.
//! `encode_uri_component` disalin ke sini (duplikasi dari
//! `phisingataubukan.js`) karena rule port tidak menyentuh file plugin lain.
//!
//! Catatan: baris `pub mod izen;` perlu ditambah ke `tools/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `encodeURIComponent` — spasi jadi `%20`, `+'` di-encode. Sama persis dengan
// implementasi di `plugins/tools/phisingataubukan.js` (disalin karena file
// plugin lain dilarang disentuh).
pub(crate) fn encode_uri_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(*b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[plugin(
    name = "izen",
    alias = ["skiplink", "izen"],
    category = "tools",
    description = "Bypass shortlink / skiplink menggunakan izen",
    usage = ".izen link",
    example = ".izen https://sfl.gl/xxxxx",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn izen(ctx: Ctx) -> Reply {
    let link = ctx.args.first().cloned().unwrap_or_default();
    if link.is_empty() {
        return Reply::text(
            "🔗 *SKIPLINK BYPASS* 🔗\n\n\
             Halo kak! Punya link yang ribet ngelewatin iklan? Sini aku bantu lewatin biar langsung ke tujuan akhir!\n\n\
             *Cara Pakai:*\n\
             👉 `.izen <link>`\n\n\
             *Contoh:*\n\
             👉 `.izen https://sfl.gl/xxxxx`",
        );
    }

    let api_url = format!(
        "https://anabot.my.id/api/tools/izenLOL?url={}&apikey=freeApikey",
        encode_uri_component(&link)
    );
    let Ok(res) = ctx.http.get(&api_url) else {
        return Reply::text("❌ Maaf kak, terjadi kesalahan sistem! 😭\nError: HTTP tidak tersedia");
    };
    let json = res.json().unwrap_or(serde_json::Value::Null);

    if !json
        .get("data")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.get("result"))
        .is_some()
    {
        return Reply::text("❌ Waduh kak, gagal ngelewatin link-nya nih! Coba link lain ya.");
    }

    let result = json
        .get("data")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.get("result"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    Reply::text(format!(
        "✅ *BERHASIL BYPASS LINK!* ✅\n\n\
         *Link Asli:* \n\
         🔗 {link}\n\n\
         *Hasil Bypass:* \n\
         🚀 {result}\n\n\
         Semoga ngebantu ya kak! ✨"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: serde_json::Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v.to_string()),
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
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(|s| s.to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            command: "izen".into(),
            http,
            ..Default::default()
        };
        izen(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("izen").unwrap();
        assert_eq!(p.meta.name, "izen");
        assert_eq!(p.meta.alias, ["skiplink", "izen"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_link_tampilkan_bantuan() {
        let r = run(Stub::json(serde_json::json!({})), "");
        assert!(body(&r).contains("SKIPLINK BYPASS"));
    }

    #[test]
    fn encode_uri_component_mirip_js() {
        assert_eq!(encode_uri_component("a b"), "a%20b");
        assert_eq!(encode_uri_component("a+b"), "a%2Bb");
    }

    #[test]
    fn jaringan_gagal_kembalikan_pesan_kesalahan() {
        let r = run(Stub::offline(), "https://sfl.gl/x");
        assert!(body(&r).contains("kesalahan sistem"));
    }

    #[test]
    fn hasil_tanpa_result_mengambil_gagal() {
        let r = run(Stub::json(serde_json::json!({ "data": {} })), "https://sfl.gl/x");
        assert!(body(&r).contains("gagal ngelewatin link-nya nih"));
    }

    #[test]
    fn hasil_berhasil_tampilkan_link_dan_hasil() {
        let data = serde_json::json!({
            "data": { "result": { "result": "https://tujuan.akhiri.com" } }
        });
        let r = run(Stub::json(data), "https://sfl.gl/xxxxx");
        assert!(body(&r).contains("BERHASIL BYPASS LINK!"));
        assert!(body(&r).contains("https://sfl.gl/xxxxx"));
        assert!(body(&r).contains("https://tujuan.akhiri.com"));
    }
}
