//! `plugins/download/alightmotion.js` — unduh project/preset Alight Motion.
//!
//! `ponytail:` JS mengirim `document: { url }` (sumber `Url`, bukan byte), dan
//! nama berkas memakai `config.bot.name`. Di `Ctx::default()` nama bot kosong,
//! jadi tes di bawah menghasilkan `AlightMotion_.zip`; produksi (lebih
//! `Runtime::from_config`) menghasilkan `AlightMotion_Oktz Bot.zip` seperti JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `alightmotion.js:23-28` — kartu pakai saat `text` kosong.
fn help(prefix: &str) -> String {
    format!(
        "📥 *ALIGHT MOTION DOWNLOADER*\n\n\
         Fitur ini membantumu mengunduh project atau preset dari Alight Motion dengan mudah!\n\n\
         *Cara Penggunaan:*\n\
         - Ketik *{prefix}amdl <link_alight_motion>*\n\n\
         *Contoh:* {prefix}amdl https://alight.link/xxxxx"
    )
}

/// `alightmotion.js:36` — endpoint + query `url`, di-encode `encodeURIComponent`.
fn api_url(target_url: &str) -> String {
    format!(
        "https://kyzznekoo.zone.id/api/alightmotion/download?url={}",
        encode_uri_component(target_url)
    )
}

/// `alightmotion.js:49` — `data.status` truthy, `data.data` ada, dan
/// `data.data.url` string tidak kosong. `||` JS membuat `0`/`""`/`null` gagal,
// jadi `as_str` yang kosong ikut ditolak.
fn download_url(body: &Value) -> Option<String> {
    let status_ok = body.get("status").is_some_and(crate::common::js_truthy_value);
    let d = body.get("data").filter(|v| status_ok && !v.is_null())?;
    d.get("url")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// `src/lib/error.js` + `config.errorTemplate` (`config.js:153`).
///
/// `pushName || 'User'` jadi `'User'` karena `m.pushName` tidak ada di `Ctx`;
// `prefix || '.'` dan `command || '?'` tetap dihormati.
fn error_template(prefix: &str, command: &str) -> String {
    let prefix = if prefix.is_empty() { "." } else { prefix };
    let command = if command.is_empty() { "?" } else { command };
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

#[plugin(
    name = "alightmotiondl",
    alias = ["alightmotion", "amdl"],
    category = "downloader",
    description = "Download project/preset Alight Motion",
    usage = ".amdl <link>",
    example = ".amdl https://alight.link/...",
    cooldown = 10,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn alightmotion(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🕕")` di `:31` terjadi sebelum pemeriksaan; `Reply` hanya mampu
    // membawa satu reaksi, jadi yang ikut ke balasan hanyalah reaksi penutup.
    let Ok(res) = ctx.http.get(&api_url(text)) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let Some(url) = download_url(&body) else {
        return Reply::text(
            "Maaf, link Alight Motion tersebut tidak valid atau project sudah dihapus.",
        )
        .with_reaction("❌");
    };

    let file_name = format!("AlightMotion_{}.zip", ctx.config().bot.name);
    Reply::document(
        MediaSource::url(url).with_mimetype("application/zip"),
        &file_name,
    )
    .with_caption("✅ Berhasil mengunduh preset Alight Motion!")
    .with_reaction("✅")
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
        fn body(v: Value) -> Arc<Stub> {
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
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "alightmotiondl".into(),
            http,
            ..Ctx::default()
        };
        alightmotion(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("alightmotiondl").unwrap();
        assert_eq!(p.meta.name, "alightmotiondl");
        assert_eq!(p.meta.alias, ["alightmotion", "amdl"]);
        assert_eq!(p.meta.category, "downloader");
        assert_eq!(p.meta.description, "Download project/preset Alight Motion");
        assert_eq!(p.meta.usage, ".amdl <link>");
        assert_eq!(p.meta.example, ".amdl https://alight.link/...");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve_to_this_plugin() {
        for a in ["alightmotiondl", "alightmotion", "amdl"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "alightmotiondl",
                "alias {a}"
            );
        }
    }

    #[test]
    fn tanpa_link_menampilkan_kartu_pakai_dan_tidak_beri_reaksi() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_di_encode_seperti_encode_uri_component_js() {
        assert_eq!(
            api_url("https://alight.link/abc"),
            "https://kyzznekoo.zone.id/api/alightmotion/download?url=https%3A%2F%2Falight.link%2Fabc"
        );
    }

    #[test]
    fn respons_tanpa_status_data_atau_url_menolak() {
        for body in [
            serde_json::json!({}),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": {} }),
            serde_json::json!({ "status": true, "data": { "url": "" } }),
            serde_json::json!({ "status": false, "data": { "url": "https://x.zip" } }),
            Value::Null,
        ] {
            let http = Stub::body(body.clone());
            let r = run(http.clone(), "https://alight.link/abc");
            assert_eq!(
                text_of(&r),
                "Maaf, link Alight Motion tersebut tidak valid atau project sudah dihapus.",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn jalur_berhasil_mengirim_dokumen_dengan_url_dan_nama_berkas() {
        let body = serde_json::json!({ "status": true, "data": { "url": "https://cdn/file.zip" } });
        let http = Stub::body(body.clone());
        let r = run(http.clone(), "https://alight.link/abc");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Document {
            media,
            file_name,
            caption,
            ..
        } = r.without_mutations().tanpa_reaksi()
        else {
            panic!("harus document, dapat {:?}", r.without_mutations());
        };
        assert_eq!(caption, "✅ Berhasil mengunduh preset Alight Motion!");
        assert_eq!(file_name, "AlightMotion_.zip");
        assert_eq!(
            media,
            &Some(MediaSource::Url {
                url: "https://cdn/file.zip".into(),
                mimetype: Some("application/zip".to_string()),
            })
        );
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://kyzznekoo.zone.id/api/alightmotion/download?url=https%3A%2F%2Falight.link%2Fabc"]
        );
    }

    #[test]
    fn nama_berkas_mengikuti_bot_name_dari_config() {
        let body = serde_json::json!({ "status": true, "data": { "url": "https://cdn/file.zip" } });
        let mut args = "https://alight.link/abc".split_whitespace();
        let ctx = Ctx {
            text: "https://alight.link/abc".into(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "alightmotiondl".into(),
            http: Stub::body(body),
            config: Arc::new(oktz_plugin_api::config::Runtime {
                bot: oktz_plugin_api::config::BotInfo {
                    name: "Oktz Bot".into(),
                    ..oktz_plugin_api::config::BotInfo::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        let r = alightmotion(ctx);
        // `m.react("✅")` membungkus balasan dokumen (`alightmotion.js:63`).
        let Reply::Document { file_name, .. } = r.without_mutations().tanpa_reaksi() else {
            panic!("harus document");
        };
        assert_eq!(file_name, "AlightMotion_Oktz Bot.zip");
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_pesan_error_dan_reaksi_sil() {
        let r = run(Stub::offline(), "https://alight.link/abc");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.alightmotiondl` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn error_template_mengikuti_ganti_prefix_dan_command() {
        // `prefix || '.'` / `command || '?'` di `src/lib/error.js:5-6`.
        assert_eq!(
            error_template("", ""),
            "☢ Kayaknya command `.?` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }
}
