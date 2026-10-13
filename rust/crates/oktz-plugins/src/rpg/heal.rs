//! `plugins/rpg/heal.js` — istirahat gratis untuk memulihkan HP + stamina.
//!
//! Berkas ini sekaligus jadi tempat helper RPG untuk 27 plugin lain di batch
//! ini (`crate::rpg::heal::*`), karena `common.rs` milik agent lain tidak
//! boleh disentuh.
//!
//! Yang tidak ikut ke port: reaksi 🛌 dan pesan "Tidur dulu ah..." sebelum
//! jeda `setTimeout` (`Reply` hanya bisa satu pesan), dan kartu level-up dari
//! `checkAndNotifyLevelUp` yang butuh canvas + `sock.sendMedia`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

/// `MAX_EXP` (`level.js:226`, `database.js:668`).
pub(crate) const MAX_EXP: i64 = 9_000_000_000;

/// `MAX_KOIN` (`database.js:650`) — hanya `gacha` yang lewat `db.updateKoin`.
pub(crate) const MAX_KOIN: i64 = 9_000_000_000_000;

/// `Math.random()` — `[0, 1)`.
pub(crate) fn math_random() -> f64 {
    use rand::Rng as _;
    rand::rng().random_range(0.0..1.0)
}

/// `Math.floor(Math.random() * (max - min + 1)) + min` — inklusif kedua ujung.
pub(crate) fn roll_incl(min: i64, max: i64) -> i64 {
    min + crate::common::roll((max - min + 1) as u32) as i64
}

/// `if (!user.rpg) user.rpg = {}` — objek `rpg` siap diubah lalu ditulis
/// balik utuh (penyimpanan `setUser` hanya merge di level atas).
pub(crate) fn rpg_map(ctx: &Ctx) -> Map<String, Value> {
    ctx.user_field("rpg")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// `if (!user.inventory) user.inventory = {}`.
pub(crate) fn inventory_map(ctx: &Ctx) -> Map<String, Value> {
    ctx.user_field("inventory")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Field numerik di dalam objek `rpg`.
pub(crate) fn rpg_int(rpg: &Map<String, Value>, key: &str) -> Option<i64> {
    rpg.get(key).and_then(Value::as_i64)
}

/// `x || d` — `0` falsy di JS, jadi nol ikut diganti `d`.
pub(crate) fn or_default(v: Option<i64>, d: i64) -> i64 {
    match v {
        Some(x) if x != 0 => x,
        _ => d,
    }
}

/// `x ?? d` — `0` tetap `0`; hanya null/undefined yang jatuh ke `d`.
pub(crate) fn coalesce(v: Option<i64>, d: i64) -> i64 {
    v.unwrap_or(d)
}

/// `Number.prototype.toLocaleString()` tanpa argumen locale.
///
/// Default locale runtime Node adalah `en-US`, jadi pemisah ribuan koma
/// (`5000` → `5,000`) — beda dari `toLocaleString("id-ID")` yang memakai
/// titik (`thousands_id`).
pub(crate) fn thousands_en(n: i64) -> String {
    crate::tools::kalkulatormbg::thousands_id(n as f64).replace('.', ",")
}

/// `db.updateExp` (`database.js:664`) — tanpa cek level, jadi `rpg` tidak ikut
/// disentuh meski level naik.
pub(crate) fn exp_gain_only(old_exp: i64, gain: i64) -> i64 {
    if old_exp == -1 {
        -1
    } else {
        (old_exp + gain).clamp(0, MAX_EXP)
    }
}

/// `addExpWithLevelCheck` (`level.js:217`) minus kartu level-up (canvas).
///
/// Panggil SESUDAH plugin mengurangi stamina: `checkAndNotifyLevelUp`
/// menimpa `health`/`mana`/`stamina` ke maksimum baru kalau level naik.
pub(crate) fn apply_exp(rpg: &mut Map<String, Value>, old_exp: i64, gain: i64) -> i64 {
    let new_exp = exp_gain_only(old_exp, gain);
    let old_level = crate::user::exp::calculate_level(old_exp);
    let new_level = crate::user::exp::calculate_level(new_exp);
    if new_level > old_level {
        let max_health = 100 + (new_level - 1) * 10;
        let max_mana = 100 + (new_level - 1) * 5;
        let max_stamina = 100 + (new_level - 1) * 5;
        rpg.insert("level".into(), json!(new_level));
        rpg.insert("maxHealth".into(), json!(max_health));
        rpg.insert("maxMana".into(), json!(max_mana));
        rpg.insert("maxStamina".into(), json!(max_stamina));
        rpg.insert("health".into(), json!(max_health));
        rpg.insert("mana".into(), json!(max_mana));
        rpg.insert("stamina".into(), json!(max_stamina));
    }
    new_exp
}

/// `user.level || 1` — field level ATAS (`database.js:570`), bukan
/// `rpg.level` yang ditulis `level.js:136`.
pub(crate) fn user_level(ctx: &Ctx) -> i64 {
    or_default(ctx.user_field("level").and_then(Value::as_i64), 1)
}

/// `db.getUser(jid)` untuk JID selain pengirim.
///
/// `Ctx` hanya membawa data sender; user lain hanya ada di roster grup
/// (`ctx.roster.participants[].data`). Di luar situ — termasuk private chat,
/// tempat roster selalu kosong — hasilnya `None`, sama seperti row yang tidak
/// pernah ada di store.
pub(crate) fn other_user<'a>(ctx: &'a Ctx, jid: &str) -> Option<&'a Value> {
    let number = crate::common::jid_number(jid);
    ctx.roster
        .participants
        .iter()
        .find(|p| p.jid == jid || crate::common::jid_number(&p.jid) == number)
        .map(|p| &p.data)
        .filter(|data| !data.is_null())
}

/// `user.rpg.<key>` milik user lain, `0` kalau tidak ada.
#[allow(dead_code)] // dipakai plugin rpg lain di batch ini
pub(crate) fn other_rpg_int(ctx: &Ctx, jid: &str, key: &str) -> i64 {
    other_user(ctx, jid)
        .and_then(|u| u.get("rpg"))
        .and_then(|r| r.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

/// `Math.floor(100 * Math.random())` — angka 0..=99 untuk membandingkan
/// dengan `chance` dalam persen.
pub(crate) fn roll_percent() -> i64 {
    crate::common::roll(100) as i64
}

/// `createGachaAnimation` (`gacha.js:132`) — satu frame acak dari daftar.
pub(crate) fn random_rare_frame() -> &'static str {
    const FRAMES: [&str; 5] = ["🎰", "💫", "✨", "🌟", "💥"];
    FRAMES[crate::common::random_index(FRAMES.len())]
}

#[plugin(
    name = "heal",
    alias = ["sembuh", "recover"],
    category = "rpg",
    description = "Pulihkan health dengan istirahat (gratis tapi lama)",
    usage = ".heal",
    example = ".heal",
    cooldown = 600,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn heal(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);

    let health = or_default(rpg_int(&rpg, "health"), 100);
    let max_health = or_default(rpg_int(&rpg, "maxHealth"), 100);
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    let max_stamina = or_default(rpg_int(&rpg, "maxStamina"), 100);

    if health >= max_health && stamina >= max_stamina {
        return Reply::text(
            "Eits, badan kamu masih seger bugar kak! 🏋️✨\n\
             Nggak usah istirahat dulu, mending lanjut petualang aja! 🚀",
        );
    }

    let new_health = (health + 30).min(max_health);
    let new_stamina = (stamina + 50).min(max_stamina);
    rpg.insert("health".into(), json!(new_health));
    rpg.insert("stamina".into(), json!(new_stamina));

    Reply::text(format!(
        "Hooammm... seger banget abis tidur! 🥱🌞\n\n\
         Status kamu udah pulih nih:\n\
         ❤️ Health: {health} 📈 *{new_health}*\n\
         ⚡ Stamina: {stamina} 📈 *{new_stamina}*\n\n\
         Kalo males nunggu istirahat, kamu bisa beli potion di `.shop` dan ketik `.use potion` buat heal instant ya! 🥤💖"
    ))
    .with_reaction("🛌")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "heal".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("heal").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn rpg_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data["rpg"].clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("heal").unwrap();
        assert_eq!(p.meta.name, "heal");
        assert_eq!(p.meta.alias, ["sembuh", "recover"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Pulihkan health dengan istirahat (gratis tapi lama)"
        );
        assert_eq!(p.meta.usage, ".heal");
        assert_eq!(p.meta.example, ".heal");
        assert_eq!(p.meta.cooldown, 600);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["sembuh", "recover"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "heal");
        }
    }

    /// `if (health >= maxHealth && stamina >= maxStamina)` — dua syarat, jadi
    /// salah satu saja belum pulih tetap boleh istirahat.
    #[test]
    fn a_body_that_is_only_half_recovered_still_gets_a_nap() {
        let r = run(json!({"rpg": {"health": 100, "maxHealth": 100,
                                   "stamina": 20, "maxStamina": 100}}));
        assert!(text_of(&r).starts_with("Hooammm"), "{}", text_of(&r));
        assert_eq!(rpg_of(&r)["stamina"], json!(70));
    }

    #[test]
    fn a_fully_recovered_body_is_refused_and_nothing_is_written() {
        for user in [
            json!(null),
            json!({}),
            json!({"rpg": {"health": 100, "maxHealth": 100,
                           "stamina": 100, "maxStamina": 100}}),
        ] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("Eits, badan kamu masih seger"), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn recovery_is_capped_at_the_stored_maximum() {
        // maxHealth 500 tapi health 480: +30 tidak boleh melewati 500.
        let r = run(json!({"rpg": {"health": 480, "maxHealth": 500,
                                   "stamina": 90, "maxStamina": 100}}));
        let rpg = rpg_of(&r);
        assert_eq!(rpg["health"], json!(500));
        assert_eq!(rpg["stamina"], json!(100));
    }

    #[test]
    fn the_reply_names_the_old_and_the_new_value() {
        let r = run(json!({"rpg": {"health": 40, "maxHealth": 100,
                                   "stamina": 30, "maxStamina": 100}}));
        let t = text_of(&r);
        assert!(t.contains("❤️ Health: 40 📈 *70*"), "{t}");
        assert!(t.contains("⚡ Stamina: 30 📈 *80*"), "{t}");
    }

    #[test]
    fn zero_valued_stats_fall_back_to_one_hundred_because_of_or() {
        // `user.rpg.health || 100` — 0 bukan berarti habis, tapi belum pernah
        // diisi juga bukan; JS menimpanya dengan 100.
        let r = run(json!({"rpg": {"health": 0, "stamina": 0}}));
        assert!(
            r.mutations().is_empty(),
            "0 -> 100 di kedua sisi, jadi tubuh dianggap seger"
        );
    }

    #[test]
    fn unrelated_rpg_fields_survive_the_write() {
        let r = run(json!({"rpg": {"bank": 5000, "pet": {"type": "dog"},
                                   "health": 10}}));
        let rpg = rpg_of(&r);
        assert_eq!(rpg["bank"], json!(5000));
        assert_eq!(rpg["pet"], json!({"type": "dog"}));
    }

    // -- helper bersama batch ini ------------------------------------------

    #[test]
    fn apply_exp_only_rewrites_stats_when_the_level_actually_moves() {
        let mut rpg = Map::new();
        rpg.insert("stamina".into(), json!(40));
        let same = apply_exp(&mut rpg, 0, 500);
        assert_eq!(same, 500);
        assert_eq!(rpg.len(), 1, "level belum naik -> stats tak boleh ditimpa");
        assert_eq!(rpg["stamina"], json!(40));

        let mut rpg = Map::new();
        let crossed = apply_exp(&mut rpg, 9_000, 2_000);
        assert_eq!(crossed, 11_000);
        assert_eq!(rpg["level"], json!(2));
        assert_eq!(rpg["maxHealth"], json!(110));
        assert_eq!(rpg["health"], json!(110));
        assert_eq!(rpg["maxStamina"], json!(105));
    }

    #[test]
    fn exp_minus_one_means_unlimited_and_never_moves() {
        assert_eq!(exp_gain_only(-1, 5_000), -1);
        assert_eq!(apply_exp(&mut Map::new(), -1, 5_000), -1);
    }

    #[test]
    fn the_percent_roll_is_inclusive_on_both_ends() {
        for _ in 0..200 {
            assert!((0..=99).contains(&roll_percent()));
        }
    }

    #[test]
    fn another_user_is_only_readable_through_the_roster() {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![oktz_plugin_api::registry::Participant {
                    jid: "628222@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: json!({"koin": 42, "rpg": {"health": 55}}),
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        assert_eq!(other_rpg_int(&ctx, "628222@s.whatsapp.net", "health"), 55);
        assert_eq!(
            other_user(&ctx, "628222@s.whatsapp.net").unwrap()["koin"],
            json!(42)
        );
        assert!(other_user(&ctx, "628999@s.whatsapp.net").is_none());
    }
}