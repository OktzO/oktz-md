//! `plugins/owner/setlimitdefault.js` — set `defaultLimit` untuk user baru.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::owner::resetlimitdefault::CONFIG_DEFAULT_LIMIT;

pub(crate) const MIN_LIMIT: i64 = 1;
pub(crate) const MAX_LIMIT: i64 = 1000;
pub(crate) const RANGE_ERR: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Limit harus antara 1 - 1000";

/// `db.setting('defaultLimit') || config.limits?.default || 25` — nilai 0
/// falsy di JS, jadi ikut jatuh ke default; `as_i64` meniru itu lewat
/// `Option`, bukan lewat `unwrap_or`.
pub(crate) fn current_limit(ctx: &Ctx) -> i64 {
    match ctx.setting("defaultLimit") {
        Some(v) if crate::common::js_truthy_value(v) => v
            .as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .unwrap_or(CONFIG_DEFAULT_LIMIT),
        _ => CONFIG_DEFAULT_LIMIT,
    }
}

pub(crate) fn usage(ctx: &Ctx) -> String {
    format!(
        "📊 *sᴇᴛ ᴅᴇғᴀᴜʟᴛ ʟɪᴍɪᴛ*\n\n\
         > Limit default saat ini: `{}`\n\n\
         *Cara pakai:*\n\
         > `{p}setlimitdefault <jumlah>`\n\n\
         *Contoh:*\n\
         > `{p}setlimitdefault 50`",
        current_limit(ctx),
        p = ctx.prefix
    )
}

#[plugin(
    name = "setlimitdefault",
    alias = ["setdefaultlimit", "limitdefault"],
    category = "owner",
    description = "Set default limit untuk user baru",
    usage = ".setlimitdefault <jumlah>",
    example = ".setlimitdefault 50",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setlimitdefault(ctx: Ctx) -> Reply {
    // `!args[0] || isNaN(parseInt(args[0]))` — `parseInt("50px")` = 50, jadi
    // "50px" DITERIMA di sini, beda dari `dellevel` yang memakai `isNaN`.
    let parsed = ctx
        .args
        .first()
        .and_then(|a| crate::common::parse_int(a));

    let Some(new_limit) = parsed else {
        return Reply::text(usage(&ctx));
    };
    if !(MIN_LIMIT..=MAX_LIMIT).contains(&new_limit) {
        return Reply::text(RANGE_ERR);
    }
    Reply::text(format!(
        "✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Default limit diubah menjadi: `{new_limit}`\n> User baru akan mendapat limit ini"
    ))
    .with_mutation(Mutation::SetSetting {
        key: "defaultLimit".into(),
        value: serde_json::Value::from(new_limit),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(args: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setlimitdefault".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("setlimitdefault").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "defaultLimit" => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setlimitdefault").unwrap();
        assert_eq!(p.meta.name, "setlimitdefault");
        assert_eq!(p.meta.alias, ["setdefaultlimit", "limitdefault"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Set default limit untuk user baru");
        assert_eq!(p.meta.usage, ".setlimitdefault <jumlah>");
        assert_eq!(p.meta.example, ".setlimitdefault 50");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setdefaultlimit", "limitdefault"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "setlimitdefault");
        }
    }

    #[test]
    fn a_valid_number_is_stored_and_echoed() {
        let r = run("50", json!({}));
        assert_eq!(written(&r), Some(json!(50)));
        assert!(text_of(&r).contains("> Default limit diubah menjadi: `50`"));
    }

    #[test]
    fn out_of_range_numbers_are_rejected_without_writing() {
        for bad in ["0", "-1", "1001"] {
            let r = run(bad.clone(), json!({}));
            assert_eq!(text_of(&r), RANGE_ERR, "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
        // batas inklusif
        for good in ["1", "1000"] {
            assert!(written(&run(good.clone(), json!({}))).is_some(), "arg {good}");
        }
    }

    #[test]
    fn a_missing_or_unparsable_argument_shows_usage_and_writes_nothing() {
        for bad in ["", "abc", "abc def"] {
            let r = run(bad.clone(), json!({}));
            assert!(text_of(&r).starts_with("📊 *sᴇᴛ ᴅᴇғᴀᴜʟᴛ ʟɪᴍɪᴛ*"), "arg {bad:?}");
            assert!(r.mutations().is_empty(), "arg {bad:?}");
        }
    }

    #[test]
    fn usage_reports_the_stored_value_then_falls_back_to_the_config_default() {
        let r = run("", json!({"defaultLimit": 75}));
        assert!(text_of(&r).contains("> Limit default saat ini: `75`"));
        let r = run("", json!({}));
        assert!(text_of(&r).contains("> Limit default saat ini: `25`"));
    }

    #[test]
    fn usage_uses_the_live_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "setlimitdefault".into(),
            settings: json!({}),
            ..Default::default()
        };
        let out = usage(&ctx);
        assert!(out.contains("!setlimitdefault <jumlah>"));
        assert!(out.contains("!setlimitdefault 50"));
    }
}