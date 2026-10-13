//! `plugins/rpg/use.js` — pakai item consumable atau buka crate.
//!
//! Semua setter yang menginisialisasi row pakai `||` (nol ikut di-backfill ke
//! default), bukan `??` — jadi `rpg.health = 0` jadi 100 sebelum pemakaian. Ada
//! dua jalur exp di sini: `db.updateExp` (crate, mysterybox, scroll) yang TIDAK
//! mengisi stats `rpg`, dan jalur tanpa exp sama sekali untuk potion biasa.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{exp_gain_only, inventory_map, or_default, roll_excl, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

/// Bar `rpg` setelah baris inisialisasi `use.js:37-42`.
struct Stats {
    health: i64,
    max_health: i64,
    mana: i64,
    max_mana: i64,
    stamina: i64,
    max_stamina: i64,
    attack: Option<i64>,
}

impl Stats {
    fn read(rpg: &Map<String, Value>) -> Self {
        let g = |k: &str| rpg.get(k).and_then(Value::as_i64);
        Self {
            health: or_default(g("health"), 100),
            max_health: or_default(g("maxHealth"), 100),
            mana: or_default(g("mana"), 100),
            max_mana: or_default(g("maxMana"), 100),
            stamina: or_default(g("stamina"), 100),
            max_stamina: or_default(g("maxStamina"), 100),
            attack: g("attack"),
        }
    }

    fn write(&self, rpg: &mut Map<String, Value>) {
        rpg.insert("health".into(), json!(self.health));
        rpg.insert("maxHealth".into(), json!(self.max_health));
        rpg.insert("mana".into(), json!(self.mana));
        rpg.insert("maxMana".into(), json!(self.max_mana));
        rpg.insert("stamina".into(), json!(self.stamina));
        rpg.insert("maxStamina".into(), json!(self.max_stamina));
    }
}

/// `user.inventory[itemKey]--` dan BALIKDeletes kalau sudah nol.
fn consume(inv: &mut Map<String, Value>, key: &str) {
    let left = inv.get(key).and_then(Value::as_i64).unwrap_or(0) - 1;
    if left <= 0 {
        inv.remove(key);
    } else {
        inv.insert(key.to_string(), json!(left));
    }
}

fn help() -> Reply {
    Reply::text(
        "🎒 *ᴜsᴇ ɪᴛᴇᴍ*\n\n*📋 *ᴜsᴀɢᴇ:*
\n> > `.use <nama_item>`\n> > Cek inventory: `.inventory`\n",
    )
}

#[plugin(
    name = "use",
    alias = ["pake", "makan"],
    category = "rpg",
    description = "Menggunakan item consumable atau membuka crate",
    usage = ".use <item>",
    example = ".use potion",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn use_item(ctx: Ctx) -> Reply {
    let Some(item_key) = ctx.args.first().map(|a| a.to_lowercase()) else {
        return help();
    };

    let mut inv = inventory_map(&ctx);
    let count = inv.get(item_key.as_str()).and_then(Value::as_i64).unwrap_or(0);
    if count <= 0 {
        return Reply::text(format!(
            "❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴀᴅᴀ*\n\n\
             > Kamu tidak memiliki item *{item_key}*!\n\
             > Cek inventory: `.inventory`"
        ));
    }

    let mut rpg = rpg_map(&ctx);
    let mut s = Stats::read(&rpg);
    let koin = int_field(&ctx, "koin");
    let mut koin_gain = 0i64;
    let mut exp_gain = 0i64;
    let mut loot: Option<(&str, i64)> = None;

    let full_health = || {
        Reply::text("❤️ *ʜᴇᴀʟᴛʜ ᴘᴇɴᴜʜ*\n\n> Nyawa kamu sudah penuh!")
    };

    let msg = match item_key.as_str() {
        "potion" => {
            if s.health >= s.max_health {
                return full_health();
            }
            s.health = (s.health + 50).min(s.max_health);
            consume(&mut inv, &item_key);
            format!(
                "🥤 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu meminum *Health Potion*.\n> ❤️ Health sekarang: {}/{}",
                s.health, s.max_health
            )
        }
        "mpotion" => {
            if s.mana >= s.max_mana {
                return Reply::text("💧 *ᴍᴀɴᴀ ᴘᴇɴᴜʛ*\n\n> Mana kamu sudah penuh!");
            }
            s.mana = (s.mana + 50).min(s.max_mana);
            consume(&mut inv, &item_key);
            format!(
                "🧪 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu meminum *Mana Potion*.\n> 💧 Mana sekarang: {}/{}",
                s.mana, s.max_mana
            )
        }
        "stamina" => {
            if s.stamina >= s.max_stamina {
                return Reply::text("⚡ *sᴛᴀᴍɪɴᴀ ᴘᴇɴᴜʛ*\n\n> Stamina kamu sudah penuh!");
            }
            s.stamina = (s.stamina + 20).min(s.max_stamina);
            consume(&mut inv, &item_key);
            format!(
                "⚡ *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu meminum *Stamina Potion*.\n> ⚡ Stamina sekarang: {}/{}",
                s.stamina, s.max_stamina
            )
        }
        "herb" => {
            if s.health >= s.max_health {
                return full_health();
            }
            s.health = (s.health + 20).min(s.max_health);
            consume(&mut inv, &item_key);
            format!(
                "🌿 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu mengunyah *Herba*.\n> ❤️ Health sekarang: {}/{}",
                s.health, s.max_health
            )
        }
        "leather" => {
            s.attack = Some(or_default(s.attack, 10) + 3);
            consume(&mut inv, &item_key);
            format!(
                "👞 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu memakai *Kulit* sebagai pelindung.\n\
                 ⚔️ Attack bertambah: +3 (sekarang: {})",
                s.attack.unwrap()
            )
        }
        "mysterybox" => {
            consume(&mut inv, &item_key);
            // rewards[3] — `potion` dan `diamond` punya qty, dua lainnya nominal.
            match crate::common::random_index(4) {
                0 => {
                    let amount = roll_excl(1000, 50000);
                    koin_gain = amount;
                    format!("📦 *ᴍʏsᴛᴇʀʏ ʙᴏx ᴅɪʙᴜᴋᴀ!*\n\n> Kamu membuka Mystery Box...\n\
                     > 💰 Koin: +{}", thousands_id(amount as f64))
                }
                1 => {
                    let amount = roll_excl(500, 5000);
                    exp_gain = amount;
                    format!("📦 *ᴍʏsᴛᴇʀʏ ʙᴏx ᴅɪʙᴜᴋᴀ!*\n\n> Kamu membuka Mystery Box...\n\
                     > ✨ EXP: +{}", thousands_id(amount as f64))
                }
                2 => {
                    let qty = crate::rpg::jualan::roll_incl(1, 3);
                    loot = Some(("potion", qty));
                    format!("📦 *ᴍʏsᴛᴇʀʏ ʙᴏx ᴅɪʙᴜᴋᴀ!*\n\n> Kamu membuka Mystery Box...\n\
                     > 🥤 potion: +{qty}")
                }
                _ => {
                    let qty = crate::rpg::jualan::roll_incl(1, 2);
                    loot = Some(("diamond", qty));
                    format!("📦 *ᴍʏsᴛᴇʀʏ ʙᴏx ᴅɪʙᴜᴋᴀ!*\n\n> Kamu membuka Mystery Box...\n\
                     > 💠 diamond: +{qty}")
                }
            }
        }
        "bowlramen" => {
            if s.health >= s.max_health {
                return Reply::text(
                    "❤️ *ʜᴇᴀʟᴛʜ ᴘᴇɴᴜʛ*\n\n> Nyawa kamu sudah penuh, tidak perlu makan ramen lagi!",
                );
            }
            s.health = (s.health + 40).min(s.max_health);
            consume(&mut inv, &item_key);
            format!(
                "🍜 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu memakan semangkuk *Ramen Hangat*.\n\
                 > ❤️ Health memulih: {}/{}",
                s.health, s.max_health
            )
        }
        "chakra" => {
            if s.stamina >= s.max_stamina {
                return Reply::text(
                    "⚡ *sᴛᴀᴍɪɴᴀ ᴘᴇɴᴜʛ*\n\n> Stamina/Chakra kamu sudah penuh!",
                );
            }
            s.stamina = (s.stamina + 30).min(s.max_stamina);
            consume(&mut inv, &item_key);
            format!(
                "🌀 *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu menyerap *Kepingan Chakra*.\n\
                 > ⚡ Stamina bertambah: {}/{}",
                s.stamina, s.max_stamina
            )
        }
        "kunai" | "shuriken" => {
            s.attack = Some(or_default(s.attack, 10) + 2);
            consume(&mut inv, &item_key);
            format!(
                "🗡️ *ɪᴛᴇᴍ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Kamu melengkapi dirimu dengan *{}*.\n\
                 ⚔️ Attack bertambah: +2 (sekarang: {})",
                item_key.to_uppercase(),
                s.attack.unwrap()
            )
        }
        "scroll" => {
            consume(&mut inv, &item_key);
            if crate::common::random_index(2) == 0 {
                let amount = roll_excl(2000, 10000);
                koin_gain = amount;
                format!("📜 *sᴄʀᴏʟʟ ᴅɪʙᴀᴄᴀ!*\n\n> Kamu membuka Gulungan Rahasia Ninja...\n\
                 > 💰 Ryo (Koin): +{}", thousands_id(amount as f64))
            } else {
                let amount = roll_excl(1000, 8000);
                exp_gain = amount;
                format!("📜 *sᴄʀᴏʟʟ ᴅɪʙᴀᴄᴀ!*\n\n> Kamu membuka Gulungan Rahasia Ninja...\n\
                 > ✨ EXP Ninja: +{}", thousands_id(amount as f64))
            }
        }
        "common" | "uncommon" | "mythic" | "legendary" => {
            consume(&mut inv, &item_key);
            let legendary = item_key == "legendary";
            // `Math.floor(Math.random() * (legendary ? 100000 : 10000)) + 1000`
            let money = roll_excl(1000, if legendary { 101_000 } else { 11_000 });
            let exp = roll_excl(100, if legendary { 5100 } else { 600 });
            koin_gain = money;
            exp_gain = exp;
            format!(
                "🎁 *ᴄʀᴀᴛᴇ ᴅɪʙᴜᴋᴀ*\n\n> Kamu membuka *{item_key} Crate*!\n\
                 > 💰 Money: +Rp {}\n> 🚄 Exp: +{exp}",
                thousands_id(money as f64)
            )
        }
        _ => {
            return Reply::text(format!(
                "❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴅᴀᴘᴀᴛ ᴅɪɢᴜɴᴀᴋᴀɴ*\n\n> Item *{item_key}* tidak bisa digunakan langsung."
            ));
        }
    };

    if let Some((item, qty)) = loot {
        let slot = inv.entry(item.to_string()).or_insert(json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = json!(have + qty);
    }
    s.write(&mut rpg);
    if let Some(attack) = s.attack {
        rpg.insert("attack".into(), json!(attack));
    }

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let mut data = serde_json::Map::new();
    if koin_gain > 0 {
        data.insert("koin".into(), json!(koin + koin_gain));
    }
    if exp_gain > 0 {
        data.insert("exp".into(), json!(exp_gain_only(old_exp, exp_gain)));
    }
    data.insert("rpg".into(), Value::Object(rpg));
    data.insert("inventory".into(), Value::Object(inv));

    Reply::text(msg).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: Value::Object(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "use".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("use").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Versi yang memiliki reply-nya — untuk `text(run(..))` di dalam assert.
    fn text(r: Reply) -> String {
        text_of(&r).to_string()
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

    fn bag(item: &str, n: i64) -> Value {
        json!({ "inventory": { (item): n } })
    }

    /// Bar stat kosong di semua resource — supaya guard "sudah penuh" tidak
    /// memblokir. Semua nilai default tetap dibaca lewat `||`.
    fn weak(item: &str, n: i64) -> Value {
        json!({
            "inventory": { (item): n },
            "rpg": { "health": 10, "mana": 10, "stamina": 10 },
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("use").unwrap();
        assert_eq!(p.meta.name, "use");
        assert_eq!(p.meta.alias, ["pake", "makan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Menggunakan item consumable atau membuka crate"
        );
        assert_eq!(p.meta.usage, ".use <item>");
        assert_eq!(p.meta.example, ".use potion");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pake", "makan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "use");
        }
    }

    #[test]
    fn no_item_name_prints_the_usage_card() {
        let r = run(&[], json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("🎒 *ᴜsᴇ ɪᴛᴇᴍ*"), "{t}");
        assert!(t.contains("`.use <nama_item>`") && t.contains("`.inventory`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_item_that_is_not_in_the_bag_is_named_back_to_you() {
        let r = run(&["potion"], json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴀᴅᴀ*"), "{t}");
        assert!(t.contains("Kamu tidak memiliki item *potion*!"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_item_key_is_case_insensitive() {
        let r = run(&["Potion"], weak("potion", 1));
        assert!(text_of(&r).contains("Health Potion"));
    }

    #[test]
    fn an_unknown_item_is_refused_before_the_bag_is_touched() {
        let r = run(&["batu"], bag("batu", 5));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɪᴛᴇᴍ ᴛɪᴅᴀᴋ ᴅᴀᴘᴀᴛ ᴅɪɢᴜɴᴀᴋᴀɴ*"), "{t}");
        assert!(t.contains("Item *batu* tidak bisa digunakan langsung."), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn drinking_at_full_health_spends_nothing() {
        for item in ["potion", "herb", "bowlramen"] {
            let r = run(&[item], bag(item, 3));
            assert!(text_of(&r).contains("Nyawa kamu sudah penuh"), "{item}");
            assert!(r.mutations().is_empty(), "{item}");
        }
    }

    #[test]
    fn healing_caps_at_max_health_and_removes_the_last_stack_entry() {
        // `health` tidak ada -> `|| 100` = 100, sama dengan `maxHealth` 100, jadi
        // clamp kicked in dan potion tetap dimakan.
        let r = run(&["potion"], weak("potion", 1));
        let d = written(&r);
        assert!(d["inventory"].get("potion").is_none(), "sisa 0 dihapus");
        assert_eq!(d["rpg"]["health"], json!(60), "10 + 50 = 60");

        let r = run(
            &["potion"],
            json!({"inventory": {"potion": 2}, "rpg": {"health": 80, "maxHealth": 90}}),
        );
        let d = written(&r);
        assert_eq!(d["rpg"]["health"], json!(90), "80 + 50 -> clamp 90");
        assert_eq!(d["inventory"]["potion"], json!(1));
    }

    #[test]
    fn the_six_healing_items_really_move_different_resources() {
        for (item, field, after) in [
            ("potion", "health", 60i64),
            ("herb", "health", 30),
            ("bowlramen", "health", 50),
            ("mpotion", "mana", 60),
            ("stamina", "stamina", 30),
            ("chakra", "stamina", 40),
        ] {
            let r = run(&[item], weak(item, 1));
            assert_eq!(
                written(&r)["rpg"][field],
                json!(after),
                "{item} harus menaikkan {field} ke {after} (dari 10)"
            );
        }
    }

    #[test]
    fn a_full_mana_or_stamina_bar_blocks_the_potion() {
        for (item, expected) in [
            ("mpotion", "Mana kamu sudah penuh!"),
            ("stamina", "Stamina kamu sudah penuh!"),
            ("chakra", "Stamina/Chakra kamu sudah penuh!"),
        ] {
            let r = run(&[item], bag(item, 2));
            assert!(text_of(&r).contains(expected), "{item}");
            assert!(r.mutations().is_empty(), "{item}");
        }
    }

    #[test]
    fn armour_and_throwing_weapons_add_two_and_three_attack() {
        for (item, add) in [("leather", 3), ("kunai", 2), ("shuriken", 2)] {
            let r = run(&[item], bag(item, 1));
            let t = text_of(&r);
            assert!(t.contains(&format!("⚔️ Attack bertambah: +{add} (sekarang: {})", 10 + add)), "{item}: {t}");
            assert_eq!(written(&r)["rpg"]["attack"], json!(10 + add), "{item}");
        }
        // Attack yang sudah ada ditambahkan, bukan ditimpa.
        let r = run(
            &["leather"],
            json!({"inventory": {"leather": 1}, "rpg": {"attack": 40}}),
        );
        assert_eq!(written(&r)["rpg"]["attack"], json!(43));
    }

    #[test]
    fn throwing_weapons_are_upper_cased_in_the_message() {
        let t = text(run(&["shuriken"], weak("shuriken", 1)));
        assert!(t.contains("dengan *SHURIKEN*"), "{t}");
    }

    #[test]
    fn every_mystery_box_pick_pays_out_in_exactly_one_of_the_four_shapes() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1500 {
            let r = run(
                &["mysterybox"],
                json!({"koin": 1000, "exp": 0, "inventory": {"mysterybox": 1, "diamond": 0}}),
            );
            let d = written(&r);
            let t = text_of(&r);
            assert!(
                d["inventory"].get("mysterybox").is_none(),
                "kotak dimakan: {d}"
            );
            if t.contains("> 💰 Koin: +") {
                seen.insert("koin");
                assert!(d.get("exp").is_none());
                let gain = d["koin"].as_i64().unwrap() - 1000;
                assert!((1000..50000).contains(&gain), "{gain}");
            } else if t.contains("> ✨ EXP: +") {
                seen.insert("exp");
                let gain = d["exp"].as_i64().unwrap();
                assert!((500..5000).contains(&gain), "{gain}");
                assert!(d.get("koin").is_none());
            } else if t.contains("> 🥤 potion: +") {
                seen.insert("potion");
                let q = d["inventory"]["potion"].as_i64().unwrap();
                assert!((1..=3).contains(&q), "potion qty {q} di luar 1..=3");
            } else {
                assert!(t.contains("> 💠 diamond: +"), "{t}");
                seen.insert("diamond");
                let q = d["inventory"]["diamond"].as_i64().unwrap();
                assert!((1..=2).contains(&q), "{q}");
            }
            assert!(d["rpg"].is_object(), "rpg selalu ditulis");
        }
        assert_eq!(seen.len(), 4, "empat hadiah harus muncul: {seen:?}");
    }

    #[test]
    fn a_ninja_scroll_pays_either_money_or_experience() {
        for _ in 0..600 {
            let r = run(
                &["scroll"],
                json!({"koin": 0, "exp": 0, "inventory": {"scroll": 1}}),
            );
            let d = written(&r);
            let t = text_of(&r);
            assert_eq!(d["inventory"].get("scroll"), None);
            if t.contains("Ryo (Koin)") {
                let gain = d["koin"].as_i64().unwrap();
                assert!((2000..10000).contains(&gain), "{gain}");
                assert!(d.get("exp").is_none());
            } else {
                assert!(t.contains("EXP Ninja"), "{t}");
                let gain = d["exp"].as_i64().unwrap();
                assert!((1000..8000).contains(&gain), "{gain}");
                assert!(d.get("koin").is_none());
            }
        }
    }

    #[test]
    fn a_crate_pays_money_and_experience_together() {
        for crate_key in ["common", "uncommon", "mythic", "legendary"] {
            for _ in 0..120 {
                let r = run(&[crate_key], weak(crate_key, 1));
                let d = written(&r);
                let t = text_of(&r);
                assert!(t.contains(&format!("Kamu membuka *{crate_key} Crate*!")), "{t}");
                assert_eq!(d["inventory"].get(crate_key), None);
                let (money, exp) = if crate_key == "legendary" {
                    (1000..101_000, 100..5100)
                } else {
                    (1000..11_000, 100..600)
                };
                let gain_money = d["koin"].as_i64().unwrap();
                let gain_exp = d["exp"].as_i64().unwrap();
                assert!(money.contains(&gain_money), "{crate_key}: uang {gain_money}");
                assert!(exp.contains(&gain_exp), "{crate_key}: exp {gain_exp}");
            }
        }
    }

    #[test]
    fn exp_from_a_crate_does_not_rebuild_the_rpg_stats() {
        // `db.updateExp` tidak punya `checkAndNotifyLevelUp`, jadi tidak ada
        // `rpg.level` / `rpg.maxHealth` yang ditulis ulang walau level naik.
        let mut crossed = false;
        for _ in 0..400 {
            let r = run(&["common"], bag("common", 1));
            let d = written(&r);
            let gained = d["exp"].as_i64().unwrap();
            assert!(d["rpg"].get("level").is_none(), "crate tidak mengisi level: {d}");
            if gained >= 100 {
                crossed = true;
            }
        }
        assert!(crossed, "crate selalu memberi exp");
    }

    #[test]
    fn consuming_the_last_copy_removes_the_key_but_leaves_the_others() {
        let r = run(
            &["potion"],
            json!({
                "inventory": { "potion": 1, "herb": 9 },
                "rpg": { "health": 10 },
            }),
        );
        let inv = written(&r)["inventory"].as_object().unwrap().clone();
        assert!(!inv.contains_key("potion"), "{inv:?}");
        assert_eq!(inv["herb"], json!(9), "barang lain tak boleh gone");
    }

    #[test]
    fn stats_are_written_back_even_when_the_item_does_not_touch_them() {
        // `user.rpg.* = ... || default` dijalankan sebelum switch, jadi baris
        // `rpg` selalu berisi enam field setelah pemakaian item apa pun.
        let r = run(&["leather"], bag("leather", 1));
        let rpg = written(&r)["rpg"].as_object().unwrap().clone();
        for field in ["health", "maxHealth", "mana", "maxMana", "stamina", "maxStamina"] {
            assert_eq!(rpg[field], json!(100), "{field} di-backfill ke 100");
        }
    }
}
