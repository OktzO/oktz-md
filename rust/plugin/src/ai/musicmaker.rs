//! `plugins/ai/musicmaker.js` — membuat musik/lagu AI dari teks (prompt).
//!
//! `ponytail:` JS memanggil `sock.sendMessage` dua kali (audio lalu image) +
//! `m.react("🕕")` lalu `m.react("✅")`. `Reply` hanya menampung satu reaksi,
//! jadi yang dipegang adalah yang terakhir (`✅`), dan kedua pesan jadi satu
//! `Sequence` (audio dulu, image dengan caption diikuti).
//! `example` diisi persis `pluginConfig` JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

/// Endpoint `https://api.nexray.eu.cc/ai/suno?prompt=...` (`musicmaker.js:29`).
pub(crate) fn api_url(prompt: &str) -> String {
    format!(
        "https://api.nexray.eu.cc/ai/suno?prompt={}",
        crate::tools::phisingataubukan::encode_uri_component(prompt)
    )
}

/// Teks balasan saat prompt kosong (`musicmaker.js:22-23`).
pub(crate) fn help() -> String {
    "❌ Masukkan deskripsi lagu yang ingin dibuat.\n\nContoh: `.musicmaker Lagu pop romantis yang ceria`".to_string()
}

/// Teks gagal AI (`musicmaker.js:41`).
pub(crate) const AI_GAGAL: &str =
    "⚠️ AI gagal membuat lagu. Coba gunakan prompt (deskripsi) yang lain.";

/// Teks error jaringan (`musicmaker.js:68`).
pub(crate) const ERROR_SERVER: &str =
    "😔 Terjadi kesalahan saat memproses permintaan pembuatan lagu ke AI. Server AI mungkin sibuk.";

/// Caption image hasil (`musicmaker.js:46-50`).
pub(crate) fn caption(r: &Value) -> String {
    format!(
        "🎵 *MUSIC MAKER AI* 🎵\n\n\
         *Judul:* {}\n\
         *Tags:* {}\n\
         *Durasi:* {} detik\n\n\
         *Lirik:*\n{}",
        js_str(&r["title"]),
        js_str(&r["tags"]),
        js_str(&r["duration"]),
        js_str(&r["lyrics"])
    )
}

/// `String(v)` untuk nilai JSON yang masuk ke template literal (mirip
/// `ytstalk.js_str` — objek/array jadi `[object Object]`).
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

#[plugin(
    name = "musicmaker",
    alias = ["bikinlagu", "suno"],
    category = "ai",
    description = "Membuat musik atau lagu menggunakan AI dari teks (prompt)",
    usage = ".musicmaker <prompt>",
    example = ".musicmaker Lagu sedih tentang perpisahan dengan musik piano",
    cooldown = 30,
    energi = 3,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn musicmaker(ctx: Ctx) -> Reply {
    let prompt = if !ctx.text.is_empty() {
        ctx.text.trim().to_string()
    } else {
        ctx.args.join(" ")
    };
    if prompt.is_empty() {
        return Reply::text(help());
    }

    let Ok(res) = ctx.http.get(&api_url(&prompt)) else {
        return Reply::text(ERROR_SERVER).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);
    let status_ok = data.get("status").is_some_and(crate::common::js_truthy_value);
    // JS `!data.status || !data.result` (`musicmaker.js:39`): `result` harus
    // truthy juga — `result: null` di JS masuk cabang gagal, bukan jadi caption "null".
    let Some(r) = data
        .get("result")
        .filter(|v| status_ok && crate::common::js_truthy_value(v))
    else {
        return Reply::text(AI_GAGAL).with_reaction("❌");
    };

    let audio = Reply::audio(MediaSource::url(js_str(&r["url"])));
    let image = Reply::image(MediaSource::url(js_str(&r["thumbnail"])), caption(r));
    Reply::sequence(vec![audio, image]).with_reaction("✅")
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
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    /// Helper generik: `Ctx::http` bertipe `Arc<dyn HttpGet>`, jadi stub apa
    /// pun (mis. `Teks` untuk body bukan JSON) bisa dipakai di sini.
    fn run_dengan_kunci<H: HttpGet + 'static>(http: Arc<H>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "musicmaker".into(),
            http,
            ..Ctx::default()
        };
        musicmaker(ctx)
    }

    fn contoh() -> Value {
        serde_json::json!({
            "status": true,
            "result": {
                "title": "Lagu Sedih",
                "tags": "piano, sedih",
                "duration": 180,
                "lyrics": "Lirik lagu disini",
                "url": "https://cdn/audio.mp3",
                "thumbnail": "https://cdn/thumb.jpg"
            }
        })
    }

    fn urutan_media(r: &Reply) -> Vec<(String, Option<String>)> {
        // JS `m.react("✅")` terakhir (`musicmaker.js:63`) membungkus balasan
        // jadi `Reaction`; lepas dulu supaya isi Sequence-nya bisa diperiksa.
        match r.without_mutations().tanpa_reaksi() {
            Reply::Sequence(list) => list
                .iter()
                .filter_map(|e| match e {
                    Reply::Audio { media, .. } => {
                        Some(("audio".to_string(), media.as_ref().and_then(|m| match m {
                            MediaSource::Url { url, .. } => Some(url.clone()),
                            _ => None,
                        })))
                    }
                    Reply::Image { media, .. } => Some((
                        "image".to_string(),
                        media.as_ref().and_then(|m| match m {
                            MediaSource::Url { url, .. } => Some(url.clone()),
                            _ => None,
                        }),
                    )),
                    _ => None,
                })
                .collect(),
            other => {
                panic!("harus Sequence, dapat {other:?}");
            }
        }
    }

    #[test]
    fn prompt_kosong_menghasilkan_help_tanpa_reaksi() {
        let r = run_dengan_kunci(Stub::offline(), "");
        assert_eq!(r.without_mutations().text_body().unwrap(), help());
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn args_dipakai_saat_text_kosong() {
        let r = run_dengan_kunci(Stub::body(contoh()), "lagu sedih");
        let urutan = urutan_media(&r);
        assert_eq!(urutan.len(), 2);
        assert_eq!(urutan[0].0, "audio");
        assert_eq!(urutan[0].1.as_deref(), Some("https://cdn/audio.mp3"));
        assert_eq!(urutan[1].0, "image");
        assert_eq!(urutan[1].1.as_deref(), Some("https://cdn/thumb.jpg"));
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn text_menangani_daripada_args() {
        let r = run_dengan_kunci(Stub::body(contoh()), "judul lain");
        let urutan = urutan_media(&r);
        assert_eq!(urutan[0].0, "audio");
        assert_eq!(urutan[1].1.as_deref(), Some("https://cdn/thumb.jpg"));
    }

    #[test]
    fn url_membawa_prompt_ter_encode() {
        assert_eq!(
            api_url("lagu pianoku"),
            "https://api.nexray.eu.cc/ai/suno?prompt=lagu%20pianoku"
        );
    }

    #[test]
    fn status_false_atau_result_kosong_mengasilkan_ai_gagal() {
        for body in [
            serde_json::json!({ "status": false, "message": "batas" }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "result": null }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "x");
            assert_eq!(
                r.without_mutations().text_body().unwrap(),
                AI_GAGAL,
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_mengembalikan_error_server() {
        let r = run_dengan_kunci(Stub::offline(), "x");
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            ERROR_SERVER
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn body_bukan_json_ditangani() {
        let http = Stub::offline();
        let _ = http;
        // `offline` selalu error get; untuk body bukan JSON gunakan balasan string.
        #[derive(Debug)]
        struct Teks(String);
        impl HttpGet for Teks {
            fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
                Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(self.0.as_bytes()),
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
            fn post_multipart(
                &self,
                _: &str,
                _: &[(&str, oktz_plugin_api::HttpBody)],
            ) -> Result<HttpResponse, HttpError> {
                unimplemented!()
            }
        }
        let r = run_dengan_kunci(Arc::new(Teks("bukan json".into())), "x");
        assert_eq!(r.without_mutations().text_body().unwrap(), AI_GAGAL);
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r.without_mutations();
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji.as_str()),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }
}
