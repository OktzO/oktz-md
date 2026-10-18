//! `plugins/group/goodbye.js` — sakelar pesan perpisahan per grup.
//!
//! Yang dipindah: seluruh `handler` (`on`, `off`, status).
//!
//! Yang tidak dipindah:
//! - `on all`/`off all` butuh `sock.groupFetchAllParticipating()` (`:441`,
//!   `:464`) untuk tahu grup mana saja. `Reply`/pipeline tidak punya daftar
//!   grup, jadi cabang itu berhenti di jalur `catch` miliknya sendiri —
//!   reaksi `☢` + template error — bukan mengarang "diaktifkan di N grup".
//!   Gerbang owner di depan `sock` tetap dipindah karena itu yang dilihat user.
//! - Dua reaksi berurutan (`🕕` lalu `☢`) tidak bisa dibawa: `Reply::Reaction`
//!   hanya menyimpan satu emoji, jadi yang dikirim adalah reaksi terakhir.
//! - `sendGoodbyeMessage` (`:125`) bukan handler plugin: ia butuh
//!   `sock.profilePictureUrl`, `sock.waUploadToServer`,
//!   `generateWAMessageFromContent`/`relayMessage`, `sock.sendPreview`,
//!   `createGoodbyeCard` (canvas), `saluranCtx()`, dan `goodbyeType` 2..7.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const OWNER_ONLY: &str = "❌ Hanya owner yang bisa menggunakan fitur ini!";

fn already_active(prefix: &str) -> String {
    format!(
        "⚠️ *ɢᴏᴏᴅʙʏᴇ ᴀʟʀᴇᴀᴅʏ ᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *✅ ON*\n\
         > Goodbye sudah aktif di grup ini.\n\n\
         _Gunakan `{prefix}goodbye off` untuk menonaktifkan._"
    )
}

fn already_inactive(prefix: &str) -> String {
    format!(
        "⚠️ *ɢᴏᴏᴅʙʏᴇ ᴀʟʀᴇᴀᴅʏ ɪɴᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *❌ OFF*\n\
         > Goodbye sudah nonaktif di grup ini.\n\n\
         _Gunakan `{prefix}goodbye on` untuk mengaktifkan._"
    )
}

fn turned_on(prefix: &str) -> String {
    format!(
        "✅ *ɢᴏᴏᴅʙʏᴇ ᴀᴋᴛɪꜰ*\n\n\
         > Goodbye message berhasil diaktifkan!\n\
         > Member yang keluar akan diberi pesan.\n\n\
         _Gunakan `{prefix}setgoodbye` untuk custom pesan._"
    )
}

fn turned_off() -> String {
    "❌ *ɢᴏᴏᴅʙʏᴇ ɴᴏɴᴀᴋᴛɪꜰ*\n\n\
     > Goodbye message berhasil dinonaktifkan.\n\
     > Member yang keluar tidak akan diberi pesan."
        .to_string()
}

fn settings_text(prefix: &str, on: bool) -> String {
    format!(
        "👋 *ɢᴏᴏᴅʙʏᴇ sᴇᴛᴛɪɴɢs*\n\n\
         > Status: *{status}*\n\n\
         ```━━━ ᴘɪʟɪʜᴀɴ ━━━```\n\
         > `{prefix}goodbye on` → Aktifkan\n\
         > `{prefix}goodbye off` → Nonaktifkan\n\
         > `{prefix}goodbye on all` → Global ON (owner)\n\
         > `{prefix}goodbye off all` → Global OFF (owner)\n\
         > `{prefix}setgoodbye` → Custom pesan\n\
         > `{prefix}resetgoodbye` → Reset default",
        status = if on { "✅ ON" } else { "❌ OFF" },
    )
}

/// Cabang `* all`: gerbang owner, lalu jalur `catch` milik JS sendiri karena
/// `groupFetchAllParticipating` tidak ada di `Ctx`.
fn global_unavailable(ctx: &Ctx) -> Reply {
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[plugin(
    name = "goodbye",
    alias = ["bye", "leave"],
    category = "group",
    description = "Mengatur goodbye message untuk grup",
    usage = ".goodbye <on/off>",
    example = ".goodbye on",
    cooldown = 5,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn goodbye(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let sub2 = ctx.args.get(1).map(|a| a.to_lowercase());
    // `groupData.goodbye === true` — perbandingan ketat, nilai `"true"` atau
    // `1` di store tidak dihitung aktif.
    let current = ctx.group_field("goodbye") == Some(&serde_json::Value::Bool(true));

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
                data: serde_json::json!({ "goodbye": true, "leave": true }),
            })
        }
        Some("off") => {
            if !current {
                return Reply::text(already_inactive(&ctx.prefix));
            }
            Reply::text(turned_off()).with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: serde_json::json!({ "goodbye": false, "leave": false }),
            })
        }
        _ => Reply::text(settings_text(&ctx.prefix, current)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use serde_json::{Value};

    const CHAT: &str = "120363@g.us";

    fn run(args: &[&str], group: Value, is_owner: bool) -> Reply {
        (find("goodbye").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "goodbye".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: CHAT.into(),
            is_group: true,
            is_owner,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            group_data: group,
            ..Default::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn written(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, CHAT);
                data.clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("goodbye").unwrap();
        assert_eq!(p.meta.name, "goodbye");
        assert_eq!(p.meta.alias, ["bye", "leave"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mengatur goodbye message untuk grup");
        assert_eq!(p.meta.usage, ".goodbye <on/off>");
        assert_eq!(p.meta.example, ".goodbye on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin, "isAdmin:true di JS");
        assert!(!p.meta.is_bot_admin, "tidak ada di JS");
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["bye", "leave"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("goodbye"), "{a}");
        }
    }

    #[test]
    fn on_writes_both_flags_and_says_nothing_more() {
        let r = run(&["on"], serde_json::json!({}), false);
        assert_eq!(written(&r), serde_json::json!({ "goodbye": true, "leave": true }));
        let t = text_of(&r);
        assert!(t.starts_with("✅ *ɢᴏᴏᴅʙʏᴇ ᴀᴋᴛɪꜰ*"), "{t}");
        assert!(t.contains("> Goodbye message berhasil diaktifkan!"), "{t}");
        assert!(t.ends_with("_Gunakan `.setgoodbye` untuk custom pesan._"), "{t}");
    }

    #[test]
    fn off_writes_both_flags_off() {
        let r = run(&["off"], serde_json::json!({ "goodbye": true }), false);
        assert_eq!(written(&r), serde_json::json!({ "goodbye": false, "leave": false }));
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴏᴏᴅʙʏᴇ ɴᴏɴᴀᴋᴛɪꜰ*\n\n> Goodbye message berhasil dinonaktifkan.\n> Member yang keluar tidak akan diberi pesan."
        );
    }

    #[test]
    fn turning_on_twice_warns_instead_of_writing_again() {
        let r = run(&["on"], serde_json::json!({ "goodbye": true }), false);
        let t = text_of(&r);
        assert!(t.starts_with("⚠️ *ɢᴏᴏᴅʙʏᴇ ᴀʟʀᴇᴀᴅʏ ᴀᴄᴛɪᴠᴇ*"), "{t}");
        assert!(t.contains("> Status: *✅ ON*"), "{t}");
        assert!(t.ends_with("_Gunakan `.goodbye off` untuk menonaktifkan._"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn turning_off_twice_warns_instead_of_writing_again() {
        let r = run(&["off"], serde_json::json!({}), false);
        let t = text_of(&r);
        assert!(t.starts_with("⚠️ *ɢᴏᴏᴅʙʏᴇ ᴀʟʀᴇᴀᴅʏ ɪɴᴀᴄᴛɪᴠᴇ*"), "{t}");
        assert!(t.contains("> Status: *❌ OFF*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_non_boolean_goodbye_flag_counts_as_off() {
        // `groupData.goodbye === true` — string `"true"` bukan `true`.
        for stored in [serde_json::json!("true"), serde_json::json!(1), serde_json::json!(null), serde_json::json!("yes")] {
            let r = run(&["on"], serde_json::json!({ "goodbye": stored }), false);
            assert_eq!(written(&r), serde_json::json!({ "goodbye": true, "leave": true }), "{stored}");
        }
    }

    #[test]
    fn without_an_action_it_only_prints_the_settings_board() {
        for group in [serde_json::json!({}), serde_json::json!({ "goodbye": true })] {
            let r = run(&[], group.clone(), false);
            let t = text_of(&r);
            let on = group["goodbye"] == serde_json::json!(true);
            assert!(
                t.starts_with("👋 *ɢᴏᴏᴅʙʏᴇ sᴇᴛᴛɪɴɢs*"),
                "{group}: {t}"
            );
            assert!(t.contains(&format!("> Status: *{}*", if on { "✅ ON" } else { "❌ OFF" })), "{t}");
            assert!(r.mutations().is_empty(), "{group} tidak boleh menulis");
        }
    }

    #[test]
    fn the_settings_board_lists_every_documented_action() {
        let t = text_of(&run(&[], serde_json::json!({}), false));
        assert!(t.contains("\n```━━━ ᴘɪʟɪʜᴀɴ ━━━```\n"), "{t}");
        for line in [
            "> `.goodbye on` → Aktifkan",
            "> `.goodbye off` → Nonaktifkan",
            "> `.goodbye on all` → Global ON (owner)",
            "> `.goodbye off all` → Global OFF (owner)",
            "> `.setgoodbye` → Custom pesan",
            "> `.resetgoodbye` → Reset default",
        ] {
            assert!(t.contains(line), "hilang: {line}\n{t}");
        }
    }

    #[test]
    fn the_global_branches_are_owner_only() {
        for args in [vec!["on", "all"], vec!["off", "all"]] {
            let r = run(&args, serde_json::json!({}), false);
            assert_eq!(text_of(&r), OWNER_ONLY, "{args:?}");
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn the_global_branches_never_claim_to_have_changed_any_group() {
        // `groupFetchAllParticipating` tidak ada di `Ctx`; jalur `catch` JS
        // yang dipakai, bukan pesan sukses "diaktifkan di N grup".
        for args in [vec!["on", "all"], vec!["off", "all"]] {
            let r = run(&args, serde_json::json!({}), true);
            assert!(text_of(&r).starts_with("☢ Kayaknya command `.goodbye`"), "{args:?}");
            assert!(r.mutations().is_empty(), "{args:?} tidak boleh menulis ke store");
            match r.without_mutations() {
                Reply::Reaction { emoji, .. } => assert_eq!(emoji, "☢", "{args:?}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn a_bare_on_never_takes_the_global_branch() {
        let r = run(&["on"], serde_json::json!({}), true);
        assert_eq!(written(&r), serde_json::json!({ "goodbye": true, "leave": true }));
        assert!(text_of(&r).starts_with("✅ *ɢᴏᴏᴅʙʏᴇ ᴀᴋᴛɪꜰ*"));
    }

    #[test]
    fn the_subcommands_are_lowercased_before_matching() {
        let r = run(&["ON"], serde_json::json!({}), false);
        assert_eq!(written(&r), serde_json::json!({ "goodbye": true, "leave": true }));
        let r = run(&["OFF", "ALL"], serde_json::json!({ "goodbye": true }), false);
        assert_eq!(text_of(&r), OWNER_ONLY);
    }
}