//! `plugins/owner/savedb.js` — kirim `database/db.json` sebagai dokumen.
//!
//! Jalur berkasnya mengikuti JS apa adanya (`database/db.json`), bukan letak
//! store Rust (`database/settings.json`) — permintaan owner yang men-download
//! database hasil backup tetap file yang sama dengan produksi JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::{jakarta_parts, now_ms, pad2};

const PATH: &str = "database/db.json";

/// `formatPattern(now, 'YYYY-MM-DD_HH-mm-ss')` (`ganti-namabot`/`savedb`).
pub(crate) fn nama_backup(ms: i64) -> String {
    let (y, bln, hari, jam, menit, detik) = jakarta_parts(ms);
    format!(
        "db_backup_{y}-{}-{}_{}-{}-{}.json",
        pad2(bln),
        pad2(hari),
        pad2(jam),
        pad2(menit),
        pad2(detik)
    )
}

/// `(stats.size / 1024).toFixed(2)`.
fn ukuran_kb(bytes: usize) -> String {
    format!("{:.2}", bytes as f64 / 1024.0)
}

/// Caption `savedb.js:35-43`.
fn caption(ms: i64, kb: &str) -> String {
    let (y, bln, hari, jam, menit, detik) = jakarta_parts(ms);
    format!(
        "📦 *ᴅᴀᴛᴀʙᴀsᴇ ʙᴀᴄᴋᴜᴘ*\n\n\
         ╭┈┈⬡「 📋 *ɪɴғᴏ* 」\n\
         ┃ 📁 File: `db.json`\n\
         ┃ 📊 Size: `{kb} KB`\n\
         ┃ 📅 Date: `{}/{}/{y}`\n\
         ┃ ⏰ Time: `{}-{}-{}`\n\
         ╰┈┈┈┈┈┈┈┈⬡",
        pad2(hari),
        pad2(bln),
        pad2(jam),
        pad2(menit),
        pad2(detik),
    )
}

#[plugin(
    name = "savedb",
    alias = ["backupdb", "downloaddb", "getdb"],
    category = "owner",
    description = "Download file database",
    usage = ".savedb",
    example = ".savedb",
    cooldown = 30,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn savedb(ctx: Ctx) -> Reply {
    // `savedb.js:17-19` — gate di dalam handler; router sudah menolak
    // non-owner lebih dulu lewat `isOwner: true` di metadata.
    if !ctx.is_owner {
        return Reply::text("❌ *Owner Only!*");
    }

    if !ctx.berkas().ada(PATH) {
        return Reply::text("❌ File database tidak ditemukan!");
    }

    let Ok(data) = ctx.berkas().baca(PATH) else {
        return Reply::text(crate::common::error_template(&ctx));
    };

    let ms = now_ms();
    let kb = ukuran_kb(data.len());
    let nama = nama_backup(ms);

    Reply::document(MediaSource::bytes(data), nama).with_caption(caption(ms, &kb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::{Berkas, BerkasPalsu};
    use std::sync::Arc;

    fn ctx_of(berkas: BerkasPalsu, owner: bool) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "savedb".into(),
            is_owner: owner,
            berkas: Arc::new(berkas),
            ..Ctx::default()
        }
    }

    fn isi() -> BerkasPalsu {
        let b = BerkasPalsu::baru();
        let _ = b.tulis(PATH, b"{\"a\":1}");
        b
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("savedb").unwrap();
        assert_eq!(p.meta.name, "savedb");
        assert_eq!(p.meta.alias, ["backupdb", "downloaddb", "getdb"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Download file database");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn bukan_owner_ditolak() {
        assert_eq!(savedb(ctx_of(isi(), false)), Reply::text("❌ *Owner Only!*"));
    }

    #[test]
    fn tanpa_berkas_membalas_tidak_ditemukan() {
        assert_eq!(
            savedb(ctx_of(BerkasPalsu::baru(), true)),
            Reply::text("❌ File database tidak ditemukan!")
        );
    }

    #[test]
    fn berkas_dikirim_sebagai_dokumen_json_dengan_caption() {
        let r = savedb(ctx_of(isi(), true));
        let Reply::Document {
            media,
            file_name,
            caption,
            ..
        } = r.without_mutations()
        else {
            panic!("harus dokumen");
        };
        assert!(file_name.starts_with("db_backup_"), "{file_name}");
        assert!(file_name.ends_with(".json"));
        match media {
            Some(MediaSource::Bytes { data, .. }) => assert_eq!(&**data, b"{\"a\":1}"),
            lain => panic!("harus byte, dapat {lain:?}"),
        }
        assert!(caption.contains("📁 File: `db.json`"));
        // 7 byte / 1024 = 0.01 (dua desimal, seperti toFixed(2)).
        assert!(caption.contains("📊 Size: `0.01 KB`"), "{caption}");
        assert!(caption.contains("╰┈┈┈┈┈┈┈┈⬡"));
    }

    #[test]
    fn nama_menang_atas_alias_saat_kunci_sama() {
        // `savedb` memakai alias `backupdb`; `plugins.js:539` mendahulukan
        // nama, jadi `.backupdb` tetap milik `plugins/owner/backupdb.js`.
        assert_eq!(
            oktz_plugin_api::find("backupdb").unwrap().meta.name,
            "backupdb"
        );
        assert_eq!(
            oktz_plugin_api::find("downloaddb").unwrap().meta.name,
            "savedb"
        );
    }

    #[test]
    fn nama_backup_memakai_pola_js() {
        // 2025-10-21 12:34:56 WIB = 2025-10-21T05:34:56Z.
        let ms = 1_761_024_896_000;
        let nama = nama_backup(ms);
        assert_eq!(nama, "db_backup_2025-10-21_12-34-56.json");
    }
}
