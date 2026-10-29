//! `plugins/canvas/fakecall.js` — gambar panggilan palsu WhatsApp (Android/iOS).
//!
//! `ponytail:` `m.react('🕕')` lalu reaksi `📞`/`❌`/`☢` — `Reply` hanya
//! menyimpan satu reaksi, jadi yang dipegang adalah yang terakhir (`📞` saat
//! berhasil, `❌` saat gagal foto profil/API, `☢` saat error template).
//! `m.download`/`m.quoted.download` → `ctx.download_media()` /
//! `download_quoted_media()`.
//! `sock.profilePictureUrl` → `ctx.sock().foto_profil(sender, "image")`.
//! `form-data` multipart → `HttpCall::post(...).multipart(...)`.
//! Header `form.getHeaders()` tidak ditiru (`HttpGet` tidak punya setter
//! header per-request).
//! `te(...)` → `crate::common::error_template(&ctx)`.

use oktz_plugin_api::{Ctx, HttpCall, HttpError, MediaSource, MultipartPart, Reply, plugin};

/// Balasan saat format tidak ada `|` atau teks kosong (`fakecall.js:25-39`).
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "Halo! Sepertinya format yang kamu masukkan kurang tepat.\n\n\
         Fitur ini digunakan untuk membuat gambar panggilan palsu (Fake Call) seolah-olah seseorang sedang meneleponmu secara langsung.\n\n\
         *Daftar Perintah yang Tersedia:*\n\
         - *{prefix}fakecall* (Untuk tampilan panggilan Android)\n\
         - *{prefix}fakecall-ios* (Untuk tampilan panggilan iPhone atau iOS)\n\n\
         *Cara Penggunaan:*\n\
         Ketik perintah diikuti dengan *Nama* dan *Durasi* yang dipisahkan oleh tanda pipa (|).\n\n\
         *Contoh Penggunaan:*\n\
         - *{prefix}fakecall Zann | 03:33:33*\n\
         - *{prefix}fakecall-ios Sayangku | 12:00:00*\n\n\
         *Tips Tambahan:*\n\
         Kamu bisa me-reply (membalas) sebuah gambar jika ingin menggunakan foto tersebut sebagai avatar profil sang penelepon!"
    )
}

/// `https://my.izuka-api.xyz/api/canvas/{endpoint}` (`fakecall.js:88`).
pub(crate) fn api_url(endpoint: &str) -> String {
    format!("https://my.izuka-api.xyz/api/canvas/{endpoint}")
}

/// Nama avatar part (`form.append('avatarUrl', ...)`) (`fakecall.js:79`).
const AVATAR_FILE: &str = "avatar.jpg";

/// Caption tidak ada (`fakecall.js` hanya mengirim image tanpa caption).
const CAPTION: &str = "";

#[plugin(
    name = "fakecall",
    alias = ["fakecall-android", "fakecall-ios"],
    category = "canvas",
    description = "Membuat gambar fake call WhatsApp (Tersedia versi Android dan iOS)",
    usage = ".fakecall nama | durasi",
    example = ".fakecall Zann | 19:00",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fakecall(ctx: Ctx) -> Reply {
    let cmd = ctx.command.to_lowercase();
    let text = ctx.text.trim();

    if !text.contains('|') {
        return Reply::text(bantuan(&ctx.prefix));
    }

    let mut parts = text.split('|').map(str::trim);
    let nama = parts.next().unwrap_or("");
    let durasi = parts.next().unwrap_or("");

    if nama.is_empty() {
        return Reply::text(
            "Maaf, nama penelepon tidak boleh dikosongkan. Silakan isi namanya terlebih dahulu!",
        );
    }
    if durasi.is_empty() {
        return Reply::text(
            "Maaf, durasi panggilan tidak boleh dikosongkan. Silakan isi durasinya terlebih dahulu!",
        );
    }

    // Coba ambil avatar: pesan pemicu (image) → quoted (image) → foto profil.
    let avatar = avatar_bytes(&ctx).or_else(|| {
        fallback_avatar(&*ctx.http).ok().filter(|b| !b.is_empty())
    });

    let endpoint = if cmd == "fakecall-ios" {
        "fakecall-ios"
    } else {
        "fakecall-android"
    };

    let Ok(res) = ctx.http.call(&HttpCall::post(api_url(endpoint)).multipart(vec![
        MultipartPart::file("avatarUrl", AVATAR_FILE, avatar.unwrap_or_default()),
        MultipartPart::field("name", nama),
        MultipartPart::field("duration", durasi),
    ])) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("❌");
    };

    Reply::image(
        MediaSource::bytes(res.body.bytes().to_vec()),
        CAPTION,
    )
    .with_reaction("📞")
}

/// Ambil byte avatar: pesan pemicu (image) → quoted (image) → foto profil.
fn avatar_bytes(ctx: &Ctx) -> Option<Vec<u8>> {
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

/// Fallback gambar default (`fakecall.js:73`).
fn fallback_avatar(http: &dyn oktz_plugin_api::HttpGet) -> Result<Vec<u8>, HttpError> {
    http.get("https://files.catbox.moe/nwvkbt.png")
        .map(|res| res.body.bytes().to_vec())
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse, MediaSource};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        panggilan: Mutex<Vec<(String, Vec<u8>)>>,
        balasan: Option<Vec<u8>>,
        gagal: bool,
    }

    impl Stub {
        fn body(bytes: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(bytes),
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

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma multipart")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma multipart")
        }
        fn post_multipart(
            &self,
            url: &str,
            parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            self.panggilan
                .lock()
                .unwrap()
                .push((url.to_string(), badan_multipart(parts)));
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        /// Plugin memakai `HttpCall::multipart` (`fakecall.js:78-93`); default
        /// `HttpGet::call` menolak multipart, jadi stub menimpanya dan mencatat
        /// `panggilan` hanya untuk request API (bukan avatar cadangan catbox).
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            let badan = match req.body.as_ref() {
                Some(HttpBody::Multipart(parts)) => badan_file(parts),
                _ => Vec::new(),
            };
            self.panggilan.lock().unwrap().push((req.url.clone(), badan));
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    /// Badan multipart termasuk `filename` part berkas — `form-data` JS
    /// (`form.append('avatarUrl', buffer, { filename: 'avatar.jpg' })`).
    fn badan_file(parts: &[MultipartPart]) -> Vec<u8> {
        let mut out = Vec::new();
        for p in parts {
            out.extend_from_slice(format!("{}=", p.name).as_bytes());
            if let Some(nama_berkas) = &p.file_name {
                out.extend_from_slice(nama_berkas.as_bytes());
                out.push(b' ');
            }
            out.extend_from_slice(p.body.bytes());
            out.push(b'\n');
        }
        out
    }

    fn badan_multipart(parts: &[(&str, HttpBody)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (nama, body) in parts {
            out.extend_from_slice(format!("{nama}=").as_bytes());
            out.extend_from_slice(body.bytes());
            out.push(b'\n');
        }
        out
    }

    fn run(http: Arc<Stub>, cmd: &str, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: cmd.into(),
            http,
            ..Ctx::default()
        };
        fakecall(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_pipa_menghasilkan_bantuan_tanpa_reaksi() {
        let http = Stub::offline();
        let r = run(http.clone(), "fakecall", "");
        assert_eq!(text_of(&r), bantuan("."));
        assert_eq!(reaksi(&r), None);
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn nama_kosong_ditolak() {
        let http = Stub::offline();
        let r = run(http.clone(), "fakecall", " | 03:33");
        assert_eq!(
            text_of(&r),
            "Maaf, nama penelepon tidak boleh dikosongkan. Silakan isi namanya terlebih dahulu!"
        );
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn durasi_kosong_ditolak() {
        let http = Stub::offline();
        let r = run(http.clone(), "fakecall", "Zann |");
        assert_eq!(
            text_of(&r),
            "Maaf, durasi panggilan tidak boleh dikosongkan. Silakan isi durasinya terlebih dahulu!"
        );
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn android_dan_ios_menggunakan_endpoint_beda() {
        assert_eq!(api_url("fakecall-android"), "https://my.izuka-api.xyz/api/canvas/fakecall-android");
        assert_eq!(
            api_url("fakecall-ios"),
            "https://my.izuka-api.xyz/api/canvas/fakecall-ios"
        );
    }

    #[test]
    fn multipart_mengirim_avatar_nama_dan_durasi() {
        let http = Stub::body(b"gambar-fake-call".to_vec());
        let r = run(http.clone(), "fakecall", "Zann | 03:33:33");
        assert_eq!(reaksi(&r), Some("📞"));
        let panggilan = http.panggilan.lock().unwrap().clone();
        assert_eq!(panggilan.len(), 1);
        assert_eq!(
            panggilan[0].0,
            "https://my.izuka-api.xyz/api/canvas/fakecall-android"
        );
        let badan = String::from_utf8_lossy(&panggilan[0].1);
        assert!(badan.contains("name=Zann"));
        assert!(badan.contains("duration=03:33:33"));
        assert!(badan.contains("avatar.jpg"));
    }

    #[test]
    fn ios_menggunakan_endpoint_ios() {
        let http = Stub::body(b"x".to_vec());
        let r = run(http.clone(), "fakecall-ios", "Zann | 12:00");
        assert_eq!(reaksi(&r), Some("📞"));
        let panggilan = http.panggilan.lock().unwrap().clone();
        assert_eq!(
            panggilan[0].0,
            "https://my.izuka-api.xyz/api/canvas/fakecall-ios"
        );
    }

    #[test]
    fn gagal_jaringan_menggunakan_error_template_dan_reaksi_sil() {
        let http = Stub::offline();
        let r = run(http.clone(), "fakecall", "Zann | 03:33");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "fakecall"));
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn berhasil_mengembalikan_gambar_dengan_bytes() {
        let http = Stub::body(b"PNG".to_vec());
        let r = run(http.clone(), "fakecall", "Zann | 03:33");
        assert_eq!(reaksi(&r), Some("📞"));
        assert!(r.has_media());
        match r.media_source() {
            Some(MediaSource::Bytes { data, .. }) => assert_eq!(data.as_ref(), b"PNG"),
            other => panic!("harus bytes, dapat {other:?}"),
        }
    }
}
