//! `plugins/ai/tofigurev2.js` — ubah gambar ke style Figure v2.
//!
//! Alur JS: unduh media (quoted dulu, baru trigger) → `uploadImage` →
//! `applyImageEffect('tofigurav3')` → kirim gambar. Nama efeknya `'tofigurav3'`,
//! beda dari nama plugin — kuirk JS yang ditiru apa adanya
//! (`tofigurev2.js:44`).
//!
//! `uploadImage` (`src/lib/uploader.js` + `src/lib/upload-providers.js`)
//! disalin ke berkas ini; lihat blok SALINAN.
//!
//! Catatan: baris `pub mod tofigurev2;` perlu ditambah ke `ai/mod.rs` secara
//! manual (mod.rs sudah memuat plugin lain, dilarang disentuh).

use oktz_plugin_api::{
    Ctx, HttpCall, HttpGet, HttpResponse, IncomingKind, MediaSource, MultipartPart, Reply, plugin,
};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

// ===========================================================================
// SALINAN `src/lib/uploader.js` + `src/lib/upload-providers.js`
//
// Aturan port-brief: helper bersama belum ada di crate, jadi blok ini disalin
// identik ke empat berkas ai (toemotebatu, tofigurev2, toghibli, tomekah).
// Angkat ke `crate::common` kalau plugin lain turut memakainya.
//
// Tidak bisa disalin: `withTimeout(..., 20000)` per provider — mesin sinkron
// ini tidak punya timer. `UreqHttp` sudah memasang timeout global 20 detik
// (`ureq_http.rs:24`), kebetulan sama dengan `PROVIDER_TIMEOUT_MS` JS.
// ===========================================================================

/// `upload-providers.js:16-17` — UA browser untuk host yang menolaknya.
const UA_UPLOAD: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// Nama berkas + content-type: JS selalu memanggil
/// `uploadImage(buffer, 'image.jpg')`, dan `mime.lookup('image.jpg')` =
/// `image/jpeg` (`upload-providers.js:19`).
const NAMA_BERKAS: &str = "image.jpg";
const MIME_BERKAS: &str = "image/jpeg";

/// `res.ok` — padanan `if (!res.ok) throw`.
fn respons_ok(res: &HttpResponse) -> bool {
    (200..300).contains(&res.status)
}

/// `typeof url !== 'string' || !url` (`uploader.js:30`).
fn url_dari(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `litterbox` (`upload-providers.js:21-37`).
fn litterbox(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    let res = http
        .call(
            &HttpCall::post("https://litterbox.catbox.moe/resources/internals/api.php").multipart(
                vec![
                    MultipartPart::field("reqtype", "fileupload"),
                    MultipartPart::field("time", "72h"),
                    MultipartPart::file("fileToUpload", NAMA_BERKAS, buffer.to_vec())
                        .with_mimetype(MIME_BERKAS),
                ],
            ),
        )
        .map_err(|e| e.message)?;
    if !respons_ok(&res) {
        return Err("Litterbox gagal".into());
    }
    let url = res.text().trim().to_string();
    if !url.starts_with("http") {
        return Err("Invalid response".into());
    }
    Ok(url)
}

/// `kappa` (`upload-providers.js:39-59`).
fn kappa(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    let res = http
        .call(
            &HttpCall::post("https://kappa.lol/api/upload")
                .header("User-Agent", UA_UPLOAD)
                .header("Accept", "*/*")
                .header("Origin", "https://kappa.lol")
                .header("Referer", "https://kappa.lol/")
                .multipart(vec![
                    MultipartPart::file("file", NAMA_BERKAS, buffer.to_vec())
                        .with_mimetype(MIME_BERKAS),
                ]),
        )
        .map_err(|e| e.message)?;
    if !respons_ok(&res) {
        return Err("Kappa gagal".into());
    }
    let data = res.json().unwrap_or(Value::Null);
    url_dari(data.get("link")).ok_or_else(|| "Invalid response".into())
}

/// `uguu` (`upload-providers.js:61-82`).
fn uguu(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    let res = http
        .call(
            &HttpCall::post("https://uguu.se/upload.php")
                .header("accept", "*/*")
                .header("origin", "https://uguu.se")
                .header("referer", "https://uguu.se/")
                .header("user-agent", UA_UPLOAD)
                .multipart(vec![
                    MultipartPart::file("files[]", NAMA_BERKAS, buffer.to_vec())
                        .with_mimetype(MIME_BERKAS),
                ]),
        )
        .map_err(|e| e.message)?;
    if !respons_ok(&res) {
        return Err("Uguu gagal".into());
    }
    let data = res.json().unwrap_or(Value::Null);
    let url = data
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| files.first())
        .and_then(|f| f.get("url"));
    if !crate::common::js_truthy(data.get("success")) {
        return Err("Invalid response".into());
    }
    url_dari(url).ok_or_else(|| "Invalid response".into())
}

/// `nekohime` (`upload-providers.js:84-101`).
fn nekohime(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    let res = http
        .call(
            &HttpCall::post("https://cdn.nekohime.site/upload").multipart(vec![
                MultipartPart::file("file", NAMA_BERKAS, buffer.to_vec())
                    .with_mimetype(MIME_BERKAS),
            ]),
        )
        .map_err(|e| e.message)?;
    if !respons_ok(&res) {
        return Err("Nekohime gagal".into());
    }
    let data = res.json().unwrap_or(Value::Null);
    // `Array.isArray(f) ? f[0]?.url : f?.url` (`upload-providers.js:97-98`).
    let url = match data.get("files") {
        Some(Value::Array(list)) => list.first().and_then(|f| f.get("url")),
        Some(f) => f.get("url"),
        None => None,
    };
    url_dari(url).ok_or_else(|| "Nekohime gagal".into())
}

/// `\s*` — jumlah byte spasi di awal teks (dipakai scanner regex di bawah).
fn lewati_spasi(t: &str) -> usize {
    t.len() - t.trim_start().len()
}

/// Regex `Download link:\s*<a href=([^>\s]+)>` (case-insensitive,
/// `upload-providers.js:124`).
fn tautan_leopard(html: &str) -> Option<String> {
    let turun = html.to_ascii_lowercase();
    let mut dari = 0;
    while let Some(rel) = turun[dari..].find("download link:") {
        let mut i = dari + rel + "download link:".len();
        dari = i;
        i += lewati_spasi(&turun[i..]);
        if !turun[i..].starts_with("<a href=") {
            continue;
        }
        let sisa = &html[i + "<a href=".len()..];
        let akhir = sisa.find(|c: char| c == '>' || c.is_whitespace());
        if let Some(akhir) = akhir.filter(|n| *n > 0) {
            return Some(sisa[..akhir].to_string());
        }
    }
    None
}

/// `leopard` (`upload-providers.js:103-127`).
fn leopard(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    const HALAMAN: &str = "https://leopard.hosting.pecon.us/upload.php";
    // Pemanasan: status tidak diperiksa, tapi kegagalan jaringan tetap
    // menggagalkan provider ini (`await fetch(...)` tanpa catch di JS).
    // `terima_semua_status` = `fetch` JS yang tidak melempar untuk 4xx/5xx.
    http.call(
        &HttpCall::get(HALAMAN)
            .header("User-Agent", UA_UPLOAD)
            .terima_semua_status(),
    )
    .map_err(|e| e.message)?;
    let html = http
        .call(
            &HttpCall::post(HALAMAN)
                .header("User-Agent", UA_UPLOAD)
                .header("Origin", "https://leopard.hosting.pecon.us")
                .header("Referer", HALAMAN)
                .multipart(vec![
                    MultipartPart::file("uploadContent", NAMA_BERKAS, buffer.to_vec())
                        .with_mimetype(MIME_BERKAS),
                    MultipartPart::field("password", ""),
                    MultipartPart::field("showname", "yes"),
                ])
                .terima_semua_status(),
        )
        .map_err(|e| e.message)?
        .text();
    tautan_leopard(&html).ok_or_else(|| "Leopard gagal".into())
}

/// `/startUpload\("([^"]+)"/` (`upload-providers.js:142`).
fn start_upload(body: &str) -> Option<String> {
    let mut dari = 0;
    while let Some(rel) = body[dari..].find("startUpload(\"") {
        let mulai = dari + rel + "startUpload(\"".len();
        dari = mulai;
        let sisa = &body[mulai..];
        if let Some(akhir) = sisa.find('"').filter(|n| *n > 0) {
            return Some(sisa[..akhir].to_string());
        }
    }
    None
}

/// Regex `id=["']file_src["'][^>]*value=["']([^"']+)["']` (case-insensitive,
/// `upload-providers.js:172`) versi scanner.
fn file_src_upload_ee(html: &str) -> Option<String> {
    let turun = html.to_ascii_lowercase();
    let mut dari = 0;
    while let Some(rel) = turun[dari..].find("id=") {
        let mulai = dari + rel + 3;
        dari = mulai;
        let sisa = &turun[mulai..];
        let Some(sisa) = sisa.strip_prefix('"').or_else(|| sisa.strip_prefix('\'')) else {
            continue;
        };
        let Some(sisa) = sisa.strip_prefix("file_src") else {
            continue;
        };
        if !sisa.starts_with('"') && !sisa.starts_with('\'') {
            continue;
        }
        // `[^>]*` — `value=` yang dicari terbatas sebelum `>` pertama.
        let setelah = mulai + 1 + "file_src".len() + 1;
        let batas = turun[setelah..]
            .find('>')
            .map_or(turun.len(), |p| setelah + p);
        let area = &turun[setelah..batas];
        let mut cari = 0;
        while let Some(rel2) = area[cari..].find("value=") {
            let vk = cari + rel2 + "value=".len();
            let sisa_kutip = &html[setelah + vk..];
            let isi = if sisa_kutip.starts_with('"') || sisa_kutip.starts_with('\'') {
                &sisa_kutip[1..]
            } else {
                cari = vk;
                continue;
            };
            if let Some(akhir) = isi.find(['"', '\'']).filter(|n| *n > 0) {
                return Some(isi[..akhir].to_string());
            }
            cari = vk;
        }
    }
    None
}

/// Regex `View file:\s*<br\s*\/?>\s*<a href=["']?([^"'>\s]+)["']?`
/// (case-insensitive, `upload-providers.js:173`) versi scanner.
fn view_file_upload_ee(html: &str) -> Option<String> {
    let turun = html.to_ascii_lowercase();
    let mut dari = 0;
    while let Some(rel) = turun[dari..].find("view file:") {
        let mut i = dari + rel + "view file:".len();
        dari = i;
        i += lewati_spasi(&turun[i..]);
        if !turun[i..].starts_with("<br") {
            continue;
        }
        i += 3;
        i += lewati_spasi(&turun[i..]);
        if turun[i..].starts_with('/') {
            i += 1;
        }
        if !turun[i..].starts_with('>') {
            continue;
        }
        i += 1;
        i += lewati_spasi(&turun[i..]);
        if !turun[i..].starts_with("<a href=") {
            continue;
        }
        i += "<a href=".len();
        let sisa = &html[i..];
        let sisa = if sisa.starts_with('"') || sisa.starts_with('\'') {
            &sisa[1..]
        } else {
            sisa
        };
        let akhir = sisa.find(|c: char| c == '"' || c == '\'' || c == '>' || c.is_whitespace());
        if let Some(akhir) = akhir.filter(|n| *n > 0) {
            return Some(sisa[..akhir].to_string());
        }
    }
    None
}

/// `uploadEe` (`upload-providers.js:129-179`).
fn upload_ee(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    // `ext` (`upload-providers.js:130-133`) ditulis apa adanya, walau dengan
    // nama berkas tetap "image.jpg" cabang `cat_file` tidak pernah tercapai.
    let ext = NAMA_BERKAS
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let is_image = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg", "avif"].contains(&ext.as_str());

    // Pemanasan GET — JS tidak memeriksa status, hanya gagal jaringan.
    // `terima_semua_status` = `fetch` JS yang tidak melempar untuk 4xx/5xx.
    http.call(
        &HttpCall::get("https://www.upload.ee/?")
            .header("User-Agent", UA_UPLOAD)
            .terima_semua_status(),
    )
    .map_err(|e| e.message)?;
    let id_body = http
        .call(
            &HttpCall::get(format!(
                "https://www.upload.ee/ubr_link_upload.php?rnd_id={}",
                crate::common::now_ms()
            ))
            .header("User-Agent", UA_UPLOAD)
            .header("Referer", "https://www.upload.ee/?")
            .terima_semua_status(),
        )
        .map_err(|e| e.message)?
        .text();
    let upload_id = start_upload(&id_body).ok_or("Upload ID tidak ditemukan")?;

    let upload_url = format!(
        "https://www.upload.ee/cgi-bin/ubr_upload.pl?X-Progress-ID={upload_id}&upload_id={upload_id}"
    );
    http.call(
        &HttpCall::post(&upload_url)
            .header("User-Agent", UA_UPLOAD)
            .header("Origin", "https://www.upload.ee")
            .header("Referer", "https://www.upload.ee/?")
            .multipart(vec![
                MultipartPart::file("upfile_0", NAMA_BERKAS, buffer.to_vec())
                    .with_mimetype(MIME_BERKAS),
                MultipartPart::field("link", ""),
                MultipartPart::field("email", ""),
                MultipartPart::field(
                    "category",
                    if is_image { "cat_picture" } else { "cat_file" },
                ),
                MultipartPart::field("big_resize", "none"),
                MultipartPart::field("small_resize", "120x90"),
            ])
            .terima_semua_status(),
    )
    .map_err(|e| e.message)?;

    let html = http
        .call(
            &HttpCall::get(format!(
                "https://www.upload.ee/?page=finished&upload_id={upload_id}"
            ))
            .header("User-Agent", UA_UPLOAD)
            .header("Referer", upload_url.as_str())
            .terima_semua_status(),
        )
        .map_err(|e| e.message)?
        .text();
    let raw = file_src_upload_ee(&html)
        .or_else(|| view_file_upload_ee(&html))
        .ok_or("Upload.ee gagal")?;

    // `replaceAll("&amp;", "&")` lalu `replaceAll("&quot;", "\"")`.
    let mut url = raw.replace("&amp;", "&").replace("&quot;", "\"");
    if is_image {
        // `String.replace("/files/", …)` JS mengganti kemunculan PERTAMA.
        url = url.replacen("/files/", "/image/", 1);
        if let Some(tanpa_html) = url.strip_suffix(".html") {
            url = tanpa_html.to_string();
        }
    }
    Ok(url)
}

/// `uploadImage(buffer, 'image.jpg')` (`uploader.js:14-40`) — provider
/// dicoba berurutan sampai ada yang mengembalikan URL.
fn upload_image(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    // Urutan `PROVIDERS` (`upload-providers.js:183-190`).
    // Alias lokal: pecah tipe kompleks pasangan nama + fn provider.
    type ProviderUnggah = (&'static str, fn(&dyn HttpGet, &[u8]) -> Result<String, String>);
    let providers: [ProviderUnggah; 6] = [
        ("Litterbox", litterbox),
        ("Kappa", kappa),
        ("Uguu", uguu),
        ("Nekohime", nekohime),
        ("Leopard", leopard),
        ("Upload.ee", upload_ee),
    ];
    let mut kegagalan: Vec<String> = Vec::new();
    for (nama, jalankan) in providers {
        match jalankan(http, buffer) {
            Ok(url) => return Ok(url),
            Err(pesan) => kegagalan.push(format!("{nama}: {pesan}")),
        }
    }
    Err(format!(
        "Upload gagal di semua host — {}",
        kegagalan.join(" | ")
    ))
}

/// UA request efek (`effect-api.js:20`).
const UA_EFEK: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// Signature PNG/JPEG/GIF/WEBP/RIFF/BMP (`effect-api.js:57-67`).
fn looks_like_image(buf: &[u8]) -> bool {
    if buf.len() < 12 {
        return false;
    }
    buf.starts_with(&[0x89, 0x50, 0x4e, 0x47])
        || buf.starts_with(&[0xff, 0xd8, 0xff])
        || buf.starts_with(b"GIF8")
        || buf.starts_with(b"RIFF")
        || buf.starts_with(b"BM")
}

/// `applyImageEffect(effect, imageUrl)` — daftar provider dari
/// `config.effectApi.providers` tidak ada di `oktz_core::config::Runtime`, jadi
/// hanya host fallback `faa` (`effect-api.js:26-28`) yang dipakai.
///
/// `ponytail:` `withTimeout(..., 45000)` dan loop provider
/// (`effect-api.js:106-120`) tidak bisa disalin tanpa timer + config; ekspos
/// `effectApi` di `Runtime` kalau host cadangan mau dipakai.
fn apply_image_effect(http: &dyn HttpGet, effect: &str, image_url: &str) -> Result<Vec<u8>, String> {
    let url = format!(
        "https://api-faa.my.id/faa/{}?url={}",
        encode_uri_component(effect),
        encode_uri_component(image_url)
    );
    let res = http
        .call(
            &HttpCall::get(&url)
                .header("user-agent", UA_EFEK)
                .header("accept", "image/*,*/*"),
        )
        .map_err(|e| e.message)?;
    if !respons_ok(&res) {
        return Err(format!("faa: HTTP {} (server error)", res.status));
    }
    let buf = res.body.bytes().to_vec();
    if !looks_like_image(&buf) {
        return Err("faa: host membalas HTML, bukan gambar".into());
    }
    Ok(buf)
}

/// Efek di `applyImageEffect` — `'tofigurav3'`, bukan nama plugin
/// (`tofigurev2.js:44`).
const EFEK: &str = "tofigurav3";

#[plugin(
    name = "tofigurev2",
    alias = ["figurev2", "figure2"],
    category = "ai",
    description = "Ubah gambar ke style Figure v2",
    usage = ".tofigurev2 (reply gambar)",
    example = ".tofigurev2",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tofigurev2(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind.is_image());

    if !is_image {
        // `tofigurev2.js:23-25` — kartu pakai memakai `${m.prefix}`.
        return Reply::text(format!(
            "🎭 *ꜰɪɢᴜʀᴇ sᴛʏʟᴇ ᴠ2*\n\n> Kirim/reply gambar untuk diubah ke style Figure\n\n`{prefix}tofigurev2`",
            prefix = ctx.prefix
        ));
    }

    // ponytail: reaksi 🕕 hilang (`tofigurev2.js:27`) — `Reply` hanya membawa
    // satu reaksi, jadi hanya reaksi penutup ❌/✅/☢ yang dikirim.

    // `tofigurev2.js:30-35` — quoted lebih dulu (`m.quoted.isMedia`), baru
    // pesan pemicu (`m.isMedia`).
    let unduhan = if ctx.quoted().is_some_and(|q| q.is_media()) {
        Some(ctx.download_quoted_media())
    } else if ctx.is_media() {
        Some(ctx.download_media())
    } else {
        None
    };
    // `tofigurev2.js:37-40` — `!buffer` → ❌. `Err` dari MediaFetch adalah
    // padanan `download()` melempar di JS (jalur catch ☢); preseden port repo
    // (toblack/tochibi/toisland) memetakannya ke cabang ❌ ini.
    let Some(Ok(buffer)) = unduhan else {
        return Reply::text("❌ Gagal mendownload gambar").with_reaction("❌");
    };

    // JS `catch` (`tofigurev2.js:52-55`) menangkap upload, efek, dan kirim:
    // semuanya jadi template error + reaksi ☢.
    let Ok(image_url) = upload_image(&*ctx.http, &buffer) else {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    };
    let Ok(hasil) = apply_image_effect(&*ctx.http, EFEK, &image_url) else {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    };
    Reply::image(MediaSource::bytes(hasil), String::new()).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpError, IncomingMedia, MediaError, MediaFetch, MediaOrigin, QuotedKind,
        QuotedMessage,
    };
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// JPEG 12 byte — lolos `looks_like_image`.
    const GAMBAR: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01";

    fn resp(status: u16, isi: impl AsRef<[u8]>) -> (u16, Vec<u8>) {
        (status, isi.as_ref().to_vec())
    }

    /// HTTP + MediaFetch sekaligus; balasan per `call` berurutan, habis
    /// berarti offline.
    #[derive(Debug)]
    struct Stub {
        balasan: Mutex<Vec<(u16, Vec<u8>)>>,
        hitung: Mutex<usize>,
        panggilan: Mutex<Vec<HttpCall>>,
        unduhan: Mutex<Vec<MediaOrigin>>,
        hasil_unduh: Result<Vec<u8>, MediaError>,
    }

    impl Stub {
        fn baru(balasan: Vec<(u16, Vec<u8>)>, hasil_unduh: Result<Vec<u8>, MediaError>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
                panggilan: Mutex::new(Vec::new()),
                unduhan: Mutex::new(Vec::new()),
                hasil_unduh,
            })
        }
    }

    impl HttpGet for Stub {
        /// Semua request plugin lewat `HttpCall` (multipart + GET header).
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.panggilan.lock().unwrap().push(req.clone());
            let i = {
                let mut hitung = self.hitung.lock().unwrap();
                let i = *hitung;
                *hitung += 1;
                i
            };
            match self.balasan.lock().unwrap().get(i).cloned() {
                Some((status, isi)) => Ok(HttpResponse {
                    status,
                    body: HttpBody::Bytes {
                        data: Arc::from(isi),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.call(&HttpCall::get(url))
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
    }

    impl MediaFetch for Stub {
        fn download(&self, origin: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            self.unduhan.lock().unwrap().push(origin);
            self.hasil_unduh.clone()
        }
    }

    fn media_gambar() -> Option<IncomingMedia> {
        Some(IncomingMedia {
            kind: IncomingKind::Image,
            mimetype: Some("image/jpeg".into()),
            file_name: None,
            caption: String::new(),
        })
    }

    fn ctx_stub(
        http: Arc<Stub>,
        media: Option<IncomingMedia>,
        quoted: Option<QuotedKind>,
        prefix: &str,
    ) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: prefix.into(),
            command: "tofigurev2".into(),
            media,
            quoted: quoted.map(|kind| QuotedMessage {
                id: "q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: ME.into(),
                text: String::new(),
                kind,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            http: http.clone(),
            media_fetch: http,
            ..Ctx::default()
        }
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaksi(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.clone()),
            _ => None,
        }
    }

    fn bytes_gambar(r: &Reply) -> Vec<u8> {
        match r.without_mutations().tanpa_reaksi() {
            Reply::Image { media, .. } => match media {
                Some(MediaSource::Bytes { data, .. }) => data.to_vec(),
                lain => panic!("harus bytes, dapat {lain:?}"),
            },
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = oktz_plugin_api::find("tofigurev2").unwrap();
        assert_eq!(p.meta.name, "tofigurev2");
        assert_eq!(p.meta.alias, ["figurev2", "figure2"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
        for a in ["figurev2", "figure2"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "tofigurev2");
        }
    }

    #[test]
    fn kartu_pakai_pakai_prefix_ctx_bukan_titik_hardcode() {
        // JS `${m.prefix}` (`tofigurev2.js:24`); stub lama menambah baris
        // "Prefix saat ini" yang tidak ada di JS dan memakai "." hardcode.
        let http = Stub::baru(vec![], Ok(Vec::new()));
        let r = tofigurev2(ctx_stub(http.clone(), None, None, "!"));
        assert_eq!(
            teks(&r),
            "🎭 *ꜰɪɢᴜʀᴇ sᴛʏʟᴇ ᴠ2*\n\n> Kirim/reply gambar untuk diubah ke style Figure\n\n`!tofigurev2`"
        );
        assert_eq!(reaksi(&r), None);
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn unduhan_quoted_dipakai_lebih_dulu() {
        let http = Stub::baru(
            vec![resp(200, b"https://litterbox.example/img.jpg"), resp(200, GAMBAR)],
            Ok(b"gambar-asli".to_vec()),
        );
        let r = tofigurev2(ctx_stub(
            http.clone(),
            Some(IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            Some(QuotedKind::Image),
            ".",
        ));
        assert_eq!(bytes_gambar(&r), GAMBAR);
        assert_eq!(reaksi(&r), Some("✅".into()));
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Quoted]);
    }

    #[test]
    fn media_pemicu_diunduh_kalau_quoted_bukan_media() {
        let http = Stub::baru(
            vec![resp(200, b"https://litterbox.example/img.jpg"), resp(200, GAMBAR)],
            Ok(b"asli".to_vec()),
        );
        let r = tofigurev2(ctx_stub(
            http.clone(),
            media_gambar(),
            Some(QuotedKind::Text),
            ".",
        ));
        assert_eq!(bytes_gambar(&r), GAMBAR);
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Trigger]);
    }

    #[test]
    fn unduhan_gagal_membalas_pesan_dan_reaksi_silang() {
        let http = Stub::baru(
            vec![],
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        );
        let r = tofigurev2(ctx_stub(http.clone(), media_gambar(), None, "."));
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), Some("❌".into()));
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn provider_upload_pertama_gagal_lalu_kedua_sukses() {
        let http = Stub::baru(
            vec![
                resp(500, b""),
                resp(200, br#"{"link":"https://cdn.example/a.jpg"}"#),
                resp(200, GAMBAR),
            ],
            Ok(b"gambar-asli".to_vec()),
        );
        let r = tofigurev2(ctx_stub(http.clone(), media_gambar(), None, "."));
        assert_eq!(bytes_gambar(&r), GAMBAR);
        assert_eq!(reaksi(&r), Some("✅".into()));
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(
            panggilan[0].url,
            "https://litterbox.catbox.moe/resources/internals/api.php"
        );
        assert_eq!(panggilan[1].url, "https://kappa.lol/api/upload");
        assert_eq!(
            panggilan[2].url,
            "https://api-faa.my.id/faa/tofigurav3?url=https%3A%2F%2Fcdn.example%2Fa.jpg"
        );
        match &panggilan[0].body {
            Some(HttpBody::Multipart(parts)) => {
                assert_eq!(parts[0].name, "reqtype");
                let berkas = parts.iter().find(|p| p.name == "fileToUpload").unwrap();
                assert_eq!(berkas.file_name.as_deref(), Some("image.jpg"));
                assert_eq!(berkas.mimetype.as_deref(), Some("image/jpeg"));
            }
            lain => panic!("harus multipart, dapat {lain:?}"),
        }
    }

    #[test]
    fn semua_provider_upload_gagal_membalas_template_error() {
        // Litterbox/Kappa/Uguu/Nekohime masing-masing 1 call, Leopard 2
        // (pemanasan + POST), Upload.ee 2 (pemanasan + rnd_id; `startUpload`
        // tidak ketemu di body kosong lalu provider menggagalkan diri).
        let balasan: Vec<(u16, Vec<u8>)> = (0..8).map(|_| resp(500, b"")).collect();
        let http = Stub::baru(balasan, Ok(b"gambar-asli".to_vec()));
        let r = tofigurev2(ctx_stub(http.clone(), media_gambar(), None, "."));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "tofigurev2"));
        assert_eq!(reaksi(&r), Some("☢".into()));
        assert_eq!(http.panggilan.lock().unwrap().len(), 8);
    }

    #[test]
    fn efek_non_2xx_membalas_template_error() {
        let http = Stub::baru(
            vec![resp(200, b"https://litterbox.example/img.jpg"), resp(500, b"")],
            Ok(b"gambar-asli".to_vec()),
        );
        let r = tofigurev2(ctx_stub(http.clone(), media_gambar(), None, "."));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "tofigurev2"));
        assert_eq!(reaksi(&r), Some("☢".into()));
    }

    #[test]
    fn respons_efek_bukan_gambar_membalas_template_error() {
        // `looksLikeImage` (`effect-api.js:57-67`) menolak HTML.
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/img.jpg"),
                resp(200, b"<html>bukan gambar</html>"),
            ],
            Ok(b"gambar-asli".to_vec()),
        );
        let r = tofigurev2(ctx_stub(http.clone(), media_gambar(), None, "."));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "tofigurev2"));
        assert_eq!(reaksi(&r), Some("☢".into()));
    }

    #[test]
    fn scanner_tautan_leopard_dan_upload_ee() {
        assert_eq!(
            tautan_leopard("Download link: <a href=https://leopard.example/x.png>").as_deref(),
            Some("https://leopard.example/x.png")
        );
        assert_eq!(
            tautan_leopard("download LINK:\n<a href=https://x/y>").as_deref(),
            Some("https://x/y")
        );
        assert_eq!(
            file_src_upload_ee(r#"<input id="file_src" name="x" value="https://upload.ee/a.html">"#)
                .as_deref(),
            Some("https://upload.ee/a.html")
        );
        assert_eq!(
            view_file_upload_ee("View file: <br /><a href=\"https://upload.ee/b.html\">").as_deref(),
            Some("https://upload.ee/b.html")
        );
        assert_eq!(
            start_upload(r#"x.startUpload("abc123", 1);"#).as_deref(),
            Some("abc123")
        );
    }
}
