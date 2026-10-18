//! `plugins/rpg/transfer.js` — kirim koin atau item ke user lain.
//!
//! Dua bar ditulis (pengirim dan penerima) dalam satu reply. Bar penerima dibaca
//! lewat `crate::rpg::jualan::roster_inventory` — lihat catatan `ponytail:` di
//! sana: di luar grup bar itu tidak terbaca dan dianggap kosong.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{jid_name, roster_inventory, roster_row};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) fn help() -> String {
    String::from(
        "🏦 *BANK SENTRAL RPG* 🏦\n\n\
         Layanan pengiriman Koin & Barang Antar-Player!\n\n\
         *Format Pengiriman:*\n\
         👉 `.transfer money 10000 @user` (Untuk Koin)\n\
         👉 `.transfer potion 5 @user` (Untuk Item)\n",
    )
}

fn done(text: String, target: &str) -> Reply {
    Reply::Text {
        text,
        mentions: vec![target.to_string()],
    }
}

#[plugin(
    name = "transfer",
    alias = ["tf", "pay"],
    category = "rpg",
    description = "Transfer uang atau item ke user lain",
    usage = ".transfer <money/nama_item> <jumlah> @user",
    example = ".transfer money 10000 @tag",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn transfer(ctx: Ctx) -> Reply {
    if ctx.args.len() < 3 {
        return Reply::text(help());
    }

    let kind = ctx.args[0].to_lowercase();
    let amount = crate::common::parse_int(&ctx.args[1]).unwrap_or(0);
    let target = ctx
        .mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()));

    let Some(target) = target.filter(|t| !t.is_empty()) else {
        return Reply::text(
            "Alamat paket nggak jelas bos! Tag dulu user yang mau dikirimin! 📦🔍",
        );
    };
    if target == ctx.sender {
        return Reply::text("Ngapain transfer ke kantong sendiri? Kurang kerjaan lu ya! 😂❌");
    }
    // `!amount` sudah menutup 0 DAN negatif sekaligus lewat `|| amount <= 0`.
    if amount <= 0 {
        return Reply::text(
            "Woy bos! Mau kirim angin doang? Jumlahnya harus lebih dari *0*! 🌬️",
        );
    }

    let koin = int_field(&ctx, "koin");

    if matches!(kind.as_str(), "money" | "balance" | "koin") {
        if koin < amount {
            return Reply::text(format!(
                "Transaksi DITOLAK! ❌\n\
                 Saldo ATM lu nggak cukup. Saldo: *Rp {}* | Mau TF: *Rp {}* 💸",
                thousands_id(koin as f64),
                thousands_id(amount as f64)
            ));
        }
        let recipient_koin = roster_row(&ctx, &target)
            .and_then(|u| u.get("koin"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        return done(
            format!(
                "💸 *TRANSFER BERHASIL!* 💸\n\n\
                 Bank Sentral telah mengirim dana:\n\
                 💳 Nominal: *Rp {}*\n\
                 👤 Penerima: @{}\n\n\
                 > _\"Terima kasih telah menggunakan layanan Bank Bot!\"_ 🏦✨",
                thousands_id(amount as f64),
                jid_name(&target)
            ),
            &target,
        )
        .mutating(vec![
            Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "koin": koin - amount }),
            },
            Mutation::SetUser {
                jid: target,
                data: serde_json::json!({ "koin": recipient_koin + amount }),
            },
        ]);
    }

    let mut sender_inv = crate::rpg::jualan::inventory_map(&ctx);
    let mut recipient_inv = roster_inventory(&ctx, &target);
    let have = sender_inv
        .get(kind.as_str())
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if have < amount {
        return Reply::text(format!(
            "Paket gagal diproses! ❌\n\
             Barang *{kind}* di gudang lu cuma ada *{have}* pcs. Lu mau ngirim *{amount}* darimana? 📦"
        ));
    }
    sender_inv.insert(kind.clone(), serde_json::json!(have - amount));
    let theirs = recipient_inv
        .get(kind.as_str())
        .and_then(Value::as_i64)
        .unwrap_or(0);
    recipient_inv.insert(kind.clone(), serde_json::json!(theirs + amount));

    done(
        format!(
            "📦 *PAKET TELAH SAMPAI!* 📦\n\n\
             Kurir berhasil mengantarkan barang:\n\
             🎁 Isi Paket: *{kind}* (x{amount})\n\
             👤 Penerima: @{}\n\n\
             > _\"Paket Pakeeetttt!!\" - Kurir Bot_ 🛵💨",
            jid_name(&target)
        ),
        &target,
    )
    .mutating(vec![
        Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "inventory": sender_inv }),
        },
        Mutation::SetUser {
            jid: target,
            data: serde_json::json!({ "inventory": recipient_inv }),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, QuotedKind, QuotedMessage};
    use oktz_plugin_api::registry::GroupRoster;

    const SENDER: &str = "628111@s.whatsapp.net";
    const TARGET: &str = "628999@s.whatsapp.net";

    fn run(args: &[&str], user: Value, partner: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "transfer".into(),
            sender: SENDER.into(),
            chat: "group@g.usap".into(),
            is_group: true,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            mentioned: vec![TARGET.into()],
            user_data: user,
            roster: GroupRoster {
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
        (oktz_plugin_api::find("transfer").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("transfer").unwrap();
        assert_eq!(p.meta.name, "transfer");
        assert_eq!(p.meta.alias, ["tf", "pay"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Transfer uang atau item ke user lain");
        assert_eq!(p.meta.usage, ".transfer <money/nama_item> <jumlah> @user");
        assert_eq!(p.meta.example, ".transfer money 10000 @tag");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private, "hanya di grup");
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tf", "pay"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "transfer");
        }
    }

    #[test]
    fn fewer_than_three_arguments_prints_the_bank_signboard() {
        for args in [vec![], vec!["money"], vec!["money", "100"]] {
            let r = run(&args, serde_json::json!({}), serde_json::json!({}));
            let t = text_of(&r);
            assert!(t.starts_with("🏦 *BANK SENTRAL RPG*"), "{args:?} -> {t}");
            assert!(t.contains("`.transfer money 10000 @user`"), "{args:?}");
            assert!(t.contains("`.transfer potion 5 @user`"), "{args:?}");
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn without_a_recipient_the_parcel_never_leaves_the_counter() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "transfer".into(),
            sender: SENDER.into(),
            chat: "group@gusap".into(),
            is_group: true,
            args: vec!["money".into(), "1000".into(), "@x".into()],
            user_data: serde_json::json!({"koin": 999_999}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("transfer").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("Alamat paket nggak jelas bos!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_quoted_message_is_a_valid_address() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "transfer".into(),
            sender: SENDER.into(),
            chat: "group@gusap".into(),
            is_group: true,
            args: vec!["money".into(), "1000".into(), "@x".into()],
            quoted: Some(QuotedMessage {
                id: "x".into(),
                sender: TARGET.into(),
                chat: "group@g.usap".into(),
                text: "".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            user_data: serde_json::json!({"koin": 5000}),
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: TARGET.into(),
                    is_admin: false,
                    is_owner: false,
                    data: serde_json::json!({"koin": 7}),
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("transfer").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("TRANSFER BERHASIL!"));
        let w = writes(&r);
        assert_eq!(w[1].1["koin"], serde_json::json!(1007));
    }

    #[test]
    fn sending_to_your_own_pocket_is_refused() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "transfer".into(),
            sender: SENDER.into(),
            chat: "group@gusap".into(),
            is_group: true,
            args: vec!["money".into(), "1000".into(), "@me".into()],
            mentioned: vec![SENDER.into()],
            user_data: serde_json::json!({"koin": 999_999}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("transfer").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("Ngapain transfer ke kantong sendiri?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_zero_or_negative_or_unparsable_amount_is_rejected() {
        for amount in ["0", "-5", "abc"] {
            let r = run(&["money", amount, "@user"], serde_json::json!({"koin": 999_999}), serde_json::json!({}));
            assert!(
                text_of(&r).contains("Mau kirim angin doang?"),
                "amount {amount:?} -> {:?}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "amount {amount:?}");
        }
    }

    #[test]
    fn the_three_money_keywords_behave_identically() {
        for kind in ["money", "balance", "koin", "MONEY"] {
            let r = run(&[kind, "1000", "@user"], serde_json::json!({"koin": 5000}), serde_json::json!({"koin": 3}));
            let w = writes(&r);
            assert_eq!(w[0].1["koin"], serde_json::json!(4000), "{kind}");
            assert_eq!(w[1].1["koin"], serde_json::json!(1003), "{kind}");
        }
    }

    #[test]
    fn an_empty_purse_cannot_cover_the_nominal() {
        let r = run(&["money", "10000", "@user"], serde_json::json!({"koin": 9999}), serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("Transaksi DITOLAK! ❌"), "{t}");
        assert!(t.contains("Saldo: *Rp 9.999*"), "{t}");
        assert!(t.contains("Mau TF: *Rp 10.000*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_item_transfer_moves_the_stack_and_mentions_the_receiver() {
        let r = run(
            &["potion", "2", "@user"],
            serde_json::json!({"inventory": {"potion": 5, "wood": 9}}),
            serde_json::json!({"inventory": {"potion": 1}}),
        );
        let w = writes(&r);
        assert_eq!(w[0].1["inventory"]["potion"], serde_json::json!(3));
        assert_eq!(w[0].1["inventory"]["wood"], serde_json::json!(9), "barang lain tak boleh gone");
        assert_eq!(w[1].1["inventory"]["potion"], serde_json::json!(3));
        assert!(text_of(&r).contains("🎁 Isi Paket: *potion* (x2)"));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[TARGET.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn an_unknown_recipient_bag_starts_from_zero_rather_than_failing() {
        let r = run(
            &["scroll", "1", "@user"],
            serde_json::json!({"inventory": {"scroll": 4}}),
            serde_json::json!({}), // row ada, tapi tanpa inventory
        );
        let w = writes(&r);
        assert_eq!(w[1].1["inventory"]["scroll"], serde_json::json!(1));
    }

    #[test]
    fn sending_more_than_the_warehouse_holds_is_refused() {
        let r = run(&["scroll", "3", "@user"], serde_json::json!({"inventory": {"scroll": 2}}), serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("Paket gagal diproses! ❌"), "{t}");
        assert!(t.contains("Barang *scroll* di gudang lu cuma ada *2* pcs"), "{t}");
        assert!(t.contains("Lu mau ngirim *3* darimana?"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_signboard_is_reusable_outside_the_handler() {
        assert!(help().contains("BANK SENTRAL RPG"));
    }
}
