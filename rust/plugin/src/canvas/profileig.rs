//! `plugins/canvas/profileig.js` — membuat canvas profile instagram.
//!
//! Alur JS: enam argumen dipisah `|` → gambar (reply/kirim gambar, diunggah)
//! atau foto profil pengirim → API `kyzznekoo.zone.id` → kirim gambar hasil
//! (`profileig.js:22-61`).
//!
//! Penyimpangan yang tercatat apa adanya:
//!
//! - **Reaksi `🕕` dibuang** (`profileig.js:30`). `Reply` hanya bisa membawa
//!   satu reaksi, jadi yang ikut adalah reaksi penutup (`✅`/`❌`).
//! - **Semua kegagalan jadi template error + `❌`.** JS membungkus unduh,
//!   unggah, API, dan kirim dalam satu `try` (`profileig.js:31-61`), jadi
//!   cabang catch yang sama dipakai untuk semuanya di sini.
//! - **`sock.profilePictureUrl` mengembalikan `Ok(None)`** kalau pengguna
//!   tidak punya foto; JS hanya menangkap lemparan, tapi hasil `None` di sana
//!   tetap berujung ke gambar default (`profileig.js:37-42`), jadi di sini
//!   `Err` dan `Ok(None)` sama-sama jatuh ke URL default.
//!
//! `pub mod profileig;` perlu ditambah manual ke `src/canvas/mod.rs` — berkas
//! ini belum ikut dikompilasi tanpa baris itu.
//!
//! `ponytail:` blok SALINAN sama persis dengan `ai/tofigurev2.rs`; angkat ke
//! `crate::common` kalau plugin di luar ai/canvas turut memakainya.

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
type Unggah = fn(&dyn HttpGet, &[u8]) -> Result<String, String>;

fn upload_image(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    // Urutan `PROVIDERS` (`upload-providers.js:183-190`).
    let providers: [(&str, Unggah); 6] = [
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

/// `profileig.js:23` — kartu saat `text` kosong; contohnya literal
/// `.profileig` (bukan `${m.prefix}`), termasuk baris `> ` berisi spasi.
pub(crate) const FORMAT_SALAH: &str = "Format salah!\n\n> Contoh: .profileig 10M|1|150|jokowi|Presiden RI|true\n> \n> _Catatan: Reply gambar untuk dijadikan foto profil._";

/// `profileig.js:25` — ada argumen yang kosong setelah di-`trim`.
pub(crate) const ARGUMEN_KURANG: &str =
    "Pastikan semua argumen diisi dengan benar, dipisah dengan tanda |.";

/// `profileig.js:41` — dipakai kalau pengirim tidak punya foto profil.
const PP_DEFAULT: &str = "https://i.ibb.co/3Fh9Q6M/blank-profile-picture.png";

/// `profileig.js:50-51`.
const UA_PROFILIG: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36";

/// `profileig.js:55`.
const CAPTION: &str = "✅ Berhasil membuat profile IG";

/// `profileig.js:45` — tujuh parameter, urutannya persis seperti JS.
fn api_url(
    url: &str,
    pengikut: &str,
    mengikuti: &str,
    postingan: &str,
    username: &str,
    bio: &str,
    verif: &str,
) -> String {
    format!(
        "https://kyzznekoo.zone.id/api/canvas/profileig?url={}&pengikut={}&mengikuti={}&postingan={}&username={}&bio={}&verif={}",
        encode_uri_component(url),
        encode_uri_component(pengikut),
        encode_uri_component(mengikuti),
        encode_uri_component(postingan),
        encode_uri_component(username),
        encode_uri_component(bio),
        encode_uri_component(verif)
    )
}

/// `profileig.js:32-43` — cabang gambar (unduh → unggah) atau foto profil.
fn gambar_atau_pp(ctx: &Ctx, is_image: bool) -> Result<String, ()> {
    if !is_image {
        // JS hanya menangkap lemparan (`profileig.js:37-42`); `Ok(None)` di
        // sini artinya hal yang sama: tidak ada URL → gambar default.
        return Ok(match ctx.sock().foto_profil(&ctx.sender, "image") {
            Ok(Some(url)) => url,
            _ => PP_DEFAULT.to_string(),
        });
    }
    // `const msg = m.quoted ? m.quoted : m` (`profileig.js:33`) — quoted
    // dipakai walau BUKAN gambar; unduhannya lalu gagal dan masuk catch.
    let buffer = if ctx.quoted().is_some() {
        ctx.download_quoted_media()
    } else {
        ctx.download_media()
    };
    let buffer = buffer.map_err(|_| ())?;
    upload_image(&*ctx.http, &buffer).map_err(|_| ())
}

#[plugin(
    name = "profileig",
    alias = ["profileig", "igprofile"],
    category = "canvas",
    description = "Membuat canvas profile instagram",
    usage = ".profileig <pengikut>|<mengikuti>|<postingan>|<username>|<bio>|<verif(true/false)>",
    example = ".profileig 10M|1|150|jokowi|Presiden RI|true",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn profileig(ctx: Ctx) -> Reply {
    // `profileig.js:23` — `!text`; spasi saja BUKAN kosong di sini, baru
    // ketahuan kosong oleh cek argumen (`profileig.js:25`).
    if ctx.text.is_empty() {
        return Reply::text(FORMAT_SALAH);
    }

    // `profileig.js:24` — `text.split("|").map(v => v.trim())`; argumen ke-7
    // dan seterusnya dibuang destructuring `const [..] = ..`.
    let mut bagian = ctx.text.split('|').map(str::trim);
    let pengikut = bagian.next().unwrap_or("");
    let mengikuti = bagian.next().unwrap_or("");
    let postingan = bagian.next().unwrap_or("");
    let username = bagian.next().unwrap_or("");
    let bio = bagian.next().unwrap_or("");
    let verif = bagian.next().unwrap_or("");
    if [pengikut, mengikuti, postingan, username, bio, verif].contains(&"") {
        return Reply::text(ARGUMEN_KURANG);
    }

    // `profileig.js:28` — gambar pemicu ATAU gambar yang di-reply.
    let is_image = ctx.is_media_kind(IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind.is_image());

    // ponytail: reaksi 🕕 (`profileig.js:30`) hilang — `Reply` hanya membawa
    // satu reaksi, jadi hanya reaksi penutup di bawah yang dikirim.

    // `profileig.js:57-61` — SEMUA kegagalan (unduh, unggah, API, kirim) masuk
    // catch yang sama: template error + reaksi ❌.
    let gagal = || {
        Reply::text(crate::common::error_template(&ctx)).with_reaction("❌")
    };

    let Ok(img_url) = gambar_atau_pp(&ctx, is_image) else {
        return gagal();
    };

    let alamat = api_url(&img_url, pengikut, mengikuti, postingan, username, bio, verif);
    let Ok(res) = ctx.http.call(
        &HttpCall::get(&alamat)
            .header("User-Agent", UA_PROFILIG)
            .header("Content-Type", "application/json"),
    ) else {
        return gagal();
    };
    // axios melempar untuk 4xx/5xx, jadi sama-sama masuk catch.
    if !respons_ok(&res) {
        return gagal();
    }

    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), CAPTION).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::sock::{AdaWhatsapp, AksiPeserta, Sock, SockError};
    use oktz_plugin_api::{
        HttpBody, HttpError, IncomingMedia, MediaError, MediaFetch, MediaOrigin, QuotedKind,
        QuotedMessage,
    };
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";
    const GAMBAR: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01";
    const ISI_API: &[u8] = b"HASIL-PROFILE-IG";

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

    /// `sock.profilePictureUrl` — hasil yang bisa disetel + hitungan panggilan.
    #[derive(Debug, Default)]
    struct StubSock {
        foto: Mutex<Option<Result<Option<String>, SockError>>>,
        dipanggil: Mutex<usize>,
    }

    impl Sock for StubSock {
        fn ada_whatsapp(&self, _: &[String]) -> Result<Vec<AdaWhatsapp>, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn foto_profil(&self, _: &str, _: &str) -> Result<Option<String>, SockError> {
            *self.dipanggil.lock().unwrap() += 1;
            match self.foto.lock().unwrap().clone() {
                Some(hasil) => hasil,
                None => Err(SockError::baru("tidak dipakai")),
            }
        }
        fn metadata_grup(&self, _: &str) -> Result<Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn semua_grup(&self) -> Result<Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn ubah_peserta(&self, _: &str, _: &[String], _: AksiPeserta) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn set_foto_profil(&self, _: &str, _: &[u8]) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn keluar_grup(&self, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn kode_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn info_undangan(&self, _: &str) -> Result<Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn buat_grup(&self, _: &str, _: &[String]) -> Result<Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn terima_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn tandai_dibaca(&self, _: &[Value]) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn presence(&self, _: &str, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn status_teks(&self, _: &str) -> Result<Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn nama_kontak(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn profil_bisnis(&self, _: &str) -> Result<Option<Value>, SockError> {
            Err(SockError::baru("tidak dipakai"))
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
        sock: Arc<StubSock>,
        text: &str,
        media: Option<IncomingMedia>,
        quoted: Option<QuotedKind>,
    ) -> Ctx {
        Ctx {
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "profileig".into(),
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
            sock,
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

    /// Isi balasan sebenarnya: `without_mutations` hanya membuang `Mutate`,
    /// jadi `Reaction { reply }` masih harus dibuka satu lapis.
    fn isi(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            lain => lain,
        }
    }

    fn bytes_gambar(r: &Reply) -> Vec<u8> {
        match isi(r) {
            Reply::Image { media, .. } => match media {
                Some(MediaSource::Bytes { data, .. }) => data.to_vec(),
                lain => panic!("harus bytes, dapat {lain:?}"),
            },
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn meta_sesuai_config_js() {
        let p = oktz_plugin_api::find("profileig").unwrap();
        assert_eq!(p.meta.name, "profileig");
        assert_eq!(p.meta.alias, ["profileig", "igprofile"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.description, "Membuat canvas profile instagram");
        assert_eq!(
            p.meta.usage,
            ".profileig <pengikut>|<mengikuti>|<postingan>|<username>|<bio>|<verif(true/false)>"
        );
        assert_eq!(
            p.meta.example,
            ".profileig 10M|1|150|jokowi|Presiden RI|true"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        for a in ["profileig", "igprofile"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "profileig");
        }
    }

    #[test]
    fn text_kosong_mengirim_format_salah_tanpa_reaksi() {
        let http = Stub::baru(vec![], Ok(Vec::new()));
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(http.clone(), sock.clone(), "", None, None));
        assert_eq!(teks(&r), FORMAT_SALAH);
        assert_eq!(reaksi(&r), None);
        assert!(http.panggilan.lock().unwrap().is_empty());
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0);
    }

    #[test]
    fn argumen_kurang_atau_kosong_mengirim_pesan_argumen() {
        let http = Stub::baru(vec![], Ok(Vec::new()));
        let sock = Arc::new(StubSock::default());
        // Termasuk spasi-saja (bukan `!text`, tapi splitnya kosong) dan
        // argumen di tengah yang kosong setelah trim.
        for text in [
            "   ",
            "10M|1|150|jokowi|Presiden RI",
            "|1|150|jokowi|Presiden RI|true",
            "10M|1|150|jokowi||true",
            "10M|1|150|jokowi|Presiden RI|",
        ] {
            let r = profileig(ctx_stub(http.clone(), sock.clone(), text, None, None));
            assert_eq!(teks(&r), ARGUMEN_KURANG, "input {text:?}");
            assert_eq!(reaksi(&r), None, "input {text:?}");
        }
        assert!(http.panggilan.lock().unwrap().is_empty());
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0);
    }

    #[test]
    fn argumen_ketujuh_diabaikan() {
        let http = Stub::baru(vec![resp(200, ISI_API)], Ok(Vec::new()));
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Ok(Some("https://pp.example/a.jpg".into())));
        let r = profileig(ctx_stub(
            http.clone(),
            sock,
            "10M|1|150|jokowi|Presiden RI|true|extra",
            None,
            None,
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(
            panggilan[0].url,
            api_url(
                "https://pp.example/a.jpg",
                "10M",
                "1",
                "150",
                "jokowi",
                "Presiden RI",
                "true"
            )
        );
    }

    #[test]
    fn gambar_pemicu_diunggah_lalu_api_dipanggil() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/img.jpg"),
                resp(200, ISI_API),
            ],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(
            http.clone(),
            sock.clone(),
            "10M|1|150|jokowi|Presiden RI|true",
            media_gambar(),
            None,
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Trigger]);
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0, "PP tidak dipakai");
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(panggilan.len(), 2);
        assert_eq!(panggilan[0].url, "https://litterbox.catbox.moe/resources/internals/api.php");
        assert_eq!(
            panggilan[1].url,
            api_url(
                "https://litterbox.example/img.jpg",
                "10M",
                "1",
                "150",
                "jokowi",
                "Presiden RI",
                "true"
            )
        );
        // `profileig.js:50-51` — UA + Content-Type dikirim apa adanya.
        let headers = &panggilan[1].headers;
        assert!(headers.iter().any(|(k, v)| k == "User-Agent" && v == UA_PROFILIG));
        assert!(headers.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn quoted_gambar_dipakai_lebih_dulu() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/img.jpg"),
                resp(200, ISI_API),
            ],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(
            http.clone(),
            sock,
            "10M|1|150|jokowi|Presiden RI|true",
            media_gambar(),
            Some(QuotedKind::Image),
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Quoted]);
    }

    #[test]
    fn quoted_bukan_gambar_tetap_diunduh_sebagai_gambar_lalu_gagal() {
        // `profileig.js:28` — `isImage` dari gambar pemicu; `:33` memilih
        // quoted APA PUN isinya, jadi unduhan quoted teks yang gagal.
        let http = Stub::baru(
            vec![],
            Err(MediaError {
                message: "bukan media".into(),
            }),
        );
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(
            http.clone(),
            sock.clone(),
            "10M|1|150|jokowi|Presiden RI|true",
            media_gambar(),
            Some(QuotedKind::Text),
        ));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "profileig"));
        assert_eq!(reaksi(&r), Some("❌".into()));
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Quoted]);
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0);
    }

    #[test]
    fn tanpa_gambar_pakai_foto_profil_tanpa_unggah() {
        let http = Stub::baru(vec![resp(200, ISI_API)], Ok(Vec::new()));
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Ok(Some("https://pp.example/a.jpg".into())));
        let r = profileig(ctx_stub(
            http.clone(),
            sock.clone(),
            "10M|1|150|jokowi|Presiden RI|true",
            None,
            None,
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(panggilan.len(), 1, "PP langsung dipakai, tanpa unggah");
        assert!(panggilan[0].url.contains("url=https%3A%2F%2Fpp.example%2Fa.jpg"));
        assert_eq!(*sock.dipanggil.lock().unwrap(), 1);
    }

    #[test]
    fn foto_profil_error_atau_null_pakai_url_default() {
        for hasil in [
            Err(SockError::baru("gaada pp")),
            Ok(None),
        ] {
            let http = Stub::baru(vec![resp(200, ISI_API)], Ok(Vec::new()));
            let sock = Arc::new(StubSock::default());
            *sock.foto.lock().unwrap() = Some(hasil);
            let r = profileig(ctx_stub(
                http.clone(),
                sock,
                "10M|1|150|jokowi|Presiden RI|true",
                None,
                None,
            ));
            assert_eq!(bytes_gambar(&r), ISI_API);
            let panggilan = http.panggilan.lock().unwrap();
            assert_eq!(
                panggilan[0].url,
                api_url(
                    PP_DEFAULT,
                    "10M",
                    "1",
                    "150",
                    "jokowi",
                    "Presiden RI",
                    "true"
                )
            );
        }
    }

    #[test]
    fn unggahan_gagal_membalas_template_error_dan_silang() {
        // Litterbox/Kappa/Uguu/Nekohime masing-masing 1 call, Leopard 2,
        // Upload.ee 2 (lihat catatan yang sama di `ai/tofigurev2.rs`).
        let balasan: Vec<(u16, Vec<u8>)> = (0..8).map(|_| resp(500, b"")).collect();
        let http = Stub::baru(balasan, Ok(GAMBAR.to_vec()));
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(
            http.clone(),
            sock,
            "10M|1|150|jokowi|Presiden RI|true",
            media_gambar(),
            None,
        ));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "profileig"));
        assert_eq!(reaksi(&r), Some("❌".into()));
        assert_eq!(http.panggilan.lock().unwrap().len(), 8);
    }

    #[test]
    fn api_non_2xx_membalas_template_error_dan_silang() {
        let http = Stub::baru(
            vec![resp(200, b"https://litterbox.example/img.jpg"), resp(500, b"")],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = profileig(ctx_stub(
            http.clone(),
            sock,
            "10M|1|150|jokowi|Presiden RI|true",
            media_gambar(),
            None,
        ));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "profileig"));
        assert_eq!(reaksi(&r), Some("❌".into()));
    }

    #[test]
    fn api_url_mengurutkan_dan_mengenkode_tujuh_param() {
        assert_eq!(
            api_url(
                "https://a.example/b c.jpg",
                "10M",
                "1",
                "150",
                "jokowi",
                "Presiden RI",
                "true"
            ),
            "https://kyzznekoo.zone.id/api/canvas/profileig?url=https%3A%2F%2Fa.example%2Fb%20c.jpg&pengikut=10M&mengikuti=1&postingan=150&username=jokowi&bio=Presiden%20RI&verif=true"
        );
    }
}
