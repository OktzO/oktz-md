//! `plugins/clan/clanmembers.js` — daftar member clan beserta level & koin.
//!
//! `ponytail:` JS memanggil `db.getUser(jid)` untuk SETIAP member. Baris store
//! milik orang lain tidak bisa dibaca dari `Ctx` — yang tersedia cuma baris
//! sender (`ctx.user_data`) dan baris peserta grup yang sudah dimuat
//! (`ctx.roster.participants[].data`). Tiga sumber itu dipakai di sini.
//! Member yang tidak ada di ketiganya memakai default yang sama dengan JS:
//! `level` 1 dan `koin` 0.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::clan::clancreate::{MAX_MEMBERS, clans, emblem, field_str, members};
use crate::common::jid_number;
use crate::tools::kalkulatormbg::thousands_id;

/// `memberUser?.rpg?.level || memberUser?.level || 1` — `rpg.level` menang,
/// dan `0`/`null` jatuh ke `level` biasa lalu ke 1.
pub(crate) fn level_of(user: &Value) -> i64 {
    let rpg = user
        .get("rpg")
        .and_then(|r| r.get("level"))
        .filter(|v| crate::common::js_truthy(Some(v)))
        .and_then(Value::as_i64);
    let plain = user.get("level").and_then(Value::as_i64);
    rpg.or(plain).unwrap_or(1)
}

/// `(memberUser?.koin || 0).toLocaleString('id-ID')` — `koin` string non-angka
/// jadi `0` di JS (`Number("abc") || 0`).
pub(crate) fn koin_of(user: &Value) -> String {
    let k = user.get("koin");
    if !crate::common::js_truthy(k) {
        return thousands_id(0.0);
    }
    thousands_id(k.and_then(Value::as_f64).unwrap_or(0.0))
}

/// Baris store milik `jid`, dari tiga sumber yang benar-benar tersedia.
pub(crate) fn user_row<'a>(ctx: &'a Ctx, jid: &str) -> Option<&'a Value> {
    if jid == ctx.sender {
        return Some(&ctx.user_data);
    }
    ctx.roster
        .participants
        .iter()
        .find(|p| p.jid == jid)
        .map(|p| &p.data)
}

pub(crate) fn line(leader: bool, jid: &str, user: &Value) -> String {
    format!(
        "{} @{}  Lv.{} · Rp {}",
        if leader { "👑" } else { "•" },
        jid_number(jid),
        level_of(user),
        koin_of(user),
    )
}

pub(crate) fn header(emblem: &str, name: &str, lines: &[String], total: usize) -> String {
    let mut txt = format!("{emblem} *{name}* — Members\n\n");
    txt.push_str(&lines.join("\n"));
    txt.push_str(&format!("\n\n{total}/{MAX_MEMBERS} members"));
    txt
}

#[plugin(
    name = "clanmembers",
    alias = ["clanmember", "guildmembers"],
    category = "clan",
    description = "Lihat daftar member clan",
    usage = ".clanmembers",
    example = ".clanmembers",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clanmembers(ctx: Ctx) -> Reply {
    let clan_id = ctx
        .user_field("clanId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if clan_id.is_empty() {
        return Reply::text("❌ Kamu belum punya clan");
    }

    let all = clans(&ctx);
    let Some(clan) = all.get(&clan_id) else {
        return Reply::text("❌ Clan tidak ditemukan");
    };

    let leader = field_str(clan, "leader");
    let list = members(clan);
    let total = list.len();
    let lines: Vec<String> = list
        .iter()
        .map(|jid| {
            let empty = Value::Null;
            let row = user_row(&ctx, jid).unwrap_or(&empty);
            line(*jid == leader, jid, row)
        })
        .collect();

    let text = header(&emblem(clan), &field_str(clan, "name"), &lines, total);
    Reply::Text {
        text,
        mentions: list,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::json;

    const SENDER: &str = "628111@s.whatsapp.net";
    const LEADER: &str = "628999@s.whatsapp.net";

    fn run(user: Value, clans: Value, roster: Vec<Participant>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clanmembers".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: String::new(),
            args: Vec::new(),
            is_group: true,
            roster: GroupRoster {
                participants: roster,
                is_bot_admin: false,
            },
            user_data: user,
            settings: json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("clanmembers").unwrap().handler)(ctx)
    }

    fn clan(members: Value) -> Value {
        json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": LEADER,
            "members": members,
        })
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn participant(jid: &str, data: Value) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: false,
            is_owner: false,
            data,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clanmembers").unwrap();
        assert_eq!(p.meta.name, "clanmembers");
        assert_eq!(p.meta.alias, ["clanmember", "guildmembers"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Lihat daftar member clan");
        assert_eq!(p.meta.usage, ".clanmembers");
        assert_eq!(p.meta.example, ".clanmembers");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["clanmember", "guildmembers"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "clanmembers");
        }
    }

    #[test]
    fn the_guards_report_no_clan_and_missing_clan() {
        let r = run(json!({}), json!({}), vec![]);
        assert_eq!(text_of(&r), "❌ Kamu belum punya clan");
        assert!(r.mutations().is_empty());

        let r = run(json!({"clanId": "HILANG"}), json!({}), vec![]);
        assert_eq!(text_of(&r), "❌ Clan tidak ditemukan");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_leader_gets_a_crown_and_everyone_else_a_bullet() {
        let r = run(
            json!({"clanId": "DRG"}),
            json!({ "DRG": clan(json!([LEADER, SENDER])) }),
            vec![],
        );
        let t = text_of(&r);
        assert!(t.starts_with("🐉 *Dragon* — Members\n\n"), "{t}");
        assert!(t.contains("👑 @628999  Lv.1 · Rp 0"), "{t}");
        assert!(t.contains("• @628111  Lv.1 · Rp 0"), "{t}");
        assert!(t.ends_with("\n\n2/50 members"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(
                mentions,
                &[
                    LEADER.to_string(),
                    SENDER.to_string(),
                ]
            ),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn the_leader_marker_follows_the_clan_leader_field_not_the_order() {
        let r = run(
            json!({"clanId": "DRG"}),
            json!({ "DRG": clan(json!([SENDER, LEADER])) }),
            vec![],
        );
        let t = text_of(&r);
        assert!(
            t.find("• @628111").unwrap() < t.find("👑 @628999").unwrap(),
            "urutan member tidak menentukan crown: {t}"
        );
    }

    #[test]
    fn roster_and_sender_rows_supply_levels_and_koin() {
        let r = run(
            json!({"clanId": "DRG", "level": 7, "koin": 25_000}),
            json!({ "DRG": clan(json!([SENDER, LEADER])) }),
            vec![participant(LEADER, json!({"rpg": {"level": 12}, "koin": 1234}))],
        );
        let t = text_of(&r);
        assert!(t.contains("• @628111  Lv.7 · Rp 25.000"), "baris sender: {t}");
        assert!(t.contains("👑 @628999  Lv.12 · Rp 1.234"), "baris roster: {t}");
    }

    #[test]
    fn rpg_level_wins_over_the_plain_level_field() {
        let plain = json!({"level": 3, "rpg": {"level": 9}});
        assert_eq!(level_of(&plain), 9);
        let only_rpg0 = json!({"level": 3, "rpg": {"level": 0}});
        assert_eq!(level_of(&only_rpg0), 3, "`rpg.level = 0` falsy → `level`");
        assert_eq!(level_of(&json!({})), 1);
        assert_eq!(level_of(&json!({"level": "abc"})), 1);
    }

    #[test]
    fn koin_falls_back_to_zero_for_missing_or_unparsable_values() {
        assert_eq!(koin_of(&json!({})), "0");
        assert_eq!(koin_of(&json!({"koin": null})), "0");
        assert_eq!(koin_of(&json!({"koin": "abc"})), "0");
        assert_eq!(koin_of(&json!({"koin": 0})), "0");
        assert_eq!(koin_of(&json!({"koin": 1_234_567})), "1.234.567");
    }

    #[test]
    fn the_device_suffix_survives_in_the_display_number() {
        let r = run(
            json!({"clanId": "DRG"}),
            json!({ "DRG": clan(json!(["628999:3@s.whatsapp.net"])) }),
            vec![],
        );
        assert!(text_of(&r).contains("@628999:3"), "{}", text_of(&r));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999:3@s.whatsapp.net".to_string()])
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }
}
