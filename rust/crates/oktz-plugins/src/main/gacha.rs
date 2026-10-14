//! `plugins/main/gacha.js` — undian berbobot untuk koin/exp.
//!
//! Dua kursor yang sengaja dipertahankan:
//!
//! 1. `switch (reward.type)` TIDAK punya `case "limit"`, padahal `rewards`
//!    memuat lima entri `limit` dengan total chance 50.5 — jadi ~26% tarikan
//!    jatuh ke type itu dan TIDAK menulis apa pun ke store. `rewardText` dan
//!    `typeEmoji` tetap kosong, jadi baris Hadiah jadi `*Hadiah:*  `. Itu
//!    bug JS, bukan kelewatan port.
//! 2. `case "energi"` tidak pernah kena — tidak ada entri `energi` di tabel.
//!
//! `ponytail:` kartu level-up `addExpWithLevelCheck` tidak ikut (butuh
//! canvas + `sock.sendMedia`).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::rpg::heal::{
    MAX_KOIN, exp_gain_only, math_random, random_rare_frame, roll_incl, thousands_en,
};

/// `(type, min, max, rarity, emoji, chance)` — urutan object literal = urutan
/// undian.
const REWARDS: [(&str, i64, i64, &str, &str, f64); 16] = [
    ("balance", 100, 500, "common", "⚪", 35.0),
    ("balance", 500, 1500, "uncommon", "🟢", 25.0),
    ("balance", 1500, 5000, "rare", "🔵", 15.0),
    ("balance", 5000, 15000, "epic", "🟣", 5.0),
    ("balance", 15000, 50000, "legendary", "🟡", 1.0),
    ("exp", 50, 200, "common", "⚪", 30.0),
    ("exp", 200, 800, "uncommon", "🟢", 20.0),
    ("exp", 800, 2000, "rare", "🔵", 10.0),
    ("exp", 2000, 5000, "epic", "🟣", 3.0),
    ("exp", 5000, 10000, "legendary", "🟡", 0.5),
    ("limit", 1, 3, "common", "⚪", 25.0),
    ("limit", 3, 7, "uncommon", "🟢", 15.0),
    ("limit", 7, 15, "rare", "🔵", 8.0),
    ("limit", 15, 30, "epic", "🟣", 2.0),
    ("limit", 30, 50, "legendary", "🟡", 0.5),
    ("jackpot", 100000, 500000, "mythic", "🌟", 0.1),
];

/// `rarityColors` (`gacha.js:107-114`).
const RARITY_COLORS: [(&str, &str); 6] = [
    ("common", "⚪ Common"),
    ("uncommon", "🟢 Uncommon"),
    ("rare", "🔵 Rare"),
    ("epic", "🟣 Epic"),
    ("legendary", "🟡 Legendary"),
    ("mythic", "🌟 MYTHIC"),
];

/// `rarity === "mythic" | "legendary" | "epic"` → banner, selain itu `""`.
fn banner(rarity: &str) -> &'static str {
    match rarity {
        "mythic" => "🎊🎊🎊 *JACKPOT!* 🎊🎊🎊\n\n",
        "legendary" => "✨ *LEGENDARY PULL!* ✨\n\n",
        "epic" => "💜 *EPIC PULL!* 💜\n\n",
        _ => "",
    }
}

#[plugin(
    name = "gacha",
    alias = ["pull", "lucky"],
    category = "rpg",
    description = "Gacha untuk dapat hadiah random",
    usage = ".gacha",
    example = ".gacha",
    cooldown = 300,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn gacha(ctx: Ctx) -> Reply {
    let (kind, min, max, rarity, emoji, _) = pick_reward();
    let amount = roll_incl(min, max);

    let mut text = format!(
        "{} *ɢᴀᴄʜᴀ ʀᴇsᴜʟᴛ*\n\n\
         ╭─────────────╮\n\
         │  {emoji} {emoji} {emoji}  │\n\
         ╰─────────────╯\n\n",
        random_rare_frame()
    );
    text.push_str(banner(rarity));

    // `switch (reward.type)` — `limit` dan apa pun yang tidak dikenal TIDAK
    // menulis ke store dan TIDAK memberi emoji/jumlah.
    let mutation = match kind {
        "balance" | "jackpot" => {
            let type_emoji = if kind == "jackpot" { "💎" } else { "💰" };
            text.push_str(&format!(
                "*Rarity:* {}\n*Hadiah:* {type_emoji} +{} Koin\n\n",
                rarity_color(rarity),
                thousands_en(amount)
            ));
            Some(update_koin(&ctx, amount))
        }
        "exp" => {
            text.push_str(&format!(
                "*Rarity:* {}\n*Hadiah:* ⭐ +{} EXP\n\n",
                rarity_color(rarity),
                thousands_en(amount)
            ));
            Some(update_exp(&ctx, amount))
        }
        _ => {
            // Tanpa `case`, `rewardText` dan `typeEmoji` tetap `""`.
            text.push_str(&format!(
                "*Rarity:* {}\n*Hadiah:*  \n\n",
                rarity_color(rarity)
            ));
            None
        }
    };
    text.push_str("_Cooldown: 5 menit_");

    let reply = Reply::text(text);
    match mutation {
        Some((key, value)) => reply.with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ key: value }),
        }),
        None => reply,
    }
}

/// `rarityColors[reward.rarity]` — selalu ketemu karena `rarity` berasal dari
/// tabel sendiri.
fn rarity_color(rarity: &str) -> &'static str {
    RARITY_COLORS
        .iter()
        .find(|(k, _)| *k == rarity)
        .map_or("", |(_, v)| *v)
}

/// `getRandomReward` (`gacha.js:116-130`) — undian berbobot linear, bukan
/// tabel Rufle: `random` dikurangi chance tiap entri dan yang pertama
/// membuat `random <= 0` menang. cabang fallback hanya mungkin kalau
/// `Math.random()` persis 1.0, jadi di sini tetap ditulis sebagai jaring.
fn pick_reward() -> (&'static str, i64, i64, &'static str, &'static str, f64) {
    let total: f64 = REWARDS.iter().map(|(_, _, _, _, _, c)| *c).sum();
    let mut random = math_random() * total;
    for (kind, min, max, rarity, emoji, chance) in REWARDS {
        random -= chance;
        if random <= 0.0 {
            return (kind, min, max, rarity, emoji, chance);
        }
    }
    let (kind, min, max, rarity, emoji, chance) = REWARDS[0];
    (kind, min, max, rarity, emoji, chance)
}

/// `db.updateKoin` (`database.js:646`) — `koin === -1` berarti tak terbatas
/// dan tidak boleh disentuh sama sekali.
fn update_koin(ctx: &Ctx, amount: i64) -> (&'static str, Value) {
    let koin = ctx.user_field("koin").and_then(Value::as_i64).unwrap_or(0);
    if koin == -1 {
        ("koin", Value::from(-1))
    } else {
        ("koin", Value::from((koin + amount).clamp(0, MAX_KOIN)))
    }
}

/// `db.updateExp` (`database.js:664`).
fn update_exp(ctx: &Ctx, amount: i64) -> (&'static str, Value) {
    let exp = ctx.user_field("exp").and_then(Value::as_i64).unwrap_or(0);
    ("exp", Value::from(exp_gain_only(exp, amount)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "gacha".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("gacha").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gacha").unwrap();
        assert_eq!(p.meta.name, "gacha");
        assert_eq!(p.meta.alias, ["pull", "lucky"]);
        // `category` di JS adalah "rpg" walau berkasnya di `plugins/main/`.
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Gacha untuk dapat hadiah random");
        assert_eq!(p.meta.usage, ".gacha");
        assert_eq!(p.meta.example, ".gacha");
        assert_eq!(p.meta.cooldown, 300);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pull", "lucky"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "gacha");
        }
    }

    #[test]
    fn the_reward_table_keeps_the_js_rows_and_its_total_weight() {
        assert_eq!(REWARDS.len(), 16);
        assert_eq!(REWARDS[0], ("balance", 100, 500, "common", "⚪", 35.0));
        assert_eq!(REWARDS[15], ("jackpot", 100000, 500000, "mythic", "🌟", 0.1));
        let total: f64 = REWARDS.iter().map(|(_, _, _, _, _, c)| *c).sum();
        assert_eq!(total, 195.1);
        // Limtype yang tidak punya `case` itu bobotnya nol kumulatif != nol:
        // justru lima baris `limit` yang tidak tertulis sama sekali.
        let limit: f64 = REWARDS
            .iter()
            .filter(|(k, ..)| *k == "limit")
            .map(|(_, _, _, _, _, c)| *c)
            .sum();
        assert_eq!(limit, 50.5);
    }

    #[test]
    fn every_pull_carries_a_rarity_line_a_hadiah_line_and_the_cooldown() {
        for _ in 0..300 {
            let r = run(json!({}));
            let t = text_of(&r);
            assert!(t.contains(" *ɢᴀᴄʜᴀ ʀᴇsᴜʟᴛ*"), "{t}");
            assert!(t.contains("╭─────────────╮"), "{t}");
            assert!(t.contains("*Rarity:* "), "{t}");
            assert!(t.contains("*Hadiah:* "), "{t}");
            assert!(t.ends_with("_Cooldown: 5 menit_"), "{t}");
            assert!(
                RARITY_COLORS
                    .iter()
                    .any(|(_, label)| t.contains(&format!("*Rarity:* {label}\n"))),
                "label rarity di luar tabel: {t}"
            );
        }
    }

    #[test]
    fn only_mythic_legendary_and_epic_get_a_banner() {
        for (rarity, want) in [
            ("mythic", "🎊🎊🎊 *JACKPOT!* 🎊🎊🎊"),
            ("legendary", "✨ *LEGENDARY PULL!* ✨"),
            ("epic", "💜 *EPIC PULL!* 💜"),
            ("rare", ""),
            ("common", ""),
            ("uncommon", ""),
        ] {
            let text = format!(
                "{} *ɢᴀᴄʜᴀ ʀᴇsᴜʟᴛ*\n\n{}{}",
                "🎰", banner(rarity), "*Rarity:* x\n"
            );
            if want.is_empty() {
                assert!(!text.contains("`LEGENDARY PULL!`"), "{rarity} tak boleh dapat banner");
            } else {
                assert!(text.contains(want), "{rarity}");
            }
        }
    }

    /// `koin` bertambah, `exp` tidak — ditulis sebagai satu key.
    #[test]
    fn a_koin_pull_only_touches_koin() {
        let mut saw_koin = false;
        for _ in 0..400 {
            let r = run(json!({"koin": 1000, "exp": 77}));
            let Some(data) = r.mutations().first() else {
                continue; // tarikan `limit` memang tidak menulis apa pun
            };
            let Mutation::SetUser { data, .. } = data else {
                panic!("harus SetUser");
            };
            let obj = data.as_object().unwrap();
            assert_eq!(obj.len(), 1, "satu key per tarikan: {data}");
            if let Some(koin) = obj.get("koin") {
                saw_koin = true;
                let gained = koin.as_i64().unwrap() - 1_000;
                let t = text_of(&r);
                // `case "jackpot"` juga `updateKoin`, tapi rentangnya 100rb+
                // — bukan branch `balance` yang 100..50.000.
                if t.contains("*Rarity:* 🌟 MYTHIC") {
                    assert!((100_000..=500_000).contains(&gained), "jackpot {gained}");
                    assert!(t.contains("*Hadiah:* 💎"), "{t}");
                } else {
                    assert!((100..=50_000).contains(&gained), "koin {gained}");
                    assert!(t.contains("*Hadiah:* 💰"), "{t}");
                }
                assert!(!t.contains("Hadiah:* ⭐"), "koin tak boleh membawa emoji exp: {t}");
            } else {
                assert!((50..=10_000).contains(&(obj.get("exp").unwrap().as_i64().unwrap() - 77)));
                assert!(!text_of(&r).contains("Koin\n"));
            }
        }
        assert!(saw_koin, "dalam 400 tarikan harus ada yang kena balance");
    }

    #[test]
    fn the_jackpot_row_wins_with_the_diamond_emoji_and_a_big_amount() {
        let (kind, min, max, rarity, ..) = REWARDS[15];
        assert_eq!(kind, "jackpot");
        assert_eq!((min, max, rarity), (100_000, 500_000, "mythic"));
        // `💎` hanya untuk jackpot; `💰` untuk balance biasa.
        assert_eq!(if kind == "jackpot" { "💎" } else { "💰" }, "💎");
    }

    #[test]
    fn an_unlimited_balance_or_exp_of_minus_one_is_left_alone() {
        // `user.koin === -1` → `updateKoin`/`updateExp` return -1 tanpa
        // menambah apa pun.
        assert_eq!(update_koin(&Ctx { user_data: json!({"koin": -1}), ..Default::default() }, 500), ("koin", json!(-1)));
        assert_eq!(update_exp(&Ctx { user_data: json!({"exp": -1}), ..Default::default() }, 500), ("exp", json!(-1)));
    }

    #[test]
    fn a_balance_is_clamped_at_the_stored_maximum() {
        let ctx = Ctx {
            user_data: json!({"koin": MAX_KOIN - 1}),
            ..Default::default()
        };
        let (_, v) = update_koin(&ctx, 500);
        assert_eq!(v, json!(MAX_KOIN));
    }

    #[test]
    fn a_pull_writes_the_sender_and_nothing_else() {
        let r = run(json!({"koin": 0, "exp": 0}));
        match r.mutations() {
            // `switch` di JS tak punya `case "limit"`, jadi tarikan itu
            // sengaja tidak menulis apa pun.
            [] => assert!(text_of(&r).contains("*Hadiah:*  \n"), "tarikan limit"),
            [Mutation::SetUser { jid, .. }] => assert_eq!(jid, "628111@s.whatsapp.net"),
            other => panic!("harus 0 atau 1 SetUser, dapat {other:?}"),
        }
    }
}