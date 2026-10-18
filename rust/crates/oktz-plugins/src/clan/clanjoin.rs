//! `plugins/clan/clanjoin.js` — gabung ke clan berdasarkan ID, nama, atau nama clan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::clan::clancreate::{
    MAX_MEMBERS, clans, emblem, field_str, find_clan_entry, members, members_value, save_clans,
};
use crate::common::jid_number;

pub(crate) fn help() -> &'static str {
    "🏰 *JOIN CLAN*\n\nMasukkan ID clan!\n\nContoh: *.clanjoin clan_123456*\nCek ID: *.clanleaderboard*"
}

pub(crate) fn welcome(emblem: &str, sender_num: &str, name: &str, leader: &str, total: usize) -> String {
    format!(
        "{emblem} *WELCOME!*\n\n\
         @{sender_num} bergabung ke *{name}*\n\n\
         Leader: @{leader_num}\n\
         Members: {total}/{MAX_MEMBERS}\n\n\
         Lihat info: *.claninfo*",
        leader_num = jid_number(leader),
    )
}

#[plugin(
    name = "clanjoin",
    alias = ["joinclan", "guildjoin"],
    category = "clan",
    description = "Gabung ke clan",
    usage = ".clanjoin <clan_id>",
    example = ".clanjoin clan_123456",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clanjoin(ctx: Ctx) -> Reply {
    let arg = ctx.text.trim().to_string();
    if arg.is_empty() {
        return Reply::text(help());
    }

    if crate::common::js_truthy(ctx.user_field("clanId")) {
        return Reply::text("❌ Kamu sudah punya clan\nKeluar dulu: *.clanleave*");
    }

    let mut all = clans(&ctx);
    let Some((id, clan)) = find_clan_entry(&all, &arg) else {
        return Reply::text("❌ Clan tidak ditemukan");
    };
    let id = id.clone();
    let clan = clan.clone();

    let name = field_str(&clan, "name");
    if !crate::common::js_truthy(clan.get("isOpen")) {
        return Reply::text(format!("❌ *{name}* sedang tertutup"));
    }

    let mut list = members(&clan);
    if list.len() >= MAX_MEMBERS {
        return Reply::text(format!("❌ *{name}* sudah penuh ({MAX_MEMBERS}/{MAX_MEMBERS})"));
    }

    let emb = emblem(&clan);
    let leader = clan
        .get("leader")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    list.push(ctx.sender.clone());
    let total = list.len();

    let mut updated = clan.clone();
    if let Some(o) = updated.as_object_mut() {
        o.insert("members".into(), members_value(&list));
    }
    all.insert(id.clone(), updated);

    let text = welcome(&emb, jid_number(&ctx.sender), &name, &leader, total);
    let mut mentions = vec![ctx.sender.clone()];
    if !leader.is_empty() {
        mentions.push(leader);
    }
    Reply::Text { text, mentions }
        .with_mutation(save_clans(all))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "clanId": id }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(user: Value, arg: &str, clans: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clanjoin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: arg.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            settings: serde_json::json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("clanjoin").unwrap().handler)(ctx)
    }

    fn clan(extra: Value) -> Value {
        let mut c = serde_json::json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": "628999@s.whatsapp.net",
            "members": ["628999@s.whatsapp.net"],
            "isOpen": true,
        });
        if let Some(o) = c.as_object_mut() {
            for (k, v) in extra.as_object().unwrap() {
                o.insert(k.clone(), v.clone());
            }
        }
        c
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (Value, String, String) {
        let cm = r.mutations().iter().find(|m| matches!(m, Mutation::SetSetting { .. })).unwrap();
        let um = r.mutations().iter().find(|m| matches!(m, Mutation::SetUser { .. })).unwrap();
        match (cm, um) {
            (Mutation::SetSetting { value, .. }, Mutation::SetUser { jid, data }) => (
                value["DRG"]["members"].clone(),
                jid.clone(),
                data["clanId"].as_str().unwrap_or("").to_string(),
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clanjoin").unwrap();
        assert_eq!(p.meta.name, "clanjoin");
        assert_eq!(p.meta.alias, ["joinclan", "guildjoin"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Gabung ke clan");
        assert_eq!(p.meta.usage, ".clanjoin <clan_id>");
        assert_eq!(p.meta.example, ".clanjoin clan_123456");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["joinclan", "guildjoin"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "clanjoin");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_help_card() {
        let r = run(serde_json::json!({}), "", serde_json::json!({}));
        assert_eq!(text_of(&r), help());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_user_who_already_has_a_clan_is_told_to_leave_first() {
        let r = run(serde_json::json!({"clanId": "LAIN"}), "DRG", serde_json::json!({ "DRG": clan(serde_json::json!({})) }));
        assert_eq!(text_of(&r), "❌ Kamu sudah punya clan\nKeluar dulu: *.clanleave*");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_clan_is_reported() {
        let r = run(serde_json::json!({}), "Zzz", serde_json::json!({ "DRG": clan(serde_json::json!({})) }));
        assert_eq!(text_of(&r), "❌ Clan tidak ditemukan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_closed_clan_refuses_the_join() {
        let r = run(
            serde_json::json!({}),
            "DRG",
            serde_json::json!({ "DRG": clan(serde_json::json!({"isOpen": false})) }),
        );
        assert_eq!(text_of(&r), "❌ *Dragon* sedang tertutup");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_full_clan_refuses_the_join() {
        let full: Vec<String> = (0..50).map(|i| format!("62800{i}@s.whatsapp.net")).collect();
        let r = run(serde_json::json!({}), "DRG", serde_json::json!({ "DRG": clan(serde_json::json!({"members": full})) }));
        assert_eq!(text_of(&r), "❌ *Dragon* sudah penuh (50/50)");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_successful_join_appends_the_sender_and_links_the_clan_id() {
        let r = run(serde_json::json!({}), "DRG", serde_json::json!({ "DRG": clan(serde_json::json!({})) }));
        let (list, jid, id) = written(&r);
        assert_eq!(list, serde_json::json!(["628999@s.whatsapp.net", "628111@s.whatsapp.net"]));
        assert_eq!(jid, "628111@s.whatsapp.net");
        assert_eq!(id, "DRG", "ID yang disimpan adalah KUNCI peta, bukan `clan.id`");
        let t = text_of(&r);
        assert!(t.starts_with("🐉 *WELCOME!*"), "{t}");
        assert!(t.contains("Leader: @628999"), "{t}");
        assert!(t.contains("Members: 2/50"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(
                mentions,
                &["628111@s.whatsapp.net".to_string(), "628999@s.whatsapp.net".to_string()]
            ),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn joining_by_name_or_lowercase_id_resolves_the_same_clan() {
        for arg in ["Dragon", "dragon", "DRG"] {
            let r = run(serde_json::json!({}), arg, serde_json::json!({ "DRG": clan(serde_json::json!({})) }));
            let (_, _, id) = written(&r);
            assert_eq!(id, "DRG", "arg {arg}");
        }
    }
}
