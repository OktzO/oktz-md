//! `plugins/rpg/bank.js` — pindah uang antara dompet (`koin`) dan tabungan
//! (`rpg.bank`).
//!
//! JS membaca `db.db.data.users[cleanJid]` langsung, bukan `db.getUser`, jadi
//! bank tidak ikut efek samping `setUser`. Yang ditulis tetap dua field yang
//! sama: `koin` dan `rpg.bank`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::rpg::heal::{or_default, rpg_int, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

/// `amountStr === "all"` → saldo sumber; selain itu `parseInt`, dan
/// `if (!amount || amount <= 0)` menolak `NaN` maupun 0.
fn amount_of(raw: Option<&String>, balance: i64) -> Option<i64> {
    let raw = raw?;
    let parsed = if raw == "all" { balance } else { crate::common::parse_int(raw)? };
    (parsed > 0).then_some(parsed)
}

fn bank_reply(action: &str, amount: i64, balance: i64, bank: i64) -> Reply {
    if action == "deposit" {
        Reply::text(format!(
            "Makasih kak udah nabung di Bank RPG! 🏦💖\n\n\
             ✅ Deposit berhasil: *Rp {}*\n\
             💳 Saldo Tabungan: *Rp {}*\n\n\
             Uangnya kita simpen dengan aman ya! 🔒✨",
            thousands_id(amount as f64),
            thousands_id(bank as f64)
        ))
    } else {
        Reply::text(format!(
            "Uangnya berhasil ditarik ya kak! 🏧💸\n\n\
             ✅ Penarikan: *Rp {}*\n\
             💰 Uang Cash: *Rp {}*\n\n\
             Jangan boros-boros pakainya ya! 🛍️✨",
            thousands_id(amount as f64),
            thousands_id(balance as f64)
        ))
    }
}

fn info(balance: i64, bank: i64) -> String {
    format!(
        "Halo kak! Selamat datang di Bank RPG! 🏦✨\n\
         Mau ngecek saldo atau ada keperluan lain nih?\n\n\
         💰 Uang Dompet: *Rp {}*\n\
         💳 Saldo Tabungan: *Rp {}*\n\n\
         *Layanan Bank:* 💁‍♀️\n\
         Nabung: `.bank deposit <jumlah>`\n\
         Tarik tunai: `.bank withdraw <jumlah>`\n\n\
         *(Pake kata 'all' kalau mau nabung/narik semuanya sekaligus!)* 🚀",
        thousands_id(balance as f64),
        thousands_id(bank as f64)
    )
}

#[plugin(
    name = "bank",
    alias = ["atm", "nabung", "deposit", "tarik", "withdraw"],
    category = "rpg",
    description = "Bank system untuk menyimpan uang aman dari rampok",
    usage = ".bank <deposit/withdraw> <jumlah>",
    example = ".bank deposit 10000",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn bank(ctx: Ctx) -> Reply {
    let rpg = rpg_map(&ctx);
    // `typeof rpg.bank !== "number"` → 0; selain itu `|| 0`.
    let current_bank = or_default(rpg_int(&rpg, "bank"), 0);
    let current_balance = crate::common::int_field(&ctx, "koin");

    let action = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    match action.as_str() {
        "deposit" | "depo" => {
            let Some(amount) = amount_of(ctx.args.get(1), current_balance) else {
                return Reply::text(
                    "Hayo kak, masukin jumlah koin yang bener dong! Masa nabung angka gaib 😂💸",
                );
            };
            if current_balance < amount {
                return Reply::text(format!(
                    "Eits, uang *cash* kamu nggak cukup kak! 😭\n\
                     Di dompet cuma ada *Rp {}* nih. Nyari duit dulu gih! 🏃💨",
                    thousands_id(current_balance as f64)
                ));
            }
            let new_bank = current_bank + amount;
            bank_reply("deposit", amount, current_balance - amount, new_bank).with_mutation(
                Mutation::SetUser {
                    jid: ctx.sender.clone(),
                    data: json!({ "koin": current_balance - amount, "rpg": { "bank": new_bank } }),
                },
            )
        }
        "withdraw" | "tarik" => {
            let Some(amount) = amount_of(ctx.args.get(1), current_bank) else {
                return Reply::text(
                    "Hayo kak, masukin jumlah koin yang bener dong! Mau narik angin? 😂💸",
                );
            };
            if current_bank < amount {
                return Reply::text(format!(
                    "Yahh kak, saldo tabungan kamu nggak cukup! 😭\n\
                     Di rekening cuma ada *Rp {}* nih. Jangan ngadi-ngadi deh! 🫣",
                    thousands_id(current_bank as f64)
                ));
            }
            let new_balance = current_balance + amount;
            bank_reply("withdraw", amount, new_balance, current_bank - amount).with_mutation(
                Mutation::SetUser {
                    jid: ctx.sender.clone(),
                    data: json!({ "koin": new_balance, "rpg": { "bank": current_bank - amount } }),
                },
            )
        }
        _ => Reply::text(info(current_balance, current_bank)),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bank".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("bank").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bank").unwrap();
        assert_eq!(p.meta.name, "bank");
        assert_eq!(p.meta.alias, ["atm", "nabung", "deposit", "tarik", "withdraw"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Bank system untuk menyimpan uang aman dari rampok"
        );
        assert_eq!(p.meta.usage, ".bank <deposit/withdraw> <jumlah>");
        assert_eq!(p.meta.example, ".bank deposit 10000");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["atm", "nabung", "deposit", "tarik", "withdraw"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "bank");
        }
    }

    #[test]
    fn an_unknown_action_falls_through_to_the_service_menu() {
        for args in [vec![], vec!["transfer"], vec!["tarik!"]] {
            let r = run(json!({ "koin": 1_000, "rpg": { "bank": 200 } }), &args);
            let t = text_of(&r);
            assert!(t.starts_with("Halo kak! Selamat datang di Bank RPG!"), "args {args:?}: {t}");
            assert!(t.contains("💰 Uang Dompet: *Rp 1.000*"), "{t}");
            assert!(t.contains("💳 Saldo Tabungan: *Rp 200*"), "{t}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn a_missing_user_row_reads_as_zero_everywhere() {
        for user in [json!(null), json!({}), json!({ "rpg": {} })] {
            let t_reply = run(user.clone(), &[]);
            let t = text_of(&t_reply);
            assert!(t.contains("💰 Uang Dompet: *Rp 0*"), "input {user}");
            assert!(t.contains("💳 Saldo Tabungan: *Rp 0*"), "input {user}");
        }
    }

    #[test]
    fn a_deposit_moves_money_from_the_purse_into_the_bank() {
        let r = run(json!({ "koin": 30_000, "rpg": { "bank": 5_000 } }), &["deposit", "10000"]);
        let data = write_of(&r);
        assert_eq!(data["koin"], json!(20_000));
        assert_eq!(data["rpg"]["bank"], json!(15_000));
        let t = text_of(&r);
        assert!(t.contains("✅ Deposit berhasil: *Rp 10.000*"), "{t}");
        assert!(t.contains("💳 Saldo Tabungan: *Rp 15.000*"), "{t}");
    }

    #[test]
    fn a_withdrawal_is_the_exact_mirror() {
        let r = run(json!({ "koin": 30_000, "rpg": { "bank": 5_000 } }), &["withdraw", "2000"]);
        let data = write_of(&r);
        assert_eq!(data["koin"], json!(32_000));
        assert_eq!(data["rpg"]["bank"], json!(3_000));
        let t = text_of(&r);
        assert!(t.contains("✅ Penarikan: *Rp 2.000*"), "{t}");
        assert!(t.contains("💰 Uang Cash: *Rp 32.000*"), "{t}");
    }

    #[test]
    fn both_the_full_and_the_short_action_names_work() {
        // Rekening harus berisi dana: `currentBank < amount` menolak penarikan
        // tanpa mutasi, jadi bank 0 tidak bisa dipakai di sini.
        for (full, short) in [("deposit", "depo"), ("withdraw", "tarik")] {
            let a = run(json!({ "koin": 5_000, "rpg": { "bank": 2_000 } }), &[full, "1000"]);
            let b = run(json!({ "koin": 5_000, "rpg": { "bank": 2_000 } }), &[short, "1000"]);
            assert_eq!(write_of(&a), write_of(&b), "{full} vs {short}");
        }
    }

    #[test]
    fn all_deposits_the_whole_purse_and_all_withdraws_the_whole_bank() {
        let dep = run(json!({ "koin": 7_500 }), &["deposit", "all"]);
        assert_eq!(write_of(&dep)["koin"], json!(0));
        assert_eq!(write_of(&dep)["rpg"]["bank"], json!(7_500));

        let wd = run(json!({ "koin": 10, "rpg": { "bank": 4_200 } }), &["withdraw", "all"]);
        assert_eq!(write_of(&wd)["koin"], json!(4_210));
        assert_eq!(write_of(&wd)["rpg"]["bank"], json!(0));
    }

    /// `parseInt` menerima awalan angka, dan `NaN`/`0` ditolak.
    #[test]
    fn a_bad_or_non_positive_amount_is_refused_on_both_sides() {
        for bad in ["abc", "0", "-5", "px"] {
            for action in ["deposit", "withdraw"] {
                let r = run(json!({ "koin": 9_000, "rpg": { "bank": 9_000 } }), &[action, bad]);
                let t = text_of(&r);
                assert!(t.starts_with("Hayo kak, masukin jumlah koin"), "{action} {bad}: {t}");
                assert!(r.mutations().is_empty(), "{action} {bad}");
            }
        }
    }

    #[test]
    fn depositing_more_than_the_purse_is_refused() {
        let r = run(json!({ "koin": 1_000, "rpg": { "bank": 0 } }), &["deposit", "5000"]);
        assert!(text_of(&r).starts_with("Eits, uang *cash* kamu nggak cukup"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn withdrawing_more_than_the_balance_is_refused() {
        let r = run(json!({ "koin": 1_000, "rpg": { "bank": 10 } }), &["withdraw", "500"]);
        assert!(text_of(&r).starts_with("Yahh kak, saldo tabungan kamu nggak cukup"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_full_deposit_uses_the_purse_not_the_bank_as_its_ceiling() {
        // `all` untuk withdraw mengambil `currentBank`, bukan `currentBalance`.
        let r = run(json!({ "koin": 90_000, "rpg": { "bank": 25 } }), &["withdraw", "all"]);
        assert_eq!(write_of(&r)["rpg"]["bank"], json!(0));
        assert_eq!(write_of(&r)["koin"], json!(90_025));
    }
}