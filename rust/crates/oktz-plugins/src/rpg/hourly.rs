//! `plugins/rpg/hourly.js` — klaim hadiah sekali per jam.
//!
//! `ponytail:` `config.isPremium(m.sender)` adalah fungsi server; `Ctx` hanya
//! membawa satu flag owner dan field `isPremium` milik pengirim —
//! partner tier tidak terlihat, jadi premium yang berasal dari luar store
//! user tidak terdeteksi di sini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::rpg::heal::{apply_exp, or_default, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

const COOLDOWN_MS: i64 = 3_600_000;
const NORMAL_EXP: i64 = 200;
const NORMAL_MONEY: i64 = 1_000;
const PREMIUM_EXP: i64 = 1_000;
const PREMIUM_MONEY: i64 = 5_000;

/// `msToTime` (`hourly.js:21`) — menit + detik, tanpa jam.
fn ms_to_time(ms: i64) -> String {
    let ms = ms.max(0);
    let minutes = (ms / 1000 / 60) % 60;
    let seconds = (ms / 1000) % 60;
    format!("{minutes} menit {seconds} detik")
}

/// `config.isPremium(sender)` semampunya yang bisa dijawab `Ctx`.
fn is_premium(ctx: &Ctx) -> bool {
    ctx.is_owner || ctx.user_field("isPremium").and_then(Value::as_bool) == Some(true)
}

#[plugin(
    name = "hourly",
    alias = ["jam", "perjam"],
    category = "rpg",
    description = "Klaim hadiah per jam",
    usage = ".hourly",
    example = ".hourly",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn hourly(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let now = crate::common::now_ms();
    let last_claim = or_default(rpg.get("lastHourly").and_then(Value::as_i64), 0);

    if now - last_claim < COOLDOWN_MS {
        return Reply::text(format!(
            "Eits, buru-buru amat kak! 😂\n\n\
             Jatah jam ini udah kamu ambil, tunggu *{}* lagi ya baru balik ke sini! 🏃💨",
            ms_to_time(COOLDOWN_MS - (now - last_claim))
        ));
    }

    let premium = is_premium(&ctx);
    let exp_reward = if premium { PREMIUM_EXP } else { NORMAL_EXP };
    let money_reward = if premium { PREMIUM_MONEY } else { NORMAL_MONEY };

    rpg.insert("lastHourly".into(), json!(now));
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    Reply::text(format!(
        "WAKTUNYA GAJIAN JAM-JAMAN! ⏰✨\n\n\
         Ini dia jatah kamu:\n\
         💸 Koin: *+Rp {}*\n\
         📈 EXP: *+{}*\n\n\
         Balik lagi 1 jam kemudian ya kak! 😘",
        thousands_id(money_reward as f64),
        thousands_id(exp_reward as f64)
    ))
    .with_reaction("⏰")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": crate::common::int_field(&ctx, "koin") + money_reward,
                      "exp": new_exp,
                      "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hourly".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("hourly").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("hourly").unwrap();
        assert_eq!(p.meta.name, "hourly");
        assert_eq!(p.meta.alias, ["jam", "perjam"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Klaim hadiah per jam");
        assert_eq!(p.meta.usage, ".hourly");
        assert_eq!(p.meta.example, ".hourly");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["jam", "perjam"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "hourly");
        }
    }

    #[test]
    fn the_timer_formats_minutes_and_seconds_only() {
        assert_eq!(ms_to_time(0), "0 menit 0 detik");
        assert_eq!(ms_to_time(1_000), "0 menit 1 detik");
        assert_eq!(ms_to_time(90_000), "1 menit 30 detik");
        // `% 60` di JS: satu jam penuh jadi 0 menit — quirk itu ikut apa adanya.
        assert_eq!(ms_to_time(COOLDOWN_MS), "0 menit 0 detik");
        assert_eq!(ms_to_time(-5), "0 menit 0 detik");
    }

    #[test]
    fn a_first_claim_pays_the_free_tier_and_stamps_the_moment() {
        let r = run(json!({ "koin": 500, "exp": 0 }));
        let t = text_of(&r);
        assert!(t.starts_with("WAKTUNYA GAJIAN JAM-JAMAN!"), "{t}");
        assert!(t.contains("💸 Koin: *+Rp 1.000*"), "{t}");
        assert!(t.contains("📈 EXP: *+200*"), "{t}");
        let data = write_of(&r);
        assert_eq!(data["koin"], json!(1_500));
        assert_eq!(data["exp"], json!(200));
        assert!(data["rpg"]["lastHourly"].as_i64().unwrap() > 0);
    }

    #[test]
    fn a_premium_member_gets_five_times_the_payout() {
        let r = run(json!({ "isPremium": true, "koin": 0, "exp": 0 }));
        let t = text_of(&r);
        assert!(t.contains("💸 Koin: *+Rp 5.000*"), "{t}");
        assert!(t.contains("📈 EXP: *+1.000*"), "{t}");
        let data = write_of(&r);
        assert_eq!(data["koin"], json!(5_000));
        assert_eq!(data["exp"], json!(1_000));
    }

    #[test]
    fn an_owner_counts_as_premium_even_without_the_flag() {
        let ctx = Ctx {
            is_owner: true,
            user_data: json!({ "isPremium": false, "koin": 0, "exp": 0 }),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("hourly").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("*+Rp 5.000*"), "{}", text_of(&r));
    }

    #[test]
    fn a_claim_made_within_the_hour_is_refused_and_writes_nothing() {
        let now = crate::common::now_ms();
        for ago in [0, 1_000, 3_599_000] {
            let r = run(json!({ "rpg": { "lastHourly": now - ago }, "koin": 500 }));
            let t = text_of(&r);
            assert!(t.starts_with("Eits, buru-buru amat kak!"), "yang lalu {ago} ms: {t}");
            assert!(r.mutations().is_empty(), "yang lalu {ago} ms");
        }
    }

    /// `user.rpg.lastHourly || 0` — key yang hilang berarti belum pernah klaim,
    /// jadi `now - 0` jauh lebih besar dari cooldown dan klaim langsung boleh.
    #[test]
    fn a_missing_stamp_counts_as_never_claimed() {
        for user in [json!(null), json!({}), json!({ "rpg": {} })] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("WAKTUNYA GAJIAN"), "input {user}");
            assert_eq!(r.mutations().len(), 1, "input {user}");
        }
    }

    #[test]
    fn a_claim_older_than_an_hour_is_allowed_again() {
        let now = crate::common::now_ms();
        let r = run(json!({ "rpg": { "lastHourly": now - COOLDOWN_MS - 1 } }));
        assert!(text_of(&r).starts_with("WAKTUNYA GAJIAN"));
    }
}