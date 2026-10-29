//! `plugins/rpg/work.js` — bekerja untuk mendapatkan uang.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: guard stamina pakai `user.rpg.stamina || 100` (nol falsy),
//! dan `job.name.substring(3)` memotong 3 UNIT UTF-16 — bukan 3 karakter Rust.
//! Untuk `👨‍🌾 Petani` itu berarti emote terakhir ikut tertinggal
//! (`🌾 Petani`), sedangkan `🧹 Cleaning Service` menjadi `Cleaning Service`.
//! Hasil potongnya sudah dihitung di tabel JOB agar tidak salah baca emoji.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 10;

/// `jobs` (`work.js:35-42`) — `(nama tampil, nama tanpa 3 unit UTF-16 pertama,
/// gaji min, gaji max)`. Kolom kedua cuma dipakai pesan antara
/// `Otw berangkat kerja jadi *…* dulu kak!` yang tidak bisa dikirim di port ini;
/// isinya tetap dihitung supaya tidak ada yang salah mengira `chars().skip(3)`
/// sama dengan `substring(3)`.
pub(crate) const JOBS: [(&str, &str, i64, i64); 6] = [
    ("👨‍🌾 Petani", "🌾 Petani", 1000, 3000),
    ("🧹 Cleaning Service", "Cleaning Service", 2000, 5000),
    ("📦 Kurir", "Kurir", 3000, 7000),
    ("👨‍🍳 Koki", "🍳 Koki", 4000, 10000),
    ("👨‍💻 Programmer", "💻 Programmer", 8000, 20000),
    ("👨‍⚕️ Dokter", "⚕️ Dokter", 15000, 30000),
];

#[plugin(
    name = "work",
    alias = ["kerja", "job"],
    category = "rpg",
    description = "Bekerja untuk mendapatkan uang",
    usage = ".work",
    example = ".work",
    cooldown = 180,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn work(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Hadeh kak, badan kamu udah loyo banget! 🥵💦\n\n\
             Kerja butuh *{STAMINA_COST} Stamina*, tapi sisa *{stamina}* doang.\n\
             Istirahat dulu kek, jangan diforsir ntar tepar! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let (label, _bare, min, max) = JOBS[crate::common::random_index(JOBS.len())];
    let salary = roll_incl(min, max);
    let exp_gain = salary / 10;
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "CIE YANG ABIS KERJA! 💸✨\n\n\
         Gila, gajinya lumayan banget nih:\n\
         💼 Profesi: *{label}*\n\
         💵 Gaji Bersih: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: *-{STAMINA_COST}*\n\n\
         Kerja keras bagai quda membuahkan hasil kak! Lanjutkan! 🐴🔥",
        thousands_id(salary as f64)
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": int_field(&ctx, "koin") + salary, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "work".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("work").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("work").unwrap();
        assert_eq!(p.meta.name, "work");
        assert_eq!(p.meta.alias, ["kerja", "job"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Bekerja untuk mendapatkan uang");
        assert_eq!(p.meta.usage, ".work");
        assert_eq!(p.meta.example, ".work");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kerja", "job"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "work");
        }
    }

    #[test]
    fn nine_stamina_is_refused_but_zero_is_not_because_of_the_or_guard() {
        let r = run(serde_json::json!({"rpg": {"stamina": 9}}));
        assert!(text_of(&r).contains("sisa *9* doang"));
        assert!(r.mutations().is_empty());

        let r = run(serde_json::json!({"rpg": {"stamina": 0}, "koin": 0, "exp": 0}));
        assert_eq!(written(&r)["rpg"]["stamina"], serde_json::json!(90), "0 || 100 -> 100 - 10");
    }

    #[test]
    fn each_profession_pays_inside_its_own_band() {
        let mut seen = [false; 6];
        for _ in 0..3000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            let (_, _, min, max) = JOBS
                .iter()
                .find(|(label, _, _, _)| t.contains(&format!("💼 Profesi: *{label}*")))
                .copied()
                .unwrap_or_else(|| panic!("profesi tak dikenal: {t}"));
            let salary: i64 = t
                .split("💵 Gaji Bersih: *+Rp ")
                .nth(1)
                .and_then(|s| s.split('*').next())
                .and_then(|s| s.replace('.', "").parse().ok())
                .unwrap_or_else(|| panic!("gaji tak terbaca: {t}"));
            assert!((min..=max).contains(&salary), "{t}");
            assert_eq!(written(&r)["koin"], serde_json::json!(salary));
            assert_eq!(written(&r)["exp"], serde_json::json!(salary / 10));
            for (i, (label, _, _, _)) in JOBS.iter().enumerate() {
                seen[i] |= t.contains(&format!("💼 Profesi: *{label}*"));
            }
        }
        assert!(seen.iter().all(|s| *s), "enam profesi harus muncul");
    }

    #[test]
    fn the_stripped_job_label_matches_a_three_utf16_unit_cut() {
        // Empat dari enam nama profesinya pakai ZWJ; cutting per SATU CHAR Rust
        // akan salah dan membuang emote yang salah.
        assert_eq!(JOBS[0].1, "🌾 Petani", "👨(2) + ZWJ(1) -> sisa emote kedua");
        assert_eq!(JOBS[1].1, "Cleaning Service", "🧹(2) + spasi(1)");
        assert_eq!(JOBS[2].1, "Kurir");
        assert_eq!(JOBS[4].1, "💻 Programmer");
        assert_eq!(JOBS[5].1, "⚕️ Dokter", "⚕ + VS16 ikut tertinggal");
    }

    #[test]
    fn the_existing_balance_is_added_to_the_paycheque() {
        let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 250_000, "exp": 0}));
        assert!(written(&r)["koin"].as_i64().unwrap() > 250_000);
    }
}
