//! `plugins/owner/welcomeall.js` — nyalakan/matikan welcome di semua grup.
//!
//! `ponytail:` sama seperti `goodbyeall.js`: `sock.groupFetchAllParticipating()`
//! tidak di-port, jadi daftar grup tidak bisa diambil dan `db.setGroup(..,
//! { welcome })` tidak punya target. Lihat catatan di `goodbyeall.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "👋 *ᴡᴇʟᴄᴏᴍᴇ ɢʟᴏʙᴀʟ*\n\n\
         > Aktifkan/nonaktifkan welcome di SEMUA grup sekaligus\n\n\
         ╭┈┈⬡「 📋 *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ* 」\n\
         ┃ {prefix}welcomeall on\n\
         ┃ {prefix}welcomeall off\n\
         ╰┈┈┈┈┈┈┈┈⬡"
    )
}

pub(crate) fn unavailable(action: &str) -> String {
    format!(
        "⚠️ *ᴡᴇʟᴄᴏᴍᴇ ɢʟᴏʙᴀʟ ᴛɪᴅᴀᴋ ᴅɪᴘᴏʀᴛ*\n\n\
         > `sock.groupFetchAllParticipating()` tidak ada di `Ctx`, jadi daftar\n\
         > grup tidak bisa diambil dan `setGroup` tidak pernah dijalankan.\n\
         > Aksi: `{action}`"
    )
}

pub(crate) fn is_global_action(action: &str) -> bool {
    action == "on" || action == "off"
}

#[plugin(
    name = "welcomeall",
    alias = ["wcall", "globalwelcome"],
    category = "owner",
    description = "Aktifkan/nonaktifkan welcome di semua grup",
    usage = ".welcomeall <on/off>",
    example = ".welcomeall on",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn welcomeall(ctx: Ctx) -> Reply {
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
        (find("welcomeall").unwrap().handler)(Ctx {
            prefix: prefix.into(),
            command: "welcomeall".into(),
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
        let p = find("welcomeall").unwrap();
        assert_eq!(p.meta.name, "welcomeall");
        assert_eq!(p.meta.alias, ["wcall", "globalwelcome"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Aktifkan/nonaktifkan welcome di semua grup");
        assert_eq!(p.meta.usage, ".welcomeall <on/off>");
        assert_eq!(p.meta.example, ".welcomeall on");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["wcall", "globalwelcome"] {
            assert_eq!(find(a).unwrap().meta.name, "welcomeall", "alias {a}");
        }
    }

    #[test]
    fn no_argument_shows_the_welcome_usage_box() {
        assert_eq!(text_of(&[]), usage("."));
        assert!(usage(".").contains("┃ .welcomeall on"));
        assert!(usage(".").contains("┃ .welcomeall off"));
    }

    /// JS memakai `${m.prefix}welcomeall on`, bukan titik literal.
    #[test]
    fn the_usage_box_carries_the_live_prefix() {
        let out = run_with("!", &[]).text_body().unwrap().to_string();
        assert!(out.contains("┃ !welcomeall on"), "{out}");
        assert!(out.contains("┃ !welcomeall off"), "{out}");
    }

    #[test]
    fn an_unrelated_argument_is_treated_as_no_argument() {
        for bad in ["nyala", "enable", "off=1"] {
            assert_eq!(text_of(&[bad]), usage("."), "arg {bad:?}");
        }
    }

    #[test]
    fn the_action_is_lowercased_before_validation() {
        assert_eq!(text_of(&["ON"]), unavailable("on"));
        assert_eq!(text_of(&["OFF"]), unavailable("off"));
    }

    #[test]
    fn it_stays_distinct_from_goodbyeall() {
        let g = crate::owner::goodbyeall::usage(".");
        assert_ne!(usage("."), g, "dua plugin tidak boleh berbagi teks usage");
        assert!(usage(".").contains("welcome") && !usage(".").contains("goodbye"));
    }

    #[test]
    fn no_group_is_touched_when_the_group_list_is_unreachable() {
        assert!(run(&["on"]).mutations().is_empty());
        assert!(run(&["off"]).mutations().is_empty());
    }
}
