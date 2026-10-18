//! `plugins/tools/sendngl.js` — kirim pesan NGL.
//!
//! `ponytail:` `sendNgl` di JS (`src/lib/stalker-fallback.js`) menjalankan
//! fallback cuki → nexray, dan reaksi `🎴` sebelum pemakaian dihapus karena
//! `Reply` hanya mampu satu reaksi (yang penutup). Semua jaringan lewat
//! `ctx.http`; tidak menyalin logika `stalker-fallback.js` ke sini, melainkan
//! meniru alurnya (cuki dulu, nexray cadangan) lewat dua permintaan HTTP.

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::{json, Value};

/// `sendngl.js:27-32` — pesan saat `link` hilang.
fn link_missing(prefix: &str) -> String {
    format!(
        "*LINK NGL NYA MANA ??*\n\
         Contoh: `{prefix}sendngl https://ngl.link/xxxx | hai`"
    )
}

/// `sendngl.js:30-32` — pesan saat `kata` hilang.
fn kata_missing(prefix: &str) -> String {
    format!(
        "*KATA KATA NYA MANA ??*\n\n\
         Contoh: `{prefix}sendngl https://ngl.link/xxxx | hai`"
    )
}

/// `src/lib/error.js` + `config.errorTemplate` (`config.js:153`).
///
/// `pushName || 'User'` jadi `'User'` karena `m.pushName` tidak ada di `Ctx`;
// `prefix || '.'` dan `command || '?'` tetap dihormati. Duplikasi sengaja
// (lihat catatan di `stalker/discordstalk.rs`).
fn error_template(prefix: &str, command: &str) -> String {
    let prefix = if prefix.is_empty() { "." } else { prefix };
    let command = if command.is_empty() { "?" } else { command };
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

/// `sendNgl(link, text, 1)` di `src/lib/stalker-fallback.js:315-354`.
///
/// Cuki: `POST https://api.cuki.biz.id/api/ephoto/ngl?apikey=<key>` dengan
// JSON `{ link, text }`; sukses kalau `res.data.success !== false`.
/// Nexray: `GET https://api.nexray.eu.cc/tools/spamngl?url=...&jumlah=1&pesan=...`;
/// sukses kalau `res.data.status` truthy.
fn send_ngl(
    http: &dyn HttpGet,
    cuki_key: &str,
    link: &str,
    kata: &str,
) -> Result<(), HttpError> {
    if !cuki_key.is_empty() {
        let url = format!(
            "https://api.cuki.biz.id/api/ephoto/ngl?apikey={}",
            crate::tools::phisingataubukan::encode_uri_component(cuki_key)
        );
        if let Ok(res) = http.post_json(&url, &serde_json::json!({ "link": link, "text": kata })) {
            let data = res.json().unwrap_or(Value::Null);
            if !data
                .get("success")
                .is_some_and(|v| v == &Value::Bool(false))
            {
                return Ok(());
            }
        }
    }

    let url = format!(
        "https://api.nexray.eu.cc/tools/spamngl?url={}&jumlah=1&pesan={}",
        crate::tools::phisingataubukan::encode_uri_component(link),
        crate::tools::phisingataubukan::encode_uri_component(kata),
    );
    let res = http.get(&url)?;
    let data = res.json().unwrap_or(Value::Null);
    if data.get("status").is_some_and(crate::common::js_truthy_value) {
        Ok(())
    } else {
        Err(oktz_plugin_api::HttpError {
            message: "semua provider gagal".into(),
        })
    }
}

#[plugin(
    name = "sendngl",
    alias = [],
    category = "tools",
    description = "Send NGL",
    usage = ".sendngl <url> | <text>",
    example = ".sendngl https://ngl.link/xxxx | hai",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn sendngl(ctx: Ctx) -> Reply {
    let mut parts = ctx.text.split('|');
    let link = parts.next().map(|s| s.trim());
    let kata = parts.next().map(|s| s.trim());

    let Some(link) = link.filter(|s| !s.is_empty()) else {
        return Reply::text(link_missing(&ctx.prefix));
    };
    let Some(kata) = kata.filter(|s| !s.is_empty()) else {
        return Reply::text(kata_missing(&ctx.prefix));
    };

    match send_ngl(&*ctx.http, &ctx.config().api_keys.cuki, link, kata) {
        Ok(()) => Reply::text(format!(
            "✅ *DONE*\n\nBerhasil mengirim pesan!\nTarget: {link}\nPesan: {kata}"
        ))
        .with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        cuki: Option<String>,
        nexray: Option<String>,
        seen: Mutex<Vec<(String, Option<String>)>>, // (url, body json atau None)
    }

    impl Stub {
        fn cuki_ok() -> Arc<Stub> {
            Arc::new(Stub {
                cuki: Some(serde_json::json!({ "success": true }).to_string()),
                ..Default::default()
            })
        }
        fn cuki_reject() -> Arc<Stub> {
            Arc::new(Stub {
                cuki: Some(serde_json::json!({ "success": false, "error": "nope" }).to_string()),
                ..Default::default()
            })
        }
        fn nexray_ok() -> Arc<Stub> {
            Arc::new(Stub {
                nexray: Some(serde_json::json!({ "status": true, "result": "ok" }).to_string()),
                ..Default::default()
            })
        }
        fn all_fail() -> Arc<Stub> {
            Arc::new(Stub::default())
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub::default())
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push((url.to_string(), None));
            match &self.nexray {
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
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.to_string(), Some(body.to_string())));
            match &self.cuki {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: Some("application/json".into()),
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma post_json dan get")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma post_json dan get")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "sendngl".into(),
            http,
            ..Ctx::default()
        };
        sendngl(ctx)
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
        let p = oktz_plugin_api::find("sendngl").unwrap();
        assert_eq!(p.meta.name, "sendngl");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Send NGL");
        assert_eq!(p.meta.usage, ".sendngl <url> | <text>");
        assert_eq!(p.meta.example, ".sendngl https://ngl.link/xxxx | hai");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_link_mengeluarkan_pesan_link_missing() {
        let http = Stub::cuki_ok();
        let r = run(http.clone(), "");
        assert_eq!(
            text_of(&r),
            "*LINK NGL NYA MANA ??*\nContoh: `.sendngl https://ngl.link/xxxx | hai`"
        );
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn tanpa_kata_mengeluarkan_pesan_kata_missing() {
        let http = Stub::cuki_ok();
        let r = run(http.clone(), "https://ngl.link/xxxx |");
        assert_eq!(
            text_of(&r),
            "*KATA KATA NYA MANA ??*\n\nContoh: `.sendngl https://ngl.link/xxxx | hai`"
        );
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn spasi_di_sekitar_pipa_dicabut() {
        let http = Stub::cuki_ok();
        let r = run(http.clone(), "  https://ngl.link/x   |   hai kamu  ");
        assert_eq!(
            text_of(&r),
            "✅ *DONE*\n\nBerhasil mengirim pesan!\nTarget: https://ngl.link/x\nPesan: hai kamu"
        );
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn cuki_berhasil_tanpa_mengirim_ke_nexray() {
        let http = Stub::cuki_ok();
        let r = run(http.clone(), "https://ngl.link/x | hai");
        assert_eq!(text_of(&r), "✅ *DONE*\n\nBerhasil mengirim pesan!\nTarget: https://ngl.link/x\nPesan: hai");
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].0,
            "https://api.cuki.biz.id/api/ephoto/ngl?apikey="
        );
        assert_eq!(seen[0].1.as_deref(), Some(r#"{"link":"https://ngl.link/x","text":"hai"}"#));
    }

    #[test]
    fn cuki_tanpa_kunci_menggunakan_nexray() {
        let http = Stub::nexray_ok();
        let r = run(http.clone(), "https://ngl.link/x | hai");
        assert_eq!(text_of(&r), "✅ *DONE*\n\nBerhasil mengirim pesan!\nTarget: https://ngl.link/x\nPesan: hai");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].0,
            "https://api.nexray.eu.cc/tools/spamngl?url=https%3A%2F%2Fngl.link%2Fx&jumlah=1&pesan=hai"
        );
        assert!(seen[0].1.is_none());
    }

    #[test]
    fn cuki_menolak_maka_nexray_digunakan() {
        let http = Arc::new(Stub {
            cuki: Some(serde_json::json!({ "success": false }).to_string()),
            nexray: Some(serde_json::json!({ "status": true }).to_string()),
            ..Default::default()
        });
        let r = run(http.clone(), "u | t");
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn keduanya_gagal_mengembalikan_pesan_error_dan_reaksi_sil() {
        let http = Stub::all_fail();
        let r = run(http.clone(), "https://ngl.link/x | hai");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.sendngl` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn error_template_mengikuti_ganti_prefix_dan_command() {
        assert_eq!(
            crate::common::error_template("", ""),
            "☢ Kayaknya command `.?` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }
}
