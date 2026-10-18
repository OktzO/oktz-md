//! `plugins/owner/setownertype.js` — atur variant tampilan owner message.
//!
//! `ponytail:` `db.setting`/`db.save` (`src/lib/database.js`) dan
//! `sock.sendMessage` dengan `interactiveButtons` tidak tersedia di
//! `Ctx`/`Reply`; handler diakhiri `error_template` + `☢` (pola `swm.rs`).
//! Tabel varian, parsing opsi, dan teks pilihan di-port agar bisa diuji.
//! Saat store + tombol interaktif tersedia, tambahkan
//! `pub mod setownertype;` di `src/owner/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::json;

/// `VARIANTS` (`setownertype.js:18-21`).
pub(crate) const VARIANTS: [(&str, &str, &str); 2] = [
    ("1", "Current Design", "Tampilan default saat ini"),
    ("2", "Multiple Contact", "Kirim contact card semua owner"),
];

/// `current = db.setting('ownerType') || 1` — baca dari settings.
pub(crate) fn owner_type_saat_ini(ctx: &Ctx) -> i64 {
    ctx.setting("ownerType")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1)
}

/// `/^v?[1-3]$/` → id (`setownertype.js:28-29`).
pub(crate) fn parse_varian(variant: &str) -> Option<i64> {
    let bersih = variant.trim_start_matches('v');
    if bersih.len() == 1 && bersih.chars().next().is_some_and(|c| matches!(c, '1' | '2' | '3')) {
        bersih.parse::<i64>().ok()
    } else {
        None
    }
}

/// Teks saat pilihan variant (`setownertype.js:53-54`).
pub(crate) fn teks_pilihan(current: i64, nama_bot: &str) -> String {
    let varian = VARIANTS.iter().find(|(id, _, _)| id == &current.to_string());
    let (nama, _) = varian.map(|(_, n, d)| (*n, *d)).unwrap_or(("Current Design", "Tampilan default saat ini"));
    format!(
        "🎨 *sᴇᴛ ᴏᴡɴᴇʀ ᴛʏᴘᴇ*\n\n> Type saat ini: *V{current}*\n> _{}_\n\n> Pilih variant owner:",
        nama
    )
}

/// Daftar tombol variant (`setownertype.js:42-51`), tanpa markup tombol WA.
pub(crate) fn daftar_varian(current: i64) -> Vec<String> {
    let mut out = Vec::new();
    for (id, nama, _) in VARIANTS {
        let tandai = if id.parse::<i64>().unwrap_or(0) == current { " ✓" } else { "" };
        out.push(format!("V{id}{tandai} - {nama}"));
    }
    out
}

/// Sukses ganti variant (`setownertype.js:33-38`).
pub(crate) fn berhasil_ganti(id: i64) -> String {
    let varian = VARIANTS.iter().find(|(i, _, _)| *i == id.to_string());
    let (nama, desc) = varian.map(|(_, n, d)| (*n, *d)).unwrap_or(("", ""));
    format!(
        "✅ Owner type diubah ke *V{id}*\n\n> *{nama}*\n> _{desc}_"
    )
}

#[plugin(
    name = "setownertype",
    alias = ["ownertype", "ownervariant", "ownerstyle"],
    category = "owner",
    description = "Mengatur variant tampilan owner message",
    usage = ".setownertype",
    example = ".setownertype",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setownertype(ctx: Ctx) -> Reply {
    // `db.setting('ownerType')`/`db.save` dan `interactiveButtons` tidak
    // tersedia di Ctx/Reply — diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("setownertype").unwrap();
        assert_eq!(p.meta.name, "setownertype");
        assert_eq!(p.meta.alias, ["ownertype", "ownervariant", "ownerstyle"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn tabel_varian_sama_dengan_js() {
        assert_eq!(VARIANTS.len(), 2);
        assert_eq!(VARIANTS[0], ("1", "Current Design", "Tampilan default saat ini"));
        assert_eq!(VARIANTS[1], ("2", "Multiple Contact", "Kirim contact card semua owner"));
    }

    #[test]
    fn owner_type_saat_ini_default_1() {
        let ctx = Ctx {
            settings: serde_json::json!({}),
            ..Ctx::default()
        };
        assert_eq!(owner_type_saat_ini(&ctx), 1);
    }

    #[test]
    fn owner_type_saat_ini_baca_dari_settings() {
        let ctx = Ctx {
            settings: serde_json::json!({ "ownerType": 2 }),
            ..Ctx::default()
        };
        assert_eq!(owner_type_saat_ini(&ctx), 2);
    }

    #[test]
    fn parse_varian_mirip_regex_js() {
        assert_eq!(parse_varian("1"), Some(1));
        assert_eq!(parse_varian("v1"), Some(1));
        assert_eq!(parse_varian("2"), Some(2));
        assert_eq!(parse_varian("v3"), Some(3));
        assert_eq!(parse_varian("4"), None);
        assert_eq!(parse_varian("v4"), None);
        assert_eq!(parse_varian("abc"), None);
        assert_eq!(parse_varian(""), None);
    }

    #[test]
    fn daftar_varian_tandai_terpilih() {
        let out = daftar_varian(1);
        assert_eq!(out, vec!["V1 ✓ - Current Design", "V2 - Multiple Contact"]);
        let out = daftar_varian(2);
        assert_eq!(out, vec!["V1 - Current Design", "V2 ✓ - Multiple Contact"]);
    }

    #[test]
    fn berhasil_ganti_bentuk_js() {
        assert_eq!(
            berhasil_ganti(2),
            "✅ Owner type diubah ke *V2*\n\n> *Multiple Contact*\n> _Kirim contact card semua owner_"
        );
    }

    #[test]
    fn teks_pilihan_bawah_dari_varian() {
        assert!(
            teks_pilihan(1, "Oktz").contains("Type saat ini: *V1*")
        );
        assert!(teks_pilihan(1, "Oktz").contains("Current Design"));
    }

    #[test]
    fn handler_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setownertype".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = setownertype(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "setownertype")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
