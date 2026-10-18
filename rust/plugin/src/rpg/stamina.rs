//! `plugins/rpg/stamina.js` — cek stamina, dan isi ulang dengan 5.000 koin.
//!
//! `ponytail:` JS mengirim lewat `sendRpgPreview` (`sock.sendPreview`) yang
//! menempelkan thumbnail RPG + `config.info.website` ke caption dan menyetel
//! judul/deskripsi kartu. `Ctx` tidak punya `sock`, jadi teksnya sama persis
//! tapi terkirim sebagai balasan biasa tanpa kartu preview.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{coalesce, or_default, rpg_map};

/// `const potionCost = 5000` (`stamina.js:37`).
pub(crate) const POTION_COST: i64 = 5_000;

/// `createStaminaBar` (`stamina.js:19-23`) — 10 sel,
/// `Math.round(current / max * 10)` terisi.
///
/// `ponytail:` `Math.round(Infinity)` / `NaN` di JS membuat `repeat` melempar
/// `RangeError` (tidak ada `try/catch` di handler), sedangkan `repeat` Rust
/// panik. Hasilnya dijepit ke `0..=10` supaya data rusak tidak menjatuhkan
/// proses — `maxStamina` sendiri selalu terisi `|| 100` sebelum ini dipanggil.
pub(crate) fn stamina_bar(current: i64, max: i64) -> String {
    if max <= 0 {
        return "░".repeat(10);
    }
    let filled = ((current as f64 / max as f64) * 10.0)
        .round()
        .clamp(0.0, 10.0) as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
}

/// `user.rpg.stamina ?? 100` (`stamina.js:31`).
fn stamina_of(rpg: &Map<String, Value>) -> i64 {
    coalesce(rpg.get("stamina").and_then(Value::as_i64), 100)
}

/// `user.rpg.maxStamina || 100` (`stamina.js:32`) — nol ikut jadi 100.
fn max_stamina_of(rpg: &Map<String, Value>) -> i64 {
    or_default(rpg.get("maxStamina").and_then(Value::as_i64), 100)
}

#[plugin(
    name = "stamina",
    alias = ["cekstamina"],
    category = "rpg",
    description = "Cek dan pulihkan stamina",
    usage = ".stamina / .stamina isi",
    example = ".stamina",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stamina(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let current = stamina_of(&rpg);
    let max = max_stamina_of(&rpg);

    let sub_cmd = crate::common::first_keyword(&ctx);
    if matches!(sub_cmd.as_deref(), Some("isi") | Some("restore") | Some("heal")) {
        if current >= max {
            return Reply::text("⚡ *sᴛᴀᴍɪɴᴀ ᴘᴇɴᴜʜ*\n\n> Stamina kamu sudah penuh!");
        }

        let koin = or_default(ctx.user_field("koin").and_then(Value::as_i64), 0);
        if koin < POTION_COST {
            return Reply::text(format!(
                "❌ *sᴀʟᴅᴏ ᴛɪᴅᴀᴋ ᴄᴜᴋᴜᴘ*\n\n\
                 > Biaya: Rp {}\n\
                 > Saldo: Rp {}",
                crate::tools::kalkulatormbg::thousands_id(POTION_COST as f64),
                crate::tools::kalkulatormbg::thousands_id(koin as f64)
            ));
        }

        let restored = max - current;
        rpg.insert("stamina".into(), serde_json::json!(max));

        return Reply::text(format!(
            "⚡ *sᴛᴀᴍɪɴᴀ ᴅɪɪsɪ*\n\n\
             *💊 *ʀᴇsᴛᴏʀᴇ:*\n\n\
             > ⚡ Stamina: *+{restored}*\n\
             > 💵 Biaya: *-Rp {}*\n\
             > 📊 Sekarang: *{max}/{max}*\n",
            crate::tools::kalkulatormbg::thousands_id(POTION_COST as f64)
        ))
        .with_reaction("⚡")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin - POTION_COST, "rpg": rpg }),
        });
    }

    // Cabang status tidak memanggil `db.save()`, jadi tidak ada mutasi —
    // default `?? 100` / `|| 100` di atas cuma hidup selama handler berjalan.
    Reply::text(format!(
        "⚡ *sᴛᴀᴍɪɴᴀ sᴛᴀᴛᴜs*\n\n\
         *📊 *ɪɴꜰᴏ:*\n\n\
         > ⚡ Stamina: *{current}/{max}*\n\
         > 📊 [{}]\n\
         \n\n\
         > Isi stamina: `{}stamina isi` (Rp 5.000)\n\
         > Stamina pulih otomatis setiap jam",
        stamina_bar(current, max),
        ctx.prefix
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, arg: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "stamina".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.unwrap_or("").into(),
            args: arg.map(|a| vec![a.to_string()]).unwrap_or_default(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("stamina").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn data_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1, "harus ada satu SetUser");
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("stamina").unwrap();
        assert_eq!(p.meta.name, "stamina");
        assert_eq!(p.meta.alias, ["cekstamina"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Cek dan pulihkan stamina");
        assert_eq!(p.meta.usage, ".stamina / .stamina isi");
        assert_eq!(p.meta.example, ".stamina");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("cekstamina").unwrap().meta.name, "stamina");
    }

    #[test]
    fn the_status_card_shows_the_bar_and_the_restore_hint() {
        let r = run(serde_json::json!({"koin": 0, "rpg": {"stamina": 50, "maxStamina": 100}}), None);
        let t = text_of(&r);
        assert!(t.starts_with("⚡ *sᴛᴀᴍɪɴᴀ sᴛᴀᴛᴜs*\n\n"), "{t}");
        assert!(t.contains("> ⚡ Stamina: *50/100*\n"), "{t}");
        assert!(t.contains("> 📊 [█████░░░░░]\n"), "{t}");
        assert!(t.contains("> Isi stamina: `.stamina isi` (Rp 5.000)\n"), "{t}");
        assert!(t.ends_with("> Stamina pulih otomatis setiap jam"), "{t}");
        assert!(r.mutations().is_empty(), "cabang status tidak menyimpan apa pun");
        assert_eq!(reaction_of(&r), None, "cabang status tidak bereaksi");
    }

    #[test]
    fn missing_rpg_defaults_to_a_full_bar() {
        let r = run(serde_json::json!(null), None);
        assert!(text_of(&r).contains("> ⚡ Stamina: *100/100*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("[██████████]"), "{}", text_of(&r));
    }

    /// `user.rpg.stamina ?? 100` bukan `|| 100`: 0 tetap 0.
    #[test]
    fn zero_stamina_stays_zero_while_zero_max_falls_back_to_one_hundred() {
        let r = run(serde_json::json!({"rpg": {"stamina": 0, "maxStamina": 0}}), None);
        let t = text_of(&r);
        assert!(t.contains("> ⚡ Stamina: *0/100*"), "{t}");
        assert!(t.contains("[░░░░░░░░░░]"), "{t}");
    }

    #[test]
    fn restoring_costs_five_thousand_and_refills_the_bar() {
        let r = run(serde_json::json!({"koin": 20_000, "rpg": {"stamina": 25, "maxStamina": 100}}), Some("isi"));
        let t = text_of(&r);
        assert!(t.starts_with("⚡ *sᴛᴀᴍɪɴᴀ ᴅɪɪsɪ*\n\n"), "{t}");
        assert!(t.contains("*💊 *ʀᴇsᴛᴏʀᴇ:*\n\n"), "{t}");
        assert!(t.contains("> ⚡ Stamina: *+75*"), "{t}");
        assert!(t.contains("> 💵 Biaya: *-Rp 5.000*"), "{t}");
        assert!(t.contains("> 📊 Sekarang: *100/100*"), "{t}");
        assert_eq!(reaction_of(&r), Some("⚡"));

        let data = data_of(&r);
        assert_eq!(data["koin"], serde_json::json!(15_000));
        assert_eq!(data["rpg"]["stamina"], serde_json::json!(100));
        assert_eq!(data["rpg"]["maxStamina"], serde_json::json!(100));
    }

    #[test]
    fn restore_and_heal_are_read_case_insensitively() {
        for arg in ["isi", "restore", "heal", "ISI", "Restore", "HEAL"] {
            let r = run(serde_json::json!({"koin": 9_000, "rpg": {"stamina": 10}}), Some(arg));
            assert!(text_of(&r).starts_with("⚡ *sᴛᴀᴍɪɴᴀ ᴅɪɪsɪ*"), "{arg}");
        }
        // Sub-command lain tetap jatuh ke kartu status.
        for arg in ["", "penuh", "isi2"] {
            let r = run(serde_json::json!({"koin": 9_000, "rpg": {"stamina": 10}}), Some(arg));
            assert!(text_of(&r).starts_with("⚡ *sᴛᴀᴍɪɴᴀ sᴛᴀᴛᴜs*"), "{arg}");
        }
    }

    #[test]
    fn a_full_bar_is_refused_before_the_purse_is_read() {
        let r = run(serde_json::json!({"koin": 0, "rpg": {"stamina": 100, "maxStamina": 100}}), Some("isi"));
        assert_eq!(
            text_of(&r),
            "⚡ *sᴛᴀᴍɪɴᴀ ᴘᴇɴᴜʜ*\n\n> Stamina kamu sudah penuh!"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn too_little_coin_reports_the_cost_and_the_balance() {
        let r = run(serde_json::json!({"koin": 4_999, "rpg": {"stamina": 10}}), Some("isi"));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *sᴀʟᴅᴏ ᴛɪᴅᴀᴋ ᴄᴜᴋᴜᴘ*\n\n"), "{t}");
        assert!(t.contains("> Biaya: Rp 5.000"), "{t}");
        assert!(t.contains("> Saldo: Rp 4.999"), "{t}");
        assert!(r.mutations().is_empty());
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn the_bar_is_ten_cells_for_every_input() {
        assert_eq!(stamina_bar(0, 100), "░".repeat(10));
        assert_eq!(stamina_bar(100, 100), "█".repeat(10));
        assert_eq!(stamina_bar(50, 100), "█████░░░░░");
        // 1/8 = 1.25 sel -> dibulatkan jadi 1.
        assert_eq!(stamina_bar(1, 8), "█░░░░░░░░░");
        for current in [-100, 0, 1, 37, 100, 250, i64::MAX] {
            for max in [-5, 1, 100, i64::MAX] {
                let bar = stamina_bar(current, max);
                assert_eq!(
                    bar.chars().count(),
                    10,
                    "current {current} max {max} -> {bar}"
                );
            }
        }
        // Nilai rusak tidak boleh membuat panik.
        assert_eq!(stamina_bar(5, 0), "░".repeat(10));
        assert_eq!(stamina_bar(5, -3), "░".repeat(10));
    }

    #[test]
    fn unrelated_rpg_fields_survive_the_restore() {
        let r = run(
            serde_json::json!({"koin": 20_000, "rpg": {"stamina": 10, "bank": 500, "pet": {"type": "dog"}}}),
            Some("restore"),
        );
        let data = data_of(&r);
        assert_eq!(data["rpg"]["bank"], serde_json::json!(500));
        assert_eq!(data["rpg"]["pet"], serde_json::json!({"type": "dog"}));
    }

    #[test]
    fn a_non_numeric_field_falls_back_instead_of_panicking() {
        for rpg in [serde_json::json!({"stamina": "abc", "maxStamina": "abc"}), serde_json::json!("bukan objek")] {
            let r = run(serde_json::json!({"rpg": rpg}), None);
            assert!(
                text_of(&r).starts_with("⚡ *sᴛᴀᴍɪɴᴀ sᴛᴀᴛᴜs*"),
                "{}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty());
        }
    }
}
