//! `plugins/rpg/duel.js` — duel PvP dengan player lain.
//!
//! Catatan: `mod.rs` kategori `rpg` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod duel;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/rpg/mod.rs`.
//!
//! `getDatabase()`, `addExpWithLevelCheck()`, `sendRpgPreview()`, dan
//! `setTimeout` tidak punya padanan di `Ctx`. State player1 dibaca dari
//! `ctx.user_data`, state player2 dari roster grup (`db.getUser(target)`
//! digantikan [`crate::rpg::heal::other_user`]). Preview dan jeda 2 detik
//! diabaikan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::parse_int;
use crate::rpg::heal::other_user;

/// `db.getUser(target) || db.setUser(target)` (`duel.js:46`) — data lawan dari
/// store; row yang belum ada dibuat dengan default `setUser` (`koin` 0,
/// `rpg` {}).
fn opponent(ctx: &Ctx, target: &str) -> Value {
    other_user(ctx, target)
        .cloned()
        .unwrap_or_else(|| json!({ "koin": 0, "rpg": {} }))
}

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

fn jid_number(jid: &str) -> &str {
    jid.split('@').next().unwrap_or(jid)
}

fn rpg_level(rpg: &Value) -> i64 {
    rpg.get("level").and_then(Value::as_i64).unwrap_or(1)
}

fn rpg_health(rpg: &Value) -> i64 {
    rpg.get("health").and_then(Value::as_i64).unwrap_or(100)
}

fn rpg_koin(user: &Value) -> i64 {
    user.get("koin").and_then(Value::as_i64).unwrap_or(0)
}

#[plugin(
    name = "duel",
    alias = ["pvp", "fight"],
    category = "rpg",
    description = "Duel PvP dengan player lain",
    usage = ".duel @user <bet>",
    example = ".duel @user 5000",
    cooldown = 120,
    energi = 1,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn duel(ctx: Ctx) -> Reply {
    // `m.mentionedJid?.[0] || m.quoted?.sender` (`duel.js:25`).
    let target = ctx
        .mentioned
        .first()
        .cloned()
        .or_else(|| ctx.quoted().map(|q| q.sender.clone()));

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => {
            return Reply::text(
                "⚔️ *DUEL TARUHAN* ⚔️\n\n\
                 Tantang teman kamu untuk duel dengan uang taruhan kak!\n\n\
                 *Cara Menantang:*\n\
                 👉 `.duel @user 5000`\n\
                 _(Artinya kamu ngajak dia duel dengan taruhan Rp 5.000)_",
            )
        }
    };

    if target == ctx.sender {
        return Reply::text(
            "Hihihi kak, masak kamu mau ngajak berantem sama cermin? Tag teman yang lain yuk! 😂",
        );
    }

    // `args[1] ? parseInt(args[1]) : 1000` (`duel.js:26`) — `parseInt` yang
    // gagal berarti `NaN`, dan `isNaN(bet)` menolaknya (`duel.js:41`); di sini
    // `NaN` jatuh ke 0 supaya gerbang `< 1000` yang sama menolak.
    let bet = ctx
        .args
        .get(1)
        .map(|s| parse_int(s).unwrap_or(0))
        .unwrap_or(1000);

    if bet < 1000 {
        return Reply::text("Taruhan tidak valid! Minimal *Rp 1.000*. Gunakan angka.");
    }

    let player1 = ctx.user_data.clone();
    let player2 = opponent(&ctx, &target);

    if rpg_koin(&player1) < bet {
        return Reply::text(format!(
            "Aduh kak, saldo kamu nggak cukup buat pasang taruhan segitu!\n\
             Koin kamu sekarang: *Rp {}*",
            format_idr(rpg_koin(&player1))
        ));
    }

    if rpg_koin(&player2) < bet {
        return Reply::text(
            "Yah kak, sepertinya saldo lawan kamu nggak cukup buat meladeni taruhan ini. Cari lawan lain atau turunin taruhannya ya!",
        );
    }

    let mut p1_rpg = player1.get("rpg").cloned().unwrap_or(serde_json::json!({}));
    let mut p2_rpg = player2.get("rpg").cloned().unwrap_or(serde_json::json!({}));
    if p1_rpg.get("health").is_none() {
        p1_rpg["health"] = serde_json::json!(100);
    }
    if p2_rpg.get("health").is_none() {
        p2_rpg["health"] = serde_json::json!(100);
    }

    if rpg_health(&p1_rpg) < 30 {
        return Reply::text(format!(
            "Eh tunggu kak! Darah kamu sekarat banget (*{} HP*). Minimal harus punya *30 HP* buat ikut duel. Istirahat dulu yuk! 💉",
            rpg_health(&p1_rpg)
        ));
    }

    let p1_power = rpg_level(&p1_rpg) * 10 + (rand::random::<f64>() * 50.0) as i64;
    let p2_power = rpg_level(&p2_rpg) * 10 + (rand::random::<f64>() * 50.0) as i64;

    let (winner_jid, loser_jid) = if p1_power > p2_power {
        (ctx.sender.clone(), target.clone())
    } else {
        (target.clone(), ctx.sender.clone())
    };

    let winner_data = if winner_jid == ctx.sender {
        player1.clone()
    } else {
        player2.clone()
    };
    let loser_data = if loser_jid == ctx.sender {
        player1.clone()
    } else {
        player2.clone()
    };

    let winner_koin = rpg_koin(&winner_data) + bet;
    let loser_koin = rpg_koin(&loser_data) - bet;
    let loser_health = (rpg_health(&p2_rpg).saturating_sub(20)).max(0);

    const EXP_GAIN: i64 = 500;

    let txt = format!(
        "⚔️ *HASIL DUEL BERDARAH* ⚔️\n\n\
         🏆 *Pemenang:* @{}\n\
         💀 *Kalah:* @{} (Mundur dengan luka parah)\n\n\
         🎁 *Pemenang Berhak Membawa Pulang:*\n\
         > 💰 Uang Taruhan Lawan: *+Rp {}*\n\
         > ✨ Bonus EXP Pertarungan: *+{} EXP*",
        jid_number(&winner_jid),
        jid_number(&loser_jid),
        format_idr(bet),
        EXP_GAIN
    );

    Reply::text(txt)
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({
                "koin": if ctx.sender == winner_jid { winner_koin } else { loser_koin },
                "rpg": if ctx.sender == winner_jid { p1_rpg } else { serde_json::json!({ "level": rpg_level(&p1_rpg), "health": rpg_health(&p1_rpg) }) },
            }),
        })
        .with_mutation(Mutation::SetUser {
            jid: target.clone(),
            data: serde_json::json!({
                "koin": if target == winner_jid { winner_koin } else { loser_koin },
                "rpg": if target == winner_jid { p2_rpg } else { serde_json::json!({ "level": rpg_level(&p2_rpg), "health": loser_health }) },
            }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "123456@g.us".into(),
            prefix: ".".into(),
            command: "duel".into(),
            user_data: serde_json::json!({ "koin": 5000, "rpg": { "level": 5, "health": 100 } }),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("duel").unwrap();
        assert_eq!(p.meta.name, "duel");
        assert_eq!(p.meta.alias, ["pvp", "fight"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["pvp", "fight"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "duel");
        }
    }

    #[test]
    fn tanpa_target_mengembalikan_panduan() {
        let mut c = ctx();
        c.mentioned = vec![];
        let r = duel(c);
        assert!(text_of(&r).contains("⚔️ *DUEL TARUHAN*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn duel_diri_sendiri_ditolak() {
        let mut c = ctx();
        c.mentioned = vec!["628111@s.whatsapp.net".into()];
        let r = duel(c);
        assert!(text_of(&r).contains("cermin"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn taruhan_kurang_dari_1000_ditolak() {
        let mut c = ctx();
        c.args = vec!["@628999".into(), "500".into()];
        let r = duel(c);
        assert!(text_of(&r).contains("Minimal *Rp 1.000*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn saldo_tidak_cukup_ditolak() {
        let mut c = ctx();
        c.user_data = serde_json::json!({ "koin": 500, "rpg": { "level": 5, "health": 100 } });
        c.args = vec!["@628999".into(), "5000".into()];
        let r = duel(c);
        assert!(text_of(&r).contains("saldo kamu nggak cukup"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn health_dibawah_30_ditolak() {
        let mut c = ctx();
        c.user_data = serde_json::json!({ "koin": 5000, "rpg": { "level": 5, "health": 20 } });
        c.args = vec!["@628999".into(), "1000".into()];
        // `db.getUser(target) || db.setUser(target)` (`duel.js:46`): row lawan
        // yang belum ada dibuat default `koin` 0, jadi gerbang saldo lawan
        // (`duel.js:52`) menangkap lebih dulu. Data user lain hanya terbaca
        // dari roster, jadi lawan harus punya row di situ.
        c.roster.participants.push(Participant {
            jid: "628999@s.whatsapp.net".into(),
            data: serde_json::json!({ "koin": 5000, "rpg": { "level": 5, "health": 100 } }),
            ..Default::default()
        });
        let r = duel(c);
        assert!(text_of(&r).contains("sekarat banget"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn format_idr_pemisah_titik() {
        assert_eq!(format_idr(1000), "1.000");
        assert_eq!(format_idr(1000000), "1.000.000");
        assert_eq!(format_idr(500), "500");
        assert_eq!(format_idr(0), "0");
    }
}
