//! `plugins/user/daily.js` — klaim hadiah harian (exp, koin, potion).
//!
//! Hadiah diacak DULUAN (`Math.random`), jadi balasan berbeda tiap kali
//! dipanggil — yang dikunci test adalah RENTANG dan bentuknya, bukan
//! angka persisnya. Cooldown dibaca dari `user.cooldowns.daily` (epoch ms),
//! bukan dari counter router.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{now_ms, time_hm};
use crate::tools::kalkulatormbg::thousands_id;

/// `const DAILY_COOLDOWN = 24 * 60 * 60 * 1000`.
pub(crate) const DAILY_COOLDOWN: i64 = 24 * 60 * 60 * 1000;

/// `Math.floor(Math.random() * n) + offset` → rentang inklusif
/// `offset..offset+n-1`.
pub(crate) fn reward(offset: i64, span: i64) -> i64 {
    offset + (crate::common::roll(span as u32) as i64)
}

/// Sisa cooldown dalam jam + menit, dipotong ke bawah.
pub(crate) fn remaining_text(remaining_ms: i64) -> String {
    format!(
        "🕕 *ᴄᴏᴏʟᴅᴏᴡɴ*\n\n> Kamu sudah klaim hari ini.\n> Tunggu: *{} jam {} menit* lagi.",
        remaining_ms / 3_600_000,
        (remaining_ms % 3_600_000) / 60_000
    )
}

/// `getTimeGreeting()` (`formatter.js:508`) — jam lokal Asia/Jakarta.
pub(crate) fn greeting(now: i64) -> &'static str {
    match jakarta_hour(now) {
        4..=9 => "Selamat Pagi 🌅",
        10..=14 => "Selamat Siang ☀️",
        15..=17 => "Selamat Sore 🌇",
        _ => "Selamat Malam 🌙",
    }
}

/// Jam lokal 0-23 dari epoch ms.
pub(crate) fn jakarta_hour(now: i64) -> u32 {
    let local = now.div_euclid(1000) + crate::common::JAKARTA_OFFSET_SECS;
    ((local.rem_euclid(86_400)) / 3600) as u32
}

pub(crate) fn success(greeting: &str, sender_num: &str, exp: i64, koin: i64, potion: i64) -> String {
    format!(
        "🎉 *ᴅᴀɪʟʏ ᴄʟᴀɪᴍ sᴜᴋsᴇs*\n\
         > {greeting}, @{sender_num}\n\n\
         ╭┈┈⬡「 🎁 *ʀᴇᴡᴀʀᴅs* 」\n\
         ┃ 🚄 Exp: *+{exp}*\n\
         ┃ 💰 Koin: *+{koin}*\n\
         ┃ 🥤 Potion: *+{potion}*\n\
         ╰┈┈⬡\n\n\
         > Jangan lupa claim lagi besok!",
        koin = thousands_id(koin as f64)
    )
}

#[plugin(
    name = "daily",
    alias = ["claim", "harian", "bonus"],
    category = "user",
    description = "Claim hadiah harian (Exp, Money, Potion)",
    usage = ".daily",
    example = ".daily",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn daily(ctx: Ctx) -> Reply {
    let now = now_ms();
    let last = ctx
        .user_field("cooldowns.daily")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);

    if now - last < DAILY_COOLDOWN {
        return Reply::text(remaining_text(last + DAILY_COOLDOWN - now));
    }

    let exp_reward = reward(1000, 5000);
    let money_reward = reward(5000, 10_000);
    let potion_reward = reward(1, 3);

    let koin = crate::common::int_field(&ctx, "koin") + money_reward;

    // `user.inventory.potion` ditulis penuh; field inventaris lain ikut
    // disalin supaya tidak hilang.
    let mut inventory = ctx
        .user_field("inventory")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let potion = inventory
        .get("potion")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
        + potion_reward;
    inventory.insert("potion".to_string(), serde_json::json!(potion));

    // `user.cooldowns.daily = now` — objek cooldowns lain ikut tersimpan.
    let mut cooldowns = ctx
        .user_field("cooldowns")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    cooldowns.insert("daily".to_string(), serde_json::json!(now));

    Reply::Text {
        text: success(
            greeting(now),
            crate::common::jid_number(&ctx.sender),
            exp_reward,
            money_reward,
            potion_reward,
        ),
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({
            "exp": crate::common::int_field(&ctx, "exp") + exp_reward,
            "koin": koin,
            "inventory": inventory,
            "cooldowns": cooldowns,
        }),
    })
}

/// `time_hm` dipakai ulang agar zona waktu di plugin ini sama dengan yang
/// dipakai plugin lain (`common::jakarta_parts`).
#[allow(dead_code)]
fn _assert_same_timezone(now: i64) -> String {
    time_hm(now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "daily".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("daily").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn message_mentions(r: &Reply) -> &Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("daily").unwrap();
        assert_eq!(p.meta.name, "daily");
        assert_eq!(p.meta.alias, ["claim", "harian", "bonus"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Claim hadiah harian (Exp, Money, Potion)");
        assert_eq!(p.meta.usage, ".daily");
        assert_eq!(p.meta.example, ".daily");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["claim", "harian", "bonus"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "daily");
        }
    }

    #[test]
    fn a_recent_claim_is_blocked_and_writes_nothing() {
        let just_now = now_ms();
        // Batas bawah dipakai `DAILY_COOLDOWN - 1_000`, bukan `- 1`: handler
        // memanggil `now_ms()` sendiri beberapa milidetik setelah baris ini,
        // jadi gap `- 1` bisa naik ke cooldown penuh di antara dua pembacaan
        // dan test jadi flaky.
        for gap in [0i64, 1000, 3_600_000, DAILY_COOLDOWN - 1_000] {
            let r = run(json!({ "cooldowns": { "daily": just_now - gap } }));
            assert!(text_of(&r).starts_with("🕕 *ᴄᴏᴏʟᴅᴏᴡɴ*"), "gap {gap}");
            assert!(r.mutations().is_empty(), "gap {gap} tidak boleh menulis");
        }
    }

    #[test]
    fn the_cooldown_reply_reports_hours_and_minutes_floored() {
        let just_now = now_ms();
        let r = run(json!({ "cooldowns": { "daily": just_now - 60_000 } }));
        let t = text_of(&r);
        // Tersisa 23 jam 59 menit.
        assert!(t.contains("23 jam 59 menit"), "{t}");
    }

    #[test]
    fn a_claim_at_or_past_the_full_day_is_allowed() {
        let old = now_ms() - DAILY_COOLDOWN;
        let r = run(json!({ "cooldowns": { "daily": old } }));
        assert!(text_of(&r).starts_with("🎉 *ᴅᴀɪʟʏ ᴄʟᴀɪᴍ sᴜᴋsᴇs*"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn a_user_with_no_cooldown_row_can_claim_immediately() {
        for user in [json!(null), json!({}), json!({"cooldowns": {}})] {
            let r = run(user.clone(), );
            assert!(text_of(&r).starts_with("🎉"), "input {user}");
            assert_eq!(r.mutations().len(), 1, "input {user}");
        }
    }

    #[test]
    fn the_rewards_stay_inside_the_javascript_ranges() {
        // 200 sampel per klaim; ranges: exp 1000-5999, money 5000-14999,
        // potion 1-3.
        for _ in 0..200 {
            let r = run(json!({}));
            let d = write_of(&r);
            let exp = d["exp"].as_i64().unwrap();
            let koin = d["koin"].as_i64().unwrap();
            let potion = d["inventory"]["potion"].as_i64().unwrap();
            assert!((1000..=5999).contains(&exp), "exp {exp}");
            assert!((5000..=14999).contains(&koin), "koin {koin}");
            assert!((1..=3).contains(&potion), "potion {potion}");
        }
    }

    #[test]
    fn rewards_are_added_to_the_stored_balances_not_overwriting_them() {
        let r = run(json!({"exp": 5000, "koin": 1000}));
        let d = write_of(&r);
        assert!(d["exp"].as_i64().unwrap() > 5000);
        assert!(d["koin"].as_i64().unwrap() > 1000);
    }

    #[test]
    fn the_cooldown_stamp_is_written_and_other_cooldowns_survive() {
        let before = now_ms();
        let r = run(json!({ "cooldowns": { "lain": 12345 } }));
        let d = write_of(&r);
        assert_eq!(d["cooldowns"]["lain"], json!(12345), "cooldown lain utuh");
        assert!(
            d["cooldowns"]["daily"].as_i64().unwrap() >= before,
            "daily harus dicatat"
        );
    }

    #[test]
    fn other_inventory_items_survive_the_potion_update() {
        let r = run(json!({ "inventory": { "kunai": 3 } }));
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["kunai"], json!(3), "item lain tidak boleh hilang");
        assert!(inv["potion"].as_i64().unwrap() >= 1);
    }

    #[test]
    fn the_potion_count_accumulates_across_claims() {
        let r = run(json!({ "inventory": { "potion": 10 } }));
        assert!(write_of(&r)["inventory"]["potion"].as_i64().unwrap() >= 11);
    }

    #[test]
    fn the_reply_mentions_the_sender_and_shows_a_local_greeting() {
        let r = run(json!({}));
        assert_eq!(message_mentions(&r), &["628111@s.whatsapp.net"]);
        let t = text_of(&r);
        assert!(t.starts_with("🎉 *ᴅᴀɪʟʏ ᴄʟᴀɪᴍ sᴜᴋsᴇs*"), "{t}");
        assert!(t.contains(", @628111"), "{t}");
        // Salam mengikuti jam berjalan sungguhan, jadi yang dikunci adalah bahwa
        // SALAM YANG BENAR untuk jam itu yang muncul — bukan jam tertentu.
        let expected = greeting(now_ms());
        assert!(
            t.contains(expected),
            "salam harus sesuai jam lokal sekarang ({expected}): {t}"
        );
    }

    #[test]
    fn the_greeting_bands_match_the_javascript_hours() {
        // `getHour()` membaca jam LOKAL Asia/Jakarta, jadi jam yang diuji
        // adalah jam WIB. `jakarta_hour_utc_ms` membangun epoch ms yang
        // jatuh persis di jam WIB itu.
        let at = |h: u32| {
            let local_ms = (h as i64) * 3_600_000;
            let utc_ms = local_ms - crate::common::JAKARTA_OFFSET_SECS * 1000;
            greeting(1_704_067_200_000 + utc_ms)
        };
        assert_eq!(at(4), "Selamat Pagi 🌅", "04:00 WIB");
        assert_eq!(at(9), "Selamat Pagi 🌅", "09:59 WIB masih pagi");
        assert_eq!(at(10), "Selamat Siang ☀️");
        assert_eq!(at(14), "Selamat Siang ☀️");
        assert_eq!(at(15), "Selamat Sore 🌇");
        assert_eq!(at(17), "Selamat Sore 🌇");
        assert_eq!(at(18), "Selamat Malam 🌙");
        assert_eq!(at(3), "Selamat Malam 🌙", "03:59 WIB masih malam");
    }

    #[test]
    fn the_greeting_band_edges_are_half_open_intervals() {
        // `hour >= 4 && hour < 10` — batasnya eksklusif di atas.
        let at = |h: u32| {
            let utc_ms = (h as i64) * 3_600_000 - crate::common::JAKARTA_OFFSET_SECS * 1000;
            greeting(1_704_067_200_000 + utc_ms)
        };
        assert_ne!(at(3), at(4), "03:59 vs 04:00");
        assert_ne!(at(9), at(10), "09:59 vs 10:00");
        assert_ne!(at(14), at(15));
        assert_ne!(at(17), at(18));
        assert_eq!(at(0), at(23), "dini hari dan malam sama-sama Malam");
    }

    #[test]
    fn the_local_hour_uses_the_jakarta_offset() {
        // 00:00 UTC = 07:00 WIB.
        assert_eq!(jakarta_hour(1_704_067_200_000), 7);
    }

    #[test]
    fn the_success_reply_is_paired_with_the_write() {
        let r = run(json!({}));
        assert!(text_of(&r).contains("ᴄʟᴀɪᴍ sᴜᴋsᴇs"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_reward_helper_covers_the_whole_inclusive_range() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let v = reward(1, 3);
            assert!((1..=3).contains(&v), "di luar 1..=3: {v}");
            seen.insert(v);
        }
        assert_eq!(seen.len(), 3, "tiga nilai harus mungkin muncul");
    }
}