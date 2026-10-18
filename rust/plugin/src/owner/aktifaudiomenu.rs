//! `plugins/owner/aktifaudiomenu.js` — flag `audioMenu`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

/// `db.setting('audioMenu') !== false` — default `true` (bukan `false`):
/// audio menu aktif sampai ada yang mematikannya secara eksplisit.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    match ctx.setting("audioMenu") {
        Some(serde_json::Value::Bool(b)) => *b,
        // `!== false` — semua bentuk lain (termasuk undefined) lolos.
        _ => true,
    }
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "🔊 *ᴀᴜᴅɪᴏ ᴍᴇɴᴜ sᴇᴛᴛɪɴɢ*\n\n\
         > Status: *{}*\n\n\
         *Cara pakai:*\n\
         > `{}aktifaudiomenu ya` - Aktifkan audio\n\
         > `{}aktifaudiomenu gak` - Nonaktifkan audio",
        if current_enabled(ctx) { "✅ Aktif" } else { "❌ Nonaktif" },
        ctx.prefix,
        ctx.prefix
    )
}

#[plugin(
    name = "aktifaudiomenu",
    alias = ["audiomenu", "setaudiomenu", "toggleaudiomenu"],
    category = "owner",
    description = "Toggle audio saat menampilkan menu",
    usage = ".aktifaudiomenu ya/gak",
    example = ".aktifaudiomenu ya",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn aktifaudiomenu(ctx: Ctx) -> Reply {
    let current = current_enabled(&ctx);
    // Opsi yang diterima: `ya`/`on`/`1`/`aktif` dan `gak`/`off`/`0`/`nonaktif`.
    // Semuanya lowercase karena argumennya sudah diproses router.
    let wants_on = matches!(
        first_keyword(&ctx).as_deref(),
        Some("ya") | Some("on") | Some("1") | Some("aktif")
    );
    let wants_off = matches!(
        first_keyword(&ctx).as_deref(),
        Some("gak") | Some("off") | Some("0") | Some("nonaktif")
    );

    if !wants_on && !wants_off {
        // Tanpa argumen DAN opsi tak dikenal memakai teks yang sama.
        return Reply::text(status_text(&ctx));
    }

    // `if (current) return ...sudah aktif` — men-toggle ke nilai yang sama
    // ditolak, bukan diam-diam menulis ulang nilai yang memang sudah ada.
    if wants_on == current {
        return Reply::text(if current {
            "⚠️ Audio menu sudah aktif!"
        } else {
            "⚠️ Audio menu sudah nonaktif!"
        });
    }

    let enable = wants_on;
    Reply::text(if enable {
        "✅ Audio menu *diaktifkan*!\n\n> Sekarang ketika ada yang ketik `.menu`, audio akan muncul."
    } else {
        "❌ Audio menu *dinonaktifkan*!\n\n> Sekarang `.menu` tidak akan ada audio."
    })
    .with_reaction("✅")
    .with_mutation(Mutation::SetSetting {
        key: "audioMenu".into(),
        value: serde_json::Value::Bool(enable),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "aktifaudiomenu".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("aktifaudiomenu").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "audioMenu");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("aktifaudiomenu").unwrap();
        assert_eq!(p.meta.name, "aktifaudiomenu");
        assert_eq!(
            p.meta.alias,
            ["audiomenu", "setaudiomenu", "toggleaudiomenu"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Toggle audio saat menampilkan menu");
        assert_eq!(p.meta.usage, ".aktifaudiomenu ya/gak");
        assert_eq!(p.meta.example, ".aktifaudiomenu ya");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["audiomenu", "setaudiomenu", "toggleaudiomenu"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "aktifaudiomenu"
            );
        }
    }

    #[test]
    fn the_default_when_the_setting_is_absent_is_on() {
        // `db.setting('audioMenu') !== false` — undefined !== false, jadi
        // aktif. Default `false` akan mematikan audio untuk instalasi baru.
        assert!(current_enabled(&Ctx::default()));
        assert!(text_of(&run(serde_json::json!({}), "")).contains("✅ Aktif"));
    }

    #[test]
    fn only_an_explicit_boolean_false_turns_it_off() {
        assert!(current_enabled(&Ctx {
            settings: serde_json::json!({"audioMenu": true}),
            ..Default::default()
        }));
        assert!(!current_enabled(&Ctx {
            settings: serde_json::json!({"audioMenu": false}),
            ..Default::default()
        }));
        // `!== false` berarti bentuk lain tetap "aktif".
        for weird in [serde_json::json!(0), serde_json::json!(""), serde_json::json!(null)] {
            assert!(
                current_enabled(&Ctx {
                    settings: serde_json::json!({ "audioMenu": weird }),
                    ..Default::default()
                }),
                "nilai non-false tidak mematikan audio"
            );
        }
    }

    #[test]
    fn every_spelling_of_yes_activates_and_of_no_deactivates() {
        for on in ["ya", "on", "1", "aktif"] {
            assert_eq!(stored(&run(serde_json::json!({"audioMenu": false}), on)), serde_json::json!(true), "arg {on}");
        }
        for off in ["gak", "off", "0", "nonaktif"] {
            assert_eq!(stored(&run(serde_json::json!({"audioMenu": true}), off)), serde_json::json!(false), "arg {off}");
        }
    }

    #[test]
    fn re_selecting_the_current_state_is_refused_and_writes_nothing() {
        let already_on = run(serde_json::json!({"audioMenu": true}), "ya");
        assert_eq!(text_of(&already_on), "⚠️ Audio menu sudah aktif!");
        assert!(already_on.mutations().is_empty());

        let already_off = run(serde_json::json!({"audioMenu": false}), "gak");
        assert_eq!(text_of(&already_off), "⚠️ Audio menu sudah nonaktif!");
        assert!(already_off.mutations().is_empty());
    }

    #[test]
    fn a_fresh_store_accepts_the_off_switch_directly() {
        // Default `true`, jadi `gak` adalah perubahan nyata.
        let r = run(serde_json::json!({}), "gak");
        assert_eq!(stored(&r), serde_json::json!(false));
        assert!(text_of(&r).contains("❌ Audio menu *dinonaktifkan*!"));
    }

    #[test]
    fn an_unknown_option_shows_status_and_writes_nothing() {
        for bad in ["", "mungkin", "2", "ya2", "benar"] {
            let r = run(serde_json::json!({}), bad);
            assert!(
                text_of(&r).starts_with("🔊 *ᴀᴜᴅɪᴏ ᴍᴇɴᴜ sᴇᴛᴛɪɴɢ*"),
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn the_change_reply_reacts_and_is_paired_with_the_write() {
        let r = run(serde_json::json!({"audioMenu": false}), "ya");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(text_of(&r).contains("diaktifkan"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_status_reply_uses_the_live_prefix() {
        let t = status_text(&Ctx {
            prefix: "~".into(),
            ..Default::default()
        });
        assert!(t.contains("~aktifaudiomenu ya"), "{t}");
        assert!(t.contains("~aktifaudiomenu gak"));
    }
}