//! `plugins/owner/goodbyeall.js` — nyalakan/matikan goodbye di semua grup.
//!
//! `ponytail:` `sock.groupFetchAllParticipating()` TIDAK di-port. Daftarnya
//! grup hanya ada di Baileys, dan `Ctx` tidak menyediakannya: `ctx.group_data`
//! adalah baris store milik SATU grup, bukan peta semua grup. Tanpa daftar itu
//! `db.setGroup(.., { leave })` tidak punya target, jadi plugin berhenti di
//! balasan "tidak tersedia" dan TIDAK menulis store apa pun. Yang dipindah:
//! parsing aksi dan teks usage.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "👋 *ɢᴏᴏᴅʙʏᴇ ɢʟᴏʙᴀʟ*\n\n\
         > Aktifkan/nonaktifkan goodbye di SEMUA grup sekaligus\n\n\
         ╭┈┈⬡「 📋 *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ* 」\n\
         ┃ {prefix}goodbyeall on\n\
         ┃ {prefix}goodbyeall off\n\
         ╰┈┈┈┈┈┈┈┈⬡"
    )
}

/// Ganti hasil setelah `groupFetchAllParticipating` — aksi yang diminta ikut
/// ditulis supaya owner tahu perintah itu tidak hilang.
pub(crate) fn unavailable(action: &str) -> String {
    format!(
        "⚠️ *ɢᴏᴏᴅʙʏᴇ ɢʟᴏʙᴀʟ ᴛɪᴅᴀᴋ ᴅɪᴘᴏʀᴛ*\n\n\
         > `sock.groupFetchAllParticipating()` tidak ada di `Ctx`, jadi daftar\n\
         > grup tidak bisa diambil dan `setGroup` tidak pernah dijalankan.\n\
         > Aksi: `{action}`"
    )
}

/// `['on','off'].includes(action)` — hanya dua aksi itu yang valid.
pub(crate) fn is_global_action(action: &str) -> bool {
    action == "on" || action == "off"
}

#[plugin(
    name = "goodbyeall",
    alias = ["gball", "globalgoodbye", "leaveall"],
    category = "owner",
    description = "Aktifkan/nonaktifkan goodbye di semua grup",
    usage = ".goodbyeall <on/off>",
    example = ".goodbyeall on",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn goodbyeall(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    if !is_global_action(&action) {
        return Reply::text(usage(&ctx.prefix));
    }
    Reply::text(unavailable(&action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(args: &[&str]) -> Reply {
        run_with(".", args)
    }

    fn run_with(prefix: &str, args: &[&str]) -> Reply {
        (find("goodbyeall").unwrap().handler)(Ctx {
            prefix: prefix.into(),
            command: "goodbyeall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        })
    }

    fn text_of(args: &[&str]) -> String {
        run(args).text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("goodbyeall").unwrap();
        assert_eq!(p.meta.name, "goodbyeall");
        assert_eq!(p.meta.alias, ["gball", "globalgoodbye", "leaveall"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Aktifkan/nonaktifkan goodbye di semua grup");
        assert_eq!(p.meta.usage, ".goodbyeall <on/off>");
        assert_eq!(p.meta.example, ".goodbyeall on");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["gball", "globalgoodbye", "leaveall"] {
            assert_eq!(find(a).unwrap().meta.name, "goodbyeall", "alias {a}");
        }
    }

    #[test]
    fn no_argument_shows_the_usage_box() {
        assert_eq!(text_of(&[]), usage("."));
        assert!(usage(".").contains("┃ .goodbyeall on"));
        assert!(usage(".").contains("┃ .goodbyeall off"));
    }

    /// JS memakai `${m.prefix}goodbyeall on`, bukan titik literal.
    #[test]
    fn the_usage_box_carries_the_live_prefix() {
        let out = run_with("!", &[]).text_body().unwrap().to_string();
        assert!(out.contains("┃ !goodbyeall on"), "{out}");
        assert!(out.contains("┃ !goodbyeall off"), "{out}");
    }

    #[test]
    fn anything_other_than_on_or_off_shows_the_usage_box() {
        // `m.args` sudah dipecah oleh whitespace, jadi tidak ada string kosong
        // atau spasi nyasar — cukup aksi yang memang tidak dikenal.
        for bad in ["nyala", "enable", "1", "ON!"] {
            assert_eq!(text_of(&[bad]), usage("."), "arg {bad:?}");
        }
    }

    #[test]
    fn the_action_is_lowercased_before_validation() {
        assert_eq!(
            text_of(&["ON"]),
            unavailable("on"),
            "JS `args[0]?.toLowerCase()`"
        );
        assert_eq!(text_of(&["Off"]), unavailable("off"));
    }

    #[test]
    fn on_and_off_are_the_only_accepted_actions() {
        assert!(is_global_action("on"));
        assert!(is_global_action("off"));
        assert!(!is_global_action("enable"));
        assert!(!is_global_action(""));
    }

    #[test]
    fn no_group_is_touched_when_the_group_list_is_unreachable() {
        // SetGroup hanya sah kalau daftar grup nyata; di sini tidak ada.
        assert!(run(&["on"]).mutations().is_empty());
        assert!(run(&["off"]).mutations().is_empty());
    }
}
