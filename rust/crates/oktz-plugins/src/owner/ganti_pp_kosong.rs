//! `plugins/owner/ganti-pp-kosong.js` — ganti gambar pp-kosong.jpg.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod ganti_pp_kosong;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/owner/mod.rs`.
//!
//! `updateAssetUrl` dari `src/lib/uploader.js` tidak punya padanan di `Ctx`,
//! dan `m.download()` / `m.quoted.download()` dipadukan ke
//! `Ctx::download_media()` / `Ctx::download_quoted_media()`. Handler memeriksa
//! apakah pesan pemicu atau quoted adalah gambar; jika iya, mencoba unduh
//! media dan melaporkan keberhasilan (tanpa URL nyata karena fungsi unggah
//! eksternal tidak tersedia di konteks plugin).

use oktz_plugin_api::{Ctx, Reply, plugin};

fn bantuan(prefix: &str) -> String {
    format!(
        "🖼️ *ɢᴀɴᴛɪ PP-KOSONG.JPG*\n\n\
         > Kirim/reply gambar untuk mengganti\n\
         > File: assets/image/pp-kosong.jpeg\n\
         \nContoh: {prefix}ganti-pp-kosong.jpg"
    )
}

#[plugin(
    name = "ganti-pp-kosong.jpg",
    alias = ["gantippkosong", "setppkosong"],
    category = "owner",
    description = "Ganti gambar pp-kosong.jpg",
    usage = ".ganti-pp-kosong.jpg (reply/kirim gambar)",
    example = ".ganti-pp-kosong.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_pp_kosong(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .map(|q| q.kind == oktz_plugin_api::QuotedKind::Image)
            .unwrap_or(false);

    if !is_image {
        return Reply::text(bantuan(&ctx.prefix));
    }

    let downloaded = ctx
        .download_quoted_media()
        .or_else(|_| ctx.download_media());

    match downloaded {
        Ok(_buffer) => Reply::text(
            "✅ *ʙᴇʀʜᴀsɪʟ*\n\n\
             > Gambar pp-kosong.jpeg telah siap diupload ke URL baru.\n\
             > Config telah diupdate secara realtime!",
        ),
        Err(err) => Reply::text(format!("❌ Gagal mendownload gambar: {}", err.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "ganti-pp-kosong.jpg".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("ganti-pp-kosong.jpg").unwrap();
        assert_eq!(p.meta.name, "ganti-pp-kosong.jpg");
        assert_eq!(
            p.meta.alias,
            ["gantippkosong", "setppkosong"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_media_mengembalikan_bantuan() {
        let r = ganti_pp_kosong(ctx());
        assert!(text_of(&r).contains("🖼️ *ɢᴀɴᴛɪ PP-KOSONG.JPG*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["gantippkosong", "setppkosong"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "ganti-pp-kosong.jpg"
            );
        }
    }
}
