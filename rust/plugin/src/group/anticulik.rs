//! `plugins/group/anticulik.js` — sakelar "bot keluar kalau ditambah tanpa izin".
//!
//! Yang dipindah: seluruh `handler` (`on`/`off`/status/opsi salah).
//!
//! Yang tidak dipindah:
//! - `saluranCtx()` (`context.js`) pada cabang `on` (`:51`). `Reply` tidak punya
//!   jalur context sama sekali, jadi balasan itu kehilangan badge newsletter.
//!   Batasan yang sudah didokumentasikan di `BLOCKED.md` §15, bukan penyimpangan
//!   port.
//! - `handleAntiCulik` (`:71`) bukan handler plugin: ia dipanggil dari event
//!   `groups.update` dan butuh `event.participants`/`event.author`,
//!   `sock.groupLeave`, dan `sock.user.id`/`.lid`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `group.anticulik ?? db.setting("anticulik") ?? "off"`, lalu `=== "on"`.
/// `??` melompati `null`/`undefined` saja, jadi nilai falsy lain (`false`, `""`)
/// TIDAK membuat status jatuh ke setting — persisoperator `??` di JS.
fn mode_on(ctx: &Ctx) -> bool {
    let value = ctx
        .group_field("anticulik")
        .filter(|v| !v.is_null())
        .or_else(|| ctx.setting("anticulik").filter(|v| !v.is_null()));
    value.and_then(|v| v.as_str()) == Some("on")
}

fn status_text(prefix: &str, on: bool) -> String {
    format!(
        "🛡️ *Anti Culik*\n\n\
         Bot akan otomatis keluar dari grup jika ditambah oleh orang yang tidak dikenal tanpa izin.\n\n\
         *STATUS:*\n\
         > Mode: *{mode}*\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}anticulik on* — Aktifkan\n\
         > *{prefix}anticulik off* — Nonaktifkan\n\n\
         _Jika aktif, bot hanya bisa join via *{prefix}join* atau ditambah oleh owner_",
        mode = if on { "Aktif ✅" } else { "Nonaktif ❌" },
    )
}

fn active_text(prefix: &str) -> String {
    format!(
        "🛡️ *Anti Culik Aktif*\n\n\
         > Bot akan keluar otomatis jika ditambah tanpa izin\n\
         > Satu-satunya cara bot bisa join: *{prefix}join* oleh owner\n\n\
         _Member yang menambah bot akan diberi peringatan_"
    )
}

fn inactive_text() -> String {
    "🛡️ *Anti Culik Nonaktif*\n\n\
     > Bot tidak akan keluar otomatis jika ditambah ke grup\n\
     > Siapapun bisa menambahkan bot ke grup"
        .to_string()
}

fn invalid_text(prefix: &str) -> String {
    format!(
        "❌ *Opsi Tidak Valid*\n\n> Gunakan *{prefix}anticulik on* atau *{prefix}anticulik off*"
    )
}

fn write_status(ctx: &Ctx, value: &str, text: String) -> Reply {
    Reply::text(text).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "anticulik": value }),
    })
}

#[plugin(
    name = "anticulik",
    alias = ["antikidnap", "antiileng", "anticulikgc"],
    category = "group",
    description = "Bot otomatis keluar grup jika ditambah tanpa izin",
    usage = ".anticulik on/off",
    example = ".anticulik on",
    cooldown = 5,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = true,
    is_private = false
)]
pub fn anticulik(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        return Reply::text(status_text(&ctx.prefix, mode_on(&ctx)));
    }

    if option == "on" {
        return write_status(&ctx, "on", active_text(&ctx.prefix));
    }

    if option == "off" {
        return write_status(&ctx, "off", inactive_text());
    }

    Reply::text(invalid_text(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use serde_json::{Value};

    fn run(text: &str, group: Value, setting: Value) -> Reply {
        (find("anticulik").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "anticulik".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            settings: serde_json::json!({ "anticulik": setting }),
            ..Default::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn written(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data.clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("anticulik").unwrap();
        assert_eq!(p.meta.name, "anticulik");
        assert_eq!(p.meta.alias, ["antikidnap", "antiileng", "anticulikgc"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Bot otomatis keluar grup jika ditambah tanpa izin"
        );
        assert_eq!(p.meta.usage, ".anticulik on/off");
        assert_eq!(p.meta.example, ".anticulik on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner, "isOwner:true di JS");
        assert!(!p.meta.is_premium);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin, "tidak ada di JS");
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["antikidnap", "antiileng", "anticulikgc"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("anticulik"), "{a}");
        }
    }

    #[test]
    fn without_option_it_only_reports_and_writes_nothing() {
        let r = run("", serde_json::json!({}), Value::Null);
        assert!(text_of(&r).contains("> Mode: *Nonaktif ❌*"));
        assert!(r.mutations().is_empty(), "status tidak boleh menulis");
    }

    #[test]
    fn status_prefers_group_then_global_setting() {
        let r = run("", serde_json::json!({ "anticulik": "on" }), serde_json::json!("on"));
        assert!(text_of(&r).contains("> Mode: *Aktif ✅*"));

        // Group belum punya key, setting global yang menentukan.
        let r = run("", serde_json::json!({}), serde_json::json!("on"));
        assert!(text_of(&r).contains("> Mode: *Aktif ✅*"));

        // Group wins walau global berbeda.
        let r = run("", serde_json::json!({ "anticulik": "off" }), serde_json::json!("on"));
        assert!(text_of(&r).contains("> Mode: *Nonaktif ❌*"));
    }

    #[test]
    fn a_falsy_group_value_does_not_fall_through_to_the_setting() {
        // `??` hanya melompati null/undefined; `false` tetap dipakai, jadi
        // status nonaktif walau setting global `on`.
        let r = run("", serde_json::json!({ "anticulik": false }), serde_json::json!("on"));
        assert!(text_of(&r).contains("> Mode: *Nonaktif ❌*"), "{}", text_of(&r));
    }

    #[test]
    fn missing_group_and_missing_setting_both_report_nonaktif() {
        let r = run("", Value::Null, Value::Null);
        assert!(text_of(&r).contains("> Mode: *Nonaktif ❌*"));
    }

    #[test]
    fn on_and_off_write_the_group_field() {
        let on = run("on", serde_json::json!({}), Value::Null);
        assert_eq!(written(&on), serde_json::json!({ "anticulik": "on" }));
        assert!(text_of(&on).starts_with("🛡️ *Anti Culik Aktif*"));

        let off = run("off", serde_json::json!({ "anticulik": "on" }), Value::Null);
        assert_eq!(written(&off), serde_json::json!({ "anticulik": "off" }));
        assert!(text_of(&off).starts_with("🛡️ *Anti Culik Nonaktif*"));
    }

    #[test]
    fn the_option_is_lowercased_and_trimmed() {
        assert_eq!(written(&run("  ON  ", serde_json::json!({}), Value::Null)), serde_json::json!({ "anticulik": "on" }));
        assert_eq!(written(&run("Off", serde_json::json!({}), Value::Null)), serde_json::json!({ "anticulik": "off" }));
    }

    #[test]
    fn an_unknown_option_is_refused_without_writing() {
        let r = run("mungkin", serde_json::json!({}), Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ *Opsi Tidak Valid*\n\n> Gunakan *.anticulik on* atau *.anticulik off*"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_status_text_keeps_the_prefix_inside_the_markdown() {
        let r = (find("anticulik").unwrap().handler)(Ctx {
            prefix: "!".into(),
            command: "anticulik".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        });
        let t = text_of(&r);
        assert!(t.contains("> *!anticulik on* — Aktifkan"), "{t}");
        assert!(t.ends_with("via *!join* atau ditambah oleh owner_"), "{t}");
    }
}