//! `plugins/group/publicthisgc.js` — toggle `publicGroups`/`selfGroups`.
//!
//! Simetris dengan [`crate::group::selfthisgc`]. Perbedaannya pada cabangnya:
//! mode public hanya ditolak kalau grup sudah public DAN tidak sekaligus
//! self — kalau sudah public tapi masih self, plugin diam-diam mengeluarkan
//! keluar dari `selfGroups` supaya mode public benar-benar berlaku.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::group::selfthisgc::{public_groups, self_groups};

pub(crate) fn is_public_group(ctx: &Ctx) -> bool {
    public_groups(ctx).iter().any(|g| g == &ctx.chat)
}

pub(crate) fn already_public(prefix: &str) -> String {
    format!(
        "ℹ️ *ɢʀᴜᴘ ɪɴɪ sᴜᴅᴀʜ ᴍᴏᴅᴇ ᴘᴜʙʟɪᴄ*\n\n\
         > Bot merespon semua member di grup ini\n\n\
         _Gunakan {prefix}selfthisgc untuk menutup akses_"
    )
}

pub(crate) fn applied(prefix: &str) -> String {
    format!(
        "🌐 *ᴍᴏᴅᴇ ᴘᴜʙʟɪᴄ ᴅɪᴀᴋᴛɪꜰᴋᴀsɪ*\n\n\
         > Bot sekarang merespon semua member di grup ini\n\
         > Override mode global aktif untuk grup ini\n\n\
         📋 *Grup lain tidak terpengaruh*\n\n\
         _Gunakan {prefix}selfthisgc untuk menutup akses lagi_"
    )
}

/// Daftar `publicGroups` dengan `chat` ditambahkan kalau belum ada.
pub(crate) fn public_groups_after(ctx: &Ctx) -> Vec<serde_json::Value> {
    let mut list: Vec<serde_json::Value> =
        public_groups(ctx).into_iter().map(serde_json::Value::String).collect();
    if !list.iter().any(|v| v.as_str() == Some(ctx.chat.as_str())) {
        list.push(serde_json::Value::String(ctx.chat.clone()));
    }
    list
}

/// Daftar `selfGroups` dengan `chat` dibuang — SELALU, bahkan kalau grup ini
/// tidak ada di sana (`filter` di JS berjalan pada array kosong dengan hasil
/// kosong, dan hasilnya tetap ditulis).
pub(crate) fn self_groups_after(ctx: &Ctx) -> Vec<serde_json::Value> {
    self_groups(ctx)
        .into_iter()
        .filter(|g| g != &ctx.chat)
        .map(serde_json::Value::String)
        .collect()
}

#[plugin(
    name = "publicthisgc",
    alias = ["publicgc", "publicgroup", "publicthisgroup"],
    category = "group",
    description = "Aktifkan mode public hanya di grup ini",
    usage = ".publicthisgc",
    example = ".publicthisgc",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn publicthisgc(ctx: Ctx) -> Reply {
    let is_self = self_groups(&ctx).iter().any(|g| g == &ctx.chat);
    // `isPublicGroup && !isSelfGroup` — dua-duanya harus benar.
    if is_public_group(&ctx) && !is_self {
        return Reply::text(already_public(&ctx.prefix));
    }

    Reply::text(applied(&ctx.prefix))
        .with_reaction("🌐")
        .with_mutation(Mutation::SetSetting {
            key: "selfGroups".into(),
            value: serde_json::Value::Array(self_groups_after(&ctx)),
        })
        .with_mutation(Mutation::SetSetting {
            key: "publicGroups".into(),
            value: serde_json::Value::Array(public_groups_after(&ctx)),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "publicthisgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("publicthisgc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply, key: &str) -> Value {
        assert_eq!(r.mutations().len(), 2);
        r.mutations()
            .iter()
            .find_map(|m| match m {
                Mutation::SetSetting { key: k, value } if k == key => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("key {key} tidak ditulis"))
    }

    fn list_of(v: &Value) -> Vec<String> {
        v.as_array()
            .expect("harus array")
            .iter()
            .map(|x| x.as_str().expect("harus string").to_string())
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("publicthisgc").unwrap();
        assert_eq!(p.meta.name, "publicthisgc");
        assert_eq!(
            p.meta.alias,
            ["publicgc", "publicgroup", "publicthisgroup"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Aktifkan mode public hanya di grup ini"
        );
        assert_eq!(p.meta.usage, ".publicthisgc");
        assert_eq!(p.meta.example, ".publicthisgc");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["publicgc", "publicgroup", "publicthisgroup"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "publicthisgc"
            );
        }
    }

    #[test]
    fn an_already_public_and_not_self_group_is_left_alone() {
        let r = run(serde_json::json!({"publicGroups": ["120363@g.us"]}));
        assert!(text_of(&r).starts_with("ℹ️ *ɢʀᴜᴘ ɪɴɪ sᴜᴅᴀʜ ᴍᴏᴅᴇ ᴘᴜʙʟɪᴄ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_group_that_is_both_public_and_self_is_still_switched() {
        // `isPublicGroup && !isSelfGroup` — kalau dua-duanya ada, plugin
        // lanjut supaya selfGroups dibersihkan.
        let r = run(serde_json::json!({
            "publicGroups": ["120363@g.us"],
            "selfGroups": ["120363@g.us", "a@g.us"],
        }));
        assert!(!text_of(&r).starts_with("ℹ️"), "harus lanjut, bukan berhenti");
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["a@g.us"]);
        assert_eq!(list_of(&setting_of(&r, "publicGroups")), ["120363@g.us"]);
    }

    #[test]
    fn the_chat_is_added_to_public_groups_when_absent() {
        let r = run(serde_json::json!({"publicGroups": ["x@g.us"]}));
        assert_eq!(
            list_of(&setting_of(&r, "publicGroups")),
            ["x@g.us", "120363@g.us"]
        );
    }

    #[test]
    fn the_chat_is_removed_from_self_groups_at_the_same_time() {
        let r = run(serde_json::json!({"selfGroups": ["120363@g.us", "a@g.us"]}));
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["a@g.us"]);
    }

    #[test]
    fn both_settings_are_always_written_together() {
        for s in [serde_json::json!({}), serde_json::json!({"publicGroups": []}), serde_json::json!({"selfGroups": []})] {
            assert_eq!(run(s.clone()).mutations().len(), 2, "input {s}");
        }
    }

    #[test]
    fn an_absent_setting_becomes_an_empty_list() {
        let r = run(serde_json::json!({}));
        assert_eq!(list_of(&setting_of(&r, "publicGroups")), ["120363@g.us"]);
        assert!(list_of(&setting_of(&r, "selfGroups")).is_empty());
    }

    #[test]
    fn a_non_array_setting_reads_as_an_empty_list() {
        let r = run(serde_json::json!({"publicGroups": "x", "selfGroups": 7}));
        assert_eq!(list_of(&setting_of(&r, "publicGroups")), ["120363@g.us"]);
        assert!(list_of(&setting_of(&r, "selfGroups")).is_empty());
    }

    #[test]
    fn the_change_reacts_with_a_globe() {
        match run(serde_json::json!({})).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🌐"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_reply_names_the_counterpart_command() {
        let r = run(serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.contains("selfthisgc"), "{t}");
        assert!(already_public("~").contains("~selfthisgc"));
    }
}