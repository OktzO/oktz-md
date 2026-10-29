//! `plugins/rpg/adventure.js` — petualangan acak: menang dapat koin + exp,
//! kalah kehilangan HP (dan kalau habis, exp dipotong setengah).
//!
//! Yang tidak ikut ke port: `m.react("🗺️")` dan pesan "Mengepak ransel..."
//! sebelum jeda `setTimeout` (`Reply` hanya bisa satu pesan), plus kartu
//! level-up yang butuh canvas.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{apply_exp, or_default, rpg_int, rpg_map, roll_incl, roll_percent};
use crate::tools::kalkulatormbg::thousands_id;

const LOCATIONS: [&str; 6] = [
    "🌲 Hutan Gelap",
    "🏔️ Gunung Es Abadi",
    "🏜️ Padang Pasir Kematian",
    "🌋 Gunung Berapi",
    "🏰 Kastil Tua Berhantu",
    "🌊 Pantai Misterius",
];

/// `Math.random() < 0.6` — 60 dari 100 hasil undian, bukan 60 dari 99.
const WIN_PERCENT: i64 = 60;

/// `user.rpg.health = user.rpg.health || 100` — nol ikut jadi 100.
fn current_health(rpg: &Map<String, Value>) -> i64 {
    or_default(rpg_int(rpg, "health"), 100)
}

#[plugin(
    name = "adventure",
    alias = ["adv", "petualangan"],
    category = "rpg",
    description = "Berpetualang untuk mendapat Exp dan hadiah",
    usage = ".adventure",
    example = ".adventure",
    cooldown = 120,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn adventure(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let health = current_health(&rpg);

    if health < 30 {
        return Reply::text(format!(
            "Aduh kak, HP kamu sekarat nih! 😭💔\n\n\
             Minimal butuh *30 HP* buat berpetualang biar nggak mati di jalan.\n\
             Sekarang HP kamu cuma sisa *{health} HP*. Yuk nge-heal dulu! 💉✨"
        ));
    }

    let location = LOCATIONS[crate::common::random_index(LOCATIONS.len())];

    if roll_percent() < WIN_PERCENT {
        let exp_gain = roll_incl(500, 2_499);
        let money_gain = roll_incl(2_000, 11_999);
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

        return Reply::text(format!(
            "🗡️ *PETUALANGAN BERHASIL!!* 🗡️\n\n\
             📍 Lokasi: *{location}*\n\n\
             Wah hebat kak! Kamu berhasil ngalahin monster penjaga dan nemuin peti harta karun!\n\
             💰 Koin: *+Rp {}*\n\
             📈 EXP: *+{}*\n\n\
             Kembali dengan selamat! Lanjut petualang lagi nanti ya kak! 🚀✨",
            thousands_id(money_gain as f64),
            thousands_id(exp_gain as f64)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": crate::common::int_field(&ctx, "koin") + money_gain,
                          "exp": new_exp,
                          "rpg": rpg }),
        });
    }

    let health_loss = roll_incl(10, 39);
    let left = (health - health_loss).max(0);
    rpg.insert("health".into(), serde_json::json!(left));

    let mut msg = format!(
        "☠️ *DISERGAP MONSTER!!* ☠️\n\n\
         📍 Lokasi: *{location}*\n\n\
         Aduh kak! Langkah kamu ketahuan, sekelompok monster nyerang bertubi-tubi!\n\
         ❤️ HP Berkurang: *-{health_loss} HP* (Sisa: {left})\n\n"
    );

    // `user.exp = Math.floor((user.exp || 0) / 2)` jalan DI LUAR
    // `addExpWithLevelCheck`, jadi tidak ada cek level dan tak pernah naik level.
    let exp_after = if left <= 0 {
        msg.push_str(
            "💀 *KAMU MATI!*\n\
             Yaampun kak...Kamu tewas di tempat. EXP kamu kena penalti 50% nh. 💔🥀",
        );
        Some(or_default(ctx.user_field("exp").and_then(Value::as_i64), 0) / 2)
    } else {
        msg.push_str("Untung kamu masih sempet kabur kak! Mending istirahat dulu buat ngeheal ya! 🏃💨");
        None
    };

    let data = match exp_after {
        Some(exp) => serde_json::json!({ "exp": exp, "rpg": rpg }),
        None => serde_json::json!({ "rpg": rpg }),
    };
    Reply::text(msg).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "adventure".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("adventure").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("adventure").unwrap();
        assert_eq!(p.meta.name, "adventure");
        assert_eq!(p.meta.alias, ["adv", "petualangan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Berpetualang untuk mendapat Exp dan hadiah"
        );
        assert_eq!(p.meta.usage, ".adventure");
        assert_eq!(p.meta.example, ".adventure");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["adv", "petualangan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "adventure");
        }
    }

    #[test]
    fn a_weak_body_is_turned_away_without_writing_anything() {
        // `user.rpg.health || 100`: 0 berarti "belum diisi" -> 100, bukan sekarat.
        for health in [1, 10, 29] {
            let r = run(serde_json::json!({ "rpg": { "health": health }, "koin": 100 }));
            let t = text_of(&r);
            assert!(
                t.starts_with("Aduh kak, HP kamu sekarat"),
                "health {health}: {t}"
            );
            assert!(t.contains(&format!("sisa *{health} HP*")), "{t}");
            assert!(r.mutations().is_empty(), "health {health}");
        }
    }

    /// `health < 30` — ambangnya strict, jadi 30 tetap boleh petualang.
    #[test]
    fn the_threshold_is_exactly_thirty() {
        let mut went_through = false;
        for _ in 0..200 {
            let r = run(serde_json::json!({ "rpg": { "health": 30 }, "koin": 0 }));
            if !text_of(&r).starts_with("Aduh kak, HP kamu sekarat") {
                went_through = true;
                assert_eq!(r.mutations().len(), 1);
            }
        }
        assert!(went_through, "health 30 harus lolos ambang");
    }

    #[test]
    fn an_absent_health_reads_as_one_hundred_and_is_never_blocked() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "rpg": null })] {
            let r = run(user.clone());
            assert!(
                !text_of(&r).starts_with("Aduh kak, HP kamu sekarat"),
                "input {user}"
            );
        }
    }

    #[test]
    fn a_win_pays_coins_and_exp_and_leaves_health_alone() {
        let mut won = false;
        for _ in 0..400 {
            let r = run(serde_json::json!({ "rpg": { "health": 100 }, "koin": 500, "exp": 0 }));
            let t = text_of(&r);
            if !t.starts_with("🗡️ *PETUALANGAN BERHASIL!!*") {
                continue;
            }
            won = true;
            let data = write_of(&r);
            assert_eq!(data["rpg"], serde_json::json!({ "health": 100 }), "k menang tak boleh dikurangi HP");
            let gained_koin = data["koin"].as_i64().unwrap() - 500;
            let gained_exp = data["exp"].as_i64().unwrap();
            assert!((2_000..=11_999).contains(&gained_koin), "koin {gained_koin}");
            assert!((500..=2_499).contains(&gained_exp), "exp {gained_exp}");
            assert!(t.contains(&format!("+Rp {}", thousands_id(gained_koin as f64))), "{t}");
            assert!(t.contains(&format!("*+{}*", thousands_id(gained_exp as f64))), "{t}");
        }
        assert!(won, "dalam 400 percobaan harus ada yang menang");
    }

    #[test]
    fn a_loss_without_death_only_costs_hp_and_keeps_exp() {
        let mut lost = false;
        for _ in 0..400 {
            let r = run(serde_json::json!({ "rpg": { "health": 60 }, "koin": 500, "exp": 900 }));
            let t = text_of(&r);
            if !t.starts_with("☠️ *DISERGAP MONSTER!!*") {
                continue;
            }
            let data = write_of(&r);
            let left = data["rpg"]["health"].as_i64().unwrap();
            assert!((21..=50).contains(&left), "sisa HP {left}");
            lost = true;
            assert!(
                data.get("exp").is_none(),
                "exp hanya ditulis kalau mati: {data}"
            );
            assert!(t.contains("sempet kabur"), "{t}");
        }
        assert!(lost, "dalam 400 percobaan harus ada kalah tanpa mati");
    }

    #[test]
    fn dying_clamps_health_to_zero_and_halves_the_exp() {
        let mut died = false;
        for _ in 0..400 {
            let r = run(serde_json::json!({ "rpg": { "health": 30 }, "koin": 7, "exp": 901 }));
            let t = text_of(&r);
            if !t.contains("💀 *KAMU MATI!*") {
                continue;
            }
            died = true;
            let data = write_of(&r);
            assert_eq!(data["rpg"]["health"], serde_json::json!(0));
            assert_eq!(data["exp"], serde_json::json!(450), "Math.floor(901 / 2), bukan 451");
            assert!(data.get("koin").is_none(), "kalah tak menulis koin sama sekali");
        }
        assert!(died, "dalam 400 percobaan harus ada yang mati");
    }

    #[test]
    fn the_location_is_always_one_of_the_six() {
        for _ in 0..200 {
            let r = run(serde_json::json!({ "rpg": { "health": 100 } }));
            let t = text_of(&r);
            assert!(
                LOCATIONS
                    .iter()
                    .any(|loc| t.contains(&format!("📍 Lokasi: *{loc}*"))),
                "lokasi di luar tabel: {t}"
            );
        }
    }
}