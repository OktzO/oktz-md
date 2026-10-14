//! `plugins/clan/clankick.js` — leader mengeluarkan member dari clan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::clan::clancreate::{
    MAX_MEMBERS, clans, emblem, field_str, members, members_value, save_clans,
};
use crate::clan::claninvite::target_of;
use crate::common::jid_number;

pub(crate) fn help() -> &'static str {
    "👢 *CLAN KICK*\n\nTag atau reply member yang mau dikeluarkan\n\nContoh: *.clankick @user*"
}

pub(crate) fn kicked(emblem: &str, target_num: &str, name: &str, left: usize) -> String {
    format!(
        "{emblem} *KICKED*\n\n@{target_num} dikeluarkan dari *{name}*\nSisa members: {left}/{MAX_MEMBERS}"
    )
}

#[plugin(
    name = "clankick",
    alias = ["kickclan"],
    category = "clan",
    description = "Kick member dari clan (leader only)",
    usage = ".clankick @user",
    example = ".clankick @user",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clankick(ctx: Ctx) -> Reply {
    let clan_id = ctx
        .user_field("clanId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if clan_id.is_empty() {
        return Reply::text("❌ Kamu belum punya clan");
    }

    let mut all = clans(&ctx);
    let Some(clan) = all.get(&clan_id).cloned() else {
        return Reply::text("❌ Clan tidak ditemukan");
    };

    // `clan.leader !== m.sender` — perbandingan string persis, bukan
    // perbandingan nomor; device suffix pada JID tidak dinormalkan.
    if field_str(&clan, "leader") != ctx.sender {
        return Reply::text("❌ Hanya leader yang bisa kick");
    }

    let Some(target) = target_of(&ctx) else {
        return Reply::text(help());
    };

    if target == ctx.sender {
        return Reply::text("❌ Tidak bisa kick diri sendiri");
    }

    let list = members(&clan);
    if !list.contains(&target) {
        return Reply::text("❌ User bukan member clan ini");
    }

    let kept: Vec<String> = list.into_iter().filter(|m| *m != target).collect();
    let left = kept.len();
    let name = field_str(&clan, "name");
    let emb = emblem(&clan);

    let mut updated = clan.clone();
    if let Some(o) = updated.as_object_mut() {
        o.insert("members".into(), members_value(&kept));
    }
    all.insert(clan_id, updated);

    let text = kicked(&emb, jid_number(&target), &name, left);
    Reply::Text {
        text,
        mentions: vec![target.clone()],
    }
    .with_mutation(save_clans(all))
    .with_mutation(Mutation::SetUser {
        jid: target,
        // `db.setUser(target, { clanId: null })` — `null`, bukan `delete`.
        data: json!({ "clanId": Value::Null }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, QuotedKind, QuotedMessage};
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn ctx_of(user: Value, mention: Option<&str>, clans: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "clankick".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: String::new(),
            args: Vec::new(),
            mentioned: mention.map(str::to_string).into_iter().collect(),
            user_data: user,
            settings: json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        }
    }

    fn clan(leader: &str, members: Value) -> Value {
        json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": leader,
            "members": members,
        })
    }

    fn run(c: Ctx) -> Reply {
        (oktz_plugin_api::find("clankick").unwrap().handler)(c)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn kept_of(r: &Reply) -> Value {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetSetting { .. }))
            .expect("harus ada SetSetting");
        match m {
            Mutation::SetSetting { value, .. } => value["DRG"]["members"].clone(),
            _ => unreachable!(),
        }
    }

    fn cleared_of(r: &Reply) -> (String, Value) {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetUser { .. }))
            .expect("harus ada SetUser");
        match m {
            Mutation::SetUser { jid, data } => (jid.clone(), data["clanId"].clone()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clankick").unwrap();
        assert_eq!(p.meta.name, "clankick");
        assert_eq!(p.meta.alias, ["kickclan"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Kick member dari clan (leader only)");
        assert_eq!(p.meta.usage, ".clankick @user");
        assert_eq!(p.meta.example, ".clankick @user");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_guard_order_matches_js() {
        // Tanpa clan → pesan "belum punya clan".
        let r = run(ctx_of(json!({}), Some("628999@s.whatsapp.net"), json!({})));
        assert_eq!(text_of(&r), "❌ Kamu belum punya clan");

        // Clan hilang → "tidak ditemukan".
        let r = run(ctx_of(json!({"clanId": "HILANG"}), Some("628999@s.whatsapp.net"), json!({})));
        assert_eq!(text_of(&r), "❌ Clan tidak ditemukan");

        // Bukan leader → refused, sebelum cek target.
        let r = run(ctx_of(
            json!({"clanId": "DRG"}),
            Some("628999@s.whatsapp.net"),
            json!({ "DRG": clan("628000@s.whatsapp.net", json!(["628999@s.whatsapp.net"])) }),
        ));
        assert_eq!(text_of(&r), "❌ Hanya leader yang bisa kick");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_leader_without_a_target_sees_the_help_card() {
        let r = run(ctx_of(
            json!({"clanId": "DRG"}),
            None,
            json!({ "DRG": clan(SENDER, json!([SENDER])) }),
        ));
        assert_eq!(text_of(&r), help());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn kicking_yourself_or_a_non_member_is_refused() {
        let base = json!({ "DRG": clan(SENDER, json!([SENDER, "628999@s.whatsapp.net"])) });
        let self_kick = run(ctx_of(json!({"clanId": "DRG"}), Some(SENDER), base.clone()));
        assert_eq!(text_of(&self_kick), "❌ Tidak bisa kick diri sendiri");
        assert!(self_kick.mutations().is_empty());

        let stranger = run(ctx_of(json!({"clanId": "DRG"}), Some("628777@s.whatsapp.net"), base));
        assert_eq!(text_of(&stranger), "❌ User bukan member clan ini");
        assert!(stranger.mutations().is_empty());
    }

    #[test]
    fn a_quoted_member_can_be_kicked_without_a_mention() {
        let ctx = Ctx {
            quoted: Some(QuotedMessage {
                id: "X".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: SENDER.into(),
                text: String::new(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..ctx_of(
                json!({"clanId": "DRG"}),
                None,
                json!({ "DRG": clan(SENDER, json!([SENDER, "628999@s.whatsapp.net"])) }),
            )
        };
        let r = run(ctx.clone());
        assert_eq!(kept_of(&r), json!([SENDER]));
        let (jid, cleared) = cleared_of(&r);
        assert_eq!(jid, "628999@s.whatsapp.net");
        assert_eq!(cleared, Value::Null, "`clanId: null`, bukan delete key");
        assert!(text_of(&r).contains("Sisa members: 1/50"), "{}", text_of(&r));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()])
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn kicking_the_last_remaining_member_empties_the_list() {
        let r = run(ctx_of(
            json!({"clanId": "DRG"}),
            Some("628999@s.whatsapp.net"),
            json!({ "DRG": clan(SENDER, json!(["628999@s.whatsapp.net"])) }),
        ));
        assert_eq!(kept_of(&r), json!([]));
        assert!(text_of(&r).contains("Sisa members: 0/50"), "{}", text_of(&r));
    }

    #[test]
    fn the_device_suffix_is_not_stripped_when_matching_the_leader() {
        // `clan.leader !== m.sender` di JS membandingkan string penuh; JID
        // `628111:3@s.whatsapp.net` tidak dianggap leader yang sama.
        let r = run(ctx_of(
            json!({"clanId": "DRG"}),
            Some("628999@s.whatsapp.net"),
            json!({ "DRG": clan("628111:3@s.whatsapp.net", json!(["628999@s.whatsapp.net"])) }),
        ));
        assert_eq!(text_of(&r), "❌ Hanya leader yang bisa kick");
    }
}
