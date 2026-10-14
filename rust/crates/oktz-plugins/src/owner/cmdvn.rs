//! `plugins/owner/cmdvn.js` — flag `cmdVn`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

/// `db.setting('cmdVn') || false`.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    crate::common::js_truthy(ctx.setting("cmdVn"))
}

pub(crate) const ON_MSG: &str =
    "✅ *ᴄᴍᴅ ᴠɴ ᴀᴋᴛɪꜰ*\n\n> Kirim voice note berisi nama command\n> Bot akan transkrip dan jalankan otomatis\n> Contoh: VN \"menu\" → trigger .menu";
pub(crate) const OFF_MSG: &str =
    "❌ CMD VN *dinonaktifkan*. Command via text normal.";

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "🎤 *ᴄᴍᴅ ᴠᴏɪᴄᴇ ɴᴏᴛᴇ*\n\n\
         > Status: *{}*\n\n\
         > `{p}cmdvn on` — Command via VN\n\
         > `{p}cmdvn off` — Command via text (default)\n\n\
         > Saat ON, kirim VN berisi nama command\n\
         > Contoh: VN \"menu\" → trigger .menu",
        if current_enabled(ctx) { "✅ ON" } else { "❌ OFF" },
        p = ctx.prefix
    )
}

pub(crate) fn invalid(prefix: &str) -> String {
    format!("❌ Gunakan `{prefix}cmdvn on` atau `{prefix}cmdvn off`")
}

#[plugin(
    name = "cmdvn",
    alias = ["voicecommand", "vncmd"],
    category = "owner",
    description = "Aktifkan command via voice note",
    usage = ".cmdvn <on/off>",
    example = ".cmdvn on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn cmdvn(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        // `subCmd === 'status'` ikut cabang status bersama ketiadaan argumen.
        None | Some("status") => Reply::text(status_text(&ctx)),
        Some("on") => Reply::text(ON_MSG).with_mutation(Mutation::SetSetting {
            key: "cmdVn".into(),
            value: serde_json::Value::Bool(true),
        }),
        Some("off") => Reply::text(OFF_MSG).with_mutation(Mutation::SetSetting {
            key: "cmdVn".into(),
            value: serde_json::Value::Bool(false),
        }),
        Some(_) => Reply::text(invalid(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cmdvn".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("cmdvn").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "cmdVn");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cmdvn").unwrap();
        assert_eq!(p.meta.name, "cmdvn");
        assert_eq!(p.meta.alias, ["voicecommand", "vncmd"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Aktifkan command via voice note");
        assert_eq!(p.meta.usage, ".cmdvn <on/off>");
        assert_eq!(p.meta.example, ".cmdvn on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["voicecommand", "vncmd"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "cmdvn");
        }
    }

    #[test]
    fn no_argument_and_the_word_status_share_the_status_branch() {
        for s in [json!({}), json!({"cmdVn": true}), json!({"cmdVn": false})] {
            let none = run(s.clone(), "");
            let status = run(s.clone(), "status");
            assert_eq!(text_of(&none), text_of(&status), "input {s}");
            assert!(text_of(&none).starts_with("🎤 *ᴄᴍᴅ ᴠᴏɪᴄᴇ ɴᴏᴛᴇ*"));
            assert!(none.mutations().is_empty(), "status tidak menulis");
            assert!(status.mutations().is_empty());
        }
    }

    #[test]
    fn the_status_line_reflects_the_stored_flag() {
        assert!(text_of(&run(json!({"cmdVn": true}), "")).contains("✅ ON"));
        assert!(text_of(&run(json!({"cmdVn": false}), "")).contains("❌ OFF"));
        assert!(
            text_of(&run(json!({}), "")).contains("❌ OFF"),
            "`|| false` berarti key yang hilang tampil OFF"
        );
    }

    #[test]
    fn on_and_off_write_their_own_boolean_and_react_to_nothing() {
        let on = run(json!({}), "on");
        assert_eq!(stored(&on), json!(true));
        assert_eq!(text_of(&on), ON_MSG);
        // Plugin ini TIDAK memanggil m.react — tidak boleh ada Reaction.
        assert!(matches!(on.without_mutations(), Reply::Text { .. }));

        let off = run(json!({}), "off");
        assert_eq!(stored(&off), json!(false));
        assert_eq!(text_of(&off), OFF_MSG);
    }

    #[test]
    fn an_unknown_sub_command_is_rejected_and_writes_nothing() {
        for bad in ["maybe", "1", "ON!", "enable"] {
            let r = run(json!({}), bad);
            assert!(text_of(&r).starts_with("❌ Gunakan"), "input {bad}");
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn the_invalid_reply_carries_the_prefix() {
        assert_eq!(invalid("~"), "❌ Gunakan `~cmdvn on` atau `~cmdvn off`");
    }

    #[test]
    fn the_status_reply_uses_the_live_prefix_except_where_the_js_hardcodes_it() {
        let t = status_text(&Ctx {
            prefix: "#".into(),
            settings: json!({}),
            ..Default::default()
        });
        assert!(t.contains("#cmdvn on"), "prefix substitusi: {t}");
        assert!(t.contains("#cmdvn off"));
        // `VN "menu" → trigger .menu` — literal di JS, bukan `{prefix}menu`.
        // Menempelkan prefix di sini akan mengubah teks yang ditampilkan.
        assert!(t.contains("VN \"menu\" → trigger .menu"), "{t}");
        assert!(!t.contains("trigger #menu"));
    }

    #[test]
    fn no_branch_depends_on_the_group_roster() {
        // Plugin ini hanya menyentuh settings; roster kosong, beranggota,
        // atau berisi admin/owner tidak boleh mengubah satu pun cabang.
        use oktz_plugin_api::registry::{GroupRoster, Participant};
        let p = |admin: bool, owner: bool| Participant {
            jid: "628111@s.whatsapp.net".into(),
            is_admin: admin,
            is_owner: owner,
            data: Default::default(),
        };
        let rosters = [
            GroupRoster::default(),
            GroupRoster {
                participants: vec![p(false, false)],
                is_bot_admin: false,
            },
            GroupRoster {
                participants: vec![p(true, false)],
                is_bot_admin: true,
            },
            GroupRoster {
                participants: vec![p(true, true)],
                is_bot_admin: true,
            },
        ];
        for roster in rosters {
            for arg in ["", "on", "off", "zzz"] {
                let base = run(json!({}), arg);
                let with_roster = {
                    let ctx = Ctx {
                        prefix: ".".into(),
                        command: "cmdvn".into(),
                        text: arg.into(),
                        args: arg.split_whitespace().map(str::to_string).collect(),
                        is_group: true,
                        roster: roster.clone(),
                        ..Default::default()
                    };
                    (oktz_plugin_api::find("cmdvn").unwrap().handler)(ctx)
                };
                assert_eq!(
                    text_of(&base),
                    text_of(&with_roster),
                    "arg {arg:?} berubah karena roster"
                );
                assert_eq!(base.mutations().len(), with_roster.mutations().len());
            }
        }
    }
}