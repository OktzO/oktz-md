//! `plugins/owner/stopbcpc.js` — set flag berhenti untuk broadcast private.
//!
//! `ponytail:` JS memakai global mutable `global.statusBcpc` /
//! `global.stopBcpc`. `Ctx` hanya membawa `sessions`, jadi keduanya dipindah ke
//! sana DENGAN KEY YANG SAMA — `bcpc.js` yang di-port harus memakai
//! `session_set("statusBcpc", ..)` dan `session_take("stopBcpc")`, kalau tidak
//! kedua plugin tidak pernah melihat satu sama lain.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

pub(crate) const STATUS_KEY: &str = "statusBcpc";
pub(crate) const STOP_KEY: &str = "stopBcpc";
pub(crate) const NOT_RUNNING: &str = "❌ Tidak ada broadcast private yang sedang berjalan.";
pub(crate) const STOPPING: &str = "⏹️ Menghentikan broadcast private...";

pub(crate) fn is_running(ctx: &Ctx) -> bool {
    ctx.session_get(STATUS_KEY).is_some_and(|v| crate::common::js_truthy_value(&v))
}

#[plugin(
    name = "stopbcpc",
    alias = ["stopbroadcastpc"],
    category = "owner",
    description = "Hentikan broadcast private yang sedang berjalan",
    usage = ".stopbcpc",
    example = ".stopbcpc",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stopbcpc(ctx: Ctx) -> Reply {
    if !is_running(&ctx) {
        return Reply::text(NOT_RUNNING);
    }
    ctx.session_set(STOP_KEY, Value::Bool(true));
    Reply::text(STOPPING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(entries: &[(&str, Value)]) -> Reply {
        let mut map = std::collections::HashMap::new();
        for (k, v) in entries {
            map.insert((*k).to_string(), v.clone());
        }
        let ctx = Ctx {
            prefix: ".".into(),
            command: "stopbcpc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            sessions: std::sync::Arc::new(std::sync::Mutex::new(map)),
            ..Default::default()
        };
        (oktz_plugin_api::find("stopbcpc").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("stopbcpc").unwrap();
        assert_eq!(p.meta.name, "stopbcpc");
        assert_eq!(p.meta.alias, ["stopbroadcastpc"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Hentikan broadcast private yang sedang berjalan"
        );
        assert_eq!(p.meta.usage, ".stopbcpc");
        assert_eq!(p.meta.example, ".stopbcpc");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("stopbroadcastpc").unwrap().meta.name,
            "stopbcpc"
        );
    }

    #[test]
    fn a_running_broadcast_gets_the_stop_flag_set() {
        let ctx = Ctx {
            command: "stopbcpc".into(),
            sessions: oktz_plugin_api::new_session_store(),
            ..Default::default()
        };
        ctx.session_set(STATUS_KEY, json!(true));
        let r = (oktz_plugin_api::find("stopbcpc").unwrap().handler)(ctx.clone());
        assert_eq!(r.text_body(), Some(STOPPING));
        assert_eq!(
            ctx.session_get(STOP_KEY),
            Some(json!(true)),
            "bcpc.js yang di-port harus membaca key ini"
        );
    }

    #[test]
    fn nothing_running_reports_the_short_message_and_sets_nothing() {
        let ctx = Ctx {
            command: "stopbcpc".into(),
            sessions: oktz_plugin_api::new_session_store(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("stopbcpc").unwrap().handler)(ctx.clone());
        assert_eq!(r.text_body(), Some(NOT_RUNNING));
        assert_eq!(ctx.session_get(STOP_KEY), None);
    }

    #[test]
    fn the_status_flag_is_read_with_js_truthiness() {
        for (stored, running) in [
            (json!(true), true),
            (json!(1), true),
            (json!("ya"), true),
            (json!(false), false),
            (json!(0), false),
            (json!(""), false),
        ] {
            let ctx = Ctx {
                command: "stopbcpc".into(),
                sessions: oktz_plugin_api::new_session_store(),
                ..Default::default()
            };
            ctx.session_set(STATUS_KEY, stored.clone());
            assert_eq!(is_running(&ctx), running, "stored {stored}");
        }
    }

    #[test]
    fn unrelated_session_keys_do_not_make_it_look_busy() {
        assert_eq!(
            run(&[("sesiBCPC", json!(true))]).text_body(),
            Some(NOT_RUNNING)
        );
    }
}