//! `plugins/group/listwarn.js` — `groups[chat].warnings` (peta jid → array).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{date_dmy, jid_number, non_empty_map_entries};

const EMPTY_TARGET: &str = "✅ Tidak ada member dengan warning di grup ini.";
const NONE: &str = "tidak memiliki warning.";

/// `groupData.maxWarnings || 3` — `0` falsy jadi kembali ke 3.
pub(crate) fn max_warnings(ctx: &Ctx) -> i64 {
    match ctx.group_field("maxWarnings").and_then(Value::as_i64) {
        Some(0) | None => 3,
        Some(n) => n,
    }
}

/// `{ reason, time }` untuk satu entri warning, dengan tanggal Jakarta.
pub(crate) fn warning_line(v: &Value, i: usize) -> String {
    let reason = v.get("reason").and_then(Value::as_str).unwrap_or("undefined");
    // `timeHelper.fromTimestamp(w.time, 'DD/MM/YYYY')` — `undefined` jadi
    // `new Date(undefined)` = Invalid Date, `formatPattern` apa adanya, tapi
    // tidak pernah jadi crash di JS. Di sini timestamp rusak ditampilkan
    // sebagai epoch supaya barisnya tetap utuh.
    let ts = v.get("time").and_then(Value::as_i64).unwrap_or(0);
    format!("*{}.* {reason}\n   └ _{}_\n", i + 1, date_dmy(ts))
}

/// `warnings[jid] || []` — array apa adanya kalau ada dan memang array.
pub(crate) fn warnings_of<'a>(ctx: &'a Ctx, jid: &str) -> &'a [Value] {
    ctx.group_field("warnings")
        .and_then(|w| w.get(jid))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

#[plugin(
    name = "listwarn",
    alias = ["warnings", "cekwarn", "warnlist"],
    category = "group",
    description = "Melihat daftar warning member",
    usage = ".listwarn atau .listwarn @user",
    example = ".listwarn @user",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn listwarn(ctx: Ctx) -> Reply {
    let max = max_warnings(&ctx);

    // `ponytail:` JS memilih `m.quoted` sebelum `m.mentionedJid[0]`;
    // `Ctx` tidak membawa pesan yang di-reply, jadi reply-mode tidak
    // terjangkau. Tambahkan `quoted_sender: Option<String>` ke `Ctx` kalau
    // router menyediakannya.
    if let Some(target) = ctx.mentioned.first() {
        let user_warnings = warnings_of(&ctx, target);
        let num = jid_number(target);
        if user_warnings.is_empty() {
            return Reply::Text {
                text: format!("✅ @{num} {NONE}"),
                mentions: vec![target.clone()],
            };
        }
        let mut txt = format!("⚠️ *ᴡᴀʀɴɪɴɢ @{num}*\n\n");
        txt.push_str(&format!("> Total: *{}/{}*\n\n", user_warnings.len(), max));
        for (i, w) in user_warnings.iter().enumerate() {
            txt.push_str(&warning_line(w, i));
        }
        return Reply::Text {
            text: txt,
            mentions: vec![target.clone()],
        };
    }

    let entries = non_empty_map_entries(ctx.group_field("warnings"));
    if entries.is_empty() {
        return Reply::text(EMPTY_TARGET);
    }

    let mut txt = String::from("⚠️ *ᴅᴀꜰᴛᴀʀ ᴡᴀʀɴɪɴɢ*\n\n");
    let mut mentions = Vec::with_capacity(entries.len());
    for (i, (jid, data)) in entries.iter().enumerate() {
        let count = data.as_array().map_or(0, Vec::len);
        txt.push_str(&format!(
            "*{}.* @{} - *{count}/{max}* warning\n",
            i + 1,
            jid_number(jid)
        ));
        mentions.push(jid.clone());
    }
    txt.push_str(&format!(
        "\n> Ketik `{}listwarn @user` untuk detail",
        ctx.prefix
    ));
    Reply::Text {
        text: txt,
        mentions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(group: serde_json::Value, mentioned: Vec<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "listwarn".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            mentioned: mentioned.into_iter().map(String::from).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("listwarn").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        match r {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listwarn").unwrap();
        assert_eq!(p.meta.name, "listwarn");
        assert_eq!(p.meta.alias, ["warnings", "cekwarn", "warnlist"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Melihat daftar warning member");
        assert_eq!(p.meta.usage, ".listwarn atau .listwarn @user");
        assert_eq!(p.meta.example, ".listwarn @user");
        assert_eq!(p.meta.cooldown, 5);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["warnings", "cekwarn", "warnlist"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listwarn");
        }
    }

    // -- cabang "semua user" ----------------------------------------------

    #[test]
    fn absent_group_data_takes_the_empty_branch() {
        let r = run(json!(null), vec![]);
        assert_eq!(text_of(&r), EMPTY_TARGET);
        assert!(mentions_of(&r).is_empty());
    }

    #[test]
    fn empty_warning_arrays_are_filtered_out_of_the_list() {
        let r = run(
            json!({"warnings": {"628111@s.whatsapp.net": [], "628222@s.whatsapp.net": []}}),
            vec![],
        );
        assert_eq!(text_of(&r), EMPTY_TARGET);
    }

    #[test]
    fn listing_all_numbers_from_one_and_mentions_every_flagged_jid() {
        let r = run(
            json!({"warnings": {
                "628111@s.whatsapp.net": [{"reason": "spam", "time": 0}, {"reason": "nagar", "time": 0}],
                "628222@s.whatsapp.net": [{"reason": "toxic", "time": 0}]
            }}),
            vec![],
        );
        assert_eq!(
            text_of(&r),
            concat!(
                "⚠️ *ᴅᴀꜰᴛᴀʀ ᴡᴀʀɴɪɴɢ*\n\n",
                "*1.* @628111 - *2/3* warning\n",
                "*2.* @628222 - *1/3* warning\n",
                "\n> Ketik `.listwarn @user` untuk detail"
            )
        );
        assert_eq!(
            mentions_of(&r),
            &["628111@s.whatsapp.net", "628222@s.whatsapp.net"]
        );
    }

    #[test]
    fn max_warnings_zero_falls_back_to_three() {
        let r = run(
            json!({"maxWarnings": 0, "warnings": {"628111@s.whatsapp.net": [{"reason": "x", "time": 0}]}}),
            vec![],
        );
        assert!(text_of(&r).contains("*1/3* warning"), "0 falsy -> default 3");
    }

    #[test]
    fn a_custom_max_warnings_is_used_in_both_branches() {
        let group = json!({
            "maxWarnings": 5,
            "warnings": {"628111@s.whatsapp.net": [{"reason": "x", "time": 0}]}
        });
        assert!(text_of(&run(group.clone(), vec![])).contains("*1/5* warning"));
        assert!(text_of(&run(group.clone(), vec!["628111@s.whatsapp.net"])).contains("> Total: *1/5*"));
    }

    // -- cabang "@user" ----------------------------------------------------

    #[test]
    fn a_mention_for_someone_without_warnings_says_so() {
        let r = run(json!({"warnings": {"628222@s.whatsapp.net": [{"reason": "x", "time": 0}]}}), vec!["628333@s.whatsapp.net"]);
        assert_eq!(text_of(&r), "✅ @628333 tidak memiliki warning.");
        assert_eq!(mentions_of(&r), &["628333@s.whatsapp.net"]);
    }

    #[test]
    fn a_mention_with_no_warnings_map_at_all_still_says_so() {
        let r = run(json!(null), vec!["628333@s.whatsapp.net"]);
        assert_eq!(text_of(&r), "✅ @628333 tidak memiliki warning.");
    }

    #[test]
    fn a_mention_with_warnings_lists_them_with_reasons_and_dates() {
        let r = run(
            json!({"warnings": {"628333@s.whatsapp.net": [
                {"reason": "kasar", "time": 1_767_225_600_000i64},
                {"reason": "spam", "time": 1_767_225_600_000i64}
            ]}}),
            vec!["628333@s.whatsapp.net"],
        );
        assert_eq!(
            text_of(&r),
            format!(
                "⚠️ *ᴡᴀʀɴɪɴɢ @628333*\n\n> Total: *2/3*\n\n*1.* kasar\n   └ _{}_\n*2.* spam\n   └ _{}_\n",
                date_dmy(1_767_225_600_000),
                date_dmy(1_767_225_600_000)
            )
        );
        assert_eq!(mentions_of(&r), &["628333@s.whatsapp.net"]);
    }

    #[test]
    fn mention_wins_over_the_all_users_list() {
        let r = run(
            json!({"warnings": {
                "628222@s.whatsapp.net": [{"reason": "a", "time": 0}],
                "628333@s.whatsapp.net": [{"reason": "b", "time": 0}]
            }}),
            vec!["628333@s.whatsapp.net"],
        );
        assert!(text_of(&r).starts_with("⚠️ *ᴡᴀʀɴɪɴɢ @628333*"));
        assert!(!text_of(&r).contains("628222"));
    }

    // -- format tanggal ----------------------------------------------------

    #[test]
    fn warning_dates_render_dd_mm_yyyy_in_jakarta() {
        // 2026-01-01T00:00:00Z = 2026-01-01 07:00 WIB.
        assert_eq!(date_dmy(1_767_225_600_000), "01/01/2026");
        // 2025-12-31T17:30:00Z = 2026-01-01 00:30 WIB —?= tanggal bergeser.
        assert_eq!(date_dmy(1_767_212_200_000), "01/01/2026");
    }

    #[test]
    fn a_warning_without_a_timestamp_still_renders_a_row() {
        let r = run(
            json!({"warnings": {"628333@s.whatsapp.net": [{"reason": "x"}]}}),
            vec!["628333@s.whatsapp.net"],
        );
        assert!(text_of(&r).contains("*1.* x\n   └ _01/01/1970_\n"));
    }

    #[test]
    fn a_non_array_entry_is_treated_as_no_warnings() {
        let r = run(json!({"warnings": {"628333@s.whatsapp.net": "bukan-array"}}), vec!["628333@s.whatsapp.net"]);
        assert_eq!(text_of(&r), "✅ @628333 tidak memiliki warning.");
    }
}