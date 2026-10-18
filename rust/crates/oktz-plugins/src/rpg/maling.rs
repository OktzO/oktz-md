//! `plugins/rpg/maling.js` — mencopet orang di pasar.
//!
//! Catatan: `mod.rs` kategori `rpg` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod maling;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/rpg/mod.rs`.
//!
//! `getDatabase()`, `addExpWithLevelCheck()`, dan `sendRpgPreview()` tidak
//! punya padanan di `Ctx`. State dibaca dari `ctx.user_data` dan ditulis
//! kembali lewat `Mutation::SetUser`. Preview dan jeda 2,5 detik diabaikan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

fn format_idr(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let mut formatted: String = out.chars().rev().collect();
    if n < 0 {
        formatted = format!("-{formatted}");
    }
    formatted
}

struct Outcome {
    success: bool,
    money: i64,
    exp: i64,
    fine: i64,
    health: i64,
    msg: &'static str,
}

const OUTCOMES: [Outcome; 6] = [
    Outcome {
        success: true,
        money: 20000,
        exp: 500,
        fine: 0,
        health: 0,
        msg: "GILA! Lu dapet dompet isinya *ATM Black Card* sama duit gepokan! 🤑",
    },
    Outcome {
        success: true,
        money: 8000,
        exp: 200,
        fine: 0,
        health: 0,
        msg: "Lumayan lah, dapet dompet kulit asli isi cepek puluhan ribu. 😏",
    },
    Outcome {
        success: true,
        money: 2000,
        exp: 50,
        fine: 0,
        health: 0,
        msg: "Apes lu, dapet dompet bapak-bapak isinya KTP sama struk Indomaret doang! 😑 Tapi ada nyelip dikit duit.",
    },
    Outcome {
        success: false,
        money: 0,
        exp: 0,
        fine: 15000,
        health: 30,
        msg: "WOY MALING!!! 😱 Emak-emak teriak dan lu *dihakimi massa* ampe gigi copot!",
    },
    Outcome {
        success: false,
        money: 0,
        exp: 0,
        fine: 25000,
        health: 10,
        msg: "Lagi asik nyeloteh dompet, eh tangan lu *digenggam intel* nyamar! 👮‍♂️ Busted!",
    },
    Outcome {
        success: false,
        money: 0,
        exp: 0,
        fine: 0,
        health: 0,
        msg: "Sialan! Target ngerasa ada yang raba-raba tasnya, langsung kabur di kerumunan! 😤 Gagal bro.",
    },
];

const WEIGHTS: [i64; 6] = [5, 20, 30, 15, 10, 20];

fn pick_outcome() -> &'static Outcome {
    let rand = rand::random::<f64>() * 100.0;
    let mut cumulative = 0i64;
    for (i, w) in WEIGHTS.iter().enumerate() {
        cumulative += w;
        if rand <= cumulative as f64 {
            return &OUTCOMES[i];
        }
    }
    &OUTCOMES[5]
}

#[plugin(
    name = "maling",
    alias = ["copet"],
    category = "rpg",
    description = "Mencopet orang di pasar (lebih berisiko dari crime)",
    usage = ".maling",
    example = ".maling",
    cooldown = 180,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn maling(ctx: Ctx) -> Reply {
    let mut user = ctx.user_data.clone();

    let mut rpg = user.get("rpg").cloned().unwrap_or(serde_json::json!({}));
    if rpg.get("health").is_none() {
        rpg["health"] = serde_json::json!(100);
    }
    user["rpg"] = rpg.clone();

    let health = rpg.get("health").and_then(Value::as_i64).unwrap_or(100);
    if health < 40 {
        return Reply::text(format!(
            "Napas lu aja udah ngos-ngosan nyuruh nyopet! 🤒\n\
             Minimal *40 HP* ya bos, darah lu sekarang *{} HP*. Tidur dulu gih!",
            health
        ));
    }

    let outcome = pick_outcome();

    if outcome.success {
        let koin = user.get("koin").and_then(Value::as_i64).unwrap_or(0);
        let new_koin = koin + outcome.money;
        user["koin"] = serde_json::json!(new_koin);

        let txt = format!(
            "OPERASI BERSIH! 🦹‍♂️✨\n\n\
             {}\n\n\
             💰 Koin Haram: *+Rp {}*\n\
             📈 EXP Copet: *+{}*",
            outcome.msg,
            format_idr(outcome.money),
            outcome.exp
        );
        return Reply::text(txt).with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": new_koin, "rpg": rpg }),
        });
    }

    let koin = user.get("koin").and_then(Value::as_i64).unwrap_or(0);
    let actual_fine = outcome.fine.min(koin);
    let new_koin = (koin - actual_fine).max(0);
    let new_health = (health.saturating_sub(outcome.health)).max(0);

    user["koin"] = serde_json::json!(new_koin);
    rpg["health"] = serde_json::json!(new_health);
    user["rpg"] = rpg.clone();

    let mut txt = format!(
        "KACAU BALAU!! 🚨🤬\n\n\
         {}\n\n",
        outcome.msg
    );
    if outcome.fine > 0 {
        txt.push_str(&format!("💸 Duit Damai/Rampasan: *-Rp {}*\n", format_idr(actual_fine)));
    }
    if outcome.health > 0 {
        txt.push_str(&format!("🤕 Darah Bercucuran: *-{} HP*", outcome.health));
    }

    if new_health <= 0 {
        rpg["health"] = serde_json::json!(0);
        user["rpg"] = rpg.clone();
        let exp = user.get("exp").and_then(Value::as_i64).unwrap_or(0);
        let new_exp = (exp / 2).max(0);
        user["exp"] = serde_json::json!(new_exp);
        txt.push_str(&format!(
            "\n\n💀 *INNALILLAHI... KAMU MATI GEGARA DIGEBUKIN MASSA!*\n\
             Exp-mu hangus 50%! 😭"
        ));
    }

    Reply::text(txt).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": new_koin, "rpg": rpg, "exp": user.get("exp") }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "maling".into(),
            user_data: serde_json::json!({ "koin": 50000, "rpg": { "health": 100 }, "exp": 1000 }),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("maling").unwrap();
        assert_eq!(p.meta.name, "maling");
        assert_eq!(p.meta.alias, ["copet"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn alias_copet_menunjuk_ke_maling() {
        assert_eq!(oktz_plugin_api::find("copet").unwrap().meta.name, "maling");
    }

    #[test]
    fn health_dibawah_40_ditolak() {
        let mut c = ctx();
        c.user_data = serde_json::json!({ "koin": 50000, "rpg": { "health": 30 } });
        let r = maling(c);
        assert!(text_of(&r).contains("Minimal *40 HP*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn weighted_outcomes_semua_terjangkau() {
        let mut seen = [false; 6];
        for _ in 0..5000 {
            let o = pick_outcome() as *const Outcome as usize;
            for (i, out) in OUTCOMES.iter().enumerate() {
                let ptr = out as *const Outcome as usize;
                if ptr == o {
                    seen[i] = true;
                }
            }
        }
        assert!(seen.iter().all(|&s| s), "setiap outcome harus terpilih");
    }

    #[test]
    fn format_idr_pemisah_titik() {
        assert_eq!(format_idr(1000), "1.000");
        assert_eq!(format_idr(1000000), "1.000.000");
        assert_eq!(format_idr(500), "500");
    }
}
