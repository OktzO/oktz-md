//! `plugins/info/whatrolethis.js` — syarat akses tiap command dari registry.
//!
//! Baris "Premium 💎" bisa datang dari `db.setting('capprem')[nama]`, bukan
//! dari `pluginConfig.isPremium` — key-nya nama plugin, bukan alias.

use oktz_plugin_api::{Ctx, PluginMeta, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy, render_probe};

pub(crate) const HELP: &str = "🔍 *PENGECEKAN AKSES FITUR*\n\nSistem untuk mengetahui detail informasi persyaratan akses sebuah fitur atau beberapa fitur sekaligus.\n\n*PENGGUNAAN:*\n- *.whatrolethis <nama_fitur1> <nama_fitur2> ...*\n\n*CONTOH PENGGUNAAN:*\n- *.whatrolethis hd jpm warn*\n\n*PENJELASAN:*\nMasukkan perintah beserta satu atau banyak nama fitur yang ingin kamu cek. Pisahkan dengan spasi.";

pub(crate) const HEADER: &str = "🔍 *DETAIL AKSES FITUR*\n\n";

/// Baris syarat akses, urutan `if` di `whatrolethis.js:41-52`. Kosong =
/// semua orang. `premium` = hasil resolusi `db.setting('capprem')` terhadap
/// `pluginConfig.isPremium`, bukan `m.is_premium` mentah.
pub(crate) fn roles_of(m: &PluginMeta, premium: bool) -> Vec<&'static str> {
    let mut roles = Vec::new();
    if m.is_owner {
        roles.push("Owner Only 👑");
    }
    if m.is_admin {
        roles.push("Admin Grup 👮");
    }
    if m.is_bot_admin {
        roles.push("Bot Admin 🤖");
    }
    if m.is_group {
        roles.push("Grup Only 👥");
    }
    if m.is_private {
        roles.push("Private Chat 📱");
    }
    if premium {
        roles.push("Premium 💎");
    }
    if roles.is_empty() {
        roles.push("Free / Semua Orang 🆓");
    }
    roles
}

/// `✅ *name*\n` + tiap role di-indentasi dua spasi.
pub(crate) fn render_one(meta: &PluginMeta, premium: bool) -> String {
    let list = roles_of(meta, premium)
        .iter()
        .map(|r| format!("  - {r}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("✅ *{}*\n{list}\n\n", meta.name)
}

#[plugin(
    name = "whatrolethis",
    alias = ["whatrole", "cekrole", "cekakses"],
    category = "info",
    description = "Cek persyaratan akses banyak fitur sekaligus",
    usage = ".whatrolethis <nama_fitur1> <nama_fitur2> ...",
    example = ".whatrolethis hd jpm warn",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn whatrolethis(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(HELP);
    }
    let overrides = ctx.setting("capprem").and_then(Value::as_object);
    let body = render_probe(HEADER, &ctx.args, |name| {
        match oktz_plugin_api::find(name) {
            Some(p) => {
                // `overrides[name] !== undefined ? overrides[name] : isPremium`
                let premium = overrides
                    .and_then(|m| m.get(name))
                    .map_or(p.meta.is_premium, |v| js_truthy(Some(v)));
                render_one(&p.meta, premium)
            }
            None => String::new(),
        }
    });
    Reply::text(body).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Ctx;
    use serde_json::{Value};

    fn run(args: &[&str]) -> String {
        run_with(args, serde_json::json!({}))
    }

    fn run_with(args: &[&str], settings: Value) -> String {
        handler(args, settings)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn handler(args: &[&str], settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "whatrolethis".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("whatrolethis").unwrap().handler)(ctx)
    }

    fn meta_of(name: &str) -> PluginMeta {
        oktz_plugin_api::find(name).unwrap().meta.clone()
    }

    /// Roles dari `pluginConfig` apa adanya — tanpa `capprem`.
    fn roles_of_meta(name: &str) -> Vec<&'static str> {
        let m = meta_of(name);
        roles_of(&m, m.is_premium)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("whatrolethis").unwrap();
        assert_eq!(p.meta.name, "whatrolethis");
        assert_eq!(p.meta.alias, ["whatrole", "cekrole", "cekakses"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Cek persyaratan akses banyak fitur sekaligus");
        assert_eq!(p.meta.usage, ".whatrolethis <nama_fitur1> <nama_fitur2> ...");
        assert_eq!(p.meta.example, ".whatrolethis hd jpm warn");
        assert_eq!(p.meta.cooldown, 5);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["whatrole", "cekrole", "cekakses"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "whatrolethis");
        }
    }

    #[test]
    fn no_arguments_returns_the_help_block_verbatim() {
        assert_eq!(run(&[]), HELP);
        assert!(HELP.contains("- *.whatrolethis <nama_fitur1> <nama_fitur2> ...*"));
    }

    // -- tabel role --------------------------------------------------------

    #[test]
    fn a_plugin_with_no_restriction_is_free_for_everyone() {
        assert_eq!(roles_of_meta("dare"), ["Free / Semua Orang 🆓"]);
        assert_eq!(
            roles_of(&PluginMeta::default(), false),
            ["Free / Semua Orang 🆓"]
        );
    }

    #[test]
    fn each_flag_adds_exactly_its_own_line_in_the_js_order() {
        assert_eq!(roles_of_meta("tagall"), ["Admin Grup 👮", "Grup Only 👥"]);
        assert_eq!(roles_of_meta("listadmin"), ["Grup Only 👥"]);
        assert_eq!(
            roles_of_meta("templateplugin"),
            ["Owner Only 👑"],
            "templateplugin owner-only dan `isPrivate: false`"
        );
        let all = roles_of(
            &PluginMeta {
                is_owner: true,
                is_admin: true,
                is_bot_admin: true,
                is_group: true,
                is_private: true,
                is_premium: true,
                ..PluginMeta::default()
            },
            true,
        );
        assert_eq!(
            all,
            [
                "Owner Only 👑",
                "Admin Grup 👮",
                "Bot Admin 🤖",
                "Grup Only 👥",
                "Private Chat 📱",
                "Premium 💎"
            ],
            "urutan harus sama dengan urutan `if` di JS"
        );
        assert_eq!(
            roles_of(&PluginMeta::default(), true),
            ["Premium 💎"],
            "premium tetap baris terakhir walau semua flag lain mati"
        );
    }

    #[test]
    fn a_group_only_command_never_reports_private_chat() {
        let r = run(&["tagall"]);
        assert!(r.contains("\n  - Admin Grup 👮"));
        assert!(
            r.contains("\n  - Grup Only 👥"),
            "baris role terakhir tidak punya newline — balasan sudah di-trim"
        );
        assert!(!r.contains("Private Chat"), "isPrivate=false untuk tagall");
        assert!(!r.contains("Free / Semua Orang"));
    }

    #[test]
    fn rendering_indents_every_role_by_two_spaces_and_blank_lines_follow() {
        let m = meta_of("tagall");
        assert_eq!(
            render_one(&m, m.is_premium),
            "✅ *tagall*\n  - Admin Grup 👮\n  - Grup Only 👥\n\n"
        );
    }

    // -- loop ---------------------------------------------------------------

    #[test]
    fn an_alias_resolves_and_reports_the_canonical_name() {
        assert!(run(&["everyone"]).starts_with("🔍 *DETAIL AKSES FITUR*\n\n✅ *tagall*"));
        assert_eq!(run(&["everyone"]), run(&["tagall"]));
    }

    #[test]
    fn an_unknown_command_gets_the_not_found_line() {
        assert_eq!(
            run(&["tidak-ada"]),
            "🔍 *DETAIL AKSES FITUR*\n\n❌ *tidak-ada* : Tidak ditemukan!"
        );
    }

    #[test]
    fn the_lookup_is_case_insensitive() {
        assert_eq!(run(&["TAGALL"]), run(&["tagall"]));
        assert!(run(&["TAGALL"]).contains("✅ *tagall*"));
    }

    #[test]
    fn many_arguments_keep_their_order_and_the_reply_is_trimmed() {
        let out = run(&["dare", "tidak-ada", "listadmin"]);
        assert_eq!(
            out,
            concat!(
                "🔍 *DETAIL AKSES FITUR*\n\n",
                "✅ *dare*\n  - Free / Semua Orang 🆓\n\n",
                "❌ *tidak-ada* : Tidak ditemukan!\n\n",
                "✅ *listadmin*\n  - Grup Only 👥"
            )
        );
        assert!(!out.ends_with('\n') && !out.ends_with(' '));
    }

    #[test]
    fn the_whole_new_batch_is_probeable_through_the_plugin() {
        let out = run(&[
            "tagall", "listadmin", "listantilink", "listtoxic", "listwarn", "topchat", "intro",
            "rulesgrup", "inventory", "koin", "exp", "level", "energi", "birthday",
            "howmuchenergy", "whatrolethis", "fiturpremium",
        ]);
        assert!(!out.contains("Tidak ditemukan"), "salah nama/alias:\n{out}");
        assert_eq!(out.matches("✅ *").count(), 17);
    }

    // -- override `db.setting('capprem')` -----------------------------------

    #[test]
    fn an_absent_or_empty_capprem_setting_falls_back_to_the_plugin_meta() {
        let baseline = run(&["dare", "tagall"]);
        for settings in [
            serde_json::json!({}),
            serde_json::json!({"capprem": {}}),
            serde_json::json!({"capprem": null}),
            serde_json::json!({"capprem": [1, 2]}),
        ] {
            assert_eq!(
                run_with(&["dare", "tagall"], settings.clone()),
                baseline,
                "setting harus transparan saat kosong/aneh: {settings}"
            );
        }
    }

    #[test]
    fn capprem_true_adds_the_premium_line_to_a_free_command() {
        let out = run_with(&["dare"], serde_json::json!({"capprem": {"dare": true}}));
        assert_eq!(out, "🔍 *DETAIL AKSES FITUR*\n\n✅ *dare*\n  - Premium 💎");
    }

    #[test]
    fn capprem_false_removes_the_premium_line_from_a_premium_command() {
        let meta = oktz_plugin_api::find("cekprem").unwrap().meta.clone();
        assert!(meta.is_premium, "premis test: cekprem harus plugin premium");
        assert!(!meta.is_group && !meta.is_private, "satu-satunya syaratnya premium");
        assert_eq!(
            run_with(&["cekprem"], serde_json::json!({})),
            "🔍 *DETAIL AKSES FITUR*\n\n✅ *cekprem*\n  - Premium 💎"
        );
        assert_eq!(
            run_with(&["cekprem"], serde_json::json!({"capprem": {"cekprem": false}})),
            "🔍 *DETAIL AKSES FITUR*\n\n✅ *cekprem*\n  - Free / Semua Orang 🆓",
            "`capprem[cekprem] = false` mematikan satu-satunya role"
        );
    }

    #[test]
    fn a_falsy_but_present_override_wins_over_the_plugin_meta() {
        // `overrides[name] !== undefined` benar-benar dipakai di JS, jadi nilai
        // falsy yang tersimpan tetap mengalahkan `isPremium`.
        for value in [serde_json::json!(false), serde_json::json!(null), serde_json::json!(0), serde_json::json!("")] {
            let out = run_with(&["cekprem"], serde_json::json!({ "capprem": { "cekprem": value } }));
            assert!(out.ends_with("  - Free / Semua Orang 🆓"), "override {value}: {out}");
        }
    }

    #[test]
    fn capprem_is_keyed_by_canonical_name_not_by_alias() {
        // `cekrole` = alias dari whatrolethis, jadi override "cekrole" tidak
        // pernah dibaca; yang dipakai adalah meta plugin itu sendiri.
        let by_alias = run_with(&["cekrole"], serde_json::json!({"capprem": {"cekrole": true}}));
        assert!(
            by_alias.contains("Free / Semua Orang 🆓"),
            "override by alias diabaikan: {by_alias}"
        );
        let by_name = run_with(
            &["cekrole"],
            serde_json::json!({"capprem": {"whatrolethis": true}}),
        );
        assert!(by_name.contains("  - Premium 💎"), "{by_name}");
    }

    #[test]
    fn only_the_probe_branch_carries_a_reaction() {
        let help_reply = handler(&[], serde_json::json!({}));
        assert!(matches!(
            help_reply.without_mutations(),
            Reply::Text { .. }
        ));

        let probed = handler(&["dare"], serde_json::json!({}));
        let mut cur = probed.without_mutations();
        let mut seen = 0;
        while let Reply::Reaction { emoji, reply } = cur {
            assert_eq!(emoji, "✅");
            seen += 1;
            cur = reply;
        }
        assert_eq!(seen, 1);
        assert!(matches!(cur, Reply::Text { .. }));
        assert!(probed.mutations().is_empty(), "plugin ini hanya membaca");
    }
}