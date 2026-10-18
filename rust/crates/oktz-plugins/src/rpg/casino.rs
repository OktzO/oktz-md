//! `plugins/rpg/casino.js` —jeu dua angka 0..99, seri berarti balik modal.
//!
//! `all` dibaca dengan `/^all$/i` (case-insensitive, whole-string), dan
//! `parseInt` dipakai SEBELUM cek minimum — jadi `.casino 500abc` tetap
//! Bets 500, bukan ditolak.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::rpg::heal::or_default;
use crate::tools::kalkulatormbg::thousands_id;

const MIN_BET: i64 = 1_000;

/// `Math.floor(Math.random() * 100)` — 0..=99.
fn score() -> i64 {
    crate::common::roll(100) as i64
}

/// `isNaN(bet) || bet < 1000`.
fn bet_of(raw: &str, koin: i64) -> Option<i64> {
    let bet = if raw.eq_ignore_ascii_case("all") {
        koin
    } else {
        crate::common::parse_int(raw)?
    };
    (bet >= MIN_BET).then_some(bet)
}

fn how_to(prefix: &str) -> String {
    format!(
        "🎰 *LAS VEGAS KELILING* 🎰\n\n\
         Selamat datang di Kasino! Mau ngadu nasib sama bandar?\n\n\
         *Cara Judi:*\n\
         👉 `{prefix}casino <jumlah>`\n\n\
         Contoh:\n\
         👉 `{prefix}casino 10000`\n\
         👉 `{prefix}casino all` (Nekat bener!)"
    )
}

#[plugin(
    name = "casino",
    alias = ["judi", "gamble"],
    category = "rpg",
    description = "Bermain casino untuk judi",
    usage = ".casino <jumlah>",
    example = ".casino 10000",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn casino(ctx: Ctx) -> Reply {
    let Some(raw) = ctx.args.first() else {
        return Reply::text(how_to(&ctx.prefix));
    };

    let koin = or_default(ctx.user_field("koin").and_then(Value::as_i64), 0);
    let Some(bet) = bet_of(raw, koin) else {
        return Reply::text(
            "Hadeh... mau judi kok modal receh? 💸\n\
             Minimal taruhan di sini *Rp 1.000* bro!",
        );
    };

    if bet > koin {
        return Reply::text(format!(
            "Jangan ngutang bos! 😂\n\
             Uang lu cuma *Rp {}* tapi sok-sokan *Rp {}*.\n\
             Sana kerja dulu!",
            thousands_id(koin as f64),
            thousands_id(bet as f64)
        ));
    }

    let (player, bot) = (score(), score());
    let (result, emoji, delta, taunt) = if player > bot {
        ("MENANG!", "🎉", bet, "\"Cih! Kebetulan doang lu hoki kali ini.\" - *Bandar* 😒")
    } else if player < bot {
        ("KALAH TELAK!", "💸", -bet, "\"AHAHA! Udah miskin makin miskin lu! Pulang sana!\" - *Bandar* 😈")
    } else {
        ("SERI!", "🤝", 0, "\"Hoo... Imbang ya? Boleh juga nyali lu.\" - *Bandar* 👀")
    };
    let new_koin = (koin + delta).max(0);

    let money_line = if delta != 0 {
        format!(
            "Uang Bandar: *{}Rp {}*\n\n",
            if delta > 0 { "+" } else { "" },
            thousands_id(delta as f64)
        )
    } else {
        "Uang Kembali (Balik Modal)\n\n".to_string()
    };

    Reply::text(format!(
        "🎰 *MEJA KASINO DITUTUP!* 🎰\n\n\
         *Papan Skor:*\n\
         👤 Poin Lu: *{player}*\n\
         🤖 Poin Bandar: *{bot}*\n\n\
         *Hasil: {emoji} {result}*\n\
         {money_line}\
         {taunt}\n\n\
         *Sisa Saldo Lu:* Rp {}",
        thousands_id(new_koin as f64)
    ))
    .with_reaction(emoji)
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": new_koin }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, arg: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "casino".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.unwrap_or("").into(),
            args: arg.map(|a| vec![a.to_string()]).unwrap_or_default(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("casino").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("casino").unwrap();
        assert_eq!(p.meta.name, "casino");
        assert_eq!(p.meta.alias, ["judi", "gamble"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Bermain casino untuk judi");
        assert_eq!(p.meta.usage, ".casino <jumlah>");
        assert_eq!(p.meta.example, ".casino 10000");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["judi", "gamble"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "casino");
        }
    }

    #[test]
    fn without_a_bet_the_how_to_is_shown_and_nothing_is_written() {
        let r = run(serde_json::json!({ "koin": 50_000 }), None);
        let t = text_of(&r);
        assert!(t.starts_with("🎰 *LAS VEGAS KELILING*"), "{t}");
        assert!(t.contains("`.casino all`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    /// `isNaN(bet) || bet < 1000` — batasnya inklusif: tepat 1000 lolos.
    #[test]
    fn a_bet_below_one_thousand_is_refused() {
        for bad in ["999", "0", "-5000", "abc", "1e3"] {
            let r = run(serde_json::json!({ "koin": 99_999 }), Some(bad));
            assert!(
                text_of(&r).starts_with("Hadeh... mau judi kok modal receh"),
                "bet {bad}"
            );
            assert!(r.mutations().is_empty(), "bet {bad}");
        }
    }

    #[test]
    fn exactly_one_thousand_is_accepted() {
        let mut played = false;
        for _ in 0..50 {
            let r = run(serde_json::json!({ "koin": 99_999 }), Some("1000"));
            if !text_of(&r).starts_with("Hadeh") {
                played = true;
            }
        }
        assert!(played, "1000 harus lolos");
    }

    /// `parseInt("1000abc")` = 1000 — bukan NaN.
    #[test]
    fn parse_int_lets_a_numeric_prefix_through() {
        assert_eq!(bet_of("1000abc", 0), Some(1000));
        assert_eq!(bet_of("2000px", 0), Some(2000));
        assert_eq!(bet_of("abc", 0), None);
    }

    #[test]
    fn betting_more_than_the_purse_is_refused() {
        let r = run(serde_json::json!({ "koin": 999 }), Some("1000"));
        let t = text_of(&r);
        assert!(t.starts_with("Jangan ngutang bos!"), "{t}");
        assert!(t.contains("Rp 999"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn all_bets_the_whole_purse_and_is_still_subject_to_the_minimum() {
        // koin 500 < 1000 -> "modal receh", bukan "jangan ngutang".
        let small = run(serde_json::json!({ "koin": 500 }), Some("all"));
        assert!(text_of(&small).starts_with("Hadeh"), "{}", text_of(&small));
        assert_eq!(bet_of("all", 500), None);

        // koin 0 juga jatuh ke cabang minimum.
        assert_eq!(bet_of("ALL", 10_000), Some(10_000));
        assert_eq!(bet_of("AlL", 10_000), Some(10_000));
    }

    #[test]
    fn every_round_names_the_scores_the_outcome_and_the_new_balance() {
        let mut win = 0;
        let mut loss = 0;
        let mut draw = 0;
        for _ in 0..900 {
            let r = run(serde_json::json!({ "koin": 10_000 }), Some("1000"));
            let t = text_of(&r);
            assert!(t.starts_with("🎰 *MEJA KASINO DITUTUP!*"), "{t}");
            assert!(t.contains("👤 Poin Lu: *"), "{t}");
            assert!(t.contains("🤖 Poin Bandar: *"), "{t}");
            let data = &r.mutations()[0];
            let Mutation::SetUser { data, .. } = data else {
                panic!("harus SetUser");
            };
            let after = data["koin"].as_i64().unwrap();
            if t.contains("🎉 MENANG!") {
                win += 1;
                assert_eq!(after, 11_000);
                assert!(t.contains("Uang Bandar: *+Rp 1.000*"), "{t}");
            } else if t.contains("💸 KALAH TELAK!") {
                loss += 1;
                assert_eq!(after, 9_000);
                // `moneyChange > 0 ? "+" : ""` — kalah TIDAK diberi minus.
                assert!(t.contains("Uang Bandar: *Rp 1.000*"), "{t}");
                assert!(!t.contains("*-Rp"), "tanda minus hanya untuk jumlah positif: {t}");
            } else {
                draw += 1;
                assert_eq!(after, 10_000, "seri: balik modal");
                assert!(t.contains("Uang Kembali (Balik Modal)"), "{t}");
            }
        }
        assert!(win > 0 && loss > 0 && draw > 0, "win={win} loss={loss} draw={draw}");
    }

    #[test]
    fn the_scores_stay_inside_zero_and_ninety_nine() {
        for _ in 0..500 {
            let s = score();
            assert!((0..=99).contains(&s), "skor {s}");
        }
    }
}