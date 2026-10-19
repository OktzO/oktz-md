//! `plugins/canvas/fakeml.js` — membuat fake ML profile card.
//!
//! Alur JS: ambil gambar (quoted gambar → gambar pemicu → foto profil pengirim
//! → aset `pp-kosong`, `fakeml.js:33-58`) → unggah (`uploadTo0x0`) → API
//! `fakelobyml` (`fakeml.js:70`) → kirim hasilnya sebagai gambar.
//!
//! `uploadTo0x0` di JS sekarang hanya meneruskan ke `uploadImage`
//! (`src/lib/tmpfiles.js:7-16`), jadi rantai 6 host di blok SALINAN bawah ini
//! memang helper yang dipakai `fakeml.js:64-67`.
//!
//! Penyimpangan yang tercatat apa adanya:
//!
//! - **Reaksi `🕕` dibuang** (`fakeml.js:62`). `Reply` hanya bisa membawa satu
//!   reaksi, jadi yang ikut adalah reaksi penutup (`✅`/`❌`).
//! - **API diambil eksplisit.** JS menyerahkan URL API ke
//!   `sock.sendMedia(..., { type: 'image' })` dan Baileys yang mengunduhnya
//!   (`fakeml.js:68-76`); di sini `ctx.http.get` yang mengunduh, supaya
//!   kegagalan API tetap masuk cabang catch yang sama (`fakeml.js:78-81`:
//!   reaksi `❌` + balas `Coba lagi`), bukan jadi error jaringan pipeline.
//! - **Aset `pp-kosong` tidak ada / gagal dibaca** di JS melempar keluar
//!   handler (`fakeml.js:56`); di sini `None` diperlakukan seperti buffer
//!   kosong dan memakai pesan ❌ yang sama.
//!
//! `pub mod fakeml;` perlu ditambah manual ke `src/canvas/mod.rs` — berkas ini
//! belum ikut dikompilasi tanpa baris itu.
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
            if let Some(akhir) = isi.find(|c: char| c == '"' || c == '\'').filter(|n| *n > 0) {
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
            &HttpCall::get(&format!(
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
            &HttpCall::get(&format!(
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
    let providers: [(&str, fn(&dyn HttpGet, &[u8]) -> Result<String, String>); 6] = [
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

/// Balasan yang sama dipakai dua kali: `!buffer` (`fakeml.js:60`).
const MINTA_GAMBAR: &str = "❌ Kirim/reply gambar untuk dijadikan avatar!";

/// `fakeml.js:26-30` — kartu pakai; `${m.prefix}` muncul dua kali.
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "🎮 *ꜰᴀᴋᴇ ᴍʟ ᴘʀᴏꜰɪʟᴇ*\n\n\
         > Masukkan nama untuk profile\n\n\
         *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
         > 1. Kirim foto + caption `{prefix}fakeml <nama>`\n\
         > 2. Reply foto dengan `{prefix}fakeml <nama>`"
    )
}

/// `fakeml.js:70` — API canvas; `encodeURIComponent` untuk kedua parameter.
fn api_url(avatar: &str, nickname: &str) -> String {
    format!(
        "https://api.nexray.web.id/maker/fakelobyml?avatar={}&nickname={}",
        encode_uri_component(avatar),
        encode_uri_component(nickname)
    )
}

/// `getAssetBuffer("pp-kosong")` (`fakeml.js:56`) — baca berkasnya lewat
/// `ctx.berkas()`; jalur asetnya dari `ctx.config().assets`.
fn aset_pp_kosong(ctx: &Ctx) -> Option<Vec<u8>> {
    let jalur = ctx.config().assets.get("pp-kosong")?;
    ctx.berkas().baca(jalur).ok()
}

/// Cabang `fakeml.js:49-58` — foto profil pengirim; apa pun yang gagal
/// (sock melempar, tanpa foto, unduhan non-2xx) jatuh ke aset `pp-kosong`.
fn foto_profil_atau_aset(ctx: &Ctx) -> Option<Vec<u8>> {
    match ctx.sock().foto_profil(&ctx.sender, "image") {
        Ok(Some(url)) => match ctx.http.get(&url) {
            Ok(res) if respons_ok(&res) => Some(res.body.bytes().to_vec()),
            _ => aset_pp_kosong(ctx),
        },
        _ => aset_pp_kosong(ctx),
    }
}

#[plugin(
    name = "fakeml",
    alias = ["mlbbfake", "mlcard", "mlfake"],
    category = "canvas",
    description = "Membuat fake ML profile card",
    usage = ".fakeml <nama> (reply/kirim foto)",
    example = ".fakeml Misaki",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fakeml(ctx: Ctx) -> Reply {
    let nama = ctx.text.trim();
    if nama.is_empty() {
        return Reply::text(bantuan(&ctx.prefix));
    }

    // `fakeml.js:33-58` — quoted gambar dulu, baru gambar pemicu, terakhir PP.
    let unduhan: Option<Result<Vec<u8>, ()>> = if ctx.quoted().is_some_and(|q| q.kind.is_image()) {
        Some(ctx.download_quoted_media().map_err(|_| ()))
    } else if ctx.is_media_kind(IncomingKind::Image) {
        Some(ctx.download_media().map_err(|_| ()))
    } else {
        None
    };

    let buffer = match unduhan {
        // `fakeml.js:40-42`/`:46-48` — `m.reply(te(...))` TANPA `return`;
        // alurnya lanjut ke pemeriksaan `!buffer`, jadi user menerima DUA
        // balasan berurutan.
        Some(Err(())) => {
            return Reply::sequence_text([
                crate::common::error_template(&ctx.prefix, &ctx.command),
                MINTA_GAMBAR.to_string(),
            ]);
        }
        Some(Ok(byte)) => Some(byte),
        None => foto_profil_atau_aset(&ctx),
    };

    let Some(buffer) = buffer else {
        return Reply::text(MINTA_GAMBAR);
    };

    // ponytail: reaksi 🕕 (`fakeml.js:62`) hilang — `Reply` hanya membawa
    // satu reaksi, jadi hanya reaksi penutup di bawah yang dikirim.

    // `fakeml.js:78-81` — catch hanya membalas `Coba lagi` + reaksi ❌, BUKAN
    // template error.
    let gagal = || Reply::text("Coba lagi").with_reaction("❌");
    let Ok(avatar) = upload_image(&*ctx.http, &buffer) else {
        return gagal();
    };
    let Ok(res) = ctx.http.get(&api_url(&avatar, nama)) else {
        return gagal();
    };
    // Baileys yang mengunduh `sendMedia(url, { type: 'image' })`; status
    // non-2xx melempar di sana, jadi di sini pun masuk cabang gagal yang sama.
    if !respons_ok(&res) {
        return gagal();
    }
    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::{Berkas as _, BerkasPalsu};
    use oktz_plugin_api::config::{Assets, Runtime};
    use oktz_plugin_api::sock::{AdaWhatsapp, AksiPeserta, Sock, SockError};
    use oktz_plugin_api::{
        HttpBody, HttpError, IncomingMedia, MediaError, MediaFetch, MediaOrigin, QuotedKind,
        QuotedMessage,
    };
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// Byte gambar uji + byte balasan API.
    const GAMBAR: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01";
    const ISI_API: &[u8] = b"HASIL-API";
    const ASET: &[u8] = b"PP-KOSONG";

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

    /// `sock.profilePictureUrl` — hasil atau error yang bisa disetel, dan
    /// hitungan panggilan supaya cabang tanpa gambar bisa dipastikan.
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
        prefix: &str,
        text: &str,
        media: Option<IncomingMedia>,
        quoted: Option<QuotedKind>,
    ) -> Ctx {
        Ctx {
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: prefix.into(),
            command: "fakeml".into(),
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

    /// Daftarkan aset `pp-kosong` di config + berkas palsu.
    fn dengan_aset(mut ctx: Ctx, jalur: &str, isi: &[u8]) -> Ctx {
        let berkas = BerkasPalsu::baru();
        berkas.tulis(jalur, isi).unwrap();
        let mut assets = std::collections::BTreeMap::new();
        assets.insert("pp-kosong".to_string(), jalur.to_string());
        ctx.config = Arc::new(Runtime {
            assets: Assets(assets),
            ..Default::default()
        });
        ctx.berkas = Arc::new(berkas);
        ctx
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn urutan_teks(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Sequence(list) => list
                .iter()
                .map(|i| i.text_body().unwrap_or_default().to_string())
                .collect(),
            lain => vec![lain.text_body().unwrap_or_default().to_string()],
        }
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
        let p = oktz_plugin_api::find("fakeml").unwrap();
        assert_eq!(p.meta.name, "fakeml");
        assert_eq!(p.meta.alias, ["mlbbfake", "mlcard", "mlfake"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.description, "Membuat fake ML profile card");
        assert_eq!(p.meta.usage, ".fakeml <nama> (reply/kirim foto)");
        assert_eq!(p.meta.example, ".fakeml Misaki");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        for a in ["mlbbfake", "mlcard", "mlfake"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "fakeml");
        }
    }

    #[test]
    fn nama_kosong_atau_spasi_mengirim_kartu_pakai_tanpa_reaksi() {
        let http = Stub::baru(vec![], Ok(Vec::new()));
        let sock = Arc::new(StubSock::default());
        for (prefix, text) in [(".", ""), (".", "   "), ("!", "")] {
            let r = fakeml(ctx_stub(
                http.clone(),
                sock.clone(),
                prefix,
                text,
                None,
                None,
            ));
            assert_eq!(teks(&r), bantuan(prefix), "input {text:?}");
            assert_eq!(reaksi(&r), None, "input {text:?}");
        }
        assert!(http.panggilan.lock().unwrap().is_empty());
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0, "foto profil tidak dipanggil");
    }

    #[test]
    fn quoted_gambar_gagal_unduh_mengirim_template_lalu_minta_gambar() {
        let http = Stub::baru(
            vec![],
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        );
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock.clone(),
            ".",
            "Misaki",
            None,
            Some(QuotedKind::Image),
        ));
        assert_eq!(
            urutan_teks(&r),
            vec![
                crate::common::error_template(".", "fakeml"),
                MINTA_GAMBAR.to_string()
            ]
        );
        assert_eq!(reaksi(&r), None);
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Quoted]);
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn gambar_pemicu_gagal_unduh_juga_mengirim_dua_balasan() {
        let http = Stub::baru(
            vec![],
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        );
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock.clone(),
            ".",
            "Misaki",
            media_gambar(),
            None,
        ));
        assert_eq!(
            urutan_teks(&r),
            vec![
                crate::common::error_template(".", "fakeml"),
                MINTA_GAMBAR.to_string()
            ]
        );
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Trigger]);
        assert_eq!(*sock.dipanggil.lock().unwrap(), 0);
    }

    #[test]
    fn quoted_bukan_gambar_gambar_pemicu_yang_dipakai() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/img.jpg"),
                resp(200, ISI_API),
            ],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock.clone(),
            ".",
            "Misaki",
            media_gambar(),
            Some(QuotedKind::Text),
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Trigger]);
        // `fakeml.js:70` — hanya avatar + nickname yang dikirim.
        assert_eq!(
            http.panggilan.lock().unwrap()[1].url,
            api_url("https://litterbox.example/img.jpg", "Misaki")
        );
    }

    #[test]
    fn quoted_gambar_menang_atas_gambar_pemicu() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/img.jpg"),
                resp(200, ISI_API),
            ],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock.clone(),
            ".",
            "Misaki",
            media_gambar(),
            Some(QuotedKind::Image),
        ));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(*http.unduhan.lock().unwrap(), [MediaOrigin::Quoted]);
    }

    #[test]
    fn tanpa_gambar_pakai_foto_profil_lalu_api() {
        let http = Stub::baru(
            vec![
                resp(200, GAMBAR),
                resp(200, b"https://litterbox.example/pp.jpg"),
                resp(200, ISI_API),
            ],
            Ok(Vec::new()),
        );
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Ok(Some("https://pp.example/a.jpg".into())));
        let r = fakeml(ctx_stub(http.clone(), sock.clone(), ".", "Misaki", None, None));
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(panggilan[0].url, "https://pp.example/a.jpg");
        assert_eq!(
            panggilan[0].method,
            oktz_plugin_api::HttpMethod::Get
        );
        assert_eq!(panggilan[1].url, "https://litterbox.catbox.moe/resources/internals/api.php");
        assert_eq!(
            panggilan[2].url,
            api_url("https://litterbox.example/pp.jpg", "Misaki")
        );
        assert_eq!(*sock.dipanggil.lock().unwrap(), 1);
    }

    #[test]
    fn foto_profil_error_pakai_aset_pp_kosong() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/aset.jpg"),
                resp(200, ISI_API),
            ],
            Ok(Vec::new()),
        );
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Err(SockError::baru("gaada pp")));
        let ctx = dengan_aset(
            ctx_stub(http.clone(), sock.clone(), ".", "Misaki", None, None),
            "pp-kosong.jpeg",
            ASET,
        );
        let r = fakeml(ctx);
        assert_eq!(bytes_gambar(&r), ISI_API);
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(panggilan.len(), 2, "tidak ada GET foto profil");
        assert_eq!(panggilan[0].url, "https://litterbox.catbox.moe/resources/internals/api.php");
        match &panggilan[0].body {
            Some(HttpBody::Multipart(parts)) => {
                let berkas = parts.iter().find(|p| p.name == "fileToUpload").unwrap();
                assert_eq!(berkas.body.bytes(), ASET);
            }
            lain => panic!("harus multipart, dapat {lain:?}"),
        }
    }

    #[test]
    fn foto_profil_null_pakai_aset_pp_kosong() {
        let http = Stub::baru(
            vec![
                resp(200, b"https://litterbox.example/aset.jpg"),
                resp(200, ISI_API),
            ],
            Ok(Vec::new()),
        );
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Ok(None));
        let ctx = dengan_aset(
            ctx_stub(http.clone(), sock.clone(), ".", "Misaki", None, None),
            "pp-kosong.jpeg",
            ASET,
        );
        let r = fakeml(ctx);
        assert_eq!(bytes_gambar(&r), ISI_API);
        assert_eq!(reaksi(&r), Some("✅".into()));
    }

    #[test]
    fn foto_profil_non_2xx_pakai_aset_pp_kosong() {
        let http = Stub::baru(
            vec![
                resp(404, b""),
                resp(200, b"https://litterbox.example/aset.jpg"),
                resp(200, ISI_API),
            ],
            Ok(Vec::new()),
        );
        let sock = Arc::new(StubSock::default());
        *sock.foto.lock().unwrap() = Some(Ok(Some("https://pp.example/a.jpg".into())));
        let ctx = dengan_aset(
            ctx_stub(http.clone(), sock.clone(), ".", "Misaki", None, None),
            "pp-kosong.jpeg",
            ASET,
        );
        let r = fakeml(ctx);
        assert_eq!(bytes_gambar(&r), ISI_API);
        let panggilan = http.panggilan.lock().unwrap();
        assert_eq!(panggilan[0].url, "https://pp.example/a.jpg");
        assert_eq!(panggilan[1].url, "https://litterbox.catbox.moe/resources/internals/api.php");
    }

    #[test]
    fn semua_host_upload_gagal_membalas_coba_lagi_dan_reaksi_silang() {
        // Litterbox/Kappa/Uguu/Nekohime masing-masing 1 call, Leopard 2
        // (pemanasan + POST), Upload.ee 2 (pemanasan + rnd_id; `startUpload`
        // tidak ketemu di body kosong lalu provider menggagalkan diri).
        let balasan: Vec<(u16, Vec<u8>)> = (0..8).map(|_| resp(500, b"")).collect();
        let http = Stub::baru(balasan, Ok(GAMBAR.to_vec()));
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock,
            ".",
            "Misaki",
            media_gambar(),
            None,
        ));
        assert_eq!(teks(&r), "Coba lagi");
        assert_eq!(reaksi(&r), Some("❌".into()));
        assert_eq!(http.panggilan.lock().unwrap().len(), 8);
    }

    #[test]
    fn api_non_2xx_membalas_coba_lagi_dan_reaksi_silang() {
        let http = Stub::baru(
            vec![resp(200, b"https://litterbox.example/img.jpg"), resp(500, b"")],
            Ok(GAMBAR.to_vec()),
        );
        let sock = Arc::new(StubSock::default());
        let r = fakeml(ctx_stub(
            http.clone(),
            sock,
            ".",
            "Misaki",
            media_gambar(),
            None,
        ));
        assert_eq!(teks(&r), "Coba lagi");
        assert_eq!(reaksi(&r), Some("❌".into()));
    }

    #[test]
    fn api_url_mengenkode_avatar_dan_nama() {
        assert_eq!(
            api_url("https://a.example/b.jpg?x=1", "Misaki A"),
            "https://api.nexray.web.id/maker/fakelobyml?avatar=https%3A%2F%2Fa.example%2Fb.jpg%3Fx%3D1&nickname=Misaki%20A"
        );
    }
}
