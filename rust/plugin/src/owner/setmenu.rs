//! `plugins/owner/setmenu.js` — mengatur variant tampilan menu.
//!
//! Yang di-port: menyimpan variant menu ke store (`db.setting('menuVariant')`,
//! `setmenu.js:118`) supaya `main/menu` memakai kartu yang dipilih.
//!
//! `ponytail:` hanya `v2` dan `v3` yang didukung, karena itu varian yang
//! sudah di-port ke Rust. Varian JS `v1`, `v4`..`v8` butuh `sharp`, kartu
//! lokasi, atau pesan berurutan yang belum ada padanannya di Rust.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// Kunci setting di store — dibaca `main/menu` lewat
/// `ctx.setting_i64("menuVariant")`.
const KUNCI: &str = "menuVariant";

/// Varian yang tersedia di Rust: (kunci, id, nama).
const VARIANTS: &[(&str, i64, &str)] = &[
    ("v2", 2, "Varian 2 — kartu `Hello Brother` + tombol Owner/Script"),
    ("v3", 3, "Varian 3 — kartu `List Menu` + tombol Owner/Allmenu"),
];

pub(crate) fn help(prefix: &str) -> String {
    let mut t = format!(
        "🎨🖼️ *MENU VARIANT*\n\n> Atur tampilan variant menu\n\n`{prefix}setmenu <v2|v3>`\n"
    );
    for (kunci, _, nama) in VARIANTS {
        t.push_str(&format!("> {kunci} — {nama}\n"));
    }
    t
}

#[plugin(
    name = "setmenu",
    alias = ["menuvariant", "menustyle"],
    category = "owner",
    description = "Mengatur variant tampilan menu",
    usage = ".setmenu <v2|v3>",
    example = ".setmenu v3",
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
    let arg = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    if arg.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    let Some((_, id, nama)) = VARIANTS.iter().find(|(kunci, _, _)| *kunci == arg) else {
        return Reply::text(format!(
            "❌ Varian `{arg}` tidak dikenal.\n\n{}",
            help(&ctx.prefix)
        ));
    };
    Reply::text(format!("✅ {nama} aktif."))
        .with_mutation(Mutation::SetSetting {
            key: KUNCI.to_string(),
            value: serde_json::json!(id),
        })
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
    fn tanpa_argumen_menampilkan_daftar_varian() {
        let r = setmenu(ctx_of(""));
        let teks = r.text_body().unwrap();
        assert!(teks.contains("MENU VARIANT"), "{teks}");
        assert!(teks.contains("v2"), "{teks}");
        assert!(teks.contains("v3"), "{teks}");
    }

    #[test]
    fn v3_menyimpan_setting_menu_variant() {
        let r = setmenu(ctx_of("v3"));
        assert!(r.text_body().unwrap().contains("aktif"));
        let mutasi = r.mutations();
        assert_eq!(mutasi.len(), 1);
        match &mutasi[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "menuVariant");
                assert_eq!(value.as_i64(), Some(3));
            }
            lain => panic!("harus SetSetting, bukan {lain:?}"),
        }
    }

    #[test]
    fn variant_tidak_dikenal_menolak_dengan_bantuan() {
        let r = setmenu(ctx_of("v9"));
        let teks = r.text_body().unwrap();
        assert!(teks.contains("tidak dikenal"), "{teks}");
        assert!(r.mutations().is_empty(), "tidak boleh menyimpan apa pun");
    }
}
