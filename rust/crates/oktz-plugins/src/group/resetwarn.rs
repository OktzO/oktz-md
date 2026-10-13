//! `plugins/group/resetwarn.js` — hapus satu kunci dari `groups[chat].warnings`.
//!
//! Alias `resetwarn` menulis ulang SELURUH peta `warnings` (JS
//! `db.setGroup(m.chat, { ...groupData, warnings })`), jadi mutasinya bukan
//! satu field — memotong key-nya dari salinan peta yang sudah dibaca.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::jid_number;

/// `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*` — dua contoh perintah pakai prefix yang sedang hidup.
pub(crate) fn no_target(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> Reply pesan user + `{prefix}resetwarn`\n\
         > Atau: `{prefix}resetwarn @user`"
    )
}

/// `groupData.maxWarnings || 3` — `0` falsy, jadi balik ke 3.
pub(crate) fn max_warns(ctx: &Ctx) -> i64 {
    match ctx.group_field("maxWarnings").and_then(Value::as_i64) {
        Some(0) | None => 3,
        Some(n) => n,
    }
}

/// `warnings[jid] || []` — array apa adanya kalau ada dan memang array.
fn warns_of<'a>(ctx: &'a Ctx, jid: &str) -> &'a [Value] {
    ctx.group_field("warnings")
        .and_then(|w| w.get(jid))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `m.quoted` dulu, baru `m.mentionedJid[0]` — `None` kalau tidak ada
/// keduanya.
fn target_of(ctx: &Ctx) -> Option<String> {
    ctx.quoted()
        .map(|q| q.sender.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| ctx.mentioned.first().cloned())
}

#[plugin(
    name = "resetwarn",
    alias = ["clearwarn", "hapuswarn", "delwarn"],
    category = "group",
    description = "Reset warning member",
    usage = ".resetwarn @user",
    example = ".resetwarn @user",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn resetwarn(ctx: Ctx) -> Reply {
    let Some(target) = target_of(&ctx) else {
        return Reply::text(no_target(&ctx.prefix));
    };

    let name = jid_number(&target);
    let warns = warns_of(&ctx, &target);
    if warns.is_empty() {
        return Reply::Text {
            text: format!("✅ @{name} tidak memiliki warning."),
            mentions: vec![target],
        };
    }

    // `delete warnings[targetUser]` — key hilang, jadi peta-nya ditulis ulang
    // utuh: mutasi merge, jadi key yang dibuang harus hilang dari payload.
    let mut next = ctx
        .group_field("warnings")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    if let Some(map) = next.as_object_mut() {
        map.remove(&target);
    }

    let max = max_warns(&ctx);
    Reply::Text {
        text: format!(
            "✅ *ᴡᴀʀɴɪɴɢ ᴅɪʀᴇsᴇᴛ*\nWarning @{name} berhasil direset!\n\
             Sebelumnya: *{}/{max}*\nSekarang: *0/{max}*",
            warns.len()
        ),
        mentions: vec![target],
    }
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "warnings": next }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::QuotedMessage;
    use serde_json::json;

    fn run(group: Value, mentioned: &[&str], quoted: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetwarn".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            mentioned: mentioned.iter().map(|s| s.to_string()).collect(),
            quoted: quoted.map(|sender| QuotedMessage {
                id: "MSG1".into(),
                sender: sender.into(),
                chat: "120363@g.us".into(),
                text: "x".into(),
                kind: oktz_plugin_api::QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("resetwarn").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
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
        let p = oktz_plugin_api::find("resetwarn").unwrap();
        assert_eq!(p.meta.name, "resetwarn");
        assert_eq!(p.meta.alias, ["clearwarn", "hapuswarn", "delwarn"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Reset warning member");
        assert_eq!(p.meta.usage, ".resetwarn @user");
        assert_eq!(p.meta.example, ".resetwarn @user");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["clearwarn", "hapuswarn", "delwarn"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "resetwarn");
        }
    }

    #[test]
    fn without_a_mention_or_a_quote_the_plugin_asks_for_one() {
        let r = run(json!({}), &[], None);
        assert_eq!(
            text_of(&r),
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> Reply pesan user + `.resetwarn`\n> Atau: `.resetwarn @user`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_target_without_warnings_says_so_and_writes_nothing() {
        for g in [
            json!(null),
            json!({}),
            json!({"warnings": {}}),
            json!({"warnings": {"628333@s.whatsapp.net": []}}),
        ] {
            let r = run(g.clone(), &["628333@s.whatsapp.net"], None);
            assert_eq!(
                text_of(&r),
                "✅ @628333 tidak memiliki warning.",
                "input {g}"
            );
            assert_eq!(mentions_of(&r), &["628333@s.whatsapp.net".to_string()]);
            assert!(r.mutations().is_empty(), "tidak boleh menulis: {g}");
        }
    }

    #[test]
    fn the_reset_writes_the_map_without_the_target_key() {
        let r = run(
            json!({"warnings": {
                "628222@s.whatsapp.net": [{"reason": "a", "time": 0}],
                "628333@s.whatsapp.net": [{"reason": "b", "time": 0}, {"reason": "c", "time": 0}]
            }}),
            &["628333@s.whatsapp.net"],
            None,
        );
        let written = &group_write(&r)["warnings"];
        assert!(written.get("628333@s.whatsapp.net").is_none(), "{written}");
        assert_eq!(written.as_object().unwrap().len(), 1, "sisa member lain");
        assert_eq!(
            text_of(&r),
            "✅ *ᴡᴀʀɴɪɴɢ ᴅɪʀᴇsᴇᴛ*\nWarning @628333 berhasil direset!\n\
             Sebelumnya: *2/3*\nSekarang: *0/3*"
        );
        assert_eq!(mentions_of(&r), &["628333@s.whatsapp.net".to_string()]);
    }

    #[test]
    fn a_custom_max_warnings_appears_in_both_counters() {
        let r = run(
            json!({"maxWarnings": 7, "warnings": {"628333@s.whatsapp.net": [{"reason": "a"}]}}),
            &["628333@s.whatsapp.net"],
            None,
        );
        assert!(text_of(&r).contains("Sebelumnya: *1/7*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("Sekarang: *0/7*"), "{}", text_of(&r));
    }

    #[test]
    fn max_warnings_zero_falls_back_to_three() {
        let r = run(
            json!({"maxWarnings": 0, "warnings": {"628333@s.whatsapp.net": [{"reason": "a"}]}}),
            &["628333@s.whatsapp.net"],
            None,
        );
        assert!(text_of(&r).contains("Sebelumnya: *1/3*"), "0 falsy -> default 3");
    }

    #[test]
    fn the_quote_wins_over_the_mention() {
        let r = run(
            json!({"warnings": {
                "628222@s.whatsapp.net": [{"reason": "a"}],
                "628333@s.whatsapp.net": [{"reason": "b"}]
            }}),
            &["628222@s.whatsapp.net"],
            Some("628333@s.whatsapp.net"),
        );
        assert!(text_of(&r).contains("@628333"), "{}", text_of(&r));
        assert!(!text_of(&r).contains("@628222"), "{}", text_of(&r));
    }

    #[test]
    fn a_device_suffix_in_the_jid_is_kept_in_the_mention_but_not_the_display() {
        let r = run(
            json!({"warnings": {"628333:7@s.whatsapp.net": [{"reason": "a"}]}}),
            &["628333:7@s.whatsapp.net"],
            None,
        );
        assert!(text_of(&r).contains("Warning @628333:7"), "{}", text_of(&r));
        assert_eq!(mentions_of(&r), &["628333:7@s.whatsapp.net".to_string()]);
    }
}