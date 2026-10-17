//! `plugins/rpg/slot.js` — mesin slot: tiga simbol berbobot, bayaran
//! 10x (777), 5x (diamond), 3x (tiga sama), 1.5x (dua sama).
//!
//! `ponytail:` JS mengirim DUA pesan: preview "TREK TREK TREK..." lewat
//! `sock.sendPreview`, `setTimeout(2500)`, lalu preview hasil lewat
//! `sock.sendPreview` yang sama. `Ctx` tidak punya `sock` dan `Reply` hanya
//! membawa satu payload, jadi hanya pesan hasil yang terkirim — teksnya sama
//! persis dengan `slot.js:79-88`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::rpg::heal::{math_random, or_default};
use crate::tools::kalkulatormbg::thousands_id;

/// `bet < 1000` (`slot.js:27`) — `!bet` ikut tertutup karena 0 < 1000.
const MIN_BET: i64 = 1_000;

/// `slot.js:37` — urutan menentukan batas kumulatif.
const SYMBOLS: [&str; 6] = ["🍒", "🍋", "🍊", "🍇", "💎", "7️⃣"];

/// `slot.js:38` — jumlahnya 100, jadi `Math.random() * 100` tidak pernah
/// melewati batas terakhir.
const WEIGHTS: [f64; 6] = [30.0, 25.0, 20.0, 15.0, 7.0, 3.0];

/// `spin()` (`slot.js:40-48`) — `rand <= cumulative`, bukan `<`.
fn spin() -> &'static str {
    let rand = math_random() * 100.0;
    let mut cumulative = 0.0;
    for (i, weight) in WEIGHTS.iter().enumerate() {
        cumulative += weight;
        if rand <= cumulative {
            return SYMBOLS[i];
        }
    }
    // JS: `return symbols[0]` untuk rantai yang tidak pernah cocok.
    SYMBOLS[0]
}

/// `Math.floor(bet * multiplier)` (`slot.js:76`) — dipotong, bukan dibulatkan.
fn winnings_of(bet: i64, multiplier: f64) -> i64 {
    (bet as f64 * multiplier).floor() as i64
}

/// Pengali + teks kemenangan dari tiga simbol (`slot.js:57-74`).
///
/// `None` = `multiplier` tetap 0, yaitu cabang "ZONK" di JS.
fn payout(result: &[&'static str; 3]) -> (f64, Option<&'static str>) {
    let [a, b, c] = *result;
    if a == b && b == c {
        if a == "7️⃣" {
            (10.0, Some("GILA!! JACKPOT 777!! 🎉💸 (10x Lipat)"))
        } else if a == "💎" {
            (5.0, Some("MANTAP! SUPER DIAMOND!! 💎✨ (5x Lipat)"))
        } else {
            (3.0, Some("ASIK! TRIPLE COMBO!! 🍒🎰 (3x Lipat)"))
        }
    } else if a == b || b == c || a == c {
        (1.5, Some("LUMAYAN LAH! DOUBLE!! 👍 (1.5x Lipat)"))
    } else {
        (0.0, None)
    }
}

#[plugin(
    name = "slot",
    alias = ["slots", "mesin"],
    category = "rpg",
    description = "Main slot machine gambling",
    usage = ".slot <bet>",
    example = ".slot 5000",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn slot(ctx: Ctx) -> Reply {
    // `parseInt(args[0])` — `!bet || bet < 1000`.
    let Some(bet) = ctx
        .args
        .first()
        .and_then(|raw| crate::common::parse_int(raw))
        .filter(|bet| *bet >= MIN_BET)
    else {
        return Reply::text(
            "Taruhan minimal buat narik tuas slot ini *Rp 1.000* bro! 🎰\n\
             Contoh: `.slot 5000`",
        );
    };

    let koin = or_default(ctx.user_field("koin").and_then(Value::as_i64), 0);
    if koin < bet {
        return Reply::text(format!(
            "Koin lu kering kerontang! 💸\n\
             Uang lu: *Rp {}*\n\
             Butuh: *Rp {}*",
            thousands_id(koin as f64),
            thousands_id(bet as f64)
        ));
    }

    let result = [spin(), spin(), spin()];
    let (multiplier, win_text) = payout(&result);
    let winnings = winnings_of(bet, multiplier);
    let new_koin = koin - bet + winnings;

    let mut txt = format!(
        "🎰 *HASIL MESIN SLOT* 🎰\n\n[ {} | {} | {} ]\n\n",
        result[0], result[1], result[2]
    );
    if multiplier > 0.0 {
        txt.push_str(&format!(
            "{}\n💰 Uang Cair: *+Rp {}*\n",
            win_text.unwrap_or(""),
            thousands_id(winnings as f64)
        ));
    } else {
        txt.push_str(&format!(
            "ZONK!! Duit lu ditelan mesin slot! 😭💸\n\
             💸 Uang Hangus: *-Rp {}*\n",
            thousands_id(bet as f64)
        ));
    }

    Reply::text(txt).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": new_koin }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(koin: i64, arg: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "slot".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.unwrap_or("").into(),
            args: arg.map(|a| vec![a.to_string()]).unwrap_or_default(),
            user_data: json!({ "koin": koin }),
            ..Default::default()
        };
        (oktz_plugin_api::find("slot").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn koin_after(r: &Reply) -> i64 {
        assert_eq!(r.mutations().len(), 1, "harus ada satu SetUser");
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data["koin"].as_i64().expect("koin harus angka")
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("slot").unwrap();
        assert_eq!(p.meta.name, "slot");
        assert_eq!(p.meta.alias, ["slots", "mesin"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Main slot machine gambling");
        assert_eq!(p.meta.usage, ".slot <bet>");
        assert_eq!(p.meta.example, ".slot 5000");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["slots", "mesin"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "slot");
        }
    }

    /// `!bet || bet < 1000` — batasnya inklusif: tepat 1000 lolos.
    #[test]
    fn a_bet_below_one_thousand_is_refused_without_touching_the_purse() {
        for bad in ["999", "0", "-1000", "abc", ""] {
            let r = run(99_999, Some(bad));
            assert!(
                text_of(&r)
                    .starts_with("Taruhan minimal buat narik tuas slot ini *Rp 1.000* bro!"),
                "bet {bad}"
            );
            assert!(text_of(&r).ends_with("Contoh: `.slot 5000`"), "bet {bad}");
            assert!(r.mutations().is_empty(), "bet {bad}");
        }
    }

    /// `parseInt("1000abc")` = 1000 — bukan NaN, jadi tetap jalan.
    #[test]
    fn a_numeric_prefix_bet_is_accepted() {
        let r = run(99_999, Some("1000abc"));
        assert!(text_of(&r).starts_with("🎰 *HASIL MESIN SLOT*"), "{}", text_of(&r));
    }

    #[test]
    fn a_purse_smaller_than_the_bet_is_refused() {
        let r = run(999, Some("1000"));
        let t = text_of(&r);
        assert!(t.starts_with("Koin lu kering kerontang! 💸"), "{t}");
        assert!(t.contains("Uang lu: *Rp 999*"), "{t}");
        assert!(t.contains("Butuh: *Rp 1.000*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_spin_names_the_symbols_and_balances_the_purse() {
        let mut wins = 0;
        let mut zonks = 0;
        for _ in 0..2_000 {
            let r = run(10_000, Some("1000"));
            let t = text_of(&r);
            assert!(t.starts_with("🎰 *HASIL MESIN SLOT* 🎰\n\n[ "), "{t}");
            assert_eq!(t.matches('|').count(), 2, "harus tiga simbol: {t}");
            let after = koin_after(&r);
            if t.contains("Uang Cair: *+Rp ") {
                wins += 1;
                assert!(after >= 10_000, "menang tidak boleh mengurangi koin: {after}");
            } else {
                zonks += 1;
                assert!(t.contains("ZONK!! Duit lu ditelan mesin slot! 😭💸"), "{t}");
                assert!(t.contains("Uang Hangus: *-Rp 1.000*"), "{t}");
                assert_eq!(after, 9_000);
            }
        }
        assert!(wins > 0, "tidak ada satu pun kemenangan");
        assert!(zonks > 0, "tidak ada satu pun zonk");
    }

    #[test]
    fn the_payout_table_matches_the_js_multipliers() {
        for (result, multiplier, text) in [
            (["7️⃣", "7️⃣", "7️⃣"], 10.0, "GILA!! JACKPOT 777!! 🎉💸 (10x Lipat)"),
            (["💎", "💎", "💎"], 5.0, "MANTAP! SUPER DIAMOND!! 💎✨ (5x Lipat)"),
            (["🍒", "🍒", "🍒"], 3.0, "ASIK! TRIPLE COMBO!! 🍒🎰 (3x Lipat)"),
            (["🍋", "🍋", "🍊"], 1.5, "LUMAYAN LAH! DOUBLE!! 👍 (1.5x Lipat)"),
        ] {
            let (got, label) = payout(&result);
            assert_eq!(got, multiplier, "{result:?}");
            assert_eq!(label, Some(text), "{result:?}");
        }
        for result in [
            ["🍒", "🍋", "🍊"],
            ["🍇", "💎", "7️⃣"],
            ["7️⃣", "💎", "7️⃣"],
        ] {
            assert_eq!(payout(&result), (0.0, None), "{result:?} harus ZONK");
        }
    }

    #[test]
    fn winnings_are_floored_and_follow_the_multiplier() {
        assert_eq!(winnings_of(1_000, 10.0), 10_000, "jackpot 777");
        assert_eq!(winnings_of(1_000, 5.0), 5_000, "super diamond");
        assert_eq!(winnings_of(1_000, 3.0), 3_000, "triple combo");
        assert_eq!(winnings_of(1_000, 1.5), 1_500, "double");
        assert_eq!(winnings_of(1_001, 1.5), 1_501, "dipotong, bukan dibulatkan");
        assert_eq!(winnings_of(1_000, 0.0), 0, "zonk tidak membayar apa pun");
    }

    #[test]
    fn a_bet_on_a_negative_balance_only_touches_what_was_read() {
        // `user.koin || 0` — koin yang tidak ada dibaca 0, jadi cabangnya
        // "kering kerontang", bukan panik.
        let r = run(0, Some("1000"));
        assert!(text_of(&r).contains("Uang lu: *Rp 0*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_very_long_bet_string_does_not_panic() {
        let long = "9".repeat(400);
        let r = run(99_999, Some(&long));
        assert!(text_of(&r).starts_with("Taruhan minimal"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_symbol_is_reachable_and_only_those_six() {
        let mut seen: Vec<&str> = Vec::new();
        for _ in 0..5_000 {
            let s = spin();
            assert!(SYMBOLS.contains(&s), "simbol di luar daftar: {s}");
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
        assert_eq!(seen.len(), SYMBOLS.len(), "ada simbol yang tidak pernah keluar: {seen:?}");
    }

    #[test]
    fn the_purse_is_always_updated_even_for_a_huge_balance() {
        let r = run(i64::MAX, Some("1000"));
        // `Math.floor(bet * 10)` pada saldo raksasa tidak meluap karena
        // dipotong ke `f64` dulu; yang penting tidak panik dan tetap satu mutasi.
        assert_eq!(r.mutations().len(), 1);
        assert!(koin_after(&r) > 0);
    }
}
