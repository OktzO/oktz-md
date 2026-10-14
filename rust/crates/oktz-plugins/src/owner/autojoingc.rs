//! `plugins/owner/autojoingc.js` — toggle `setting('autoJoinGc')`.
//!
//! `ponytail:` hanya `handler` yang di-port. `autoJoinDetector` di JS ikut
//! diekspor dan dipakai middleware untuk memanggil `sock.groupAcceptInvite` —
//! butuh `sock`, jadi tidak ada di sini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::js_truthy;

fn status_text(ctx: &Ctx, current: bool) -> String {
    format!(
        "🔗 *AUTO JOIN GROUP*\n\nStatus: *{}*\n\n`{p}autojoingc on` — aktifkan\n`{p}autojoingc off` — nonaktifkan",
        if current { "ON ✅" } else { "OFF ❌" },
        p = ctx.prefix
    )
}

#[plugin(
    name = "autojoingc",
    alias = ["autojoin", "autojoingroup"],
    category = "owner",
    description = "Auto join grup dari link yang terdeteksi di chat",
    usage = ".autojoingc on/off",
    example = ".autojoingc on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autojoingc(ctx: Ctx) -> Reply {
    let arg = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if !matches!(arg.as_str(), "on" | "off") {
        // `db.setting("autoJoinGc") || false` — nilai falsy apa pun
        // (termasuk key yang belum pernah di-set) tampil sebagai OFF.
        let current = js_truthy(ctx.setting("autoJoinGc"));
        return Reply::text(status_text(&ctx, current));
    }

    let enabled = arg == "on";
    Reply::text(format!(
        "{} Auto join group *{}*",
        if enabled { "✅" } else { "❌" },
        if enabled { "diaktifkan" } else { "dinonaktifkan" }
    ))
    .with_mutation(Mutation::SetSetting {
        key: "autoJoinGc".into(),
        value: json!(enabled),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autojoingc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autojoingc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "autoJoinGc");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autojoingc").unwrap();
        assert_eq!(p.meta.name, "autojoingc");
        assert_eq!(p.meta.alias, ["autojoin", "autojoingroup"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Auto join grup dari link yang terdeteksi di chat"
        );
        assert_eq!(p.meta.usage, ".autojoingc on/off");
        assert_eq!(p.meta.example, ".autojoingc on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["autojoin", "autojoingroup"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autojoingc");
        }
    }

    #[test]
    fn an_unset_flag_reads_as_off() {
        for settings in [json!({}), json!({"autoJoinGc": false}), json!({"autoJoinGc": null})] {
            let r = run("", settings.clone());
            assert!(text_of(&r).contains("Status: *OFF ❌*"), "input {settings}");
            assert!(r.mutations().is_empty(), "input {settings}");
        }
    }

    #[test]
    fn a_stored_true_reads_as_on() {
        assert!(text_of(&run("", json!({"autoJoinGc": true}))).contains("Status: *ON ✅*"));
    }

    #[test]
    fn any_argument_other_than_on_or_off_only_shows_status() {
        for bad in ["", "enable", "1", "yes", "of"] {
            let r = run(bad.clone(), json!({}));
            assert!(text_of(&r).starts_with("🔗 *AUTO JOIN GROUP*"), "input {bad:?}");
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn on_and_off_write_their_own_boolean() {
        assert_eq!(stored(&run("on", json!({}))), json!(true));
        assert_eq!(stored(&run("off", json!({"autoJoinGc": true}))), json!(false));
    }

    #[test]
    fn the_switch_is_case_insensitive() {
        assert_eq!(stored(&run("ON", json!({}))), json!(true));
        assert_eq!(stored(&run("Off", json!({}))), json!(false));
    }

    #[test]
    fn the_applied_reply_names_the_direction_written() {
        assert!(text_of(&run("on", json!({}))).contains("diaktifkan"));
        assert!(text_of(&run("off", json!({}))).contains("dinonaktifkan"));
    }
}