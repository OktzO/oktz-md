//! `plugins/rpg/streamer.js` — live streaming, donasi besar tapi bisa kena ban.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: cabang ban menulis `db.db.data.users[nomor].lastStreamer`
//! langsung ke objek store, bukan lewat `db.setUser`. Karena `getUser` juga
//! mengembalikan objek live yang sama, ini identik dengan menulis field
//! `lastStreamer` pada row itu — begitu juga pengurangan stamina yang terjadi
//! beberapa baris sebelumnya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::common::now_ms;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 20;

/// `const extraCooldown = 300` dalam detik.
pub(crate) const EXTRA_COOLDOWN_MS: i64 = 300 * 1000;

#[plugin(
    name = "streamer",
    alias = ["live", "vtuber"],
    category = "rpg",
    description = "Live streaming game dapet donasi besar tapi resiko dibanned platform!",
    usage = ".streamer",
    example = ".streamer",
    cooldown = 180,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn streamer(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Mata sepet kebanyakan natap layar! 😵\n\n\
             Streaming butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Tidur dulu woi! 🛏️"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.15 {
        return Reply::text(format!(
            "AKUN STREAMING KAMU DI-BANNED! 🚫😱\n\n\
             Ada viewers iseng yang ngereport akun kamu gara-gara afk terlalu lama!\n\
             Kamu tidak dapet donasi apa-apa dan **terkena penalti larangan streaming selama 5 menit ekstra**!\n\n\
             ⚡ Stamina tetap berkurang: -{STAMINA_COST}\n\n\
             Sabar ya bang, coba ajuin banding... 😔"
        ))
        .with_reaction("🚫")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "lastStreamer": now_ms() + EXTRA_COOLDOWN_MS, "rpg": rpg }),
        });
    }

    let earning = roll_incl(10000, 39999);
    let saweran = if gacha > 0.85 {
        roll_incl(50000, 149999)
    } else {
        0
    };
    let total = earning + saweran;
    let exp_gain = total / 30;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = format!(
        "LIVE STREAMING SELESAI! 🎥✨\n\n\
         💵 Pendapatan Iklan: *+Rp {}*\n",
        thousands_id(earning as f64)
    );
    if saweran > 0 {
        txt += &format!(
            "🐳 SAWERAN SULTAN: *+Rp {}*\nWah, ada penonton sultan nyawer paus! 🐋🔥\n",
            thousands_id(saweran as f64)
        );
    }
    txt += &format!(
        "📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Makasih yang udah nyawer! Lopyu pull! 💖"
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin + total, "exp": new_exp, "rpg": rpg }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "streamer".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("streamer").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("streamer").unwrap();
        assert_eq!(p.meta.name, "streamer");
        assert_eq!(p.meta.alias, ["live", "vtuber"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Live streaming game dapet donasi besar tapi resiko dibanned platform!"
        );
        assert_eq!(p.meta.usage, ".streamer");
        assert_eq!(p.meta.example, ".streamer");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["live", "vtuber"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "streamer");
        }
    }

    #[test]
    fn tired_eyes_end_the_stream_before_it_starts() {
        let r = run(json!({"rpg": {"stamina": 19}}));
        let t = text_of(&r);
        assert!(t.starts_with("Mata sepet kebanyakan natap layar!"), "{t}");
        assert!(t.contains("sisa stamina kamu *19*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_stream_costs_twenty_stamina() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(80));
        }
    }

    #[test]
    fn a_ban_writes_a_five_minute_penalty_and_no_money() {
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 5000, "exp": 0}));
            let d = written(&r);
            let t = text_of(&r);
            if t.starts_with("AKUN STREAMING KAMU DI-BANNED") {
                let until = d["lastStreamer"].as_i64().unwrap();
                let delta = until - now_ms();
                assert!((295_000..=300_000).contains(&delta), "sisa penalti {delta}ms");
                assert_eq!(d["rpg"]["stamina"], json!(80), "stamina tetap hangsus");
                assert!(d.get("koin").is_none() && d.get("exp").is_none());
                return;
            }
        }
        panic!("cabang ban tidak terjadi");
    }

    /// Semua nominal `*+Rp …*` di balasan, urutAppearance.
    fn rupiahs(t: &str) -> Vec<i64> {
        t.split("*+Rp ")
            .skip(1)
            .map(|s| {
                s[..s.find('*').expect("nominal harus ditutup *")]
                    .replace('.', "")
                    .parse()
                    .expect("nominal harus angka")
            })
            .collect()
    }

    #[test]
    fn ad_revenue_and_the_sultan_donation_add_up_into_the_purse() {
        let mut sultan = 0;
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 1000, "exp": 0}));
            let t = text_of(&r);
            if t.starts_with("AKUN STREAMING") {
                continue;
            }
            let d = written(&r);
            let koin = d["koin"].as_i64().unwrap() - 1000;
            let parts = rupiahs(t);
            assert_eq!(parts.len(), 1 + usize::from(t.contains("SAWERAN SULTAN")), "{t}");
            assert_eq!(koin, parts.iter().sum::<i64>(), "kantong = total: {t}");
            assert!((10000..=39999).contains(&parts[0]), "iklan {}", parts[0]);
            if parts.len() == 2 {
                sultan += 1;
                assert!((50000..=149999).contains(&parts[1]), "saweran {}", parts[1]);
            }
            assert_eq!(d["exp"].as_i64().unwrap(), koin / 30);
        }
        assert!(sultan > 0, "gacha > 0.85 harus muncul");
    }
}
