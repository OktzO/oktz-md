//! `plugins/ephoto/ephoto.js` — efek teks dari en.ephoto360.com.
//!
//! `pluginConfig.name` di JS adalah array 32 efek; `#[plugin]` hanya menerima
//! satu string, jadi efek pertama jadi `name` dan sisanya jadi alias, lalu
//! alias JS (`ephoto`) menyusul — aturan yang sama seperti `siapa.rs` dan
//! `husbualbum.rs`.
//!
//! `ponytail:` scraping-nya belum di-port. JS butuh `cheerio` untuk membaca
//! `<input name=token|build_server|build_server_id|form_value_input>` dari HTML,
//! lalu mengulang dua POST yang WAJIB membawa cookie sesi dari GET pertama
//! (`headers.cookie`, `ephoto.js:92,105`). `HttpGet` tidak punya tempat
//! memasang header, jadi alur tiga langkah itu tidak bisa dijalankan apa adanya.
//! Yang dipindah di sini SEMUA logika plugin: peta efek, daftar efek, guard
//! teks kosong, guard efek tak dikenal, dan teks pesan. Pemanggilan scraper
//! menjadi satu fungsi [`ephoto`] yang selalu gagal di port ini, jadi cabang
//! "gagal" tetap utuh dan tetap bisa diuji — persis pola `instagramdl.rs`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

/// `EFFECT_URLS` (`ephoto.js:5-38`) — urutan objek JS dipertahankan karena
/// `Object.keys(...).map(...)` di `ephoto.js:117` ikut bergantung padanya.
/// Kunci bukan identifier Rust yang sah (`1917style`), jadi tabel bukan map.
pub(crate) const EFEK: [(&str, &str); 32] = [
    (
        "glitchtext",
        "https://en.ephoto360.com/create-digital-glitch-text-effects-online-767.html",
    ),
    (
        "writetext",
        "https://en.ephoto360.com/write-text-on-wet-glass-online-589.html",
    ),
    (
        "advancedglow",
        "https://en.ephoto360.com/advanced-glow-effects-74.html",
    ),
    (
        "typographytext",
        "https://en.ephoto360.com/create-typography-text-effect-on-pavement-online-774.html",
    ),
    (
        "pixelglitch",
        "https://en.ephoto360.com/create-pixel-glitch-text-effect-online-769.html",
    ),
    (
        "neonglitch",
        "https://en.ephoto360.com/create-impressive-neon-glitch-text-effects-online-768.html",
    ),
    (
        "flagtext",
        "https://en.ephoto360.com/nigeria-3d-flag-text-effect-online-free-753.html",
    ),
    (
        "flag3dtext",
        "https://en.ephoto360.com/free-online-american-flag-3d-text-effect-generator-725.html",
    ),
    (
        "deletingtext",
        "https://en.ephoto360.com/create-eraser-deleting-text-effect-online-717.html",
    ),
    (
        "blackpinkstyle",
        "https://en.ephoto360.com/online-blackpink-style-logo-maker-effect-711.html",
    ),
    (
        "glowingtext",
        "https://en.ephoto360.com/create-glowing-text-effects-online-706.html",
    ),
    (
        "underwatertext",
        "https://en.ephoto360.com/3d-underwater-text-effect-online-682.html",
    ),
    (
        "logomaker",
        "https://en.ephoto360.com/free-bear-logo-maker-online-673.html",
    ),
    (
        "cartoonstyle",
        "https://en.ephoto360.com/create-a-cartoon-style-graffiti-text-effect-online-668.html",
    ),
    (
        "papercutstyle",
        "https://en.ephoto360.com/multicolor-3d-paper-cut-style-text-effect-658.html",
    ),
    (
        "watercolortext",
        "https://en.ephoto360.com/create-a-watercolor-text-effect-online-655.html",
    ),
    (
        "effectclouds",
        "https://en.ephoto360.com/write-text-effect-clouds-in-the-sky-online-619.html",
    ),
    (
        "blackpinklogo",
        "https://en.ephoto360.com/create-blackpink-logo-online-free-607.html",
    ),
    (
        "gradienttext",
        "https://en.ephoto360.com/create-3d-gradient-text-effect-online-600.html",
    ),
    (
        "summerbeach",
        "https://en.ephoto360.com/write-in-sand-summer-beach-online-free-595.html",
    ),
    (
        "luxurygold",
        "https://en.ephoto360.com/create-a-luxury-gold-text-effect-online-594.html",
    ),
    (
        "multicoloredneon",
        "https://en.ephoto360.com/create-multicolored-neon-light-signatures-591.html",
    ),
    (
        "sandsummer",
        "https://en.ephoto360.com/write-in-sand-summer-beach-online-576.html",
    ),
    (
        "galaxywallpaper",
        "https://en.ephoto360.com/create-galaxy-wallpaper-mobile-online-528.html",
    ),
    (
        "1917style",
        "https://en.ephoto360.com/1917-style-text-effect-523.html",
    ),
    (
        "makingneon",
        "https://en.ephoto360.com/making-neon-light-text-effect-with-galaxy-style-521.html",
    ),
    (
        "royaltext",
        "https://en.ephoto360.com/royal-text-effect-online-free-471.html",
    ),
    (
        "freecreate",
        "https://en.ephoto360.com/free-create-a-3d-hologram-text-effect-441.html",
    ),
    (
        "galaxystyle",
        "https://en.ephoto360.com/create-galaxy-style-free-name-logo-438.html",
    ),
    (
        "amongustext",
        "https://en.ephoto360.com/create-a-cover-image-for-the-game-among-us-online-762.html",
    ),
    (
        "rainytext",
        "https://en.ephoto360.com/foggy-rainy-text-effect-75.html",
    ),
    ("lighteffects", "https://en.ephoto360.com/light-effects-74.html"),
];

/// `name` (efek pertama) + sisa array `name` + alias JS `ephoto` — urutan ini
/// dipakai tes untuk membandingkan attribute `#[plugin]` dengan array JS.
#[cfg(test)]
pub(crate) const COMMANDS: [&str; 33] = [
    "glitchtext",
    "writetext",
    "advancedglow",
    "typographytext",
    "pixelglitch",
    "neonglitch",
    "flagtext",
    "flag3dtext",
    "deletingtext",
    "blackpinkstyle",
    "glowingtext",
    "underwatertext",
    "logomaker",
    "cartoonstyle",
    "papercutstyle",
    "watercolortext",
    "effectclouds",
    "blackpinklogo",
    "gradienttext",
    "summerbeach",
    "luxurygold",
    "multicoloredneon",
    "sandsummer",
    "galaxywallpaper",
    "1917style",
    "makingneon",
    "royaltext",
    "freecreate",
    "galaxystyle",
    "amongustext",
    "rainytext",
    "lighteffects",
    "ephoto",
];

/// `EFFECT_URLS[command]` (`ephoto.js:130`).
pub(crate) fn url_efek(command: &str) -> Option<&'static str> {
    EFEK
        .iter()
        .find(|(nama, _)| *nama == command)
        .map(|(_, url)| *url)
}

/// `ephoto.js:117-123` — daftar efek untuk command `ephoto`.
pub(crate) fn daftar_efek(prefix: &str) -> String {
    let mut out = String::new();
    for (nama, _) in EFEK.iter() {
        out.push_str(&format!("• `{prefix}{nama}`\n"));
    }
    out.pop();
    out
}

/// `ephoto.js:127` — teks kosong.
pub(crate) fn tanpa_teks(prefix: &str, command: &str) -> String {
    format!("❌ *ᴇʀʀᴏʀ*\n\n> Masukkan text!\n> *Contoh:* {prefix}{command} Foto-AI")
}

/// `ephoto.js:132` — efek tidak ada di peta.
pub(crate) const EFEK_TAK_ADA: &str = "❌ Efek tidak ditemukan";

/// `te(m.prefix, m.command, m.pushName)` dengan `config.errorTemplate`
/// (`config.js:153`). `m.pushName` tidak ada di `Ctx`, jadi fallback `'User'`
/// yang dipakai — sama seperti `tod.rs`/`wwkill.rs`.
#[allow(dead_code)]
pub(crate) fn error_template(prefix: &str, command: &str) -> String {
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

/// `ephoto(url, textInput)` (`ephoto.js:64`) — satu-satunya bagian yang belum
/// bisa di-port: butuh `cheerio` + dua POST ber-cookie. Ganti dengan alur
/// scraping sungguhan; teks, daftar efek, dan semua cabang di file ini tetap
/// berlaku.
fn ephoto(_url: &str, _text: &str) -> Result<String, String> {
    Err("scraper ephoto360 (cheerio + FormData + cookie sesi) belum di-port".into())
}

#[plugin(
    name = "glitchtext",
    alias = [
        "writetext", "advancedglow", "typographytext", "pixelglitch", "neonglitch", "flagtext",
        "flag3dtext", "deletingtext", "blackpinkstyle", "glowingtext", "underwatertext",
        "logomaker", "cartoonstyle", "papercutstyle", "watercolortext", "effectclouds",
        "blackpinklogo", "gradienttext", "summerbeach", "luxurygold", "multicoloredneon",
        "sandsummer", "galaxywallpaper", "1917style", "makingneon", "royaltext", "freecreate",
        "galaxystyle", "amongustext", "rainytext", "lighteffects", "ephoto"
    ],
    category = "ephoto",
    description = "Buat efek text keren dengan berbagai style",
    usage = ".<effect> <text>",
    example = ".glitchtext Foto-AI",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn glitchtext(ctx: Ctx) -> Reply {
    let command = ctx.command.to_lowercase();
    let text = ctx.text.trim();

    if command == "ephoto" {
        return Reply::text(format!(
            "🎨 *ᴇᴘʜᴏᴛᴏ ᴇꜰꜰᴇᴄᴛs*\n\n\
             > Buat efek text keren!\n\n\
             ╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴇꜰᴇᴋ* 」\n{}\n╰┈┈┈┈┈┈┈┈⬡\n\n\
             > *Contoh:* {}glitchtext Foto-AI",
            daftar_efek(&ctx.prefix),
            ctx.prefix
        ));
    }

    if text.is_empty() {
        return Reply::text(tanpa_teks(&ctx.prefix, &command));
    }

    let Some(effect_url) = url_efek(&command) else {
        return Reply::text(EFEK_TAK_ADA);
    };

    match ephoto(effect_url, text) {
        // `sock.sendMedia(..., null, m, { type: 'image' })` — caption null,
        // jadi caption kosong dan yang dibawa hanya URL gambarnya.
        Ok(image_url) => Reply::image(MediaSource::url(image_url), "").with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::NoHttp;
    use std::sync::Arc;

    fn run(command: &str, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: if text.is_empty() {
                Vec::new()
            } else {
                vec![text.into()]
            },
            http: Arc::new(NoHttp),
            ..Default::default()
        };
        (oktz_plugin_api::find("glitchtext").unwrap().handler)(ctx)
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

    /// `ephoto.js:118-123` dirakit utuh supaya teks tampilan bisa dibandingkan
    /// tanpa menyalin 32 baris efek dua kali.
    fn harapan_daftar(prefix: &str) -> String {
        format!(
            "🎨 *ᴇᴘʜᴏᴛᴏ ᴇꜰꜰᴇᴄᴛs*\n\n\
             > Buat efek text keren!\n\n\
             ╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴇꜰᴇᴋ* 」\n{}\n╰┈┈┈┈┈┈┈┈⬡\n\n\
             > *Contoh:* {prefix}glitchtext Foto-AI",
            daftar_efek(prefix)
        )
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("glitchtext").unwrap();
        assert_eq!(p.meta.name, "glitchtext");
        assert_eq!(p.meta.alias.len(), 32);
        assert_eq!(
            p.meta.alias,
            COMMANDS[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(p.meta.category, "ephoto");
        assert_eq!(p.meta.description, "Buat efek text keren dengan berbagai style");
        assert_eq!(p.meta.usage, ".<effect> <text>");
        assert_eq!(p.meta.example, ".glitchtext Foto-AI");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_effect_name_resolves_to_this_plugin() {
        for key in COMMANDS {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.clone()),
                Some("glitchtext".to_string()),
                "key {key} tidak ditemukan"
            );
        }
    }

    #[test]
    fn the_effect_map_keeps_the_js_key_order_and_urls() {
        assert_eq!(EFEK.len(), 32);
        assert_eq!(EFEK[0].0, "glitchtext");
        assert_eq!(
            EFEK[0].1,
            "https://en.ephoto360.com/create-digital-glitch-text-effects-online-767.html"
        );
        assert_eq!(EFEK[31].0, "lighteffects");
        assert_eq!(EFEK[31].1, "https://en.ephoto360.com/light-effects-74.html");
        // `1917style` bukan identifier Rust — masih harus terbaca sebagai efek.
        assert_eq!(EFEK[24].0, "1917style");
        assert_eq!(url_efek("1917style"), Some(EFEK[24].1));
        assert_eq!(url_efek("glitchtext"), Some(EFEK[0].1));
        assert_eq!(url_efek("ephoto"), None, "alias bukan kunci peta");
        assert_eq!(url_efek("tidak-ada"), None);
    }

    #[test]
    fn the_ephoto_alias_lists_every_effect_in_order() {
        let r = run("ephoto", "");
        assert_eq!(text_of(&r), harapan_daftar("."));
        assert_eq!(reaksi(&r), None, "daftar efek bukan kegagalan");
    }

    #[test]
    fn the_prefix_is_substituted_in_the_effect_list() {
        assert!(daftar_efek(".").starts_with("• `.glitchtext`\n"));
        assert!(daftar_efek("!").starts_with("• `!glitchtext`\n"));
        assert!(harapan_daftar("!").contains("> *Contoh:* !glitchtext Foto-AI"));
    }

    #[test]
    fn an_empty_text_asks_for_one_before_touching_the_network() {
        for command in ["glitchtext", "royaltext", "1917style"] {
            for text in ["", "   "] {
                let r = run(command, text);
                assert_eq!(
                    text_of(&r),
                    format!("❌ *ᴇʀʀᴏʀ*\n\n> Masukkan text!\n> *Contoh:* .{command} Foto-AI"),
                    "command {command}, teks {text:?}"
                );
            }
        }
    }

    #[test]
    fn an_unknown_effect_is_rejected() {
        let r = run("glitchtex", "Foto-AI");
        assert_eq!(text_of(&r), EFEK_TAK_ADA);
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn a_failed_scrape_reacts_and_reports_the_error_template() {
        let r = run("glitchtext", "Foto-AI");
        assert_eq!(reaksi(&r), Some("☢"));
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.glitchtext` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }

    #[test]
    fn the_effect_list_has_one_line_per_effect_with_the_prefix_applied() {
        let daftar = daftar_efek("!");
        let baris: Vec<&str> = daftar.lines().collect();
        assert_eq!(baris.len(), 32);
        assert_eq!(baris[0], "• `!glitchtext`");
        assert_eq!(baris[31], "• `!lighteffects`");
        assert!(!daftar.ends_with('\n'), "join tidak menambah baris kosong");
    }
}
