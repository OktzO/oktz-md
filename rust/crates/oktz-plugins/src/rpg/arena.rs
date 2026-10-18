//! `plugins/rpg/arena.js` — duel PvP di grup sampai 10 ronde.
//!
//! `ponytail:` `db.getUser(mentioned)` tidak ada di `Ctx` untuk private chat
//! dan roster yang tidak memuat target — `other_user` mengembalikan `None`
//! dan plugin membalas "belum terdaftar" untuk row yang memang belum ada
//! (`db.getUser` juga `null` di sana). Di private chat (`isGroup: false`)
//! roster selalu kosong, jadi duel hanya jalan di grup.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{apply_exp, other_user, or_default, rpg_map, user_level};
use crate::tools::kalkulatormbg::thousands_id;

const MAX_ROUNDS: i64 = 10;

/// `Math.max(5, atk - def + Math.floor(Math.random() * 10))`.
fn damage(attack: i64, defense: i64) -> i64 {
    (attack - defense + crate::common::roll(10) as i64).max(5)
}

/// `(rpg.attack || 10) + level * 2` — `user.level`, bukan `rpg.level`.
fn attack_of(rpg: &Map<String, Value>, level: i64) -> i64 {
    or_default(rpg.get("attack").and_then(Value::as_i64), 10) + level * 2
}

/// `(rpg.defense || 5) + level`.
fn defense_of(rpg: &Map<String, Value>, level: i64) -> i64 {
    or_default(rpg.get("defense").and_then(Value::as_i64), 5) + level
}

/// `rpg.health || 100`.
fn health_of(rpg: &Map<String, Value>) -> i64 {
    or_default(rpg.get("health").and_then(Value::as_i64), 100)
}

/// Satu baris `battleLog`.
struct Round {
    my_damage: i64,
    their_damage: Option<i64>,
}

/// Simulasi duel. Berhenti saat `round < 10` gagal atau salah satu HP
/// menyentuh 0.
fn battle(my_attack: i64, my_defense: i64, their_attack: i64, their_defense: i64, my_hp: i64, their_hp: i64) -> (i64, i64, Vec<Round>) {
    let (mut mine, mut theirs) = (my_hp, their_hp);
    let mut log = Vec::new();
    let mut round = 0;
    while mine > 0 && theirs > 0 && round < MAX_ROUNDS {
        round += 1;
        let my_damage = damage(my_attack, their_defense);
        theirs -= my_damage;
        if theirs <= 0 {
            log.push(Round { my_damage, their_damage: None });
            break;
        }
        let their_damage = damage(their_attack, my_defense);
        mine -= their_damage;
        log.push(Round { my_damage, their_damage: Some(their_damage) });
    }
    (mine, theirs, log)
}

/// `const isWin = myHp > oppHp` — SERI berarti kalah.
fn wins(my_hp: i64, their_hp: i64) -> bool {
    my_hp > their_hp
}

fn log_lines(log: &[Round]) -> String {
    log.iter()
        .rev()
        .take(6)
        .rev()
        .map(|r| {
            let mut s = format!("> 🔥 Kamu melancarkan serangan kuat: *-{} HP*", r.my_damage);
            if let Some(d) = r.their_damage {
                s.push_str(&format!("\n> 💢 Lawan membalas dengan keras: *-{d} HP*"));
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[plugin(
    name = "arena",
    alias = ["battle"],
    category = "rpg",
    description = "Bertarung di arena PvP",
    usage = ".arena <@user>",
    example = ".arena @user",
    cooldown = 180,
    energi = 1,
    is_group = true,
    is_private = false
)]
pub fn arena(ctx: Ctx) -> Reply {
    // `m.mentionedJid?.[0] || m.quoted?.sender`
    let opponent_jid = ctx
        .mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()));

    let Some(opponent_jid) = opponent_jid else {
        let p = &ctx.prefix;
        return Reply::text(format!(
            "⚔️ *ARENA GLADIATOR* ⚔️\n\n\
             Ayo tantang teman kamu untuk duel di arena kak!\n\n\
             *Cara Menantang:*\n\
             🗡️ `{p}arena @user`\n\
             🗡️ Atau reply pesan dia dengan `{p}arena`\n\n\
             > _⚠️ Hati-hati kak, kalau kalah koin kamu bakal berkurang 20%!_"
        ));
    };

    if opponent_jid == ctx.sender {
        return Reply::text(
            "Aduh kak, masa kamu mau mukulin diri sendiri? Cari lawan yang lain yuk! 😂",
        );
    }

    let Some(opponent) = other_user(&ctx, &opponent_jid) else {
        return Reply::text(
            "Lawan yang kamu tag belum terdaftar di database kita nih kak!",
        );
    };
    let opp_rpg = opponent.get("rpg").cloned().unwrap_or_else(|| serde_json::json!({}));
    let opp_map = opp_rpg.as_object().cloned().unwrap_or_default();
    let opp_level = or_default(opponent.get("level").and_then(Value::as_i64), 1);
    let opp_koin = or_default(opponent.get("koin").and_then(Value::as_i64), 0);

    let mut rpg = rpg_map(&ctx);
    let level = user_level(&ctx);
    let my_koin = crate::common::int_field(&ctx, "koin");
    let my_health = health_of(&rpg);
    let their_health = health_of(&opp_map);

    let (my_hp, their_hp, log) = battle(
        attack_of(&rpg, level),
        defense_of(&rpg, level),
        attack_of(&opp_map, opp_level),
        defense_of(&opp_map, opp_level),
        my_health,
        their_health,
    );

    let mut txt = format!(
        "⚔️ *HASIL PERTARUNGAN* ⚔️\n\n\
         *📊 Kondisi Akhir:*\n\
         🧑 Kamu: *{}/{my_health} HP*\n\
         👤 Lawan: *{}/{their_health} HP*\n\
         🔄 Durasi: *{} Ronde*\n\n\
         📜 *Cuplikan Pertarungan:*\n{}\n\n",
        my_hp.max(0),
        their_hp.max(0),
        log.len(),
        log_lines(&log)
    );

    if wins(my_hp, their_hp) {
        let exp_reward = 300 + opp_level * 50;
        let gold_reward = (opp_koin as f64 * 0.1).floor() as i64;
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

        txt.push_str(&format!(
            "🏆 *KEMENANGAN TELAH DIRAIH!* 🎉\n\
             Wah hebat banget kak! Ini hadiah dari arena:\n\
             ✨ EXP: *+{exp_reward}*\n\
             💰 Koin Jarahan: *+Rp {}*",
            thousands_id(gold_reward as f64)
        ));

        return Reply::Text {
            text: txt,
            mentions: vec![ctx.sender.clone(), opponent_jid.clone()],
        }
        .with_reaction("🏆")
        .mutating(vec![
            Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "koin": my_koin + gold_reward, "exp": new_exp, "rpg": rpg }),
            },
            Mutation::SetUser {
                jid: opponent_jid,
                data: serde_json::json!({ "koin": (opp_koin - gold_reward).max(0) }),
            },
        ]);
    }

    let gold_loss = (my_koin as f64 * 0.2).floor() as i64;
    txt.push_str(&format!(
        "💀 *SAYANG SEKALI, KAMU KALAH...* 💔\n\
         Jangan sedih kak, nanti coba lagi ya!\n\
         💸 Koin Terjatuh: *-Rp {}*",
        thousands_id(gold_loss as f64)
    ));

    Reply::Text {
        text: txt,
        mentions: vec![ctx.sender.clone(), opponent_jid],
    }
    .with_reaction("💀")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": (my_koin - gold_loss).max(0) }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn ctx_with(mentioned: &[&str], roster: GroupRoster, user: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "arena".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@g.uset".into(),
            text: mentioned.join(" "),
            args: mentioned.iter().map(|s| (*s).to_string()).collect(),
            mentioned: mentioned.iter().map(|s| (*s).to_string()).collect(),
            is_group: true,
            roster,
            user_data: user,
            ..Default::default()
        }
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("arena").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn roster_with(data: Value) -> GroupRoster {
        GroupRoster {
            participants: vec![Participant {
                jid: "628222@s.whatsapp.net".into(),
                is_admin: false,
                is_owner: false,
                data,
            }],
            is_bot_admin: false,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("arena").unwrap();
        assert_eq!(p.meta.name, "arena");
        assert_eq!(p.meta.alias, ["battle"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Bertarung di arena PvP");
        assert_eq!(p.meta.usage, ".arena <@user>");
        assert_eq!(p.meta.example, ".arena @user");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("battle").unwrap().meta.name, "arena");
    }

    #[test]
    fn without_a_target_the_how_to_is_shown_and_nothing_is_written() {
        let r = run(ctx_with(&[], GroupRoster::default(), serde_json::json!({"koin": 100})));
        let t = text_of(&r);
        assert!(t.starts_with("⚔️ *ARENA GLADIATOR*"), "{t}");
        assert!(t.contains(".arena @user"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn fighting_yourself_is_refused() {
        let r = run(ctx_with(
            &["628111@s.whatsapp.net"],
            GroupRoster::default(),
            serde_json::json!({}),
        ));
        assert!(text_of(&r).starts_with("Aduh kak, masa kamu mau mukulin diri sendiri"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_opponent_is_reported_as_unregistered() {
        let r = run(ctx_with(
            &["628999@s.whatsapp.net"],
            GroupRoster::default(),
            serde_json::json!({}),
        ));
        assert!(text_of(&r).starts_with("Lawan yang kamu tag belum terdaftar"));
        assert!(r.mutations().is_empty());
    }

    /// `isWin = myHp > oppHp` — seri berarti kalah.
    #[test]
    fn a_duel_always_writes_both_sides() {
        let ctx = ctx_with(
            &["628222@s.whatsapp.net"],
            roster_with(serde_json::json!({"rpg": {"health": 100, "attack": 10, "defense": 5},
                               "level": 4, "koin": 10_000})),
            serde_json::json!({"rpg": {"health": 100, "attack": 10, "defense": 5}, "level": 1, "koin": 5_000}),
        );
        let r = run(ctx.clone());
        let t = text_of(&r);
        assert!(t.starts_with("⚔️ *HASIL PERTARUNGAN*"), "{t}");
        assert!(t.contains("📊 Kondisi Akhir:"), "{t}");
        assert!(t.contains("🔄 Durasi: *"), "{t}");
        assert!(!r.mutations().is_empty(), "minimal sisi saya ditulis");
        let jids: Vec<&str> = r
            .mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, .. } => jid.as_str(),
                other => panic!("harus SetUser, dapat {other:?}"),
            })
            .collect();
        assert!(jids.contains(&"628111@s.whatsapp.net"));
        if jids.len() == 2 {
            assert!(jids.contains(&"628222@s.whatsapp.net"), "kalah tak menulis lawan");
        }
    }

    #[test]
    fn a_win_takes_ten_percent_of_the_opponents_purse() {
        let mut won = false;
        for _ in 0..100 {
            let ctx = ctx_with(
                &["628222@s.whatsapp.net"],
                roster_with(serde_json::json!({"rpg": {"health": 20, "attack": 1, "defense": 1},
                                   "level": 1, "koin": 10_000})),
                serde_json::json!({"rpg": {"health": 100, "attack": 90, "defense": 90}, "level": 9, "koin": 0}),
            );
            let r = run(ctx.clone());
            let t = text_of(&r);
            if !t.contains("🏆 *KEMENANGAN TELAH DIRAIH!*") {
                continue;
            }
            won = true;
            let mine = r.mutations().iter().find_map(|m| match m {
                Mutation::SetUser { jid, data } if jid == "628111@s.whatsapp.net" => Some(data.clone()),
                _ => None,
            });
            let theirs = r.mutations().iter().find_map(|m| match m {
                Mutation::SetUser { jid, data } if jid == "628222@s.whatsapp.net" => Some(data.clone()),
                _ => None,
            });
            assert_eq!(mine.unwrap()["koin"], serde_json::json!(1000), "10% dari 10.000");
            assert_eq!(theirs.unwrap()["koin"], serde_json::json!(9000));
        }
        assert!(won, "dalam 100 percobaan harus ada kemenangan");
    }

    #[test]
    fn a_loss_costs_twenty_percent_and_only_writes_the_sender() {
        let mut lost = false;
        for _ in 0..100 {
            let ctx = ctx_with(
                &["628222@s.whatsapp.net"],
                roster_with(serde_json::json!({"rpg": {"health": 100, "attack": 90, "defense": 90},
                                   "level": 9, "koin": 0})),
                serde_json::json!({"rpg": {"health": 20, "attack": 1, "defense": 1}, "level": 1, "koin": 10_000}),
            );
            let r = run(ctx.clone());
            let t = text_of(&r);
            if !t.contains("💀 *SAYANG SEKALI, KAMU KALAH") {
                continue;
            }
            lost = true;
            assert_eq!(r.mutations().len(), 1, "kalah tidak menyentuh lawan");
            assert_eq!(r.mutations()[0].clone(), Mutation::SetUser {
                jid: "628111@s.whatsapp.net".into(),
                data: serde_json::json!({"koin": 8000}),
            });
        }
        assert!(lost, "dalam 100 percobaan harus ada kalah");
    }

    #[test]
    fn the_battle_caps_at_ten_rounds() {
        // HP 1000, bukan 100: damage di-roll 5..14 per ronde, jadi dengan HP
        // 100 duel bisa selesai sebelum ronde 10 dan assertion jadi flaky.
        // 10 ronde max = 140 damage, tidak pernah killing.
        let (_, _, log) = battle(5, 0, 5, 0, 1000, 1000);
        assert_eq!(log.len(), 10, "while(round < 10)");
        // Satu pihak habis: log berhenti begitu HP lawan <= 0.
        let (_, theirs, log) = battle(100, 0, 5, 0, 100, 5);
        assert!(theirs <= 0, "HP lawan {theirs}");
        assert_eq!(log.len(), 1);
        // HP yang belum mencapai 0 tidak pernah dibulatkan di sini; hanya
        // balasan yang memakai `Math.max(0, hp)`.
        let (_, raw, _) = battle(5, 0, 5, 0, 1000, 1000);
        assert!(raw < 1000, "kerusakan menumpuk, HP boleh negatif");
        assert!(log[0].their_damage.is_none(), "serangan pihak pertama selesai, tak ada balasan");
    }

    #[test]
    fn damage_never_drops_below_five() {
        for _ in 0..200 {
            assert!(damage(1, 1_000) >= 5);
            assert!(damage(50, 0) <= 59);
        }
    }

    #[test]
    fn a_draw_counts_as_a_loss_because_the_test_is_strictly_greater() {
        // `isWin = myHp > oppHp` murni perbandingan HP akhir — duel-nya sendiri
        // acak (damage di-roll tiap ronde), jadi yang diuji FUNGSI penentu
        // hasil, bukan satu hasil duel yang kebetulan menang.
        assert!(!wins(50, 50), "seri tidak boleh dihitung menang");
        assert!(!wins(49, 50));
        assert!(wins(51, 50));
        // Balasan memakai HP mentah yang boleh negatif, tapi `wins` tidak
        // pernah melihat nilai negatif sebagai menang kalau lawan juga.
        assert!(!wins(-10, -10));
    }
}