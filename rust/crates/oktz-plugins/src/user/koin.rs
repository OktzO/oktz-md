//! `plugins/user/koin.js` — baca `users[sender].koin`.
//!
//! `ponytail:` `Ctx::user_data` hanya diisi untuk `sender` (lihat
//! `oktz-wa/src/pipeline.rs`), sedangkan JS membaca `db.getUser(targetJid)`
//! setelah mention. Untuk target selain sender angka yang tampil adalah milik
//! pengirim. Tambahkan `user_data_for(target)` ke `Ctx` kalau router mau
//! membaca state target.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{int_field, user_target};

/// `formatKoin` (`koin.js:12-17`) — batas bawah eksklusif untuk tiap satuan.
pub(crate) fn format_koin(num: i64) -> String {
    let n = num as f64;
    if n >= 1_000_000_000_000.0 {
        return format!("{:.2}T", n / 1_000_000_000_000.0);
    }
    if n >= 1_000_000_000.0 {
        return format!("{:.2}B", n / 1_000_000_000.0);
    }
    if n >= 1_000_000.0 {
        return format!("{:.2}M", n / 1_000_000.0);
    }
    if n >= 1_000.0 {
        return format!("{:.2}K", n / 1_000.0);
    }
    crate::tools::kalkulatormbg::thousands_id(n)
}

/// `config.isOwner(targetJid)` — hanya bisa dijawab untuk pengirim; `Ctx`
/// membawa satu flag untuk satu orang.
pub(crate) fn status_of(ctx: &Ctx) -> &'static str {
    if ctx.is_owner {
        "Owner"
    } else if ctx.user_field("isPremium").and_then(Value::as_bool) == Some(true) {
        "Premium"
    } else {
        "Free"
    }
}

pub(crate) fn render(ctx: &Ctx, target: &crate::common::UserTarget) -> String {
    let is_self = target.jid == ctx.sender;
    let koin_display = format_koin(int_field(ctx, "koin"));

    let mut text = String::from("*〔 💰 KOIN INFO 〕*\n\n");
    text.push_str(&format!("*〔 👤 User 〕* {}\n", target.name));
    text.push_str(&format!("*〔 💰 Koin 〕* {koin_display}\n"));
    text.push_str(&format!("*〔 💎 Status 〕* {}\n", status_of(ctx)));

    if is_self {
        text.push_str("\n*〔 🛒 SHOP 〕*\n");
        text.push_str("• `.buyenergi <jml>` (1 = 100 koin)\n");
        text.push_str("• `.buyfitur` (1 = 3000 koin)\n");
        text.push_str("\n_🎮 Main game untuk dapat koin!_");
    }
    text
}

#[plugin(
    name = "koin",
    alias = ["saldo", "money", "cash", "coin", "coins"],
    category = "user",
    description = "Cek koin user",
    usage = ".koin [@user]",
    example = ".koin",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn koin(ctx: Ctx) -> Reply {
    let target = user_target(&ctx);
    Reply::text(render(&ctx, &target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: serde_json::Value, mentioned: Vec<&str>, is_owner: bool) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "koin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            is_owner,
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("koin").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("koin").unwrap();
        assert_eq!(p.meta.name, "koin");
        assert_eq!(p.meta.alias, ["saldo", "money", "cash", "coin", "coins"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Cek koin user");
        assert_eq!(p.meta.usage, ".koin [@user]");
        assert_eq!(p.meta.example, ".koin");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_private && !p.meta.is_group);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["saldo", "money", "cash", "coin", "coins"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "koin");
        }
    }

    // -- format angka ------------------------------------------------------

    #[test]
    fn format_koin_switches_unit_at_each_threshold() {
        assert_eq!(format_koin(0), "0");
        assert_eq!(format_koin(999), "999");
        assert_eq!(format_koin(1_000), "1.00K");
        assert_eq!(format_koin(12_345), "12.35K");
        assert_eq!(format_koin(999_999), "1000.00K", "kurang dari 1M, belum naik satuan");
        assert_eq!(format_koin(1_000_000), "1.00M");
        assert_eq!(format_koin(1_500_000_000), "1.50B");
        assert_eq!(format_koin(2_000_000_000_000), "2.00T");
    }

    // -- cabang state tersimpan -------------------------------------------

    #[test]
    fn an_absent_user_row_reads_zero_and_free() {
        for user in [json!(null), json!({}), json!({"koin": null})] {
            let out = run(user.clone(), vec![], false);
            assert!(out.starts_with("*〔 💰 KOIN INFO 〕*\n\n*〔 👤 User 〕* Kamu\n*〔 💰 Koin 〕* 0\n*〔 💎 Status 〕* Free\n"), "{user}");
        }
    }

    #[test]
    fn a_stored_balance_is_formatted_and_shown() {
        let out = run(json!({"koin": 1250000}), vec![], false);
        assert!(out.contains("*〔 💰 Koin 〕* 1.25M\n"));
        assert!(out.contains("*〔 💎 Status 〕* Free\n"));
    }

    #[test]
    fn premium_and_owner_both_replace_free_in_the_status_line() {
        assert!(run(json!({"isPremium": true}), vec![], false).contains("Status 〕* Premium"));
        assert!(run(json!({"isPremium": true}), vec![], true).contains("Status 〕* Owner"));
        assert!(
            run(json!({"isPremium": false}), vec![], false).contains("Status 〕* Free"),
            "`false` harus tetap Free, bukan `undefined`"
        );
    }

    // -- cabang target -----------------------------------------------------

    #[test]
    fn the_shop_block_only_appears_for_the_sender() {
        let self_out = run(json!({"koin": 5}), vec![], false);
        assert!(self_out.contains("*〔 🛒 SHOP 〕*"));
        assert!(self_out.ends_with("_🎮 Main game untuk dapat koin!_"));

        let other_out = run(json!({"koin": 5}), vec!["628999@s.whatsapp.net"], false);
        assert!(!other_out.contains("🛒 SHOP"));
        assert!(other_out.contains("*〔 👤 User 〕* 628999\n"));
        assert!(other_out.ends_with("*〔 💎 Status 〕* Free\n"));
    }

    #[test]
    fn a_lid_target_is_numbered_without_the_server() {
        let out = run(json!({"koin": 1}), vec!["628999@lid"], false);
        assert!(out.contains("*〔 👤 User 〕* 628999\n"));
        assert!(!out.contains("SHOP"));
    }
}