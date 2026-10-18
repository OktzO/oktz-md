//! `plugins/group/welcome.js` — sakelar pesan sambutan per grup.
//!
//! Yang dipindah: seluruh `handler` (`on`, `off`, status, gerbang owner
//! `on all`/`off all`).
//!
//! Yang tidak dipindah:
//! - Aksi `on all`/`off all` butuh `sock.groupFetchAllParticipating()` (`:390`,
//!   `:413`) untuk tahu grup mana saja. `Reply`/pipeline tidak punya daftar
//!   grup, jadi cabang itu berhenti di jalur `catch` miliknya sendiri —
//!   reaksi `☢` + template error — bukan mengarang "diaktifkan di N grup"
//!   (pola `goodbye.rs`). Gerbang owner di depan `sock` tetap dipindah karena
//!   itu yang dilihat user.
//! - Dua reaksi berurutan (`🕕` lalu `✅`/`☢`) tidak bisa dibawa:
//!   `Reply::Reaction` hanya menyimpan satu emoji, jadi yang terkirim adalah
//!   reaksi terakhir.
//! - `sendWelcomeMessage` (`:123`) bukan handler plugin: ia butuh
//!   `sock.profilePictureUrl`, `prepareWAMessageMedia`/`relayMessage`,
//!   `sock.sendPreview`/`sendText`, `getAssetBuffer`, `saluranCtx()`, dan
//!   `welcomeType` 2..8.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

/// `config.messages.ownerOnly` (`config.js:198`) — literal statis di config,
/// sama seperti `CONFIG_PREFIX` di `common.rs`.
const OWNER_ONLY: &str = "*Akses Ditolak!* Fitur ini khusus untuk Owner bot.";

fn already_active(prefix: &str) -> String {
    format!(
        "⚠️ *ᴡᴇʟᴄᴏᴍᴇ ᴀʟʀᴇᴀᴅʏ ᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *✅ ON*\n\
         > Welcome sudah aktif di grup ini.\n\n\
         _Gunakan `{prefix}welcome off` untuk menonaktifkan._"
    )
}

fn already_inactive(prefix: &str) -> String {
    format!(
        "⚠️ *ᴡᴇʟᴄᴏᴍᴇ ᴀʟʀᴇᴀᴅʏ ɪɴᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *❌ OFF*\n\
         > Welcome sudah nonaktif di grup ini.\n\n\
         _Gunakan `{prefix}welcome on` untuk mengaktifkan._"
    )
}

fn turned_on(prefix: &str) -> String {
    format!(
        "✅ *ᴡᴇʟᴄᴏᴍᴇ ᴀᴋᴛɪꜰ*\n\n\
         > Welcome message berhasil diaktifkan!\n\
         > Member baru akan disambut otomatis.\n\n\
         _Gunakan `{prefix}setwelcome` untuk custom pesan._"
    )
}

fn turned_off() -> String {
    "❌ *ᴡᴇʟᴄᴏᴍᴇ ɴᴏɴᴀᴋᴛɪꜰ*\n\n\
     > Welcome message berhasil dinonaktifkan.\n\
     > Member baru tidak akan disambut."
        .to_string()
}

fn settings_text(prefix: &str, on: bool) -> String {
    format!(
        "👋 *ᴡᴇʟᴄᴏᴍᴇ sᴇᴛᴛɪɴɢs*\n\n\
         > Status: *{status}*\n\n\
         ```━━━ ᴘɪʟɪʜᴀɴ ━━━```\n\
         > `{prefix}welcome on` → Aktifkan\n\
         > `{prefix}welcome off` → Nonaktifkan\n\
         > `{prefix}welcome on all` → Global ON (owner)\n\
         > `{prefix}welcome off all` → Global OFF (owner)\n\
         > `{prefix}setwelcome` → Custom pesan\n\
         > `{prefix}resetwelcome` → Reset default",
        status = if on { "✅ ON" } else { "❌ OFF" },
    )
}

/// Cabang `* all`: gerbang owner, lalu jalur `catch` milik JS sendiri karena
/// `groupFetchAllParticipating` tidak ada di `Ctx`.
fn global_unavailable(ctx: &Ctx) -> Reply {
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[plugin(
    name = "welcome",
    alias = ["wc"],
    category = "group",
    description = "Mengatur welcome message untuk grup",
    usage = ".welcome <on/off>",
    example = ".welcome on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn welcome(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let sub2 = ctx.args.get(1).map(|a| a.to_lowercase());
    // `groupData.welcome === true` — perbandingan ketat, nilai `"true"` atau
    // `1` di store tidak dihitung aktif.
    let current = ctx.group_field("welcome") == Some(&Value::Bool(true));

    if sub.as_deref() == Some("on") && sub2.as_deref() == Some("all") {
        if !ctx.is_owner {
            return Reply::text(OWNER_ONLY);
        }
        return global_unavailable(&ctx);
    }

    if sub.as_deref() == Some("off") && sub2.as_deref() == Some("all") {
        if !ctx.is_owner {
            return Reply::text(OWNER_ONLY);
        }
        return global_unavailable(&ctx);
    }

    match sub.as_deref() {
        Some("on") => {
            if current {
                return Reply::text(already_active(&ctx.prefix));
            }
            Reply::text(turned_on(&ctx.prefix)).with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: serde_json::json!({ "welcome": true }),
            })
        }
        Some("off") => {
            if !current {
                return Reply::text(already_inactive(&ctx.prefix));
            }
            Reply::text(turned_off()).with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: serde_json::json!({ "welcome": false }),
            })
        }
        _ => Reply::text(settings_text(&ctx.prefix, current)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(group: Value, args: &[&str], is_owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "welcome".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            is_group: true,
            is_owner,
            group_data: group,
            ..Default::default()
        };
        (find("welcome").unwrap().handler)(ctx)
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
        let p = find("welcome").unwrap();
        assert_eq!(p.meta.name, "welcome");
        assert_eq!(p.meta.alias, ["wc"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mengatur welcome message untuk grup");
        assert_eq!(p.meta.usage, ".welcome <on/off>");
        assert_eq!(p.meta.example, ".welcome on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_bot_admin);
        assert!(!p.meta.usage.is_empty() && !p.meta.example.is_empty());
    }

    #[test]
    fn the_alias_resolves_here() {
        assert_eq!(find("wc").map(|e| e.meta.name.as_str()), Some("welcome"));
    }

    #[test]
    fn turning_on_writes_the_group_flag() {
        let r = run(serde_json::json!({}), &["on"], false);
        assert_eq!(text_of(&r), turned_on("."));
        assert_eq!(
            r.mutations(),
            &[Mutation::SetGroup {
                jid: "120363@g.us".into(),
                data: serde_json::json!({ "welcome": true }),
            }]
        );
    }

    #[test]
    fn turning_on_twice_says_already_active_and_writes_nothing() {
        let r = run(serde_json::json!({ "welcome": true }), &["on"], false);
        assert_eq!(text_of(&r), already_active("."));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn turning_off_writes_the_group_flag() {
        let r = run(serde_json::json!({ "welcome": true }), &["off"], false);
        assert_eq!(text_of(&r), turned_off());
        assert_eq!(
            r.mutations(),
            &[Mutation::SetGroup {
                jid: "120363@g.us".into(),
                data: serde_json::json!({ "welcome": false }),
            }]
        );
    }

    #[test]
    fn turning_off_twice_says_already_inactive_and_writes_nothing() {
        let r = run(serde_json::json!({}), &["off"], false);
        assert_eq!(text_of(&r), already_inactive("."));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn without_args_it_shows_the_settings_with_the_live_status() {
        let off = text_of(&run(serde_json::json!({}), &[], false));
        assert!(off.starts_with("👋 *ᴡᴇʟᴄᴏᴍᴇ sᴇᴛᴛɪɴɢs*"), "{off}");
        assert!(off.contains("> Status: *❌ OFF*"), "{off}");
        assert!(off.contains("> `.welcome on all` → Global ON (owner)"), "{off}");

        let on = text_of(&run(serde_json::json!({ "welcome": true }), &[], false));
        assert!(on.contains("> Status: *✅ ON*"), "{on}");
    }

    #[test]
    fn the_active_check_is_strict_equality_with_true() {
        // `"true"` (string) dan `1` tidak dihitung aktif, persis `=== true`.
        for weird in [serde_json::json!("true"), serde_json::json!(1)] {
            assert!(
                text_of(&run(serde_json::json!({ "welcome": weird }), &["on"], false)).contains("ᴀᴋᴛɪꜰ"),
                "harus tetap bisa dinyalakan: {weird}"
            );
        }
    }

    #[test]
    fn global_on_for_an_owner_lands_on_the_js_catch_path() {
        let r = run(serde_json::json!({}), &["on", "all"], true);
        assert_eq!(reaction_of(&r), "☢");
        assert_eq!(
            text_of(&r),
            crate::common::error_template(".", "welcome")
        );
        assert!(r.mutations().is_empty(), "tidak ada grup yang ditulis");
    }

    #[test]
    fn global_off_for_an_owner_lands_on_the_js_catch_path_too() {
        let r = run(serde_json::json!({}), &["off", "all"], true);
        assert_eq!(reaction_of(&r), "☢");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn global_branches_refuse_non_owners_before_touching_sock() {
        for args in [["on", "all"], ["off", "all"]] {
            let r = run(serde_json::json!({}), &args, false);
            assert_eq!(text_of(&r), OWNER_ONLY);
            assert!(r.mutations().is_empty());
        }
    }
}
