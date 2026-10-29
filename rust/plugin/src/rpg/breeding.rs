//! `plugins/rpg/breeding.js` — kawin silang peliharaan dua pemain.
//!
//! `ponytail:` `db.getUser(mentioned)` hanya terbaca lewat roster grup
//! (`other_user`), dan `isGroup: true` di JS — jadi di private chat plugin ini
//! selalu menjawab "nggak punya peliharaan" karena pasangan tidak ada di
//! store yang bisa dibaca. `PET_NAMES[type]` juga bisa `undefined` untuk tipe
//! tak dikenal; `?.split(" ")[1] || "Baby"` menutupnya di `nama` bayi.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::heal::{apply_exp, or_default, other_user, rpg_map};

const BREEDING_COST: i64 = 3_000;

/// Tipe yang dianggap langka untuk reward exp dan banner.
const RARE: [&str; 5] = ["lion", "wolf", "phoenix", "dragon", "thunderbunny"];

/// `(key, hasil)` — `default` dipakai kalau pasangan tak dikenal.
const BREEDING_RESULTS: [(&str, &[&str]); 9] = [
    ("bird+bird", &["bird", "bird", "phoenix"]),
    ("cat+bird", &["cat", "bird", "phoenix"]),
    ("cat+cat", &["cat", "cat", "lion"]),
    ("cat+dog", &["cat", "dog", "rabbit"]),
    ("dog+dog", &["dog", "dog", "wolf"]),
    ("dog+rabbit", &["dog", "rabbit", "wolf"]),
    ("fish+fish", &["fish", "fish", "dragon"]),
    ("rabbit+rabbit", &["rabbit", "rabbit", "thunderbunny"]),
    ("default", &["cat", "dog", "bird", "fish", "rabbit"]),
];

const PET_NAMES: [(&str, &str); 10] = [
    ("bird", "🐦 Burung"),
    ("cat", "🐱 Kucing"),
    ("dog", "🐕 Anjing"),
    ("dragon", "🐉 Naga"),
    ("fish", "🐟 Ikan"),
    ("lion", "🦁 Singa"),
    ("phoenix", "🔥 Phoenix"),
    ("rabbit", "🐰 Kelinci"),
    ("thunderbunny", "⚡ Thunder Bunny"),
    ("wolf", "🐺 Serigala"),
];

fn pet_name(tipe: &str) -> Option<&'static str> {
    PET_NAMES
        .iter()
        .find(|(k, _)| *k == tipe)
        .map(|(_, v)| *v)
}

/// `[myPet.type, partnerPet.type].sort().join("+")`.
fn breed_key(mine: &str, theirs: &str) -> String {
    let mut pair = [mine, theirs];
    pair.sort_unstable();
    pair.join("+")
}

fn offspring(key: &str) -> &'static str {
    let results = BREEDING_RESULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| {
            BREEDING_RESULTS
                .iter()
                .find(|(k, _)| *k == "default")
                .map(|(_, v)| *v)
                .expect("baris default selalu ada")
        });
    results[crate::common::random_index(results.len())]
}

/// `PET_NAMES[type]?.split(" ")[1] || "Baby"` — buang emote, sisakan nama.
fn baby_name(tipe: &str) -> String {
    pet_name(tipe)
        .and_then(|n| n.split(' ').nth(1))
        .unwrap_or("Baby")
        .to_string()
}

#[plugin(
    name = "breeding",
    alias = ["breed", "kawin", "petbreed"],
    category = "rpg",
    description = "Breeding pets untuk mendapat pet baru",
    usage = ".breeding @user",
    example = ".breeding @user",
    cooldown = 3600,
    energi = 3,
    is_group = true,
    is_private = false
)]
pub fn breeding(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let target = ctx
        .mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()));

    let Some(target) = target else {
        let p = &ctx.prefix;
        return Reply::text(format!(
            "💕 *Peternakan & Kawin Silang* 💕\n\n\
             Sistem ini ngebolehin peliharaan kamu kawin sama peliharaan player lain!\n\
             Siapa tau dapet keturunan langka! ✨\n\n\
             *Cara Pemakaian:*\n\
             👉 `{p}breeding @user_target`\n\n\
             *Syarat:* \n\
             1. Kamu & Target sama-sama punya pet\n\
             2. Kedua pet minimal Level 5\n\
             3. Biaya persalinan: *Rp 3.000*"
        ));
    };

    if target == ctx.sender {
        return Reply::text(
            "Hayo, mau kawin sama diri sendiri? Nggak bisa dong! Tag temanmu! 😂❌",
        );
    }

    let Some(my_pet) = rpg.get("pet").filter(|p| crate::common::js_truthy(Some(p))) else {
        return Reply::text(format!(
            "Kamu aja belum punya peliharaan kak! Beli dulu sana di `{}petshop` 😭",
            ctx.prefix
        ));
    };

    let partner_pet = other_user(&ctx, &target)
        .and_then(|u| u.get("rpg"))
        .and_then(|r| r.get("pet"))
        .filter(|p| crate::common::js_truthy(Some(p)));

    let Some(partner_pet) = partner_pet else {
        return Reply::text(
            "Target yang kamu tag ternyata nggak punya peliharaan! Kasihan pet kamu dicuekin. 💔",
        );
    };

    let my_level = or_default(my_pet.get("level").and_then(Value::as_i64), 1);
    if my_level < 5 {
        return Reply::text(format!(
            "Pet kamu masih terlalu bocil buat kawin! Minimal *Level 5* kak (Sekarang Level {my_level}). 🐣"
        ));
    }

    let partner_level = or_default(partner_pet.get("level").and_then(Value::as_i64), 1);
    if partner_level < 5 {
        return Reply::text(format!(
            "Pet pasanganmu masih terlalu kecil buat dikawinin! Minimal *Level 5* kak (Sekarang Level {partner_level}). 🐣"
        ));
    }

    let koin = crate::common::int_field(&ctx, "koin");
    if koin < BREEDING_COST {
        return Reply::text(format!(
            "Uang kamu nggak cukup buat bayar dokter hewan kak! Butuh Rp {BREEDING_COST}. 😭"
        ));
    }

    let my_type = my_pet.get("type").and_then(Value::as_str).unwrap_or_default();
    let partner_type = partner_pet
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let baby = offspring(&breed_key(my_type, partner_type));
    let is_rare = RARE.contains(&baby);

    let mut storage = rpg
        .get("petStorage")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    storage.push(serde_json::json!({
        "type": baby,
        "name": baby_name(baby),
        "level": 1,
        "exp": 0,
        "hunger": 100,
        "stats": Value::Null,
        "birthDate": crate::common::now_ms(),
    }));
    let stored = storage.len();
    rpg.insert("petStorage".into(), Value::Array(storage));

    let exp_reward = if is_rare { 500 } else { 200 };
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    let mut txt = String::from("AWWW!! ADA KELAHIRAN BARU! 🍼✨\n\n");
    if is_rare {
        txt.push_str("🎉 *LUCKY!! KETURUNAN LANGKA!!* 🎉\n");
    }
    txt.push_str(&format!(
        "Selamat! Kamu berhasil menetaskan bayi:\n\
         🐣 Spesies: *{}*\n\n\
         Dapat EXP *+{exp_reward}*\n\
         Biaya Bersalin: *Rp -3.000*\n\n\
         *(Bayi peliharaanmu disimpan ke dalam Pet Storage. Total simpananmu: {stored} ekor)*",
        pet_name(baby).unwrap_or("Baby")
    ));

    Reply::Text {
        text: txt,
        mentions: vec![ctx.sender.clone(), target],
    }
    .with_reaction(if is_rare { "🎉" } else { "✅" })
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": koin - BREEDING_COST, "exp": new_exp, "rpg": rpg }),
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn ctx_for(mentioned: &[&str], partner: Option<Value>, user: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "breeding".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@g.uset".into(),
            text: mentioned.join(" "),
            args: mentioned.iter().map(|s| (*s).to_string()).collect(),
            mentioned: mentioned.iter().map(|s| (*s).to_string()).collect(),
            is_group: true,
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: "628222@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: partner.unwrap_or(Value::Null),
                }],
                is_bot_admin: false,
            },
            user_data: user,
            ..Default::default()
        }
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("breeding").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn ready(tipe: &str, level: i64) -> Value {
        serde_json::json!({ "rpg": { "pet": { "type": tipe, "level": level } } })
    }

    fn write_of(r: &Reply) -> Value {
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
        let p = oktz_plugin_api::find("breeding").unwrap();
        assert_eq!(p.meta.name, "breeding");
        assert_eq!(p.meta.alias, ["breed", "kawin", "petbreed"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Breeding pets untuk mendapat pet baru");
        assert_eq!(p.meta.usage, ".breeding @user");
        assert_eq!(p.meta.example, ".breeding @user");
        assert_eq!(p.meta.cooldown, 3600);
        assert_eq!(p.meta.energi, 3);
        assert!(p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["breed", "kawin", "petbreed"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "breeding");
        }
    }

    #[test]
    fn without_a_target_the_how_to_is_shown_and_nothing_is_written() {
        let r = run(ctx_for(&[], None, serde_json::json!({ "koin": 99_999 })));
        let t = text_of(&r);
        assert!(t.starts_with("💕 *Peternakan & Kawin Silang*"), "{t}");
        assert!(t.contains("Biaya persalinan: *Rp 3.000*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn breeding_with_yourself_is_refused() {
        let r = run(ctx_for(
            &["628111@s.whatsapp.net"],
            Some(ready("dog", 9)),
            serde_json::json!({ "rpg": { "pet": { "type": "cat", "level": 9 } }, "koin": 99_999 }),
        ));
        assert!(text_of(&r).starts_with("Hayo, mau kawin sama diri sendiri"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_caller_must_own_a_pet_first() {
        // `{}` truthy di JS — pet tanpa `level` tetap "punya pet".
        let pet = serde_json::json!(null);
        let r = run(ctx_for(
            &["628222@s.whatsapp.net"],
            Some(ready("dog", 9)),
            serde_json::json!({ "rpg": { "pet": pet }, "koin": 99_999 }),
        ));
        assert!(text_of(&r).starts_with("Kamu aja belum punya peliharaan"), "pet {pet}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_partner_must_own_a_pet_too() {
        for partner in [None, Some(serde_json::json!({})), Some(serde_json::json!({ "rpg": {} }))] {
            let r = run(ctx_for(
                &["628222@s.whatsapp.net"],
                partner.clone(),
                serde_json::json!({ "rpg": { "pet": { "type": "cat", "level": 9 } }, "koin": 99_999 }),
            ));
            assert!(
                text_of(&r).starts_with("Target yang kamu tag ternyata nggak punya peliharaan"),
                "partner {partner:?}"
            );
            assert!(r.mutations().is_empty(), "partner {partner:?}");
        }
    }

    /// `if (!user.rpg.pet)` — `{}` di JS itu TRUTHY, jadi pet tanpa `level`
    /// tetap dianggap ada (dan `level || 1` jadi 1).
    #[test]
    fn both_pets_must_reach_level_five() {
        for (mine, theirs) in [(4, 9), (9, 4), (1, 1)] {
            let r = run(ctx_for(
                &["628222@s.whatsapp.net"],
                Some(ready("dog", theirs)),
                serde_json::json!({ "rpg": { "pet": { "type": "cat", "level": mine } }, "koin": 99_999 }),
            ));
            let t = text_of(&r);
            assert!(
                t.contains("🐣"),
                "level saya {mine} / pasangan {theirs}: {t}"
            );
            assert!(r.mutations().is_empty(), "level saya {mine} / pasangan {theirs}");
        }
    }

    #[test]
    fn a_pet_without_a_level_field_reads_as_level_one_and_is_refused() {
        let r = run(ctx_for(
            &["628222@s.whatsapp.net"],
            Some(ready("dog", 9)),
            serde_json::json!({ "rpg": { "pet": { "type": "cat" } }, "koin": 99_999 }),
        ));
        assert!(text_of(&r).contains("Sekarang Level 1"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_fee_is_charged_and_the_baby_lands_in_pet_storage() {
        let mut rare = 0;
        let mut common = 0;
        for _ in 0..400 {
            let r = run(ctx_for(
                &["628222@s.whatsapp.net"],
                Some(ready("cat", 9)),
                serde_json::json!({ "rpg": { "pet": { "type": "cat", "level": 9 } }, "koin": 10_000, "exp": 0 }),
            ));
            let t = text_of(&r);
            assert!(t.starts_with("AWWW!! ADA KELAHIRAN BARU!"), "{t}");
            let data = write_of(&r);
            assert_eq!(data["koin"], serde_json::json!(7_000), "10.000 - 3.000");
            let babies = data["rpg"]["petStorage"].as_array().unwrap();
            let baby = &babies[0];
            assert_eq!(baby["level"], serde_json::json!(1));
            assert_eq!(baby["exp"], serde_json::json!(0));
            assert_eq!(baby["hunger"], serde_json::json!(100));
            assert_eq!(baby["stats"], Value::Null);
            assert!(baby["birthDate"].as_i64().unwrap() > 0);
            let exp_reward = data["exp"].as_i64().unwrap();
            if t.contains("🎉 *LUCKY!! KETURUNAN LANGKA!!*") {
                assert_eq!(exp_reward, 500);
                rare += 1;
            } else {
                assert_eq!(exp_reward, 200);
                common += 1;
            }
            assert!(t.contains(&format!("Total simpananmu: {} ekor", babies.len())));
        }
        assert!(rare > 0 && common > 0, "rare={rare} common={common}");
    }

    #[test]
    fn a_second_breeding_appends_to_the_existing_storage() {
        let r = run(ctx_for(
            &["628222@s.whatsapp.net"],
            Some(ready("dog", 9)),
            serde_json::json!({ "rpg": { "pet": { "type": "dog", "level": 9 },
                             "petStorage": [{ "type": "cat", "name": "Miko", "level": 4 }] },
                   "koin": 99_999, "exp": 0 }),
        ));
        let babies = write_of(&r)["rpg"]["petStorage"].as_array().unwrap().clone();
        assert_eq!(babies.len(), 2);
        assert_eq!(babies[0]["name"], serde_json::json!("Miko"), "baby lama tak boleh tertimpa");
        assert_eq!(babies[1]["level"], serde_json::json!(1));
    }

    #[test]
    fn an_empty_purse_cannot_pay_the_vet() {
        for koin in [0, 2_999] {
            let r = run(ctx_for(
                &["628222@s.whatsapp.net"],
                Some(ready("dog", 9)),
                serde_json::json!({ "rpg": { "pet": { "type": "cat", "level": 9 } }, "koin": koin }),
            ));
            assert!(text_of(&r).starts_with("Uang kamu nggak cukup"), "koin {koin}");
            assert!(r.mutations().is_empty(), "koin {koin}");
        }
    }

    #[test]
    fn the_tables_keep_every_js_row() {
        assert_eq!(BREEDING_RESULTS.len(), 9);
        assert_eq!(PET_NAMES.len(), 10);
        assert_eq!(pet_name("thunderbunny"), Some("⚡ Thunder Bunny"));
        assert_eq!(pet_name("naga"), None);
        assert_eq!(baby_name("dragon"), "Naga");
        assert_eq!(baby_name("naga"), "Baby");
    }

    #[test]
    fn the_breed_key_is_sorted_so_order_does_not_matter() {
        assert_eq!(breed_key("dog", "cat"), "cat+dog");
        assert_eq!(breed_key("cat", "dog"), "cat+dog");
        assert_eq!(breed_key("rabbit", "rabbit"), "rabbit+rabbit");
        // Pasangan tak dikenal jatuh ke `default`.
        assert!(["cat", "dog", "bird", "fish", "rabbit"].contains(&offspring("zebra+zebra")));
    }
}