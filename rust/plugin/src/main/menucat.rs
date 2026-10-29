//! `plugins/main/menucat.js` — daftar command per kategori.
//!
//! Mengirim kartu native-flow (`interactiveMessage`) seperti bot JS varian 2:
//! header gambar `foto2`, body berisi bracket box tiap kategori, footer
//! "Pilih tombol dibawah untuk kembali ke menu utama", dan tombol
//! `quick_reply` untuk kembali ke menu utama / daftar kategori.
//!
//! Kalau WA gagal merender kartu, teks body dikirim sebagai cadangan
//! (JS `menucat.js` catch → `m.reply(txt)`).
//!
//! `ponytail:` batas yang masih ada dibanding JS:
//! - Simbol permission diambil dari `PluginMeta` plugin. `PluginMeta` tidak
//!   punya field `limit`, jadi simbol `Ⓛ` hanya muncul di legenda, tidak di
//!   baris command.
//! - Huruf permission tidak dikecilkan. JS `toLowerCase()` mengubah `Ⓞ` jadi
//!   `ⓞ` (huruf kecil lingkaran); di sini simbolnya dibiarkan agar tetap
//!   terbaca.
//! - `case` dari `case/foto.js` belum di-port, jadi command case tidak ikut
//!   dihitung maupun ditampilkan.
//! - `botMode` per grup tidak ada di `Ctx`, jadi mode `md` yang dipakai
//!   (kategori `pushkontak` dan `store` disembunyikan dari daftar).
//! - Setting `menucatVariant` tidak dibaca; selalu varian 2 (kartu).

use oktz_plugin_api::{
    Ctx, KartuHeader, MediaSource, PluginMeta, Reply, Tombol, all_plugins, plugin,
};
use serde_json::json;

use crate::common::{jakarta_parts, now_ms};

/// Gambar header kartu — `assets/image/foto2.jpeg` (`menucat.js:233`).
///
/// Binary-nya di-embed: aset tidak ikut ke paket binary, jadi tanpa ini
/// header bisa kosong saat runtime.
const ASET_FOTO2: &[u8] = include_bytes!("../../../assets/image/foto2.jpeg");

/// Urutan kategori `menucat.js:155-181`.
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
    "jpm",
    "pushkontak",
    "panel",
    "ephoto",
    "store",
];

/// Kategori yang disembunyikan mode `md` (`botmode.js:19-25`).
const DISEMBUNYI_MD: &[&str] = &["cpanel", "pushkontak", "store"];

/// Emoji kategori `menucat.js:31-60`.
///
/// `bawaan` dipakai kalau kategorinya tidak ada di peta: daftar kategori
/// pakai "📋" (`menucat.js:217`), satu kategori pakai "📁" (`menucat.js:393`).
fn emoji(kategori: &str, bawaan: &'static str) -> &'static str {
    match kategori {
        "owner" => "👑",
        "main" => "🏠",
        "utility" => "🔧",
        "fun" => "🎮",
        "group" => "👥",
        "download" => "📥",
        "search" => "🔍",
        "tools" => "🛠️",
        "sticker" => "🖼️",
        "ai" => "🤖",
        "game" => "🎯",
        "media" => "🎬",
        "info" => "ℹ️",
        "religi" => "☪️",
        "panel" => "🖥️",
        "user" => "📊",
        "jpm" => "📢",
        "pushkontak" => "📱",
        "ephoto" => "🎨",
        "store" => "🛒",
        "linode" => "☁️",
        "random" => "🎲",
        "canvas" => "🎨",
        "vps" => "🌊",
        "premium" => "💎",
        "convert" => "🔄",
        "economy" => "💰",
        "cek" => "📋",
        _ => bawaan,
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

/// `toSmallCaps()` (`menucat.js:62-96`).
///
/// Huruf ASCII diganti glyph small-caps. Karakter non-ASCII (simbol
/// permission `ⓄⓟⒶⒼⓅ`) dibiarkan apa adanya supaya tetap terbaca —
/// JS `toLowerCase()` sebenarnya mengubahnya ke huruf kecil lingkaran.
fn to_small_caps(teks: &str) -> String {
    // JS `toLowerCase()` juga menurunkan huruf non-ASCII; di sini hanya
    // ASCII. Huruf non-ASCII tidak ada di peta small-caps, jadi hasilnya
    // sama untuk isi yang dipakai plugin.
    teks.chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            match c {
                'a' => 'ᴀ',
                'b' => 'ʙ',
                'c' => 'ᴄ',
                'd' => 'ᴅ',
                'e' => 'ᴇ',
                'f' => 'ꜰ',
                'g' => 'ɢ',
                'h' => 'ʜ',
                'i' => 'ɪ',
                'j' => 'ᴊ',
                'k' => 'ᴋ',
                'l' => 'ʟ',
                'm' => 'ᴍ',
                'n' => 'ɴ',
                'o' => 'ᴏ',
                'p' => 'ᴘ',
                'q' => 'ǫ',
                'r' => 'ʀ',
                's' => 's',
                't' => 'ᴛ',
                'u' => 'ᴜ',
                'v' => 'ᴠ',
                'w' => 'ᴡ',
                'x' => 'x',
                'y' => 'ʏ',
                'z' => 'ᴢ',
                lain => lain,
            }
        })
        .collect()
}

/// `createBracketBox()` (`menucat.js:98-105`).
///
/// Judul box dipakai apa adanya; tiap baris isi dikecilkan dengan
/// [`to_small_caps`].
fn bracket_box(emoji: &str, judul: &str, baris: &[String]) -> String {
    let mut t = format!("╭─〔 {emoji} `{judul}`\n");
    for b in baris {
        t.push_str(&format!("┃ *{}*\n", to_small_caps(b)));
    }
    t.push_str("╰─⬣\n\n");
    t
}

/// `getCommandSymbols()` (`menucat.js:107-118`).
///
/// Simbol `Ⓛ` (limit) tidak ada karena `PluginMeta` tidak punya field itu.
fn simbol(meta: &PluginMeta) -> String {
    let mut s = Vec::new();
    if meta.is_owner {
        s.push("Ⓞ");
    }
    if meta.is_premium {
        s.push("ⓟ");
    }
    if meta.is_admin {
        s.push("Ⓐ");
    }
    if meta.is_group {
        s.push("Ⓖ");
    }
    if meta.is_private {
        s.push("Ⓟ");
    }
    if s.is_empty() {
        String::new()
    } else {
        format!(" {}", s.join(" "))
    }
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

/// `config.bot.developer`.
fn pengembang_bot(ctx: &Ctx) -> &str {
    let d = &ctx.config().bot.developer;
    if d.is_empty() {
        "Owner"
    } else {
        d.as_str()
    }
}

/// Nama plugin dalam satu kategori (`getCommandsByCategory()`).
///
/// Baris tiap plugin: `{prefix}{nama}{simbol}`. Urut alfabet — JS memakai
/// urutan file plugin, Rust tidak punya itu.
fn baris_perintah(kategori: &str, prefix: &str) -> Vec<String> {
    let mut daftar: Vec<(String, String)> = all_plugins()
        .iter()
        .copied()
        .filter(|p| p.meta.is_enabled && p.meta.category == kategori)
        .map(|p| (p.meta.name.clone(), simbol(&p.meta)))
        .collect();
    daftar.sort_by(|a, b| a.0.cmp(&b.0));
    daftar
        .into_iter()
        .map(|(nama, sym)| format!("{prefix}{nama}{sym}"))
        .collect()
}

/// Indeks kategori di [`URUTAN`]; kategori di luar daftar urut paling akhir.
fn urutan_index(kategori: &str) -> usize {
    URUTAN.iter().position(|u| *u == kategori).unwrap_or(999)
}

/// Kategori yang tampil di daftar — `visibleCats` (`menucat.js:193-200`).
fn kategori_daftar(ctx: &Ctx) -> Vec<String> {
    let mut cats: Vec<String> = Vec::new();
    for p in all_plugins().iter().copied() {
        let cat = &p.meta.category;
        if !p.meta.is_enabled {
            continue;
        }
        if cat.as_str() == "owner" && !ctx.is_owner {
            continue;
        }
        if DISEMBUNYI_MD.contains(&cat.as_str()) {
            continue;
        }
        if !cats.iter().any(|c| c.as_str() == cat.as_str()) {
            cats.push(cat.clone());
        }
    }
    cats.sort_by(|a, b| {
        urutan_index(a)
            .cmp(&urutan_index(b))
            .then_with(|| a.cmp(b))
    });
    cats
}

/// Body daftar kategori — `menucat.js:202-224`.
fn body_daftar(ctx: &Ctx) -> String {
    let prefix = prefix(ctx);
    let mut t = bracket_box(
        "🤖",
        "KETERANGAN",
        &[
            "Ⓞ = Hanya untuk owner".to_string(),
            "ⓟ = Hanya untuk premium".to_string(),
            "Ⓛ = Membutuhkan limit".to_string(),
            "Ⓐ = Hanya untuk admin".to_string(),
            "Ⓖ = Hanya di dalam grup".to_string(),
            "Ⓟ = Hanya di private chat".to_string(),
        ],
    );
    for cat in kategori_daftar(ctx) {
        t.push_str(&bracket_box(
            emoji(&cat, "📋"),
            &to_small_caps(&cat),
            &baris_perintah(&cat, prefix),
        ));
    }
    t
}

/// Body satu kategori — `menucat.js:393-405`.
fn body_kategori(kategori: &str, baris: &[String]) -> String {
    let mut t = bracket_box(emoji(kategori, "📁"), &to_small_caps(kategori), baris);
    t.push_str(&format!("Total: `{}` commands", baris.len()));
    t
}

#[plugin(
    name = "menucat",
    alias = ["menucategory", "categorymenu"],
    category = "main",
    description = "Menampilkan daftar command per kategori",
    usage = ".menucat <kategori>",
    example = ".menucat main",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn menucat(ctx: Ctx) -> Reply {
    let prefix = prefix(&ctx);
    let argumen = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if argumen.is_empty() {
        let body = body_daftar(&ctx);
        return Reply::interactive(body.clone())
            .with_footer("Pilih tombol dibawah untuk kembali ke menu utama")
            .with_header(
                KartuHeader::baru()
                    .judul("")
                    .subjudul("")
                    .media(MediaSource::bytes(ASET_FOTO2.to_vec()).with_mimetype("image/jpeg")),
            )
            .with_button(Tombol::balas_cepat(
                "🍅 Kembali Ke Menu Utama",
                format!("{prefix}menu"),
            ))
            .with_params(json!({
                "limited_time_offer": {
                    "text": sapa(),
                    "url": "Hai",
                    "copy_code": format!("Dibuat oleh {}", pengembang_bot(&ctx)),
                    "expiration_time": now_ms() + 1_000_000,
                }
            }))
            .with_mentions(vec![ctx.sender.clone()])
            .with_forwarded()
            .with_forwarding_score(9)
            .with_participant("0@s.whatsapp.net")
            .with_quote_teks(nama_bot(&ctx))
            .with_fallback_teks(body);
    }

    // `allCategories` = kategori yang ada di registry (`menucat.js:368-371`).
    let Some(kategori) = all_plugins()
        .iter()
        .copied()
        .filter(|p| p.meta.is_enabled)
        .map(|p| p.meta.category.clone())
        .find(|c| c.to_lowercase() == argumen)
    else {
        return Reply::text(format!(
            "❌ *KATEGORI TIDAK DITEMUKAN*\n\n> Kategori `{argumen}` tidak ada.\n> Ketik `{prefix}menucat` untuk list kategori."
        ));
    };

    if kategori.as_str() == "owner" && !ctx.is_owner {
        return Reply::text("❌ *AKSES DITOLAK*\n\n> Kategori ini hanya untuk owner.");
    }

    let baris = baris_perintah(&kategori, prefix);
    let body = body_kategori(&kategori, &baris);
    Reply::interactive(body.clone())
        .with_footer("Pilih tombol dibawah untuk kembali ke menu utama")
        .with_header(
            KartuHeader::baru()
                .judul("")
                .subjudul("")
                .media(MediaSource::bytes(ASET_FOTO2.to_vec()).with_mimetype("image/jpeg")),
        )
        .with_button(Tombol::balas_cepat(
            "📂 Kembali Ke Daftar Kategori",
            format!("{prefix}menucat"),
        ))
        .with_button(Tombol::balas_cepat(
            "🍅 Kembali Ke Menu Utama",
            format!("{prefix}menu"),
        ))
        .with_params(json!({
            "limited_time_offer": {
                "text": sapa(),
                "url": "Hai",
                "expiration_time": now_ms() + 10_000,
            }
        }))
        .with_mentions(vec![ctx.sender.clone()])
        .with_forwarded()
        .with_forwarding_score(9)
        .with_participant("0@s.whatsapp.net")
        .with_quote_teks(nama_bot(&ctx))
        .with_fallback_teks(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Kartu;
    use oktz_plugin_api::config::Runtime;

    fn ctx(kategori: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            args: if kategori.is_empty() {
                Vec::new()
            } else {
                vec![kategori.into()]
            },
            command: "menucat".into(),
            sender: "628111@s.whatsapp.net".into(),
            is_owner: true,
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
    fn tanpa_argumen_kirim_kartu_dengan_satu_tombol() {
        let k = kartu(menucat(ctx("")));
        assert_eq!(k.buttons.len(), 1, "{k:?}");
        assert_eq!(k.buttons[0].aksi.as_str(), "quick_reply");
        assert_eq!(k.buttons[0].params["display_text"], "🍅 Kembali Ke Menu Utama");
        assert_eq!(k.buttons[0].params["id"], ".menu");

        let header = k.header.as_ref().expect("header kartu harus ada");
        assert_eq!(header.title.as_deref(), Some(""));
        assert_eq!(header.subtitle.as_deref(), Some(""));
        assert!(matches!(
            header.media.as_ref(),
            Some(MediaSource::Bytes { .. })
        ));

        assert!(k.body.contains("KETERANGAN"), "{}", k.body);
        assert!(k.body.contains("ᴍᴀɪɴ"), "{}", k.body);
        assert_eq!(
            k.footer.as_deref(),
            Some("Pilih tombol dibawah untuk kembali ke menu utama")
        );
        assert_eq!(k.mentions, vec!["628111@s.whatsapp.net"]);
        assert!(k.forwarded);
        assert_eq!(k.forwarding_score, Some(9));
        assert_eq!(k.participant.as_deref(), Some("0@s.whatsapp.net"));
        assert_eq!(k.quote_teks.as_deref(), Some("Oktz Bot"));
        assert!(k.fallback_teks.is_some());
    }

    #[test]
    fn kategori_main_kirim_kartu_dengan_dua_tombol() {
        let k = kartu(menucat(ctx("main")));
        assert_eq!(k.buttons.len(), 2, "{k:?}");
        assert_eq!(k.buttons[0].params["display_text"], "📂 Kembali Ke Daftar Kategori");
        assert_eq!(k.buttons[0].params["id"], ".menucat");
        assert_eq!(k.buttons[1].params["display_text"], "🍅 Kembali Ke Menu Utama");
        assert_eq!(k.buttons[1].params["id"], ".menu");
        assert!(k.body.contains("Total: `"), "{}", k.body);
        assert!(k.body.contains("commands"), "{}", k.body);
        assert!(k.header.is_some());
    }

    #[test]
    fn kategori_tidak_dikenal_membalas_teks() {
        let r = menucat(ctx("tidakada"));
        assert!(!matches!(r.without_mutations(), Reply::Interactive { .. }));
        let t = r.text_body().unwrap_or_default();
        assert!(t.contains("KATEGORI TIDAK DITEMUKAN"), "{t}");
    }

    #[test]
    fn non_owner_ditolak_di_kategori_owner() {
        let mut c = ctx("owner");
        c.is_owner = false;
        let r = menucat(c);
        assert!(!matches!(r.without_mutations(), Reply::Interactive { .. }));
        let t = r.text_body().unwrap_or_default();
        assert!(t.contains("AKSES DITOLAK"), "{t}");
    }

    #[test]
    fn small_caps_mengikuti_peta_js() {
        assert_eq!(to_small_caps("Main"), "ᴍᴀɪɴ");
        assert_eq!(to_small_caps("Ⓞ = Hanya"), "Ⓞ = ʜᴀɴʏᴀ");
    }

    #[test]
    fn bracket_box_memakai_format_js() {
        let t = bracket_box("🤖", "KETERANGAN", &["Ⓞ = Hanya untuk owner".to_string()]);
        assert_eq!(
            t,
            "╭─〔 🤖 `KETERANGAN`\n┃ *Ⓞ = ʜᴀɴʏᴀ ᴜɴᴛᴜᴋ ᴏᴡɴᴇʀ*\n╰─⬣\n\n"
        );
    }
}
