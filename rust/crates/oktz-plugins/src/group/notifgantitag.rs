//! `plugins/group/notifgantitag.js` — toggle `groups[chat].notifLabelChange`.
//!
//! Yang dipindah: cabang lokal `on`/`off` lengkap dengan tulis ke store
//! (`Mutation::SetGroup`) dan seluruh teks tampilannya.
//!
//! `ponytail:` cabang `on all`/`off all` butuh
//! `sock.groupFetchAllParticipating()` (`notifgantitag.js:34` dan `:57`) untuk
//! tahu daftar grup. `Ctx` tidak punya daftar itu, jadi kedua cabang berhenti
//! di balasan "tidak di-port" — penting: tanpa cabang ini `on all` akan jatuh
//! ke `on` lokal dan diam-diam hanya mengubah satu grup.
//!
//! Ekspor kedua file ini, `handleLabelChange` (`:120`), adalah hook event
//! `protocolMessage` type 30 yang mengirim lewat `sock.sendText` +
//! `saluranCtx()`; keduanya tidak ada di API plugin dan tidak ikut di-port.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

const OWNER_ONLY: &str = "❌ Hanya owner yang bisa menggunakan fitur ini!";

/// `notifgantitag.js:34` — daftar grup tidak bisa diambil, jadi tidak ada
/// `setGroup` yang dijalankan untuk branch `all`.
pub(crate) fn global_unavailable(action: &str) -> String {
    format!(
        "⚠️ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ɢʟᴏʙᴀʟ ᴛɪᴅᴀᴋ ᴅɪᴘᴏʀᴛ*\n\n\
         > `sock.groupFetchAllParticipating()` tidak ada di `Ctx`, jadi daftar\n\
         > grup tidak bisa diambil dan `setGroup` tidak pernah dijalankan.\n\
         > Aksi: `{action} all`"
    )
}

/// `notifgantitag.js:76-81`.
fn already_on(prefix: &str) -> String {
    format!(
        "⚠️ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ᴀʟʀᴇᴀᴅʏ ᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *✅ ON*\n\
         > Notifikasi ganti label sudah aktif di grup ini.\n\n\
         _Gunakan `{prefix}notifgantitag off` untuk menonaktifkan._"
    )
}

/// `notifgantitag.js:84-89`.
const NOW_ON: &str = "✅ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ᴀᴋᴛɪꜰ*\n\n\
> Notifikasi perubahan label member berhasil diaktifkan!\n\
> Bot akan memberitahu ketika ada member yang labelnya diganti.\n\n\
_Contoh: Admin menambahkan tag \"VIP\" ke member_";

/// `notifgantitag.js:93-98`.
fn already_off(prefix: &str) -> String {
    format!(
        "⚠️ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ᴀʟʀᴇᴀᴅʏ ɪɴᴀᴄᴛɪᴠᴇ*\n\n\
         > Status: *❌ OFF*\n\
         > Notifikasi ganti label sudah nonaktif di grup ini.\n\n\
         _Gunakan `{prefix}notifgantitag on` untuk mengaktifkan._"
    )
}

/// `notifgantitag.js:101-104`.
const NOW_OFF: &str = "❌ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ɴᴏɴᴀᴋᴛɪꜰ*\n\n\
> Notifikasi perubahan label member berhasil dinonaktifkan.";

/// `notifgantitag.js:107-117`. Yang berubah per status hanya baris `Status:`.
fn menu(prefix: &str, status: bool) -> String {
    format!(
        "🏷️ *ɴᴏᴛɪꜰ ɢᴀɴᴛɪ ᴛᴀɢ/ʟᴀʙᴇʟ*\n\n\
         > Status: *{}*\n\n\
         ```━━━ ᴘɪʟɪʜᴀɴ ━━━```\n\
         > `{prefix}notifgantitag on` → Aktifkan\n\
         > `{prefix}notifgantitag off` → Nonaktifkan\n\
         > `{prefix}notifgantitag on all` → Global ON (owner)\n\
         > `{prefix}notifgantitag off all` → Global OFF (owner)\n\n\
         > 📋 *Fitur ini akan memberitahu saat:*\n\
         > • Admin menambahkan label ke member\n\
         > • Admin menghapus label dari member\n\
         > • Label member berubah",
        if status { "✅ ON" } else { "❌ OFF" }
    )
}

#[plugin(
    name = "notifgantitag",
    alias = ["notiflabel", "notiftag", "labeltag"],
    category = "group",
    description = "Mengatur notifikasi perubahan label/tag member",
    usage = ".notifgantitag <on/off>",
    example = ".notifgantitag on",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = false
)]
pub fn notifgantitag(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let sub2 = ctx.args.get(1).map(|a| a.to_lowercase());

    if sub.as_deref() == Some("on") && sub2.as_deref() == Some("all") {
        if !ctx.is_owner {
            return Reply::text(OWNER_ONLY);
        }
        return Reply::text(global_unavailable("on"));
    }

    if sub.as_deref() == Some("off") && sub2.as_deref() == Some("all") {
        if !ctx.is_owner {
            return Reply::text(OWNER_ONLY);
        }
        return Reply::text(global_unavailable("off"));
    }

    // `groupData.notifLabelChange === true` — perbandingan ketat, bukan
    // kebenaran JS: angka 1 atau string "on" di store tidak dihitung aktif.
    let current = ctx.group_field("notifLabelChange") == Some(&json!(true));

    if sub.as_deref() == Some("on") {
        if current {
            return Reply::text(already_on(&ctx.prefix));
        }
        return Reply::text(NOW_ON).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: json!({ "notifLabelChange": true }),
        });
    }

    if sub.as_deref() == Some("off") {
        if !current {
            return Reply::text(already_off(&ctx.prefix));
        }
        return Reply::text(NOW_OFF).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: json!({ "notifLabelChange": false }),
        });
    }

    Reply::text(menu(&ctx.prefix, current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use serde_json::Value;

    fn run(group: Value, args: &[&str], is_owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notifgantitag".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            group_data: group,
            ..Default::default()
        };
        (find("notifgantitag").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn write(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data.clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("notifgantitag").unwrap();
        assert_eq!(e.meta.name, "notifgantitag");
        assert_eq!(e.meta.alias, ["notiflabel", "notiftag", "labeltag"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(
            e.meta.description,
            "Mengatur notifikasi perubahan label/tag member"
        );
        assert_eq!(e.meta.usage, ".notifgantitag <on/off>");
        assert_eq!(e.meta.example, ".notifgantitag on");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin);
        // Field yang tidak ada di JS berarti false, bukan true.
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
        assert!(!e.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["notiflabel", "notiftag", "labeltag"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("notifgantitag"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn turning_on_writes_the_flag_and_the_full_message() {
        let r = run(json!({}), &["on"], false);
        assert_eq!(body(&r), NOW_ON);
        assert_eq!(write(&r), json!({"notifLabelChange": true}));
    }

    #[test]
    fn turning_off_writes_the_flag_and_the_full_message() {
        let r = run(json!({"notifLabelChange": true}), &["off"], false);
        assert_eq!(body(&r), NOW_OFF);
        assert_eq!(write(&r), json!({"notifLabelChange": false}));
    }

    #[test]
    fn repeating_the_same_state_only_warns() {
        let on = run(json!({"notifLabelChange": true}), &["on"], false);
        assert!(body(&on).starts_with("⚠️ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ᴀʟʀᴇᴀᴅʏ ᴀᴄᴛɪᴠᴇ*"));
        assert!(on.mutations().is_empty());

        let off = run(json!({}), &["off"], false);
        assert!(body(&off).starts_with("⚠️ *ɴᴏᴛɪꜰ ʟᴀʙᴇʟ ᴀʟʀᴇᴀᴅʏ ɪɴᴀᴄᴛɪᴠᴇ*"));
        assert!(off.mutations().is_empty());
    }

    #[test]
    fn only_the_literal_true_counts_as_on() {
        // `=== true`, bukan kebenaran JS: 1 dan "on" tetap "nonaktif".
        for stored in [json!(1), json!("on"), json!("true"), json!({})] {
            let r = run(
                json!({ "notifLabelChange": stored.clone() }),
                &["on"],
                false,
            );
            assert_eq!(write(&r), json!({"notifLabelChange": true}), "{stored}");
        }
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert!(run(json!({}), &["ON"], false).mutations().len() == 1);
        assert!(
            run(json!({"notifLabelChange": true}), &["Off"], false)
                .mutations()
                .len()
                == 1
        );
        // Argumen lain jatuh ke menu, tanpa menulis.
        for other in ["nyala", "1", "on!"] {
            let r = run(json!({}), &[other], false);
            assert!(
                body(&r).starts_with("🏷️ *ɴᴏᴛɪꜰ ɢᴀɴᴛɪ ᴛᴀɢ/ʟᴀʙᴇʟ*"),
                "{other:?}"
            );
            assert!(r.mutations().is_empty(), "{other:?}");
        }
    }

    #[test]
    fn the_menu_reports_the_stored_status_and_uses_the_live_prefix() {
        let off = run(json!({}), &[], false);
        assert!(body(&off).contains("> Status: *❌ OFF*"), "{}", body(&off));
        let on = run(json!({"notifLabelChange": true}), &[], false);
        assert!(body(&on).contains("> Status: *✅ ON*"), "{}", body(&on));
        assert!(
            !body(&off).contains("` .notifgantitag"),
            "tidak boleh ada spasi di dalam backtick perintah"
        );
        assert!(body(&off).contains("→ Global ON (owner)"));
        assert!(
            body(&off).contains("```━━━ ᴘɪʟɪʜᴀɴ ━━━```"),
            "pagar tiga backtick"
        );
    }

    #[test]
    fn the_global_variants_never_fall_through_to_the_local_one() {
        for action in ["on", "off"] {
            for owner in [true, false] {
                let r = run(json!({}), &[action, "all"], owner);
                assert!(
                    r.mutations().is_empty(),
                    "{action} all/{owner} tidak boleh menulis store"
                );
                if owner {
                    assert_eq!(body(&r), global_unavailable(action), "{action}");
                } else {
                    assert_eq!(body(&r), OWNER_ONLY, "{action}");
                }
            }
        }
    }

    #[test]
    fn all_is_only_global_when_the_second_argument_is_exactly_all() {
        // `on anything` bukan `on all`, jadi harus jadi `on` lokal biasa.
        let r = run(json!({}), &["on", "semua"], false);
        assert_eq!(write(&r), json!({"notifLabelChange": true}));
        assert_eq!(body(&r), NOW_ON);
    }
}
