//! `plugins/clan/clanwar.js` — clanvs clan: power, hasil acak, hadiah.
//!
//! `ponytail:` JS memanggil `db.getUser(jid)` untuk SETIAP member kedua clan
//! (power, lalu `updateKoin`/`updateExp`/`updateEnergi`). Baris store milik
//! orang lain tidak bisa dibaca dari `Ctx` — yang tersedia cuma baris sender
//! (`ctx.user_data`) dan baris peserta grup yang sudah dimuat
//! (`ctx.roster.participants[].data`), sama seperti yang dicatat di
//! `clanmembers.rs`. Member di luar dua sumber itu dihitung dengan default JS
//! (level 1, exp 0) untuk power, dan TIDAK bisa diberi hadiah karena nilai
//! lama `koin`/`exp`/`energi`-nya tidak diketahui. Jumlah yang terlewat
//! disebut di balasan supaya hadiah hilang tidak senyap.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::clan::clancreate::{add_koin, clans, emblem, field_num, field_str, find_clan_entry, members, save_clans};
use crate::clan::clanmembers::{level_of, user_row};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const KOIN_WIN: f64 = 30_000.0;
pub(crate) const KOIN_LOSE: f64 = 6_000.0;
pub(crate) const EXP_WIN: f64 = 15_000.0;
pub(crate) const EXP_LOSE: f64 = 3_000.0;
pub(crate) const ENERGI_WIN: f64 = 15.0;
pub(crate) const ENERGI_LOSE: f64 = 3.0;
pub(crate) const CLAN_EXP_WIN: f64 = 5_000.0;
pub(crate) const CLAN_EXP_LOSE: f64 = 1_000.0;

/// `updateExp` clamp: `MAX_EXP = 9000000000` (`database.js:663`).
pub(crate) const MAX_EXP: i64 = 9_000_000_000;
/// `level = Math.floor(exp / 10000) + 1` (`clanwar.js:139`).
pub(crate) const EXP_PER_LEVEL: f64 = 10_000.0;
pub(crate) const MIN_MEMBERS: usize = 3;

/// Hadiah mentahnya, sudah diskalakan per level clan lawan.
pub(crate) struct Rewards {
    pub koin: f64,
    pub exp: f64,
    pub energi: f64,
}

/// `getScaledRewards(clan)` — `mult = 1 + level * 0.1`, tiap angka di-floor.
pub(crate) fn scaled_rewards(clan: &Value, win: bool) -> Rewards {
    let level = {
        let l = field_num(clan, "level");
        if l == 0.0 { 1.0 } else { l }
    };
    let mult = 1.0 + level * 0.1;
    let (koin, exp, energi) = if win {
        (KOIN_WIN, EXP_WIN, ENERGI_WIN)
    } else {
        (KOIN_LOSE, EXP_LOSE, ENERGI_LOSE)
    };
    Rewards {
        koin: (koin * mult).floor(),
        exp: (exp * mult).floor(),
        energi: (energi * mult).floor(),
    }
}

/// `user?.rpg?.exp || user?.exp || 0` — `rpg.exp` menang, nilai falsy jatuh
/// ke `exp` biasa lalu ke 0.
pub(crate) fn exp_of(user: &Value) -> f64 {
    let rpg = user
        .get("rpg")
        .and_then(|r| r.get("exp"))
        .filter(|v| crate::common::js_truthy(Some(v)))
        .and_then(Value::as_f64);
    let plain = user.get("exp").and_then(Value::as_f64);
    rpg.or(plain).unwrap_or(0.0)
}

/// `calculatePower(db, clan)`, di-floor di akhir.
pub(crate) fn calculate_power(ctx: &Ctx, clan: &Value) -> f64 {
    let mut total = 0.0f64;
    for jid in members(clan) {
        // Baris yang tak terbaca → default JS, bukan 0: clan minimal 3 member
        // supaya ini tidak pernah menghasilkan power negatif.
        let (level, exp) = match user_row(ctx, &jid) {
            Some(u) => (level_of(u) as f64, exp_of(u)),
            None => (1.0, 0.0),
        };
        total += (level * 100.0) + (exp / 10.0);
    }
    let level = field_num(clan, "level");
    let level = if level == 0.0 { 1.0 } else { level };
    let wins = field_num(clan, "wins");
    let wins = if wins == 0.0 { 0.0 } else { wins };
    total += level * 500.0 + wins * 50.0;
    total.floor()
}

/// `Math.random() < power1 / total` — `Math.random()` seragam di `[0,1)`, jadi
/// undian memakai 1 juta undian seragam dan dibandingkan sebagai pecahan.
/// `total` tidak pernah 0 (level >= 1 menambah 500), tapi tetap dijaga.
pub(crate) fn simulate_war(power1: f64, power2: f64) -> bool {
    let total = power1 + power2;
    if total <= 0.0 {
        return false;
    }
    const TICKS: f64 = 1_000_000.0;
    (crate::common::random_index(1_000_000) as f64) / TICKS < power1 / total
}

/// `Math.round((p1 / total) * 10)` lalu 10 kotak hijau + merah.
pub(crate) fn power_bar(p1: f64, p2: f64) -> String {
    let total = p1 + p2;
    let ratio = if total <= 0.0 {
        0
    } else {
        ((p1 / total) * 10.0).round().clamp(0.0, 10.0) as usize
    };
    "🟩".repeat(ratio) + &"🟥".repeat(10 - ratio)
}

/// `Math.floor(exp / 10000) + 1`.
pub(crate) fn level_from_exp(exp: f64) -> i64 {
    (exp / EXP_PER_LEVEL).floor() as i64 + 1
}

pub(crate) const HELP: &str = "⚔️ *CLAN WAR*\n\n\
     Tantang clan lain untuk berperang!\n\n\
     Contoh: *.clanwar clan_123456*\n\
     Cek ID: *.clanleaderboard*\n\n\
     Syarat: Minimal 3 member per clan\n\
     Cooldown: 1 jam";

pub(crate) fn result_text(
    my: &Value,
    enemy: &Value,
    my_power: f64,
    enemy_power: f64,
    is_win: bool,
    mine: &Rewards,
    skipped: usize,
) -> String {
    let (my_e, en_e) = (emblem(my), emblem(enemy));
    let winner = if is_win { my } else { enemy };
    let winner_e = if is_win { &my_e } else { &en_e };
    let mut txt = format!(
        "⚔️ *WAR RESULT*\n\n\
         {my_e} *{}*  vs  *{}* {en_e}\n\
         💪 {}  vs  {}\n\
         {}\n\n\
         {winner_e} *{} WINS!*\n\n",
        field_str(my, "name"),
        field_str(enemy, "name"),
        thousands_id(my_power),
        thousands_id(enemy_power),
        power_bar(my_power, enemy_power),
        field_str(winner, "name"),
    );
    if is_win {
        txt.push_str("🎁 Reward per member:\n");
        txt.push_str(&format!(
            "+Rp {} · +{} EXP · +{} Energi\n",
            thousands_id(mine.koin),
            thousands_id(mine.exp),
            thousands_id(mine.energi)
        ));
        txt.push_str(&format!("+{} Clan EXP", thousands_id(CLAN_EXP_WIN)));
    } else {
        txt.push_str("😔 Konsolasi per member:\n");
        txt.push_str(&format!(
            "+Rp {} · +{} EXP · +{} Energi\n",
            thousands_id(mine.koin),
            thousands_id(mine.exp),
            thousands_id(mine.energi)
        ));
        txt.push_str(&format!("+{} Clan EXP", thousands_id(CLAN_EXP_LOSE)));
    }
    if skipped > 0 {
        txt.push_str(&format!(
            "\n\n⚠️ {skipped} member tidak dapat diberi hadiah: baris store-nya \
             tidak terbaca dari `Ctx`."
        ));
    }
    txt
}

/// `db.updateKoin/updateExp/updateEnergi` untuk satu member.
///
/// `koin`/`exp`/`energi` bernilai `-1` adalah sentinel "tidak pernah
/// dibatasi" di `database.js`; kalau sudah `-1`, fungsi JS mengembalikan
/// `-1` tanpa menulis apa pun. Nilai `-1` karena alasan lain (owner/premium)
/// tidak bisa dievaluasi di sini — `config` tidak terbuka lewat `Ctx`.
///
/// `None` = baris store-nya tidak terbaca, jadi tidak boleh ada mutasi.
fn reward_member(ctx: &Ctx, jid: &str, r: &Rewards) -> Option<Mutation> {
    let user = user_row(ctx, jid)?;
    let koin = user.get("koin").and_then(Value::as_i64).unwrap_or(0);
    let exp = user.get("exp").and_then(Value::as_i64).unwrap_or(0);
    let energi = user.get("energi").and_then(Value::as_i64).unwrap_or(0);
    if koin == -1 || exp == -1 || energi == -1 {
        return None;
    }
    Some(Mutation::SetUser {
        jid: jid.to_string(),
        data: json!({
            "koin": add_koin(koin, r.koin as i64),
            "exp": (exp + r.exp as i64).clamp(0, MAX_EXP),
            "energi": (energi + r.energi as i64).max(0),
        }),
    })
}

/// `clan.wins || 0` sebagai bulat. Semua penghitung clan di JS berupa
/// bilangan bulat, jadi hasilnya ditulis sebagai integer JSON — `json!(1.0)`
/// akan jadi `1.0` dan plugin lain yang baca `as_i64` ikut salah.
fn field_int(clan: &Value, key: &str) -> i64 {
    let v = field_num(clan, key);
    if v.is_finite() { v as i64 } else { 0 }
}

fn bump(clan: &mut Value, key: &str, add: i64) {
    let cur = field_int(clan, key);
    if let Some(o) = clan.as_object_mut() {
        o.insert(key.to_string(), json!(cur + add));
    }
}

fn set_exp(clan: &mut Value, add: i64) {
    let exp = field_int(clan, "exp") + add;
    if let Some(o) = clan.as_object_mut() {
        o.insert("exp".into(), json!(exp));
        o.insert("level".into(), json!(level_from_exp(exp as f64)));
    }
}

/// Hitung ulang clan yang berubah kalah/menang: `wins`/`losses` naik satu,
/// `exp` dapat `clanExp*`, `level` dihitung ulang dari `exp` — bukan ditambah.
fn settle(clan: &mut Value, win: bool) {
    if win {
        bump(clan, "wins", 1);
        set_exp(clan, CLAN_EXP_WIN as i64);
    } else {
        bump(clan, "losses", 1);
        set_exp(clan, CLAN_EXP_LOSE as i64);
    }
}

#[plugin(
    name = "clanwar",
    alias = ["war", "guildwar"],
    category = "clan",
    description = "War melawan clan lain",
    usage = ".clanwar <clan_id>",
    example = ".clanwar clan_123456",
    cooldown = 3600,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clanwar(ctx: Ctx) -> Reply {
    let my_clan_id = ctx
        .user_field("clanId")
        .filter(|v| crate::common::js_truthy(Some(v)))
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(my_clan_id) = my_clan_id else {
        return Reply::text("❌ Kamu belum punya clan");
    };

    // JS: `m.text?.trim()` — SELURUH teks, bukan `args[0]`.
    let target = ctx.text.trim().to_string();
    if target.is_empty() {
        return Reply::text(HELP);
    }

    let mut all: Map<String, Value> = clans(&ctx);
    let Some((my_key, my_clan)) = all.get_key_value(&my_clan_id).map(|(k, v)| (k.clone(), v.clone())) else {
        return Reply::text("❌ Clan kamu tidak ditemukan");
    };
    // `enemyKey` bisa sama dengan `myKey`: `target` yang cocok lewat nama atau
    // `clan.id` (bukan lewat kunci peta) lolos dari cek "war melawan diri
    // sendiri" di JS. Kasus itu ditangani di bawah, bukan dicegah.
    let Some((enemy_key, enemy)) = find_clan_entry(&all, &target) else {
        return Reply::text("❌ Clan lawan tidak ditemukan");
    };
    let (enemy_key, enemy) = (enemy_key.clone(), enemy.clone());

    if my_clan_id == target {
        return Reply::text("❌ Tidak bisa war melawan clan sendiri");
    }
    if members(&my_clan).len() < MIN_MEMBERS {
        return Reply::text("❌ Clan kamu butuh minimal 3 member");
    }
    if members(&enemy).len() < MIN_MEMBERS {
        return Reply::text("❌ Clan lawan butuh minimal 3 member");
    }

    let my_power = calculate_power(&ctx, &my_clan);
    let enemy_power = calculate_power(&ctx, &enemy);
    let is_win = simulate_war(my_power, enemy_power);

    let my_rewards = scaled_rewards(&my_clan, is_win);
    let enemy_rewards = scaled_rewards(&enemy, !is_win);

    let mut my_new = my_clan.clone();
    settle(&mut my_new, is_win);
    // Kalau lawan ternyata clan sendiri (cocok lewat nama), JS memakai satu
    // objek yang sama untuk `myClan` dan `enemyClan`, jadi kedua `settle`
    // menumpuk di satu baris. Menyalin `my_new` mempertahankan itu.
    let mut enemy_new = if enemy_key == my_key {
        my_new.clone()
    } else {
        enemy.clone()
    };
    settle(&mut enemy_new, !is_win);

    let my_members = members(&my_clan);
    let enemy_members = members(&enemy);
    let total_members = my_members.len() + enemy_members.len();

    let mut mutations = Vec::with_capacity(total_members);
    let mut skipped = 0usize;
    for jid in &my_members {
        match reward_member(&ctx, jid, &my_rewards) {
            Some(m) => mutations.push(m),
            None => skipped += 1,
        }
    }
    for jid in &enemy_members {
        match reward_member(&ctx, jid, &enemy_rewards) {
            Some(m) => mutations.push(m),
            None => skipped += 1,
        }
    }

    // `insert` kedua menimpa kalau kuncinya sama, jadi baris yang tersimpan
    // berisi hasil kedua `settle`.
    all.insert(my_key, my_new);
    all.insert(enemy_key, enemy_new);

    let text = result_text(
        &my_clan,
        &enemy,
        my_power,
        enemy_power,
        is_win,
        &my_rewards,
        skipped,
    );

    let mut writes = Vec::with_capacity(total_members + 1);
    writes.push(save_clans(all));
    writes.extend(mutations);

    Reply::text(text).mutating(writes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;
    use oktz_plugin_api::find;
    use oktz_plugin_api::registry::GroupRoster;

    /// Counter ditulis sebagai integer JSON, sama seperti yang ditulis
    /// `clancreate` — kalau di sini dipakai `f64`, `json!(0.0)` ikut menguji
    /// kebocoran float yang tidak ada di store sungguhan.
    fn clan(id: &str, name: &str, ms: &[&str], level: f64, wins: f64, exp: f64) -> Value {
        json!({
            "id": id,
            "name": name,
            "emblem": "🐉",
            "level": level as i64,
            "wins": wins as i64,
            "losses": 0,
            "exp": exp as i64,
            "members": ms,
        })
    }

    fn ctx_of(user: Value, text: &str, clans_map: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "clanwar".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            settings: json!({ "clans": clans_map }),
            ..Default::default()
        }
    }

    fn run(c: Ctx) -> Reply {
        (find("clanwar").unwrap().handler)(c)
    }

    fn text_of(c: Ctx) -> String {
        run(c).text_body().expect("harus ada teks").to_string()
    }

    fn saved_clans(r: &Reply) -> Map<String, Value> {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetSetting { .. }))
            .expect("harus ada SetSetting clans");
        match m {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "clans");
                value.as_object().unwrap().clone()
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("clanwar").unwrap();
        assert_eq!(p.meta.name, "clanwar");
        assert_eq!(p.meta.alias, ["war", "guildwar"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "War melawan clan lain");
        assert_eq!(p.meta.usage, ".clanwar <clan_id>");
        assert_eq!(p.meta.example, ".clanwar clan_123456");
        assert_eq!(p.meta.cooldown, 3600);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["war", "guildwar"] {
            assert_eq!(find(a).unwrap().meta.name, "clanwar", "alias {a}");
        }
    }

    #[test]
    fn a_sender_without_a_clan_is_turned_away() {
        let c = ctx_of(json!({}), "Dragon", json!({}));
        assert_eq!(text_of(c), "❌ Kamu belum punya clan");
    }

    #[test]
    fn no_target_shows_the_help_block() {
        let c = ctx_of(
            json!({ "clanId": "A" }),
            "",
            json!({ "A": clan("A", "Alpha", &["628111@s.whatsapp.net"], 1.0, 0.0, 0.0) }),
        );
        let out = text_of(c);
        assert!(out.starts_with("⚔️ *CLAN WAR*"), "{out}");
        assert!(out.contains("Syarat: Minimal 3 member per clan"), "{out}");
    }

    #[test]
    fn a_missing_own_clan_and_a_missing_enemy_are_distinct_messages() {
        let only_a = json!({ "A": clan("A", "Alpha", &["628111@s.whatsapp.net"], 1.0, 0.0, 0.0) });
        assert_eq!(
            text_of(ctx_of(json!({ "clanId": "Z" }), "Alpha", only_a.clone())),
            "❌ Clan kamu tidak ditemukan"
        );
        assert_eq!(
            text_of(ctx_of(json!({ "clanId": "A" }), "Nope", only_a)),
            "❌ Clan lawan tidak ditemukan"
        );
    }

    #[test]
    fn fighting_your_own_clan_id_is_refused() {
        let map = json!({ "A": clan("A", "Alpha", &["1", "2", "3"], 1.0, 0.0, 0.0) });
        assert_eq!(
            text_of(ctx_of(json!({ "clanId": "A" }), "A", map)),
            "❌ Tidak bisa war melawan clan sendiri"
        );
    }

    #[test]
    fn both_clans_need_at_least_three_members() {
        let mut map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
            "B": clan("B", "Beta", &["4@s", "5@s"], 1.0, 0.0, 0.0),
        });
        let me = json!({ "clanId": "A" });
        assert_eq!(
            text_of(ctx_of(me.clone(), "B", map.clone())),
            "❌ Clan lawan butuh minimal 3 member"
        );
        map["A"] = json!(clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s"], 1.0, 0.0, 0.0));
        assert_eq!(
            text_of(ctx_of(me, "B", map)),
            "❌ Clan kamu butuh minimal 3 member"
        );
    }

    #[test]
    fn the_enemy_is_found_by_id_key_then_by_name_then_by_its_id_field() {
        let map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
            "XYZ": clan("BE7", "Beta", &["4@s", "5@s", "6@s"], 1.0, 0.0, 0.0),
        });
        let me = json!({ "clanId": "A" });
        for target in ["XYZ", "beta", "BE7"] {
            let r = run(ctx_of(me.clone(), target, map.clone()));
            assert!(
                !r.text_body().unwrap().contains("tidak ditemukan"),
                "target {target} harus ketemu"
            );
        }
    }

    #[test]
    fn power_counts_member_levels_from_the_readable_user_rows() {
        let clan_v = clan(
            "A",
            "Alpha",
            &["628111@s.whatsapp.net", "628222@s.whatsapp.net", "628333@s.whatsapp.net"],
            2.0,
            3.0,
            0.0,
        );
        let mut ctx = ctx_of(json!({ "clanId": "A" }), "B", json!({}));
        ctx.user_data = json!({ "rpg": { "level": 10, "exp": 500 } });
        ctx.roster = GroupRoster {
            participants: vec![Participant {
                jid: "628222@s.whatsapp.net".into(),
                is_admin: false,
                is_owner: false,
                data: json!({ "level": 20, "exp": 1000 }),
            }],
            is_bot_admin: false,
        };
        // 10*100 + 500/10 = 1050; 20*100 + 1000/10 = 2100; 1*100 + 0 = 100;
        // level 2*500 = 1000; wins 3*50 = 150 -> 4400
        assert_eq!(calculate_power(&ctx, &clan_v), 4400.0);
    }

    #[test]
    fn a_clan_with_no_readable_rows_still_has_the_documented_minimum_power() {
        let clan_v = clan("A", "Alpha", &["1@s", "2@s", "3@s"], 0.0, 0.0, 0.0);
        let ctx = ctx_of(json!({ "clanId": "A" }), "B", json!({}));
        // 3 member * (1*100 + 0) + level default 1 * 500 + 0 = 800
        assert_eq!(calculate_power(&ctx, &clan_v), 800.0);
    }

    #[test]
    fn rewards_scale_by_one_tenth_of_the_clan_level_and_floor() {
        // level 1 -> mult 1.1 -> 30000*1.1 = 33000
        assert_eq!(scaled_rewards(&json!({ "level": 1 }), true).koin, 33_000.0);
        // level 10 -> mult 2.0 -> 60000; exp 30000; energi 30
        let r = scaled_rewards(&json!({ "level": 10 }), true);
        assert_eq!((r.koin, r.exp, r.energi), (60_000.0, 30_000.0, 30.0));
        // level 0 -> default 1
        assert_eq!(scaled_rewards(&json!({ "level": 0 }), false).koin, 6_600.0);
    }

    #[test]
    fn level_is_recomputed_from_total_exp() {
        assert_eq!(level_from_exp(0.0), 1);
        assert_eq!(level_from_exp(9_999.0), 1);
        assert_eq!(level_from_exp(10_000.0), 2);
        assert_eq!(level_from_exp(25_500.0), 3);
    }

    #[test]
    fn the_power_bar_is_always_exactly_ten_squares() {
        for (a, b) in [(1.0, 9.0), (5.0, 5.0), (9.0, 1.0), (0.0, 1.0), (1.0, 0.0)] {
            let bar = power_bar(a, b);
            assert_eq!(bar.chars().count(), 10, "power {a} vs {b} -> {bar}");
        }
        assert!(power_bar(10.0, 0.0).starts_with("🟩🟩🟩🟩🟩"));
        assert!(power_bar(0.0, 10.0).ends_with("🟥🟥🟥🟥🟥"));
    }

    #[test]
    fn a_war_settles_both_clans_and_rewrites_both_levels() {
        let map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
            "B": clan("B", "Beta", &["4@s", "5@s", "6@s"], 1.0, 0.0, 0.0),
        });
        let r = run(ctx_of(json!({ "clanId": "A", "koin": 0, "exp": 0, "energi": 10 }), "B", map));
        let after = saved_clans(&r);
        let a = &after["A"];
        let b = &after["B"];
        // Tepat satu dari dua clan naik `wins`, yang lain `losses`.
        let a_won = field_num(a, "wins") == 1.0;
        assert_eq!(a_won, field_num(b, "losses") == 1.0, "satu menang satu kalah");
        assert_eq!(field_num(a, "losses") == 1.0, !a_won);
        // exp = 5000 -> level 1 ; exp = 1000 -> level 1
        assert_eq!(field_num(a, "exp"), if a_won { CLAN_EXP_WIN } else { CLAN_EXP_LOSE });
        assert_eq!(a["level"], json!(1), "floor(5000/10000)+1 dan floor(1000/10000)+1");
        assert_eq!(b["level"], json!(1));
        // Nama clan lain tidak ikut berubah.
        assert_eq!(field_str(b, "name"), "Beta");
    }

    #[test]
    fn only_members_whose_row_is_readable_get_a_reward_mutation() {
        let map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
            "B": clan("B", "Beta", &["4@s", "5@s", "6@s"], 1.0, 0.0, 0.0),
        });
        let mut ctx = ctx_of(
            json!({ "clanId": "A", "koin": 100, "exp": 10, "energi": 10 }),
            "B",
            map,
        );
        ctx.roster = GroupRoster {
            participants: vec![Participant {
                jid: "2@s".into(),
                is_admin: false,
                is_owner: false,
                data: json!({ "koin": 0, "exp": 0, "energi": 0 }),
            }],
            is_bot_admin: false,
        };
        let r = run(ctx);
        let set_users: Vec<_> = r
            .mutations()
            .iter()
            .filter(|m| matches!(m, Mutation::SetUser { .. }))
            .collect();
        assert_eq!(set_users.len(), 2, "sender + satu anggota roster");
        let jids: Vec<&str> = set_users
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, .. } => jid.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert!(jids.contains(&"628111@s.whatsapp.net"));
        assert!(jids.contains(&"2@s"));
        assert!(
            r.text_body().unwrap().contains("4 member tidak dapat diberi hadiah"),
            "sisanya harus disebut, bukan hilang senyap"
        );
    }

    #[test]
    fn a_minus_one_sentinel_member_is_left_untouched() {
        let clan_v = clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0);
        let mut ctx = ctx_of(
            json!({ "clanId": "A", "koin": -1, "exp": 0, "energi": 0 }),
            "B",
            json!({}),
        );
        ctx.roster = GroupRoster {
            participants: vec![Participant {
                jid: "2@s".into(),
                is_admin: false,
                is_owner: false,
                data: json!({ "koin": 5, "exp": 0, "energi": 0 }),
            }],
            is_bot_admin: false,
        };
        assert_eq!(
            reward_member(&ctx, "628111@s.whatsapp.net", &scaled_rewards(&clan_v, true)),
            None,
            "koin = -1 berarti tak pernah dibatasi; JS tidak menulis apa pun"
        );
        assert!(
            reward_member(&ctx, "2@s", &scaled_rewards(&clan_v, true)).is_some()
        );
        assert_eq!(reward_member(&ctx, "9@s", &scaled_rewards(&clan_v, true)), None);
    }

    #[test]
    fn rewards_clamp_at_zero_and_at_the_store_maximums() {
        let mut ctx = ctx_of(
            json!({ "clanId": "A", "koin": 5, "exp": 0, "energi": 0 }),
            "B",
            json!({}),
        );
        ctx.roster = GroupRoster::default();
        let m = reward_member(&ctx, "628111@s.whatsapp.net", &scaled_rewards(&json!({ "level": 1 }), true))
            .expect("ada mutasi");
        match m {
            Mutation::SetUser { data, .. } => {
                assert_eq!(data["energi"], json!(16), "Math.max(0, 0 + 16)");
                assert_eq!(data["koin"], json!(5 + 33_000));
            }
            _ => unreachable!(),
        }
        // `Math.max(0, energi + amount)` — hanya `energi == -1` yang lolos
        // sebagai sentinel; nilai negatif lain tetap dijepit ke 0.
        ctx.user_data = json!({ "clanId": "A", "koin": 0, "exp": 0, "energi": -100 });
        match reward_member(&ctx, "628111@s.whatsapp.net", &scaled_rewards(&json!({ "level": 1 }), true)).unwrap() {
            Mutation::SetUser { data, .. } => {
                assert_eq!(data["energi"], json!(0), "max(0, -100 + 16) = 0");
                assert_eq!(data["koin"], json!(33_000), "Math.max(0, ..) di updateKoin");
            }
            _ => unreachable!(),
        }

        ctx.user_data = json!({ "clanId": "A", "koin": 9_000_000_000_000i64 - 1, "exp": 9_000_000_000i64, "energi": 0 });
        let m = reward_member(&ctx, "628111@s.whatsapp.net", &scaled_rewards(&json!({ "level": 1 }), true)).unwrap();
        match m {
            Mutation::SetUser { data, .. } => {
                assert_eq!(data["koin"], json!(9_000_000_000_000i64), "MAX_KOIN");
                assert_eq!(data["exp"], json!(9_000_000_000i64), "MAX_EXP");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn clan_counters_are_written_as_json_integers() {
        let map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
            "B": clan("B", "Beta", &["4@s", "5@s", "6@s"], 1.0, 0.0, 0.0),
        });
        let after = saved_clans(&run(ctx_of(
            json!({ "clanId": "A", "koin": 0, "exp": 0, "energi": 0 }),
            "B",
            map,
        )));
        for c in [&after["A"], &after["B"]] {
            for key in ["wins", "losses", "exp", "level"] {
                assert!(
                    c[key].is_i64(),
                    "{key} harus integer JSON, bukan {}: {c}",
                    c[key]
                );
            }
        }
    }

    #[test]
    fn a_target_that_resolves_back_to_my_own_clan_still_settles_once_per_side() {
        // `.clanwar Alpha` — "Alpha" bukan `clanId`, jadi cek "diri sendiri"
        // di JS tidak menangkapnya dan `myClan`/`enemyClan` menunjuk objek
        // yang sama: wins dan losses naik dua-duanya, exp dapat 5000+1000.
        let map = json!({
            "A": clan("A", "Alpha", &["628111@s.whatsapp.net", "2@s", "3@s"], 1.0, 0.0, 0.0),
        });
        let after = saved_clans(&run(ctx_of(
            json!({ "clanId": "A", "koin": 0, "exp": 0, "energi": 0 }),
            "Alpha",
            map,
        )));
        let a = &after["A"];
        assert_eq!(a["wins"], json!(1));
        assert_eq!(a["losses"], json!(1));
        assert_eq!(a["exp"], json!(6_000));
        assert_eq!(a["level"], json!(1));
        assert_eq!(after.len(), 1, "hanya satu clan tersimpan");
    }

    /// `result_text` mencetak `mine` apa adanya di kedua cabang; PEMILIH
    /// angka win/lose ada di handler (`scaled_rewards(clan, is_win)`), jadi
    /// test ini harus mengoper set hadiah yang sesuai dengan cabangnya.
    #[test]
    fn the_loser_see_the_consolation_line_and_the_winner_the_reward_line() {
        let win_r = scaled_rewards(&json!({ "level": 1 }), true);
        let lose_r = scaled_rewards(&json!({ "level": 1 }), false);
        let my = json!({ "name": "Alpha", "emblem": "🐉" });
        let enemy = json!({ "name": "Beta", "emblem": "🔥" });

        let win = result_text(&my, &enemy, 900.0, 100.0, true, &win_r, 0);
        assert!(win.contains("🐉 *Alpha*  vs  *Beta* 🔥"), "{win}");
        assert!(win.contains("💪 900  vs  100"), "{win}");
        assert!(win.contains("Alpha WINS!"), "{win}");
        // Balasan diawali header `WAR RESULT`, jadi baris hadiah dicari, bukan
        // dianchor ke depan.
        assert!(win.contains("🎁 Reward per member:"), "{win}");
        assert!(win.contains("+Rp 33.000 · +16.500 EXP · +16 Energi"), "{win}");
        assert!(win.contains("+5.000 Clan EXP"), "{win}");

        let lose = result_text(&my, &enemy, 100.0, 900.0, false, &lose_r, 0);
        assert!(lose.contains("Beta WINS!"), "{lose}");
        assert!(lose.contains("😔 Konsolasi per member:"), "{lose}");
        assert!(lose.contains("+Rp 6.600 · +3.300 EXP · +3 Energi"), "{lose}");
        assert!(lose.contains("+1.000 Clan EXP"), "{lose}");
        assert!(!lose.contains("member tidak dapat"), "skipped=0 -> tanpa peringatan");
    }

    /// Balok kedua: `winnerClan` danReward/Consolasi benar-benar bergantung
    /// pada cabang, bukan pada isi `mine`.
    #[test]
    fn the_printed_amounts_follow_the_set_handed_in() {
        let win_r = scaled_rewards(&json!({ "level": 1 }), true);
        let lose_r = scaled_rewards(&json!({ "level": 1 }), false);
        assert_eq!((win_r.koin, win_r.exp, win_r.energi), (33_000.0, 16_500.0, 16.0));
        assert_eq!((lose_r.koin, lose_r.exp, lose_r.energi), (6_600.0, 3_300.0, 3.0));
    }
}
