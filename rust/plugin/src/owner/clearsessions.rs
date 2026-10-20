//! `plugins/owner/clearsessions.js` — hapus isi `storage/sessions/` kecuali
//! `creds.json`.
//!
//! JS menghapus folder secara rekursif (`fs.rmSync(..., { recursive: true })`);
//! `Berkas::hapus` hanya bisa menghapus berkas, jadi subfolder di dalam
//! `storage/sessions/` dilewati diam-diam — persis `catch {}` kosong di JS,
//! hanya saja tidak ikut terhitung sebagai `deleted`.

use oktz_plugin_api::{Ctx, Reply, plugin};

const FOLDER: &str = "storage/sessions";
const CREDS: &str = "creds.json";

/// `clearsessions.js:48-58`.
pub(crate) fn ringkasan(dihapus: usize, dilewati: usize) -> String {
    format!(
        "╭┈┈⬡「 🗑️ *ᴄʟᴇᴀʀ sᴇssɪᴏɴs* 」\n\
         ┃\n\
         ┃ ㊗ ᴅᴇʟᴇᴛᴇᴅ: *{dihapus}* file\n\
         ┃ ㊗ sᴋɪᴘᴘᴇᴅ: *{dilewati}* file\n\
         ┃ ㊗ ɴᴏᴛᴇ: creds.json tidak dihapus\n\
         ┃\n\
         ╰┈┈⬡\n\n\
         > _Session files berhasil dibersihkan!_\n\
         > _Restart bot jika diperlukan._"
    )
}

#[plugin(
    name = "clearsessions",
    alias = ["clearsession", "delsession", "delsessions"],
    category = "owner",
    description = "Menghapus semua session di storage/sessions/",
    usage = ".clearsessions",
    example = ".clearsessions",
    cooldown = 60,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clearsessions(ctx: Ctx) -> Reply {
    let Ok(isi) = ctx.berkas().daftar(FOLDER) else {
        return Reply::text("❌ Folder sessions tidak ditemukan!");
    };

    let sapu = Reply::None.with_reaction("🗑️");

    if isi.is_empty() {
        return Reply::sequence(vec![sapu, Reply::text("📁 Folder sessions sudah kosong!")]);
    }

    let mut dihapus = 0;
    let mut dilewati = 0;
    for nama in &isi {
        if nama == CREDS {
            dilewati += 1;
            continue;
        }
        if ctx.berkas().hapus(&format!("{FOLDER}/{nama}")).is_ok() {
            dihapus += 1;
        }
    }

    Reply::sequence(vec![
        sapu,
        Reply::None.with_reaction("✅"),
        Reply::text(ringkasan(dihapus, dilewati)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::uji::{reaksi, teks};
    use oktz_plugin_api::berkas::{Berkas, BerkasPalsu};
    use std::sync::Arc;

    fn ctx_of(berkas: BerkasPalsu) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "clearsessions".into(),
            is_owner: true,
            berkas: Arc::new(berkas),
            ..Ctx::default()
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("clearsessions").unwrap();
        assert_eq!(p.meta.alias, ["clearsession", "delsession", "delsessions"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 60);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn folder_kosong_dibalas_setelah_reaksi_sapu() {
        let r = clearsessions(ctx_of(BerkasPalsu::baru()));
        assert_eq!(reaksi(&r), ["🗑️"]);
        assert_eq!(teks(&r), "📁 Folder sessions sudah kosong!");
    }

    #[test]
    fn creds_json_dilewati_sisanya_dihapus() {
        let b = BerkasPalsu::baru();
        for nama in [CREDS, "a.json", "b.json"] {
            let _ = b.tulis(&format!("{FOLDER}/{nama}"), b"x");
        }
        let r = clearsessions(ctx_of(b));
        assert_eq!(reaksi(&r), ["🗑️", "✅"]);
        let isi = teks(&r);
        assert!(isi.contains("ᴅᴇʟᴇᴛᴇᴅ: *2* file"), "{isi}");
        assert!(isi.contains("sᴋɪᴘᴘᴇᴅ: *1* file"), "{isi}");
    }
}
