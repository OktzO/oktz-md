//! `plugins/clan/clanleave.js` — keluar dari clan; leader terakhir membubarkan clan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::clan::clancreate::{
    clans, emblem, field_str, members, members_value, save_clans,
};

pub(crate) fn cleaned() -> &'static str {
    "❌ Clan tidak ditemukan, data dibersihkan"
}

pub(crate) fn must_transfer() -> &'static str {
    "❌ Kamu adalah leader!\n\nTransfer dulu: *.clantransfer @user*\nAtau kick semua member terlebih dahulu"
}

pub(crate) fn disbanded(emblem: &str, name: &str) -> String {
    format!("{emblem} Clan *{name}* telah dibubarkan")
}

#[plugin(
    name = "clanleave",
    alias = ["leaveclan", "guildleave"],
    category = "clan",
    description = "Keluar dari clan",
    usage = ".clanleave",
    example = ".clanleave",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clanleave(ctx: Ctx) -> Reply {
    let clan_id = ctx
        .user_field("clanId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if clan_id.is_empty() {
        return Reply::text("❌ Kamu belum punya clan");
    }

    let mut all = clans(&ctx);
    let clear = Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "clanId": Value::Null }),
    };

    let Some(clan) = all.get(&clan_id).cloned() else {
        // `db.setUser(m.sender, { clanId: null })` lalu balas — clan hilang
        // tapi pointer user tidak, jadi ini cabang yang benar-benar membersihkan.
        return Reply::text(cleaned()).with_mutation(clear);
    };

    let list = members(&clan);
    if field_str(&clan, "leader") == ctx.sender {
        if list.len() > 1 {
            return Reply::text(must_transfer());
        }
        let name = field_str(&clan, "name");
        let emb = emblem(&clan);
        // `delete db.db.data.clans[user.clanId]` — clan hilang sepenuhnya.
        all.remove(&clan_id);
        return Reply::text(disbanded(&emb, &name))
            .with_mutation(save_clans(all))
            .with_mutation(clear);
    }

    let kept: Vec<String> = list.into_iter().filter(|m| *m != ctx.sender).collect();
    let name = field_str(&clan, "name");
    let mut updated = clan.clone();
    if let Some(o) = updated.as_object_mut() {
        o.insert("members".into(), members_value(&kept));
    }
    all.insert(clan_id, updated);

    Reply::text(format!("👋 Kamu keluar dari *{name}*"))
        .with_mutation(save_clans(all))
        .with_mutation(clear)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn run(user: Value, clans: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clanleave".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: String::new(),
            args: Vec::new(),
            user_data: user,
            settings: serde_json::json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("clanleave").unwrap().handler)(ctx)
    }

    fn clan(leader: &str, members: Value) -> Value {
        serde_json::json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": leader,
            "members": members,
        })
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn clans_written(r: &Reply) -> Value {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetSetting { .. }))
            .expect("harus ada SetSetting");
        match m {
            Mutation::SetSetting { value, .. } => value.clone(),
            _ => unreachable!(),
        }
    }

    fn cleared(r: &Reply) -> Value {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetUser { .. }))
            .expect("harus ada SetUser");
        match m {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                data["clanId"].clone()
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clanleave").unwrap();
        assert_eq!(p.meta.name, "clanleave");
        assert_eq!(p.meta.alias, ["leaveclan", "guildleave"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Keluar dari clan");
        assert_eq!(p.meta.usage, ".clanleave");
        assert_eq!(p.meta.example, ".clanleave");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn a_user_without_a_clan_is_stopped_before_anything_is_written() {
        let r = run(serde_json::json!({}), serde_json::json!({ "DRG": clan(OTHER, serde_json::json!([OTHER])) }));
        assert_eq!(text_of(&r), "❌ Kamu belum punya clan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_dangling_clan_id_is_cleaned_up_instead_of_failing() {
        let r = run(serde_json::json!({"clanId": "HILANG"}), serde_json::json!({ "DRG": clan(OTHER, serde_json::json!([OTHER])) }));
        assert_eq!(text_of(&r), cleaned());
        assert_eq!(cleared(&r), Value::Null);
        assert_eq!(
            r.mutations().len(),
            1,
            "tidak ada yang ditulis ke peta clan"
        );
    }

    #[test]
    fn a_leader_with_company_cannot_leave() {
        let r = run(
            serde_json::json!({"clanId": "DRG"}),
            serde_json::json!({ "DRG": clan(SENDER, serde_json::json!([SENDER, OTHER])) }),
        );
        assert_eq!(text_of(&r), must_transfer());
        assert!(r.mutations().is_empty(), "tidak boleh ada write parsial");
    }

    #[test]
    fn a_lone_leader_disbands_the_clan_and_clears_the_pointer() {
        let r = run(serde_json::json!({"clanId": "DRG"}), serde_json::json!({ "DRG": clan(SENDER, serde_json::json!([SENDER])) }));
        assert_eq!(text_of(&r), "🐉 Clan *Dragon* telah dibubarkan");
        assert_eq!(cleared(&r), Value::Null);
        let written = clans_written(&r);
        assert!(
            written.get("DRG").is_none(),
            "clan dihapus dari peta: {written}"
        );
    }

    #[test]
    fn a_regular_member_is_only_removed_from_the_list() {
        let r = run(
            serde_json::json!({"clanId": "DRG"}),
            serde_json::json!({ "DRG": clan(OTHER, serde_json::json!([OTHER, SENDER])) }),
        );
        assert_eq!(text_of(&r), "👋 Kamu keluar dari *Dragon*");
        assert_eq!(cleared(&r), Value::Null);
        let written = clans_written(&r);
        assert_eq!(written["DRG"]["members"], serde_json::json!([OTHER]));
        assert_eq!(written["DRG"]["leader"], serde_json::json!(OTHER), "leader tidak berubah");
    }

    #[test]
    fn a_member_missing_from_the_list_still_clears_the_pointer() {
        // JS `filter` tidak melempar; hasilnya tetap daftar lama.
        let r = run(
            serde_json::json!({"clanId": "DRG"}),
            serde_json::json!({ "DRG": clan(OTHER, serde_json::json!([OTHER])) }),
        );
        assert_eq!(text_of(&r), "👋 Kamu keluar dari *Dragon*");
        assert_eq!(clans_written(&r)["DRG"]["members"], serde_json::json!([OTHER]));
    }
}
