//! `plugins/group/antitoxic.js` — sakelar anti-toxic per grup.
//!
//! Hanya handler `.antitoxic` yang dipindah. Tiga hal lain tidak:
//! - `isToxic` (:26) membangun `RegExp` dari kata yang disimpan, jadi butuh
//!   mesin regex. Dipakai `src/handler.js:863`, bukan handler ini.
//! - `handleToxicMessage` (:62) butuh `sock.sendMessage({delete})` (:73),
//!   `sock.groupParticipantsUpdate` (:81), dan `sock.sendMessage` (:88, :98).
//! - `gpMsg` (:50) membaca `config.groupProtection` dan hanya dipakai
//!   `handleToxicMessage`.
//!
//! `DEFAULT_TOXIC_WORDS` dipakai ulang dari
//! [`crate::group::listtoxic`] — konstanta yang sama di `antitoxic.js:20-24`,
//! supaya tidak ada dua salinan yang bisa melenceng diam-diam.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::group::listtoxic::DEFAULT_TOXIC_WORDS;

const ON: &str = "✅ *Antitoxic diaktifkan*";
const OFF: &str = "❌ *Antitoxic dinonaktifkan*";
const BAD_WARN: &str = "❌ Masukkan angka 1-10\n> Contoh: `.antitoxic warn 5`";
const BAD_METHOD: &str = "❌ Pilih metode: *kick* atau *delete*\n> Contoh: `.antitoxic metode kick`";
const UNKNOWN: &str =
    "❌ Sub-command tidak dikenal.\n> Ketik `.antitoxic` untuk melihat daftar command.";

fn status(ctx: &Ctx) -> String {
    let on = crate::common::js_truthy(ctx.group_field("antitoxic"));
    // `toxicWords?.length || 20` — array kosong (len 0) juga falsy, jadi
    // balik ke daftar bawaan, bukan menampilkan "0".
    let mut count = ctx.group_list("toxicWords").len();
    if count == 0 {
        count = DEFAULT_TOXIC_WORDS.len();
    }
    // `toxicMaxWarn || 3` — 0 falsy.
    let max_warn = ctx
        .group_field("toxicMaxWarn")
        .and_then(serde_json::Value::as_i64)
        .filter(|v| *v != 0)
        .unwrap_or(3);
    // `toxicMethod || 'kick'` — string kosong falsy.
    let method = ctx
        .group_field("toxicMethod")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("kick");

    format!(
        "🛡️ *ᴀɴᴛɪᴛᴏxɪᴄ*\n\n\
         > Status: *{}*\n\
         > Kata: *{count}*\n\
         > Max Warn: *{max_warn}*\n\
         > Metode: *{method}*\n\n\
         *Command:*\n\
         > `.antitoxic on/off`\n\
         > `.antitoxic warn <1-10>`\n\
         > `.antitoxic metode kick/delete`\n\
         > `.addtoxic <kata>`\n\
         > `.deltoxic <kata>`\n\
         > `.listtoxic`",
        if on { "✅ ON" } else { "❌ OFF" }
    )
}

fn write(ctx: &Ctx, data: serde_json::Value, text: &str) -> Reply {
    Reply::text(text).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[plugin(
    name = "antitoxic",
    alias = ["toxic", "antitoxik"],
    category = "group",
    description = "Mengatur antitoxic di grup",
    usage = ".antitoxic <on/off/warn/metode>",
    example = ".antitoxic on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn antitoxic(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());

    let Some(sub) = sub else {
        return Reply::text(status(&ctx));
    };

    match sub.as_str() {
        "on" => write(&ctx, serde_json::json!({ "antitoxic": true }), ON).with_reaction("✅"),
        "off" => write(&ctx, serde_json::json!({ "antitoxic": false }), OFF).with_reaction("❌"),
        "warn" => {
            // `parseInt(args[1])`, `NaN` jadi 0 lewat `!count`.
            let count = ctx
                .args
                .get(1)
                .and_then(|a| crate::common::parse_int(a))
                .unwrap_or(0);
            if !(1..=10).contains(&count) {
                return Reply::text(BAD_WARN);
            }
            write(
                &ctx,
                serde_json::json!({ "toxicMaxWarn": count }),
                &format!("✅ Max peringatan diubah ke *{count}*"),
            )
            .with_reaction("✅")
        }
        "metode" | "method" | "mode" => {
            let method = ctx.args.get(1).map(|a| a.to_lowercase());
            let Some(method) = method.filter(|m| matches!(m.as_str(), "kick" | "delete")) else {
                return Reply::text(BAD_METHOD);
            };
            // `&method` di `Value` — teks balasan tetap pakai string yang sama.
            write(
                &ctx,
                serde_json::json!({ "toxicMethod": &method }),
                &format!("✅ Metode diubah ke *{method}*"),
            )
            .with_reaction("✅")
        }
        _ => Reply::text(UNKNOWN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antitoxic".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antitoxic").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("antitoxic").unwrap();
        assert_eq!(p.meta.name, "antitoxic");
        assert_eq!(p.meta.alias, ["toxic", "antitoxik"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mengatur antitoxic di grup");
        assert_eq!(p.meta.usage, ".antitoxic <on/off/warn/metode>");
        assert_eq!(p.meta.example, ".antitoxic on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        // `isBotAdmin` tidak ada di pluginConfig JS.
        assert!(!p.meta.is_bot_admin);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["toxic", "antitoxik"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antitoxic");
        }
    }

    #[test]
    fn no_subcommand_only_reports_and_never_writes() {
        let r = run(serde_json::json!({}), "");
        assert!(r.mutations().is_empty());
        assert!(text_of(&r).contains("> Status: *❌ OFF*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Kata: *20*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Max Warn: *3*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Metode: *kick*"), "{}", text_of(&r));
    }

    #[test]
    fn the_status_panel_reads_every_stored_field() {
        let r = run(
            serde_json::json!({
                "antitoxic": true,
                "toxicWords": ["satu", "dua", "tiga"],
                "toxicMaxWarn": 7,
                "toxicMethod": "delete"
            }),
            "",
        );
        assert!(text_of(&r).contains("> Status: *✅ ON*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Kata: *3*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Max Warn: *7*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Metode: *delete*"), "{}", text_of(&r));
    }

    #[test]
    fn falsy_stored_values_fall_back_to_the_defaults() {
        // `toxicWords?.length || 20`, `toxicMaxWarn || 3`, `toxicMethod || 'kick'`
        let r = run(
            serde_json::json!({ "toxicWords": [], "toxicMaxWarn": 0, "toxicMethod": "" }),
            "",
        );
        assert!(text_of(&r).contains("> Kata: *20*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Max Warn: *3*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Metode: *kick*"), "{}", text_of(&r));
    }

    #[test]
    fn on_and_off_write_the_flag_with_their_reaction() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antitoxic": true }));
        assert!(matches!(
            on.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let off = run(serde_json::json!({ "antitoxic": true }), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antitoxic": false }));
        assert!(matches!(
            off.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "❌"
        ));
    }

    #[test]
    fn warn_accepts_one_to_ten_and_stores_the_parsed_number() {
        for n in 1..=10 {
            let r = run(serde_json::json!({}), &format!("warn {n}"));
            assert_eq!(written(&r), &serde_json::json!({ "toxicMaxWarn": n }), "warn {n}");
            assert!(
                text_of(&r).contains(&format!("✅ Max peringatan diubah ke *{n}*")),
                "{}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn warn_outside_the_range_is_refused_and_writes_nothing() {
        // `!count || count < 1 || count > 10` — "abc" jadi NaN -> !count.
        for text in ["warn", "warn 0", "warn -3", "warn 11", "warn abc"] {
            let r = run(serde_json::json!({}), text);
            assert_eq!(text_of(&r), BAD_WARN, "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn warn_uses_parse_int_so_trailing_digits_are_dropped() {
        let r = run(serde_json::json!({}), "warn 5luck");
        assert_eq!(written(&r), &serde_json::json!({ "toxicMaxWarn": 5 }));
        assert!(
            text_of(&r).contains("*5*"),
            "angka yang tampil harus hasil parseInt: {}",
            text_of(&r)
        );
    }

    #[test]
    fn all_three_method_spellings_write_the_same_field() {
        for input in ["metode", "method", "mode"] {
            for method in ["kick", "delete"] {
                let r = run(serde_json::json!({}), &format!("{input} {method}"));
                assert_eq!(
                    written(&r),
                    &serde_json::json!({ "toxicMethod": method }),
                    "{input} {method}"
                );
                assert!(
                    text_of(&r).contains(&format!("✅ Metode diubah ke *{method}*")),
                    "{}",
                    text_of(&r)
                );
            }
        }
    }

    #[test]
    fn an_unknown_method_is_refused_and_writes_nothing() {
        for text in ["metode", "metode warn", "metode hapus", "metode kickall"] {
            let r = run(serde_json::json!({}), text);
            assert_eq!(text_of(&r), BAD_METHOD, "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn an_unknown_subcommand_falls_through_to_the_tail_reply() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), UNKNOWN);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn subcommands_are_lowercased_before_matching() {
        assert_eq!(written(&run(serde_json::json!({}), "ON")), &serde_json::json!({ "antitoxic": true }));
        assert_eq!(written(&run(serde_json::json!({}), "METODE DELETE")), &serde_json::json!({ "toxicMethod": "delete" }));
    }
}