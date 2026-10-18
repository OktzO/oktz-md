//! `plugins/rpg/buykoin.js` — tukar EXP menjadi Koin.
//!
//! Kursor penting: biaya EXP dibaca dari `user.exp` SEBELUM ditulis, dan
//! `parseInt` menerima awalan angka (`"10abc"` → 10). `Math.floor(exp / 2)`
//! untuk `all` dibulatkan ke bawah, tidak ke atas.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::int_field;
use crate::tools::kalkulatormbg::thousands_id;

/// `const EXP_PER_KOIN = 2`.
pub(crate) const EXP_PER_KOIN: i64 = 2;

pub(crate) fn info(exp: i64, koin: i64) -> String {
    format!(
        "💱 *Buy Koin*\n\n\
         > Tukar EXP menjadi Koin!\n\n\
         *📊 Kurs:*\n\
         > 💎 {EXP_PER_KOIN} EXP = 1 Koin\n\n\
         *📋 Saldo:*\n\
         > 🚄 EXP: *{}*\n\
         > 💰 Koin: *{}*\n\n\
         > Contoh: `.buykoin 10000`\n\
         > Akan menggunakan {} EXP untuk 10.000 Koin",
        thousands_id(exp as f64),
        thousands_id(koin as f64),
        thousands_id((10_000 * EXP_PER_KOIN) as f64)
    )
}

/// `amountStr === 'all' || 'max'` → semua EXP, dibulatkan ke bawah.
pub(crate) fn koin_amount(arg: &str, exp: i64) -> Option<i64> {
    let raw = if arg == "all" || arg == "max" {
        exp.div_euclid(EXP_PER_KOIN)
    } else {
        crate::common::parse_int(arg)?
    };
    // `if (!koinAmount || koinAmount <= 0)` — 0 dan negatif ditolak.
    (raw > 0).then_some(raw)
}

pub(crate) fn not_enough(exp_needed: i64, exp: i64) -> String {
    format!(
        "❌ *EXP tidak cukup!*\n\n\
         > Dibutuhkan: *{}* EXP\n\
         > EXP kamu: *{}* EXP\n\n\
         > Maksimal: *{}* Koin*",
        thousands_id(exp_needed as f64),
        thousands_id(exp as f64),
        thousands_id(exp.div_euclid(EXP_PER_KOIN) as f64)
    )
}

pub(crate) fn done(exp_needed: i64, gained: i64, exp: i64, koin: i64) -> String {
    format!(
        "💱 *Tukar Berhasil!*\n\n\
         *📋 Detail:*\n\
         > 🚄 EXP: *-{}*\n\
         > 💰 Koin: *+{}*\n\n\
         *📊 Saldo Sekarang:*\n\
         > 🚄 EXP: *{}*\n\
         > 💰 Koin: *{}*",
        thousands_id(exp_needed as f64),
        thousands_id(gained as f64),
        thousands_id(exp as f64),
        thousands_id(koin as f64)
    )
}

#[plugin(
    name = "buykoin",
    alias = ["belikoin", "belicoin", "exptokoin", "exptocoin"],
    category = "rpg",
    description = "Tukar EXP menjadi Koin",
    usage = ".buykoin <jumlah>",
    example = ".buykoin 10000",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn buykoin(ctx: Ctx) -> Reply {
    let exp = int_field(&ctx, "exp");
    let koin = int_field(&ctx, "koin");

    let Some(arg) = ctx.args.first() else {
        return Reply::text(info(exp, koin));
    };

    let Some(amount) = koin_amount(arg, exp) else {
        return Reply::text("❌ Masukkan jumlah koin yang valid!");
    };

    let exp_needed = amount * EXP_PER_KOIN;
    if exp < exp_needed {
        return Reply::text(not_enough(exp_needed, exp));
    }

    let new_exp = exp - exp_needed;
    let new_koin = koin + amount;

    Reply::text(done(exp_needed, amount, new_exp, new_koin))
        .with_reaction("💱")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "exp": new_exp, "koin": new_koin }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buykoin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("buykoin").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("buykoin").unwrap();
        assert_eq!(p.meta.name, "buykoin");
        assert_eq!(p.meta.alias, ["belikoin", "belicoin", "exptokoin", "exptocoin"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Tukar EXP menjadi Koin");
        assert_eq!(p.meta.usage, ".buykoin <jumlah>");
        assert_eq!(p.meta.example, ".buykoin 10000");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["belikoin", "belicoin", "exptokoin", "exptocoin"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "buykoin");
        }
    }

    #[test]
    fn no_argument_shows_the_exchange_rate_and_current_balances() {
        for (user, exp_s, koin_s) in [
            (serde_json::json!({}), "0", "0"),
            (serde_json::json!({"exp": 25000, "koin": 1000}), "25.000", "1.000"),
        ] {
            let r = run(user.clone(), "");
            let t = text_of(&r);
            assert!(t.starts_with("💱 *Buy Koin*"), "input {user}");
            assert!(t.contains("> 💎 2 EXP = 1 Koin"), "{t}");
            assert!(t.contains(&format!("> 🚄 EXP: *{exp_s}*")), "{t}");
            assert!(t.contains(&format!("> 💰 Koin: *{koin_s}*")), "{t}");
            assert!(r.mutations().is_empty(), "info tidak boleh menulis");
        }
    }

    #[test]
    fn a_successful_exchange_writes_both_exp_and_koin() {
        let r = run(serde_json::json!({"exp": 10000, "koin": 500}), "1000");
        let data = write_of(&r);
        assert_eq!(data["exp"], serde_json::json!(8000), "10000 - 1000*2");
        assert_eq!(data["koin"], serde_json::json!(1500), "500 + 1000");
        assert!(text_of(&r).contains("Tukar Berhasil!"));
        assert!(text_of(&r).contains("> 🚄 EXP: *-2.000*"));
        assert!(text_of(&r).contains("> 💰 Koin: *+1.000*"));
    }

    #[test]
    fn all_and_max_convert_every_remaining_exp_flooring_the_division() {
        for word in ["all", "max"] {
            // 10001 EXP -> floor(10001/2) = 5000 koin, sisa 1 EXP.
            let r = run(serde_json::json!({"exp": 10001, "koin": 0}), word);
            let data = write_of(&r);
            assert_eq!(data["exp"], serde_json::json!(1), "sisa 1 EXP, arg {word}");
            assert_eq!(data["koin"], serde_json::json!(5000), "arg {word}");
        }
    }

    #[test]
    fn an_odd_exp_leaves_a_stranded_point_that_all_cannot_reach() {
        // 9999 EXP -> floor(9999/2) = 4999 koin, sisa 1 EXP. `.buykoin all`
        // kedua kalinya harus gagal, bukan memenuhi 1 EXP.
        let first = run(serde_json::json!({"exp": 9999, "koin": 0}), "all");
        assert_eq!(write_of(&first)["exp"], serde_json::json!(1));
        let second = run(serde_json::json!({"exp": 1, "koin": 0}), "all");
        assert_eq!(
            text_of(&second),
            "❌ Masukkan jumlah koin yang valid!",
            "1 EXP // 2 = 0 koin"
        );
        assert!(second.mutations().is_empty());
    }

    #[test]
    fn insufficient_exp_is_reported_with_the_maximum_possible() {
        let r = run(serde_json::json!({"exp": 100, "koin": 0}), "100");
        let t = text_of(&r);
        assert!(t.starts_with("❌ *EXP tidak cukup!*"), "{t}");
        assert!(t.contains("Dibutuhkan: *200* EXP"), "{t}");
        assert!(t.contains("EXP kamu: *100* EXP"), "{t}");
        assert!(t.contains("Maksimal: *50* Koin"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_exact_balance_is_enough_and_is_consumed_to_zero() {
        let r = run(serde_json::json!({"exp": 2000, "koin": 0}), "1000");
        let data = write_of(&r);
        assert_eq!(data["exp"], serde_json::json!(0));
        assert_eq!(data["koin"], serde_json::json!(1000));
    }

    #[test]
    fn a_zero_or_negative_or_unparsable_amount_is_rejected() {
        for bad in ["0", "-5", "abc"] {
            let r = run(serde_json::json!({"exp": 100000}), bad);
            assert_eq!(
                text_of(&r),
                "❌ Masukkan jumlah koin yang valid!",
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn no_argument_at_all_shows_info_because_args_is_empty() {
        // `amountStr = args[0]` undefined -> cabang info. Token whitespace
        // bukan "tanpa argumen" di JS karena router sudah memecahnya.
        let r = run(serde_json::json!({"exp": 100000}), "");
        assert!(text_of(&r).starts_with("💱 *Buy Koin*"));
        assert!(r.mutations().is_empty());

        let r = run(serde_json::json!({"exp": 100000}), "  ");
        assert_eq!(
            text_of(&r),
            "❌ Masukkan jumlah koin yang valid!",
            "token whitespace di-parse jadi NaN, bukan cabang info"
        );
    }

    #[test]
    fn parse_int_takes_the_leading_integer_and_ignores_the_rest() {
        // `parseInt("1.5.2")` = 1, `parseInt("12abc")` = 12 — bukan NaN.
        // Jadi string ini justru MENJUAL 1 koin, bukan ditolak.
        for (arg, expected_koin) in [("1.5.2", 1), ("10abc", 10), ("7px", 7)] {
            let r = run(serde_json::json!({"exp": 100000, "koin": 0}), arg);
            assert_eq!(
                write_of(&r)["koin"],
                serde_json::json!(expected_koin),
                "parseInt({arg:?}) harus jadi {expected_koin}"
            );
        }
    }

    #[test]
    fn parse_int_accepts_a_numeric_prefix_but_stops_at_the_first_letter() {
        // `parseInt("1000abc")` = 1000 di JS.
        let r = run(serde_json::json!({"exp": 100000, "koin": 0}), "1000abc");
        let data = write_of(&r);
        assert_eq!(data["exp"], serde_json::json!(98000));
        assert_eq!(data["koin"], serde_json::json!(1000));
    }

    #[test]
    fn a_user_with_no_stored_row_still_gets_the_info_branch_without_panicking() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"exp": null})] {
            let r = run(user.clone(), "");
            assert!(text_of(&r).starts_with("💱 *Buy Koin*"), "input {user}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn an_empty_store_cannot_buy_anything_with_all() {
        let r = run(serde_json::json!({}), "all");
        assert_eq!(text_of(&r), "❌ Masukkan jumlah koin yang valid!");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_success_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({"exp": 10000}), "1");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "💱"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(text_of(&r).contains("Tukar Berhasil!"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_helper_contracts_hold() {
        assert_eq!(koin_amount("5", 100), Some(5));
        assert_eq!(koin_amount("all", 101), Some(50));
        assert_eq!(koin_amount("max", 101), Some(50));
        assert_eq!(koin_amount("all", 1), None);
        assert_eq!(koin_amount("0", 100), None);
        assert_eq!(koin_amount("-1", 100), None);
        assert_eq!(koin_amount("abc", 100), None);
    }
}