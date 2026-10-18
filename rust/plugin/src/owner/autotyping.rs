//! `plugins/owner/autotyping.js` — flag `autoTyping`.
//!
//! `ponytail:` `saluranCtx()` (`autotyping.js:39`) ditempel sebagai
//! `contextInfo` forwarded-newsletter; `Reply` tidak punya field context, jadi
//! yang hilang hanya metadata itu. Teks balasannya utuh.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy_value;

/// `db.setting('autoTyping') ?? config.features?.autoTyping ?? true`.
///
/// Rantai `??` hanya jatuh ke ruas berikutnya saat nilainya `null`/`undefined`,
/// jadi nilai `false` yang tersimpan tetap terbaca `false`. Ruas terakhir
/// selalu `true`: `config.js:159` `features.autoTyping = true` dan `Ctx`
/// tidak membawa `config`, jadi konstantanya memang tidak bisa berubah.
fn current(ctx: &Ctx) -> bool {
    match ctx.setting("autoTyping") {
        None | Some(serde_json::Value::Null) => true,
        Some(v) => js_truthy_value(v),
    }
}

#[plugin(
    name = "autotyping",
    alias = ["typing", "autoketik"],
    category = "owner",
    description = "Auto typing indicator saat menerima pesan",
    usage = ".autotyping on/off",
    example = ".autotyping on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autotyping(ctx: Ctx) -> Reply {
    // `m.text?.toLowerCase()?.trim()` (`autotyping.js:23`) — lowercase dulu,
    // baru trim. Urutannya tidak bisa dibalik tanpa mengubah hasil untuk
    // input berisi huruf besar non-ASCII.
    let lowered = ctx.text.to_lowercase();
    let option = lowered.trim();

    if option.is_empty() {
        return Reply::text(format!(
            "⌨️ *Auto Typing*\n\n\
             > Status: *{status}*\n\n\
             *PENGGUNAAN:*\n\
             > *{p}autotyping on* — Aktifkan\n\
             > *{p}autotyping off* — Nonaktifkan\n\n\
             _Bot akan menampilkan indikator typing saat menerima pesan_",
            status = if current(&ctx) { "Aktif ✅" } else { "Nonaktif ❌" },
            p = ctx.prefix
        ));
    }

    match option {
        "on" => Reply::text(
            "⌨️ *Auto Typing Aktif*\n\n\
             > Bot akan menampilkan indikator typing",
        )
        .with_mutation(Mutation::SetSetting {
            key: "autoTyping".into(),
            value: serde_json::json!(true),
        }),
        "off" => Reply::text(
            "⌨️ *Auto Typing Nonaktif*\n\n\
             > Bot tidak akan menampilkan indikator typing",
        )
        .with_mutation(Mutation::SetSetting {
            key: "autoTyping".into(),
            value: serde_json::json!(false),
        }),
        _ => Reply::text(format!(
            "❌ *Opsi Tidak Valid*\n\n> Gunakan *{p}autotyping on* atau *{p}autotyping off*",
            p = ctx.prefix
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::js_truthy;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autotyping".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autotyping").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "autoTyping");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autotyping").unwrap();
        assert_eq!(p.meta.name, "autotyping");
        assert_eq!(p.meta.alias, ["typing", "autoketik"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Auto typing indicator saat menerima pesan"
        );
        assert_eq!(p.meta.usage, ".autotyping on/off");
        assert_eq!(p.meta.example, ".autotyping on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["typing", "autoketik"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autotyping");
        }
    }

    #[test]
    fn no_argument_reports_status_and_writes_nothing() {
        for text in ["", "   ", "\t\n"] {
            let r = run(text, serde_json::json!({}));
            assert!(
                text_of(&r).starts_with("⌨️ *Auto Typing*"),
                "input {text:?} -> {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_missing_setting_falls_through_to_the_config_default() {
        // `?? config.features?.autoTyping ?? true` dengan `autoTyping: true`
        // di `config.js:159` — key yang belum pernah ditulis tetap AKTIF.
        assert!(text_of(&run("", serde_json::json!({}))).contains("*Aktif ✅*"));
        assert!(
            text_of(&run("", serde_json::json!({ "autoTyping": null })))
                .contains("*Aktif ✅*"),
            "null juga kena `??`"
        );
    }

    #[test]
    fn a_stored_false_is_not_overwritten_by_the_default() {
        // `??` hanya menangkap null/undefined — `false` harus tetap Nonaktif.
        assert!(text_of(&run("", serde_json::json!({"autoTyping": false}))).contains("*Nonaktif ❌*"));
        assert!(text_of(&run("", serde_json::json!({"autoTyping": true}))).contains("*Aktif ✅*"));
    }

    #[test]
    fn the_status_block_keeps_the_full_help_text() {
        let r = run("", serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.contains("*PENGGUNAAN:*"), "{t}");
        assert!(t.contains("> *.autotyping on* — Aktifkan"), "{t}");
        assert!(t.contains("> *.autotyping off* — Nonaktifkan"), "{t}");
        assert!(
            t.ends_with("_Bot akan menampilkan indikator typing saat menerima pesan_"),
            "{t}"
        );
    }

    #[test]
    fn on_writes_true() {
        let r = run("on", serde_json::json!({}));
        assert_eq!(stored(&r), serde_json::json!(true));
        assert_eq!(
            text_of(&r),
            "⌨️ *Auto Typing Aktif*\n\n> Bot akan menampilkan indikator typing"
        );
    }

    #[test]
    fn off_writes_false() {
        let r = run("off", serde_json::json!({"autoTyping": true}));
        assert_eq!(stored(&r), serde_json::json!(false));
        assert_eq!(
            text_of(&r),
            "⌨️ *Auto Typing Nonaktif*\n\n> Bot tidak akan menampilkan indikator typing"
        );
    }

    #[test]
    fn an_unknown_option_is_refused_and_writes_nothing() {
        for bad in ["maybe", "1", "enable", "onn", "on off"] {
            let r = run(bad, serde_json::json!({}));
            assert!(
                text_of(&r).starts_with("❌ *Opsi Tidak Valid*"),
                "input {bad:?} -> {}",
                text_of(&r)
            );
            assert!(
                text_of(&r).ends_with("> Gunakan *.autotyping on* atau *.autotyping off*"),
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn the_option_is_lowercased_and_trimmed_before_matching() {
        // `m.text?.toLowerCase()?.trim()` — " ON " tetap kena cabang `on`.
        for text in ["ON", "  on  ", "On"] {
            assert_eq!(stored(&run(text, serde_json::json!({}))), serde_json::json!(true), "input {text:?}");
        }
        for text in ["OFF", " off ", "Off"] {
            assert_eq!(stored(&run(text, serde_json::json!({}))), serde_json::json!(false), "input {text:?}");
        }
    }

    #[test]
    fn switching_does_not_react() {
        // JS tidak memanggil `m.react` di kedua cabang; `saluranCtx()` pun
        // bukan reaksi. Reply harus polos.
        for text in ["on", "off"] {
            match run(text, serde_json::json!({})).without_mutations() {
                Reply::Text { .. } => {}
                other => panic!("input {text:?} harus Text, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn a_non_boolean_stored_value_keeps_the_js_truthiness() {
        // `${current ? ...}` memakai operator ternary, bukan perbandingan
        // `=== true` — string non-kosong tetap truthy seperti di JS.
        assert!(js_truthy(Some(&serde_json::json!("ya"))));
        assert!(text_of(&run("", serde_json::json!({"autoTyping": "ya"}))).contains("*Aktif ✅*"));
        assert!(text_of(&run("", serde_json::json!({"autoTyping": ""}))).contains("*Nonaktif ❌*"));
        assert!(text_of(&run("", serde_json::json!({"autoTyping": 0}))).contains("*Nonaktif ❌*"));
    }
}