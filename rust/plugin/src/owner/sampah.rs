//! `plugins/owner/sampah.js` — menghapus semua sampah di folder `temp`.
//!
//! `NOTE (mod.rs):` `owner/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/owner/mod.rs`:
//!   pub mod sampah;
use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "sampah",
    alias = ["clearsampah", "cleartemp", "deltemp"],
    category = "owner",
    description = "Menghapus semua sampah di temp",
    usage = ".sampah",
    example = ".sampah",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn sampah(ctx: Ctx) -> Reply {
    let berkas = ctx.berkas();

    if !berkas.ada("temp") {
        return Reply::text("❌ Folder temp tidak ditemukan!");
    }

    let files = match berkas.daftar("temp") {
        Ok(list) => list,
        Err(_) => return Reply::text("❌ Gagal membaca isi folder temp"),
    };

    if files.is_empty() {
        return Reply::text("📁 Folder temp sudah kosong!");
    }

    let mut deleted = 0usize;
    for nama in &files {
        let path = format!("temp/{nama}");
        if berkas.hapus(&path).is_ok() {
            deleted += 1;
        }
    }

    Reply::text(format!(
        "🗑️ *TEMP CLEANED!*\n\n\
         > Total file/folder dihapus: *{deleted}*"
    ))
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::BerkasPalsu;

    fn ctx_sampah(berkas: std::sync::Arc<dyn oktz_plugin_api::berkas::Berkas>) -> Ctx {
        Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "sampah".into(),
            is_owner: true,
            berkas,
            ..Ctx::default()
        }
    }

    #[test]
    fn folder_temp_tidak_ada_mengembalikan_pesan() {
        let b = BerkasPalsu::baru();
        let r = sampah(ctx_sampah(std::sync::Arc::new(b)));
        assert_eq!(r.text_body().unwrap(), "❌ Folder temp tidak ditemukan!");
    }

    #[test]
    fn folder_temp_kosong_mengembalikan_pesan() {
        let b = BerkasPalsu::baru();
        b.buat_folder("temp").unwrap();
        let r = sampah(ctx_sampah(std::sync::Arc::new(b)));
        assert_eq!(r.text_body().unwrap(), "📁 Folder temp sudah kosong!");
    }

    #[test]
    fn menghapus_file_dalam_temp() {
        let b = BerkasPalsu::baru();
        b.buat_folder("temp").unwrap();
        b.tulis_teks("temp/a.txt", "isi a").unwrap();
        b.tulis_teks("temp/b.txt", "isi b").unwrap();
        let r = sampah(ctx_sampah(std::sync::Arc::new(b)));
        let text = r.text_body().unwrap();
        assert!(text.contains("dihapus: *2*"));
        assert!((b.ada("temp/a.txt") == false));
        assert!((b.ada("temp/b.txt") == false));
    }
}
