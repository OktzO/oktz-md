//! `plugins/group/notifsholat.js` — `groups[chat].notifSholat` on/off.
//!
//! Status yang ditampilkan bukan field yang ditulis plugin ini:
//! `autoSholat` (grup → global → default) menentukan "Pengingat", sedangkan
//! `notifSholat` menentukan "Status Grup" dan itu yang disimpan di sini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::group::notifclosegroup::ADMIN_ONLY;

/// `ponytail:` `m.isAdmin` tidak ada di `Ctx`; guard memakainya
/// `ctx.roster.is_admin(sender)`, roster grup untuk pesan yang sama.
/// `group.autoSholat ?? setting('autoSholat') ?? false` — `??` hanya
/// null/undefined, jadi `false` milik grup tidak ditimpa global.
fn group_auto(ctx: &Ctx) -> bool {
    ctx.group_field("autoSholat")
        .filter(|v| !v.is_null())
        .map(crate::common::js_truthy_value)
        .unwrap_or_else(|| {
            ctx.setting("autoSholat")
                .map(crate::common::js_truthy_value)
                .unwrap_or(false)
        })
}

/// `group.autoSholatKota || setting('autoSholatKota') || { nama: 'KOTA JAKARTA' }`
/// — `||` bukan `??`, jadi nilai falsy apa pun jatuh ke global.
fn kota(ctx: &Ctx) -> Value {
    ctx.group_field("autoSholatKota")
        .filter(|v| crate::common::js_truthy_value(v))
        .or_else(|| ctx.setting("autoSholatKota").filter(|v| crate::common::js_truthy_value(v)))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "nama": "KOTA JAKARTA" }))
}

fn kota_nama(ctx: &Ctx) -> String {
    kota(ctx)
        .get("nama")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "undefined".to_string())
}

fn overview(ctx: Ctx) -> Reply {
    let status_global = if group_auto(&ctx) { "✅ AKTIF" } else { "❌ NONAKTIF" };
    let status_grup = if ctx.group_data.get("notifSholat").and_then(Value::as_bool) == Some(false) {
        "❌ NONAKTIF"
    } else {
        "✅ AKTIF"
    };

    Reply::text(format!(
        "🕌 *PENGINGAT WAKTU SHOLAT*\n\n\
         Status Pengingat (grup ini): *{status_global}*\n\
         Status Grup: *{status_grup}*\n\
         Lokasi: *{}*\n\n\
         *PENGATURAN GRUP:*\n\
         • *{}notifsholat on* — Aktifkan notif di grup ini\n\
         • *{}notifsholat off* — Nonaktifkan notif di grup ini\n\n\
         *CARA KERJA:*\n\
         1. Mengirimkan mp3 adzan & gambar jadwal saat masuk waktu sholat\n\
         2. Mengikuti jadwal real-time dari myquran.com\n\
         3. Owner dapat mengatur ulang dengan perintah *{}autosholat* di grup ini.\n\
         4. Jika grup merasa terganggu, admin dapat mematikan khusus untuk grup ini.",
        kota_nama(&ctx),
        ctx.prefix,
        ctx.prefix,
        ctx.prefix,
    ))
}

#[plugin(
    name = "notifsholat",
    alias = ["notifsolat"],
    category = "group",
    description = "Toggle notifikasi sholat untuk grup ini",
    usage = ".notifsholat on/off",
    example = ".notifsholat on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn notifsholat(ctx: Ctx) -> Reply {
    if !ctx.roster.is_admin(&ctx.sender) && !ctx.is_owner {
        return Reply::text(ADMIN_ONLY);
    }

    let arg = ctx.args.first().map(|a| a.to_lowercase());
    let enable = match arg.as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => return overview(ctx),
    };

    if enable {
        return Reply::text(format!(
            "✅ *ɴᴏᴛɪꜰ sʜᴏʟᴀᴛ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*\n\n\
             > Grup ini akan menerima pengingat waktu sholat\n\
             > Lokasi: {}",
            kota_nama(&ctx)
        ))
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "notifSholat": true }),
        });
    }

    Reply::text("❌ *ɴᴏᴛɪꜰ sʜᴏʟᴀᴛ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*").with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "notifSholat": false }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::Participant;
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::json;

    fn run(group: Value, settings: Value, arg: Option<&str>, admin: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notifsholat".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: arg.into_iter().map(String::from).collect(),
            group_data: group,
            settings,
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: "628111@s.whatsapp.net".into(),
                    is_admin: admin,
                    is_owner: false,
                    data: Value::Null,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("notifsholat").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notifsholat").unwrap();
        assert_eq!(p.meta.name, "notifsholat");
        assert_eq!(p.meta.alias, ["notifsolat"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Toggle notifikasi sholat untuk grup ini");
        assert_eq!(p.meta.usage, ".notifsholat on/off");
        assert_eq!(p.meta.example, ".notifsholat on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin, "tidak ada di pluginConfig JS");
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("notifsolat").unwrap().meta.name, "notifsholat");
    }

    #[test]
    fn a_member_who_is_neither_admin_nor_owner_is_rejected() {
        let r = run(json!({}), json!({}), Some("on"), false);
        assert_eq!(text_of(&r), ADMIN_ONLY);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn without_an_argument_the_plugin_shows_both_statuses() {
        let r = run(json!({}), json!({}), None, true);
        let t = text_of(&r);
        assert!(t.starts_with("🕌 *PENGINGAT WAKTU SHOLAT*"), "{t}");
        assert!(t.contains("Status Pengingat (grup ini): *❌ NONAKTIF*"), "{t}");
        assert!(t.contains("Status Grup: *✅ AKTIF*"), "{t}");
        assert!(t.contains("Lokasi: *KOTA JAKARTA*"), "default kota: {t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_group_auto_setting_is_used_before_the_global_one() {
        let r = run(
            json!({"autoSholat": true}),
            json!({"autoSholat": false, "autoSholatKota": {"nama": "BANDUNG"}}),
            None,
            true,
        );
        let t = text_of(&r);
        assert!(t.contains("Status Pengingat (grup ini): *✅ AKTIF*"), "{t}");
        assert!(t.contains("Lokasi: *BANDUNG*"), "kota grup menang: {t}");
    }

    #[test]
    fn an_explicit_false_in_the_group_is_not_overwritten_by_the_global_value() {
        // `??` di JS: `false` bukan nullish, jadi tetap nonaktif walau global aktif.
        let r = run(json!({"autoSholat": false}), json!({"autoSholat": true}), None, true);
        let t = text_of(&r);
        assert!(t.contains("Status Pengingat (grup ini): *❌ NONAKTIF*"), "{t}");
    }

    #[test]
    fn notif_sholat_is_only_nonaktif_when_explicitly_false() {
        for (stored, expected) in [
            (json!({}), "✅ AKTIF"),
            (json!({"notifSholat": null}), "✅ AKTIF"),
            (json!({"notifSholat": true}), "✅ AKTIF"),
            (json!({"notifSholat": false}), "❌ NONAKTIF"),
        ] {
            let r = run(stored.clone(), json!({}), None, true);
            let t = text_of(&r);
            assert!(t.contains(&format!("Status Grup: *{expected}*")), "input {stored}: {t}");
        }
    }

    #[test]
    fn on_writes_true_and_names_the_location() {
        let r = run(
            json!({}),
            json!({"autoSholatKota": {"nama": "SURABAYA"}}),
            Some("on"),
            true,
        );
        assert_eq!(
            text_of(&r),
            "✅ *ɴᴏᴛɪꜰ sʜᴏʟᴀᴛ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*\n\n\
             > Grup ini akan menerima pengingat waktu sholat\n\
             > Lokasi: SURABAYA"
        );
        assert_eq!(group_write(&r), &json!({"notifSholat": true}));
    }

    #[test]
    fn off_writes_false_and_says_nothing_about_the_location() {
        let r = run(json!({}), json!({}), Some("off"), true);
        assert_eq!(text_of(&r), "❌ *ɴᴏᴛɪꜰ sʜᴏʟᴀᴛ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*");
        assert_eq!(group_write(&r), &json!({"notifSholat": false}));
    }

    #[test]
    fn an_unknown_keyword_shows_the_overview_instead_of_writing() {
        let r = run(json!({}), json!({}), Some("nyanyi"), true);
        assert!(text_of(&r).starts_with("🕌 *PENGINGAT WAKTU SHOLAT*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_overview_help_uses_the_live_prefix() {
        let r = run(json!({}), json!({}), None, true);
        let t = text_of(&r);
        assert!(t.contains("• *.notifsholat on* — Aktifkan notif di grup ini"), "{t}");
        assert!(t.contains("• *.notifsholat off* — Nonaktifkan notif di grup ini"), "{t}");
        assert!(t.contains("perintah *.autosholat* di grup ini"), "{t}");
    }
}