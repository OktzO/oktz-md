//! `plugins/clan/claninvite.js` — tambah target ke clan dan tautkan `clanId`-nya.
//!
//! JS memakai `m.mentionedJid?.[0] || m.quoted?.sender`; `Ctx` menyediakan
//! keduanya, jadi urutan itu dipertahankan.
//!
//! `ponytail:` JS menolak target yang `getUser(target).clanId` sudah terisi —
//! baris store milik user lain tidak bisa dibaca dari `Ctx`. Yang di sini
//! dijequivalentkan dengan "target sudah jadi anggota clan mana pun" dari peta
//! `clans`. Selisihnya satu: user dengan `clanId` menunjuk clan yang sudah
//! dihapus lolos di Rust, sedangkan JS menolaknya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::clan::clancreate::{
    MAX_MEMBERS, clans, emblem, field_str, members, members_value, save_clans,
};
use crate::common::jid_number;

pub(crate) fn help() -> &'static str {
    "📨 *CLAN INVITE*\n\nTag atau reply user yang mau diundang\n\nContoh: *.claninvite @user*"
}

pub(crate) fn full() -> String {
    format!("❌ Clan sudah penuh ({MAX_MEMBERS}/{MAX_MEMBERS})")
}

pub(crate) fn invited(emblem: &str, target_num: &str, name: &str, total: usize) -> String {
    format!(
        "{emblem} *INVITED!*\n\n@{target_num} bergabung ke *{name}*\nMembers: {total}/{MAX_MEMBERS}"
    )
}

/// `m.mentionedJid?.[0] || m.quoted?.sender` — mention menang atas reply.
pub(crate) fn target_of(ctx: &Ctx) -> Option<String> {
    ctx.mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()))
        .filter(|t| !t.is_empty())
}

#[plugin(
    name = "claninvite",
    alias = ["inviteclan"],
    category = "clan",
    description = "Invite & langsung tambahkan user ke clan",
    usage = ".claninvite @user",
    example = ".claninvite @user",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn claninvite(ctx: Ctx) -> Reply {
    let clan_id = ctx
        .user_field("clanId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if clan_id.is_empty() {
        return Reply::text("❌ Kamu belum punya clan");
    }

    let mut all = clans(&ctx);
    let Some(clan) = all.get(&clan_id) else {
        return Reply::text("❌ Clan tidak ditemukan");
    };

    let Some(target) = target_of(&ctx) else {
        return Reply::text(help());
    };

    if target == ctx.sender {
        return Reply::text("❌ Tidak bisa invite diri sendiri");
    }

    let mut list = members(clan);
    if list.len() >= MAX_MEMBERS {
        return Reply::text(full());
    }

    let name = field_str(clan, "name");
    let emb = emblem(clan);

    // `if (targetUser?.clanId) return …` — lihat catatan `ponytail:` di atas.
    if all.values().any(|c| members(c).contains(&target)) {
        return Reply::text("❌ User tersebut sudah punya clan");
    }

    list.push(target.clone());
    let total = list.len();

    all.insert(clan_id.clone(), {
        let mut c = clan.clone();
        if let Some(o) = c.as_object_mut() {
            o.insert("members".into(), members_value(&list));
        }
        c
    });

    let text = invited(&emb, jid_number(&target), &name, total);
    Reply::Text {
        text,
        mentions: vec![ctx.sender.clone(), target.clone()],
    }
    .with_mutation(save_clans(all))
    .with_mutation(Mutation::SetUser {
        jid: target,
        data: serde_json::json!({ "clanId": clan_id }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, QuotedKind, QuotedMessage};
    use serde_json::{Value};

    fn ctx_of(user: Value, mention: Option<&str>, quoted: Option<&str>, clans: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "claninvite".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            args: Vec::new(),
            mentioned: mention.map(str::to_string).into_iter().collect(),
            quoted: quoted.map(|s| QuotedMessage {
                id: "X".into(),
                sender: s.into(),
                chat: "628111@s.whatsapp.net".into(),
                text: String::new(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            timestamp: 0,
        }),
            user_data: user,
            settings: serde_json::json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        }
    }

    fn clan(members: Value) -> Value {
        serde_json::json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": "628111@s.whatsapp.net",
            "members": members,
        })
    }

    fn run(c: Ctx) -> Reply {
        (oktz_plugin_api::find("claninvite").unwrap().handler)(c)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (Value, String, String) {
        let clan_m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetSetting { .. }))
            .expect("harus ada SetSetting");
        let target_m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetUser { .. }))
            .expect("harus ada SetUser");
        match (clan_m, target_m) {
            (
                Mutation::SetSetting { value, .. },
                Mutation::SetUser { jid, data },
            ) => {
                let clan_id = data["clanId"].as_str().unwrap_or("").to_string();
                (value["DRG"]["members"].clone(), jid.clone(), clan_id)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("claninvite").unwrap();
        assert_eq!(p.meta.name, "claninvite");
        assert_eq!(p.meta.alias, ["inviteclan"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Invite & langsung tambahkan user ke clan");
        assert_eq!(p.meta.usage, ".claninvite @user");
        assert_eq!(p.meta.example, ".claninvite @user");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn a_sender_without_a_clan_is_stopped_before_anything_else() {
        let r = run(ctx_of(serde_json::json!({}), Some("628999@s.whatsapp.net"), None, serde_json::json!({})));
        assert_eq!(text_of(&r), "❌ Kamu belum punya clan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_clan_id_that_points_nowhere_is_reported() {
        let r = run(ctx_of(serde_json::json!({"clanId": "HILANG"}), Some("628999@s.whatsapp.net"), None, serde_json::json!({})));
        assert_eq!(text_of(&r), "❌ Clan tidak ditemukan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn without_a_target_the_help_branch_runs() {
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            None,
            None,
            serde_json::json!({ "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])) }),
        ));
        assert_eq!(text_of(&r), help());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn inviting_yourself_is_refused_even_with_a_mention() {
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            Some("628111@s.whatsapp.net"),
            None,
            serde_json::json!({ "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])) }),
        ));
        assert_eq!(text_of(&r), "❌ Tidak bisa invite diri sendiri");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_mention_adds_the_target_and_links_them_to_the_clan() {
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            Some("628999@s.whatsapp.net"),
            None,
            serde_json::json!({ "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])) }),
        ));
        let (list, target, clan_id) = written(&r);
        assert_eq!(list, serde_json::json!(["628111@s.whatsapp.net", "628999@s.whatsapp.net"]));
        assert_eq!(target, "628999@s.whatsapp.net");
        assert_eq!(clan_id, "DRG");
        assert!(text_of(&r).contains("@628999 bergabung ke *Dragon*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("Members: 2/50"), "{}", text_of(&r));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(
                mentions,
                &["628111@s.whatsapp.net".to_string(), "628999@s.whatsapp.net".to_string()]
            ),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_quoted_sender_works_when_nothing_is_mentioned() {
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            None,
            Some("628777@s.whatsapp.net"),
            serde_json::json!({ "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])) }),
        ));
        let (list, _, _) = written(&r);
        assert_eq!(list, serde_json::json!(["628111@s.whatsapp.net", "628777@s.whatsapp.net"]));
    }

    #[test]
    fn a_mention_wins_over_a_quote() {
        let c = ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            Some("628999@s.whatsapp.net"),
            Some("628777@s.whatsapp.net"),
            serde_json::json!({ "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])) }),
        );
        let (list, target, clan_id) = written(&run(c.clone()));
        assert_eq!(list, serde_json::json!(["628111@s.whatsapp.net", "628999@s.whatsapp.net"]));
        assert_eq!(target, "628999@s.whatsapp.net");
        assert_eq!(clan_id, "DRG");
    }

    #[test]
    fn a_full_clan_refuses_the_invite() {
        let full_list: Vec<String> = (0..50).map(|i| format!("62800{i}@s.whatsapp.net")).collect();
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            Some("628999@s.whatsapp.net"),
            None,
            serde_json::json!({ "DRG": clan(serde_json::json!(full_list)) }),
        ));
        assert_eq!(text_of(&r), full());
        assert!(r.mutations().is_empty());

        // 49 masih boleh — batasnya `>= 50` checked sebelum push.
        let mut almost = full_list.clone();
        almost.pop();
        let r = run(ctx_of(
            serde_json::json!({ "clanId": "DRG" }),
            Some("628999@s.whatsapp.net"),
            None,
            serde_json::json!({ "DRG": clan(serde_json::json!(almost)) }),
        ));
        let (list, _, _) = written(&r);
        assert_eq!(list.as_array().unwrap().len(), MAX_MEMBERS);
    }

    #[test]
    fn a_target_who_is_already_in_another_clan_is_refused() {
        let clans = serde_json::json!({
            "DRG": clan(serde_json::json!(["628111@s.whatsapp.net"])),
            "OTH": { "id": "OTH", "name": "Lain", "emblem": "🐺",
                     "leader": "628000@s.whatsapp.net",
                     "members": ["628999@s.whatsapp.net"] },
        });
        let r = run(ctx_of(serde_json::json!({"clanId": "DRG"}), Some("628999@s.whatsapp.net"), None, clans));
        assert_eq!(text_of(&r), "❌ User tersebut sudah punya clan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_target_helper_matches_the_js_falsy_semantics() {
        let c = ctx_of(serde_json::json!({}), Some(""), None, serde_json::json!({}));
        assert_eq!(target_of(&c), None, "mention string kosong bukan target");
    }
}
