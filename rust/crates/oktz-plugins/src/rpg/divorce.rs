//! `plugins/rpg/divorce.js` — cerai dari pasangan, biaya 25.000.
//!
//! `ponytail:` `db.getUser(spouseJid)` hanya terbaca lewat roster grup, dan
//! plugin ini `isGroup: false` — jadi di private chat pasangan tidak pernah
//! ditemukan dan hanya sisi pemanggil yang dibersihkan. `spouse` dan
//! `marriedAt` ditulis `null`, bukan `delete`, jadi tetap terbaca sebagai
//! "tidak punya pasangan" (`!user.rpg.spouse`).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;
use serde_json::json;

use crate::common::jid_number;
use crate::rpg::heal::{other_user, rpg_map};

const DIVORCE_COST: i64 = 25_000;

#[plugin(
    name = "divorce",
    alias = ["cerai", "pisah"],
    category = "rpg",
    description = "Bercerai dari pasangan",
    usage = ".divorce",
    example = ".divorce",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn divorce(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);

    // `!user.rpg.spouse` — string apa pun (termasuk "") dianggap pasangan.
    let spouse = rpg
        .get("spouse")
        .filter(|s| crate::common::js_truthy(Some(s)))
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(spouse) = spouse else {
        return Reply::text(
            "Halu tingkat tinggi... Nikah aja belum masa udah mau cerai? 😂💔\n\
             Cari pasangan dulu gih pake `.marry @user`",
        );
    };

    let koin = crate::common::int_field(&ctx, "koin");
    if koin < DIVORCE_COST {
        return Reply::text(format!(
            "Aduh, biaya pengacara buat cerai mahal bos! 😭\n\
             Butuh *Rp 25.000* buat tanda tangan surat cerai, duit lu cuma *Rp {}*.\n\
             Tahan dulu aja berantemnya!",
            crate::tools::kalkulatormbg::thousands_id(koin as f64)
        ));
    }

    rpg.insert("spouse".into(), Value::Null);
    rpg.insert("marriedAt".into(), Value::Null);

    // Sisi pasangan hanya boleh ditulis kalau barisnya memang terbaca.
    let partner_mutation = other_user(&ctx, &spouse)
        .and_then(|u| u.get("rpg"))
        .filter(|r| crate::common::js_truthy(Some(r)))
        .map(|_| {
            let mut partner_rpg = rpg.clone();
            partner_rpg.clear();
            partner_rpg.insert("spouse".into(), Value::Null);
            partner_rpg.insert("marriedAt".into(), Value::Null);
            Mutation::SetUser {
                jid: spouse.clone(),
                data: json!({ "rpg": partner_rpg }),
            }
        });

    let mut mutations = vec![Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin - DIVORCE_COST, "rpg": rpg }),
    }];
    mutations.extend(partner_mutation);

    Reply::Text {
        text: format!(
            "⛈️ *SIDANG PERCERAIAN SELESAI* ⛈️\n\n\
             Palu telah diketuk. Dengan berat hati, hubungan antara:\n\
             💔 @{sender}\n\
             \x20        -- PUTUS DENGAN --\n\
             💔 @{spouse}\n\n\
             😭 *RESMI BERAKHIR! KINI KALIAN KEMBALI JOMBLO!* 😭\n\n\
             💸 Biaya Pengacara/Sidang: *Rp -{cost}*\n\n\
             > _\"Sudah sudah... nangisnya di pojokan aja. Life must go on...\" - Hakim Bot_ 🥀🚬",
            sender = jid_number(&ctx.sender),
            spouse = jid_number(&spouse),
            cost = crate::tools::kalkulatormbg::thousands_id(DIVORCE_COST as f64)
        ),
        mentions: vec![ctx.sender.clone(), spouse],
    }
    .with_reaction("💔")
    .mutating(mutations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use serde_json::json;

    fn run(user: Value, partner: Option<Value>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "divorce".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: "628222@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: partner.unwrap_or(Value::Null),
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("divorce").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn married(koin: i64) -> Value {
        json!({ "koin": koin,
                "rpg": { "spouse": "628222@s.whatsapp.net",
                         "marriedAt": 1_700_000_000_000_i64 } })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("divorce").unwrap();
        assert_eq!(p.meta.name, "divorce");
        assert_eq!(p.meta.alias, ["cerai", "pisah"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Bercerai dari pasangan");
        assert_eq!(p.meta.usage, ".divorce");
        assert_eq!(p.meta.example, ".divorce");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["cerai", "pisah"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "divorce");
        }
    }

    #[test]
    fn an_unmarried_user_is_mocked_and_nothing_is_written() {
        for user in [json!(null), json!({}), json!({ "rpg": {} })] {
            let r = run(user.clone(), None);
            assert!(
                text_of(&r).starts_with("Halu tingkat tinggi... Nikah aja belum"),
                "input {user}"
            );
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn the_fee_is_charged_and_the_marriage_is_cleared() {
        let r = run(married(50_000), Some(json!({ "rpg": { "spouse": "628111@s.whatsapp.net" } })));
        let t = text_of(&r);
        assert!(t.starts_with("⛈️ *SIDANG PERCERAIAN SELESAI*"), "{t}");
        assert!(t.contains("💔 @628111"), "{t}");
        assert!(t.contains("💔 @628222"), "{t}");
        assert!(t.contains("💸 Biaya Pengacara/Sidang: *Rp -25.000*"), "{t}");
        let mine = &r.mutations()[0];
        let Mutation::SetUser { jid, data } = mine else {
            panic!("harus SetUser");
        };
        assert_eq!(jid, "628111@s.whatsapp.net");
        assert_eq!(data["koin"], json!(25_000));
        assert_eq!(data["rpg"]["spouse"], Value::Null);
        assert_eq!(data["rpg"]["marriedAt"], Value::Null);
    }

    #[test]
    fn an_unknown_partner_leaves_only_the_callers_side_cleared() {
        // Private chat: roster kosong, `db.getUser(spouseJid)` tak terbaca.
        let r = run(married(50_000), None);
        assert_eq!(r.mutations().len(), 1);
        assert_eq!(r.mutations()[0].clone(), Mutation::SetUser {
            jid: "628111@s.whatsapp.net".into(),
            data: json!({ "koin": 25_000,
                          "rpg": { "spouse": Value::Null, "marriedAt": Value::Null } }),
        });
    }

    #[test]
    fn a_readable_partner_with_rpg_is_cleared_too() {
        let r = run(
            married(50_000),
            Some(json!({ "rpg": { "spouse": "628111@s.whatsapp.net", "marriedAt": 1, "love": 40 } })),
        );
        assert_eq!(r.mutations().len(), 2);
        let second = &r.mutations()[1];
        let Mutation::SetUser { jid, data } = second else {
            panic!("harus SetUser");
        };
        assert_eq!(jid, "628222@s.whatsapp.net");
        assert_eq!(data["rpg"]["spouse"], Value::Null);
        assert_eq!(data["rpg"]["marriedAt"], Value::Null);
        assert!(
            data["rpg"].get("love").is_none(),
            "hanya dua field yang ditulis; sisanya mengikuti `??` di store"
        );
    }

    #[test]
    fn the_fee_is_refused_below_twenty_five_thousand() {
        for koin in [0, 24_999] {
            let r = run(married(koin), None);
            let t = text_of(&r);
            assert!(t.starts_with("Aduh, biaya pengacara buat cerai mahal"), "koin {koin}: {t}");
            assert!(
                t.contains(&format!(
                    "duit lu cuma *Rp {}*",
                    crate::tools::kalkulatormbg::thousands_id(koin as f64)
                )),
                "{t}"
            );
            assert!(r.mutations().is_empty(), "koin {koin}");
        }
    }

    #[test]
    fn exactly_twenty_five_thousand_is_enough() {
        let r = run(married(DIVORCE_COST), None);
        assert!(text_of(&r).starts_with("⛈️ *SIDANG PERCERAIAN SELESAI*"), "{}", text_of(&r));
        assert_eq!(r.mutations()[0].clone(), Mutation::SetUser {
            jid: "628111@s.whatsapp.net".into(),
            data: json!({ "koin": 0,
                          "rpg": { "spouse": Value::Null, "marriedAt": Value::Null } }),
        });
    }

    #[test]
    fn both_sides_are_mentioned_in_the_reply() {
        let r = run(married(50_000), None);
        // JS: `m.react("💔")` lalu `m.reply(txt, { mentions })` — reaksi
        // membungkus balasan, jadi mentions ada di `Text` di dalam `Reaction`.
        let Reply::Reaction { emoji, reply } = r.without_mutations() else {
            panic!("harus Reaction, dapat {:?}", r.without_mutations());
        };
        assert_eq!(emoji, "💔");
        let Reply::Text { mentions, .. } = reply.as_ref() else {
            panic!("harus Text di dalam Reaction");
        };
        assert_eq!(
            mentions,
            &[
                "628111@s.whatsapp.net".to_string(),
                "628222@s.whatsapp.net".to_string()
            ]
        );
    }
}