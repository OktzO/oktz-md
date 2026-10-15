//! `plugins/download/mediafiredl.js` — unduh file dari MediaFire.
//!
//! `ponytail:` scraper `mediafire` (`src/scraper/mediafire.js`) mem-parsing HTML
//! halaman MediaFire dengan `cheerio`, dan `oktz-plugins` tidak punya parser
//! HTML (crate baru dilarang). Yang dipindah di sini SEMUA logika plugin: guard
//! URL, kartu pakai, dan `getFileName` beserta `decodeURIComponent`-nya.
//! Pemanggilan scraper menjadi satu fungsi `scrape` yang selalu gagal di port
//! ini, jadi cabang `catch` yang menyala — persis seperti scraper yang melempar.
//!
//! `ponytail:` `Reply` tidak punya variant document, jadi hasil `sendMessage`
//! hanya terkirim sebagai nama file teks.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::now_ms;
use crate::fun::tod::error_template;

/// `mediafiredl.js:43` — `/mediafire\.com/i`, tidak dikunci ke host.
pub(crate) fn is_mediafire_url(url: &str) -> bool {
    url.to_lowercase().contains("mediafire.com")
}

/// `mediafiredl.js:35-40` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}mfdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}mfdl https://www.mediafire.com/file/xxx`"
    )
}

/// `mediafiredl.js:44` — seluruh pesan dibungkus asterisk, termasuk `link`/`File`.
pub(crate) const BAD_URL: &str = "❌ *URL tidak valid. Gunakan link MediaFire.*";

/// `mediafiredl.js:52` — respons tanpa `download.link_download`.
pub(crate) const NO_LINK: &str =
    "❌ Gagal mendapatkan link download. File mungkin tidak tersedia.";

/// Bagian hasil `mediafire` (`src/scraper/mediafire.js:72-82`) yang dibaca
/// plugin: `meta.title` dan `download.link_download`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediafireFile {
    pub title: String,
    pub link_download: String,
}

/// `decodeURIComponent` — hanya `%XX`, spasi tidak disentuh (beda dari
/// `decodeURIComponent` versi form). Rangkaian `%` yang tidak sah dibiarkan
/// apa adanya; di JS itu melempar `URIError`, yang tidak akan pernah terjadi
/// karena string-nya berasal dari URL MediaFire.
///
/// Argumen byte per byte lalu didekode UTF-8 sekaligus: `from_utf8_lossy`
/// satu kali menjaga urutan byte tetap sama seperti `decodeURIComponent`.
pub(crate) fn decode_uri_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `mediafiredl.js:20-29` — `getFileName`. Argumen `now_ms` menggantikan
/// `Date.now()` supaya cabang cadangan bisa diuji.
pub(crate) fn get_file_name(result: &MediafireFile, now_ms: i64) -> String {
    let direct_name = result.title.trim();
    let url_name = decode_uri_component(
        result
            .link_download
            .split('/')
            .next_back()
            .map(|s| s.split('?').next().unwrap_or(""))
            .unwrap_or(""),
    );
    let extension = if url_name.contains('.') {
        format!(".{}", url_name.rsplit('.').next().unwrap_or(""))
    } else {
        String::new()
    };
    if !direct_name.is_empty() && !extension.is_empty() && !direct_name.contains('.') {
        return format!("{direct_name}{extension}");
    }
    if !direct_name.is_empty() {
        return direct_name.to_string();
    }
    if !url_name.is_empty() {
        return url_name;
    }
    format!("mediafire_{now_ms}{extension}")
}

/// `mediafire(url)` — `cheerio` tidak tersedia, jadi selalu gagal.
fn scrape(_url: &str) -> Result<MediafireFile, ()> {
    Err(())
}

#[plugin(
    name = "mediafiredl",
    alias = ["mfdl", "mediafire", "mf"],
    category = "download",
    description = "Download file dari MediaFire",
    usage = ".mfdl <url>",
    example = ".mfdl https://www.mediafire.com/file/xxx",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn mediafiredl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_mediafire_url(url) {
        return Reply::text(BAD_URL);
    }

    // `mediafire.js:69` melempar saat `link_download` kosong, jadi scraper itu
    // tidak pernah sampai ke pemeriksaan `mediafiredl.js:50`.
    let Ok(result) = scrape(url) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("🕕");
    };
    if result.link_download.is_empty() {
        return Reply::text(NO_LINK).with_reaction("❌");
    }

    // `mediafiredl.js:54-66` — `sendMessage` document; tidak ada padanannya.
    Reply::text(get_file_name(&result, now_ms())).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mfdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            ..Default::default()
        };
        (oktz_plugin_api::find("mediafiredl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn file(title: &str, link: &str) -> MediafireFile {
        MediafireFile {
            title: title.to_string(),
            link_download: link.to_string(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("mediafiredl").unwrap();
        assert_eq!(p.meta.name, "mediafiredl");
        assert_eq!(p.meta.alias, ["mfdl", "mediafire", "mf"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download file dari MediaFire");
        assert_eq!(p.meta.usage, ".mfdl <url>");
        assert_eq!(p.meta.example, ".mfdl https://www.mediafire.com/file/xxx");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mfdl", "mediafire", "mf"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mediafiredl");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card() {
        for text in ["", "   "] {
            assert_eq!(text_of(&run(text)), help("."), "input {text:?}");
        }
    }

    #[test]
    fn a_url_without_the_domain_is_rejected() {
        assert_eq!(text_of(&run("https://drive.google.com/x")), BAD_URL);
    }

    #[test]
    fn the_unanchored_regex_quirk_of_the_js_is_kept() {
        assert!(is_mediafire_url("https://mediafire.com.evil.example/x"));
        assert!(is_mediafire_url("https://notmediafire.com/x"));
        assert!(is_mediafire_url("https://WWW.MEDIAFIRE.COM/file/x"));
    }

    #[test]
    fn decode_uri_component_handles_utf8_and_leaves_bad_escapes_alone() {
        assert_eq!(decode_uri_component("a%20b"), "a b");
        assert_eq!(decode_uri_component("nama%20file%2Ezip"), "nama file.zip");
        assert_eq!(decode_uri_component("caf%C3%A9.txt"), "café.txt");
        assert_eq!(decode_uri_component("100%"), "100%");
        assert_eq!(decode_uri_component("%zz"), "%zz");
        assert_eq!(decode_uri_component("%2"), "%2");
        assert_eq!(decode_uri_component("+a+"), "+a+", "spasi tidak jadi plus");
    }

    #[test]
    fn the_file_name_prefers_the_page_title_plus_the_extension() {
        assert_eq!(
            get_file_name(&file("Contoh Video", "https://d.example/a%20video.mp4?k=1"), 0),
            "Contoh Video.mp4"
        );
        // `directName` yang sudah punya titik: pakai apa adanya, ekstensi URL diabaikan.
        assert_eq!(
            get_file_name(&file("arsip.tar.gz", "https://d.example/a.zip"), 0),
            "arsip.tar.gz"
        );
        // Tanpa titik pada title tapi URL juga tanpa titik: tidak ada ekstensi.
        assert_eq!(
            get_file_name(&file("Contoh Video", "https://d.example/download"), 0),
            "Contoh Video"
        );
    }

    #[test]
    fn the_file_name_falls_back_to_the_url_then_to_a_timestamp() {
        assert_eq!(
            get_file_name(&file("", "https://d.example/VIDEO.mp4?k=1"), 0),
            "VIDEO.mp4"
        );
        assert_eq!(
            get_file_name(&file("   ", "https://d.example/"), 1_700_000_000_000),
            "mediafire_1700000000000",
            "tanpa nama dan tanpa ekstensi: pakai stempel waktu"
        );
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        // `scrape` selalu gagal di port ini, jadi handler tidak pernah mencapai
        // cabang kirim — persis yang terjadi kalau `cheerio` gagal di Node.
        let r = run("https://www.mediafire.com/file/xxx");
        assert_eq!(text_of(&r), error_template(".", "mfdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam harus tetap ada di cabang catch"
        );
    }
}