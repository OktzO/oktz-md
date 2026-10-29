//! `plugins/rpg/marry.js` — menikahi player lain.
//!
//! Plugin ini menulis DUA row sekaligus (pengirim dan pasangan), jadi reply-nya
//! membawa dua mutasi `SetUser`.
//!
//! `ponytail:` bar pasangan dibaca lewat `crate::rpg::jualan::roster_row`, yang
//! hanya melihat anggota grup. Di private chat pasangan yang sudah punya
//! `rpg.spouse` bisa terlewat — lihat catatan helper itu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::common::now_ms;
use crate::rpg::jualan::{jid_name, roster_rpg, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const MARRIAGE_COST: i64 = 50_000;

pub(crate) fn info(prefix: &str) -> String {
    format!(
        "💒 *CATATAN SIPIL RPG* 💒\n\n\
         Mau melamar ayang? Tag orangnya di sini!\n\n\
         *Cara Melamar:*\n\
        👉 `{prefix}marry @user`\n\n\
         *Syarat:* \n\
         💍 Biaya Nikah: *Rp 50.000*\n\
         (Pastikan doi belum punya pasangan ya!)"
    )
}

#[plugin(
    name = "marry",
    alias = ["nikah", "wedding", "propose"],
    category = "rpg",
    description = "Menikahi player lain",
    usage = ".marry @user",
    example = ".marry @user",
    cooldown = 60,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn marry(ctx: Ctx) -> Reply {
    let target = ctx
        .mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()));

    let Some(target) = target.filter(|t| !t.is_empty()) else {
        return Reply::text(info(&ctx.prefix));
    };

    if target == ctx.sender {
        return Reply::text(
            "Aduh kasihan banget jomblo kronis... Masa mau nikah sama diri sendiri? Cari jodoh sana! 😭💔",
        );
    }

    let spouse = ctx
        .user_field("rpg.spouse")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(spouse) = spouse {
        return Reply::Text {
            text: format!(
                "HEH! Lu kan udah punya pasangan si @{}!\n\
                 Mau poligami? Di server ini nggak boleh! Cerai dulu gih sana pakai `.divorce` 😡🔪",
                jid_name(spouse)
            ),
            mentions: vec![spouse.to_string()],
        };
    }

    let mut partner_rpg = roster_rpg(&ctx, &target);
    if partner_rpg
        .get("spouse")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty())
    {
        return Reply::Text {
            text: format!(
                "Sakit tak berdarah... 🥀\n@{} ternyata udah nikah sama orang lain!\n\
                 Langkahmu terhenti di *friendzone*...",
                jid_name(&target)
            ),
            mentions: vec![target],
        };
    }

    let koin = int_field(&ctx, "koin");
    if koin < MARRIAGE_COST {
        return Reply::text(format!(
            "Astaga... miskin kok nekat mau nikah? 🤦‍♂️\n\
             Biaya KUA dan katering *Rp 50.000*, tapi duit lu cuma *Rp {}*.\n\
             Kerja keras dulu bang!",
            thousands_id(koin as f64)
        ));
    }

    let now = now_ms();
    let mut rpg = rpg_map(&ctx);
    rpg.insert("spouse".into(), serde_json::json!(target));
    rpg.insert("marriedAt".into(), serde_json::json!(now));
    partner_rpg.insert("spouse".into(), serde_json::json!(ctx.sender));
    partner_rpg.insert("marriedAt".into(), serde_json::json!(now));

    Reply::Text {
        text: format!(
            "💒 *PENGUMUMAN PERNIKAHAN!!* 💒\n\n\
             Segenap penghuni server mengucapkan selamat kepada:\n\n\
             👨‍💼/👰 @{}\n           💖 dengan 💖\n👨‍💼/👰 @{}\n\n\
             🎉 *MEREKA RESMI MENJADI PASANGAN!* 🎉\n\n\
             💍 Biaya Resepsi: *Rp -{}*\n\n\
             > _\"Semoga langgeng sampai akhir hayat server ini!\" - Pendeta Bot_ 🥺💕",
            jid_name(&ctx.sender),
            jid_name(&target),
            thousands_id(MARRIAGE_COST as f64)
        ),
        mentions: vec![ctx.sender.clone(), target.clone()],
    }
    .with_reaction("💍")
    .mutating(vec![
        Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin - MARRIAGE_COST, "rpg": rpg }),
        },
        Mutation::SetUser {
            jid: target,
            data: serde_json::json!({ "rpg": partner_rpg }),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, QuotedKind, QuotedMessage};

    const SENDER: &str = "628111@s.whatsapp.net";
    const TARGET: &str = "628999@s.whatsapp.net";

    fn run(user: Value, target: Option<&str>, partner: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "marry".into(),
            sender: SENDER.into(),
            chat: "group@g.usap".into(),
            is_group: true,
            mentioned: target.into_iter().map(String::from).collect(),
            user_data: user,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![Participant {
                    jid: TARGET.into(),
                    is_admin: false,
                    is_owner: false,
                    data: partner,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("marry").unwrap().handler)(ctx)
    }

    fn run_quoted(user: Value, partner: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "marry".into(),
            sender: SENDER.into(),
            chat: "group@gusap".into(),
            is_group: true,
            quoted: Some(QuotedMessage {
                id: "x".into(),
                sender: TARGET.into(),
                chat: "group@gusap".into(),
                text: "".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            timestamp: 0,
        }),
            user_data: user,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![Participant {
                    jid: TARGET.into(),
                    is_admin: false,
                    is_owner: false,
                    data: partner,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("marry").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn writes(r: &Reply) -> Vec<(&str, &Value)> {
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, data } => (jid.as_str(), data),
                other => panic!("harus SetUser, dapat {other:?}"),
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("marry").unwrap();
        assert_eq!(p.meta.name, "marry");
        assert_eq!(p.meta.alias, ["nikah", "wedding", "propose"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Menikahi player lain");
        assert_eq!(p.meta.usage, ".marry @user");
        assert_eq!(p.meta.example, ".marry @user");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private, "hanya di grup");
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["nikah", "wedding", "propose"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "marry");
        }
    }

    #[test]
    fn without_a_target_it_prints_the_civil_registry_notice() {
        let r = run(serde_json::json!({"koin": 999_999}), None, serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("💒 *CATATAN SIPIL RPG*"), "{t}");
        assert!(t.contains("👉 `.marry @user`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_quoted_message_works_as_the_target_too() {
        let r = run_quoted(serde_json::json!({"koin": 100_000}), serde_json::json!({}));
        assert!(text_of(&r).contains("MEREKA RESMI MENJADI PASANGAN!"));
        assert_eq!(writes(&r).len(), 2);
    }

    #[test]
    fn proposing_to_yourself_is_refused_before_the_price_is_ever_read() {
        let r = run(serde_json::json!({}), Some(SENDER), serde_json::json!({}));
        assert!(text_of(&r).contains("jomblo kronis"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_paired_proposer_is_sent_to_the_divorce_desk() {
        let r = run(
            serde_json::json!({"koin": 100_000, "rpg": {"spouse": "628777@s.whatsapp.net"}}),
            Some(TARGET),
            serde_json::json!({}),
        );
        let t = text_of(&r);
        assert!(t.starts_with("HEH! Lu kan udah punya pasangan si @628777!"), "{t}");
        assert!(t.contains("`.divorce`"), "{t}");
        assert_eq!(r.mutations().len(), 0);
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628777@s.whatsapp.net".to_string()])
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_paired_partner_puts_the_proposer_in_the_friendzone() {
        let r = run(
            serde_json::json!({"koin": 100_000}),
            Some(TARGET),
            serde_json::json!({"rpg": {"spouse": "628888@s.whatsapp.net"}}),
        );
        let t = text_of(&r);
        assert!(t.contains("@628999 ternyata udah nikah sama orang lain!"), "{t}");
        assert!(t.contains("friendzone"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_broke_proposer_is_turned_away_with_their_actual_balance() {
        let r = run(serde_json::json!({"koin": 49_999}), Some(TARGET), serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("Astaga... miskin kok nekat mau nikah?"), "{t}");
        assert!(t.contains("*Rp 49.999*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_wedding_writes_both_rows_and_mentions_both_people() {
        let r = run(serde_json::json!({"koin": 100_000}), Some(TARGET), serde_json::json!({"rpg": {}}));
        let w = writes(&r);
        assert_eq!(w.len(), 2, "dua bar harus ditulis");
        assert_eq!(w[0].0, SENDER);
        assert_eq!(w[1].0, TARGET);
        assert_eq!(w[0].1["koin"], serde_json::json!(50_000), "50.000 terpakai");
        assert_eq!(w[0].1["rpg"]["spouse"], serde_json::json!(TARGET));
        assert_eq!(w[1].1["rpg"]["spouse"], serde_json::json!(SENDER));
        assert_eq!(
            w[0].1["rpg"]["marriedAt"], w[1].1["rpg"]["marriedAt"],
            "kedua pihak getting timestamp yang sama"
        );
        assert!(w[0].1["rpg"]["marriedAt"].as_i64().unwrap() > 0);

        match r.without_mutations() {
            Reply::Reaction { emoji, reply } => {
                assert_eq!(emoji, "💍");
                match reply.as_ref() {
                    Reply::Text { mentions, .. } => {
                        assert_eq!(mentions, &[SENDER.to_string(), TARGET.to_string()])
                    }
                    other => panic!("harus Text di dalam Reaction, dapat {other:?}"),
                }
            }
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_wedding_reaction_wraps_the_announcement() {
        let r = run(serde_json::json!({"koin": 100_000}), Some(TARGET), serde_json::json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "💍"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_registry_notice_is_reusable_outside_the_handler() {
        assert!(info("#").contains("`#marry @user`"));
    }
}
