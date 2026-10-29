//! `plugins/tools/spamngl.js` — send NGL Spam.
//!
//! Catatan: `mod.rs` kategori `tools` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod spamngl;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/tools/mod.rs`.
//!
//! `setTimeout(4000)` tidak punya padanan di `Ctx`; loop dijalankan tanpa
//! jeda. `axios.get` tidak di-await di JS (dikirim sekaligus); di Rust
//! setiap panggilan `ctx.http.get` adalah sinkron, jadi loop akan menunggu
//! satu per satu.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

#[plugin(
    name = "spamngl",
    alias = [],
    category = "tools",
    description = "Send NGL Spam",
    usage = ".spamngl <url> | <text> | <jumlah>",
    example = ".spamngl https://ngl.link/xxxx | hai | 10",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn spamngl(ctx: Ctx) -> Reply {
    let parts: Vec<&str> = ctx.text.split('|').collect();
    let link = parts.first().copied().unwrap_or("").trim();
    let kata = parts.get(1).copied().unwrap_or("").trim();
    let jumlah_str = parts.get(2).copied().unwrap_or("").trim();

    if link.is_empty() {
        return Reply::text(format!(
            "*LINK NGL NYA MANA ??*\nContoh: `{prefix}spamngl https://ngl.link/xxxx | hai | 10`",
            prefix = ctx.prefix
        ));
    }
    if kata.is_empty() {
        return Reply::text(format!(
            "*KATA KATA NYA MANA ??*\n\nContoh: `{prefix}spamngl https://ngl.link/xxxx | hai | 10`",
            prefix = ctx.prefix
        ));
    }
    if jumlah_str.is_empty() {
        return Reply::text(format!(
            "*JUMLAH NYA MANA ??*\n\nContoh: `{prefix}spamngl https://ngl.link/xxxx | hai | 10`",
            prefix = ctx.prefix
        ));
    }
    let jumlah: i64 = match jumlah_str.parse() {
        Ok(n) if n >= 0 => n,
        _ => {
            return Reply::text(format!(
                "*JUMLAH NYA HARUS ANGKA*\n\nContoh: `{prefix}spamngl https://ngl.link/xxxx | hai | 10`",
                prefix = ctx.prefix
            ));
        }
    };

    let api_key = ctx.config().api_keys.cuki.clone();
    if api_key.is_empty() {
        return Reply::text(
            "❌ *API Key belum diset!*\n\n\
             > Isi `APIKEY_CUKI` di file `.env`, lalu restart bot",
        );
    }

    // JS tidak menunggu setiap axios.get; di Rust loop sinkron tanpa jeda.
    for _ in 0..jumlah {
        let _ = ctx.http.get(&format!(
            "https://api.cuki.biz.id/api/tools/sendngl?apikey={}&link={}&text={}",
            encode_uri_component(&api_key),
            encode_uri_component(link),
            encode_uri_component(kata)
        ));
    }

    Reply::text(format!(
        "✅ *DONE*\n\n\
         Berhasil mengirim spam NGL Message!\n\
         Target: {link}\n\
         Pesan: {kata} ({jumlah}x)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::Value;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    // `HttpGet` butuh `Debug` pada implementor.
    #[derive(Debug)]
    struct Stub {
        call_count: Mutex<i64>,
    }

    impl Stub {
        fn new() -> Arc<Stub> {
            Arc::new(Stub {
                call_count: Mutex::new(0),
            })
        }
    }

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            *self.call_count.lock().unwrap() += 1;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(b"{}".to_vec()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "spamngl".into(),
            text: "https://ngl.link/xxxx | hai | 3".into(),
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    cuki: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("spamngl").unwrap();
        assert_eq!(p.meta.name, "spamngl");
        assert!(p.meta.alias.is_empty(), "alias JS: []");
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_link_mengembalikan_error() {
        let mut c = ctx();
        c.text = " | hai | 3".into();
        let r = spamngl(c);
        assert!(text_of(&r).contains("LINK NGL NYA MANA ??"));
    }

    #[test]
    fn tanpa_kata_mengembalikan_error() {
        let mut c = ctx();
        c.text = "https://ngl.link/xxxx | | 3".into();
        let r = spamngl(c);
        assert!(text_of(&r).contains("KATA KATA NYA MANA ??"));
    }

    #[test]
    fn tanpa_jumlah_mengembalikan_error() {
        let mut c = ctx();
        c.text = "https://ngl.link/xxxx | hai |".into();
        let r = spamngl(c);
        assert!(text_of(&r).contains("JUMLAH NYA MANA ??"));
    }

    #[test]
    fn jumlah_bukan_angka_ditolak() {
        let mut c = ctx();
        c.text = "https://ngl.link/xxxx | hai | abc".into();
        let r = spamngl(c);
        assert!(text_of(&r).contains("JUMLAH NYA HARUS ANGKA"));
    }

    #[test]
    fn api_key_kosong_mengembalikan_error() {
        let mut c = ctx();
        c.config = Arc::new(oktz_plugin_api::config::Runtime::default());
        let r = spamngl(c);
        assert!(text_of(&r).contains("API Key belum diset"));
    }

    #[test]
    fn sukses_mengirim_spam() {
        let http = Stub::new();
        let mut c = ctx();
        c.http = http.clone();
        let r = spamngl(c);
        assert!(text_of(&r).contains("✅ *DONE*"));
        assert!(text_of(&r).contains("https://ngl.link/xxxx"));
        assert!(text_of(&r).contains("(3x)"));
        assert_eq!(*http.call_count.lock().unwrap(), 3);
    }

    #[test]
    fn jumlah_nol_tidak_mengirim_request() {
        let http = Stub::new();
        let mut c = ctx();
        c.http = http.clone();
        c.text = "https://ngl.link/xxxx | hai | 0".into();
        let r = spamngl(c);
        assert!(text_of(&r).contains("✅ *DONE*"));
        assert_eq!(*http.call_count.lock().unwrap(), 0);
    }
}
