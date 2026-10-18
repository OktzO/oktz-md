//! `plugins/owner/setmenu.js` — mengatur variant tampilan menu.

use oktz_plugin_api::{Ctx, Reply, plugin};

const VARIANTS: &[(&str, u32, &str, &str, &str)] = &[
    ("v1", 1, "BASIC", "", "🖼️"),
    ("v2", 2, "PREMIUM", "", "✅"),
    ("v3", 3, "PREMIUM", "", "✅"),
    ("v4", 4, "LV", "", "✅"),
    ("v5", 5, "LV 2", "", "✅"),
    ("v6", 6, "LOCATION", "Location Message dengan Buttons", "📍"),
    ("v7", 7, "RA NGERTI", "Location Message dengan Buttons", "📍"),
    ("v8", 8, "THUMBNAIL", "Thumbnail Image", "📍"),
];

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎨🖼️ *MENU VARIANT*\n\n> Atur tampilan variant menu\n\n`{prefix}setmenu <v1-v8>`",
        prefix = prefix
    )
}

#[plugin(
    name = "setmenu",
    alias = ["menuvariant", "menustyle"],
    category = "owner",
    description = "Mengatur variant tampilan menu",
    usage = ".setmenu <v1-v16>",
    example = ".setmenu v8",
    cooldown = 3,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setmenu(ctx: Ctx) -> Reply {
    // `setmenu.js` butuh:
    //   1. `db.setting('menuVariant')` / `db.setting(key, v)` + `db.save()` —
    //      store, tidak ada di `Ctx`.
    //   2. `getAssetBuffer('foto')` — baca file asset dari disk, tidak ada di
    //      `Ctx`.
    //   3. `sock.sendButton` — kirim pesan tombol interaktif, I/O WA yang belum
    //      ada di `Ctx` class A.
    //
    // Variant v1-v8 (dari JS; file menyebut v1-v7 di teks error tapi objek
    // VARIANTS punya v8) dicerminkan konstanta di atas.
    //
    // `ponytail:` sediakan akses store + asset buffer + kirim tombol ke `Ctx`,
    // lalu replikasikan loop variant dan tombol single_select.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "setmenu".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = setmenu(ctx_of(""));
        assert!(r.text_body().unwrap().contains("MENU VARIANT"));
    }

    #[test]
    fn variant_v8_ada_dalam_konstanta() {
        assert!(VARIANTS.iter().any(|(k, _, _, _, _)| *k == "v8"));
    }
}
