//! `plugins/rpg/enchant.js` — naikkan level enchant sebuah barang.
//!
//! Kursor: biaya naik per level (`cost * (currentLevel + 1)`) dan tingkat
//! keberhasilan turun 5 poin per level dengan lantai 20%. Uang hilang baik
//! berhasil atau gagal — `user.koin -= cost` jalan SEBELUM lemparan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::rpg::heal::{apply_exp, inventory_map, math_random, or_default, rpg_map};

/// `(key, name, stat, bonus, cost, successRate)` — urutan object literal =
/// urutan tampil.
type Enchantable = (
    &'static str,
    &'static str,
    &'static str,
    i64,
    i64,
    i64,
);

const ENCHANTABLE: [Enchantable; 7] = [
    ("sword", "⚔️ Pedang", "attack", 5, 500, 70),
    ("shield", "🛡️ Perisai", "defense", 4, 500, 70),
    ("armor", "🦺 Armor", "health", 20, 800, 60),
    ("helmet", "⛑️ Helm", "defense", 3, 400, 75),
    ("bow", "🏹 Busur", "attack", 4, 450, 72),
    ("goldsword", "🗡️ Pedang Emas", "attack", 10, 2_000, 50),
    ("diamondarmor", "💎 Armor Berlian", "health", 50, 5_000, 40),
];

const MAX_LEVEL: i64 = 10;
const SUCCESS_EXP: i64 = 150;

fn item(key: &str) -> Option<&'static Enchantable> {
    ENCHANTABLE.iter().find(|(k, ..)| *k == key)
}

fn enchant_level(rpg: &Map<String, Value>, key: &str) -> i64 {
    rpg.get("enchants")
        .and_then(|e| e.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

/// `Math.max(20, successRate - currentLevel * 5)` — lantai 20%.
fn adjusted_rate(rate: i64, level: i64) -> i64 {
    (rate - level * 5).max(20)
}

/// `Math.random() * 100 < adjustedRate` — sumber acak tunggal plugin ini.
/// `rate` dalam persen, `roll` dalam 0..1. `<` ketat, sama seperti JS.
fn roll_succeeds(rate: i64, roll: f64) -> bool {
    roll * 100.0 < rate as f64
}

fn book(rpg: &Map<String, Value>) -> String {
    let mut txt = String::from("✨ *ᴇɴᴄʜᴀɴᴛ - ᴅᴘɢʀᴀᴅᴇ ᴇǫᴜɪᴘ*\n\n> Tingkatkan equipment untuk bonus stats!\n\n*📦 *ɪᴛᴇᴍ:*\n\n");
    for (key, name, stat, bonus, cost, rate) in ENCHANTABLE {
        let level = enchant_level(rpg, key);
        txt.push_str(&format!(
            "> {name}\n\
             > 📊 Level: {level}/{MAX_LEVEL}\n\
             > 💪 Bonus: +{bonus} {stat}\n\
             > 💰 Cost: {}\n\
             > 🎯 Rate: {rate}%\n\
             > → `{key}`\n> \n",
            crate::tools::kalkulatormbg::thousands_id(cost as f64)
        ));
    }
    txt
}

#[plugin(
    name = "enchant",
    alias = ["upgrade", "tingkatkan"],
    category = "rpg",
    description = "Upgrade equipment dengan enchantment",
    usage = ".enchant <item>",
    example = ".enchant sword",
    cooldown = 120,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn enchant(ctx: Ctx) -> Reply {
    let inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let Some(key) = crate::common::first_keyword(&ctx) else {
        return Reply::text(book(&rpg));
    };

    let Some(&(_, name, stat, bonus, base_cost, rate)) = item(&key) else {
        return Reply::text(format!(
            "❌ Item tidak bisa di-enchant!\n\n> Ketik `{}enchant` untuk melihat daftar.",
            ctx.prefix
        ));
    };

    let owned = or_default(inv.get(&key).and_then(Value::as_i64), 0);
    if owned < 1 {
        return Reply::text(format!("❌ Kamu tidak punya {name}!"));
    }

    let level = enchant_level(&rpg, &key);
    if level >= MAX_LEVEL {
        return Reply::text(format!("❌ {name} sudah level MAX ({MAX_LEVEL})!"));
    }

    let cost = base_cost * (level + 1);
    let koin = crate::common::int_field(&ctx, "koin");
    if koin < cost {
        return Reply::text(format!(
            "❌ *ʙᴀʟᴀɴᴄᴇ ᴋᴜʀᴀɴɢ*\n\n> Butuh: {}\n> Balance: {}",
            crate::tools::kalkulatormbg::thousands_id(cost as f64),
            crate::tools::kalkulatormbg::thousands_id(koin as f64)
        ));
    }

    let cost_text = crate::tools::kalkulatormbg::thousands_id(cost as f64);
    let rate_now = adjusted_rate(rate, level);

    if roll_succeeds(rate_now, math_random()) {
        let mut enchants = rpg
            .get("enchants")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        enchants.insert(key.clone(), json!(level + 1));
        rpg.insert("enchants".into(), Value::Object(enchants));
        // `user.rpg[item.stat] = (user.rpg[item.stat] || 0) + item.bonus`
        let cur = or_default(rpg.get(stat).and_then(Value::as_i64), 0);
        rpg.insert(stat.to_string(), json!(cur + bonus));

        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, SUCCESS_EXP);

        return Reply::text(format!(
            "🎉 *ᴇɴᴄʜᴀɴᴛ ʙᴇᴄʜᴀsɪʟ!*\n\n*✨ *ʀᴇsᴜʟᴛ:*
\n\
             > 📦 Item: *{name}*\n\
             > 📊 Level: *{level} → {}*\n\
             > 💪 Bonus: *+{bonus} {stat}*\n\
             > 💰 Cost: *-{cost_text}*\n\
             > ✨ EXP: *+{SUCCESS_EXP}*\n",
            level + 1
        ))
        .with_reaction("🎉")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin - cost, "exp": new_exp, "rpg": rpg }),
        });
    }

    Reply::text(format!(
        "💔 *ᴇɴᴄʜᴀɴᴛ ɢᴀɢᴀʟ!*\n\n*😢 *ʀᴇsᴜʟᴛ:*
\n\
         > 📦 Item: *{name}*\n\
         > 📊 Level: *{level}* (tidak naik)\n\
         > 💰 Cost: *-{cost_text}* (hangus)\n\
         \n\n\
         💡 *Tips:* Coba lagi! Rate: {rate_now}%"
    ))
    .with_reaction("💔")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin - cost }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "enchant".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("enchant").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("enchant").unwrap();
        assert_eq!(p.meta.name, "enchant");
        assert_eq!(p.meta.alias, ["upgrade", "tingkatkan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Upgrade equipment dengan enchantment");
        assert_eq!(p.meta.usage, ".enchant <item>");
        assert_eq!(p.meta.example, ".enchant sword");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["upgrade", "tingkatkan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "enchant");
        }
    }

    #[test]
    fn without_an_argument_the_catalogue_lists_every_row_with_its_level() {
        let t_reply = run(json!({ "rpg": { "enchants": { "sword": 3 } } }), "");
        let t = text_of(&t_reply);
        assert!(t.starts_with("✨ *ᴇɴᴄʜᴀɴᴛ - ᴅᴘɢʀᴀᴅᴇ ᴇǫᴜɪᴘ*"), "{t}");
        for (key, name, stat, bonus, cost, rate) in ENCHANTABLE {
            assert!(t.contains(&format!("→ `{key}`")), "{key} hilang");
            assert!(t.contains(&format!("> {name}\n")), "{name} hilang");
            assert!(t.contains(&format!("💪 Bonus: +{bonus} {stat}")), "{key} salah");
            assert!(t.contains(&format!("💰 Cost: {}", thousands(cost))), "{key} salah");
            assert!(t.contains(&format!("🎯 Rate: {rate}%")), "{key} salah");
        }
        assert!(t.contains("📊 Level: 3/10\n"), "sword: {t}");
        assert!(t.contains("📊 Level: 0/10\n"), "bow: {t}");
    }

    fn thousands(n: i64) -> String {
        crate::tools::kalkulatormbg::thousands_id(n as f64)
    }

    #[test]
    fn an_unknown_item_is_refused() {
        let r = run(json!({ "inventory": { "sword": 1 }, "koin": 99_999 }), "potion");
        let t = text_of(&r);
        assert!(t.starts_with("❌ Item tidak bisa di-enchant!"), "{t}");
        assert!(t.contains("`.enchant` untuk melihat daftar"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_item_has_to_be_owned_first() {
        for owned in [0, -1] {
            let r = run(json!({ "inventory": { "sword": owned }, "koin": 99_999 }), "sword");
            assert!(text_of(&r).starts_with("❌ Kamu tidak punya ⚔️ Pedang!"), "owned {owned}");
            assert!(r.mutations().is_empty(), "owned {owned}");
        }
    }

    #[test]
    fn a_maxed_item_cannot_be_enchanted_again() {
        let r = run(
            json!({ "inventory": { "sword": 1 }, "rpg": { "enchants": { "sword": 10 } },
                    "koin": 99_999 }),
            "sword",
        );
        assert!(text_of(&r).starts_with("❌ ⚔️ Pedang sudah level MAX (10)!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_cost_scales_with_the_current_level_and_blocks_an_empty_purse() {
        // level 2 -> cost = 500 * 3 = 1.500.
        let r = run(
            json!({ "inventory": { "sword": 1 }, "rpg": { "enchants": { "sword": 2 } },
                    "koin": 1_499 }),
            "sword",
        );
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ʙᴀʟᴀɴᴄᴇ ᴋᴜʀᴀɴɢ*"), "{t}");
        assert!(t.contains("> Butuh: 1.500"), "{t}");
        assert!(t.contains("> Balance: 1.499"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_success_raises_the_level_the_stat_and_pays_exp() {
        let mut won = 0;
        for _ in 0..600 {
            let r = run(
                json!({ "inventory": { "sword": 1 }, "rpg": { "enchants": { "sword": 0 },
                                                                 "attack": 10 },
                        "koin": 10_000, "exp": 0 }),
                "sword",
            );
            let data = write_of(&r);
            // Cabang sukses satu-satunya yang menulis `rpg`; gagal cuma memotong
            // koin. Membaca bentuk write, bukan dekorasi small-caps yang beda
            // bentuk normalisasi Unicode-nya antar file.
            if data.get("rpg").is_none() {
                continue;
            }
            won += 1;
            let t = text_of(&r);
            assert_eq!(data["koin"], json!(9_500), "500 * (0 + 1)");
            assert_eq!(data["exp"], json!(150));
            assert_eq!(data["rpg"]["enchants"]["sword"], json!(1));
            assert_eq!(data["rpg"]["attack"], json!(15), "10 + bonus 5");
            assert!(t.contains("📊 Level: *0 → 1*"), "{t}");
            assert!(t.contains("💰 Cost: *-500*"), "{t}");
            assert!(t.contains("✨ EXP: *+150*"), "{t}");
        }
        assert!(won > 0, "dalam 600 percobaan harus ada setidaknya satu sukses");
    }

    /// Batas lemparannya deterministik: `<` ketat, jadi roll tepat sama dengan
    /// rate/100 masih gagal.
    #[test]
    fn the_roll_is_strictly_below_the_rate() {
        assert!(roll_succeeds(70, 0.0));
        assert!(roll_succeeds(70, 0.699_999));
        assert!(!roll_succeeds(70, 0.7), "70 < 70 false");
        assert!(!roll_succeeds(70, 0.9));
        assert!(roll_succeeds(20, 0.199_999), "lantai 20% masih bisa kena");
        assert!(!roll_succeeds(20, 0.2));
    }

    #[test]
    fn a_failure_still_costs_the_money_but_leaves_the_item_alone() {
        let mut lost = 0;
        for _ in 0..600 {
            let r = run(
                json!({ "inventory": { "sword": 1 }, "rpg": { "enchants": { "sword": 9 },
                                                                 "attack": 10 },
                        "koin": 10_000, "exp": 0 }),
                "sword",
            );
            let t = text_of(&r);
            if !t.starts_with("💔 *ᴇɴᴄʜᴀɴᴛ ɢᴀɢᴀʟ!*") {
                continue;
            }
            lost += 1;
            let data = write_of(&r);
            // level 9 -> cost 500 * 10 = 5.000, rate max(20, 70 - 45) = 25%.
            assert_eq!(data["koin"], json!(5_000));
            assert!(data.get("exp").is_none(), "gagal tak dapat exp");
            assert!(data.get("rpg").is_none(), "gagal tak menyentuh rpg");
            assert!(t.contains("Rate: 25%"), "{t}");
        }
        assert!(lost > 0, "dalam 600 percobaan harus ada setidaknya satu gagal");
    }

    /// `Math.max(20, rate - level * 5)` — tidak pernah di bawah 20.
    #[test]
    fn the_success_rate_never_falls_below_twenty_percent() {
        assert_eq!(adjusted_rate(70, 0), 70);
        assert_eq!(adjusted_rate(70, 5), 45);
        assert_eq!(adjusted_rate(70, 9), 25);
        assert_eq!(adjusted_rate(70, 10), 20);
        assert_eq!(adjusted_rate(70, 99), 20);
        assert_eq!(adjusted_rate(40, 50), 20);
    }

    #[test]
    fn other_enchant_levels_are_left_untouched_by_a_failed_roll() {
        let r = run(
            json!({ "inventory": { "sword": 1 }, "rpg": { "enchants": { "bow": 4 } },
                    "koin": 10_000 }),
            "sword",
        );
        let t = text_of(&r);
        if t.starts_with("💔") {
            assert!(write_of(&r).get("rpg").is_none(), "gagal tak boleh menulis rpg");
        } else {
            assert_eq!(write_of(&r)["rpg"]["enchants"]["bow"], json!(4), "level bow tak boleh hilang");
        }
    }
}