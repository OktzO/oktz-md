//! `plugins/rpg/weekly.js` — klaim hadiah mingguan.
//!
//! Cooldown dibaca dari `user.cooldowns.weekly` (epoch ms), bukan dari counter
//! router — `cooldown: 0` di `pluginConfig` hanya karena router tidak ikut
//! menghitungnya sendiri.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::{int_field, now_ms};
use crate::rpg::jualan::{exp_gain_only, inventory_map, or_default, roll_incl, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

/// `WEEKLY_COOLDOWN = 7 * 24 * 60 * 60 * 1000`.
pub(crate) const WEEKLY_COOLDOWN: i64 = 7 * 24 * 60 * 60 * 1000;

/// Sisa cooldown dipecah jadi hari + jam, dipotong ke bawah seperti `Math.floor`.
pub(crate) fn remaining_text(remaining_ms: i64) -> String {
    let day = 24 * 60 * 60 * 1000;
    let hour = 60 * 60 * 1000;
    format!(
        "Wah, jatah mingguan kamu udah ludes kak! 😂\n\n\
         Tunggu *{} hari {} jam* lagi ya buat gajian mingguan berikutnya! 🗓️💨",
        remaining_ms / day,
        (remaining_ms % day) / hour
    )
}

#[plugin(
    name = "weekly",
    alias = ["mingguan"],
    category = "rpg",
    description = "Claim hadiah mingguan (lebih besar dari daily)",
    usage = ".weekly",
    example = ".weekly",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn weekly(ctx: Ctx) -> Reply {
    let user = &ctx.user_data;
    let cooldowns = user
        .get("cooldowns")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    // `user.cooldowns.weekly || 0` — nol ikut jadi 0, sama saja.
    let last = or_default(
        cooldowns.get("weekly").and_then(Value::as_i64),
        0,
    );
    let now = now_ms();

    if now - last < WEEKLY_COOLDOWN {
        return Reply::text(remaining_text(last + WEEKLY_COOLDOWN - now));
    }

    let exp_reward = roll_incl(10000, 29999);
    let money_reward = roll_incl(30000, 79999);
    let crate_reward = roll_incl(1, 3);

    let mut inv = inventory_map(&ctx);
    let have = inv.get("uncommon").and_then(Value::as_i64).unwrap_or(0);
    inv.insert("uncommon".into(), json!(have + crate_reward));

    let rpg = rpg_map(&ctx);
    let old_exp = or_default(user.get("exp").and_then(Value::as_i64), 0);
    let new_exp = exp_gain_only(old_exp, exp_reward);
    let mut cd = cooldowns;
    cd.insert("weekly".into(), json!(now));

    let mut data = Map::new();
    data.insert("cooldowns".into(), Value::Object(cd));
    data.insert("exp".into(), json!(new_exp));
    data.insert("inventory".into(), Value::Object(inv));
    data.insert("koin".into(), json!(int_field(&ctx, "koin") + money_reward));
    data.insert("rpg".into(), Value::Object(rpg));

    Reply::text(format!(
        "DORRR! GAJIAN MINGGUAN CAIRRR! 🎉🎊🤑\n\n\
         Gila, jatah kamu minggu ini gede banget:\n\
         📈 EXP: *+{}*\n\
         💰 Koin: *+Rp {}*\n\
         🛍️ Uncommon Crate: *+{crate_reward}x*\n\n\
         Duitnya jangan lupa ditabung ya kak di bank (`.bank`)! 🏦💖",
        thousands_id(exp_reward as f64),
        thousands_id(money_reward as f64)
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: Value::Object(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "weekly".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("weekly").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("weekly").unwrap();
        assert_eq!(p.meta.name, "weekly");
        assert_eq!(p.meta.alias, ["mingguan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Claim hadiah mingguan (lebih besar dari daily)"
        );
        assert_eq!(p.meta.usage, ".weekly");
        assert_eq!(p.meta.example, ".weekly");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("mingguan").unwrap().meta.name, "weekly");
    }

    #[test]
    fn a_brand_new_or_stale_row_can_claim_immediately() {
        for user in [
            json!({}),
            json!({"cooldowns": {}}),
            json!({"cooldowns": {"weekly": 0}}),
            json!({"cooldowns": {"weekly": 1}}),
        ] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("DORRR! GAJIAN MINGGUAN"), "{user}");
            assert_eq!(r.mutations().len(), 1, "{user}");
        }
    }

    #[test]
    fn claiming_this_instant_blocks_the_next_one() {
        let r = run(json!({"cooldowns": {"weekly": now_ms()}}));
        let t = text_of(&r);
        assert!(t.starts_with("Wah, jatah mingguan kamu udah ludes kak!"), "{t}");
        // `last + WEEKLY - now` — `now` diambil sedikit SESUDAH test, jadi
        // sisa bisa 167 atau 168 jam tergantung kecepatan jam dinding.
        let (days, hours) = t
            .split("Tunggu *")
            .nth(1)
            .map(|s| {
                let mut w = s.split_whitespace();
                let d: i64 = w.next().unwrap_or_default().parse().unwrap();
                let h: i64 = w.nth(1).unwrap_or_default().parse().unwrap();
                (d, h)
            })
            .unwrap_or_else(|| panic!("tidak ada hitung mundur: {t}"));
        assert!(
            days * 24 + hours == 167 || days * 24 + hours == 168,
            "sisa harus ~168 jam, dapat {days} hari {hours} jam: {t}"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_countdown_rounds_down_to_whole_days_and_hours() {
        let day = 24 * 60 * 60 * 1000;
        let hour = 60 * 60 * 1000;
        assert!(remaining_text(day * 3 + hour * 5).contains("*3 hari 5 jam*"));
        assert!(remaining_text(day * 7).contains("*7 hari 0 jam*"));
        assert!(remaining_text(day * 2 + hour * 23 + 59 * 60_000).contains("*2 hari 23 jam*"));
    }

    #[test]
    fn a_row_exactly_one_week_old_is_claimable_again() {
        let r = run(json!({"cooldowns": {"weekly": now_ms() - WEEKLY_COOLDOWN}}));
        assert!(text_of(&r).starts_with("DORRR!"), "{:?}", text_of(&r));
    }

    #[test]
    fn the_payout_lands_inside_the_documented_bands_and_stamps_the_timer() {
        for _ in 0..200 {
            let before = now_ms();
            let r = run(json!({"koin": 500, "exp": 100, "inventory": {"uncommon": 2}}));
            let d = written(&r);
            let after = now_ms();
            let t = text_of(&r);

            let exp: i64 = t
                .split("📈 EXP: *+")
                .nth(1)
                .and_then(|s| s.split('*').next())
                .and_then(|s| s.replace('.', "").parse().ok())
                .unwrap();
            let money: i64 = t
                .split("💰 Koin: *+Rp ")
                .nth(1)
                .and_then(|s| s.split('*').next())
                .and_then(|s| s.replace('.', "").parse().ok())
                .unwrap();
            let crates: i64 = t
                .split("Uncommon Crate: *+")
                .nth(1)
                .and_then(|s| s.split('x').next())
                .and_then(|s| s.parse().ok())
                .unwrap();
            assert!((10000..=29999).contains(&exp), "exp {exp}");
            assert!((30000..=79999).contains(&money), "uang {money}");
            assert!((1..=3).contains(&crates), "crate {crates}");

            assert_eq!(d["exp"], json!(100 + exp));
            assert_eq!(d["koin"], json!(500 + money));
            assert_eq!(d["inventory"]["uncommon"], json!(2 + crates));

            let stamped = d["cooldowns"]["weekly"].as_i64().unwrap();
            assert!((before..=after).contains(&stamped), "timestamp harus waktu claim");
        }
    }

    #[test]
    fn other_countdown_keys_survive_the_write() {
        let r = run(json!({"cooldowns": {"daily": 12345}}));
        assert_eq!(written(&r)["cooldowns"]["daily"], json!(12345));
        assert!(written(&r)["cooldowns"]["weekly"].as_i64().unwrap() > 12345);
    }
}
