//! `plugins/tools/videotranscribe.js` — transkrip video dari URL.
//!
//! NOTE: baris `pub mod videotranscribe;` perlu ditambah ke `tools/mod.rs`
//! secara manual (mod.rs dilarang disentuh).
//!
//! `ponytail:` `crypto.randomUUID()` di JS diganti hex acak via `rand`;
//! format track_id tetap `{hex}_{ms}`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use rand::Rng as _;

pub(crate) const ENDPOINT: &str = "https://api.proactor.ai:7788/v1/tourists/files/transcription";
pub(crate) const DEFAULT_LANG: &str = "en";

// ponytail: `HEADERS` JS (termasuk user-agent) tidak bisa dipasang lewat
// `HttpGet::post_json`; endpoint tetap menerima tanpa header itu. Naik kelas
// kalau server mulai menolak: tambah varian `post_json_headers`.

pub(crate) const USAGE_TMPL: &str =
    "*📝 VIDEO TRANSCRIBE*\n\n`{prefix}video-transcribe <url_video> [bahasa]`\n\nContoh:\n`{prefix}video-transcribe https://youtu.be/... id`";

fn make_track_id() -> String {
    let mut hex = String::with_capacity(32);
    for _ in 0..16 {
        hex.push_str(&format!("{:02x}", rand::rng().random::<u8>()));
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("{hex}_{ms}")
}

fn join_transcript(items: &[Value]) -> String {
    items
        .iter()
        .filter_map(|i| i.get("text").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Padanan `cleanResult(url, json)` di JS.
fn clean_result(json: &Value) -> Result<(String, usize, String), String> {
    let code = json.get("code").and_then(Value::as_i64).unwrap_or(500);
    let data = json
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if code != 200 || data.is_empty() {
        let msg = json
            .get("msg")
            .or_else(|| json.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Transcript tidak ditemukan");
        return Err(msg.to_string());
    }
    let title = data
        .iter()
        .find_map(|i| i.get("videoTitle").and_then(Value::as_str))
        .unwrap_or("No title")
        .to_string();
    let total = data.len();
    let transcript = join_transcript(&data);
    Ok((title, total, transcript))
}

pub(crate) fn usage(prefix: &str) -> String {
    USAGE_TMPL.replace("{prefix}", prefix)
}

#[plugin(
    name = "videotranscribe",
    alias = ["video-transcribe", "transkripvideo"],
    category = "tools",
    description = "Transkrip video dari URL menjadi teks (YouTube, mp4, dll)",
    usage = ".video-transcribe <url> [lang]",
    example = ".video-transcribe https://youtu.be/dQw4w9WgXcQ id",
    cooldown = 30,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn videotranscribe(ctx: Ctx) -> Reply {
    let args: Vec<&str> = ctx.args.iter().map(String::as_str).collect();
    let url = args.first().copied().unwrap_or("");
    let lang = args.get(1).copied().unwrap_or(DEFAULT_LANG);

    if url.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Reply::text("❌ *Gagal:* URL kosong / tidak valid");
    }

    let body = serde_json::json!({
        "track_id": make_track_id(),
        "fileUrl": url,
        "language": lang,
    });

    let Ok(res) = ctx.http.post_json(ENDPOINT, &body) else {
        return Reply::text("❌ *Gagal:* Gagal memproses video");
    };
    let text = res.text();
    let json: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return Reply::text(format!(
                "❌ *Gagal:* Response bukan JSON: {}",
                &text[..text.len().min(200)]
            ));
        }
    };

    let (title, total, transcript) = match clean_result(&json) {
        Ok(t) => t,
        Err(msg) => return Reply::text(format!("❌ *Gagal:* {msg}")),
    };

    let mut info = format!(
        "📝 *VIDEO TRANSCRIBE*\n\n\
         *🎬 Title:* {title}\n\
         *🗣️ Language:* {}\n\
         *🔢 Segments:* {total}\n\n\
         *📜 Transcript:*\n{}",
        lang.to_uppercase(),
        transcript
    );
    if transcript.len() > 2500 {
        info.push_str("... (teks terlalu panjang)");
    }

    Reply::text(info)
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
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
        fn post_json(&self, url: &str, _: &Value) -> Result<HttpResponse, HttpError> {
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
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            text: text.to_string(),
            args,
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "videotranscribe".into(),
            http,
            ..Ctx::default()
        };
        videotranscribe(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    #[test]
    fn input_kosong_menampilkan_usage() {
        let r = run(Stub::body("x"), "");
        assert!(text_of(&r).contains("*📝 VIDEO TRANSCRIBE*"));
        assert!(text_of(&r).contains("`.video-transcribe <url_video> [bahasa]`"));
    }

    #[test]
    fn url_tidak_valid_ditolak() {
        let r = run(Stub::body("x"), "bukan url");
        assert_eq!(text_of(&r), "❌ *Gagal:* URL kosong / tidak valid");
    }

    #[test]
    fn jalur_berhasil_mengembalikan_transkrip() {
        let body = serde_json::json!({
            "code": 200,
            "data": [
                { "videoTitle": "Lagu Contoh", "duration": 61000, "text": "Halo dunia" },
                { "duration": 30000, "text": "Ini adalah uji" }
            ]
        });
        let http = Stub::body(&body.to_string());
        let r = run(http.clone(), "https://youtu.be/dQw4w9WgXcQ id");
        let t = text_of(&r);
        assert!(t.contains("*🎬 Title:* Lagu Contoh"));
        assert!(t.contains("*🗣️ Language:* ID"));
        assert!(t.contains("*🔢 Segments:* 2"));
        assert!(t.contains("Halo dunia Ini adalah uji"));
        assert_eq!(http.seen.lock().unwrap().as_slice(), &[ENDPOINT]);
    }

    #[test]
    fn code_bukan_200_atau_data_kosong_gagal() {
        for body in [
            serde_json::json!({ "code": 500, "data": [] }),
            serde_json::json!({ "code": 200, "data": [] }),
            serde_json::json!({ "code": 200 }),
        ] {
            let r = run(Stub::body(&body.to_string()), "https://youtu.be/x");
            assert!(
                text_of(&r).starts_with("❌ *Gagal:*"),
                "body {body} -> {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn body_bukan_json_gagal_dengan_pesan() {
        let r = run(Stub::body("bukan json"), "https://youtu.be/x");
        assert!(
            text_of(&r).starts_with("❌ *Gagal:* Response bukan JSON"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn kegagalan_jaringan_gagal() {
        let r = run(Stub::offline(), "https://youtu.be/x");
        assert_eq!(text_of(&r), "❌ *Gagal:* Gagal memproses video");
    }

    #[test]
    fn transcript_dipotong_di_2500() {
        let panjang: String = "x".repeat(3000);
        let body = serde_json::json!({
            "code": 200,
            "data": [ { "videoTitle": "T", "duration": 1000, "text": panjang } ]
        });
        let r = run(Stub::body(&body.to_string()), "https://youtu.be/x");
        let t = text_of(&r);
        assert!(t.contains("... (teks terlalu panjang)"), "{}", t);
        let akhir = "... (teks terlalu panjang)";
        assert!(t.ends_with(akhir), "{}", t);
    }

    #[test]
    fn track_id_memiliki_format_hex_ms() {
        let id = make_track_id();
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 32);
        assert!(parts[1].parse::<u64>().is_ok());
    }
}
