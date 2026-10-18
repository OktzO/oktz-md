//! `plugins/tools/getpaste.js` — ambil konten dari Pastebin.
//!
//! NOTE: baris `pub mod getpaste;` perlu ditambah ke `tools/mod.rs` secara
//! manual (mod.rs dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

const BASE: &str = "https://pastebin.com/raw/";

pub(crate) const USAGE_TMPL: &str =
    "📋 *ɢᴇᴛ ᴘᴀsᴛᴇʙɪɴ*\n\n> Masukkan link Pastebin yang valid\n\n> Contoh: `{prefix}getpaste https://pastebin.com/Gu8RZaqv`";

/// Ekstrak ID paste dari link, padanan `_id` di JS:
/// `/pastebin\.com\/(?:raw\/)?([a-zA-Z0-9]+)/` — kalau tidak cocok, pakai
/// input apa adanya (JS melakukan hal yang sama).
pub(crate) fn paste_id(link: &str) -> String {
    let Some(start) = link.find("pastebin.com/") else {
        return link.to_string();
    };
    let after = &link[start + "pastebin.com/".len()..];
    let after = after.strip_prefix("raw/").unwrap_or(after);
    let id: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if id.is_empty() {
        link.to_string()
    } else {
        id
    }
}

pub(crate) fn usage(prefix: &str) -> String {
    USAGE_TMPL.replace("{prefix}", prefix)
}

#[plugin(
    name = "getpaste",
    alias = ["pastebin", "getpb"],
    category = "tools",
    description = "Ambil konten dari Pastebin",
    usage = ".getpaste <link pastebin>",
    example = ".getpaste https://pastebin.com/Gu8RZaqv",
    cooldown = 5,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn getpaste(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();

    if text.is_empty() || !text.contains("pastebin.com") {
        return Reply::text(usage(&ctx.prefix));
    }

    let id = paste_id(text);
    let url = format!("{BASE}{id}");

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.text();

    if body.is_empty() {
        return Reply::text("").with_reaction("✅");
    }

    Reply::text(body).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: &str) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(String::new()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
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
            text: text.to_string(),
            args: Vec::new(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "getpaste".into(),
            http,
            ..Ctx::default()
        };
        getpaste(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn input_kosong_atau_bukan_pastebin_menampilkan_usage() {
        for t in ["", "   ", "hello", "https://google.com"] {
            let r = run(Stub::body("isi"), t);
            assert_eq!(text_of(&r), usage("."), "input {t:?}");
            assert_eq!(reaction_of(&r), None, "input {t:?}");
        }
    }

    #[test]
    fn id_diekstrak_dari_link_lengkap_dan_raw() {
        assert_eq!(paste_id("https://pastebin.com/Gu8RZaqv"), "Gu8RZaqv");
        assert_eq!(
            paste_id("https://pastebin.com/raw/Gu8RZaqv"),
            "Gu8RZaqv"
        );
        assert_eq!(paste_id("https://pastebin.com/abc123 extra"), "abc123");
        assert_eq!(paste_id("tidak ada link"), "tidak ada link");
    }

    #[test]
    fn jalur_berhasil_mengembalikan_konten_dengan_reaksi_sukses() {
        let http = Stub::body("ini isi pastebin");
        let r = run(http.clone(), "https://pastebin.com/Gu8RZaqv");
        assert_eq!(text_of(&r), "ini isi pastebin");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://pastebin.com/raw/Gu8RZaqv"]
        );
    }

    #[test]
    fn body_kosong_masih_sukses_tanpa_teks() {
        let http = Stub::empty();
        let r = run(http.clone(), "https://pastebin.com/Gu8RZaqv");
        assert_eq!(text_of(&r), "");
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run(Stub::offline(), "https://pastebin.com/Gu8RZaqv");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.getpaste` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
