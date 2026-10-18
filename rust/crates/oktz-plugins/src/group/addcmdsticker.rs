//! `plugins/group/addcmdsticker.js` — daftarkan sticker sebagai shortcut command.
//!
//! `ponytail:` dua hal tidak ikut.
//!
//! 1. `getQuotedStickerHash` membaca `fileSha256`, lalu `mediaKey`, dari
//!    `quoted.message.stickerMessage` (`sticker-command.js:80-101`).
//!    `QuotedMessage` tidak punya keduanya, jadi tidak ada kunci yang bisa
//!    ditulis — dan karena pemeriksaan itu mendahului `getPlugin`, semua
//!    cabang setelah "reply sticker" mati. Plugin diam, bukanAddition yang
//!    menipu.
//! 2. `src/lib/sticker-command.js` menulis ke `database/stickerCommands.json`,
//!    bukan ke store plugin. Pemetaannya ke store global: key
//!    `setting("stickerCommands")` dengan bentuk yang sama persis
//!    (hash → `{command, addedBy, addedAt}`) — sama yang dipakai
//!    `delstickercmd`.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `database/stickerCommands.json` → `setting("stickerCommands")`.
pub(crate) const STORE: &str = "stickerCommands";

pub(crate) const NO_QUOTE: &str = "⚠️ *Reply sticker* yang ingin dijadikan command!";
pub(crate) const NOT_STICKER: &str = "⚠️ Pesan yang di-reply bukan *sticker*!";

/// `commandName.toLowerCase().replace(/^\./, '')` — satu titik di depan saja.
/// Cabang JS yang memakainya sudah mati (lihat `ponytail:` modul), jadi hanya
/// tes yang memakai helper ini.
#[cfg(test)]
pub(crate) fn clean_cmd(raw: &str) -> String {
    let lower = raw.to_lowercase();
    lower.strip_prefix('.').unwrap_or(&lower).to_string()
}

/// Nama command yang terdaftar, sesuai urutan key di store — persis urutan
/// iterasi `Object.entries(stickerCommands)` di JS.
pub(crate) fn registered(ctx: &Ctx) -> Vec<String> {
    ctx.setting_entries(STORE)
        .map(|(_, v)| {
            v.get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

/// Blok pembuka tetap + kartu "AKTIF" kalau store tidak kosong.
pub(crate) fn help_text(names: &[String]) -> String {
    let mut txt = String::from(
        "🖼️ *sᴛɪᴄᴋᴇʀ ᴛᴏ ᴄᴏᴍᴍᴀɴᴅ*\n\n\
         > Reply sticker + ketik command yang ingin dijadikan shortcut.\n\n\
         *Contoh:*\n\
         > Reply sticker, lalu ketik:\n\
         > `.addcmdsticker menu`\n\n",
    );
    if !names.is_empty() {
        txt.push_str("╭┈┈⬡「 📋 *ᴀᴋᴛɪꜰ」\n");
        for cmd in names.iter().take(10) {
            txt.push_str(&format!("┃ 🖼️ → `{cmd}`\n"));
        }
        if names.len() > 10 {
            txt.push_str(&format!("┃ ... dan {} lainnya\n", names.len() - 10));
        }
        txt.push_str("╰┈┈┈┈┈┈┈┈⬡");
    }
    txt
}

#[plugin(
    name = "addcmdsticker",
    alias = ["addstickercmd", "setsticker", "stickeradd"],
    category = "group",
    description = "Jadikan sticker sebagai shortcut command",
    usage = ".addcmdsticker <command> (reply sticker)",
    example = ".addcmdsticker menu",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn addcmdsticker(ctx: Ctx) -> Reply {
    let command_name = ctx.args.first().cloned();

    if command_name.is_none() {
        return Reply::text(help_text(&registered(&ctx)));
    }

    let Some(quoted) = ctx.quoted() else {
        return Reply::text(NO_QUOTE);
    };

    if !quoted.kind.is_sticker() {
        return Reply::text(NOT_STICKER);
    }

    // `getQuotedStickerHash` tidak bisa dihitung (lihat `ponytail:` modul).
    // Karena pemeriksaan itu mendahului `getPlugin(cleanCmd)` di JS, semua
    // cabang setelahnya mati — plugin diam, bukan melapor sukses tanpa kunci
    // yang benar-benar ditulis.
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn quoted(kind: QuotedKind) -> Option<QuotedMessage> {
        Some(QuotedMessage {
            id: "MSG1".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        })
    }

    fn run(args: &[&str], settings: serde_json::Value, q: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addcmdsticker".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            quoted: q,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("addcmdsticker").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn full() -> serde_json::Value {
        serde_json::json!({"stickerCommands": {
            "hashaaa": {"command": "menu", "addedBy": "628111", "addedAt": 1},
            "hashbbb": {"command": "promo", "addedBy": "628222", "addedAt": 2}
        }})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addcmdsticker").unwrap();
        assert_eq!(p.meta.name, "addcmdsticker");
        assert_eq!(
            p.meta.alias,
            ["addstickercmd", "setsticker", "stickeradd"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Jadikan sticker sebagai shortcut command");
        assert_eq!(p.meta.usage, ".addcmdsticker <command> (reply sticker)");
        assert_eq!(p.meta.example, ".addcmdsticker menu");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        // JS: isGroup false DAN isPrivate false — command yang tidak jalan di
        // mode mana pun.
        assert!(!p.meta.is_group);
        assert!(!p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["addstickercmd", "setsticker", "stickeradd"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("addcmdsticker")
            );
        }
    }

    #[test]
    fn no_argument_shows_the_help_without_the_active_card() {
        for s in [serde_json::json!({}), serde_json::json!({"stickerCommands": {}})] {
            let __balasan = run(&[], s.clone(), None);
            let t = body(&__balasan);
            assert!(t.starts_with("🖼️ *sᴛɪᴄᴋᴇʀ ᴛᴏ ᴄᴏᴍᴍᴀɴᴅ*"), "{s}: {t}");
            assert!(t.contains("> Reply sticker, lalu ketik:\n> `.addcmdsticker menu`"), "{t}");
            assert!(!t.contains("ᴀᴋᴛɪꜰ"), "kartu hanya muncul kalau ada entri: {t}");
            assert!(run(&[], s, None).mutations().is_empty());
        }
    }

    #[test]
    fn the_active_card_lists_commands_in_store_order() {
        let __balasan = run(&[], full(), None);
        let t = body(&__balasan);
        assert!(t.contains("╭┈┈⬡「 📋 *ᴀᴋᴛɪꜰ」\n"), "{t}");
        assert!(t.contains("┃ 🖼️ → `menu`\n┃ 🖼️ → `promo`\n"), "{t}");
        assert!(t.ends_with("╰┈┈┈┈┈┈┈┈⬡"), "{t}");
    }

    #[test]
    fn more_than_ten_entries_show_the_overflow_count() {
        let mut map = serde_json::Map::new();
        for i in 0..13 {
            map.insert(
                format!("hash{i:02}"),
                serde_json::json!({"command": format!("cmd{i:02}"), "addedBy": "628111", "addedAt": i}),
            );
        }
        let s = serde_json::json!({"stickerCommands": map});
        let __balasan = run(&[], s.clone(), None);
        let t = body(&__balasan);
        assert!(t.contains("┃ 🖼️ → `cmd09`\n"), "10 terakhir yang tampil: {t}");
        assert!(!t.contains("`cmd10`"), "potong di 10: {t}");
        assert!(t.contains("┃ ... dan 3 lainnya\n"), "13 - 10 = 3: {t}");

        // Tepat 10 → tidak ada baris overflow.
        let mut map10 = serde_json::Map::new();
        for i in 0..10 {
            map10.insert(format!("h{i}"), serde_json::json!({"command": format!("c{i}")}));
        }
        let __balasan = run(&[], serde_json::json!({ "stickerCommands": map10 }), None);
        let t = body(&__balasan);
        assert!(!t.contains("lainnya"), "tepat 10 tidak overflow: {t}");
    }

    #[test]
    fn a_command_without_a_reply_is_rejected() {
        let r = run(&["menu"], full(), None);
        assert_eq!(body(&r), NO_QUOTE);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_reply_that_is_not_a_sticker_is_rejected() {
        for kind in [
            QuotedKind::Text,
            QuotedKind::Image,
            QuotedKind::Video,
            QuotedKind::Document,
        ] {
            let r = run(&["menu"], full(), quoted(kind.clone()));
            assert_eq!(body(&r), NOT_STICKER, "jenis {}", kind.wire_type());
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_sticker_reply_stores_nothing_while_the_hash_is_unreachable() {
        let r = run(&["menu"], full(), quoted(QuotedKind::Sticker));
        assert!(r.is_none(), "tidak boleh ada teks yang iets");
        assert!(
            r.mutations().is_empty(),
            "tanpa fileSha256 tidak ada kunci hash yang jujur"
        );
    }

    #[test]
    fn the_command_name_is_lowercased_with_one_leading_dot_stripped() {
        assert_eq!(clean_cmd("Menu"), "menu");
        assert_eq!(clean_cmd(".PROMO"), "promo");
        assert_eq!(clean_cmd("..promo"), ".promo");
        assert_eq!(clean_cmd("menu."), "menu.");
    }
}