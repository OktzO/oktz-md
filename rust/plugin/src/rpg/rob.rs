//! `plugins/rpg/rob.js` — rampok koin member grup lain.
//!
//! Yang tidak ikut ke port: preview `*Sssstttt...* Bersembunyi` sebelum jeda
//! `setTimeout`, dan kartu level-up dari `addExpWithLevelCheck` yang butuh
//! canvas + `sock.sendMedia` — EXP-nya sendiri tetap lewat
//! [`crate::rpg::jualan::apply_exp`].
//!
//! `db.getUser(target)` untuk member lain dibaca dari
//! [`crate::rpg::heal::other_user`], jadi target yang row-nya belum pernah ada
//! di store jatuh ke cabang "nggak ketemu".

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::heal::other_user;
use crate::rpg::jualan::{apply_exp, math_random, or_default, rpg_int, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

const SUCCESS_RATE: f64 = 0.4;
const MIN_VICTIM_KOIN: i64 = 1_000;
const MIN_ROBBER_HP: i64 = 30;
const EXP_GAIN: i64 = 300;
const HEALTH_LOSS: i64 = 25;

/// `m.mentionedJid?.[0] || m.quoted?.sender` — string kosong falsy, jadi
/// lompat ke kutipan.
fn target_of(ctx: &Ctx) -> Option<String> {
    ctx.mentioned
        .iter()
        .chain(ctx.quoted.as_ref().map(|q| &q.sender))
        .find(|jid| !jid.is_empty())
        .cloned()
}

/// `maxSteal = Math.floor(koin * 0.3)`, `stolen = Math.floor(Math.random() * maxSteal) + 1000`.
///
/// `Math.random()` ada di `[0, 1)`, jadi bagian acaknya di `0..maxSteal` dan
/// jumlah curian di `1000 ..= maxSteal + 999` — bukan sampai `maxSteal`.
/// `maxSteal` bisa 0 (korban miskin) — `roll(0)` akan panic, jadi nol
/// ditangani eksplisit seperti `Math.random() * 0` di JS.
fn stolen_of(victim_koin: i64) -> i64 {
    let max_steal = victim_koin * 3 / 10;
    let hasil = if max_steal > 0 {
        crate::common::roll(max_steal as u32) as i64
    } else {
        0
    };
    hasil + 1_000
}

#[plugin(
    name = "rob",
    alias = ["rampok", "mug"],
    category = "rpg",
    description = "Rampok uang player lain (berisiko)",
    usage = ".rob @user",
    example = ".rob @user",
    cooldown = 600,
    energi = 1,
    is_group = true,
    is_private = false
)]
pub fn rob(ctx: Ctx) -> Reply {
    let Some(target) = target_of(&ctx) else {
        return Reply::text(
            "Hayo, mau malak siapa nih? 🦹‍♂️🔪\nTag target yang mau dirampok hartanya!\n\
             Contoh: `.rob @user`",
        );
    };
    if target == ctx.sender {
        return Reply::text("Sakit jiwa lu? Masa ngerampok dompet sendiri! 😂❌");
    }

    let Some(victim) = other_user(&ctx, &target) else {
        return Reply::text(
            "Target buronanmu nggak ketemu di database! Kayaknya dia udah kabur duluan. 🏃💨",
        );
    };
    let victim_koin = victim.get("koin").and_then(Value::as_i64).unwrap_or(0);
    if victim_koin < MIN_VICTIM_KOIN {
        return Reply::text(
            "Yaelah, target lu miskin parah! Duitnya di bawah Rp 1.000, masa tega dirampok? \
             Cari mangsa yang tajir dong! 😤",
        );
    }

    let mut rpg = rpg_map(&ctx);
    let health = or_default(rpg_int(&rpg, "health"), 100);
    rpg.insert("health".into(), serde_json::json!(health));
    if health < MIN_ROBBER_HP {
        return Reply::text(format!(
            "Woy bos, badan lu tinggal tulang gitu masih nekat ngerampok?! 🤒\n\
             Minimal *30 HP*, darah lu cuma *{health} HP*. Berobat sana!"
        ));
    }

    let robber_koin = int_field(&ctx, "koin");
    let number = crate::common::jid_number(&target);

    if math_random() < SUCCESS_RATE {
        let stolen = stolen_of(victim_koin);
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, EXP_GAIN);
        return Reply::text_mention(
            format!(
                "MANTAP! TARGET BERHASIL DIPALAK! 🦹‍♂️💰\n\n\
                 Lu berhasil nakutin si @{number} sampai ngencing di celana!\n\
                 Uang hasil palakan: *+Rp {}*\n\
                 Bonus EXP Begal: *+{EXP_GAIN}*\n\n\
                 *Buru kabur sebelum polis dateng!!!* 🚓💨",
                thousands_id(stolen as f64)
            ),
            target.clone(),
        )
        .with_mutation(Mutation::SetUser {
            jid: target.clone(),
            data: serde_json::json!({ "koin": victim_koin - stolen }),
        })
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": robber_koin + stolen, "exp": new_exp, "rpg": rpg }),
        });
    }

    let fine = crate::common::roll(10_000) as i64 + 5_000;
    let actual_fine = fine.min(robber_koin);
    rpg.insert("health".into(), serde_json::json!((health - HEALTH_LOSS).max(0)));

    Reply::text(format!(
        "GOBLOK! KETAHUAN WARGA!! 🚨🤬\n\n\
         Bukannya dapet duit, lu malah ketangkep basah terus *digebukin warga 1 RT*!\n\
         💸 Duit lu disita RT: *-Rp {}*\n\
         🤕 Badan Babak Belur: *-{HEALTH_LOSS} HP*\n\n\
         *MAMPUS LU, makanya jangan main-main di mari!* 🤣",
        thousands_id(actual_fine as f64)
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": (robber_koin - actual_fine).max(0), "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, QuotedMessage};

    const SENDER: &str = "628111@s.whatsapp.net";
    const TARGET: &str = "628999@s.whatsapp.net";

    fn victim(koin: Option<i64>) -> Participant {
        Participant {
            jid: TARGET.into(),
            is_admin: false,
            is_owner: false,
            data: match koin {
                Some(k) => serde_json::json!({ "koin": k }),
                None => Value::Null,
            },
        }
    }

    fn run(robber: Value, mentioned: Vec<String>, target_row: Option<Participant>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rob".into(),
            sender: SENDER.into(),
            chat: "120363@g.userc.us".into(),
            mentioned,
            is_group: true,
            user_data: robber,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: target_row.into_iter().collect(),
                ..Default::default()
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("rob").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn user_of<'a>(r: &'a Reply, jid: &str) -> &'a serde_json::Value {
        r.mutations()
            .iter()
            .find_map(|m| match m {
                Mutation::SetUser { jid: j, data } if j == jid => Some(data),
                _ => None,
            })
            .unwrap_or_else(|| panic!("tidak ada SetUser untuk {jid}"))
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("rob").unwrap();
        assert_eq!(p.meta.name, "rob");
        assert_eq!(p.meta.alias, ["rampok", "mug"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Rampok uang player lain (berisiko)");
        assert_eq!(p.meta.usage, ".rob @user");
        assert_eq!(p.meta.example, ".rob @user");
        assert_eq!(p.meta.cooldown, 600);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["rampok", "mug"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "rob");
        }
    }

    #[test]
    fn without_a_target_it_asks_who_to_rob_and_writes_nothing() {
        for mentioned in [vec![], vec!["".to_string()]] {
            let r = run(serde_json::json!({ "koin": 0 }), mentioned, None);
            let t = text_of(&r);
            assert!(t.starts_with("Hayo, mau malak siapa nih?"), "{t}");
            assert!(t.contains("`.rob @user`"), "{t}");
            assert!(r.mutations().is_empty());
        }
    }

    /// `m.quoted?.sender` jadi target kalau tidak ada mention.
    #[test]
    fn a_quoted_message_can_supply_the_target() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rob".into(),
            sender: SENDER.into(),
            mentioned: vec![],
            quoted: Some(QuotedMessage {
                id: "x".into(),
                sender: TARGET.into(),
                chat: "120363@g.userc.us".into(),
                text: "h".into(),
                kind: oktz_plugin_api::QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("rob").unwrap().handler)(ctx);
        // Target ada tapi row-nya tidak → cabang "tidak ketemu", bukan help.
        assert!(
            text_of(&r).starts_with("Target buronanmu nggak ketemu di database!"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn robbing_yourself_is_refused() {
        let r = run(
            serde_json::json!({ "koin": 5_000 }),
            vec![SENDER.into()],
            Some(victim(Some(5_000))),
        );
        assert!(text_of(&r).starts_with("Sakit jiwa lu?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_target_without_a_stored_row_is_reported_as_missing() {
        let r = run(serde_json::json!({ "koin": 5_000 }), vec![TARGET.into()], Some(victim(None)));
        assert!(
            text_of(&r).starts_with("Target buronanmu nggak ketemu di database!"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    /// `(victim.koin || 0) < 1000` — tepat 1000 lolos, 999 ditolak.
    #[test]
    fn a_target_below_one_thousand_coins_is_refused() {
        for koin in [0, 1, 999] {
            let r = run(
                serde_json::json!({ "koin": 50_000, "rpg": { "health": 100 } }),
                vec![TARGET.into()],
                Some(victim(Some(koin))),
            );
            assert!(
                text_of(&r).starts_with("Yaelah, target lu miskin parah!"),
                "koin {koin}"
            );
            assert!(r.mutations().is_empty(), "koin {koin}");
        }
        assert_eq!(victim(Some(1_000)).data["koin"], serde_json::json!(1_000));
    }

    /// `robber.rpg.health = robber.rpg.health || 100` — HP 0 jadi 100 lagi.
    #[test]
    fn zero_health_is_read_back_as_one_hundred_before_the_gate() {
        let r = run(
            serde_json::json!({ "koin": 50_000, "rpg": { "health": 0 } }),
            vec![TARGET.into()],
            Some(victim(Some(50_000))),
        );
        // HP 0 berarti 100 → lolos gerbang, jadi bukan pesan "darah cuma *0 HP*".
        assert!(
            !text_of(&r).contains("darah lu cuma"),
            "{}",
            text_of(&r)
        );
        assert!(!r.mutations().is_empty());
    }

    #[test]
    fn below_thirty_health_stops_the_rob_before_any_roll() {
        for hp in [1, 29] {
            let r = run(
                serde_json::json!({ "koin": 50_000, "rpg": { "health": hp } }),
                vec![TARGET.into()],
                Some(victim(Some(50_000))),
            );
            let t = text_of(&r);
            assert!(t.starts_with("Woy bos, badan lu tinggal tulang"), "hp {hp}: {t}");
            assert!(t.contains(&format!("*{hp} HP*")), "hp {hp}: {t}");
            assert!(r.mutations().is_empty(), "hp {hp}: gerbang tidak menyimpan apa pun");
        }
    }

    /// `Math.floor(Math.random() * Math.floor(koin * 0.3)) + 1000` — curian di
    /// `1000 ..= floor(koin * 0.3) + 999`, jadi bisa sedikit di atas 30% dompet.
    #[test]
    fn a_successful_rob_moves_between_1000_and_thirty_percent_of_the_purse() {
        let koin = 500_000;
        let max_steal = koin * 3 / 10;
        let mut seen = 0;
        for _ in 0..4000 {
            let r = run(
                serde_json::json!({ "koin": 0, "exp": 0, "rpg": { "health": 100 } }),
                vec![TARGET.into()],
                Some(victim(Some(koin))),
            );
            let t = text_of(&r);
            let gained = user_of(&r, SENDER)["koin"].as_i64().unwrap();
            if t.starts_with("MANTAP!") {
                seen += 1;
                assert!(
                    (1_000..=max_steal + 999).contains(&gained),
                    "curian {gained} di luar 1000..={}",
                    max_steal + 999
                );
                assert_eq!(user_of(&r, TARGET)["koin"], serde_json::json!(koin - gained));
                assert_eq!(user_of(&r, SENDER)["exp"], serde_json::json!(EXP_GAIN));
                assert_eq!(user_of(&r, SENDER)["rpg"]["health"], serde_json::json!(100));
                assert!(t.contains("Bonus EXP Begal: *+300*"), "{t}");
            } else {
                assert!(t.starts_with("GOBLOK! KETAHUAN WARGA!!"), "{t}");
                // Denda 5.000..14.999, dipotong koin zero → tetap zero.
                assert_eq!(gained, 0, "koin 0 jadiRp 0");
                assert_eq!(user_of(&r, SENDER)["rpg"]["health"], serde_json::json!(75));
                assert!(r.mutations().iter().all(|m| !matches!(
                    m, Mutation::SetUser { jid, .. } if jid == TARGET
                )), "kegagalan tidak menyentuh koin target");
            }
        }
        assert!(seen > 0, "cabang berhasil harus muncul");
    }

    /// `m.reply(txt, { mentions: [target] })` — mention-nya JID penuh target,
    /// dan sender TIDAK ikut (JS override `mentionedJid`).
    #[test]
    fn the_success_reply_names_the_target_number_and_mentions_only_them() {
        let mut checked = false;
        for _ in 0..4000 {
            let r = run(
                serde_json::json!({ "koin": 0, "rpg": { "health": 100 } }),
                vec![TARGET.into()],
                Some(victim(Some(500_000))),
            );
            if !text_of(&r).starts_with("MANTAP!") {
                continue;
            }
            assert!(text_of(&r).contains("si @628999 sampai"), "{}", text_of(&r));
            match r.without_mutations() {
                Reply::Text { mentions, .. } => assert_eq!(mentions, &[TARGET.to_string()]),
                other => panic!("harus Text, dapat {other:?}"),
            }
            checked = true;
            break;
        }
        assert!(checked, "cabang berhasil harus muncul");
    }

    /// `Math.floor(Math.random() * 10000) + 5000` lalu `Math.min(fine, koin)`
    /// dan `Math.max(0, koin - fine)`.
    #[test]
    fn a_caught_rob_fines_five_to_fifteen_thousand_capped_by_the_purse() {
        for purse in [0, 3_000, 20_000] {
            let mut fined = 0;
            for _ in 0..3000 {
                let r = run(
                    serde_json::json!({ "koin": purse, "rpg": { "health": 100 } }),
                    vec![TARGET.into()],
                    Some(victim(Some(500_000))),
                );
                if !text_of(&r).starts_with("GOBLOK!") {
                    continue;
                }
                fined += 1;
                let left = user_of(&r, SENDER)["koin"].as_i64().unwrap();
                let lost = purse - left;
                if purse < 5_000 {
                    assert_eq!(lost, purse, "dompet {purse} habis, tidak jadi negatif");
                } else {
                    assert!((5_000..=14_999).contains(&lost), "denda {lost} di luar 5.000..14.999");
                }
                assert_eq!(user_of(&r, SENDER)["rpg"]["health"], serde_json::json!(75));
            }
            assert!(fined > 0, "dompet {purse} harus pernah ketahuan");
        }
    }
}
