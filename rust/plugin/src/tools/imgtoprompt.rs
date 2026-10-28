//! `plugins/tools/imgtoprompt.js` — ubah gambar menjadi prompt AI.
//!
//! `ponytail:` scraping sebenarnya ada di crate `oktz_scraper` (modul
//! `img2prompt`), jadi plugin ini cuma panggil fungsi itu — tidak menyalin
//! logika scraper ke sini.
//! Unduh media: `m.download()`/`m.quoted.download()` →
//! `ctx.download_media()` / `download_quoted_media()`.
//! `Buffer.isBuffer` / validasi kosong → cek `bytes.len() > 0`.
//! `result.status === 'eror' || !result.prompt` → teks gagal.
//! `te(...)` → `crate::common::error_template(&ctx)`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use oktz_scraper::img2prompt::imgtoprompt as img2prompt;

/// Balasan saat bukan gambar (`imgtoprompt.js:29-31`).
pub(crate) const BUKAN_GAMBAR: &str =
    "❌ *ɢᴀᴍʙᴀʀ ᴅɪʙᴜᴛᴜʜᴋᴀɴ*\n\n> Reply atau kirim gambar dengan caption .imgtoprompt";

/// Balasan saat unduh gagal (`imgtoprompt.js:40-41`).
pub(crate) const GAGAL_UNDUH: &str = "❌ Gagal mengunduh gambar";

/// Balasan saat buffer tidak valid (`imgtoprompt.js:43-45`).
pub(crate) const BUFFER_TIDAK_VALID: &str = "❌ Buffer gambar tidak valid";

/// Balasan saat scraper gagal / tidak ada prompt (`imgtoprompt.js:57-58`).
pub(crate) fn gagal_generasi(msg: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ*\n\n> {msg}")
}

/// Balasan sukses (`imgtoprompt.js:60-62`).
pub(crate) fn sukses(prompt: &str, generated_at: &str) -> String {
    format!(
        "🎨 *ɪᴍᴀɢᴇ ᴛᴏ ᴘʀᴏᴍᴘᴛ*\n\n\
         ```{prompt}```\n\n\
         > _Generated at: {generated_at}_"
    )
}

/// `new Date().toISOString()` — UTC `YYYY-MM-DDTHH:mm:ss.sssZ`.
///
/// `jakarta_parts` mengembalikan waktu WIB; kurangi offset supaya yang
/// keluar komponen UTC, bukan WIB.
fn iso_now() -> String {
    let ms = crate::common::now_ms();
    let (y, m, d, h, mi, s) =
        crate::common::jakarta_parts(ms - crate::common::JAKARTA_OFFSET_SECS * 1000);
    format!(
        "{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{:03}Z",
        ms.rem_euclid(1000)
    )
}

#[plugin(
    name = "imgtoprompt",
    alias = ["img2prompt", "imagetoprompt", "i2p"],
    category = "tools",
    description = "Mengubah gambar menjadi prompt AI",
    usage = ".imgtoprompt (reply gambar)",
    example = ".imgtoprompt",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn imgtoprompt(ctx: Ctx) -> Reply {
    let is_image = ctx
        .media
        .as_ref()
        .is_some_and(|m| m.kind == oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted
            .as_ref()
            .is_some_and(|q| q.kind.is_image());

    if !is_image {
        return Reply::text(BUKAN_GAMBAR);
    }

    let bytes = gambar_bytes(&ctx);
    let Some(bytes) = bytes else {
        return Reply::text(GAGAL_UNDUH);
    };
    if bytes.is_empty() {
        return Reply::text(BUFFER_TIDAK_VALID);
    }

    let hasil = match img2prompt(ctx.http.as_ref(), &bytes) {
        Ok(v) => v,
        Err(e) => {
            return Reply::text(gagal_generasi(&e.message));
        }
    };

    let prompt = hasil.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        let msg = hasil
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Tidak dapat menghasilkan prompt dari gambar ini");
        return Reply::text(gagal_generasi(msg));
    }

    // `result.generatedAt || new Date().toISOString()` — string kosong ikut fallback.
    let generated_at = hasil
        .get("generatedAt")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(iso_now);
    Reply::text(sukses(prompt, &generated_at))
}

/// Unduh byte gambar: pesan pemicu (image) → quoted (image).
fn gambar_bytes(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.media.as_ref().is_some_and(|m| m.kind == oktz_plugin_api::IncomingKind::Image) {
        return ctx.download_media().ok();
    }
    if let Some(q) = &ctx.quoted
        && q.kind.is_image()
    {
        return ctx.download_quoted_media().ok();
    }
    None
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, MediaError, MediaFetch, MediaOrigin};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        panggilan: Mutex<Vec<String>>,
        balasan: Option<String>,
        gagal: bool,
    }

    impl Stub {
        fn body(balasan: &str) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(balasan.to_string()),
                ..Default::default()
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                gagal: true,
                ..Default::default()
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper lewat HttpCall::call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper kirim JSON")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper kirim JSON")
        }
        fn call(&self, _req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            self.panggilan.lock().unwrap().push("post".to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default().into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    /// `m.download()` / `m.quoted.download()` palsu; `origin` diabaikan
    /// seperti stub lainnya.
    #[derive(Debug)]
    struct MediaStub(Result<Vec<u8>, MediaError>);

    impl MediaFetch for MediaStub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            self.0.clone()
        }
    }

    fn ctx_dengan_media(
        http: Arc<Stub>,
        is_trigger: bool,
        unduh: Result<Vec<u8>, MediaError>,
    ) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "imgtoprompt".into(),
            http,
            media_fetch: Arc::new(MediaStub(unduh)),
            media: if is_trigger {
                Some(oktz_plugin_api::IncomingMedia {
                    kind: oktz_plugin_api::IncomingKind::Image,
                    mimetype: Some("image/webp".into()),
                    file_name: None,
                    caption: String::new(),
                })
            } else {
                None
            },
            quoted: if is_trigger {
                None
            } else {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: ME.into(),
                    chat: ME.into(),
                    text: String::new(),
                    kind: oktz_plugin_api::QuotedKind::Image,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
            timestamp: 0,
        })
            },
            ..Ctx::default()
        }
    }

    fn run_dengan_media(http: Arc<Stub>, is_trigger: bool) -> Reply {
        imgtoprompt(ctx_dengan_media(http, is_trigger, Ok(vec![1, 2, 3])))
    }

    /// Unduhan gagal (`m.download()` menolak) — default `NoMediaFetch` juga
    /// begitu, jadi ini menjalankan cabang `GAGAL_UNDUH` (`imgtoprompt.js:40-41`).
    fn run_unduh_gagal(http: Arc<Stub>) -> Reply {
        imgtoprompt(ctx_dengan_media(
            http,
            true,
            Err(MediaError {
                message: "gagal unduh".into(),
            }),
        ))
    }

    fn run_tanpa_media() -> Reply {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "imgtoprompt".into(),
            http: Stub::offline(),
            ..Ctx::default()
        };
        imgtoprompt(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn bukan_gambar_ditolak_tanpa_jaringan() {
        let r = run_tanpa_media();
        assert_eq!(text_of(&r), BUKAN_GAMBAR);
    }

    #[test]
    fn unduh_gagal_hasilkan_teks_gagal() {
        let http = Stub::offline();
        let r = run_unduh_gagal(http);
        assert_eq!(text_of(&r), GAGAL_UNDUH);
    }

    #[test]
    fn sukses_mengembalikan_prompt_dan_generated_at() {
        let http = Stub::body(r#"{"prompt":"a cat","generatedAt":"2024-01-01"}"#);
        let r = run_dengan_media(http, true);
        let t = text_of(&r);
        assert!(t.contains("🎨 *ɪᴍᴀɢᴇ ᴛᴏ ᴘʀᴏᴍᴘᴛ*"));
        assert!(t.contains("```a cat```"));
        assert!(t.contains("Generated at: 2024-01-01"));
    }

    #[test]
    fn prompt_kosong_hasilkan_teks_gagal() {
        let http = Stub::body(r#"{}"#);
        let r = run_dengan_media(http, true);
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɢᴀɢᴀʟ*"));
        assert!(t.contains("Tidak dapat menghasilkan prompt dari gambar ini"));
    }

    #[test]
    fn body_bukan_json_hasilkan_objek_kosong_dan_gagal() {
        let http = Stub::body("<html>halo</html>");
        let r = run_dengan_media(http, true);
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɢᴀɢᴀʟ*"));
    }

    #[test]
    fn generated_at_kosong_pakai_waktu_sekarang() {
        let http = Stub::body(r#"{"prompt":"x","generatedAt":""}"#);
        let r = run_dengan_media(http, true);
        let t = text_of(&r);
        assert!(t.contains("```x```"));
        assert!(!t.contains("Generated at: _"));
    }

    #[test]
    fn quoted_gambar_bisa_dipakai() {
        let http = Stub::body(r#"{"prompt":"x","generatedAt":"t"}"#);
        let r = run_dengan_media(http, false);
        assert!(text_of(&r).contains("```x```"));
    }
}
