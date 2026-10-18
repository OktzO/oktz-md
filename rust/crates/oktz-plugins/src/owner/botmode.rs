//! `plugins/owner/botmode.js` — mode bot global atau per grup.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy;

pub(crate) const VALID_MODES: [&str; 6] = ["md", "cpanel", "store", "pushkontak", "otp", "all"];

/// `MODE_DESCRIPTIONS` (`botmode.js:19-25`) dalam urutan deklarasi — JS
/// mengiterasi `Object.entries`, jadi urutan ini yang tampil.
pub(crate) const MODE_DESCRIPTIONS: [(&str, &str); 6] = [
    ("md", "Mode default, semua fitur kecuali panel/store/pushkontak"),
    ("cpanel", "Mode panel, main + group + sticker + owner + tools + panel"),
    ("store", "Mode store manual, main + group + sticker + owner + store"),
    (
        "pushkontak",
        "Mode pushkontak, main + group + sticker + owner + pushkontak",
    ),
    ("otp", "Mode OTP service, main + group + sticker + owner + otp"),
    ("all", "Mode full, SEMUA fitur dari semua mode bisa diakses"),
];

/// `db.setting('botMode') || 'all'` — string kosong ikut jadi `all`.
pub(crate) fn global_mode(ctx: &Ctx) -> String {
    match ctx.setting("botMode").and_then(|v| v.as_str()) {
        Some(m) if !m.is_empty() => m.to_string(),
        _ => "all".to_string(),
    }
}

fn status_text(ctx: &Ctx) -> String {
    let global = global_mode(ctx);
    let group_mode = ctx
        .group_field("botMode")
        .filter(|v| js_truthy(Some(v)))
        .and_then(|v| v.as_str());
    let current = if ctx.is_group {
        group_mode.unwrap_or(&global)
    } else {
        &global
    };

    let mut txt = String::from("╭┈┈⬡「 🤖 *ʙᴏᴛ ᴍᴏᴅᴇ* 」\n");
    txt.push_str(&format!("┃ ㊗ ɢʟᴏʙᴀʟ: *{}*\n", global.to_uppercase()));
    if ctx.is_group {
        txt.push_str(&format!(
            "┃ ㊗ ɢʀᴜᴘ: *{}*\n",
            group_mode.unwrap_or("INHERIT").to_uppercase()
        ));
    }
    txt.push_str("╰┈┈⬡\n\n╭┈┈⬡「 📋 *ᴀᴠᴀɪʟᴀʙʟᴇ ᴍᴏᴅᴇs* 」\n");
    for (key, desc) in MODE_DESCRIPTIONS {
        txt.push_str(&format!(
            "┃ ㊗ *{}*{}\n┃   {desc}\n",
            key.to_uppercase(),
            if key == current { " ✅" } else { "" }
        ));
    }
    txt.push_str("╰┈┈⬡\n\n*ꜰʟᴀɢ sᴛᴏʀᴇ:*\n");
    txt.push_str(&format!(
        "> `{p}botmode store` - Manual order\n> `{p}botmode md` → Mode default\n> `{p}botmode all` → Semua fitur",
        p = ctx.prefix
    ));
    txt
}

#[plugin(
    name = "botmode",
    alias = ["setmode", "mode"],
    category = "owner",
    description = "Mengatur mode bot (md/cpanel/store/pushkontak/all)",
    usage = ".botmode <mode>",
    example = ".botmode store",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn botmode(ctx: Ctx) -> Reply {
    let mode = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if mode.is_empty() {
        return Reply::text(status_text(&ctx));
    }

    if !VALID_MODES.contains(&mode.as_str()) {
        return Reply::text(format!(
            "❌ *ᴍᴏᴅᴇ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Mode tersedia: `{}`",
            VALID_MODES.join(", ")
        ));
    }

    let desc = MODE_DESCRIPTIONS
        .iter()
        .find(|(k, _)| *k == mode)
        .map_or("", |(_, d)| *d);

    let store_note = if ctx.is_group && mode == "store" {
        "\n\n📋 *Manual mode*\n> Admin perlu confirm order manual"
    } else {
        ""
    };
    let scope = if ctx.is_group {
        "_Mode grup ini juga diubah._"
    } else {
        "_Mode global diubah._"
    };

    let reply = Reply::text(format!(
        "✅ *ᴍᴏᴅᴇ ᴅɪᴜʙᴀʜ*\n\n> Mode: *{}*\n> {desc}{store_note}\n\n> {scope}",
        mode.to_uppercase()
    ))
    .with_reaction("✅");

    if ctx.is_group {
        // `storeConfig.products` di-default ke array kosong supaya `store`
        // bisa langsung dipakai; field store lain tetap diwarisi karena
        // `Mutation::SetGroup` menyatu per field.
        let mut data = serde_json::json!({ "botMode": mode });
        if mode == "store" {
            data["storeConfig"] = serde_json::json!({
                "products": ctx
                    .group_field("storeConfig.products")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            });
        }
        reply.with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data,
        })
    } else {
        reply.with_mutation(Mutation::SetSetting {
            key: "botMode".into(),
            value: serde_json::json!(mode),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, is_group: bool, group: Value, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "botmode".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("botmode").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "botMode");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn group_write(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
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
        let p = oktz_plugin_api::find("botmode").unwrap();
        assert_eq!(p.meta.name, "botmode");
        assert_eq!(p.meta.alias, ["setmode", "mode"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengatur mode bot (md/cpanel/store/pushkontak/all)"
        );
        assert_eq!(p.meta.usage, ".botmode <mode>");
        assert_eq!(p.meta.example, ".botmode store");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setmode", "mode"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "botmode");
        }
    }

    #[test]
    fn no_argument_lists_every_mode_and_writes_nothing() {
        let r = run("", false, serde_json::json!({}), serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.contains("┃ ㊗ ɢʟᴏʙᴀʟ: *ALL*"), "{t}");
        for (key, desc) in MODE_DESCRIPTIONS {
            assert!(t.contains(&key.to_uppercase()), "mode {key} hilang: {t}");
            assert!(t.contains(desc), "deskripsi {key} hilang: {t}");
        }
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_current_mode_is_the_only_one_marked_active() {
        let r = run("", false, serde_json::json!({}), serde_json::json!({ "botMode": "store" }));
        let t = text_of(&r);
        assert_eq!(t.matches(" ✅").count(), 1, "tepat satu mode aktif: {t}");
        assert!(t.contains("┃ ㊗ *STORE* ✅"), "{t}");
    }

    #[test]
    fn an_unset_global_mode_defaults_to_all() {
        assert_eq!(global_mode(&Ctx::default()), "all");
        let ctx = Ctx {
            settings: serde_json::json!({ "botMode": "" }),
            ..Default::default()
        };
        assert_eq!(global_mode(&ctx), "all", "string kosong ikut ||");
    }

    #[test]
    fn an_unknown_mode_is_refused_and_writes_nothing() {
        for bad in ["nope", "1", "stor"] {
            let r = run(bad, false, serde_json::json!({}), serde_json::json!({}));
            assert!(text_of(&r).starts_with("❌ *ᴍᴏᴅᴇ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*"), "input {bad:?}");
            assert!(text_of(&r).contains("md, cpanel, store, pushkontak, otp, all"));
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn a_private_chat_writes_the_global_setting() {
        let r = run("store", false, serde_json::json!({}), serde_json::json!({}));
        assert_eq!(setting(&r), serde_json::json!("store"));
        assert!(text_of(&r).contains("> _Mode global diubah._"), "{}", text_of(&r));
        assert!(!text_of(&r).contains("Manual mode"), "catatan store hanya di grup");
    }

    #[test]
    fn a_group_writes_the_group_record_instead() {
        let r = run("md", true, serde_json::json!({}), serde_json::json!({}));
        assert_eq!(group_write(&r), serde_json::json!({ "botMode": "md" }));
        assert!(text_of(&r).contains("> _Mode grup ini juga diubah._"), "{}", text_of(&r));
    }

    #[test]
    fn store_mode_in_a_group_adds_the_manual_note_and_seeds_products() {
        let r = run("store", true, serde_json::json!({}), serde_json::json!({}));
        assert!(text_of(&r).contains("📋 *Manual mode*"), "{}", text_of(&r));
        assert_eq!(group_write(&r)["storeConfig"], serde_json::json!({ "products": [] }));
    }

    #[test]
    fn an_existing_product_list_survives_a_store_mode_write() {
        let group = serde_json::json!({ "storeConfig": { "products": [{ "name": "S&K" }] } });
        let r = run("store", true, group, serde_json::json!({}));
        assert_eq!(
            group_write(&r)["storeConfig"]["products"],
            serde_json::json!([{ "name": "S&K" }])
        );
    }

    #[test]
    fn a_group_status_shows_inherit_when_no_group_mode_is_stored() {
        let r = run("", true, serde_json::json!({ "botMode": null }), serde_json::json!({ "botMode": "md" }));
        assert!(text_of(&r).contains("┃ ㊗ ɢʀᴜᴘ: *INHERIT*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ ㊗ *MD* ✅"), "mode global yang aktif");
    }

    #[test]
    fn the_mode_argument_is_lowercased_before_matching() {
        assert_eq!(setting(&run("STORE", false, serde_json::json!({}), serde_json::json!({}))), serde_json::json!("store"));
    }

    #[test]
    fn the_applied_reply_reacts_and_is_paired_with_the_write() {
        let r = run("all", false, serde_json::json!({}), serde_json::json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}