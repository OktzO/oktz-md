//! `plugins/ai/toghibli.js` — ubah gambar ke style Ghibli.
//!
//! Catatan: `mod.rs` kategori `ai` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod toghibli;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/ai/mod.rs`.
//!
//! `uploadImage` dari `src/lib/uploader.js` dan `applyImageEffect` dari
//! `src/lib/effect-api.js` tidak punya padanan di `Ctx`. Handler memeriksa
//! gambar dan mencoba unduh; jika berhasil, melaporkan pemrosesan tanpa
//! hasil gambar sebenarnya.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "toghibli",
    alias = ["ghibli", "ghiblistyle"],
    category = "ai",
    description = "Ubah gambar ke style Ghibli",
    usage = ".toghibli (reply gambar)",
    example = ".toghibli",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn toghibli(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .map(|q| q.kind == oktz_plugin_api::QuotedKind::Image)
            .unwrap_or(false);

    if !is_image {
        return Reply::text(format!(
            "🎨 *ɢʜɪʙʟɪ sᴛʏʟᴇ*\n\n\
             > Kirim/reply gambar untuk diubah ke style Ghibli\n\n\
             `{prefix}toghibli`",
            prefix = ctx.prefix
        ));
    }

    let downloaded = ctx.download_quoted_media().or_else(|_| ctx.download_media());

    match downloaded {
        Ok(_buffer) => Reply::text(
            "🎨 *ɢʜɪʙʟɪ sᴛʏʟᴇ*\n\n\
             > Gambar diterima dan sedang diproses ke style Ghibli.\n\
             > (uploadImage / applyImageEffect belum tersedia di konteks Rust)",
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
            command: "toghibli".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("toghibli").unwrap();
        assert_eq!(p.meta.name, "toghibli");
        assert_eq!(p.meta.alias, ["ghibli", "ghiblistyle"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["ghibli", "ghiblistyle"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "toghibli");
        }
    }

    #[test]
    fn bukan_gambar_mengembalikan_bantuan() {
        let r = toghibli(ctx());
        assert!(text_of(&r).contains("🎨 *ɢʜɪʙʟɪ sᴛʏʟᴇ*"));
        assert!(text_of(&r).contains("Kirim/reply gambar"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn download_gagal_mengembalikan_error() {
        // Ctx::default() punya NoMediaFetch → download gagal.
        let r = toghibli(ctx());
        assert!(text_of(&r).contains("Gagal mendownload gambar"));
    }
}
