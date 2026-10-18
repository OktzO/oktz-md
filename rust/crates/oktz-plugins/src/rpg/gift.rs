//! `plugins/rpg/gift.js` — beri item ke pasangan; love naik 10 per unit.
//!
//! `ponytail:` `db.getUser(spouseJid)` hanya terbaca lewat roster grup, dan
//! plugin ini `isGroup: false` — jadi di private chat pasangan tidak pernah
//! ditemukan dan baris partner tidak pernah ditulis. Capacitas hilang
//! satu-directional: item keluar dari tas pengirim tapi tidak masuk ke
//! pasangan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::jid_number;
use crate::rpg::heal::{inventory_map, other_user, or_default, rpg_map};

/// `Number.isInteger(parseInt(args[1]) || 1)` — `|| 1` membuat nilai selalu
/// >= 1, jadi guard di bawah praktis tak pernah kena.
fn amount_of(raw: Option<&String>) -> i64 {
    // `parseInt(args[1]) || 1` — undefined/NaN/0 semua jadi 1.
    match raw {
        None => 1,
        Some(a) => crate::common::parse_int(a).filter(|n| *n != 0).unwrap_or(1),
    }
}

#[plugin(
    name = "gift",
    alias = ["kasih", "hadiah"],
    category = "rpg",
    description = "Beri hadiah ke pasangan untuk meningkatkan love",
    usage = ".gift <item> <jumlah>",
    example = ".gift diamond 1",
    cooldown = 30,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn gift(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let spouse = rpg
        .get("spouse")
        .filter(|s| crate::common::js_truthy(Some(s)))
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(spouse) = spouse else {
        return Reply::text(
            "❌ *ʙᴇʟᴜᴍ ᴍᴇɴɪᴋᴀʜ*\n\n> Kamu belum menikah!\n> Nikah dulu dengan `.marry @user`",
        );
    };

    // Cek urutan mengikuti JS: jumlah dulu, baru nama item.
    let amount = amount_of(ctx.args.get(1));
    if amount <= 0 {
        return Reply::text("❌ Jumlah item tidak valid. Gunakan angka positif!");
    }

    let Some(item) = crate::common::first_keyword(&ctx) else {
        return Reply::text(
            "🎁 *ɢɪꜰᴛ*\n\n*📋 *ᴜsᴀɢᴇ:*
\n\
             > > Pilih item untuk diberikan\n\
             > > `.gift diamond 1`\n",
        );
    };

    let owned = or_default(inv.get(&item).and_then(Value::as_i64), 0);
    if owned < amount {
        return Reply::text(format!(
            "❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴄᴜᴋᴜᴘ*\n\n> Item *{item}* kamu: {owned}\n> Butuh: {amount}"
        ));
    }

    let partner = other_user(&ctx, &spouse);
    if partner.is_none() {
        return Reply::text(
            "❌ *ᴘᴀsᴀɴɢᴀɴ ɴᴏᴛ ꜰᴏᴜɴᴅ*\n\n> Pasangan tidak ditemukan di database!",
        );
    }

    inv.insert(item.clone(), serde_json::json!(owned - amount));
    let love = or_default(rpg.get("love").and_then(Value::as_i64), 0) + amount * 10;
    rpg.insert("love".into(), serde_json::json!(love));

    // Sisi pasangan hanya ditulis kalau barisnya terbaca lewat roster.
    let partner_mutation = partner.and_then(|p| {
        p.get("rpg").filter(|r| crate::common::js_truthy(Some(r)))?;
        Some(Mutation::SetUser {
            jid: spouse.clone(),
            data: serde_json::json!({ "rpg": { "love": love } }),
        })
    });

    let mut mutations = vec![Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "rpg": rpg }),
    }];
    mutations.extend(partner_mutation);

    Reply::Text {
        text: format!(
            "🎁 *ɢɪꜰᴛ sᴜᴋsᴇs*\n\n\
             > 💝 Kamu memberikan {amount}x {item}\n\
             > 👤 Untuk: @{}\n\
             > 💕 Love: +{}\n\n\
             > _So sweet! 💖_",
            jid_number(&spouse),
            amount * 10
        ),
        mentions: vec![spouse],
    }
    .mutating(mutations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn run(user: Value, partner: Option<Value>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "gift".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: "628222@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: partner.unwrap_or(Value::Null),
                }],
                is_bot_admin: false,
            },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("gift").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn married(inventory: Value) -> Value {
        serde_json::json!({ "inventory": inventory,
                "rpg": { "spouse": "628222@s.whatsapp.net", "love": 20 } })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gift").unwrap();
        assert_eq!(p.meta.name, "gift");
        assert_eq!(p.meta.alias, ["kasih", "hadiah"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Beri hadiah ke pasangan untuk meningkatkan love");
        assert_eq!(p.meta.usage, ".gift <item> <jumlah>");
        assert_eq!(p.meta.example, ".gift diamond 1");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kasih", "hadiah"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "gift");
        }
    }

    #[test]
    fn an_unmarried_user_is_told_to_marry_first() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "rpg": {} })] {
            let r = run(user.clone(), None, &["diamond", "1"]);
            assert!(text_of(&r).starts_with("❌ *ʙᴇʟᴜᴍ ᴍᴇɴɪᴋᴀʜ*"), "input {user}");
            assert!(text_of(&r).contains("Nikah dulu dengan `.marry @user`"), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    /// `parseInt(args[1]) || 1` — `||` menutup `NaN` dan `0`, jadi jumlah tak
    /// pernah nol. Negatif TETAP negatif (`-3` truthy) dan ditahan guard
    /// `amount <= 0` di bawah, jadi guard itu terjangkau.
    #[test]
    fn the_amount_is_never_zero_but_a_negative_one_is_refused() {
        assert_eq!(amount_of(None), 1, "args kosong -> `|| 1` di JS");
        assert_eq!(amount_of(Some(&"0".into())), 1);
        assert_eq!(amount_of(Some(&"abc".into())), 1);
        assert_eq!(amount_of(Some(&"-3".into())), -3, "truthy, jadi bukan 1");
        assert_eq!(amount_of(Some(&"5".into())), 5);

        let r = run(
            married(serde_json::json!({ "diamond": 9 })),
            Some(serde_json::json!({ "rpg": {} })),
            &["diamond", "-3"],
        );
        assert!(text_of(&r).starts_with("❌ Jumlah item tidak valid."), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn without_an_item_the_usage_is_shown() {
        let r = run(married(serde_json::json!({})), None, &[]);
        let t = text_of(&r);
        assert!(t.starts_with("🎁 *ɢɪꜰᴛ*"), "{t}");
        assert!(t.contains("`.gift diamond 1`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_short_bag_is_reported_with_have_over_need() {
        let r = run(married(serde_json::json!({ "diamond": 1 })), None, &["diamond", "5"]);
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴄᴜᴋᴜᴘ*"), "{t}");
        assert!(t.contains("Item *diamond* kamu: 1"), "{t}");
        assert!(t.contains("Butuh: 5"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_partner_outside_the_roster_stops_the_gift() {
        let r = run(married(serde_json::json!({ "diamond": 9 })), None, &["diamond", "2"]);
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ᴘᴀsᴀɴɢᴀɴ ɴᴏᴛ ꜰᴏᴜɴᴅ*"), "{t}");
        assert!(r.mutations().is_empty(), "tak ada pertukaran yang terjadi");
    }

    #[test]
    fn a_gift_moves_the_item_and_raises_love_by_ten_each() {
        let r = run(
            married(serde_json::json!({ "diamond": 9, "gold": 40 })),
            Some(serde_json::json!({ "rpg": { "spouse": "628111@s.whatsapp.net", "love": 5 } })),
            &["diamond", "3"],
        );
        let t = text_of(&r);
        assert!(t.starts_with("🎁 *ɢɪꜰᴛ sᴜᴋsᴇs*"), "{t}");
        assert!(t.contains("💝 Kamu memberikan 3x diamond"), "{t}");
        assert!(t.contains("👤 Untuk: @628222"), "{t}");
        assert!(t.contains("💕 Love: +30"), "{t}");
        assert_eq!(r.mutations().len(), 2, "saya dan pasangan");
        let mine = &r.mutations()[0];
        let Mutation::SetUser { jid, data } = mine else {
            panic!("harus SetUser");
        };
        assert_eq!(jid, "628111@s.whatsapp.net");
        assert_eq!(data["inventory"]["diamond"], serde_json::json!(6));
        assert_eq!(data["inventory"]["gold"], serde_json::json!(40), "item lain tak boleh hilang");
        assert_eq!(data["rpg"]["love"], serde_json::json!(50), "20 + 30");
    }

    #[test]
    fn giving_the_last_one_drains_the_stack_to_zero() {
        let r = run(
            married(serde_json::json!({ "diamond": 2 })),
            Some(serde_json::json!({ "rpg": {} })),
            &["diamond", "2"],
        );
        let mine = &r.mutations()[0];
        let Mutation::SetUser { data, .. } = mine else {
            panic!("harus SetUser");
        };
        assert_eq!(data["inventory"]["diamond"], serde_json::json!(0));
    }

    #[test]
    fn the_spouse_is_mentioned_in_the_reply() {
        let r = run(
            married(serde_json::json!({ "diamond": 5 })),
            Some(serde_json::json!({ "rpg": {} })),
            &["diamond", "1"],
        );
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628222@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }
}