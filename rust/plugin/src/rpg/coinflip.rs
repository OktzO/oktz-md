//! `plugins/rpg/coinflip.js` — gambling coin flip.
//!
//! Catatan: `mod.rs` kategori `rpg` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod coinflip;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/rpg/mod.rs`.
//!
//! `getDatabase()`, `sendRpgPreview()`, dan `setTimeout` tidak punya padanan
//! di `Ctx`. State user dibaca dari `ctx.user_data` dan ditulis kembali
//! lewat `Mutation::SetUser`. Preview dan jeda 2,5 detik di JS diabaikan
//! (tidak ada I/O sock / penundaan di konteks plugin).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::parse_int;

/// `Number(x).toLocaleString("id-ID")` — pemisah ribuan dengan titik, tanpa
/// desimal. Implementasi sendiri karena Rust tidak punya Intl di std.
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

#[plugin(
    name = "coinflip",
    alias = ["cf", "flip", "toss"],
    category = "rpg",
    description = "Gambling coin flip",
    usage = ".coinflip <heads/tails> <bet>",
    example = ".coinflip heads 5000",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn coinflip(ctx: Ctx) -> Reply {
    let user = ctx.user_data.clone();

    let choice = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if choice.is_empty()
        || !(choice == "heads" || choice == "tails" || choice == "h" || choice == "t")
    {
        return Reply::text(
            "🪙 *Tebak Koin Bandar* 🪙\n\n\
             Pilih gambar Garuda (Heads) atau Angka (Tails)!\n\n\
             *Cara Main:*\n\
             👉 `.coinflip heads <taruhan>`\n\
             👉 `.coinflip tails <taruhan>`",
        );
    }

    let bet = parse_int(ctx.args.get(1).map(|s| s.as_str()).unwrap_or(""));
    let bet = bet.unwrap_or(0);

    if bet < 1000 {
        return Reply::text("Taruhan receh ditolak! Minimal *Rp 1.000* ya bos! 🪙");
    }

    let koin = user
        .get("koin")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    if koin < bet {
        return Reply::text(format!(
            "Mana koin lu? Di kantong sisa *Rp {}* doang, sok mau taruhan *Rp {}*! 😜",
            format_idr(koin),
            format_idr(bet)
        ));
    }

    let user_choice = if choice == "heads" || choice == "h" {
        "heads"
    } else {
        "tails"
    };
    let result = if rand::random::<f64>() < 0.5 {
        "heads"
    } else {
        "tails"
    };
    let emoji = if result == "heads" { "🦅" } else { "🪙" };

    let is_win = user_choice == result;

    let mut new_user = user.clone();
    let rpg = new_user
        .get("rpg")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let remaining_koin = koin - bet;
    new_user["koin"] = serde_json::json!(remaining_koin);

    if is_win {
        let winnings = bet * 2;
        new_user["koin"] = serde_json::json!(remaining_koin + winnings);
        let txt = format!(
            "*PLAK!* Bandar menutup koin di tangannya! 👋\n\n\
             Tebakan Lu: *{}*\n\
             Koin Menunjukkan: *{}* {}\n\n\
             🎉 *MANTAP! TEBAKAN LU BENER!*\n\
             💰 Cuan: *+Rp {}*",
            user_choice.to_uppercase(),
            result.to_uppercase(),
            emoji,
            format_idr(winnings)
        );
        return Reply::text(txt)
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "koin": new_user["koin"], "rpg": rpg }),
            });
    }

    let txt = format!(
        "*PLAK!* Bandar menutup koin di tangannya! 👋\n\n\
         Tebakan Lu: *{}*\n\
         Koin Menunjukkan: *{}* {}\n\n\
         🤣 *HAHAHA! LU SALAH TEBAK!*\n\
         💸 Koin ditarik bandar: *-Rp {}*",
        user_choice.to_uppercase(),
        result.to_uppercase(),
        emoji,
        format_idr(bet)
    );
    Reply::text(txt)
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": remaining_koin, "rpg": rpg }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_sender() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "coinflip".into(),
            user_data: serde_json::json!({ "koin": 10000 }),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("coinflip").unwrap();
        assert_eq!(p.meta.name, "coinflip");
        assert_eq!(p.meta.alias, ["cf", "flip", "toss"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["cf", "flip", "toss"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "coinflip");
        }
    }

    #[test]
    fn pilihan_tidak_valid_mengembalikan_panduan() {
        let r = coinflip(ctx_sender());
        assert!(text_of(&r).contains("🪙 *Tebak Koin Bandar*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn taruhan_dibawah_1000_ditolak() {
        let mut c = ctx_sender();
        c.args = vec!["heads".into(), "500".into()];
        let r = coinflip(c);
        assert!(text_of(&r).contains("Taruhan receh ditolak!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn saldo_tidak_cukup_ditolak() {
        let mut c = ctx_sender();
        c.user_data = serde_json::json!({ "koin": 500 });
        c.args = vec!["heads".into(), "5000".into()];
        let r = coinflip(c);
        assert!(text_of(&r).contains("Mana koin lu?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn format_idr_pemisah_titik() {
        assert_eq!(format_idr(1000), "1.000");
        assert_eq!(format_idr(1000000), "1.000.000");
        assert_eq!(format_idr(500), "500");
        assert_eq!(format_idr(0), "0");
        assert_eq!(format_idr(-1000), "-1.000");
    }

    #[test]
    fn parse_int_dari_args() {
        assert_eq!(parse_int("5000"), Some(5000));
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_int("50px"), Some(50));
    }
}
