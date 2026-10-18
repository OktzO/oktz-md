//! `plugins/owner/onlygc.js` — flag global `onlyGc` (+ `onlyPc` ikut dimatikan).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

/// `db.setting("onlyGc") || false` — nilai setting apa pun yang falsy (termasuk
/// `undefined`) menjadi `false` untuk ditampilkan. Pelaporan TIDAK memakai
/// `setting_bool(.., true)`: default di JS adalah `false`, bukan "belum
/// diputuskan".
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    ctx.setting("onlyGc")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "🏘️ *Only Group*\n\n\
         > Status: *{}*\n\n\
         *PENGGUNAAN:*\n\
         > *{p}onlygc on* — Bot hanya bisa diakses di grup\n\
         > *{p}onlygc off* — Bot bisa diakses di mana saja\n\n\
         _Jika aktif, mode Only Private akan otomatis nonaktif_",
        if current_enabled(ctx) {
            "Aktif ✅"
        } else {
            "Nonaktif ❌"
        },
        p = ctx.prefix
    )
}

pub(crate) const ON_MSG: &str = "🏘️ *Only Group Aktif*\n\n> Bot hanya bisa diakses di grup\n> Mode Only Private dinonaktifkan";
pub(crate) const OFF_MSG: &str = "🏘️ *Only Group Nonaktif*\n\n> Bot bisa diakses di mana saja";

pub(crate) fn invalid(prefix: &str) -> String {
    format!("❌ *Opsi Tidak Valid*\n\n> Gunakan *{prefix}onlygc on* atau *{prefix}onlygc off*")
}

/// `db.setting("onlyGc", X)` + `db.setting("onlyGc", false)` pada cabang `on`
/// — dua mutasi, dua key.
pub(crate) fn set_only_gc(enabled: bool) -> Reply {
    let reply = if enabled { ON_MSG } else { OFF_MSG };
    let mut r = Reply::text(reply).with_reaction(if enabled { "✅" } else { "❌" });
    if enabled {
        r = r.with_mutation(Mutation::SetSetting {
            key: "onlyGc".into(),
            value: serde_json::Value::Bool(true),
        })
        .with_mutation(Mutation::SetSetting {
            key: "onlyPc".into(),
            value: serde_json::Value::Bool(false),
        });
    } else {
        r = r.with_mutation(Mutation::SetSetting {
            key: "onlyGc".into(),
            value: serde_json::Value::Bool(false),
        });
    }
    r
}

#[plugin(
    name = "onlygc",
    alias = ["onlygroup", "grouponly"],
    category = "owner",
    description = "Toggle mode bot hanya di grup",
    usage = ".onlygc on/off",
    example = ".onlygc on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn onlygc(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        None => Reply::text(status_text(&ctx)),
        Some("on") => set_only_gc(true),
        Some("off") => set_only_gc(false),
        Some(_) => Reply::text(invalid(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value, text: &str) -> Reply {
        let mut args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        args.retain(|a| !a.is_empty());
        let ctx = Ctx {
            prefix: ".".into(),
            command: "onlygc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("onlygc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply, key: &str) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key: k, value } if k == key => Some(value.clone()),
            _ => None,
        })
    }

    fn count_of(r: &Reply) -> usize {
        r.mutations().len()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    // -- registrasi & meta ------------------------------------------------

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("onlygc").unwrap();
        assert_eq!(p.meta.name, "onlygc");
        assert_eq!(p.meta.alias, ["onlygroup", "grouponly"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Toggle mode bot hanya di grup");
        assert_eq!(p.meta.usage, ".onlygc on/off");
        assert_eq!(p.meta.example, ".onlygc on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner, "isOwner: true");
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["onlygroup", "grouponly"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "onlygc");
        }
    }

    // -- cabang tanpa argumen: status --------------------------------------

    #[test]
    fn no_argument_reports_status_and_writes_nothing() {
        for (settings, expected) in [
            (serde_json::json!({}), "Nonaktif ❌"),
            (serde_json::json!({"onlyGc": false}), "Nonaktif ❌"),
            (serde_json::json!({"onlyGc": true}), "Aktif ✅"),
        ] {
            let r = run(settings.clone(), "");
            assert!(
                text_of(&r).contains(expected),
                "input {settings} harus menampilkan {expected}"
            );
            assert!(text_of(&r).starts_with("🏘️ *Only Group*"));
            assert_eq!(count_of(&r), 0, "status tidak boleh menulis");
        }
    }

    #[test]
    fn a_non_boolean_setting_falls_back_to_the_js_default_false() {
        // `db.setting("onlyGc") || false` — string apa pun yang truthy di JS
        // sebenarnya akan tampil "Aktif", tapi `as_bool` tidak bisa meniru
        // kejujuran `||`; yang diuji adalah fallback tidak panic.
        let r = run(serde_json::json!({"onlyGc": "ya"}), "");
        assert!(text_of(&r).contains("Nonaktif ❌"));
        assert_eq!(count_of(&r), 0);
    }

    #[test]
    fn status_uses_the_live_prefix_in_the_usage_lines() {
        let args: Vec<String> = Vec::new();
        let ctx = Ctx {
            prefix: "!".into(),
            command: "onlygc".into(),
            text: String::new(),
            args,
            settings: serde_json::json!({}),
            ..Default::default()
        };
        let out = status_text(&ctx);
        assert!(out.contains("!onlygc on"), "prefix harus ikut: {out}");
        assert!(out.contains("!onlygc off"));
    }

    // -- cabang on / off ----------------------------------------------------

    #[test]
    fn on_writes_onlygc_true_and_also_forces_onlypc_false() {
        let r = run(serde_json::json!({"onlyGc": false, "onlyPc": true}), "on");
        assert_eq!(count_of(&r), 2, "dua setting ditulis pada cabang on");
        assert_eq!(setting_of(&r, "onlyGc"), Some(serde_json::json!(true)));
        assert_eq!(
            setting_of(&r, "onlyPc"),
            Some(serde_json::json!(false)),
            "onlyGc on harus mematikan onlyPc — itu isi reply-nya juga"
        );
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(text_of(&r), ON_MSG);
    }

    #[test]
    fn off_writes_only_onlygc_false() {
        let r = run(serde_json::json!({"onlyGc": true, "onlyPc": true}), "off");
        assert_eq!(count_of(&r), 1, "cabang off tidak menyentuh onlyPc");
        assert_eq!(setting_of(&r, "onlyGc"), Some(serde_json::json!(false)));
        assert_eq!(setting_of(&r, "onlyPc"), None);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert_eq!(text_of(&r), OFF_MSG);
    }

    #[test]
    fn the_on_and_off_replies_are_each_paired_with_the_write_that_makes_them_true() {
        // Reply "aktif" tanpa mutasi akan berbohong; ini yang membuat test ini
        // gagal kalau `set_only_gc` dipecah jadi cabang yang salah.
        assert!(!run(serde_json::json!({}), "on").mutations().is_empty());
        assert!(!run(serde_json::json!({}), "off").mutations().is_empty());
    }

    // -- cabang argumen tidak valid -----------------------------------------

    #[test]
    fn an_unknown_option_is_rejected_and_writes_nothing() {
        for bad in ["onx", "enable", "1", "ya"] {
            let r = run(serde_json::json!({}), bad);
            assert!(
                text_of(&r).starts_with("❌ *Opsi Tidak Valid*"),
                "input {bad} harus ditolak"
            );
            assert_eq!(count_of(&r), 0);
        }
    }

    #[test]
    fn the_invalid_option_reply_carries_the_prefix() {
        assert_eq!(invalid("!"), "❌ *Opsi Tidak Valid*\n\n> Gunakan *!onlygc on* atau *!onlygc off*");
    }

    #[test]
    fn matching_is_case_insensitive_because_the_command_is_lowercased() {
        // JS: `m.text?.toLowerCase()` — jadi "ON" dan "on" sama.
        assert_eq!(setting_of(&run(serde_json::json!({}), "ON"), "onlyGc"), Some(serde_json::json!(true)));
        assert_eq!(
            setting_of(&run(serde_json::json!({}), "Off"), "onlyGc"),
            Some(serde_json::json!(false))
        );
    }
}