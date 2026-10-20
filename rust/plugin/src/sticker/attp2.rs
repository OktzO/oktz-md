//! `plugins/sticker/attp2.js` — sticker animasi teks lewat canvas.
//!
//! `ponytail:` `@napi-rs/canvas`, `addExifToWebp` (encoder WebP + EXIF), dan
//! `sock.sendMessage({sticker})` tidak tersedia di `Ctx`/`Reply`; sticker
//! animasi tidak dapat dihasilkan. Teks pakai dan jumlah frame di-port agar
//! bisa diuji. Handler untuk teks diakhiri `error_template` + `☢` (pola
//! `swm.rs`). Saat canvas + encoder WebP tersedia, tambahkan
//! `pub mod attp2;` di `src/sticker/mod.rs` secara manual (mod.rs milik plugin
//! lain — dilarang menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Jumlah frame animasi (`attp2.js:18`).
#[cfg(test)]
pub(crate) const FRAMES: usize = 8;

/// Ukuran canvas (`attp2.js:20`).
#[cfg(test)]
pub(crate) const CANVAS_SIZE: u32 = 512;

/// Warna tiap frame (`attp2.js:19`).
#[cfg(test)]
pub(crate) const WARNA: [&str; 6] = [
    "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF",
];

/// Kartu pakai / teks kosong (`attp2.js:70-72`).
pub(crate) fn pakai(prefix: &str, command: &str) -> String {
    format!(
        "⚠️ Harap masukkan teks!\nContoh: `{prefix}{command} Halo semua`"
    )
}

#[plugin(
    name = "attp2",
    alias = ["attp2"],
    category = "sticker",
    description = "Buat sticker animasi teks",
    usage = ".attp2 <teks>",
    example = ".attp2 Halo dunia",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn attp2(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix, &ctx.command));
    }

    // `@napi-rs/canvas`, `addExifToWebp`, dan `sendMessage({sticker})`
    // tidak tersedia di `Ctx`/`Reply` — diakhiri error_template seperti
    // swm.rs.
    Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("attp2").unwrap();
        assert_eq!(p.meta.name, "attp2");
        assert_eq!(p.meta.alias, ["attp2"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn konstanta_animasi() {
        assert_eq!(FRAMES, 8);
        assert_eq!(CANVAS_SIZE, 512);
        assert_eq!(WARNA.len(), 6);
        assert_eq!(WARNA[0], "#FF0000");
    }

    #[test]
    fn pakai_mengikuti_prefix_dan_command() {
        assert_eq!(
            pakai(".", "attp2"),
            "⚠️ Harap masukkan teks!\nContoh: `.attp2 Halo semua`"
        );
        assert_eq!(
            pakai("!", "attp2"),
            "⚠️ Harap masukkan teks!\nContoh: `!attp2 Halo semua`"
        );
    }

    #[test]
    fn tanpa_text_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "attp2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = attp2(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), pakai(".", "attp2"));
    }

    #[test]
    fn text_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "attp2".into(),
            text: "Halo dunia".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = attp2(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template_bawaan(".", "attp2")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
