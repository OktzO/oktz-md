//! `plugins/owner/backupdb.js` — backup database/store dan kirim ke owner.
//!
//! `ponytail:` `sendStoreBackup(sock)` TIDAK di-port: ia membaca `database/`,
//! `db.json`, `storage/database.json` dari disk dan mengirimkannya lewat
//! `sock`, dan tidak satu pun ada di `Ctx`. `Reply` hanya bisa satu pesan,
//! sedangkan JS mengirim dua (progres lalu hasil), jadi yang dipindah adalah
//! pesan progres — satu-satunya bagian yang murni string. Balasan kedua
//! (size/files/schema) butuh hasil `sendStoreBackup` yang tidak ada.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Isi backup yang-git, urutan persis seperti di `backupdb.js`.
pub(crate) const BACKUP_CONTENTS: [&str; 6] = [
    "📁 database/*.json (semua file JSON)",
    "📁 database/cpanel/* (data cPanel)",
    "📄 storage/database.json (main database)",
    "📄 db.json (root database)",
    "📄 database/main/*.json (main database)",
    "📋 backup_metadata.json (info schema)",
];

/// `backupContents.map(c => \`┃ ${c}\`).join('\n')` di dalam kepala/ekor pesan.
pub(crate) fn contents_list() -> String {
    BACKUP_CONTENTS
        .iter()
        .map(|c| format!("┃ {c}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[plugin(
    name = "backupdb",
    alias = ["dbbackup", "backupstore", "storebackup"],
    category = "owner",
    description = "Backup database/store dan kirim ke owner",
    usage = ".backupdb",
    example = "",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn backupdb(_ctx: Ctx) -> Reply {
    Reply::text(format!(
        "🕕 *Membuat backup database...*\n\n\
         ╭┈┈⬡「 📦 *ᴀᴘᴀ ʏᴀɴɢ ᴅɪ-ʙᴀᴄᴋᴜᴘ* 」\n\
         {}\n\
         ╰┈┈┈┈┈┈┈┈⬡",
        contents_list()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run() -> String {
        (find("backupdb").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "backupdb".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
        .text_body()
        .expect("harus ada teks")
        .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("backupdb").unwrap();
        assert_eq!(p.meta.name, "backupdb");
        assert_eq!(p.meta.alias, ["dbbackup", "backupstore", "storebackup"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Backup database/store dan kirim ke owner");
        assert_eq!(p.meta.usage, ".backupdb");
        assert_eq!(p.meta.cooldown, 3, "tidak ada di JS, jadi defaultConfig");
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["dbbackup", "backupstore", "storebackup"] {
            assert_eq!(find(a).unwrap().meta.name, "backupdb", "alias {a}");
        }
    }

    #[test]
    fn all_six_sources_are_listed_in_js_order() {
        let out = run();
        let listed: Vec<&str> = out
            .lines()
            .filter_map(|l| l.strip_prefix("┃ "))
            .filter(|l| BACKUP_CONTENTS.contains(l))
            .collect();
        assert_eq!(listed, BACKUP_CONTENTS);
    }

    #[test]
    fn the_progress_message_carries_its_header_and_footer() {
        let out = run();
        assert!(out.starts_with("🕕 *Membuat backup database...*"));
        assert!(out.contains("╭┈┈⬡「 📦 *ᴀᴘᴀ ʏᴀɴɢ ᴅɪ-ʙᴀᴄᴋᴜᴘ* 」"));
        assert!(out.ends_with("╰┈┈┈┈┈┈┈┈⬡"), "footer penutup: {out}");
    }

    #[test]
    fn nothing_is_written_to_the_store() {
        let r = (find("backupdb").unwrap().handler)(Ctx::default());
        assert!(r.mutations().is_empty());
    }
}
