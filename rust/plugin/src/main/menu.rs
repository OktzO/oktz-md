//! `plugins/main/menu.js` — menu utama.
//!
//! `ponytail:` yang belum sama dengan JS dan alasannya:
//! - Kartu interactive (gambar header, single_select, bottom_sheet, saluran,
//!   cuaca) tidak bisa dikirim: `Reply::Interactive` hanya membawa body+tombol,
//!   tanpa header media/nativeFlow. Yang dikirim teks `buildMenuText` — teks
//!   yang sama yang dipakai body kartu varian 2 di JS.
//! - Riwayat command teratas (`database/main/stats.json`) belum dicatat bot
//!   Rust, jadi blok "History" tidak ada.
//! - Mode global (`config.mode`) tidak ada di `Ctx`; baris Mode memakai
//!   `PUBLIC` seperti config produksi.
//! - Daftar kategori memakai mode `md` (default grup JS): kategori panel,
//!   pushkontak, store disembunyikan.

use std::collections::BTreeMap;

use oktz_plugin_api::{Ctx, Reply, all_plugins, plugin};

use crate::common::{jakarta_parts, now_ms, pad2};

/// Urutan kategori `menu.js:112-140` (sama dengan `buildMenuText`).
const URUTAN: &[&str] = &[
    "owner",
    "main",
    "utility",
    "tools",
    "fun",
    "game",
    "download",
    "search",
    "sticker",
    "media",
    "ai",
    "group",
    "religi",
    "info",
    "cek",
    "economy",
    "user",
    "canvas",
    "random",
    "premium",
    "ephoto",
    "jpm",
    "pushkontak",
    "panel",
    "store",
];

/// Kategori yang disembunyikan mode `md` (`menu.js:146`).
const DISEMBUNYI_MD: &[&str] = &["panel", "pushkontak", "store"];

fn emoji(kategori: &str) -> &'static str {
    match kategori {
        "owner" => "👑",
        "main" => "🏠",
        "utility" => "🔧",
        "tools" => "🛠️",
        "fun" => "🎮",
        "game" => "🎯",
        "download" => "📥",
        "search" => "🔍",
        "sticker" => "🖼️",
        "media" => "🎬",
        "ai" => "🤖",
        "group" => "👥",
        "religi" => "☪️",
        "info" => "ℹ️",
        "cek" => "📁",
        "economy" => "💰",
        "user" => "📊",
        "canvas" => "🎨",
        "random" => "🎲",
        "premium" => "💎",
        "ephoto" => "🖌️",
        "jpm" => "📨",
        "pushkontak" => "📱",
        "panel" => "🖥️",
        "store" => "🏪",
        _ => "📁",
    }
}

/// `getTimeGreeting()` (`formatter.js:508-515`), jam Asia/Jakarta.
fn sapa() -> &'static str {
    match jakarta_parts(now_ms()).3 {
        4..=9 => "Selamat Pagi 🌅",
        10..=14 => "Selamat Siang ☀️",
        15..=17 => "Selamat Sore 🌇",
        _ => "Selamat Malam 🌙",
    }
}

/// `formatUptime` (`stats.js:28-42`).
fn uptime(detik: u64) -> String {
    let hari = detik / 86_400;
    let jam = (detik % 86_400) / 3600;
    let menit = (detik % 3600) / 60;
    let sisa = detik % 60;

    let mut bagian = Vec::new();
    if hari > 0 {
        bagian.push(format!("{hari}d"));
    }
    if jam > 0 {
        bagian.push(format!("{jam}h"));
    }
    if menit > 0 {
        bagian.push(format!("{menit}m"));
    }
    if sisa > 0 || bagian.is_empty() {
        bagian.push(format!("{sisa}s"));
    }
    bagian.join(" ")
}

/// Kategori yang punya plugin, urut seperti JS. `owner` hanya untuk owner.
pub(crate) fn kategori(ctx: &Ctx) -> Vec<(&'static str, usize)> {
    let mut hitung: BTreeMap<&'static str, usize> = BTreeMap::new();
    for p in all_plugins() {
        let cat = p.meta.category.as_str();
        if cat == "owner" && !ctx.is_owner {
            continue;
        }
        if DISEMBUNYI_MD.contains(&cat) {
            continue;
        }
        *hitung.entry(cat).or_default() += 1;
    }
    let mut hasil: Vec<(&'static str, usize)> = hitung.into_iter().collect();
    hasil.sort_by_key(|(cat, _)| {
        URUTAN
            .iter()
            .position(|u| u == cat)
            .map_or(999, |i| i as i32)
    });
    hasil
}

fn build_menu(ctx: &Ctx) -> String {
    let prefix = if ctx.prefix.is_empty() {
        "."
    } else {
        ctx.prefix.as_str()
    };
    let (_, _, _, jam, menit, _) = jakarta_parts(now_ms());
    let push = if ctx.push_name.is_empty() {
        "User"
    } else {
        ctx.push_name.as_str()
    };
    let nomor = ctx.sender.split('@').next().unwrap_or(&ctx.sender);
    let bot = ctx.config();
    let nama_bot = if bot.bot.name.is_empty() {
        "Bot"
    } else {
        bot.bot.name.as_str()
    };
    let pengembang = if bot.bot.developer.is_empty() {
        "Owner"
    } else {
        bot.bot.developer.as_str()
    };
    let versi = if bot.bot.version.is_empty() {
        "1.2.0"
    } else {
        bot.bot.version.as_str()
    };
    let peran = if ctx.is_owner {
        "Owner"
    } else if ctx.is_premium {
        "Premium"
    } else {
        "User"
    };

    let mut t = format!("{}\n\n", sapa());
    t.push_str(&format!("      {push}\n\n"));
    t.push_str("Halo Kawan 🌱\n");
    t.push_str(&format!(
        "Perkenalkan aku {nama_bot}, buatan {pengembang} Saya siap membantu\n"
    ));
    t.push_str("kebutuhanmu, mulai dari download\n");
    t.push_str("video, main game, tanya jawab, nyari\n");
    t.push_str("info/sesuatu, bikin sticker, dan lain-lain.\n\n");

    t.push_str("    ᯓ INFO USER\n");
    t.push_str(&format!("╭   • Nama : {push}\n"));
    t.push_str(&format!("┆   • Status : {peran}\n"));
    t.push_str("┆   • Mode : PUBLIC\n");
    t.push_str(&format!("┆   • Nomor : {nomor}\n"));
    t.push_str(&format!("┆   • Waktu : {}:{} WIB\n", pad2(jam), pad2(menit)));
    t.push_str("╰➤------------------------------\n");

    t.push_str("    ᯓ INFO BOT\n");
    t.push_str(&format!("╭  • Name : {nama_bot}\n"));
    t.push_str(&format!("┆  • Author : {pengembang}\n"));
    t.push_str(&format!("┆  • Versi : {versi}\n"));
    t.push_str("┆  • Type script : Multi-Device\n");
    t.push_str(&format!("┆  • Uptime : {}\n", uptime(ctx.uptime_secs)));
    t.push_str("╰➤------------------------------\n");

    let daftar = kategori(ctx);
    if !daftar.is_empty() {
        t.push_str(" ✦ DAFTAR MENU ✦\n");
        for (i, (cat, _)) in daftar.iter().enumerate() {
            let awalan = if i == 0 { "╭   •" } else { "┆   •" };
            t.push_str(&format!(
                "{awalan} {prefix}menucat {cat} {}\n",
                emoji(cat)
            ));
        }
        t.push_str("╰➤------------------------------\n");
    }
    t
}

#[plugin(
    name = "menu",
    alias = ["help", "bantuan", "commands", "m"],
    category = "main",
    description = "Menampilkan menu utama bot",
    usage = ".menu",
    example = ".menu",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn menu(ctx: Ctx) -> Reply {
    Reply::text(build_menu(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::config::Runtime;

    fn ctx() -> Ctx {
        Ctx {
            prefix: ".".into(),
            push_name: "Oktz".into(),
            sender: "628111@s.whatsapp.net".into(),
            uptime_secs: 90_061,
            config: std::sync::Arc::new(Runtime {
                bot: oktz_plugin_api::config::BotInfo {
                    name: "Oktz Bot".into(),
                    version: "3.3.1".into(),
                    developer: "Zann".into(),
                    website: String::new(),
                },
                ..Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn teks(r: Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn memuat_info_user_bot_dan_daftar_kategori() {
        let t = teks(menu(ctx()));
        assert!(t.contains("Oktz Bot"), "{t}");
        assert!(t.contains("Zann"), "{t}");
        assert!(t.contains("Nama : Oktz"), "{t}");
        assert!(t.contains("Nomor : 628111"), "{t}");
        assert!(t.contains("Uptime : 1d 1h 1m 1s"), "{t}");
        assert!(t.contains(".menucat main 🏠"), "{t}");
        assert!(!t.contains(".menucat owner"), "owner bukan owner: {t}");
    }

    #[test]
    fn owner_melihat_kategori_owner() {
        let mut c = ctx();
        c.is_owner = true;
        let t = teks(menu(c));
        assert!(t.contains(".menucat owner 👑"), "{t}");
        assert!(t.contains("Status : Owner"), "{t}");
    }

    #[test]
    fn kategori_md_menyembunyikan_panel_pushkontak_store() {
        let semua: Vec<_> = kategori(&ctx()).into_iter().map(|(c, _)| c).collect();
        for cat in ["panel", "pushkontak", "store"] {
            assert!(!semua.contains(&cat), "{cat} harus disembunyikan: {semua:?}");
        }
    }

    #[test]
    fn semua_kategori_terdaftar_punya_emoji() {
        let semua = kategori(&ctx());
        assert!(semua.iter().all(|(_, n)| *n > 0));
    }
}
