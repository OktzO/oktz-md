//! `plugins/rpg/pet.js` — kelola pet / hewan peliharaan.
//!
//! Catatan kursor: `newName.length` di JS menghitung UNIT UTF-16, sedangkan
//! `chars().count()` menghitung SKALA. Untuk nama ASCII keduanya sama; nama
//! ber-emoji bisa beda satu poin. Limit 2..=15 dipakai apa adanya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::jualan::{inventory_map, roll_incl, rpg_map};

/// `(kunci, nama, attack, defense, luck, evolusi)`; `None` = bentuk sempurna.
pub(crate) type PetType = (&'static str, &'static str, i64, i64, i64, Option<&'static str>);

/// `PET_TYPES` (`pet.js:19-30`).
pub(crate) const PET_TYPES: [PetType; 10] = [
    ("cat", "🐱 Kucing", 5, 3, 5, Some("lion")),
    ("dog", "🐕 Anjing", 8, 5, 2, Some("wolf")),
    ("bird", "🐦 Burung", 4, 2, 8, Some("phoenix")),
    ("fish", "🐟 Ikan", 2, 2, 10, Some("dragon")),
    ("rabbit", "🐰 Kelinci", 3, 4, 6, Some("thunderbunny")),
    ("lion", "🦁 Singa", 15, 10, 8, None),
    ("wolf", "🐺 Serigala", 18, 12, 5, None),
    ("phoenix", "🔥 Phoenix", 12, 8, 15, None),
    ("dragon", "🐉 Naga", 20, 15, 12, None),
    ("thunderbunny", "⚡ Thunder Bunny", 10, 12, 18, None),
];

/// `FOOD_ITEMS` (`pet.js:32-38`) — `(kunci, nama, hunger, exp)`.
pub(crate) const FOODS: [(&str, &str, i64, i64); 5] = [
    ("bread", "🍞 Roti", 10, 5),
    ("fish", "🐟 Ikan", 20, 10),
    ("meat", "🍖 Daging", 30, 15),
    ("fruit", "🍎 Buah", 15, 8),
    ("premium_food", "⭐ Premium Food", 50, 30),
];

pub(crate) fn find_pet_type(key: &str) -> Option<&'static PetType> {
    PET_TYPES.iter().find(|p| p.0 == key)
}

pub(crate) fn find_food(key: &str) -> Option<&'static (&'static str, &'static str, i64, i64)> {
    FOODS.iter().find(|f| f.0 == key)
}

/// `pet.name`, `pet.level || 1`, `pet.exp || 0`, `pet.hunger` — semua pembacaan
/// defensif karena bar bisa dibuat plugin lain.
fn num(pet: &Map<String, Value>, key: &str) -> i64 {
    pet.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn num_or(pet: &Map<String, Value>, key: &str, default: i64) -> i64 {
    crate::rpg::jualan::or_default(pet.get(key).and_then(Value::as_i64), default)
}

/// `expNeeded = (pet.level || 1) * 100` + blok naik level. Mengembalikan
/// pesan level-up (bisa kosong). Satu kali naik per panggilan — sama seperti
/// JS yang hanya memeriksa sekali tanpa `while`.
pub(crate) fn level_up(pet: &mut Map<String, Value>, base: (i64, i64, i64), shout: &str) -> String {
    let needed = num_or(pet, "level", 1) * 100;
    if num(pet, "exp") < needed {
        return String::new();
    }
    let level = num_or(pet, "level", 1) + 1;
    pet.insert("level".into(), serde_json::json!(level));
    pet.insert("exp".into(), serde_json::json!(num(pet, "exp") - needed));
    if !pet.get("stats").is_some_and(Value::is_object) {
        let mut stats = serde_json::Map::new();
        stats.insert("attack".into(), serde_json::json!(base.0));
        stats.insert("defense".into(), serde_json::json!(base.1));
        stats.insert("luck".into(), serde_json::json!(base.2));
        pet.insert("stats".into(), Value::Object(stats));
    }
    let s = pet.get_mut("stats").and_then(Value::as_object_mut).unwrap();
    for (k, delta) in [("attack", 2), ("defense", 1), ("luck", 1)] {
        let v = s.get(k).and_then(Value::as_i64).unwrap_or(0) + delta;
        s.insert(k.to_string(), serde_json::json!(v));
    }
    format!(
        "\n🎉 *{shout} Si {} LEVEL UP jadi Level {level}!* 🎉",
        pet.get("name").and_then(Value::as_str).unwrap_or(""),
    )
}

#[plugin(
    name = "pet",
    alias = ["mypet", "hewanku", "peliharaan"],
    category = "rpg",
    description = "Kelola pet/hewan peliharaan",
    usage = ".pet <feed/train/status>",
    example = ".pet status",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn pet(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);

    // `if (!user.rpg.pet)` — `null` falsy, jadi pet null sama dengan belum punya.
    let Some(raw_pet) = ctx.user_field("rpg.pet").filter(|v| !v.is_null()).cloned() else {
        return Reply::text(format!(
            "Kamu belum punya hewan peliharaan kak! 😭\n\
             Sedih banget bertualang sendirian...\n\n\
             *Cara dapetin teman:* \n\
             🛒 Beli di `{}petshop`\n\
             💕 Dapat dari `{}breeding`\n\
             🗡️ Drop rare dari boss!",
            ctx.prefix, ctx.prefix
        ));
    };
    let mut pet = raw_pet.as_object().cloned().unwrap_or_default();

    // `PET_TYPES[pet.type]` di JS adalah `undefined` kalau jenis asing, dan
    // baris berikutnya langsung accessing `.name` — TypeError. Di sini jenis
    // asing ditolak eksplisit supaya tidak menggagalkan seluruh handler.
    let kind = pet
        .get("type")
        .and_then(Value::as_str)
        .and_then(find_pet_type);
    let Some(&(_, species, atk, def, luck, evolve)) = kind else {
        return Reply::text(format!(
            "Pet *{}* punya jenis yang tidak dikenal databasenya. 🐾\n\
             Pergi ke `{}petshop` dan beli ulang teman baru.",
            pet.get("name").and_then(Value::as_str).unwrap_or("?"),
            ctx.prefix
        ));
    };
    let base = (atk, def, luck);
    let name = pet
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let action = ctx.args.first().map(|a| a.to_lowercase());

    let mut inv = inventory_map(&ctx);

    match action.as_deref() {
        Some("feed") => {
            let Some(key) = ctx.args.get(1).map(|a| a.to_lowercase()) else {
                let mut txt = format!(
                    "Si {name} lagi ngeliatin kamu sambil jilat bibir... 🤤\nMau dikasih makan apa nih?\n\n*Daftar Makanan di Tasmu:*\n"
                );
                for (key, food, hunger, exp) in FOODS {
                    let have = inv.get(key).and_then(Value::as_i64).unwrap_or(0);
                    txt += &format!(
                        "\n*{food}* (Punya: {have}x)\n🍖 Kenyang: +{hunger} | ✨ EXP: +{exp}\n👉 Kasih makan: `.pet feed {key}`\n"
                    );
                }
                return Reply::text(txt);
            };
            let Some(&(_, food, hunger, food_exp)) = find_food(&key) else {
                return Reply::text(
                    "Waduh, jangan kasih makanan aneh-aneh kak! Kasihan nanti sakit perut 😂❌",
                );
            };
            if inv.get(key.as_str()).and_then(Value::as_i64).unwrap_or(0) < 1 {
                return Reply::text(format!(
                    "Kamu nggak punya *{food}* di tasmu! Belanja dulu sana! 🛒🏃"
                ));
            }
            if num(&pet, "hunger") >= 100 {
                return Reply::text(format!(
                    "Perut si {name} udah kepenuhan kak! Jangan disiksa disuruh makan terus! 🤢"
                ));
            }

            let left = inv.get(key.as_str()).and_then(Value::as_i64).unwrap_or(0) - 1;
            if left <= 0 {
                inv.remove(key.as_str());
            } else {
                inv.insert(key.clone(), serde_json::json!(left));
            }
            pet.insert("hunger".into(), serde_json::json!((num(&pet, "hunger") + hunger).min(100)));
            pet.insert("exp".into(), serde_json::json!(num(&pet, "exp") + food_exp));
            let msg = level_up(&mut pet, base, "WOHOO!");
            let full = num(&pet, "hunger");

            rpg.insert("pet".into(), Value::Object(pet));
            return Reply::text(format!(
                "Nyam... nyam... nyam! 🤤🍖\n\n\
                 Si *{name}* lahap banget makan *{food}* yang kamu kasih!\n\
                 🍖 Perutnya keisi *+{hunger}* ({full}/100)\n\
                 ✨ Dapet EXP *+{food_exp}*{msg}"
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "rpg": rpg, "inventory": inv }),
            });
        }
        Some("train") => {
            if num(&pet, "hunger") < 20 {
                return Reply::text(format!(
                    "Tega bener nyuruh latihan pas lagi kelaparan! 😭\nSi {name} perutnya keroncongan tuh, kasih makan dulu!"
                ));
            }
            pet.insert("hunger".into(), serde_json::json!((num(&pet, "hunger") - 15).max(0)));
            let gained = roll_incl(20, 39);
            pet.insert("exp".into(), serde_json::json!(num(&pet, "exp") + gained));
            let msg = level_up(&mut pet, base, "MANTAP!");
            rpg.insert("pet".into(), Value::Object(pet));

            return Reply::text(format!(
                "Hup! Hup! Hiyah!! 🏃‍♂️💨\n\n\
                 Si *{name}* latihan fisik keras hari ini!\n\
                 ✨ EXP Bertambah: *+{gained}*\n\
                 😰 Rasa Lapar: *-15*\n{msg}"
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "rpg": rpg, "inventory": inv }),
            });
        }
        Some("rename") => {
            let new_name = ctx.args[1..].join(" ");
            let len = new_name.chars().count();
            if new_name.is_empty() || !(2..=15).contains(&len) {
                return Reply::text(
                    "Nama apaan tuh kak? Jangan aneh-aneh ah, kasih yang bener (2-15 karakter)! 😂",
                );
            }
            let old = name.clone();
            pet.insert("name".into(), serde_json::json!(new_name));
            rpg.insert("pet".into(), Value::Object(pet));
            return Reply::text(format!(
                "Sipp! Akte kelahiran sudah diubah.\nSekarang panggil dia *{new_name}*! (Sebelumnya: {old}) ✨"
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "rpg": rpg }),
            });
        }
        Some("evolve") => {
            let Some(next) = evolve else {
                return Reply::text(format!(
                    "Batas keturunan si {name} cuma sampai sini kak, dia udah di bentuk sempurnanya! 🌟"
                ));
            };
            let level = num_or(&pet, "level", 1);
            if level < 10 {
                return Reply::text(format!(
                    "Sabar kak, si {name} masih ciut! Minimal *Level 10* baru bisa berevolusi (Sekarang baru level {level}). 🐣"
                ));
            }
            let Some(&(_, evolved_name, a, d, l, _)) = find_pet_type(next) else {
                return Reply::text(format!("Evolusi ke *{next}* belum ada di databasenya."));
            };
            pet.insert("type".into(), serde_json::json!(next));
            pet.insert(
                "stats".into(),
                serde_json::json!({ "attack": a, "defense": d, "luck": l }),
            );
            pet.insert("level".into(), serde_json::json!(1));
            pet.insert("exp".into(), serde_json::json!(0));
            rpg.insert("pet".into(), Value::Object(pet));
            return Reply::text(format!(
                "CLLINGGG!! ✨🌟\n\n\
                 Si *{name}* tiba-tiba bercahaya terang banget!\n\
                 Wah, dia udah *berevolusi* jadi *{evolved_name}* yang gagah!\n\n\
                 Status barunya nge-reset ke awal tapi jadi jauh lebih kuat lho! \
                 Cek pake `.pet status` ya! 😎🔥"
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "rpg": rpg }),
            });
        }
        _ => {}
    }

    // Kartu identitas: juga jadi fallback untuk aksi tak dikenal.
    let level = num_or(&pet, "level", 1);
    let hunger = num(&pet, "hunger");
    let hunger_status = if hunger >= 70 {
        "😊 Senang & Kenyang"
    } else if hunger >= 40 {
        "😐 Biasa Aja"
    } else {
        "😰 Keroncongan Parah!"
    };
    let stat = |key: &str, base: i64| num_or(&pet, key, base);

    let mut txt = format!(
        "🐾 *Buku Identitas Peliharaan* 🐾\n\n\
         *Profil Si {name}:*\n\
         • Spesies: *{species}*\n\
         • Level: *{level}*\n\
         • EXP: *{} / {}*\n\
         • Perut: *{hunger}/100* ({hunger_status})\n\n\
         *Kekuatan Fisik:*\n\
         ⚔️ Attack: *{}*\n\
         🛡️ Defense: *{}*\n\
         🍀 Luck: *{}*\n\n\
         *Interaksi:*\n\
         👉 `{}pet feed <makanan>` - Kasih makan\n\
         👉 `{}pet train` - Latih biar kuat\n\
         👉 `{}pet rename <nama_baru>` - Ganti nama\n",
        num(&pet, "exp"),
        level * 100,
        stat("attack", atk),
        stat("defense", def),
        stat("luck", luck),
        ctx.prefix,
        ctx.prefix,
        ctx.prefix,
    );
    if evolve.is_some() {
        txt += &format!("👉 `{}pet evolve` - Berevolusi (Bila cukup syarat)\n", ctx.prefix);
    }
    Reply::text(txt)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pet".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("pet").unwrap().handler)(ctx)
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

    fn with_pet(extra: Value) -> Value {
        let mut pet = serde_json::json!({
            "type": "cat", "name": "Kucing", "level": 1, "exp": 0,
            "hunger": 50, "stats": Value::Null,
        });
        if let (Some(p), Some(extra)) = (pet.as_object_mut(), extra.as_object()) {
            for (k, v) in extra {
                p.insert(k.clone(), v.clone());
            }
        }
        serde_json::json!({ "rpg": { "pet": pet } })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pet").unwrap();
        assert_eq!(p.meta.name, "pet");
        assert_eq!(p.meta.alias, ["mypet", "hewanku", "peliharaan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Kelola pet/hewan peliharaan");
        assert_eq!(p.meta.usage, ".pet <feed/train/status>");
        assert_eq!(p.meta.example, ".pet status");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mypet", "hewanku", "peliharaan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pet");
        }
    }

    #[test]
    fn without_a_pet_the_plugin_points_at_the_three_ways_to_get_one() {
        for user in [serde_json::json!({}), serde_json::json!({"rpg": {}}), serde_json::json!({"rpg": {"pet": null}})] {
            let r = run(&["status"], user.clone());
            let t = text_of(&r);
            assert!(t.starts_with("Kamu belum punya hewan peliharaan kak!"), "{user}: {t}");
            assert!(t.contains("`.petshop`") && t.contains("`.breeding`"), "{user}: {t}");
            assert!(r.mutations().is_empty(), "{user}");
        }
    }

    #[test]
    fn the_id_card_backfills_the_stats_from_the_species_table() {
        let r = run(&["status"], with_pet(serde_json::json!({})));
        let t = text_of(&r);
        assert!(t.starts_with("🐾 *Buku Identitas Peliharaan*"), "{t}");
        assert!(t.contains("• Spesies: *🐱 Kucing*"), "{t}");
        assert!(t.contains("• Level: *1*\n• EXP: *0 / 100*"), "{t}");
        assert!(t.contains("⚔️ Attack: *5*\n🛡️ Defense: *3*\n🍀 Luck: *5*"), "{t}");
        assert!(t.contains("• Perut: *50/100* (😐 Biasa Aja)"), "{t}");
        assert!(t.contains("`.pet evolve`"), "kucing bisa evolusi: {t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_final_form_hides_the_evolve_hint() {
        let t = text(run(
            &["status"],
            with_pet(serde_json::json!({"type": "dragon"})),
        ));
        assert!(t.contains("• Spesies: *🐉 Naga*"), "{t}");
        assert!(!t.contains("pet evolve"), "naga sudah final: {t}");
    }

    #[test]
    fn an_unknown_action_prints_the_card_instead_of_falling_silent() {
        // `handler` di JS mengembalikan `undefined` untuk aksi tak dikenal yang
        // lolos daftar aksi — sama seperti tanpa-balas. Di sini tetap kartu.
        let r = run(&["terbang"], with_pet(serde_json::json!({})));
        assert!(text_of(&r).starts_with("🐾 *Buku Identitas Peliharaan*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn feeding_without_a_food_name_lists_the_larder() {
        let r = run(&["feed"], with_pet(serde_json::json!({})));
        let t = text_of(&r);
        assert!(t.starts_with("Si Kucing lagi ngeliatin kamu sambil jilat bibir..."), "{t}");
        for (key, name, hunger, exp) in FOODS {
            assert!(t.contains(&format!("*{name}* (Punya: 0x)")), "{key}: {t}");
            assert!(t.contains(&format!("🍖 Kenyang: +{hunger} | ✨ EXP: +{exp}")), "{key}: {t}");
            assert!(t.contains(&format!("`.pet feed {key}`")), "{key}: {t}");
        }
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn feeding_needs_the_food_in_the_bag_and_a_hungry_pet() {
        let r = run(
            &["feed", "meat"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 50}}, "inventory": {}}),
        );
        assert!(text_of(&r).contains("Kamu nggak punya *🍖 Daging* di tasmu!"), "{:?}", text_of(&r));

        let r = run(
            &["feed", "meat"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 100}}, "inventory": {"meat": 3}}),
        );
        assert!(text_of(&r).contains("Perut si Kucing udah kepenuhan kak!"), "{:?}", text_of(&r));

        let r = run(
            &["feed", "batu"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "hunger": 10}}, "inventory": {"batu": 1}}),
        );
        assert!(text_of(&r).contains("jangan kasih makanan aneh-aneh"));
    }

    #[test]
    fn feeding_consumes_one_stack_and_never_leaves_a_zero_behind() {
        let r = run(
            &["feed", "bread"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 10}}, "inventory": {"bread": 1, "fish": 4}}),
        );
        let d = written(&r);
        assert_eq!(d["inventory"], serde_json::json!({"fish": 4}), "1 - 1 = 0 -> dihapus");
        assert_eq!(d["rpg"]["pet"]["hunger"], serde_json::json!(20), "10 + 10");
        assert_eq!(d["rpg"]["pet"]["exp"], serde_json::json!(5));
        assert!(text_of(&r).contains("🍖 Perutnya keisi *+10* (20/100)"));
    }

    #[test]
    fn crossing_a_hundred_food_units_still_caps_the_pet_at_one_hundred() {
        let r = run(
            &["feed", "premium_food"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 90}}, "inventory": {"premium_food": 9}}),
        );
        let d = written(&r);
        assert_eq!(d["rpg"]["pet"]["hunger"], serde_json::json!(100), "90 + 50 di-clamp");
        assert!(text_of(&r).contains("(100/100)"));
    }

    #[test]
    fn training_burns_fifteen_food_units_and_lands_in_the_gain_band() {
        for _ in 0..300 {
            let r = run(
                &["train"],
                serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 20}}}),
            );
            let p = &written(&r)["rpg"]["pet"];
            assert_eq!(p["hunger"], serde_json::json!(5), "20 - 15");
            let exp = p["exp"].as_i64().unwrap();
            assert!((20..=39).contains(&exp), "exp {exp}");
            assert!(text_of(&r).contains(&format!("✨ EXP Bertambah: *+{exp}*")));
        }
    }

    #[test]
    fn a_starving_pet_may_not_be_trained() {
        let r = run(
            &["train"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "hunger": 19}}}),
        );
        assert!(text_of(&r).contains("Tega bener nyuruh latihan pas lagi kelaparan!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_level_crossing_seeds_the_stats_from_the_species_then_bumps_them() {
        // 95 exp + 10 dari ikan = 105 >= (1 * 100).
        let r = run(
            &["feed", "fish"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 95, "hunger": 0, "stats": null}}, "inventory": {"fish": 1}}),
        );
        let p = &written(&r)["rpg"]["pet"];
        assert_eq!(p["level"], serde_json::json!(2));
        assert_eq!(p["exp"], serde_json::json!(5), "105 - 100");
        assert_eq!(p["stats"], serde_json::json!({"attack": 7, "defense": 4, "luck": 6}), "5+2 / 3+1 / 5+1");
        assert!(text_of(&r).contains("Si Kucing LEVEL UP jadi Level 2!"));
    }

    #[test]
    fn an_existing_stats_block_is_bumped_not_reseeded() {
        let r = run(
            &["train"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 1, "exp": 95, "hunger": 20, "stats": {"attack": 50, "defense": 40, "luck": 30}}}}),
        );
        assert_eq!(
            written(&r)["rpg"]["pet"]["stats"],
            serde_json::json!({"attack": 52, "defense": 41, "luck": 31})
        );
    }

    #[test]
    fn renaming_joins_the_remaining_arguments_and_checks_the_length() {
        let r = run(
            &["rename", "Oyen", "Merah"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing"}}}),
        );
        let t = text_of(&r);
        assert!(t.contains("Sekarang panggil dia *Oyen Merah*!"), "{t}");
        assert!(t.contains("(Sebelumnya: Kucing)"), "{t}");
        assert_eq!(written(&r)["rpg"]["pet"]["name"], serde_json::json!("Oyen Merah"));

        for args in [
            vec!["rename", "a"],
            vec!["rename"],
            vec!["rename", "abcdefghijklmnopq"],
        ] {
            let r = run(&args, serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing"}}}));
            assert!(text_of(&r).contains("2-15 karakter"), "{args:?}");
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn evolving_needs_level_ten_and_resets_the_progress() {
        let r = run(
            &["evolve"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 9, "exp": 400, "hunger": 50}}}),
        );
        assert!(text_of(&r).contains("Minimal *Level 10*"), "{:?}", text_of(&r));

        let r = run(
            &["evolve"],
            serde_json::json!({"rpg": {"pet": {"type": "cat", "name": "Kucing", "level": 10, "exp": 400, "hunger": 50, "stats": {"attack": 9, "defense": 9, "luck": 9}}}}),
        );
        let p = &written(&r)["rpg"]["pet"];
        assert_eq!(p["type"], serde_json::json!("lion"));
        assert_eq!(p["stats"], serde_json::json!({"attack": 15, "defense": 10, "luck": 8}), "di-reset ke base singa");
        assert_eq!(p["level"], serde_json::json!(1));
        assert_eq!(p["exp"], serde_json::json!(0));
        assert!(text_of(&r).contains("jadi *🦁 Singa* yang gagah!"));
    }

    #[test]
    fn an_unknown_species_is_reported_instead_of_crashing_the_handler() {
        let r = run(&["status"], serde_json::json!({"rpg": {"pet": {"type": "unicorn", "name": "Mane"}}}));
        assert!(text_of(&r).contains("tidak dikenal databasenya"), "{:?}", text_of(&r));
        assert!(r.mutations().is_empty());
    }
}
