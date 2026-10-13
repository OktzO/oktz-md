//! `plugins/rpg/quest.js` — papan bounty misi harian.
//!
//! Hadiah quest lewat `db.updateExp`, BUKAN `addExpWithLevelCheck` — jadi exp
//! naik tanpa stats `rpg` ikut terisi ulang walau level naik.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::{int_field, now_ms};
use crate::rpg::jualan::{exp_gain_only, or_default};
use crate::tools::kalkulatormbg::thousands_id;

/// `QUESTS` (`quest.js:19-55`) — `(id, nama, deskripsi, target, uang, exp)`.
pub(crate) const QUESTS: [(&str, &str, &str, i64, i64, i64); 5] = [
    ("mining5", "Penambang Pemula", "Mining 5 kali", 5, 10000, 1000),
    ("fishing5", "Pemancing Handal", "Fishing 5 kali", 5, 8000, 800),
    (
        "adventure3",
        "Petualang Sejati",
        "Adventure 3 kali",
        3,
        15000,
        1500,
    ),
    ("work10", "Pekerja Keras", "Work 10 kali", 10, 20000, 2000),
    ("hunt5", "Pemburu Ulung", "Hunt 5 kali", 5, 12000, 1200),
];

pub(crate) fn find_quest(id: &str) -> Option<&'static (&'static str, &'static str, &'static str, i64, i64, i64)> {
    QUESTS.iter().find(|q| q.0 == id)
}

/// `user.quest[id]` — `None` kalau misi belum pernah diambil.
fn quest_of<'a>(user: &'a Value, id: &str) -> Option<&'a Value> {
    user.get("quest")?.get(id)
}

fn quest_map(user: &Value) -> serde_json::Map<String, Value> {
    user.get("quest")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

#[plugin(
    name = "quest",
    alias = ["misi", "mission", "bounty"],
    category = "rpg",
    description = "Ambil quest harian untuk reward bonus",
    usage = ".quest",
    example = ".quest",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn quest(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let user = &ctx.user_data;

    if sub.as_deref() == Some("claim") {
        let Some(id) = ctx.args.get(1) else {
            return Reply::text("Hmm.. Misi itu nggak ada di daftar kamu kak! 📜❌");
        };
        let Some(entry) = quest_of(user, id) else {
            return Reply::text("Hmm.. Misi itu nggak ada di daftar kamu kak! 📜❌");
        };
        let Some(&(_, name, _, target, money, exp_reward)) = find_quest(id) else {
            return Reply::text("ID misinya salah kak! Cek lagi papan bounty ya! 🔍");
        };
        let progress = entry.get("progress").and_then(Value::as_i64).unwrap_or(0);
        if progress < target {
            return Reply::text(format!(
                "Misi ini belum selesai kak!\nProgress kamu: *{progress}/{target}* 🏃‍♂️💦"
            ));
        }
        if crate::common::js_truthy(entry.get("claimed")) {
            return Reply::text("Hadiah misi ini udah kamu ambil kak! 😒");
        }

        let mut quests = quest_map(user);
        if let Some(e) = quests.get_mut(id).and_then(Value::as_object_mut) {
            e.insert("claimed".into(), json!(true));
        }
        let koin = int_field(&ctx, "koin") + money;
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);

        return Reply::text(format!(
            "💰 *MISI SELESAI!!* 💰\n\n\
             Kamu berhasil nyelesaiin misi *{name}*!\n\
             Ini hadiah buat kamu kak:\n\
             💵 Uang Misi: *+Rp {}*\n\
             📈 EXP Bonus: *+{exp_reward}*\n\n\
             > _\"Kerja bagus kak!\" - Resepsionis Guild_ 👩‍💼",
            thousands_id(money as f64)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({
                "koin": koin,
                "exp": exp_gain_only(old_exp, exp_reward),
                "quest": quests,
            }),
        });
    }

    if sub.as_deref() == Some("take") {
        let Some(id) = ctx.args.get(1).cloned() else {
            return Reply::text("Misi nggak ketemu! Liat daftar lengkapnya di `.quest`");
        };
        let Some(&(_, name, desc, _, money, exp_reward)) = find_quest(&id) else {
            return Reply::text("Misi nggak ketemu! Liat daftar lengkapnya di `.quest`");
        };
        if quest_of(user, &id).is_some() {
            return Reply::text("Kamu udah ngambil misi ini kak! Kerjain dulu yuk! ⚔️");
        }
        let mut quests = quest_map(user);
        quests.insert(
            id.clone(),
            json!({ "progress": 0, "claimed": false, "takenAt": now_ms() }),
        );
        return Reply::text(format!(
            "📜 *MISI DIAMBIL!* 📜\n\n\
             Kamu mengambil satu kertas misi dari Papan Bounty! 📜✨\n\
             🎯 Target: *{name}* ({desc})\n\
             🎁 Hadiah: *Rp {}* & *{exp_reward} EXP*\n\n\
             > _\"Semoga berhasil di perjalanan ya kak!\"_ 💖",
            thousands_id(money as f64)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "quest": quests }),
        });
    }

    let mut txt = String::from(
        "📌 *PAPAN BOUNTY (MISI HARIAN)* 📌\n\n\
         Selesaikan tugas harian ini buat dapat hadiah bonus yuk kak!\n\n",
    );
    for (id, name, desc, target, money, exp_reward) in QUESTS {
        let status = match quest_of(user, id) {
            Some(entry) if crate::common::js_truthy(entry.get("claimed")) => "✅ Selesai".to_string(),
            Some(entry) if entry.get("progress").and_then(Value::as_i64).unwrap_or(0) >= target => {
                "🎁 Siap Claim".to_string()
            }
            Some(entry) => format!(
                "🏃 Sedang Dikerjakan ({}/{})",
                entry.get("progress").and_then(Value::as_i64).unwrap_or(0),
                target
            ),
            None => "📜 Tersedia".to_string(),
        };
        txt += &format!(
            "🎯 *{name}*\n   ├ Tugas: {desc}\n   ├ Reward: Rp {} & {exp_reward} EXP\n   \
             ├ Status: *{status}*\n   └ Ambil: `{}quest take {id}`\n\n",
            thousands_id(money as f64),
            ctx.prefix
        );
    }
    txt += "> 💡 Kalau misi udah selesai, ketik: `.quest claim <id_misi>`";

    Reply::text(txt)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "quest".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("quest").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("quest").unwrap();
        assert_eq!(p.meta.name, "quest");
        assert_eq!(p.meta.alias, ["misi", "mission", "bounty"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Ambil quest harian untuk reward bonus");
        assert_eq!(p.meta.usage, ".quest");
        assert_eq!(p.meta.example, ".quest");
        assert_eq!(p.meta.cooldown, 0, "cooldown plugin nil explicit 0");
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["misi", "mission", "bounty"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "quest");
        }
    }

    #[test]
    fn the_board_lists_five_quests_with_their_rewards_and_a_take_hint() {
        let r = run(&[], json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("📌 *PAPAN BOUNTY (MISI HARIAN)*"), "{t}");
        for (id, name, desc, target, money, exp) in QUESTS {
            assert!(t.contains(&format!("🎯 *{name}*\n   ├ Tugas: {desc}")), "{id}: {t}");
            assert!(
                t.contains(&format!("   ├ Reward: Rp {} & {exp} EXP", thousands_id(money as f64))),
                "{id}: {t}"
            );
            assert!(t.contains("   ├ Status: *📜 Tersedia*"), "{id}: {t}");
            assert!(t.contains(&format!("   └ Ambil: `.quest take {id}`")), "{id}: {t}");
            assert!(target > 0);
        }
        assert!(t.ends_with("`.quest claim <id_misi>`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_board_reads_the_four_states_of_a_stored_quest() {
        for (stored, expected) in [
            (json!({"progress": 1, "claimed": false}), "🏃 Sedang Dikerjakan (1/5)"),
            (json!({"progress": 5, "claimed": false}), "🎁 Siap Claim"),
            (json!({"progress": 5, "claimed": true}), "✅ Selesai"),
            (json!({"progress": 0, "claimed": true}), "✅ Selesai"),
        ] {
            let t = text(run(&[], json!({ "quest": { "mining5": stored } })));
            assert!(t.contains(expected), "{stored} -> {t}");
        }
    }

    #[test]
    fn taking_a_quest_stamps_the_row_with_a_real_timestamp() {
        let r = run(&["take", "hunt5"], json!({}));
        let entry = &written(&r)["quest"]["hunt5"];
        assert_eq!(entry["progress"], json!(0));
        assert_eq!(entry["claimed"], json!(false));
        assert!(entry["takenAt"].as_i64().unwrap() > 1_600_000_000_000);
        assert_eq!(written(&r).get("koin"), None, "take tidak membayar");
    }

    #[test]
    fn taking_the_same_quest_twice_is_refused() {
        let r = run(
            &["take", "hunt5"],
            json!({"quest": {"hunt5": {"progress": 2, "claimed": false}}}),
        );
        assert_eq!(text_of(&r), "Kamu udah ngambil misi ini kak! Kerjain dulu yuk! ⚔️");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_quest_id_cannot_be_taken_or_claimed() {
        let r = run(&["take", "boss9"], json!({}));
        assert_eq!(text_of(&r), "Misi nggak ketemu! Liat daftar lengkapnya di `.quest`");

        // Row-nya ADA tapi id-nya tak dikenal di `QUESTS`: `!user.quest[id]`
        // lolos, jadi yang menolak adalah `QUESTS.find`.
        let r = run(&["claim", "boss9"], json!({"quest": {"boss9": {"progress": 99}}}));
        assert_eq!(text_of(&r), "ID misinya salah kak! Cek lagi papan bounty ya! 🔍");

        // Row-nya tidak ada: guard pertama yang menolak.
        let r = run(&["claim", "boss9"], json!({}));
        assert_eq!(text_of(&r), "Hmm.. Misi itu nggak ada di daftar kamu kak! 📜❌");
    }

    #[test]
    fn claiming_needs_a_known_id_even_when_the_row_exists() {
        let r = run(&["claim", "nope"], json!({"quest": {"nope": {"progress": 9}}}));
        assert_eq!(text_of(&r), "ID misinya salah kak! Cek lagi papan bounty ya! 🔍");
    }

    #[test]
    fn an_unfinished_quest_reports_the_real_progress() {
        let r = run(
            &["claim", "work10"],
            json!({"quest": {"work10": {"progress": 4, "claimed": false}}}),
        );
        let t = text_of(&r);
        assert!(t.contains("Progress kamu: *4/10*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_finished_quest_pays_once_and_flips_the_claim_flag() {
        let r = run(
            &["claim", "adventure3"],
            json!({"koin": 500, "exp": 1000, "quest": {"adventure3": {"progress": 3, "claimed": false}}}),
        );
        let d = written(&r);
        assert_eq!(d["koin"], json!(15500), "500 + 15.000");
        assert_eq!(d["exp"], json!(2500), "updateExp, bukan addExpWithLevelCheck");
        assert!(d.get("rpg").is_none(), "updateExp tidak mengisi rpg");
        assert_eq!(d["quest"]["adventure3"]["claimed"], json!(true));
        assert_eq!(d["quest"]["adventure3"]["progress"], json!(3), "progress tak diubah");

        let again = run(
            &["claim", "adventure3"],
            json!({"quest": {"adventure3": {"progress": 3, "claimed": true}}}),
        );
        assert_eq!(text_of(&again), "Hadiah misi ini udah kamu ambil kak! 😒");
        assert!(again.mutations().is_empty());
    }
}
