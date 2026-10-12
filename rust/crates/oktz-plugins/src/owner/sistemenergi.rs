//! `plugins/owner/sistemenergi.js` — flag global `energi`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{first_keyword, js_truthy};

/// `db.setting("energi") !== undefined ? db.setting("energi") : true` —
/// default-nya `true`, bukan `false`. Bedanya penting: energi yang belum pernah
/// di-set berarti bot masih memotong energi, persis default pipeline.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    match ctx.setting("energi") {
        Some(v) => js_truthy(Some(v)),
        None => true,
    }
}

pub(crate) fn status_text(ctx: &Ctx, current: bool) -> String {
    format!(
        "🎛️ *KONTROL SISTEM ENERGI*\n\n\
         Fitur kendali utama untuk menghidupkan atau mematikan seluruh sistem potongan energi bot secara global.\n\n\
         *STATUS SAAT INI:*\n\
         - Mode: *{}*\n\n\
         *PENGGUNAAN:*\n\
         - *{p}sistemenergi on* — Menghidupkan potongan energi\n\
         - *{p}sistemenergi off* — Mematikan potongan energi (Unlimited mode)\n\n\
         *CONTOH PENGGUNAAN:*\n\
         - *{p}sistemenergi on*",
        if current { "AKTIF 🔋" } else { "NONAKTIF (UNLIMITED) ♾️" },
        p = ctx.prefix
    )
}

pub(crate) fn applied(is_enabled: bool) -> String {
    format!(
        "✅ *STATUS SISTEM ENERGI BERHASIL DIUBAH*\n\n\
         Sistem Energi kini telah berhasil di-*{verb}*.\n\n\
         *Status Terkini:*\n\
         - Mode: *{mode}*\n\n\
         _Semua perubahan sudah tersimpan dan akan segera diterapkan pada sistem bot._",
        verb = if is_enabled { "HIDUPKAN" } else { "MATIKAN" },
        mode = if is_enabled { "AKTIF 🔋" } else { "NONAKTIF (UNLIMITED) ♾️" }
    )
}

#[plugin(
    name = "sistemenergi",
    alias = ["sistemlimit", "energimode", "limitmode"],
    category = "owner",
    description = "Mengecek, menghidupkan, atau mematikan sistem energi secara global",
    usage = ".sistemenergi [on / off]",
    example = ".sistemenergi on",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn sistemenergi(ctx: Ctx) -> Reply {
    let current = current_enabled(&ctx);
    // `!mode || (mode !== 'on' && mode !== 'off')` — satu cabang untuk
    // ketiadaan argumen DAN untuk nilai yang bukan on/off.
    let is_enabled = match first_keyword(&ctx).as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => return Reply::text(status_text(&ctx, current)),
    };

    Reply::text(applied(is_enabled))
        .with_reaction("🕕")
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "energi".into(),
            value: serde_json::Value::Bool(is_enabled),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value, text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "sistemenergi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("sistemenergi").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Option<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "energi");
                Some(value.clone())
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("sistemenergi").unwrap();
        assert_eq!(p.meta.name, "sistemenergi");
        assert_eq!(p.meta.alias, ["sistemlimit", "energimode", "limitmode"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengecek, menghidupkan, atau mematikan sistem energi secara global"
        );
        assert_eq!(p.meta.usage, ".sistemenergi [on / off]");
        assert_eq!(p.meta.example, ".sistemenergi on");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["sistemlimit", "energimode", "limitmode"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "sistemenergi");
        }
    }

    #[test]
    fn the_default_when_the_setting_is_absent_is_enabled_not_disabled() {
        // `db.setting("energi") !== undefined ? ... : true` — sudah tidak
        // pernah di-set berarti AKTIF. Default `false` di sini akan
        // mengarang "unlimited" untuk instalasi yang tidak pernah menyetel.
        assert!(current_enabled(&Ctx::default()));
        assert!(text_of(&run(json!({}), "")).contains("AKTIF 🔋"));
    }

    #[test]
    fn status_reflects_a_stored_false_without_reading_the_pipeline_default() {
        for (settings, expected) in [
            (json!({"energi": false}), "NONAKTIF (UNLIMITED) ♾️"),
            (json!({"energi": true}), "AKTIF 🔋"),
        ] {
            let r = run(settings.clone(), "");
            assert!(text_of(&r).contains(expected), "input {settings}");
            assert!(r.mutations().is_empty(), "status tidak menulis");
        }
    }

    #[test]
    fn no_argument_shows_status_and_writes_nothing() {
        let r = run(json!({"energi": true}), "");
        assert!(text_of(&r).starts_with("🎛️ *KONTROL SISTEM ENERGI*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unrecognised_mode_shows_status_instead_of_writing() {
        // `m.args[0]?.toLowerCase()` sudah ada di JS, jadi huruf besar TIDAK
        // termasuk di sini — `ON` sah. Yang tidak sah adalah kata lain.
        for bad in ["aktif", "1", "yes", "enable", "onn"] {
            let r = run(json!({}), bad);
            assert!(
                text_of(&r).starts_with("🎛️ *KONTROL SISTEM ENERGI*"),
                "input {bad} harus jatuh ke cabang status"
            );
            assert!(r.mutations().is_empty(), "input {bad} tidak boleh menulis");
        }
    }

    #[test]
    fn the_mode_argument_is_lowercased_before_comparison() {
        for arg in ["ON", "On", "oN"] {
            assert_eq!(
                stored(&run(json!({}), arg)),
                Some(json!(true)),
                "arg {arg} harus dikenali sebagai on"
            );
        }
        for arg in ["OFF", "Off"] {
            assert_eq!(stored(&run(json!({}), arg)), Some(json!(false)), "arg {arg}");
        }
    }

    #[test]
    fn on_and_off_write_their_own_boolean() {
        assert_eq!(stored(&run(json!({}), "on")), Some(json!(true)));
        assert_eq!(stored(&run(json!({}), "off")), Some(json!(false)));
    }

    #[test]
    fn the_applied_reply_names_the_direction_actually_written() {
        assert!(text_of(&run(json!({}), "on")).contains("di-*HIDUPKAN*"));
        assert!(text_of(&run(json!({}), "off")).contains("di-*MATIKAN*"));
        assert!(text_of(&run(json!({}), "off")).contains("NONAKTIF (UNLIMITED) ♾️"));
    }

    #[test]
    fn the_applied_reply_is_paired_with_the_write_it_claims() {
        for arg in ["on", "off"] {
            assert!(
                !run(json!({}), arg).mutations().is_empty(),
                "arg {arg}: reply 'berhasil diubah' tanpa mutasi = klaim palsu"
            );
        }
    }

    #[test]
    fn both_reactions_are_emitted_in_the_js_order() {
        // `m.react('🕕')` lalu `m.react('✅')` — `with_reaction` dirangkai
        // dua kali, jadi `Reaction` terluar adalah yang kedua.
        match run(json!({}), "on").without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}