//! `plugins/download/ytmp3.js` — unduh audio YouTube.
//!
//! `ponytail:` resolver berlapis (`src/lib/resolve.js`) belum di-port. Yang
//! dipakai di sini backend agregator yang SUDAH jadi tier kedua di JS
//! (`izuka` → `/api/downloader/ytmp3`, `src/capabilities/youtube.js:257`).
//! `izuka` punya `key: null` di `AGGREGATORS`, jadi tidak butuh
//! `config.APIkey.*`.
//!
//! `ponytail:` kanonikalisasi `kanonikDari(idVideo(url))` di dalam resolver
//! tidak ikut — aggregator menerima URL persis seperti plugin JS mengirimnya
//! (`getAudioDownload` melakukan `String(url).trim()` lalu menyerahkan ke
//! `resolve`). Tambahkan langkah itu kalau scraper lokal (`ytdl`) ikut di-port.
//!
//! `ponytail:` `Reply` tidak punya variant audio, jadi file audio tidak ikut
//! terkirim — teks berisi nama berkas yang akan dikirim tetap dikirim, sama
//! seperti `spotifydl.rs` untuk Spotify.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use super::douyindl::query_encode;

/// `AGGREGATORS.izuka.base` + path di `src/capabilities/youtube.js:257`.
pub(crate) const ENDPOINT: &str = "https://my.izuka-api.xyz/api/downloader/ytmp3";

/// `FORMAT_DEFAULT` (`src/lib/media-format.js:98`).
pub(crate) const FORMAT_DEFAULT: &str = "mp3";

/// `MIMETYPE_DARI_FORMAT` (`src/lib/media-format.js:33`).
#[allow(dead_code)] // dipakai `ytmp4.rs` lewat `super::`
const MIMETYPE_DARI_FORMAT: [(&str, &str); 14] = [
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("mp4", "audio/mp4"),
    ("m4v", "audio/mp4"),
    ("aac", "audio/aac"),
    ("ogg", "audio/ogg"),
    ("oga", "audio/ogg"),
    ("opus", "audio/ogg"),
    ("webm", "audio/webm"),
    ("wav", "audio/wav"),
    ("flac", "audio/flac"),
    ("amr", "audio/amr"),
    ("mkv", "audio/x-matroska"),
    ("3gp", "audio/3gpp"),
];

/// `FORMAT_DARI_MIME` (`src/lib/media-format.js:59`).
const FORMAT_DARI_MIME: [(&str, &str); 23] = [
    ("audio/mpeg", "mp3"),
    ("audio/mpeg3", "mp3"),
    ("audio/x-mpeg-3", "mp3"),
    ("audio/mp4", "m4a"),
    ("audio/x-m4a", "m4a"),
    ("audio/aac", "aac"),
    ("audio/ogg", "ogg"),
    ("audio/opus", "opus"),
    ("audio/webm", "webm"),
    ("audio/wav", "wav"),
    ("audio/x-wav", "wav"),
    ("audio/vnd.wave", "wav"),
    ("audio/flac", "flac"),
    ("audio/x-flac", "flac"),
    ("audio/amr", "amr"),
    ("audio/3gpp", "3gp"),
    ("audio/x-matroska", "mkv"),
    ("video/mp4", "mp4"),
    ("video/webm", "webm"),
    ("video/quicktime", "mov"),
    ("video/x-matroska", "mkv"),
    ("video/3gpp", "3gp"),
    ("video/x-flv", "flv"),
];

/// `TOKEN_FORMAT` — gabungan kunci `MIMETYPE_DARI_FORMAT` dan nilai
/// `FORMAT_DARI_MIME` (`media-format.js:84`), bukan daftar terpisah: kalau
/// daftar ini melenceng dari dua tabel itu, bug "format yang berbohong" bisa
/// muncul lagi hanya di satu sisi.
const TOKEN_FORMAT: [&str; 17] = [
    "mp3", "m4a", "mp4", "m4v", "aac", "ogg", "oga", "opus", "webm", "wav", "flac", "amr", "mkv",
    "3gp", "mov", "flv", "avi",
];

/// `String(x)` untuk nilai JSON — `dasarMime`/`formatFromToken` di JS memanggil
/// bentuk ini sebelum membaca apa pun, jadi `null` ikut jadi string kosong.
pub(crate) fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// `dasarMime` (`media-format.js:103`) — parameter MIME dibuang, huruf kecil.
fn dasar_mime(raw: &str) -> String {
    raw.split(';').next().unwrap_or("").trim().to_lowercase()
}

/// `formatFromToken` (`media-format.js:112`).
pub(crate) fn format_from_token(v: Option<&Value>) -> Option<String> {
    let dasar = dasar_mime(&v.map(js_to_string).unwrap_or_default());
    let token = dasar.strip_prefix('.').unwrap_or(&dasar).to_string();
    TOKEN_FORMAT.contains(&token.as_str()).then_some(token)
}

/// `formatFromMime` (`media-format.js:119`).
pub(crate) fn format_from_mime(v: Option<&Value>) -> Option<String> {
    let dasar = dasar_mime(&v.map(js_to_string).unwrap_or_default());
    FORMAT_DARI_MIME
        .iter()
        .find(|(mime, _)| *mime == dasar)
        .map(|(_, format)| (*format).to_string())
}

/// `mimetypeFor` (`media-format.js:130`).
#[allow(dead_code)] // dipakai `ytmp4.rs` lewat `super::`
pub(crate) fn mimetype_for(format: &str) -> String {
    let token = format_from_token(Some(&Value::String(format.to_string()))).unwrap_or_default();
    let key = if token.is_empty() { format } else { token.as_str() };
    MIMETYPE_DARI_FORMAT
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
        .or_else(|| {
            MIMETYPE_DARI_FORMAT
                .iter()
                .find(|(k, _)| *k == FORMAT_DEFAULT)
                .map(|(_, v)| (*v).to_string())
        })
        .unwrap_or_default()
}

/// `extensionFor` (`media-format.js:138`).
pub(crate) fn extension_for(format: &str) -> String {
    format_from_token(Some(&Value::String(format.to_string())))
        .unwrap_or_else(|| FORMAT_DEFAULT.to_string())
}

/// `new URL(x)` hanya untuk `searchParams.get("mime")` — tanpa dependensi URL.
/// `+` jadi spasi dan `%XX` jadi byte, persis `application/x-www-form-urlencoded`.
fn query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1.split('#').next()?;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(k) == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `/^https?:\/\//i` — dipakai `urlUnduhan` dan `urlVideo` di kapabilitas.
pub(crate) fn is_http_url(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// `urlUnduhan` (`youtube.js:311`) tiga nama pertama: `dl`, `download_url`,
/// `download`. Dipisah dari cabang `video_normal`/`url` supaya `ytmp4` bisa
/// menyisipkan cabangnya di posisi yang sama dengan JS.
pub(crate) fn url_nama(isi: &Value) -> String {
    for nama in ["dl", "download_url", "download"] {
        let kandidat = isi.get(nama).map(js_to_string).unwrap_or_default();
        let kandidat = kandidat.trim().to_string();
        if is_http_url(&kandidat) {
            return kandidat;
        }
    }
    String::new()
}

/// Cabang terakhir `urlUnduhan`: field `url` polos.
pub(crate) fn url_field(isi: &Value) -> String {
    let kandidat = isi.get("url").map(js_to_string).unwrap_or_default();
    let kandidat = kandidat.trim().to_string();
    if is_http_url(&kandidat) {
        kandidat
    } else {
        String::new()
    }
}

/// `formatDilaporan` (`youtube.js:319`).
pub(crate) fn format_dilaporkan(isi: &Value, url: &str) -> Option<String> {
    if let Some(token) = format_from_token(isi.get("format")) {
        return Some(token);
    }
    for nama in ["mime", "mimetype", "mimeType", "content_type"] {
        if let Some(format) = format_from_mime(isi.get(nama)) {
            return Some(format);
        }
    }
    query_param(url, "mime").and_then(|mime| format_from_mime(Some(&Value::String(mime))))
}

/// `normalize` (`youtube.js:331`) untuk jalur audio: `urlUnduhan` sudah
/// lengkap di sini — `url` polos jadi cabang terakhir, dan `video_normal` tidak
/// ada di format mp3.
pub(crate) fn normalize(isi: &Value) -> Result<AudioDownload, ()> {
    let mut url = url_nama(isi);
    if url.is_empty() {
        url = url_field(isi);
    }
    if url.is_empty() {
        return Err(());
    }
    let title = isi.get("title").map(js_to_string).unwrap_or_default();
    Ok(AudioDownload {
        title: title.trim().to_string(),
        format: format_dilaporkan(isi, &url).unwrap_or_else(|| FORMAT_DEFAULT.to_string()),
        url,
    })
}

/// Bentuk yang `getAudioDownload` kembalikan: `{ title, format, url }`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AudioDownload {
    pub title: String,
    pub format: String,
    pub url: String,
}

/// `ytmp3.js:87` — `${title || "audio"}.${extensionFor(format)}`.
pub(crate) fn file_name(audio: &AudioDownload) -> String {
    let title = if audio.title.is_empty() {
        "audio"
    } else {
        &audio.title
    };
    format!("{title}.{}", extension_for(&audio.format))
}

/// `pastikanBentuk` + `lewatIzuka` (`youtube.js:238,257`) digabung: pemeriksaan
/// bentuk aggregator diletakkan di sini juga karena `izuka` tidak butuh key.
fn unduh_audio(ctx: &Ctx, url: &str) -> Result<AudioDownload, ()> {
    let target = format!("{ENDPOINT}?url={}", query_encode(url));
    let respons = ctx.http.get(&target).map_err(|_| ())?;
    let body = respons.json().ok_or(())?;
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(());
    }
    let result = body.get("result").filter(|r| r.is_object()).ok_or(())?;
    let punya = result
        .get("download_url")
        .map(crate::common::js_truthy_value)
        .unwrap_or(false);
    if !punya {
        return Err(());
    }
    normalize(result)
}

/// `ytmp3.js:64` — link hilang.
pub(crate) fn contoh(prefix: &str) -> String {
    format!("Contoh: {prefix}ytmp3 https://youtube.com/watch?v=xxx")
}

/// `ytmp3.js:65` — bukan host YouTube.
pub(crate) const BUKAN_YOUTUBE: &str = "❌ URL harus YouTube";

/// `ytmp3.js:99` — cabang `catch`.
pub(crate) const GAGAL: &str = "Gagal mengunduh audio.";

/// `hostYoutube` (`ytmp3.js:29`) tanpa crate regex.
pub(crate) fn host_youtube(url: &str) -> bool {
    let mentah = url.trim();
    let lengkap = tautan_lengkap(mentah);
    let dasar = match lengkap {
        Some(t) => t,
        None => mentah
            .split_whitespace()
            .find(|token| token.to_lowercase().contains("youtu"))
            .unwrap_or(mentah)
            .to_string(),
    };
    let dengan_skema = if ada_skema(&dasar) {
        dasar
    } else {
        format!("https://{dasar}")
    };
    let Some((_, host)) = host_dari(&dengan_skema) else {
        return false;
    };
    host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com")
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `/\bhttps?:\/\/[^\s<>"']+/i` — `\b` berarti posisi sebelum `h` bukan karakter
/// kata, jadi `xhhttp://` bukan awal tautan.
fn tautan_lengkap(mentah: &str) -> Option<String> {
    let bytes = mentah.as_bytes();
    for i in 0..bytes.len() {
        if !mentah.is_char_boundary(i) {
            continue;
        }
        if i > 0 && is_word_byte(bytes[i - 1]) {
            continue;
        }
        let rest = &mentah[i..];
        let rb = rest.as_bytes();
        let scheme_len = if rb.len() >= 7 && rb[..7].eq_ignore_ascii_case(b"http://") {
            7
        } else if rb.len() >= 8 && rb[..8].eq_ignore_ascii_case(b"https://") {
            8
        } else {
            continue;
        };
        let tail = &rest[scheme_len..];
        let cut = tail
            .find(|c: char| {
                c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == '\''
            })
            .unwrap_or(tail.len());
        return Some(rest[..scheme_len + cut].to_string());
    }
    None
}

/// `/^[a-z][a-z0-9+.-]*:/i`.
fn ada_skema(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return false;
    }
    for &c in &b[1..] {
        if c == b':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || c == b'+' || c == b'-' || c == b'.') {
            return false;
        }
    }
    false
}

/// `new URL(...)` yang dipakai plugin hanya untuk `protocol` + `hostname`:
/// protokol http(s), userinfo dan port dibuang, host berhenti di `/?#`.
fn host_dari(url: &str) -> Option<(String, String)> {
    let (scheme, rest) = url.trim().split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or("");
    let host = match authority.split_once(':') {
        Some((h, _port)) => h,
        None => authority,
    };
    if host.is_empty() {
        return None;
    }
    Some((scheme, host.to_lowercase()))
}

#[plugin(
    name = "ytmp3",
    alias = ["youtubemp3", "ytaudio"],
    category = "download",
    description = "Download audio YouTube",
    usage = ".ytmp3 <url>",
    example = ".ytmp3 https://youtube.com/watch?v=xxx",
    cooldown = 20,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ytmp3(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(contoh(&ctx.prefix));
    }
    if !host_youtube(url) {
        return Reply::text(BUKAN_YOUTUBE);
    }

    match unduh_audio(&ctx, url) {
        // `ponytail:` `Reply` tidak punya `Audio`, jadi yang terkirim hanya
        // nama berkas — sama seperti `spotifydl.rs`.
        Ok(audio) => Reply::text(file_name(&audio)).with_reaction("✅"),
        Err(()) => Reply::text(GAGAL).with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Result<Value, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(v),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .body
                .clone()
                .map_err(|()| HttpError { message: "offline".into() })?
                .to_string();
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
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
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ytmp3".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("ytmp3").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn ok_body() -> Value {
        json!({
            "status": true,
            "result": {
                "title": "Lagu Contoh",
                "format": "mp3",
                "download_url": "https://cdn.example/a.mp3"
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ytmp3").unwrap();
        assert_eq!(p.meta.name, "ytmp3");
        assert_eq!(p.meta.alias, ["youtubemp3", "ytaudio"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download audio YouTube");
        assert_eq!(p.meta.usage, ".ytmp3 <url>");
        assert_eq!(p.meta.example, ".ytmp3 https://youtube.com/watch?v=xxx");
        assert_eq!(p.meta.cooldown, 20);
        assert_eq!(p.meta.energi, 2);
        // JS `ytmp3.js:4-13` tidak menyebut satu pun flag permission.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["youtubemp3", "ytaudio"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "ytmp3");
        }
    }

    #[test]
    fn only_youtube_hosts_pass_the_guard() {
        for good in [
            "https://youtube.com/watch?v=xxx",
            "https://m.youtube.com/watch?v=xxx",
            "https://youtu.be/xxx",
            "https://www.youtube.com/shorts/xxx",
            "youtu.be/xxx",
            "putar https://youtu.be/xxx dong",
            "  https://youtube.com/watch?v=xxx  ",
        ] {
            assert!(host_youtube(good), "harus lolos: {good}");
        }
        for bad in [
            "https://youtube.com.evil.example/x",
            "https://notyoutube.com/x",
            "https://evil.example/?r=youtube.com",
            "ftp://youtube.com/x",
            "",
            "   ",
            "halo dunia",
        ] {
            assert!(!host_youtube(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn a_missing_link_shows_the_example_without_reacting() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), "Contoh: .ytmp3 https://youtube.com/watch?v=xxx");
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_foreign_host_is_rejected_without_any_request() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://youtube.com.evil.example/x");
        assert_eq!(text_of(&r), BUKAN_YOUTUBE);
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_resolved_download_replies_with_the_file_name() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://youtube.com/watch?v=xxx");
        assert_eq!(text_of(&r), "Lagu Contoh.mp3");
        assert_eq!(reaksi(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0],
            "https://my.izuka-api.xyz/api/downloader/ytmp3\
             ?url=https%3A%2F%2Fyoutube.com%2Fwatch%3Fv%3Dxxx",
            "spasi jadi +, byte lain jadi %XX — searchParams.set, bukan encodeURIComponent"
        );
    }

    #[test]
    fn an_empty_title_becomes_audio_and_an_empty_format_becomes_mp3() {
        let audio = AudioDownload {
            title: String::new(),
            format: String::new(),
            url: "https://cdn.example/a".into(),
        };
        assert_eq!(file_name(&audio), "audio.mp3");
        assert_eq!(mimetype_for(""), "audio/mpeg");
        assert_eq!(extension_for(""), "mp3");
    }

    #[test]
    fn the_reported_format_wins_over_the_requested_one() {
        // `media-format.js` ada karena aggregator menjawab `webm` untuk slot
        // mp3; nama berkas dan mimetype harus mengikuti yang dilaporkan host.
        let isi = json!({
            "download_url": "https://cdn.example/a?mime=audio%2Fwebm",
            "mime": "audio/webm"
        });
        let audio = normalize(&isi).unwrap();
        assert_eq!(audio.format, "webm");
        assert_eq!(file_name(&audio), "audio.webm");
        assert_eq!(mimetype_for(&audio.format), "audio/webm");
    }

    #[test]
    fn the_mime_query_parameter_is_read_when_no_field_reports_a_format() {
        let isi = json!({"url": "https://x.googlevideo.com/v?mime=audio%2Fogg%3B+codecs%3Dopus&id=1"});
        let audio = normalize(&isi).unwrap();
        assert_eq!(audio.format, "ogg");
        assert_eq!(mimetype_for(&audio.format), "audio/ogg");
    }

    #[test]
    fn an_unknown_format_token_falls_back_to_mp3() {
        assert_eq!(format_from_token(Some(&json!("360"))), None);
        assert_eq!(format_from_token(Some(&json!(".MP3"))), Some("mp3".into()));
        assert_eq!(format_from_token(Some(&Value::Null)), None);
        assert_eq!(format_from_token(None), None);
    }

    #[test]
    fn a_response_without_a_download_url_reports_the_failure_message() {
        for body in [
            json!({ "status": false, "msg": "gagal" }),
            json!({ "status": true, "result": {} }),
            json!({ "status": true, "result": { "download_url": "" } }),
            json!({ "status": true, "result": { "download_url": "ftp://x/a.mp3" } }),
            json!({ "status": true }),
            json!([]),
        ] {
            let r = run(Stub::body(body.clone()), "https://youtube.com/watch?v=xxx");
            assert_eq!(text_of(&r), GAGAL, "body {body}");
            assert_eq!(reaksi(&r), Some("❌"));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_same_message() {
        let r = run(Stub::offline(), "https://youtube.com/watch?v=xxx");
        assert_eq!(text_of(&r), GAGAL);
        assert_eq!(reaksi(&r), Some("❌"));
    }
}
