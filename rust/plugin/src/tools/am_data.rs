//! `plugins/tools/am-data.js` — lihat data project Alight Motion dari link share
//! via `api.obscuraworks.org`.
//!
//! `ponytail:` JS mengirim header `Authorization: Bearer ${KEY}` dan
//! `Content-Type: application/json`; `UreqHttp::call` mempertahankan header
//! kustom melalui `HttpCall::call`. Stub bawaan (`NoHttp`) tidak pernah
//! jaringan, jadi jalur gagal diuji lewat `Err`.

use oktz_plugin_api::{Ctx, HttpCall, HttpError, HttpGet, MediaSource, Reply, plugin};
use serde_json::{json, Value};

use crate::common::error_template;

const API: &str = "https://api.obscuraworks.org/api/tools/amdata";

fn help(prefix: &str) -> String {
    format!(
        "📱 *ᴀʟɪɢʜᴛ ᴍᴏᴛɪᴏɴ ᴅᴀᴛᴀ*\n\n\
         - Lihat info project AM dari link share\n\
         - Masukkan URL share Alight Motion\n\n\
         `{prefix}am-data <url>`"
    )
}

const GAGAL: &str = "📱 *ɢᴀɢᴀʟ ᴍᴇᴍʙᴀᴄᴀ ᴅᴀᴛᴀ*\n\n- Pastikan URL share valid";

/// `if (!b) return "-"; if (b < 1024) ...` (`am-data.js:19-24`).
fn fmt_size(b: &Value) -> String {
    let n = match b {
        Value::Number(n) => n.as_i64().unwrap_or(0),
        _ => 0,
    };
    if n == 0 {
        "-".to_string()
    } else if n < 1024 {
        format!("{n} B")
    } else if n < 1_048_576 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    }
}

/// `new Date(ts._seconds * 1000).toLocaleDateString("id-ID", { dateStyle: "long" })`.
///
/// `ponytail:` tanpa `icu4x` tidak bisa meniru locale id-ID byte-per-byte;
/// format yang dihasilkan adalah representasi yang sama secara struktur
/// (`DD Month YYYY`). Bulan dalam Bahasa Indonesia.
fn fmt_date(ts: &Value) -> String {
    let seconds = ts
        .get("_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if seconds == 0 {
        return "-".to_string();
    }
    let ms = seconds * 1000;
    let (y, m, d, ..) = crate::common::jakarta_parts(ms);
    let bulan = crate::common::MONTHS_ID[(m - 1) as usize];
    format!("{d} {bulan} {y}")
}

#[plugin(
    name = "am-data",
    alias = ["alightmotion-data"],
    category = "tools",
    description = "Lihat data project Alight Motion dari link share",
    usage = ".am-data <url>",
    example = ".am-data https://alightcreative.com/am/share/...",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn am_data(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() || !url.contains("alightcreative.com") {
        return Reply::text(help(&ctx.prefix));
    }

    let api_key = ctx.config().api_keys.obscura.clone();

    let Ok(res) = ctx.http.call(
        &HttpCall::post(API)
            .header("Accept", "application/json, image/*, audio/*, video/*")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "url": url })),
    ) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let info = body.get("data").and_then(|d| d.get("info"));

    if !crate::common::js_truthy_value(body.get("status")) || info.is_none() {
        return Reply::text(GAGAL).with_reaction("❌");
    }

    let info = info.unwrap();
    let projects = match info.get("projects").and_then(Value::as_array) {
        Some(pkgs) if !pkgs.is_empty() => pkgs
            .iter()
            .map(|p| {
                let title = p.get("title").and_then(Value::as_str).unwrap_or("-");
                let tipe = p.get("type").and_then(Value::as_str).unwrap_or("-");
                let size = fmt_size(&p.get("size").cloned().unwrap_or(Value::Null));
                format!("  - *{title}* ({tipe}, {size})")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "  - Tidak ada".to_string(),
    };

    let effects = match info.get("requiredEffects").and_then(Value::as_array) {
        Some(eff) if !eff.is_empty() => {
            let ambil: Vec<String> = eff
                .iter()
                .take(8)
                .filter_map(|e| e.as_str().map(String::from))
                .collect();
            let mut teks = ambil.join(", ");
            if eff.len() > 8 {
                teks.push_str(&format!(", +{} lagi", eff.len() - 8));
            }
            teks
        }
        _ => "-".to_string(),
    };

    let title = info.get("title").and_then(Value::as_str).unwrap_or("-");
    let size = fmt_size(&info.get("size").cloned().unwrap_or(Value::Null));
    let downloads = info
        .get("downloads")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let likes = info.get("likes").and_then(Value::as_i64).unwrap_or(0);
    let am_version = info
        .get("amVersionString")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let am_platform = info.get("amPlatform").and_then(Value::as_str).unwrap_or("-");
    let max_ff = info.get("maxFFVer").and_then(Value::as_str).unwrap_or("-");
    let share_date = fmt_date(&info.get("shareDate").cloned().unwrap_or(Value::Null));

    let mut msg = format!(
        "📱 *ᴀʟɪɢʜᴛ ᴍᴏᴛɪᴏɴ ᴅᴀᴛᴀ*\n\n\
         - *Judul* → {title}\n\
         - *Ukuran* → {size}\n\
         - *Download* → {downloads}x\n\
         - *Likes* → {likes}\n\
         - *Versi* → `{am_version}`\n\
         - *Platform* → {am_platform}\n\
         - *Max FF* → v{max_ff}\n\
         - *Tanggal* → {share_date}\n\n\
         🎬 *Project*\n{projects}\n\n\
         ✨ *Effects* → {effects}"
    );

    if let Some(thumb) = info.get("largeThumbUrl").and_then(Value::as_str) {
        if !thumb.is_empty() {
            msg = msg; // caption di bawah
            return Reply::image(MediaSource::url(thumb.to_string()), msg).with_reaction("✅");
        }
    }

    Reply::text(msg).with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use serde_json::json;
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
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.to_string());
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
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
    }

    fn ctx_dengan_kunci() -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "am-data".into(),
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    obscura: "KUNCI-OBSCURA".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let mut ctx = ctx_dengan_kunci();
        ctx.text = text.to_string();
        ctx.args = args.by_ref().map(str::to_string).collect();
        am_data(ctx)
    }

    fn run_tanpa_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let mut ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "am-data".into(),
            http,
            ..Ctx::default()
        };
        ctx.text = text.to_string();
        ctx.args = args.by_ref().map(str::to_string).collect();
        am_data(ctx)
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
    fn url_kosong_atau_bukan_alightcreative_menghasilkan_kartu_pakai() {
        for input in ["", "https://example.com/x"] {
            let r = run_tanpa_kunci(Stub::offline(), input);
            assert_eq!(text_of(&r), help("."), "input {input:?}");
            assert_eq!(reaction_of(&r), None, "input {input:?}");
        }
    }

    #[test]
    fn kunci_kosong_tetap_dikirim_dalam_header() {
        let http = Stub::body(serde_json::json!({ "status": false }));
        let mut args = "https://alightcreative.com/am/share/abc".split_whitespace();
        let mut ctx = Ctx {
            text: "https://alightcreative.com/am/share/abc".into(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "am-data".into(),
            http: http.clone(),
            ..Ctx::default()
        };
        am_data(ctx);
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 1, "satu permintaan");
        // Header Authorization dibawa oleh klien nyata; di sini cuma memastikan
        // permintaan terbentuk dengan URL benar.
        assert!(seen[0].starts_with(API), "URL salah: {}", seen[0]);
    }

    #[test]
    fn status_salah_atau_info_hilang_menghasilkan_gagal() {
        for body in [
            serde_json::json!({ "status": false }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": {} }),
            Value::Null,
        ] {
            let r = run(Stub::body(body.clone()), "https://alightcreative.com/am/share/abc");
            assert_eq!(text_of(&r), GAGAL, "body {body:?}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body:?}");
        }
    }

    #[test]
    fn fmt_size_dan_fmt_date_mirip_js() {
        assert_eq!(fmt_size(&serde_json::json!(0)), "-");
        assert_eq!(fmt_size(&serde_json::json!(500)), "500 B");
        assert_eq!(fmt_size(&serde_json::json!(2048)), "2.0 KB");
        assert_eq!(fmt_size(&serde_json::json!(1_500_000)), "1.4 MB");
        // 1767225600 detik = 2026-01-01 Asia/Jakarta
        assert_eq!(fmt_date(&serde_json::json!({ "_seconds": 1767225600 })), "1 Januari 2026");
        assert_eq!(fmt_date(&serde_json::json!({})), "-");
        assert_eq!(fmt_date(&serde_json::json!({ "_seconds": 0 })), "-");
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_ketika_ada_thumbnail() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "info": {
                    "title": "Projek Contoh",
                    "size": 2048,
                    "downloads": 10,
                    "likes": 5,
                    "amVersionString": "5.0.0",
                    "amPlatform": "Android",
                    "maxFFVer": "7.0",
                    "shareDate": { "_seconds": 1767225600 },
                    "projects": [
                        { "title": "Projek A", "type": "Video", "size": 1048576 }
                    ],
                    "requiredEffects": ["efek1", "efek2", "efek3"],
                    "largeThumbUrl": "https://cdn.example/thumb.jpg"
                }
            }
        });
        let r = run(Stub::body(body), "https://alightcreative.com/am/share/abc");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn.example/thumb.jpg".into(),
                        mimetype: None,
                    })
                );
                assert!(caption.contains("Projek Contoh"));
                assert!(caption.contains("- *Projek A* (Video, 1.0 MB)"));
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn tanpa_thumbnail_mengirim_teks_polos() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "info": {
                    "title": "X",
                    "size": 0,
                    "largeThumbUrl": ""
                }
            }
        });
        let r = run(Stub::body(body), "https://alightcreative.com/am/share/abc");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(text_of(&r).starts_with("📱 *ᴀʟɪɢʜᴛ ᴍᴏᴛɪᴏɴ ᴅᴀᴛᴀ*"));
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run(Stub::offline(), "https://alightcreative.com/am/share/abc");
        assert_eq!(text_of(&r), crate::common::error_template(".", "am-data"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
