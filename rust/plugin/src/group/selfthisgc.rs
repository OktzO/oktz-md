//! `plugins/group/selfthisgc.js` — toggle `selfGroups`/`publicGroups`.
//!
//! Kedua plugin ini menulis DUA setting sekaligus dan saling membatalkan:
//! `selfthisgc` menambah chat ke `selfGroups` DAN membuangnya dari
//! `publicGroups`. Melewatkan salah satu akan meninggalkan grup dalam dua
//! mode sekaligus, dan router tidak punya aturan tie-break.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};


/// `db.setting("selfGroups") || []` dan `db.setting("publicGroups") || []`.
pub(crate) fn self_groups(ctx: &Ctx) -> Vec<String> {
    crate::common::string_list(ctx.setting("selfGroups"))
}

pub(crate) fn public_groups(ctx: &Ctx) -> Vec<String> {
    crate::common::string_list(ctx.setting("publicGroups"))
}

pub(crate) fn is_self_group(ctx: &Ctx) -> bool {
    self_groups(ctx)
        .iter()
        .any(|g| g == &ctx.chat)
}

pub(crate) fn already_self(prefix: &str) -> String {
    format!(
        "ℹ️ *ɢʀᴜᴘ ɪɴɪ sᴜᴅᴀʜ ᴍᴏᴅᴇ sᴇʟꜰ*\n\n\
         > Bot hanya merespon owner & bot sendiri\n\n\
         _Gunakan {prefix}publicthisgc untuk membuka akses_"
    )
}

pub(crate) fn applied(prefix: &str) -> String {
    format!(
        "🔒 *ᴍᴏᴅᴇ sᴇʟꜰ ᴀᴋᴛɪꜰ*\n\n\
         > Bot di grup ini sekarang hanya merespon:\n\
         > • Owner bot\n\
         > • Bot sendiri (fromMe)\n\n\
         📋 *Grup lain tidak terpengaruh*\n\n\
         _Gunakan {prefix}publicthisgc untuk membuka akses_"
    )
}

/// Daftar `selfGroups` dengan `chat` ditambahkan kalau belum ada.
pub(crate) fn self_groups_after(ctx: &Ctx) -> Vec<serde_json::Value> {
    let mut list: Vec<serde_json::Value> =
        self_groups(ctx).into_iter().map(serde_json::Value::String).collect();
    if !list.iter().any(|v| v.as_str() == Some(ctx.chat.as_str())) {
        list.push(serde_json::Value::String(ctx.chat.clone()));
    }
    list
}

/// Daftar `publicGroups` dengan `chat` dibuang.
pub(crate) fn public_groups_after(ctx: &Ctx) -> Vec<serde_json::Value> {
    public_groups(ctx)
        .into_iter()
        .filter(|g| g != &ctx.chat)
        .map(serde_json::Value::String)
        .collect()
}

#[plugin(
    name = "selfthisgc",
    alias = ["selfgc", "selfgroup", "selfthisgroup"],
    category = "group",
    description = "Aktifkan mode self hanya di grup ini",
    usage = ".selfthisgc",
    example = ".selfthisgc",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn selfthisgc(ctx: Ctx) -> Reply {
    if is_self_group(&ctx) {
        return Reply::text(already_self(&ctx.prefix));
    }

    Reply::text(applied(&ctx.prefix))
        .with_reaction("🔒")
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
            command: "selfthisgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("selfthisgc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply, key: &str) -> Value {
        assert_eq!(r.mutations().len(), 2, "dua setting harus ditulis");
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
        let p = oktz_plugin_api::find("selfthisgc").unwrap();
        assert_eq!(p.meta.name, "selfthisgc");
        assert_eq!(p.meta.alias, ["selfgc", "selfgroup", "selfthisgroup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Aktifkan mode self hanya di grup ini");
        assert_eq!(p.meta.usage, ".selfthisgc");
        assert_eq!(p.meta.example, ".selfthisgc");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_owner, "isOwner: true");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["selfgc", "selfgroup", "selfthisgroup"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "selfthisgc");
        }
    }

    #[test]
    fn a_group_already_in_self_mode_is_left_alone() {
        let r = run(serde_json::json!({"selfGroups": ["120363@g.us"]}));
        assert!(text_of(&r).starts_with("ℹ️ *ɢʀᴜᴘ ɪɴɪ sᴜᴅᴀʜ ᴍᴏᴅᴇ sᴇʟꜰ*"));
        assert!(r.mutations().is_empty(), "sudah self = tidak ada yang ditulis");
    }

    #[test]
    fn the_chat_is_added_to_self_groups_when_absent() {
        let r = run(serde_json::json!({"selfGroups": ["a@g.us"]}));
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["a@g.us", "120363@g.us"]);
        assert!(text_of(&r).starts_with("🔒 *ᴍᴏᴅᴇ sᴇʟꜰ ᴀᴋᴛɪꜰ*"));
    }

    #[test]
    fn the_chat_is_removed_from_public_groups_at_the_same_time() {
        let r = run(serde_json::json!({"publicGroups": ["a@g.us", "120363@g.us"]}));
        assert_eq!(
            list_of(&setting_of(&r, "publicGroups")),
            ["a@g.us"],
            "grup ini harus dikeluarkan dari daftar public"
        );
    }

    #[test]
    fn both_settings_are_always_written_together() {
        for s in [
            serde_json::json!({}),
            serde_json::json!({"selfGroups": []}),
            serde_json::json!({"publicGroups": ["120363@g.us"]}),
        ] {
            assert_eq!(run(s.clone()).mutations().len(), 2, "input {s}");
        }
    }

    #[test]
    fn an_absent_setting_becomes_an_empty_list_not_a_missing_key() {
        let r = run(serde_json::json!({}));
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["120363@g.us"]);
        assert!(list_of(&setting_of(&r, "publicGroups")).is_empty());
    }

    #[test]
    fn other_groups_in_both_lists_are_preserved() {
        let r = run(serde_json::json!({
            "selfGroups": ["x@g.us"],
            "publicGroups": ["y@g.us", "120363@g.us", "z@g.us"],
        }));
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["x@g.us", "120363@g.us"]);
        assert_eq!(
            list_of(&setting_of(&r, "publicGroups")),
            ["y@g.us", "z@g.us"],
            "urutan grup lain harus tetap"
        );
    }

    #[test]
    fn a_non_array_setting_reads_as_an_empty_list() {
        // `db.setting("selfGroups") || []` — string/object bukan array, dan
        // `.includes` di JS akan berperilaku aneh. Yang aman: perlakukan sebagai kosong.
        let r = run(serde_json::json!({"selfGroups": "bukan-array", "publicGroups": 5}));
        assert_eq!(list_of(&setting_of(&r, "selfGroups")), ["120363@g.us"]);
        assert!(list_of(&setting_of(&r, "publicGroups")).is_empty());
    }

    #[test]
    fn the_change_reacts_with_a_padlock() {
        match run(serde_json::json!({})).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🔒"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_reply_names_the_counterpart_command_with_the_live_prefix() {
        assert!(text_of(&run(serde_json::json!({}))).contains(".publicthisgc"));
        assert!(already_self("~").contains("~publicthisgc"));
    }

    #[test]
    fn self_and_public_group_toggles_keep_separate_keys() {
        for (name, keys) in [
            ("selfthisgc", &["selfthisgc", "selfgc", "selfgroup", "selfthisgroup"][..]),
            (
                "publicthisgc",
                &["publicthisgc", "publicgc", "publicgroup", "publicthisgroup"][..],
            ),
        ] {
            for k in keys {
                let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(k))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, [name], "key {k} diklaim oleh {owners:?}");
            }
        }
    }
}