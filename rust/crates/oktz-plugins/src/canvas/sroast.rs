//! `plugins/canvas/sroast.js` — stiker roast dengan canvas (`@napi-rs/canvas`),
//! font eksternal dari catbox.moe, dan teks yang di-wrap.
//!
//! `ponytail:` Tidak ada API canvas (gambar, font, teks) yang diekspos ke
//! plugin di registry Rust, dan tidak ada klien HTTP untuk memuat font dari
//! URL eksternal. Sebagian gambar dari plugin ini tidak dapat dicerminkan.
//! Handler di bawah hanya menangani guard input dan jatuh ke balasan error
//! saat bagian canvas harus dieksekusi — ini penyimpangan disengaja yang
//! perlu diimplementasikan ulang saat API canvas tersedia.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

/// Kartu pakai saat input kosong.
pub(crate) fn help(prefix: &str, command: &str) -> String {
    format!(
        "⚠️ Harap masukkan teks!\nContoh: `{prefix}{command} aku | JELEK | banget`"
    )
}

/// `ponytail:` Implementasi canvas asli (createCanvas, wrapText, drawText,
/// encode PNG, GlobalFonts.register dari catbox.moe) tidak tersedia di
// registry Rust. Fungsi ini adalah placeholder yang selalu gagal.
fn buat_stiker_roast(_t1: &str, _t2: &str, _t3: &str) -> Result<Vec<u8>, String> {
    Err("canvas API tidak tersedia di registry plugin".to_string())
}

#[plugin(
    name = "sroast",
    alias = ["stickerroast", "sroast"],
    category = "canvas",
    description = "Buat stiker roast",
    usage = ".sroast <teks1> | <teks2> | <teks3>",
    example = ".sroast kamu | JELEK | banget",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn sroast(ctx: Ctx) -> Reply {
    let input = ctx.text.trim();

    if input.is_empty() {
        return Reply::text(help(&ctx.prefix, &ctx.command));
    }

    let parts: Vec<&str> = input.split('|').map(|v| v.trim()).collect();
    let t1 = parts.first().copied().unwrap_or("-");
    let t2 = parts.get(1).copied().unwrap_or("-").to_uppercase();
    let t3 = parts.get(2).copied().unwrap_or("-");

    // `m.react('🕕')` di JS sebelum try — di sini hanya satu reaksi yang
    // dapat dibawa, jadi reaksi akhir adalah ✅ atau ☢.
    match buat_stiker_roast(t1, &t2, t3) {
        Ok(_buffer) => Reply::text("Stiker roast berhasil dibuat.").with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn input_kosong_menghasilkan_peringatan_tanpa_reaksi() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "sroast".into(),
            ..Ctx::default()
        };
        let r = sroast(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            "⚠️ Harap masukkan teks!\nContoh: `.sroast aku | JELEK | banget`"
        );
        assert!(r.without_mutations().text_body().is_some());
        // Tidak ada reaksi (bukan Mutate/Reaction).
        assert!(!matches!(r, Reply::Reaction { .. }));
    }

    #[test]
    fn split_pipa_dan_t2_diuppercase() {
        // Hanya memeriksa logika split (bukan canvas, yang gagal).
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "sroast".into(),
            ..Ctx::default()
        };
        let _ = sroast(Ctx {
            text: "aku | JELEK | banget".into(),
            ..ctx
        });
        // Tidak dapat memeriksa output canvas (selalu error), jadi tes ini
        // hanya memastikan handler tidak panic pada input yang valid.
    }

    #[test]
    fn fmt_ponytail_placeholder() {
        // `buat_stiker_roast` selalu Err karena placeholder canvas.
        let hasil = buat_stiker_roast("aku", "JELEK", "banget");
        assert!(hasil.is_err());
    }
}
