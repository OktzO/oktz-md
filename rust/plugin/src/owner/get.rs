//! `plugins/owner/get.js` — advanced HTTP request tool (Owner Only).
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `sock.sendMessage` dengan banyak varian (image/video/audio/
//! document/gif `gifPlayback` + `mimetype`, dan document sebagai file saat
//! respons besar), `axios` dengan konfigurasi penuh (method sembarang,
//! `--auth` basic auth, `--header`, `validateStatus: () => true`,
//! `responseType: "arraybuffer"`), `new URL()` validasi, dan daftar blocked
//! URL (localhost/metadata). Tidak ada padanan lengkapnya di `Ctx`.
//!
//! Fungsi murni di bawah (`detect_extension`, `format_size`,
//! `get_mime_category`, `is_blocked_url`) disalin dari JS untuk dijaga agar
//! tidak hilang, dan diuji.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `get.js:23-45` — deteksi ekstensi dari `content-type`.
#[cfg(test)]
pub(crate) fn detect_extension(content_type: &str) -> &str {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("json") {
        "json"
    } else if ct.contains("html") {
        "html"
    } else if ct.contains("xml") {
        "xml"
    } else if ct.contains("javascript") {
        "js"
    } else if ct.contains("css") {
        "css"
    } else if ct.contains("yaml") {
        "yml"
    } else if ct.contains("text") {
        "txt"
    } else if ct.contains("csv") {
        "csv"
    } else if ct.contains("pdf") {
        "pdf"
    } else if ct.contains("zip") {
        "zip"
    } else if ct.contains("gzip") || ct.contains("tar") {
        "gz"
    } else if ct.contains("png") {
        "png"
    } else if ct.contains("jpeg") {
        "jpg"
    } else if ct.contains("webp") {
        "webp"
    } else if ct.contains("svg") {
        "svg"
    } else if ct.contains("mp4") {
        "mp4"
    } else if ct.contains("webm") {
        "webm"
    } else if ct.contains("mpeg") || ct.contains("mp3") {
        "mp3"
    } else if ct.contains("ogg") {
        "ogg"
    } else if ct.contains("wav") {
        "wav"
    } else {
        "bin"
    }
}

/// `get.js:70-75` — format ukuran byte.
#[cfg(test)]
pub(crate) fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut i = 0usize;
    while i < units.len() - 1 && size >= 1024.0 {
        size /= 1024.0;
        i += 1;
    }
    let decimals = if i > 0 { 2 } else { 0 };
    format!("{size:.decimals$} {}", units[i])
}

/// `get.js:47-54` — kategori MIME.
#[cfg(test)]
pub(crate) fn get_mime_category(mime_type: &str) -> &str {
    let mt = mime_type.to_ascii_lowercase();
    if mt == "image/gif" {
        "gif"
    } else if mt.starts_with("audio/") {
        "audio"
    } else if mt.starts_with("video/") {
        "video"
    } else if mt.starts_with("image/") {
        "image"
    } else if mt.starts_with("text/") || mt.contains("json") {
        "text"
    } else {
        "document"
    }
}

/// `get.js:56-68` — URL blokir (localhost/internal/metadata).
#[cfg(test)]
pub(crate) fn is_blocked_url(url: &str) -> bool {
    let blocked = [
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
        "::1",
        "169.254.",
        "metadata.google",
        "metadata.azure",
        "100.100.100.200",
    ];
    blocked.iter().any(|b| url.contains(b))
}

#[plugin(
    name = "get",
    alias = ["fetch", "http", "request", "curl"],
    category = "owner",
    description = "Advanced HTTP request tool (Owner Only) (DIBLOKIR: butuh sock + axios penuh)",
    usage = ".get <url> [options]",
    example = ".get https://api.example.com --method POST --json {\"key\":\"value\"}",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn get(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("get").unwrap();
        assert_eq!(p.meta.name, "get");
        assert_eq!(p.meta.alias, ["fetch", "http", "request", "curl"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn detect_extension_mirip_js() {
        assert_eq!(detect_extension("application/json"), "json");
        assert_eq!(detect_extension("text/html"), "html");
        assert_eq!(detect_extension("image/png"), "png");
        assert_eq!(detect_extension("image/jpeg"), "jpg");
        assert_eq!(detect_extension("audio/mpeg"), "mp3");
        assert_eq!(detect_extension("video/mp4"), "mp4");
        assert_eq!(detect_extension("application/pdf"), "pdf");
        assert_eq!(detect_extension("unknown/type"), "bin");
    }

    #[test]
    fn format_size_mirip_js() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1500), "1.46 KB");
        assert_eq!(format_size(1_048_576), "1.00 MB");
    }

    #[test]
    fn get_mime_category_mirip_js() {
        assert_eq!(get_mime_category("image/gif"), "gif");
        assert_eq!(get_mime_category("audio/mpeg"), "audio");
        assert_eq!(get_mime_category("video/mp4"), "video");
        assert_eq!(get_mime_category("image/png"), "image");
        assert_eq!(get_mime_category("application/json"), "text");
        assert_eq!(get_mime_category("application/zip"), "document");
    }

    #[test]
    fn is_blocked_url_mirip_js() {
        assert!(is_blocked_url("http://localhost/x"));
        assert!(is_blocked_url("http://127.0.0.1/"));
        assert!(is_blocked_url("http://169.254.169.254/"));
        assert!(!is_blocked_url("https://example.com/"));
        assert!(!is_blocked_url(""));
    }
}
