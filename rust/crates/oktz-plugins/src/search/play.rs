//! `plugins/search/play.js` — putar musik dari YouTube.
//!
//! `ponytail:` `yts` (`yt-search`, paket npm) tidak ada padanannya di crate
//! ini dan tidak ada modul `youtube`/`yt` di `oktz-scraper`, jadi pencarian
//! selalu gagal — persis `download::pindl` dengan resolver yang belum di-port.
//! `sock.sendPreview` juga tidak ada padanannya di `Reply`.
//!
//! Yang dipor: guard, pemformat views, susunan kartu `NOW PLAYING`, dan pesan
//! gagalnya.

#![allow(dead_code)] // jalur pencarian tidak terjangkau tanpa yt-search

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `play.js:26` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!("🎵 *ᴘʟᴀʏ*\n\n> Contoh:\n`{prefix}play komang`")
}

/// `play.js:91` — pesan kegagalan, tanpa template `te()`.
pub(crate) const GAGAL: &str =
    "Wahhh, fitur putar musiknya lagi ada kendala kak, coba lagi nanti yak, jangan spam";

/// `play.js:16-21` — `formatViews`.
///
/// `n` datang dari `yt-search` sebagai angka; `!n` menangkap `0`, `NaN`, dan
/// `undefined`, semuanya jadi `"0"`.
pub(crate) fn format_views(n: f64) -> String {
    if !n.is_finite() || n == 0.0 {
        return "0".to_string();
    }
    if n >= 1e6 {
        return format!("{}M", satu_desimal(n / 1e6));
    }
    if n >= 1e3 {
        return format!("{}K", satu_desimal(n / 1e3));
    }
    format!("{n}")
}

/// `Number.prototype.toFixed(1)` — pembulatan ke atas untuk tepat `.5`.
fn satu_desimal(n: f64) -> String {
    let kali_seri = n * 10.0;
    let bawah = kali_seri.floor();
    let naik = if kali_seri - bawah >= 0.5 { 1.0 } else { 0.0 };
    format!("{:.1}", (bawah + naik) / 10.0)
}

/// `yts(query)` (`play.js:31`) — paket npm `yt-search`, belum di-port.
fn yts(_query: &str) -> Result<serde_json::Value, ()> {
    Err(())
}

/// `play.js:42-55` — kartu `NOW PLAYING`. `desc` = deskripsi video.
pub(crate) fn now_playing(
    title: &str,
    author: &str,
    timestamp: &str,
    views: f64,
    ago: &str,
    video_id: &str,
    description: &str,
    url: &str,
) -> String {
    let mut info = String::from("🎵 *NOW PLAYING*\n\n");
    info.push_str(&format!("📌 *Judul:* {title}\n\n"));
    info.push_str("*DETAIL*\n");
    info.push_str(&format!("👤 Channel: *{author}*\n"));
    info.push_str(&format!("⏱️ Durasi: *{timestamp}*\n"));
    info.push_str(&format!("👀 Views: *{}*\n", format_views(views)));
    info.push_str(&format!("📅 Upload: *{ago}*\n"));
    info.push_str(&format!("🆔 ID: `{video_id}`\n\n"));
    // `if (video.description)` — string kosong falsy, bloknya dilewati.
    if !description.is_empty() {
        let chars: Vec<char> = description.chars().collect();
        let desc: String = chars.iter().take(150).collect();
        let desc = desc.replace('\n', " ");
        info.push_str(&format!(
            "*Deskripsi:*\n_{desc}{}_\n\n",
            if chars.len() > 150 { "..." } else { "" }
        ));
    }
    info.push_str(&format!("🔗 {url}\n\n"));
    info.push_str("_⏳ mengirim audio, harap tunggu..._");
    info
}

#[plugin(
    name = "play",
    alias = ["playaudio"],
    category = "search",
    description = "Putar musik dari YouTube",
    usage = ".play <query>",
    example = ".play komang",
    cooldown = 15,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    // Tidak ada di pluginConfig JS.
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn play(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🕐")` di `:28` sudah jalan; `catch` menggantinya dengan 😭.
    match yts(query) {
        Ok(_search) => {
            // `search.videos[0]` → `api.nexray.web.id/api/downloader/ytmp3` →
            // `sock.sendPreview` → `sock.sendMessage` audio. Preview tidak
            // ada padanannya di `Reply`, jadi tidak ada yang terkirim.
            Reply::text(GAGAL).with_reaction("😭")
        }
        Err(()) => Reply::text(GAGAL).with_reaction("😭"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "play".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("play").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("play").unwrap();
        assert_eq!(p.meta.name, "play");
        assert_eq!(p.meta.alias, ["playaudio"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Putar musik dari YouTube");
        assert_eq!(p.meta.usage, ".play <query>");
        assert_eq!(p.meta.example, ".play komang");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        // Tidak ada di pluginConfig JS.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("playaudio").unwrap().meta.name,
            "play"
        );
    }

    #[test]
    fn an_empty_query_shows_the_help_card() {
        for text in ["", "  ", "\t\n"] {
            assert_eq!(
                text_of(&run(text)),
                "🎵 *ᴘʟᴀʏ*\n\n> Contoh:\n`.play komang`",
                "input {text:?}"
            );
        }
        assert!(help("!").contains("`!play komang`"));
        assert_eq!(reaction_of(&run("")), None);
    }

    #[test]
    fn a_query_falls_into_the_catch_branch_while_yt_search_is_missing() {
        let r = run("komang");
        assert_eq!(text_of(&r), GAGAL);
        assert_eq!(reaction_of(&r), Some("😭"), "🕐 digantikan 😭");
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎵", "'\"", "a\u{202e}b", &"x".repeat(5000)] {
            assert!(!text_of(&run(input)).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn view_counts_switch_to_k_and_m_at_the_right_threshold() {
        assert_eq!(format_views(0.0), "0");
        assert_eq!(format_views(999.0), "999");
        assert_eq!(format_views(1000.0), "1.0K");
        assert_eq!(format_views(1500.0), "1.5K");
        assert_eq!(format_views(999_999.0), "1000.0K", "di bawah 1e6, belum M");
        assert_eq!(format_views(1_000_000.0), "1.0M");
        assert_eq!(format_views(2_450_000.0), "2.5M", "2.45 → 2.5, bulat ke atas");
        assert_eq!(format_views(12_345_678.0), "12.3M");
        assert_eq!(format_views(f64::NAN), "0");
        assert_eq!(format_views(f64::INFINITY), "0");
    }

    #[test]
    fn the_now_playing_card_carries_every_line_the_javascript_writes() {
        let t = now_playing(
            "Komang",
            "Vidiomalay",
            "3:45",
            12_345_678.0,
            "5 tahun lalu",
            "abc123XYZ_-",
            "Deskripsi singkat",
            "https://youtu.be/abc123XYZ_-",
        );
        assert!(t.starts_with("🎵 *NOW PLAYING*\n\n📌 *Judul:* Komang\n\n*DETAIL*\n"), "{t}");
        assert!(t.contains("👤 Channel: *Vidiomalay*\n"), "{t}");
        assert!(t.contains("👀 Views: *12.3M*\n"), "{t}");
        assert!(t.contains("🆔 ID: `abc123XYZ_-`\n\n"), "{t}");
        assert!(t.contains("*Deskripsi:*\n_Deskripsi singkat_\n\n"), "{t}");
        assert!(t.ends_with("_⏳ mengirim audio, harap tunggu..._"), "{t}");
    }

    #[test]
    fn an_empty_description_is_skipped_entirely() {
        let t = now_playing("T", "A", "1:00", 10.0, "baru", "id", "", "u");
        assert!(!t.contains("Deskripsi"), "{t}");
    }

    #[test]
    fn a_long_description_is_cut_at_150_with_its_ellipsis() {
        let desc = "a\nb".repeat(100);
        let t = now_playing("T", "A", "1:00", 10.0, "baru", "id", &desc, "u");
        let baris = t
            .lines()
            .find(|l| l.starts_with("_"))
            .expect("ada baris deskripsi");
        assert!(baris.ends_with("..._"), "{baris}");
        assert!(!baris.contains('\n'));
        // `substring(0, 150)` mengambil 150 karakter; `\n`-nya jadi spasi
        // sesudahnya, jadi panjangnya tetap 150.
        let isi = baris.trim_matches('_').trim_end_matches("...");
        assert_eq!(isi.chars().count(), 150, "{baris}");
        assert!(!baris.contains('\n'), "deskripsi satu baris: {baris}");
    }

    #[test]
    fn a_description_of_exactly_150_characters_gets_no_ellipsis() {
        let desc = "z".repeat(150);
        let t = now_playing("T", "A", "1:00", 10.0, "baru", "id", &desc, "u");
        assert!(t.contains(&format!("_{desc}_\n")), "{t}");
    }
}