//! `plugins/rpg/jualan.js` — dagang asongan keliling.
//!
//! Berkas ini sekaligus jadi tempat helper RPG untuk 27 plugin lain di batch
//! ini (`crate::rpg::jualan::*`), karena `common.rs` milik agent lain tidak
//! boleh disentuh.
//!
//! Yang tidak ikut ke port: reaksi + pesan antara sebelum jeda `setTimeout`
//! (`Reply` hanya bisa satu pesan, jadi yang dikirim hasil akhir beserta reaksi
//! TERAKHIR), dan kartu level-up dari `checkAndNotifyLevelUp` yang butuh canvas.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::int_field;
use crate::tools::kalkulatormbg::thousands_id;

/// `MAX_EXP` (`level.js:226`, `database.js:668`).
pub(crate) const MAX_EXP: i64 = 9_000_000_000;

/// `Math.random()` — `[0, 1)`.
pub(crate) fn math_random() -> f64 {
    use rand::Rng as _;
    rand::rng().random_range(0.0..1.0)
}

/// `Math.floor(Math.random() * (max - min + 1)) + min` — inklusif kedua ujung.
pub(crate) fn roll_incl(min: i64, max: i64) -> i64 {
    min + crate::common::roll((max - min + 1) as u32) as i64
}

/// `Math.floor(Math.random() * (max - min)) + min` — ujung atas eksklusif:
/// `max` tidak akan keluar (`steal.js:84`, `treasure.js:110`, `use.js`).
pub(crate) fn roll_excl(min: i64, max: i64) -> i64 {
    min + crate::common::roll((max - min) as u32) as i64
}

/// `x ?? d` — `0` tetap `0`; hanya null/undefined (dan bukan angka) yang
/// jatuh ke `d`.
pub(crate) fn coalesce(v: Option<i64>, d: i64) -> i64 {
    v.unwrap_or(d)
}

/// `x || d` — `0` falsy di JS, jadi nol ikut diganti `d`. `mining.js`,
/// `work.js`, dan `woodcut.js` memakainya untuk stamina: nol berarti "belum
/// pernah pakai", bukan "kehabisan".
pub(crate) fn or_default(v: Option<i64>, d: i64) -> i64 {
    match v {
        Some(x) if x != 0 => x,
        _ => d,
    }
}

/// `if (!user.rpg) user.rpg = {}` — objek `rpg` siap diubah.
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

/// `Number.prototype.toLocaleString()` tanpa argumen locale. Default runtime
/// Node adalah `en-US`, jadi pemisah ribuan koma (`5000` → `5,000`) — beda
/// dari `toLocaleString("id-ID")` yang memakai titik.
pub(crate) fn thousands_en(n: i64) -> String {
    thousands_id(n as f64).replace('.', ",")
}

/// `db.updateExp` (`database.js:664-672`) — TANPA cek level, jadi `rpg` tidak
/// ikut disentuh meski level naik.
pub(crate) fn exp_gain_only(old_exp: i64, gain: i64) -> i64 {
    if old_exp == -1 {
        -1
    } else {
        (old_exp + gain).clamp(0, MAX_EXP)
    }
}

/// `addExpWithLevelCheck` (`level.js:217-240`) minus kartu level-up (canvas +
/// `sock.sendMedia`, tidak tersedia). Panggil SESUDAH plugin mengurangi
/// stamina: `checkAndNotifyLevelUp` menimpa `stamina`/`health`/`mana` ke
/// maksimum baru kalau level naik.
pub(crate) fn apply_exp(rpg: &mut Map<String, Value>, old_exp: i64, gain: i64) -> i64 {
    let new_exp = exp_gain_only(old_exp, gain);
    let new_level = crate::user::exp::calculate_level(new_exp);
    if new_level > crate::user::exp::calculate_level(old_exp) {
        rpg.insert("level".into(), json!(new_level));
        rpg.insert("maxHealth".into(), json!(100 + (new_level - 1) * 10));
        rpg.insert("maxMana".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("maxStamina".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("health".into(), json!(100 + (new_level - 1) * 10));
        rpg.insert("mana".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("stamina".into(), json!(100 + (new_level - 1) * 5));
    }
    new_exp
}

/// Field numerik di dalam objek `rpg`.
pub(crate) fn rpg_int(rpg: &Map<String, Value>, key: &str) -> Option<i64> {
    rpg.get(key).and_then(Value::as_i64)
}

/// `db.getUser(jid)` untuk JID orang lain, di luar `sender` — satu-satunya
/// bar yang tersedia adalah `Participant.data` dari roster grup.
///
/// `ponytail:` `Ctx` tidak punya pembacaan store untuk JID sembarang, jadi di
/// private chat (atau mention ke non-anggota) bar selalu `None` walau row-nya
/// ada. Tambahkan `user_entry(jid)` ke `Ctx` kalau router mau menutup celah ini;
/// sementara ini plugin yang butuh row target (`marry`, `transfer`) treating
/// `None` sebagai "belum pernah ada", sama seperti `db.getUser` yang mengembalikan
/// objek kosong lewat `db.setUser(target)`.
pub(crate) fn roster_row<'a>(ctx: &'a Ctx, jid: &str) -> Option<&'a Value> {
    ctx.roster
        .participants
        .iter()
        .find(|p| p.jid == jid)
        .map(|p| &p.data)
}

/// Objek `rpg` milik JID lain; objek kosong kalau row-nya tidak terbaca.
pub(crate) fn roster_rpg(ctx: &Ctx, jid: &str) -> Map<String, Value> {
    roster_row(ctx, jid)
        .and_then(|u| u.get("rpg"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Objek `inventory` milik JID lain; objek kosong kalau row-nya tidak terbaca.
pub(crate) fn roster_inventory(ctx: &Ctx, jid: &str) -> Map<String, Value> {
    roster_row(ctx, jid)
        .and_then(|u| u.get("inventory"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// `x.split("@")[0]`.
pub(crate) fn jid_name(jid: &str) -> &str {
    jid.split('@').next().unwrap_or(jid)
}

pub(crate) const STAMINA_COST: i64 = 18;

#[plugin(
    name = "jualan",
    alias = ["dagang", "pedagang"],
    category = "rpg",
    description = "Dagang asongan keliling",
    usage = ".jualan",
    example = ".jualan",
    cooldown = 180,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn jualan(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Kaki lemes kebanyakan keliling! 🥵\n\n\
             Jualan butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Ngiyup bentar! 🏖️"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.2 {
        // Cabang rugi: TIDAK memanggil `addExpWithLevelCheck`, jadi exp utuh.
        let rugi = roll_incl(5000, 14999);
        let koin = (koin - rugi).max(0);
        return Reply::text(format!(
            "HUJAN DERAS! DAGANGAN SEPI! 🌧️🥶\n\n\
             Nggak ada yang beli dan dagangan kerupukmu melempem semua.\n\
             Kerugian Modal: *Rp {}*\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Besok harus liat prakiraan cuaca nih! ☂️",
            thousands_id(rugi as f64)
        ))
        .with_reaction("🌧️")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin, "rpg": rpg }),
        });
    }

    let (earning, head, react) = if gacha > 0.85 {
        let laku = roll_incl(40000, 119999);
        (
            laku,
            "ADA BROMTON LEWAT DIBORONG SEMUA! 🚴‍♂️✨\n\n\
             Rombongan sepeda elit mampir dan ngeborong semua isotonik sama cangcimen!\n",
            "🤑",
        )
    } else {
        let earning = roll_incl(10000, 34999);
        (
            earning,
            "DAGANGAN LAKU STANDAR! 🛒✨\n\n",
            "✅",
        )
    };
    let exp_gain = earning / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "{head}\
         💵 Omzet: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}",
        thousands_id(earning as f64)
    ))
    .with_reaction(react)
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin + earning, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "jualan".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("jualan").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("jualan").unwrap();
        assert_eq!(p.meta.name, "jualan");
        assert_eq!(p.meta.alias, ["dagang", "pedagang"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Dagang asongan keliling");
        assert_eq!(p.meta.usage, ".jualan");
        assert_eq!(p.meta.example, ".jualan");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dagang", "pedagang"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "jualan");
        }
    }

    #[test]
    fn zero_stamina_is_still_zero_because_the_js_guard_uses_nullish_coalescing() {
        // `user.rpg.stamina ?? 100` — unlike `|| 100`, 0 survives, so a user
        // who spent all stamina really is told off.
        let r = run(json!({"rpg": {"stamina": 0}}));
        let t = text_of(&r);
        assert!(t.starts_with("Kaki lemes kebanyakan keliling!"), "{t}");
        assert!(t.contains("sisa stamina kamu *0*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_user_without_rpg_row_starts_from_the_full_stamina_bar() {
        for user in [json!(null), json!({}), json!({"rpg": null}), json!({"rpg": {}})] {
            let r = run(user.clone());
            assert!(
                !text_of(&r).starts_with("Kaki lemes"),
                "stamina default 100 harus cukup: {user}"
            );
            assert_eq!(written(&r)["rpg"]["stamina"], json!(82), "input {user}");
        }
    }

    #[test]
    fn stamina_exactly_at_the_cost_is_allowed_and_drops_to_zero() {
        let r = run(json!({"rpg": {"stamina": 18}}));
        assert_eq!(written(&r)["rpg"]["stamina"], json!(0));
    }

    #[test]
    fn one_point_below_the_cost_is_refused() {
        let r = run(json!({"rpg": {"stamina": 17}}));
        assert!(text_of(&r).contains("sisa stamina kamu *17*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_successful_sale_always_costs_exactly_eighteen_stamina() {
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 1000, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(82));
        }
    }

    #[test]
    fn the_two_win_branches_land_in_the_documented_ranges() {
        let mut seen_brotmon = false;
        let mut seen_standar = false;
        for _ in 0..2000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            let koin = d["koin"].as_i64().unwrap();
            let t = text_of(&r);
            if t.starts_with("HUJAN DERAS") {
                continue;
            }
            if t.contains("BROMTON") {
                seen_brotmon = true;
                assert!((40000..=119999).contains(&koin), "omzet {koin}");
                assert_eq!(d["exp"].as_i64().unwrap(), koin / 20);
            } else {
                seen_standar = true;
                assert!((10000..=34999).contains(&koin), "omzet {koin}");
                assert_eq!(d["exp"].as_i64().unwrap(), koin / 20);
            }
        }
        assert!(seen_brotmon && seen_standar, "kedua cabang harus muncul");
    }

    #[test]
    fn the_rain_branch_takes_money_but_never_gives_exp() {
        let mut seen = false;
        for _ in 0..3000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 20000, "exp": 5000}));
            let d = written(&r);
            if text_of(&r).starts_with("HUJAN DERAS") {
                seen = true;
                assert!(d.get("exp").is_none(), "cabang rugi tidak menyentuh exp");
                assert_eq!(d["rpg"]["stamina"], json!(82));
                assert!(d["koin"].as_i64().unwrap() <= 15000, "koin {}", d["koin"]);
                break;
            }
        }
        assert!(seen, "gacha < 0.2 harus pernah terjadi dalam 3000 sampel");
    }

    #[test]
    fn losses_never_drive_the_balance_below_zero() {
        // `Math.max(0, koin - rugi)` — koin 0 dan rugi besar tetap 0, bukan
        // negatif.
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert!(written(&r)["koin"].as_i64().unwrap() >= 0);
        }
    }

    #[test]
    fn an_infinite_exp_user_stays_infinite() {
        // `user.exp !== -1` jadi -1 berarti "exp dimatikan", bukan nol.
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": -1}));
            if let Some(exp) = written(&r).get("exp") {
                assert_eq!(exp, &json!(-1), "exp -1 harus tetap -1");
            }
        }
    }

    #[test]
    fn a_level_crossing_overwrites_the_stamina_we_just_spent() {
        // 9999 EXP -> 10000 = level 2. `checkAndNotifyLevelUp` mengisi ulang
        // stamina ke maxStamina = 105, menimpa pengurangan 18.
        let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 9999}));
        let d = written(&r);
        if d.get("exp").and_then(Value::as_i64).is_some_and(|e| e > 9999) {
            assert_eq!(d["rpg"]["level"], json!(2));
            assert_eq!(d["rpg"]["maxStamina"], json!(105));
            assert_eq!(d["rpg"]["stamina"], json!(105));
            assert_eq!(d["rpg"]["maxHealth"], json!(110));
        }
    }

    #[test]
    fn the_helper_contracts_hold() {
        assert_eq!(coalesce(Some(0), 100), 0, "?? tidak mengganti nol");
        assert_eq!(coalesce(None, 100), 100);
        assert_eq!(or_default(Some(0), 100), 100, "|| mengganti nol");
        assert_eq!(or_default(None, 100), 100);
        assert_eq!(exp_gain_only(-1, 999), -1);
        assert_eq!(exp_gain_only(9_999_999_500, 1000), MAX_EXP);
        assert_eq!(exp_gain_only(100, -500), 0, "exp tidak boleh negatif");

        for _ in 0..200 {
            assert!((5000..=14999).contains(&roll_incl(5000, 14999)));
            assert!((50..150).contains(&roll_excl(50, 150)), "ujung atas eksklusif");
        }
    }
}
