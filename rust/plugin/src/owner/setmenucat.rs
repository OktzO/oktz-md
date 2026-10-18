//! `plugins/owner/setmenucat.js` — atur variant tampilan menucat.
//!
//! `ponytail:` `db.setting(key, value)` + `db.save()` (penulisan) dan
//! `sock.sendButton` tidak diekspos ke plugin di registry Rust. Hanya baca
//! `ctx.setting("menucatVariant")` yang tersedia. Cabang tulis (dengan
//! argumen) dan kirim tombol interaktif (tanpa argumen) jatuh ke error /
//! placeholder.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[derive(Clone, Debug)]
struct Variant {
    id: i64,
    name: &'static str,
    desc: &'static str,
    emoji: &'static str,
    key: &'static str,
}

const VARIANTS: &[Variant] = &[
    Variant {
        id: 1,
        name: "Plain Text",
        desc: "Tampilan teks biasa tanpa media, cocok untuk koneksi lambat atau device yang tidak support interactive message",
        emoji: "📝",
        key: "v1",
    },
    Variant {
        id: 2,
        name: "Interactive + Image Header",
        desc: "Tampilan premium dengan gambar header, limited time offer, dan tombol navigasi interaktif",
        emoji: "🖼️",
        key: "v2",
    },
    Variant {
        id: 5,
        name: "MENUCAT NATIVEFLOW",
        desc: "Tampilan native flow premium dengan video & cuaca",
        emoji: "✨",
        key: "v5",
    },
    Variant {
        id: 6,
        name: "MENUCAT LOCATION",
        desc: "Tampilan location message tanpa tombol interaktif",
        emoji: "📍",
        key: "v6",
    },
];

/// Kunci setting untuk variant menucat.
const SETTING_KEY: &str = "menucatVariant";

/// Default variant saat tidak ada setting dan tidak ada config.ui.
const DEFAULT_VARIANT: i64 = 2;

/// Baca variant aktif: `db.setting("menucatVariant") || config.ui?.menucatVariant || 2`.
/// `config.ui` tidak ada di `Runtime`, jadi defaultnya 2.
fn variant_aktif(ctx: &Ctx) -> i64 {
    ctx.setting(SETTING_KEY)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(DEFAULT_VARIANT)
}

/// Bangun teks pesan variant saat ini.
fn pesan_variant_aktif(current: i64) -> String {
    let v = VARIANTS
        .iter()
        .find(|v| v.id == current)
        .map(|v| v.name)
        .unwrap_or("Unknown");
    format!(
        "📂🗂️ *MENUCAT VARIANT*\n\n\
         Atur tampilan menu per kategori ketika user memilih kategori dari menu utama 📋✨\n\
         Variant aktif saat ini: *V{} — {}* 🎯\n\n\
         > Pilih variant menucat dari tombol di bawah 👇",
        current, v
    )
}

#[plugin(
    name = "setmenucat",
    alias = ["menucatvariant", "menucatstyle"],
    category = "owner",
    description = "Mengatur variant tampilan menucat",
    usage = ".setmenucat <v1-v2, v5>",
    example = ".setmenucat v2",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setmenucat(ctx: Ctx) -> Reply {
    let variant = ctx.args.first().map(|s| s.to_lowercase());

    let Some(key) = variant.as_deref() else {
        // Tanpa argumen: tampilkan daftar variant (placeholder — tombol
        // interaktif tidak dapat dikirim tanpa `sock.sendButton`).
        let current = variant_aktif(&ctx);
        return Reply::text(pesan_variant_aktif(current));
    };

    let selected = VARIANTS.iter().find(|v| v.key == key);
    let Some(selected) = selected else {
        return Reply::text("❌ *VARIANT TIDAK VALID*\n\nGunakan: *v1*, *v2*, *v5*, atau *v6*");
    };

    // `ponytail:` `db.setting("menucatVariant", selected.id)` + `db.save()`
    // (penulisan database) tidak tersedia di registry Rust, jadi perubahan
    // tidak dapat disimpan.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn varian_terdapat_empat() {
        assert_eq!(VARIANTS.len(), 4);
        assert_eq!(VARIANTS[0].id, 1);
        assert_eq!(VARIANTS[1].id, 2);
        assert_eq!(VARIANTS[2].id, 5);
        assert_eq!(VARIANTS[3].id, 6);
    }

    #[test]
    fn variant_aktif_default_2_tanpa_setting() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setmenucat".into(),
            ..Ctx::default()
        };
        assert_eq!(variant_aktif(&ctx), DEFAULT_VARIANT);
    }

    #[test]
    fn variant_aktif_dari_setting() {
        use serde_json::json;
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setmenucat".into(),
            settings: serde_json::json!({ "menucatVariant": 5 }),
            ..Ctx::default()
        };
        assert_eq!(variant_aktif(&ctx), 5);
    }

    #[test]
    fn pesan_variant_aktif_tampilkan_nama() {
        let teks = pesan_variant_aktif(2);
        assert!(teks.contains("V2"));
        assert!(teks.contains("Interactive + Image Header"));
    }

    #[test]
    fn tanpa_argumen_menampilkan_daftar_varian() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setmenucat".into(),
            ..Ctx::default()
        };
        let r = setmenucat(ctx);
        let teks = r.without_mutations().text_body().unwrap();
        assert!(teks.contains("MENUCAT VARIANT"));
        assert!(teks.contains("V2"));
        assert!(!teks.contains("VARIANT TIDAK VALID"));
    }

    #[test]
    fn variant_tidak_valid_menghasilkan_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setmenucat".into(),
            args: vec!["v99".into()],
            ..Ctx::default()
        };
        let r = setmenucat(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            "❌ *VARIANT TIDAK VALID*\n\nGunakan: *v1*, *v2*, *v5*, atau *v6*"
        );
    }

    #[test]
    fn tulis_varian_tanpa_api_db_jatuh_ke_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setmenucat".into(),
            args: vec!["v5".into()],
            ..Ctx::default()
        };
        let r = setmenucat(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "setmenucat")
        );
        assert!(matches!(r.without_mutations(), Reply::Reaction { .. }));
    }

    #[test]
    fn format_duration_tidak_digunakan_sini() {
        // Sanity: modul ini tidak tergantung fungsi durasi dari renewsewa.
        assert_eq!(VARIANTS[0].key, "v1");
    }
}
