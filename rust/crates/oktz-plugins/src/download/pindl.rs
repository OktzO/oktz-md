//! `plugins/download/pindl.js` — unduh gambar/video dari Pinterest.
//!
//! `ponytail:` lapisan kapabilitas (`src/capabilities/pinterest.js`) belum
//! di-port, dan seluruh jalur unduhan juga butuh `child_process` + `fs`
//! (`queueFFmpeg`, `fs.statSync`, `fs.readFileSync`) yang tidak ada di API
//! plugin. Yang dipindah di sini semua logika plugin: guard URL per label,
//! kartu pakai, `rencana_video`, `terlalu_besar`, pemilihan varian HLS, dan
//! susunan pesan gagal. `resolve_pinterest` selalu gagal, jadi yang menyala
//! adalah cabang `catch` biasa.
//!
//! `ponytail:` `Reply` tidak punya variant video maupun document, jadi file yang
//! diproses di JS tidak ikut terkirim di sini.

// Cabang unduh/HLS tidak terjangkau di sini (stub resolver selalu gagal), tapi
// kode JS-nya tetap ikut dipor supaya perilakunya bisa diuji dan nanti hidup
// begitu kapabilitasnya ada.
#![allow(dead_code)]

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{error_template, host_dari};

/// `pindl.js:20` — batas body video (dan batas file yang boleh dikirim).
pub(crate) const BATAS_BODY_VIDEO: u64 = 64 * 1024 * 1024;

/// `pindl.js:83-87` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📌 *ᴘɪɴᴛᴇʀᴇsᴛ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n\
         > Download gambar/video dari Pinterest\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}pindl https://pin.it/xxx`\n\
         > `{prefix}pindl https://pinterest.com/pin/xxx`"
    )
}

/// `pindl.js:94` — URL bukan Pinterest.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link Pinterest.";

/// `pindl.js:305-307` — tidak ada satu pun file yang terkirim.
///
/// `lastError` boleh `None` (baris "Penyebab" lalu tidak ada sama sekali) atau
/// `Some` yang dipotong 120 karakter seperti `String(lastError).slice(0, 120)`.
/// `lastError ? ... : ""` memakai kebenaran JS, bukan `Option`: string kosong
/// juga falsy, jadi baris "Penyebab" hilang. String lain (termasuk `"0"` dan
/// `"false"`) tetap truthy seperti di JS.
pub(crate) fn gagal(last_error: Option<&str>) -> String {
    let sebab = match last_error {
        Some(err) if !err.is_empty() => format!("> Penyebab: `{}`\n", potong_120(err)),
        _ => String::new(),
    };
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n\
         > Videonya gagal diunduh.\n\
         {sebab}\
         > Pinterest biasanya memblokir unduhan dari luar — coba lagi nanti ya."
    )
}

/// `String(lastError).slice(0, 120)`.
///
/// Dihitung per karakter, bukan per byte, jadi untuk teks BMP hasilnya identik
/// dengan JS. Untuk karakter astral (emoji di luar BMP) JS bisa memotong pasangan
/// surrogate dan menghasilkan setengah karakter; Rust tidak punya surrogate, jadi
/// yang dipotong adalah karakter utuh.
fn potong_120(s: &str) -> String {
    s.chars().take(120).collect()
}

/// `terlaluBesar` (`pindl.js:23-25`) — batas inklusif: file tepat sebesar batas
/// masih boleh dikirim.
pub(crate) fn terlalu_besar(ukuran: u64) -> bool {
    ukuran > BATAS_BODY_VIDEO
}

/// Hasil `rencanaVideo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rencana {
    pub master_url: String,
    pub hls: bool,
}

/// `rencanaVideo` (`pindl.js:35-41`).
///
/// Dua keanehan JS yang WAJIB dipertahankan:
/// - `/720p|480p|360p|240p/g` case-sensitive, jadi `720P` tidak diganti.
/// - `.replace(".mp4", ".m3u8")` TANPA flag `g`, jadi hanya kemunculan PERTAMA.
///
/// `hls` dihitung dari `masterUrl` SETELAH dua penggantian itu, bukan dari
/// URL asal — `https://cdn/a.m3u8` sudah HLS walau asalnya `.mp4`.
pub(crate) fn rencana_video(url: Option<&str>) -> Rencana {
    let asal = url.unwrap_or("").to_string();
    let master_url = if asal.contains(".mp4") {
        ganti_kualitas(&asal).replacen(".mp4", ".m3u8", 1)
    } else {
        asal
    };
    let hls = master_url.contains(".m3u8");
    Rencana { master_url, hls }
}

/// `/720p|480p|360p|240p/g` — global, case-sensitive, kiri ke kanan.
fn ganti_kualitas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let rest = &s[i..];
        let ketemu = ["720p", "480p", "360p", "240p"]
            .iter()
            .find(|k| rest.starts_with(**k))
            .copied();
        match ketemu {
            Some(k) => {
                out.push_str("hls");
                i += k.len();
            }
            None => {
                let c = s[i..].chars().next().expect("i ada di batas karakter");
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

/// `hostPinterest` (`pindl.js:50-62`) — guard per label: `pin.it` langsung
/// benar, host lain harus punya satu label bernama `pinterest` dan pathname-nya
/// diawali `/pin/`.
pub(crate) fn host_pinterest(url: &str) -> bool {
    let Some(host) = host_dari(url) else {
        return false;
    };
    if host == "pin.it" {
        return true;
    }
    if !host.split('.').any(|label| label == "pinterest") {
        return false;
    }
    pathname_dari(url).is_some_and(|p| {
        p.len() >= 5 && p.as_bytes()[..5].eq_ignore_ascii_case(b"/pin/")
    })
}

/// `parsed.pathname` — bagian URL setelah host dan sebelum `?`/`#`, **termasuk**
/// garis miring pembuka (`/pin/123`, bukan `pin/123`). `None` kalau URL tidak
/// punya pathname sama sekali (mis. `https://pinterest.com`).
fn pathname_dari(url: &str) -> Option<&str> {
    let rest = url.trim().split_once("://")?.1;
    let miring = rest.find('/')?;
    Some(rest[miring..].split(['?', '#']).next().unwrap_or(""))
}

/// Varian video dan audio hasil parsing master HLS.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Varian {
    pub video: Option<String>,
    pub audio: Option<String>,
}

/// `pindl.js:137-165` — baca manifest master HLS dan pilih varian VIDEO dengan
/// `BANDWIDTH` tertinggi, plus URI audio pertama yang ditandai `TYPE=AUDIO`.
///
/// `master_url` adalah `masterUrl` hasil [`rencana_video`] — bukan teks
/// manifest. `baseUrl` di JS dihitung dari URL itu, bukan dari isi body.
///
/// Baris varian dibaca lewat `lines[i + 1]` TANPA cek batas, jadi manifest yang
/// diakhiri `#EXT-X-STREAM-INF` tidak menghasilkan varian sama sekali. Itu
/// faithfully mengikuti JS, bukan mengikuti spesifikasi HLS.
pub(crate) fn pilih_varian(master_url: &str, manifest: &str) -> Varian {
    let lines: Vec<&str> = manifest
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let base = base_url_dari(master_url);
    let mut varian = Varian::default();
    let mut last_bandwidth: u64 = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("#EXT-X-STREAM-INF") {
            let bw = angka_setelah(line, "BANDWIDTH=").unwrap_or(0);
            if bw > last_bandwidth {
                last_bandwidth = bw;
                // `u?.startsWith("http") ? u : baseUrl + u` — `u` boleh `undefined` kalau
// manifestnya berakhir tepat di baris `#EXT-X-STREAM-INF`, dan `baseUrl +
// undefined` di JS menghasilkan teks `"undefined"`, bukan string kosong.
let candidate = lines.get(i + 1).copied().unwrap_or("undefined");
                varian.video = Some(jadi_absolut(candidate, &base));
            }
        }
        if line.starts_with("#EXT-X-MEDIA")
            && line.contains("TYPE=AUDIO")
            && let Some(uri) = teks_dalam_kutip(line, "URI=")
        {
            varian.audio = Some(jadi_absolut(&uri, &base));
        }
    }
    varian
}

/// `masterUrl.substring(0, masterUrl.lastIndexOf("/") + 1)`. Kalau tidak ada
/// `/`, `lastIndexOf` = -1 dan `+ 1` = 0, jadi hasilnya string kosong.
fn base_url_dari(master_url: &str) -> String {
    match master_url.rfind('/') {
        Some(i) => master_url[..i + 1].to_string(),
        None => String::new(),
    }
}

/// `/BANDWIDTH=(\d+)/` — digit pertama sesudah label.
fn angka_setelah(line: &str, label: &str) -> Option<u64> {
    let after = &line[line.find(label)? + label.len()..];
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// `/URI="([^"]+)"/`.
fn teks_dalam_kutip(line: &str, label: &str) -> Option<String> {
    let after = &line[line.find(label)? + label.len()..];
    let rest = after.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// `u?.startsWith("http") ? u : baseUrl + u` — `undefined` jadi `baseUrl`.
fn jadi_absolut(u: &str, base: &str) -> String {
    if u.starts_with("http") {
        u.to_string()
    } else {
        format!("{base}{u}")
    }
}

/// `resolver.resolve("pinterest", { url })` — lapisan kapabilitas belum
/// di-port. `Err` = melempar, persis seperti backend yang gagal.
fn resolve_pinterest(_url: &str) -> Result<Vec<serde_json::Value>, ()> {
    Err(())
}

#[plugin(
    name = "pindl",
    alias = ["pinterestdl", "pindownload", "pintdl"],
    category = "download",
    description = "Download gambar/video dari Pinterest",
    usage = ".pindl <url>",
    example = ".pindl https://pin.it/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pindl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();

    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `pindl.js:93` — syarat `includes` hanya menyaring teks tanpa struktur URL
    // lebih dulu; `host_pinterest` yang menentukan.
    let mention_pinterest = url.contains("pinterest") || url.contains("pin.it");
    if !mention_pinterest || !host_pinterest(url) {
        return Reply::text(BAD_URL);
    }

    match resolve_pinterest(url) {
        // `pindl.js:118-309` — loop media: video lewat ffmpeg + HLS, gambar
        // dikirim langsung, GIF dikonversi dulu. Semuanya butuh `sock`/`fs`/
        // `child_process`, jadi tidak ada padanannya di API plugin.
        Ok(_media) => {
            // `pindl.js:118-309` — loop media: video lewat ffmpeg + HLS, gambar
            // dikirim langsung, GIF dikonversi dulu. Semuanya butuh `sock`/`fs`/
            // `child_process`, jadi tidak ada padanannya di API plugin dan
            // `sentCount` tidak pernah naik.
            Reply::text(gagal(None)).with_reaction("❌")
        }
        // `pindl.js:310-314` — `m.pushName` tidak ada di `Ctx`, jadi `User`.
        Err(()) => Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pindl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("pindl").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("pindl").unwrap();
        assert_eq!(p.meta.name, "pindl");
        assert_eq!(p.meta.alias, ["pinterestdl", "pindownload", "pintdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download gambar/video dari Pinterest");
        assert_eq!(p.meta.usage, ".pindl <url>");
        assert_eq!(p.meta.example, ".pindl https://pin.it/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pinterestdl", "pindownload", "pintdl"] {
            assert_eq!(find(a).unwrap().meta.name, "pindl", "alias {a}");
        }
    }

    #[test]
    fn a_missing_link_shows_the_card_without_reacting() {
        for text in ["", "   ", "\t"] {
            let r = run(text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaksi(&r), None, "input {text:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "pindl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("pindl").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), help("!"));
    }

    #[test]
    fn only_real_pinterest_hosts_pass_the_guard() {
        for good in [
            "https://pin.it/xxx",
            "https://PIN.IT/xxx",
            "https://www.pinterest.com/pin/123",
            "http://pinterest.com/pin/123",
            "https://m.pinterest.com/pin/123",
            "https://sub.pinterest.com/pin/1",
            // `host === "pin.it"` tidak memeriksa pathname sama sekali.
            "https://pin.it/PIN/123",
            "https://pinterest.com/pin/",
            "  https://pin.it/xxx  ",
        ] {
            assert!(host_pinterest(good), "harus lolos: {good}");
        }
        for bad in [
            "https://pinterest.com/other/123",
            "https://notpinterest.com/pin/1",
            "https://pin.it.evil.example/x",
            "https://evil.example/x?u=pin.it",
            "pinterest.com/pin/1",
            "ftp://pinterest.com/pin/1",
            "javascript:alert(1)",
            // Tanpa pathname ada: `/^\/pin\//` tidak bisa cocok.
            "https://pinterest.com",
            "",
        ] {
            assert!(!host_pinterest(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn the_string_guard_never_lets_anything_through_on_its_own() {
        // `url.includes("pinterest") || url.includes("pin.it")` terlalu longgar,
        // tapi `host_pinterest` menyiratkan keduanya — guard plugin tetap utuh.
        for text in [
            "https://example.com/x",
            "https://notpinterest.com/pin/1",
            "pinterest",
            "pin.it",
            "halo dunia",
        ] {
            let r = run(text);
            assert_eq!(text_of(&r), BAD_URL, "input {text}");
            assert_eq!(reaksi(&r), None, "input {text}");
        }
    }

    #[test]
    fn the_resolve_failure_lands_on_the_generic_catch_branch() {
        let r = run("https://pin.it/xxx");
        assert_eq!(text_of(&r), error_template(".", "pindl"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn rencana_video_keeps_both_js_replace_quirks() {
        let r = |u: &str| rencana_video(Some(u));

        // `/720p|…/g` case-sensitive.
        assert_eq!(r("https://cdn/v_720p.mp4").master_url, "https://cdn/v_hls.m3u8");
        assert_eq!(r("https://cdn/v_480p.mp4").master_url, "https://cdn/v_hls.m3u8");
        assert_eq!(r("https://cdn/v_360p.mp4").master_url, "https://cdn/v_hls.m3u8");
        assert_eq!(r("https://cdn/v_240p.mp4").master_url, "https://cdn/v_hls.m3u8");
        assert_eq!(r("https://cdn/v_1080p.mp4").master_url, "https://cdn/v_1080p.m3u8");
        assert_eq!(
            r("https://cdn/v_720P.mp4").master_url,
            "https://cdn/v_720P.m3u8",
            "huruf besar tidak diganti"
        );
        // Setiap kemunculan diganti (flag g), termasuk yang menempel pada huruf.
        assert_eq!(r("https://cdn/a720p.mp4").master_url, "https://cdn/ahls.m3u8");
        assert_eq!(r("https://cdn/a720p360p.mp4").master_url, "https://cdn/ahlshls.m3u8");

        // `.replace(".mp4", ".m3u8")` hanya kemunculan PERTAMA (tanpa flag g).
        assert_eq!(r("https://cdn/v720p.mp4.mp4").master_url, "https://cdn/vhls.m3u8.mp4");
        assert_eq!(r("https://cdn/v.mp4.m3u8").master_url, "https://cdn/v.m3u8.m3u8");
        assert_eq!(r("https://cdn/x/y720p.mp4").master_url, "https://cdn/x/yhls.m3u8");
        assert_eq!(r("https://cdn/v.mp4?x=720p").master_url, "https://cdn/v.m3u8?x=hls");
    }

    #[test]
    fn rencana_video_only_rewrites_urls_that_contain_mp4() {
        let r = |u: &str| rencana_video(Some(u));
        assert_eq!(r("https://cdn/720p").master_url, "https://cdn/720p");
        assert_eq!(r("https://cdn/a720p360p").master_url, "https://cdn/a720p360p");
        assert_eq!(r("").master_url, "");
        assert_eq!(rencana_video(None).master_url, "", "String(null ?? \"\")");
    }

    #[test]
    fn the_hls_flag_is_read_from_the_rewritten_url() {
        assert!(rencana_video(Some("https://cdn/v_720p.mp4")).hls);
        assert!(rencana_video(Some("https://cdn/v.m3u8")).hls);
        assert!(rencana_video(Some("https://cdn/v.mp4?x=720p")).hls);
        // `includes(".mp4")` dan `includes(".m3u8")` keduanya case-sensitive.
        // `.mp4` SELALU ditulis ulang jadi `.m3u8`, jadi `hls` selalu true —
        // tidak ada URL `.mp4` yang lolos sebagai file langsung.
        assert!(rencana_video(Some("https://cdn/v.mp4")).hls);
        assert!(!rencana_video(Some("https://cdn/720p")).hls);
        assert!(!rencana_video(Some("https://cdn/v.M3U8")).hls);
        // Kecuali: `.mp4` yang ditulis ulang memunculkan huruf kecil `.m3u8`.
        assert!(rencana_video(Some("https://cdn/v.mp4.M3U8")).hls);
    }

    #[test]
    fn the_size_limit_is_inclusive() {
        assert_eq!(BATAS_BODY_VIDEO, 67_108_864);
        assert!(!terlalu_besar(0));
        assert!(!terlalu_besar(BATAS_BODY_VIDEO - 1));
        assert!(!terlalu_besar(BATAS_BODY_VIDEO), "tepat sebesar batas masih boleh");
        assert!(terlalu_besar(BATAS_BODY_VIDEO + 1));
        assert!(terlalu_besar(100 * 1024 * 1024));
    }

    #[test]
    fn the_failure_message_drops_the_cause_line_when_it_is_empty() {
        assert_eq!(
            gagal(None),
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Videonya gagal diunduh.\n\
             > Pinterest biasanya memblokir unduhan dari luar — coba lagi nanti ya."
        );
        assert_eq!(
            gagal(Some("gagal sederhana")),
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Videonya gagal diunduh.\n\
             > Penyebab: `gagal sederhana`\n\
             > Pinterest biasanya memblokir unduhan dari luar — coba lagi nanti ya."
        );
    }

    #[test]
    fn the_cause_is_cut_to_120_characters() {
        let panjang = "E".repeat(200);
        let teks = gagal(Some(&panjang));
        let isi = teks
            .split("> Penyebab: `")
            .nth(1)
            .and_then(|s| s.split('`').next())
            .expect("baris Penyebab harus ada");
        assert_eq!(isi.chars().count(), 120);
        // String kosong juga falsy di JS, jadi barisnya hilang.
        assert_eq!(gagal(Some("")), gagal(None));
    }

    #[test]
    fn the_highest_bandwidth_variant_wins() {
        let master = "https://cdn/pin/master.m3u8";
        let manifest = "#EXTM3U\n\
             #EXT-X-MEDIA:TYPE=AUDIO,URI=\"https://cdn/a.m3u8\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=800000\nlow/index.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=2400000\nhigh/index.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1200000\nmid/index.m3u8\n";
        let v = pilih_varian(master, manifest);
        assert_eq!(v.video.as_deref(), Some("https://cdn/pin/high/index.m3u8"));
        assert_eq!(v.audio.as_deref(), Some("https://cdn/a.m3u8"));
    }

    #[test]
    fn a_relative_uri_is_joined_to_the_directory_of_the_master_url() {
        // `baseUrl` dihitung dari `masterUrl`, bukan dari teks manifest.
        let manifest = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000\nseg/v.m3u8\n";
        let v = pilih_varian("https://cdn/pin/master.m3u8", manifest);
        assert_eq!(v.video.as_deref(), Some("https://cdn/pin/seg/v.m3u8"));

        // `lastIndexOf("/")` = -1 -> `+ 1` = 0 -> base kosong.
        let tanpa = pilih_varian("master.m3u8", manifest);
        assert_eq!(tanpa.video.as_deref(), Some("seg/v.m3u8"));
    }

    #[test]
    fn an_absolute_uri_is_left_alone() {
        let v = pilih_varian(
            "https://cdn/pin/master.m3u8",
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nhttps://cdn/x.m3u8\n",
        );
        assert_eq!(v.video.as_deref(), Some("https://cdn/x.m3u8"));
        // Yang diuji `startsWith("http")` tanpa slash ganda, jadi "httpX" juga lolos.
        let ganjil = pilih_varian(
            "https://cdn/pin/master.m3u8",
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nhttpX/v.m3u8\n",
        );
        assert_eq!(ganjil.video.as_deref(), Some("httpX/v.m3u8"));
    }

    #[test]
    fn a_manifest_ending_on_the_variant_line_yields_the_js_undefined_url() {
        // JS membaca `lines[i + 1]` tanpa cek batas; `undefined?.startsWith`
        // falsy, jadi `baseUrl + undefined` -> teks `"undefined"`.
        let master = "https://cdn/pin/master.m3u8";
        assert_eq!(
            pilih_varian(master, "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=9999\n").video.as_deref(),
            Some("https://cdn/pin/undefined")
        );
        assert_eq!(pilih_varian(master, ""), Varian::default());
        assert_eq!(pilih_varian(master, "   \n\n  \n"), Varian::default());
    }

    #[test]
    fn a_line_without_bandwidth_counts_as_zero() {
        // `parseInt` tidak menemukan digit -> `0`, dan `0 > 0` salah, jadi
        // varian tanpa BANDWIDTH tidak pernah tercatat. Yang menang adalah
        // `#EXT-X-STREAM-INF` kedua yang punya bandwidth.
        let v = pilih_varian(
            "https://cdn/pin/master.m3u8",
            "#EXTM3U\n#EXT-X-STREAM-INF\npertama.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=1\nkedua.m3u8\n",
        );
        assert_eq!(v.video.as_deref(), Some("https://cdn/pin/kedua.m3u8"));
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://pin.it/xxx").mutations().is_empty());
    }
}