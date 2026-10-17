//! `plugins/search/play2.js` — kartu music player interaktif (AIRICH).
//!
//! Hampir seluruh plugin ini bertumpu pada kapabilitas yang tidak ada di API
//! plugin: `ytmusic-api`, `yt-search`, `sharp`, `child_process` (ffmpeg), dan
//! `crypto.createDecipheriv` untuk metadata `savetube.vip`, plus pengiriman
//! `botForwardedMessage` AIRICH lewat `sock.relayMessage`. Tidak satu pun punya
//! padanan, jadi jalur setelah `m.react('🎧')` selalu berakhir di `catch`.
//!
//! Yang dipor penuh karena murni dan bisa diuji: parser LRC, konversi lirik
//! polos menjadi lirik bertime, normalisasi, pemformat durasi, `secondsFromTimestamp`,
//! `escapeHtml`, dan kartu pakai.

#![allow(dead_code)] // jalur playback tidak terjangkau tanpa kapabilitas di atas

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `play2.js:954` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `{prefix}play2 <judul lagu / URL YouTube>`\n\nKartu player interaktif dengan lagu full + lirik sinkron.\n\nContoh: `{prefix}play2 another love tom odell`"
    )
}

/// `play2.js:1104` — pesan `catch`.
pub(crate) fn gagal(message: &str) -> String {
    format!("❌ Gagal memproses lagu.\n\n> {message}")
}

/// `play2.js:980` — pesan yang dilempar saat pencarian YT Music tidak
/// menghasilkan entri. Ini pesan pertama yang muncul di port ini, karena
/// `ytmusic-api` tidak ada.
pub(crate) const LAGU_TIDAK_DITEMUKAN: &str = "Lagu tidak ditemukan di YT Music";

/// `play2.js:1002` — `yts` tidak menemukan video untuk URL yang diberikan.
pub(crate) const VIDEO_TIDAK_DITEMUKAN: &str = "Video tidak ditemukan";

/// `play2.js:281-285` — regex ID video; `None` = "URL YouTube tidak valid".
pub(crate) const URL_TIDAK_VALID: &str = "URL YouTube tidak valid";

/// `play2.js:295` / `:308` / `:327` / `:345`.
pub(crate) const CDN_TIDAK_TERSEDIA: &str = "CDN tidak tersedia";
pub(crate) const METADATA_KOSONG: &str = "Metadata kosong";
pub(crate) const DECRYPT_GAGAL: &str = "Decrypt metadata gagal";
pub(crate) const DOWNLOAD_GAGAL: &str = "Download gagal";

/// `play2.js:104` — `parseLrcTimestamp`. Menolak `seconds >= 60`.
pub(crate) fn parse_lrc_timestamp(menit: f64, detik: f64, pecahan: Option<&str>) -> Option<f64> {
    if !menit.is_finite() || !detik.is_finite() || detik < 0.0 || detik >= 60.0 {
        return None;
    }
    let milidetik = match pecahan {
        None => 0.0,
        Some(t) if t.is_empty() => 0.0,
        Some(t) => {
            let angka: f64 = t.parse().unwrap_or(0.0);
            match t.chars().count() {
                1 => angka * 100.0,
                2 => angka * 10.0,
                _ => t.chars().take(3).collect::<String>().parse().unwrap_or(0.0),
            }
        }
    };
    Some(menit * 60.0 + detik + milidetik / 1000.0)
}

/// Satu baris lirik bertime.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Baris {
    pub time: f64,
    pub text: String,
}

/// `parseSyncedLyrics` (`play2.js:134-173`).
///
/// Baris tanpa timestamp atau tanpa teks dilewati; hasil diurutkan naik dan
/// duplikat (waktu SELISIH < 1 ms dan teks sama) dibuang.
pub(crate) fn parse_synced_lyrics(lrc: &str) -> Vec<Baris> {
    let mut result: Vec<Baris> = Vec::new();
    for raw in lrc.split('\n') {
        let raw = raw.trim_end_matches('\r');
        if raw.trim().is_empty() {
            continue;
        }
        let mut stamps: Vec<(f64, f64, String)> = Vec::new();
        // `rawLine.replace(timestampRegex, "")` membuang SEMUA timestamp,
        // termasuk yang muncul setelah teks — jadi sisanya dikumpulkan, bukan
        // hanya potongan setelah bracket pertama.
        let mut sisa = String::new();
        let mut cursor = 0;
        while let Some(open) = raw[cursor..].find('[') {
            let open = cursor + open;
            let Some(close_rel) = raw[open..].find(']') else {
                sisa.push_str(&raw[cursor..]);
                break;
            };
            let close = open + close_rel;
            sisa.push_str(&raw[cursor..open]);
            match pecah_timestamp(&raw[open + 1..close]) {
                Some((m, d, f)) => {
                    stamps.push((m, d, f));
                    cursor = close + 1;
                }
                // Kurung siku bukan timestamp: ikut dibuang oleh regex di JS.
                None => cursor = close + 1,
            }
        }
        if cursor < raw.len() {
            sisa.push_str(&raw[cursor..]);
        }
        if stamps.is_empty() {
            continue;
        }
        let text = sisa.trim().to_string();
        if text.is_empty() {
            continue;
        }
        for (m, d, f) in stamps {
            if let Some(time) = parse_lrc_timestamp(m, d, Some(f.as_str())) {
                if time.is_finite() && time >= 0.0 {
                    result.push(Baris {
                        time,
                        text: text.clone(),
                    });
                }
            }
        }
    }
    result.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    let mut cleaned: Vec<Baris> = Vec::new();
    for item in result {
        if let Some(last) = cleaned.last()
            && (last.time - item.time).abs() < 0.001
            && last.text == item.text
        {
            continue;
        }
        cleaned.push(item);
    }
    cleaned
}

/// Isi `[..]` — `(\d{1,3}):(\d{2})(?:[.:](\d{1,3}))?`.
fn pecah_timestamp(isi: &str) -> Option<(f64, f64, String)> {
    let (menit, sisanya) = isi.split_once(':')?;
    if menit.is_empty() || menit.len() > 3 || !menit.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (detik, pecahan) = match sisanya.split_once([':', '.']) {
        Some((d, f)) => (d, f),
        None => (sisanya, ""),
    };
    if detik.len() != 2 || !detik.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !pecahan.is_empty()
        && (pecahan.len() > 3 || !pecahan.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some((menit.parse().ok()?, detik.parse().ok()?, pecahan.to_string()))
}

/// `plainLyricsToSynced` (`play2.js:179-199`) — jarak antar baris 2..8 detik.
pub(crate) fn plain_lyrics_to_synced(lyrics: &str, durasi: f64) -> Vec<Baris> {
    if lyrics.is_empty() {
        return Vec::new();
    }
    let lines: Vec<&str> = lyrics
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let mut interval = 5.0;
    if durasi.is_finite() && durasi > 0.0 && lines.len() > 1 {
        interval = (durasi / lines.len() as f64).clamp(2.0, 8.0);
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(i, text)| Baris {
            time: i as f64 * interval,
            text: text.to_string(),
        })
        .collect()
}

/// `normalizeLyrics` (`play2.js:201-215`) — buang waktu bukan angka dan teks
/// kosong setelah `trim()`, lalu urutkan.
pub(crate) fn normalize_lyrics(mut lyrics: Vec<Baris>) -> Vec<Baris> {
    lyrics.retain(|b| b.time.is_finite() && !b.text.trim().is_empty());
    for b in &mut lyrics {
        b.text = b.text.trim().to_string();
    }
    lyrics.retain(|b| !b.text.is_empty());
    lyrics.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    lyrics
}

/// `escapeHtml` (`play2.js:235-242`) — urutan penggantian penting: `&` dulu.
pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

/// `escapeAttr` = `escapeHtml` (`play2.js:244-246`).
pub(crate) fn escape_attr(text: &str) -> String {
    escape_html(text)
}

/// `secondsFromTimestamp` (`play2.js:248-257`) — 2 atau 3 bagian; selain itu 0.
pub(crate) fn seconds_from_timestamp(timestamp: &str) -> f64 {
    if timestamp.is_empty() {
        return 0.0;
    }
    let parts: Vec<f64> = timestamp
        .split(':')
        .map(|p| p.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    if parts.iter().any(|p| p.is_nan()) {
        return 0.0;
    }
    match parts.len() {
        2 => parts[0] * 60.0 + parts[1],
        3 => parts[0] * 3600.0 + parts[1] * 60.0 + parts[2],
        _ => 0.0,
    }
}

/// `formatDuration` (`play2.js:260-275`) — jam muncul hanya di atas nol.
pub(crate) fn format_duration(detik: f64) -> String {
    if !detik.is_finite() || detik <= 0.0 {
        return "0:00".to_string();
    }
    let total = detik.floor() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{:02}:{:02}", m, s)
    } else {
        format!("{m}:{s:02}")
    }
}

/// `/youtube\.com|youtu\.be/i.test(text)` (`play2.js:970`) — regex tanpa
/// flag `i`? Ada: `i` di sana, jadi pencocokan tidak membedakan huruf.
pub(crate) fn looks_like_youtube_url(text: &str) -> bool {
    let low = text.to_lowercase();
    low.contains("youtube.com") || low.contains("youtu.be")
}

/// `/[a-zA-Z0-9_-]{11}/` dari URL (`play2.js:1011`) — 11 karakter persis.
pub(crate) fn video_id_dari(url: &str) -> Option<String> {
    let kandidat: Vec<&str> = url
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .filter(|s| s.len() >= 11)
        .collect();
    kandidat
        .first()
        .map(|s| (*s)[..11].to_string())
}

/// `ytmusic-api` tidak ada di crate ini; pencarian selalu gagal dengan pesan
/// yang sama seperti saat YT Music tidak mengembalikan entri.
fn yt_music_cari(_query: &str) -> Result<serde_json::Value, String> {
    Err(LAGU_TIDAK_DITEMUKAN.to_string())
}

#[plugin(
    name = "play2",
    alias = ["spotifywebui", "playweb"],
    category = "search",
    description = "Kirim kartu music player interaktif (AIRICH) dengan lagu full + lirik sinkron",
    usage = ".play2 <judul lagu / URL YouTube>",
    example = ".play2 another love tom odell",
    cooldown = 15,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn play2(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `await m.react('🎧')` di `:958`; `catch` menggantinya dengan ❌.
    let outcome: Result<serde_json::Value, String> = if looks_like_youtube_url(text) {
        // `yts(ytUrl)` (`yt-search`) tidak ada; pesan kedua yang mungkin muncul.
        Err(VIDEO_TIDAK_DITEMUKAN.to_string())
    } else {
        yt_music_cari(text)
    };

    match outcome {
        Err(message) => Reply::text(gagal(&message)).with_reaction("❌"),
        Ok(_) => {
            // LRCLIB + `sharp` (thumbnail) + ffmpeg + `savetube` + kartu
            // AIRICH: tidak ada padanannya di API plugin.
            Reply::text(gagal(CDN_TIDAK_TERSEDIA)).with_reaction("❌")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "play2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("play2").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("play2").unwrap();
        assert_eq!(p.meta.name, "play2");
        assert_eq!(p.meta.alias, ["spotifywebui", "playweb"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Kirim kartu music player interaktif (AIRICH) dengan lagu full + lirik sinkron"
        );
        assert_eq!(p.meta.usage, ".play2 <judul lagu / URL YouTube>");
        assert_eq!(p.meta.example, ".play2 another love tom odell");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["spotifywebui", "playweb"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "play2");
        }
    }

    #[test]
    fn an_empty_query_shows_the_help_card() {
        for text in ["", "  ", "\t\n"] {
            let t = text_of(&run(text));
            assert!(t.starts_with("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*"), "input {text:?}");
            assert!(t.contains("`.play2 <judul lagu / URL YouTube>`"), "{t}");
            assert!(t.ends_with("Contoh: `.play2 another love tom odell`"), "{t}");
        }
        assert!(help("!").contains("`!play2 another love tom odell`"));
    }

    #[test]
    fn a_title_search_reports_the_yt_music_miss() {
        let r = run("another love tom odell");
        assert_eq!(
            text_of(&r),
            "❌ Gagal memproses lagu.\n\n> Lagu tidak ditemukan di YT Music"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn a_url_input_reports_the_yt_search_miss() {
        let r = run("https://www.youtube.com/watch?v=abc123XYZ_-");
        assert_eq!(
            text_of(&r),
            "❌ Gagal memproses lagu.\n\n> Video tidak ditemukan"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn odd_and_very_long_inputs_do_not_panic() {
        for input in ["<b>", "a|b", "🎧", "'\"", "a\u{202e}b", "%00"] {
            assert!(text_of(&run(input)).starts_with("❌ Gagal memproses lagu."), "input {input:?}");
        }
        let panjang = "lagu ".repeat(3000);
        assert!(text_of(&run(&panjang)).contains("Lagu tidak ditemukan"));
    }

    #[test]
    fn a_lrc_line_yields_one_entry_per_timestamp() {
        let lrc = "[00:12.00]Halo\n[00:15.50]Dunia";
        let bars = parse_synced_lyrics(lrc);
        assert_eq!(bars.len(), 2);
        assert!((bars[0].time - 12.0).abs() < 1e-9);
        assert!((bars[1].time - 15.5).abs() < 1e-9);
        assert_eq!(bars[0].text, "Halo");
        assert_eq!(bars[1].text, "Dunia");
    }

    #[test]
    fn two_timestamps_on_one_line_become_two_entries() {
        let bars = parse_synced_lyrics("[00:01.00][00:03.00]Sama");
        assert_eq!(bars.len(), 2);
        assert!((bars[0].time - 1.0).abs() < 1e-9);
        assert!((bars[1].time - 3.0).abs() < 1e-9);
        assert!(bars.iter().all(|b| b.text == "Sama"));
    }

    #[test]
    fn lrc_lines_without_timestamps_or_text_are_skipped() {
        let lrc = "bukan lirik\n\n[00:01.00]\n   \n[00:02.00]Ada\n[zz:yy]Salah";
        let bars = parse_synced_lyrics(lrc);
        assert_eq!(bars.len(), 1);
        assert!((bars[0].time - 2.0).abs() < 1e-9);
        assert_eq!(bars[0].text, "Ada");
    }

    #[test]
    fn entries_come_out_sorted_by_time() {
        let bars = parse_synced_lyrics("[00:30.00]Akhir\n[00:10.00]Awal");
        assert!((bars[0].time - 10.0).abs() < 1e-9);
        assert!((bars[1].time - 30.0).abs() < 1e-9);
    }

    #[test]
    fn identical_lines_less_than_a_millisecond_apart_are_deduplicated() {
        let bars = parse_synced_lyrics("[00:10.000]Sama\n[00:10.0005]Sama");
        assert_eq!(bars.len(), 1);
        let bars = parse_synced_lyrics("[00:10.000]Sama\n[00:10.002]Sama");
        assert_eq!(bars.len(), 2, "selisih >= 1 ms tetap dua baris");
        let bars = parse_synced_lyrics("[00:10.00]Satu\n[00:10.00]Dua");
        assert_eq!(bars.len(), 2, "teks berbeda tidak digabung");
    }

    #[test]
    fn a_seconds_value_of_sixty_or_more_is_rejected() {
        assert!(parse_lrc_timestamp(0.0, 60.0, None).is_none());
        assert!(parse_lrc_timestamp(0.0, 59.0, None).is_some());
        assert!(parse_lrc_timestamp(0.0, -1.0, None).is_none());
        assert!(parse_lrc_timestamp(f64::NAN, 1.0, None).is_none());
        assert!((parse_lrc_timestamp(1.0, 30.0, None).unwrap() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn fraction_length_decides_the_millisecond_scale() {
        let t = |f: Option<&str>| parse_lrc_timestamp(0.0, 1.0, f).unwrap();
        assert!((t(None) - 1.0).abs() < 1e-9);
        assert!((t(Some("")) - 1.0).abs() < 1e-9, "string kosong -> 0");
        assert!((t(Some("5")) - 1.5).abs() < 1e-9, "1 digit -> *100");
        assert!((t(Some("05")) - 1.05).abs() < 1e-9, "2 digit -> *10");
        assert!((t(Some("005")) - 1.005).abs() < 1e-9, "3 digit -> apa adanya");
        assert!((t(Some("1234")) - 1.123).abs() < 1e-9, "digit ke-4 dipotong");
    }

    #[test]
    fn plain_lyrics_get_an_interval_between_two_and_eight_seconds() {
        // 4 baris, durasi 16 detik -> interval 4.
        let bars = plain_lyrics_to_synced("a\nb\nc\nd", 16.0);
        assert_eq!(bars.len(), 4);
        assert_eq!(bars[0].time, 0.0);
        assert_eq!(bars[3].time, 12.0);
        // Durasi 10 detik / 4 baris = 2.5, tetap di dalam rentang.
        let bars = plain_lyrics_to_synced("a\nb\nc\nd", 10.0);
        assert_eq!(bars[3].time, 7.5);
        // Durasi sangat besar -> clamp ke 8.
        let bars = plain_lyrics_to_synced("a\nb\nc\nd", 1000.0);
        assert_eq!(bars[3].time, 24.0);
        // Durasi 0 -> default 5.
        let bars = plain_lyrics_to_synced("a\nb", 0.0);
        assert_eq!(bars[1].time, 5.0);
        // Satu baris saja -> interval default, tapi tidak terlihat.
        assert_eq!(plain_lyrics_to_synced("tunggal", 0.0).len(), 1);
        assert!(plain_lyrics_to_synced("", 10.0).is_empty());
        assert!(plain_lyrics_to_synced("   \n  ", 10.0).is_empty());
    }

    #[test]
    fn normalization_drops_bad_times_and_blank_text() {
        let bars = normalize_lyrics(vec![
            Baris {
                time: f64::NAN,
                text: "bukan waktu".into(),
            },
            Baris {
                time: 5.0,
                text: "   ".into(),
            },
            Baris {
                time: 9.0,
                text: "  spac  ".into(),
            },
            Baris {
                time: 1.0,
                text: "pertama".into(),
            },
        ]);
        assert_eq!(bars.len(), 2);
        assert!((bars[0].time - 1.0).abs() < 1e-9);
        assert_eq!(bars[1].text, "spac");
    }

    #[test]
    fn escaping_replaces_the_ampersand_first() {
        assert_eq!(
            escape_html(r#"<b>&"'</b>"#),
            "&lt;b&gt;&amp;&quot;&#039;&lt;/b&gt;"
        );
        assert_eq!(escape_html("&lt;"), "&amp;lt;", "tidak dua kali escape");
        assert_eq!(escape_html(""), "");
        assert_eq!(escape_attr("<a>"), "&lt;a&gt;");
    }

    #[test]
    fn timestamps_convert_to_seconds_for_two_or_three_parts_only() {
        assert!((seconds_from_timestamp("3:45") - 225.0).abs() < 1e-9);
        assert!((seconds_from_timestamp("1:02:03") - 3723.0).abs() < 1e-9);
        assert_eq!(seconds_from_timestamp(""), 0.0);
        assert_eq!(seconds_from_timestamp("45"), 0.0, "satu bagian -> 0");
        assert_eq!(seconds_from_timestamp("1:2:3:4"), 0.0, "empat bagian -> 0");
        assert_eq!(seconds_from_timestamp("a:b"), 0.0, "bukan angka -> 0");
    }

    #[test]
    fn duration_formatting_pads_seconds_and_drops_a_zero_hour() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(-5.0), "0:00");
        assert_eq!(format_duration(f64::NAN), "0:00");
        assert_eq!(format_duration(9.0), "0:09");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(3600.0), "1:00:00");
        assert_eq!(format_duration(3725.0), "1:02:05");
        assert_eq!(format_duration(3661.9), "1:01:01", "detik dip-floor, bukan dibulatkan");
    }

    #[test]
    fn the_youtube_url_check_is_case_insensitive() {
        assert!(looks_like_youtube_url("https://YOUTUBE.COM/watch?v=x"));
        assert!(looks_like_youtube_url("https://youtu.be/x"));
        assert!(!looks_like_youtube_url("another love tom odell"));
        assert!(!looks_like_youtube_url(""));
    }

    #[test]
    fn the_video_id_takes_exactly_eleven_characters() {
        assert_eq!(
            video_id_dari("https://www.youtube.com/watch?v=abc123XYZ_-"),
            Some("abc123XYZ_-".to_string())
        );
        assert_eq!(video_id_dari("https://youtu.be/abcdefghijk"), Some("abcdefghijk".to_string()));
        assert_eq!(video_id_dari("pendek"), None);
        assert_eq!(video_id_dari(""), None);
    }

    #[test]
    fn the_javascript_error_messages_are_kept() {
        assert_eq!(gagal("X"), "❌ Gagal memproses lagu.\n\n> X");
        assert_eq!(URL_TIDAK_VALID, "URL YouTube tidak valid");
        assert_eq!(CDN_TIDAK_TERSEDIA, "CDN tidak tersedia");
        assert_eq!(METADATA_KOSONG, "Metadata kosong");
        assert_eq!(DECRYPT_GAGAL, "Decrypt metadata gagal");
        assert_eq!(DOWNLOAD_GAGAL, "Download gagal");
    }
}