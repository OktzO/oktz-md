//! `plugins/rpg/dice.js` — lempar dadu untuk gambling.
//!
//! `ponytail:` `sendRpgPreview` + `setTimeout(2500)` butuh `sock` dan tidur
//! async, yang tidak ada di `Ctx` API (sinkron). Animasi "mengocok dadu"
//! dilewati; validasi, undian, perhitungan kemenang/kehilangan, dan mutasi
//! koin lewat `Mutation::SetUser` dipindah.
//!
//! Catatan: baris `pub mod dice;` perlu ditambah ke `rpg/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{parse_int, roll};

/// `toLocaleString("id-ID")` — pemisah ribuan dengan titik: 1000 → "1.000".
fn format_idr(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn help(prefix: &str) -> String {
    format!(
        "🎲 *Bandar Dadu Jalanan* 🎲\n\n\
         Tebak angka yang bakal keluar! (1-6)\n\n\
         *Cara Main:*\n\
         👉 `.dice <angka> <taruhan>`\n\n\
         *Contoh:*\n\
         👉 `.dice 6 5000`\n\
         Prefix saat ini: `{prefix}`"
    )
}

fn dice_emoji(result: u32) -> &'static str {
    match result {
        1 => "⚀",
        2 => "⚁",
        3 => "⚂",
        4 => "⚃",
        5 => "⚄",
        _ => "⚅",
    }
}

#[plugin(
    name = "dice",
    alias = ["dadu", "roll"],
    category = "rpg",
    description = "Lempar dadu untuk gambling",
    usage = ".dice <1-6> <bet>",
    example = ".dice 6 5000",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dice(ctx: Ctx) -> Reply {
    let guess = ctx
        .args
        .first()
        .and_then(|a| parse_int(a))
        .unwrap_or(0);
    let bet = ctx.args.get(1).and_then(|a| parse_int(a)).unwrap_or(0);

    if !(1..=6).contains(&guess) {
        return Reply::text(help(&ctx.prefix));
    }

    if bet < 1000 {
        return Reply::text(
            "Mana ada bandar nerima taruhan segitu! Minimal bawa *Rp 1.000* sini! 🎲",
        );
    }

    let user = ctx.user_data.clone();
    let koin = user.get("koin").and_then(Value::as_i64).unwrap_or(0);
    if koin < bet {
        return Reply::text(format!(
            "Duit lu kurang bos! Di saku cuma ada *Rp {}*. Jangan ngutang di mari! 😤",
            format_idr(koin)
        ));
    }

    let after_bet = koin - bet;
    let result = roll(6) + 1; // 1..=6
    let emoji = dice_emoji(result);
    let is_win = guess == i64::from(result);

    let mut txt = format!(
        "*MANGKOK DIBUKA!* 🎲💥\n\n\
         Tebakan Lu: *{guess}*\n\
         Mata Dadu: *{result}* {emoji}\n\n"
    );

    let mut new_user = user;
    let final_koin = if is_win { after_bet + bet * 5 } else { after_bet };
    // `user.koin -= bet` / `user.koin = (user.koin || 0) + winnings`
    // (`dice.js:47`, `:63`) — objek user dari store disalin lalu `koin` diganti.
    if new_user.is_object() {
        new_user["koin"] = serde_json::json!(final_koin);
    } else {
        new_user = serde_json::json!({ "koin": final_koin });
    }

    if is_win {
        let winnings = bet * 5;
        txt.push_str(&format!(
            "🎉 *GILA LU HOKI BANGET!*\n\
             💰 Uang Berlipat 5x: *+Rp {}*",
            format_idr(winnings)
        ));
    } else {
        txt.push_str(&format!(
            "🤣 *HAHAHA MAMPUS LU SALAH!*\n\
             💸 Duit Lu Ditarik Bandar: *-Rp {}*",
            format_idr(bet)
        ));
    }

    Reply::text(txt)
        .with_reaction("🎲")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: new_user,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], user_data: Value) -> Reply {
        let args_vec: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        // dice <guess> <bet>
        let ctx = Ctx {
            command: "dice".into(),
            args: args_vec,
            sender: "628111@s.whatsapp.net".into(),
            user_data,
            ..Default::default()
        };
        dice(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("dice").unwrap();
        assert_eq!(p.meta.name, "dice");
        assert_eq!(p.meta.alias, ["dadu", "roll"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn guess_di_luar_1_6_tampilkan_bantuan() {
        for g in ["0", "7", "abc"] {
            let r = run(&[g, "5000"], serde_json::json!({ "koin": 10000 }));
            assert!(body(&r).contains("Bandar Dadu Jalanan"), "guess {g}");
            assert!(r.mutations().is_empty(), "guess {g}");
        }
    }

    #[test]
    fn bet_dibawah_1000_ditolak() {
        let r = run(&["3", "500"], serde_json::json!({ "koin": 10000 }));
        assert!(body(&r).contains("Minimal bawa *Rp 1.000*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn koin_kurang_ditolak() {
        let r = run(&["3", "5000"], serde_json::json!({ "koin": 1000 }));
        assert!(body(&r).contains("Duit lu kurang bos"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn kalah_mengurangi_koin_dan_menyimpan() {
        // result acak: koin jadi 4000 (kalah) atau 9000 (menang, guess==result).
        let r = run(&["1", "1000"], serde_json::json!({ "koin": 5000 }));
        let data = stored(&r);
        let koin = data.get("koin").and_then(Value::as_i64).unwrap_or(-1);
        assert!(
            koin == 4000 || koin == 9000,
            "koin setelah taruhan harus 4000 (kalah) atau 9000 (menang), dapat {koin}"
        );
        assert!(
            body(&r).contains("Duit Lu Ditarik Bandar")
                || body(&r).contains("GILA LU HOKI BANGET"),
            "{}",
            body(&r)
        );
    }

    #[test]
    fn koin_tidak_ada_digunakan_nol() {
        let r = run(&["3", "1000"], serde_json::json!({}));
        assert!(body(&r).contains("Duit lu kurang bos"));
        assert!(r.mutations().is_empty());
    }
}
