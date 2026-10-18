//! `plugins/rpg/challenge.js` — satu tantangan harian per user, diacak ulang
//! setiap `toDateString()` berubah.
//!
//! Kursor: `new Date().toDateString()` memakai zona waktu LOKAL proses Node
//! (bukan `src/lib/time.js` yang dipaksa Asia/Jakarta), dan bentuknya
//! `"Mon Oct 12 2026"`. Di sini tetap Asia/Jakarta — satu-satunya zona yang
//! dipakai repo ini — dan bentuknya dipertahankan supaya challenge yang
//! tersimpan di store tidak ikut berubah format.
//!
//! Progres HANYA bertambah lewat plugin lain (`fishing`, `mining`, ...)
//! yang menulis `rpg.dailyChallenge.progress`; plugin ini hanya membacanya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::heal::{apply_exp, or_default, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

/// `(name, type, target, gold, exp)` — `type` tidak pernah dibaca plugin ini,
/// tapi dipertahankan agar tabel tetap identik dengan JS.
type Challenge = (&'static str, &'static str, i64, i64, i64);

const CHALLENGES: [Challenge; 7] = [
    ("⚔️ Kalahkan 5 Monster", "kill", 5, 500, 200),
    ("🎣 Tangkap 3 Ikan", "fish", 3, 300, 150),
    ("⛏️ Tambang 10 Ore", "mine", 10, 400, 180),
    ("🌱 Panen 5 Hasil Kebun", "harvest", 5, 350, 160),
    ("🧪 Racik 3 Potion", "craft", 3, 450, 190),
    ("💰 Kumpulkan 1000 Koin", "earn", 1000, 500, 250),
    ("🗺️ Selesaikan 2 Ekspedisi", "expedition", 2, 600, 300),
];

/// `Intl` weekday short + bulan short, bentuk `Date.prototype.toDateString()`.
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `new Date().toDateString()` → `"Mon Oct 12 2026"`.
fn today_string(ms: i64) -> String {
    let days = (ms.div_euclid(1000) + crate::common::JAKARTA_OFFSET_SECS).div_euclid(86_400);
    let (y, m, d, ..) = crate::common::jakarta_parts(ms);
    // 1970-01-01 adalah Kamis (4).
    let weekday = (days + 4).rem_euclid(7) as usize;
    format!(
        "{} {} {d} {y}",
        WEEKDAYS[weekday],
        MONTHS_SHORT[(m - 1) as usize]
    )
}

/// `getNewDailyChallenge()` (`challenge.js:30`) — challenges acak + progres 0.
fn new_challenge(ms: i64) -> Value {
    let (name, kind, target, gold, exp) = CHALLENGES[crate::common::random_index(CHALLENGES.len())];
    serde_json::json!({
        "name": name,
        "type": kind,
        "target": target,
        "reward": { "gold": gold, "exp": exp },
        "progress": 0,
        "date": today_string(ms),
        "claimed": false,
    })
}

fn field(c: &Value, key: &str) -> Value {
    c.get(key).cloned().unwrap_or(Value::Null)
}

#[plugin(
    name = "challenge",
    alias = ["dailychallenge", "tantangan"],
    category = "rpg",
    description = "Daily challenge untuk hadiah spesial",
    usage = ".challenge",
    example = ".challenge",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn challenge(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let now = crate::common::now_ms();
    let today = today_string(now);

    let stored = rpg.get("dailyChallenge").cloned();
    let fresh = match &stored {
        Some(c) if field(c, "date").as_str() == Some(today.as_str()) => c.clone(),
        _ => {
            let fresh = new_challenge(now);
            rpg.insert("dailyChallenge".into(), fresh.clone());
            fresh
        }
    };

    let progress = field(&fresh, "progress").as_i64().unwrap_or(0);
    let target = field(&fresh, "target").as_i64().unwrap_or(0);
    let is_complete = progress >= target;
    let claimed = field(&fresh, "claimed").as_bool().unwrap_or(false);
    let reward = field(&fresh, "reward");
    let gold = reward.get("gold").and_then(Value::as_i64).unwrap_or(0);
    let exp = reward.get("exp").and_then(Value::as_i64).unwrap_or(0);
    let name = field(&fresh, "name").as_str().unwrap_or_default().to_string();

    if crate::common::first_keyword(&ctx).as_deref() == Some("claim") {
        if !is_complete {
            return Reply::text(format!(
                "❌ Tantangannya belum selesai nih kak!\nProgress kamu baru: *{progress}/{target}*"
            ));
        }
        if claimed {
            return Reply::text(
                "Wah kak, hadiah untuk hari ini udah diambil! Tunggu tantangan baru besok ya! 😉",
            );
        }

        let mut claimed_task = fresh.clone();
        if let Some(obj) = claimed_task.as_object_mut() {
            obj.insert("claimed".into(), serde_json::json!(true));
        }
        rpg.insert("dailyChallenge".into(), claimed_task);

        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, exp);

        return Reply::text(format!(
            "🎉 *TANTANGAN HARIAN SELESAI!!* 🎉\n\n\
             Kerja bagus kak! Ini hadiah dari Guild buat kamu:\n\
             💰 Koin: *+Rp {}*\n\
             ✨ EXP: *+{}*\n\
             \n\n\
             > _Tantangan baru akan diberikan besok pagi!_",
            thousands_id(gold as f64),
            thousands_id(exp as f64)
        ))
        .with_reaction("🎉")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": crate::common::int_field(&ctx, "koin") + gold,
                          "exp": new_exp,
                          "rpg": rpg }),
        });
    }

    let mut txt = format!(
        "📋 *TANTANGAN HARIAN GUILD* 📋\n\n\
         Selesaikan tugas khusus hari ini untuk dapet uang jajan tambahan kak!\n\n\
         \n\
         *Tugas Kamu Hari Ini:*\n\
         🎯 *{name}*\n\
         📊 Progress: *{progress}/{target}*\n\
         Status: {}\n\n\
         *🎁 Hadiah Tambahan:*\n\
         💰 Koin: *Rp {}*\n\
         ✨ EXP: *{}*\n\n",
        if is_complete { "✅ *BISA DICLAIM!*" } else { "⏳ _Sedang dikerjakan..._" },
        thousands_id(gold as f64),
        thousands_id(exp as f64)
    );

    if is_complete && !claimed {
        txt.push_str(&format!(
            "> 💡 Yuk buruan ketik `{}challenge claim` untuk klaim hadiahnya kak!",
            ctx.prefix
        ));
    } else if claimed {
        txt.push_str("> ✅ Kamu hebat! Hadiah sudah diambil. Besok ada misi baru lagi ya!");
    } else {
        txt.push_str("> Semangat kerjainnya kak! Kalau udah selesai nanti ambil hadiahnya.");
    }

    // Challenge baru selalu ditulis,challenge yang masih hari ini juga —
    // `db.save()` berjalan di kedua jalur.
    Reply::text(txt).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "rpg": rpg }),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "challenge".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("challenge").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn task_of(r: &Reply) -> Value {
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data["rpg"]["dailyChallenge"].clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn today_task(progress: i64, target: i64, claimed: bool) -> Value {
        serde_json::json!({ "rpg": { "dailyChallenge": {
            "name": "⚔️ Kalahkan 5 Monster",
            "type": "kill",
            "target": target,
            "reward": { "gold": 500, "exp": 200 },
            "progress": progress,
            "date": today_string(crate::common::now_ms()),
            "claimed": claimed,
        } } })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("challenge").unwrap();
        assert_eq!(p.meta.name, "challenge");
        assert_eq!(p.meta.alias, ["dailychallenge", "tantangan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Daily challenge untuk hadiah spesial");
        assert_eq!(p.meta.usage, ".challenge");
        assert_eq!(p.meta.example, ".challenge");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dailychallenge", "tantangan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "challenge");
        }
    }

    #[test]
    fn the_date_key_uses_the_to_date_string_shape() {
        // 1970-01-01 = Kamis, 1 Januari 1970.
        assert_eq!(today_string(0), "Thu Jan 1 1970");
        assert_eq!(today_string(1_000_000_000_000), "Sun Sep 9 2001");
        assert_eq!(today_string(crate::common::now_ms()).split(' ').count(), 4);
    }

    #[test]
    fn a_stale_date_gets_a_brand_new_challenge() {
        let r = run(
            serde_json::json!({ "rpg": { "dailyChallenge": {
                "name": "LAMA", "target": 99, "reward": { "gold": 1, "exp": 1 },
                "progress": 50, "date": "Thu Jan 1 1970", "claimed": true } } }),
            "",
        );
        let task = task_of(&r);
        assert_ne!(task["name"], serde_json::json!("LAMA"), "tanggal beda -> diacak ulang");
        assert_eq!(task["progress"], serde_json::json!(0));
        assert_eq!(task["claimed"], serde_json::json!(false));
        assert!(CHALLENGES
            .iter()
            .any(|(n, ..)| *n == task["name"].as_str().unwrap()));
    }

    #[test]
    fn the_same_day_keeps_the_stored_challenge_untouched() {
        let r = run(today_task(2, 5, false), "");
        let task = task_of(&r);
        assert_eq!(task["progress"], serde_json::json!(2), "tidak boleh diacak ulang");
        assert_eq!(task["target"], serde_json::json!(5));
        let t = text_of(&r);
        assert!(t.contains("🎯 *⚔️ Kalahkan 5 Monster*"), "{t}");
        assert!(t.contains("📊 Progress: *2/5*"), "{t}");
        assert!(t.contains("⏳ _Sedang dikerjakan..._"), "{t}");
        assert!(t.contains("💰 Koin: *Rp 500*"), "{t}");
    }

    #[test]
    fn an_incomplete_task_cannot_be_claimed() {
        let r = run(today_task(2, 5, false), "claim");
        assert!(text_of(&r).starts_with("❌ Tantangannya belum selesai"));
        assert!(text_of(&r).contains("*2/5*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_claimed_task_cannot_be_claimed_twice() {
        let r = run(today_task(5, 5, true), "claim");
        assert!(text_of(&r).starts_with("Wah kak, hadiah untuk hari ini udah diambil"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_complete_unclaimed_task_pays_once_and_marks_itself() {
        let r = run(today_task(5, 5, false), "claim");
        let t = text_of(&r);
        assert!(t.starts_with("🎉 *TANTANGAN HARIAN SELESAI!!*"), "{t}");
        assert!(t.contains("💰 Koin: *+Rp 500*"), "{t}");
        assert!(t.contains("✨ EXP: *+200*"), "{t}");
        let Mutation::SetUser { data, .. } = &r.mutations()[0] else {
            panic!("harus SetUser");
        };
        assert_eq!(data["koin"], serde_json::json!(500));
        assert_eq!(data["exp"], serde_json::json!(200));
        assert_eq!(data["rpg"]["dailyChallenge"]["claimed"], serde_json::json!(true));
        assert_eq!(data["rpg"]["dailyChallenge"]["progress"], serde_json::json!(5), "progres tak direset");
    }

    #[test]
    fn progress_past_the_target_still_counts_as_complete() {
        let r = run(today_task(9, 5, false), "");
        assert!(text_of(&r).contains("✅ *BISA DICLAIM!*"), "{}", text_of(&r));
        }

    #[test]
    fn the_footer_tells_the_user_which_of_the_three_states_it_is_in() {
        assert!(text_of(&run(today_task(2, 5, false), "")).contains("> Semangat kerjainnya"));
        assert!(text_of(&run(today_task(5, 5, false), "")).contains("Yuk buruan ketik"));
        assert!(text_of(&run(today_task(5, 5, true), "")).contains("Hadiah sudah diambil"));
    }

    #[test]
    fn a_missing_user_row_still_gets_a_challenge_and_persists_it() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "rpg": {} })] {
            let r = run(user.clone(), "");
            assert!(text_of(&r).starts_with("📋 *TANTANGAN HARIAN GUILD*"), "input {user}");
            assert_eq!(r.mutations().len(), 1, "challenge baru harus disimpan");
        }
    }

    #[test]
    fn the_challenge_table_keeps_every_js_row() {
        assert_eq!(CHALLENGES.len(), 7);
        assert_eq!(CHALLENGES[0], ("⚔️ Kalahkan 5 Monster", "kill", 5, 500, 200));
        assert_eq!(CHALLENGES[6], ("🗺️ Selesaikan 2 Ekspedisi", "expedition", 2, 600, 300));
    }
}