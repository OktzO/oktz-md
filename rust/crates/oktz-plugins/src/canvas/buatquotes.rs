//! `plugins/canvas/buatquotes.js` — gambar quote bertema anime.
//!
//! `ponytail:` `@napi-rs/canvas`, `node:fs`, dan font dari GitHub tidak
//! tersedia di `Ctx`/`Reply`; gambar quote tidak dapat dihasilkan. Data
//! background (8 background) dan teks pakai di-port agar bisa diuji. Handler
//! untuk teks lengkap diakhiri `error_template` + `☢` (pola `swm.rs`).
//! Saat canvas tersedia, tambahkan `pub mod buatquotes;` di
//! `src/canvas/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

const CANVAS_WIDTH: u32 = 1254;
const CANVAS_HEIGHT: u32 = 1254;

/// `BACKGROUNDS` (`buatquotes.js:34-91`).
pub(crate) struct Background {
    pub name: &'static str,
    pub url: &'static str,
    pub text_zone: (u32, u32, u32, u32),
    pub username_zone: (u32, u32, u32, u32),
    pub username_font_size: u32,
}

pub(crate) const BACKGROUNDS: [Background; 8] = [
    Background {
        name: "l",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/L.png",
        text_zone: (775, 56, 456, 1102),
        username_zone: (890, 1167, 228, 50),
        username_font_size: 28,
    },
    Background {
        name: "gojo",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/gok.png",
        text_zone: (755, 68, 466, 1027),
        username_zone: (863, 1108, 249, 50),
        username_font_size: 28,
    },
    Background {
        name: "yuji",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/cc.png",
        text_zone: (35, 68, 466, 1027),
        username_zone: (133, 1108, 249, 50),
        username_font_size: 28,
    },
    Background {
        name: "denji",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/denji.png",
        text_zone: (655, 68, 512, 1083),
        username_zone: (795, 1152, 249, 50),
        username_font_size: 28,
    },
    Background {
        name: "thorfin",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/thorfin.png",
        text_zone: (65, 54, 489, 992),
        username_zone: (162, 1042, 249, 50),
        username_font_size: 28,
    },
    Background {
        name: "naruto",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/Naruto.png",
        text_zone: (40, 56, 481, 1065),
        username_zone: (170, 1126, 228, 50),
        username_font_size: 28,
    },
    Background {
        name: "light",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/LIghtyagami.png",
        text_zone: (38, 56, 493, 941),
        username_zone: (170, 1025, 228, 50),
        username_font_size: 28,
    },
    Background {
        name: "higuruma",
        url: "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/qca/higuruma.png",
        text_zone: (755, 68, 424, 920),
        username_zone: (840, 993, 249, 50),
        username_font_size: 28,
    },
];

/// `TEXT_STYLE` (`buatquotes.js:93-100`).
pub(crate) struct TextStyle {
    pub initial_size: u32,
    pub min_font_size: u32,
}

pub(crate) const TEXT_STYLE: TextStyle = TextStyle {
    initial_size: 75,
    min_font_size: 24,
};

/// Kartu pakai tanpa argumen (`buatquotes.js:268-287`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎨 *FITUR BUAT QUOTES ANIME*\n\n\
         Fitur ini akan membantumu merangkai kata-kata mutiara dengan latar belakang karakter anime favorit yang sangat keren!\n\n\
         *CARA PENGGUNAAN:*\n\
         - `{prefix}buatquotes <teks>`\n\
         - `{prefix}buatquotes <id_background> | <teks>`\n\
         - `{prefix}buatquotes <id_background> | <teks> | <namamu>`\n\n\
         *DAFTAR BACKGROUND PENDUKUNG (ID 1-8):*\n\
         - 1: L (Death Note)\n\
         - 2: Gojo Satoru\n\
         - 3: Yuji Itadori\n\
         - 4: Denji (Chainsaw Man)\n\
         - 5: Thorfinn\n\
         - 6: Naruto\n\
         - 7: Light Yagami\n\
         - 8: Higuruma\n\n\
         _Contoh: {prefix}buatquotes 2 | Tetaplah hidup walaupun tidak berguna | Maman_"
    )
}

#[plugin(
    name = "buatquotes",
    alias = ["bq", "quoteanime", "animequote"],
    category = "canvas",
    description = "Membuat gambar quote bertema anime secara custom.",
    usage = ".buatquotes [id background] | <teks> | [nama pembuat]",
    example = ".buatquotes 2 | Tetaplah hidup walaupun tidak berguna | Maman",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn buatquotes(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    // `@napi-rs/canvas` + `node:fs` + font eksternal tidak tersedia di
    // `Ctx`/`Reply` — diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("buatquotes").unwrap();
        assert_eq!(p.meta.name, "buatquotes");
        assert_eq!(p.meta.alias, ["bq", "quoteanime", "animequote"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn background_delapan_sama_dengan_js() {
        assert_eq!(BACKGROUNDS.len(), 8);
        assert_eq!(BACKGROUNDS[0].name, "l");
        assert_eq!(BACKGROUNDS[0].text_zone, (775, 56, 456, 1102));
        assert_eq!(BACKGROUNDS[1].name, "gojo");
        assert_eq!(BACKGROUNDS[7].name, "higuruma");
        for bg in BACKGROUNDS {
            assert!(!bg.url.is_empty());
            assert_eq!(bg.username_font_size, 28);
        }
    }

    #[test]
    fn text_style() {
        assert_eq!(TEXT_STYLE.initial_size, 75);
        assert_eq!(TEXT_STYLE.min_font_size, 24);
    }

    #[test]
    fn canvas_ukuran() {
        assert_eq!(CANVAS_WIDTH, 1254);
        assert_eq!(CANVAS_HEIGHT, 1254);
    }

    #[test]
    fn tanpa_text_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buatquotes".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = buatquotes(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), pakai("."));
    }

    #[test]
    fn text_lengkap_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buatquotes".into(),
            text: "2 | Halo dunia | Budi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = buatquotes(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "buatquotes")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
