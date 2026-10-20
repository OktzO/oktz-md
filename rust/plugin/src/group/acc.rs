//! `plugins/group/acc.js` — kelola permintaan masuk grup (accept/reject).
//!
//! Yang dipindah: meta, gerbang subcommand, dan teks bantuan.
//!
//! Yang tidak dipindah: seluruh jalur kerja (`list`/`approve`/`reject`)
//! berdiri di atas `sock.groupRequestParticipantsList` (`acc.js:52`) dan
//! `sock.groupRequestParticipantsUpdate` (`:87`, `:120`) — tidak ada
//! padanannya di `Ctx`/`Reply`. JS membungkus keduanya dengan `catch` yang
//! bereaksi `☢` + template error (`acc.js:139-142`), dan jalur itulah yang
//! selalu diambil di sini — bukan mengarang daftar permintaan. Reaksi `🕕`
//! yang dikirim JS sebelum `catch` hilang; yang terkirim reaksi terakhir.
//! Kalau kapabilitas daftar request ditambahkan ke API, cabang ini yang
//! harus diganti.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Teks bantuan (`acc.js:36-46`).
fn help_text(prefix: &str) -> String {
    format!(
        "📋 *ᴊᴏɪɴ ʀᴇQᴜᴇsᴛ ᴍᴀɴᴀɢᴇʀ*\n\n\
         ╭┈┈⬡「 📌 *ᴄᴏᴍᴍᴀɴᴅ* 」\n\
         ┃ {prefix}acc list\n\
         ┃ {prefix}acc approve all\n\
         ┃ {prefix}acc reject all\n\
         ┃ {prefix}acc approve 1|2|3\n\
         ┃ {prefix}acc reject 1|2|3\n\
         ╰┈┈┈┈┈┈┈┈⬡"
    )
}

#[plugin(
    name = "acc",
    alias = ["accall", "joinrequest", "reqjoin"],
    category = "group",
    description = "Kelola permintaan masuk grup (accept/reject)",
    usage = ".acc <list|approve|reject> [all|nomor]",
    example = ".acc approve all",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn acc(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    match sub.as_deref() {
        // `!sub || !['list', 'approve', 'reject'].includes(sub)` → bantuan.
        Some("list") | Some("approve") | Some("reject") => {
            // Jalur `catch` JS (`acc.js:139-142`) — `sock.*` tidak ada di Ctx.
            Reply::text(crate::common::error_template(&ctx))
                .with_reaction("☢")
        }
        _ => Reply::text(help_text(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(args: &[&str], prefix: &str) -> Reply {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "acc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            is_group: true,
            ..Default::default()
        };
        (find("acc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaction_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("acc").unwrap();
        assert_eq!(p.meta.name, "acc");
        assert_eq!(p.meta.alias, ["accall", "joinrequest", "reqjoin"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Kelola permintaan masuk grup (accept/reject)"
        );
        assert_eq!(p.meta.usage, ".acc <list|approve|reject> [all|nomor]");
        assert_eq!(p.meta.example, ".acc approve all");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.usage.is_empty() && !p.meta.example.is_empty());
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["accall", "joinrequest", "reqjoin"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("acc"), "{a}");
        }
    }

    #[test]
    fn without_args_it_prints_the_help() {
        let t = text_of(&run(&[], "."));
        assert!(t.starts_with("📋 *ᴊᴏɪɴ ʀᴇQᴜᴇsᴛ ᴍᴀɴᴀɢᴇʀ*"), "{t}");
        assert!(t.contains("┃ .acc approve 1|2|3"), "{t}");
        assert!(t.ends_with("╰┈┈┈┈┈┈┈┈⬡"), "{t}");
    }

    #[test]
    fn the_help_follows_the_live_prefix() {
        assert!(text_of(&run(&[], "!")).contains("┃ !acc list"));
    }

    #[test]
    fn an_unknown_subcommand_also_prints_the_help() {
        assert!(text_of(&run(&["apaaja"], ".")).starts_with("📋"));
    }

    #[test]
    fn every_real_subcommand_lands_on_the_js_catch_path() {
        for sub in ["list", "approve", "reject", "LIST"] {
            let r = run(&[sub], ".");
            assert_eq!(reaction_of(&r), "☢", "sub {sub}");
            assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "acc"));
            assert!(r.mutations().is_empty());
        }
    }
}
