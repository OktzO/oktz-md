//! `plugins/main/menu.js` — menu utama.
//!
//! Dua varian kartu native-flow (`interactiveMessage`), dipilih dari setting
//! `menuVariant` (`menu.js:337-338`, default produksi = 2):
//! - Varian 2 (default) — body memuat daftar command per kategori, tombol
//!   `cta_url` "Owner Kami" + `quick_reply` "Dapatkan Script".
//! - Varian 3 — `single_select` "🍃 List Menu" + `quick_reply` Owner/Allmenu.
//!
//! Kalau WA gagal merender kartu, teks menu dikirim sebagai cadangan
//! (JS `menu.js:1379`).
//!
//! `ponytail:` batas yang masih ada dibanding JS:
//! - Header gambar harus di-upload ke server WA dulu; plugin hanya menyodorkan
//!   byte-nya, upload dilakukan pipeline (`oktz-wa`).
//! - `config.owner.number` kosong -> tombol "Owner Kami" jadi `quick_reply`
//!   `.owner` (JS pakai `cta_url` ke `https://wa.me/{nomor}`; tanpa nomor,
//!   URL-nya jadi tautan palsu).
//! - Mode global (`config.mode`) tidak ada di `Ctx`; teks cadangan memakai
//!   `PUBLIC` seperti config produksi.
//! - Riwayat command teratas (`database/main/stats.json`) belum dicatat bot
//!   Rust, jadi blok "History" tidak ada.

use std::collections::BTreeMap;

use oktz_plugin_api::{
    Baris, Bagian, Ctx, Daftar, KartuHeader, MediaSource, Reply, Tombol, all_plugins, plugin,
};

use crate::common::{jakarta_parts, js_truthy, now_ms, pad2};

/// Gambar header kartu — `assets/image/foto.jpeg` (`menu.js:462-464`).
///
/// Binary-nya di-embed: aset tidak ikut ke paket binary, jadi tanpa ini
/// header bisa kosong saat runtime.
const ASET_FOTO: &[u8] = include_bytes!("../../../assets/image/foto.jpeg");

/// `String.fromCharCode(8206).repeat(4001)` (`menu.js:465`) — pengisi
/// panjang supaya body kartu tidak terpotong WA.
const READMORE: &str = "\u{200e}";

/// Berapa kali `READMORE` diulang — `menu.js:465`.
const BANYAK_READMORE: usize = 4001;

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

/// Prefix command — `ctx.prefix`, atau "." kalau mode tanpa prefix.
fn prefix(ctx: &Ctx) -> &str {
    if ctx.prefix.is_empty() {
        "."
    } else {
        ctx.prefix.as_str()
    }
}

/// `config.bot.name` — nilai kosong replaced nama bawaan.
fn nama_bot(ctx: &Ctx) -> &str {
    let n = &ctx.config().bot.name;
    if n.is_empty() {
        "Bot"
    } else {
        n.as_str()
    }
}

/// `config.bot.version`.
fn versi_bot(ctx: &Ctx) -> &str {
    let v = &ctx.config().bot.version;
    if v.is_empty() {
        "1.2.0"
    } else {
        v.as_str()
    }
}

/// `config.bot.developer`.
fn pengembang_bot(ctx: &Ctx) -> &str {
    let d = &ctx.config().bot.developer;
    if d.is_empty() {
        "Owner"
    } else {
        d.as_str()
    }
}

/// Angka user dari store — `user.level || 0` (`menu.js:493`).
///
/// Key store: `level`, `exp`, `energi`, `koin`, `isRegistered` (lihat
/// `oktz-wa` pipeline, normalisasi `setUser`). Key yang tidak ada = 0.
fn angka_user(ctx: &Ctx, key: &str) -> i64 {
    ctx.user_field(key)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
}

/// `toMonoUpperBold` (`menu.js:74-108`) — huruf kapital math-bold.
///
/// Peta JS disalin persis: A = U+1D5D4 .. Z = U+1D5ED (mathematical
/// sans-serif bold capital). Huruf lain lolos apa adanya.
fn mono_upper_bold(teks: &str) -> String {
    teks.to_uppercase()
        .chars()
        .map(|c| match c {
            'A' => '\u{1D5D4}',
            'B' => '\u{1D5D5}',
            'C' => '\u{1D5D6}',
            'D' => '\u{1D5D7}',
            'E' => '\u{1D5D8}',
            'F' => '\u{1D5D9}',
            'G' => '\u{1D5DA}',
            'H' => '\u{1D5DB}',
            'I' => '\u{1D5DC}',
            'J' => '\u{1D5DD}',
            'K' => '\u{1D5DE}',
            'L' => '\u{1D5DF}',
            'M' => '\u{1D5E0}',
            'N' => '\u{1D5E1}',
            'O' => '\u{1D5E2}',
            'P' => '\u{1D5E3}',
            'Q' => '\u{1D5E4}',
            'R' => '\u{1D5E5}',
            'S' => '\u{1D5E6}',
            'T' => '\u{1D5E7}',
            'U' => '\u{1D5E8}',
            'V' => '\u{1D5E9}',
            'W' => '\u{1D5EA}',
            'X' => '\u{1D5EB}',
            'Y' => '\u{1D5EC}',
            'Z' => '\u{1D5ED}',
            lain => lain,
        })
        .collect()
}

/// Kategori yang punya plugin, urut seperti JS. `owner` hanya untuk owner.
///
/// Plugin nonaktif (`is_enabled = false`) tidak dihitung — JS
/// `getCommandsByCategory` juga melewati plugin mati.
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
        if !p.meta.is_enabled {
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

/// Nama command dalam satu kategori — `getCommandsByCategory` (`menu.js:395-410`).
///
/// Urut alfabet: JS memakai urutan file plugin, Rust tidak punya itu.
fn perintah_kategori(cat: &str) -> Vec<&'static str> {
    let mut nama: Vec<&'static str> = all_plugins()
        .iter()
        .filter(|p| p.meta.category == cat && p.meta.is_enabled)
        .map(|p| p.meta.name.as_str())
        .collect();
    nama.sort_unstable();
    nama
}

/// Teks menu — padanan `buildMenuText` (`menu.js:174-260`).
///
/// Dipakai sebagai teks cadangan kalau kartu gagal dikirim (JS
/// `menu.js:1379`), bukan sebagai balasan utama.
pub(crate) fn teks_menu(ctx: &Ctx) -> String {
    let prefix = prefix(ctx);
    let (_, _, _, jam, menit, _) = jakarta_parts(now_ms());
    let push = if ctx.push_name.is_empty() {
        "User"
    } else {
        ctx.push_name.as_str()
    };
    let nomor = ctx.sender.split('@').next().unwrap_or(&ctx.sender);
    let nama_bot = nama_bot(ctx);
    let pengembang = pengembang_bot(ctx);
    let versi = versi_bot(ctx);
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

/// Blok `BOT INFORMATION` + `USER INFORMATION` — sama di varian 2 dan 3.
fn blok_info(ctx: &Ctx) -> String {
    let push = if ctx.push_name.is_empty() {
        "User"
    } else {
        ctx.push_name.as_str()
    };
    let peran = if ctx.is_owner {
        "🔥 Owner"
    } else if ctx.is_premium {
        "👑 Premium"
    } else {
        "😊 User"
    };
    let register = if js_truthy(ctx.user_field("isRegistered")) {
        "Sudah"
    } else {
        "Belum"
    };

    format!(
        "🍅 *BOT INFORMATION*\n\
         > 🤖 *Name*: {}\n\
         > ⚙️ *Version*: {}\n\
         > 👨‍💻 *Developer*: {}\n\
         > 🧩 *Library*: `OnigiS`\n\n\
         🍅 *USER INFORMATION*\n\
         > 🧑 *Name*: {}\n\
         > 🥐 *Role*: {}\n\
         > 🧀 *Level*: {}\n\
         > 🍗 *Exp*: {}\n\
         > 🍩 *Energi*: {}\n\
         > 🎏 *Koin*: {}\n\
         > 🍬 *Register*: {}",
        nama_bot(ctx),
        versi_bot(ctx),
        pengembang_bot(ctx),
        push,
        peran,
        angka_user(ctx, "level"),
        angka_user(ctx, "exp"),
        angka_user(ctx, "energi"),
        angka_user(ctx, "koin"),
        register,
    )
}

/// Body varian 2 — `menu.js:480-499`.
///
/// `readmore` + daftar command per kategori (`s`) menempel di akhir.
fn body_varian2(ctx: &Ctx) -> String {
    let mut daftar = String::new();
    let prefix = prefix(ctx);
    for (cat, _) in kategori(ctx) {
        daftar.push_str(&format!("╭─☰ {}\n", mono_upper_bold(cat)));
        for nama in perintah_kategori(cat) {
            daftar.push_str(&format!("> {prefix}{nama}\n"));
        }
        daftar.push_str("╰─⬣\n\n");
    }

    format!(
        "🥞 *Hello Brother*\n\n\
         Welcome to {}, Our bot will help you\n\n\
         {}\n\n{}{}",
        nama_bot(ctx),
        blok_info(ctx),
        READMORE.repeat(BANYAK_READMORE),
        daftar,
    )
}

/// Body varian 3 — `menu.js:564-581` (tanpa daftar command per kategori).
fn body_varian3(ctx: &Ctx) -> String {
    format!(
        "🥞 *Hello Brother* {}\n\n\
         Welcome to {}, Our bot will help you\n\n\
         {}",
        sapa(),
        nama_bot(ctx),
        blok_info(ctx),
    )
}

/// Isi `single_select` varian 3 — `menu.js:611-629`.
fn daftar_kategori(ctx: &Ctx) -> Daftar {
    let daftar = kategori(ctx);
    let mut bagian = Bagian::baru(format!(
        "Berikut adalah pilihan nya ({} kategori)",
        daftar.len()
    ));
    let prefix = prefix(ctx);
    for (cat, jumlah) in daftar {
        bagian = bagian.baris(
            Baris::baru(
                format!("{} {cat}", emoji(cat)),
                format!("{prefix}menucat {cat}"),
            )
            .description(format!("{jumlah} command")),
        );
    }
    Daftar::baru("🍃 List Menu").bagian(bagian).icon("DEFAULT")
}

/// Tombol "Owner Kami" — `menu.js:510-516`.
///
/// `cta_url` ke `https://wa.me/{nomor owner}`. Kalau nomor owner tidak ada,
/// tombolnya jadi `quick_reply` `.owner` — URL tanpa nomor cuma tautan palsu.
fn tombol_pemilik(ctx: &Ctx) -> Tombol {
    let nomor = ctx
        .config()
        .owner
        .number
        .first()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty());
    match nomor {
        Some(n) => Tombol::buka_url("🍫 Owner Kami", format!("https://wa.me/{n}")),
        None => Tombol::balas_cepat("🍫 Owner Kami", format!("{}owner", prefix(ctx))),
    }
}

/// Kartu varian 2 (default produksi) — `menu.js:453-533`.
fn kartu_varian2(ctx: &Ctx) -> Reply {
    Reply::interactive(body_varian2(ctx))
        .with_footer("Pilih tombol dibawah untuk info lebih lanjut")
        .with_header(
            KartuHeader::baru()
                .judul("")
                .subjudul("")
                .media(MediaSource::bytes(ASET_FOTO.to_vec()).with_mimetype("image/jpeg")),
        )
        .with_button(tombol_pemilik(ctx))
        .with_button(Tombol::balas_cepat(
            "🍛 Dapatkan Script ( Gratis )",
            format!("{}sc", prefix(ctx)),
        ))
        .with_mentions(vec![ctx.sender.clone()])
        .with_fallback_teks(teks_menu(ctx))
}

/// Kartu varian 3 — `menu.js:536-673`.
fn kartu_varian3(ctx: &Ctx) -> Reply {
    Reply::interactive(body_varian3(ctx))
        .with_footer("🍔 Silahkan pilih dari salah satu tombol di bawah")
        .with_header(
            KartuHeader::baru()
                .judul("")
                .subjudul(nama_bot(ctx))
                .media(MediaSource::bytes(ASET_FOTO.to_vec()).with_mimetype("image/jpeg")),
        )
        .with_button(Tombol::pilih(&daftar_kategori(ctx)))
        .with_button(Tombol::balas_cepat(
            "🧀 Owner",
            format!("{}owner", prefix(ctx)),
        ))
        .with_button(Tombol::balas_cepat(
            "💐 Allmenu",
            format!("{}allmenu", prefix(ctx)),
        ))
        .with_mentions(vec![ctx.sender.clone()])
        .with_fallback_teks(teks_menu(ctx))
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
    // `db.setting("menuVariant") || botConfig.ui?.menuVariant || 2`
    // (`menu.js:337-338`). Setting lain dari 3 = default produksi, yaitu
    // varian 2.
    match ctx.setting_i64("menuVariant") {
        Some(3) => kartu_varian3(&ctx),
        _ => kartu_varian2(&ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Kartu;
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

    fn kartu(r: Reply) -> Kartu {
        match r.without_mutations() {
            Reply::Interactive { kartu } => (**kartu).clone(),
            lain => panic!("harus kartu interaktif, bukan {lain:?}"),
        }
    }

    #[test]
    fn mono_upper_bold_mengubah_satu_huruf() {
        assert_eq!(mono_upper_bold("a"), "\u{1D5D4}");
        assert_eq!(mono_upper_bold("z"), "\u{1D5ED}");
        assert_eq!(mono_upper_bold("main"), "\u{1D5E0}\u{1D5D4}\u{1D5DC}\u{1D5E1}");
        assert_eq!(mono_upper_bold("1"), "1");
    }

    #[test]
    fn default_memakai_varian2_dengan_daftar_kategori() {
        let k = kartu(menu(ctx()));
        assert!(k.body.contains("Hello Brother"), "{}", k.body);
        assert!(k.body.contains("BOT INFORMATION"), "{}", k.body);
        assert!(k.body.contains("USER INFORMATION"), "{}", k.body);
        assert!(k.body.contains("╭─☰"), "{}", k.body);
        assert!(k.body.contains("╰─⬣"), "{}", k.body);
        // kategori "main" ditulis math-bold: 𝗠𝗔𝗜𝗡
        assert!(
            k.body.contains("╭─☰ \u{1D5E0}\u{1D5D4}\u{1D5DC}\u{1D5E1}"),
            "{}",
            k.body
        );
        assert!(k.body.contains("> .menu\n"), "{}", k.body);
        assert_eq!(
            k.footer.as_deref(),
            Some("Pilih tombol dibawah untuk info lebih lanjut")
        );
    }

    #[test]
    fn varian2_memakai_tombol_script_dan_mention_pengirim() {
        let k = kartu(menu(ctx()));
        assert_eq!(k.buttons.len(), 2, "{k:?}");
        assert_eq!(k.buttons[1].aksi.as_str(), "quick_reply");
        assert_eq!(k.buttons[1].params["display_text"], "🍛 Dapatkan Script ( Gratis )");
        assert_eq!(k.buttons[1].params["id"], ".sc");
        assert_eq!(k.mentions, vec!["628111@s.whatsapp.net"]);
        let cadangan = k.fallback_teks.expect("teks cadangan harus ada");
        assert!(cadangan.contains("DAFTAR MENU"), "{cadangan}");
    }

    #[test]
    fn tombol_owner_goingke_wa_me_kalau_nomor_owner_ada() {
        let mut c = ctx();
        c.config = std::sync::Arc::new(Runtime {
            owner: oktz_plugin_api::config::Owner {
                name: "Oktz".into(),
                number: vec!["6285608953677".into()],
            },
            bot: oktz_plugin_api::config::BotInfo {
                name: "Oktz Bot".into(),
                version: "3.3.1".into(),
                developer: "Zann".into(),
                website: String::new(),
            },
            ..Runtime::default()
        });
        let k = kartu(menu(c));
        assert_eq!(k.buttons[0].aksi.as_str(), "cta_url");
        assert_eq!(k.buttons[0].params["display_text"], "🍫 Owner Kami");
        assert_eq!(
            k.buttons[0].params["url"],
            "https://wa.me/6285608953677"
        );
        assert_eq!(
            k.buttons[0].params["merchant_url"],
            "https://wa.me/6285608953677"
        );
    }

    #[test]
    fn tanpa_nomor_owner_tombol_owner_jadi_quick_reply() {
        let k = kartu(menu(ctx()));
        assert_eq!(k.buttons[0].aksi.as_str(), "quick_reply");
        assert_eq!(k.buttons[0].params["display_text"], "🍫 Owner Kami");
        assert_eq!(k.buttons[0].params["id"], ".owner");
    }

    #[test]
    fn setting_menu_variant_3_memakai_single_select() {
        let mut c = ctx();
        c.settings = serde_json::json!({ "menuVariant": 3 });
        let k = kartu(menu(c));
        assert_eq!(k.buttons.len(), 3, "{k:?}");
        assert_eq!(k.buttons[0].aksi.as_str(), "single_select");
        assert_eq!(k.buttons[0].params["title"], "🍃 List Menu");
        assert_eq!(k.buttons[1].params["display_text"], "🧀 Owner");
        assert_eq!(k.buttons[2].params["display_text"], "💐 Allmenu");
        assert!(k.body.contains("Hello Brother"), "{}", k.body);
        assert!(!k.body.contains("╭─☰"), "varian 3 tidak punya daftar command");
    }

    #[test]
    fn setting_menu_variant_lain_dari_tiga_pakai_varian2() {
        let mut c = ctx();
        c.settings = serde_json::json!({ "menuVariant": 2 });
        let k = kartu(menu(c));
        assert_eq!(k.buttons.len(), 2, "{k:?}");
        assert!(k.body.contains("╭─☰"), "{}", k.body);
    }

    #[test]
    fn header_kartu_memakai_gambar_foto() {
        let k = kartu(menu(ctx()));
        let header = k.header.as_ref().expect("header kartu harus ada");
        assert_eq!(header.title.as_deref(), Some(""));
        assert_eq!(header.subtitle.as_deref(), Some(""));
        assert!(matches!(
            header.media.as_ref(),
            Some(MediaSource::Bytes { .. })
        ));
    }

    #[test]
    fn teks_menu_memuat_info_user_bot_dan_daftar_kategori() {
        let t = teks_menu(&ctx());
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
        let t = teks_menu(&c);
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
