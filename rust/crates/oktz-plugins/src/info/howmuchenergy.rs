//! `plugins/info/howmuchenergy.js` — biaya energi tiap command dari registry.
//!
//! Harga diambil dari `db.setting('capenergi')[nama]`, dan hanya itu yang
//! dibaca plugin ini — `config.energi.*` milik middleware, bukan plugin.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::render_probe;

pub(crate) const HELP: &str = "🔍 *PENGECEKAN ENERGI FITUR*\n\nSistem untuk mengetahui berapa besar potongan energi yang dibutuhkan untuk menggunakan satu atau banyak fitur sekaligus.\n\n*PENGGUNAAN:*\n- *.howmuchenergy <nama_fitur1> <nama_fitur2> ...*\n\n*CONTOH PENGGUNAAN:*\n- *.howmuchenergy hd jpm*\n\n*PENJELASAN:*\nMasukkan perintah beserta satu atau banyak nama fitur yang ingin kamu cek. Pisahkan dengan spasi.";

pub(crate) const HEADER: &str = "🔋 *DETAIL ENERGI FITUR*\n\n";

#[plugin(
    name = "howmuchenergy",
    alias = [],
    category = "info",
    description = "Mengecek penggunaan energi banyak fitur sekaligus",
    usage = ".howmuchenergy <nama_fitur1> <nama_fitur2> ...",
    example = ".howmuchenergy hd jpm",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn howmuchenergy(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(HELP);
    }
    // `db.setting('capenergi') || {}` — key-nya nama plugin, bukan alias.
    let overrides = ctx.setting("capenergi").and_then(Value::as_object);
    let body = render_probe(HEADER, &ctx.args, |name| {
        let cost = overrides
            .and_then(|m| m.get(name))
            .and_then(Value::as_i64)
            .unwrap_or_else(|| oktz_plugin_api::find(name).map_or(0, |p| p.meta.energi));
        format!("✅ *{name}* : {cost} Energi\n")
    });
    Reply::text(body).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Ctx;
    use serde_json::{Value, json};

    fn run(args: &[&str]) -> String {
        run_with(args, json!({}))
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
            command: "howmuchenergy".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("howmuchenergy").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("howmuchenergy").unwrap();
        assert_eq!(p.meta.name, "howmuchenergy");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "info");
        assert_eq!(
            p.meta.description,
            "Mengecek penggunaan energi banyak fitur sekaligus"
        );
        assert_eq!(p.meta.usage, ".howmuchenergy <nama_fitur1> <nama_fitur2> ...");
        assert_eq!(p.meta.example, ".howmuchenergy hd jpm");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn no_arguments_returns_the_help_block_verbatim() {
        assert_eq!(run(&[]), HELP);
        assert!(HELP.starts_with("🔍 *PENGECEKAN ENERGI FITUR*\n\n"));
        assert!(HELP.contains("- *.howmuchenergy <nama_fitur1> <nama_fitur2> ...*"));
        assert!(HELP.ends_with("Pisahkan dengan spasi."));
    }

    #[test]
    fn a_known_command_reports_its_energi_cost_under_the_header() {
        assert_eq!(
            run(&["readmore"]),
            "🔋 *DETAIL ENERGI FITUR*\n\n✅ *readmore* : 1 Energi"
        );
        assert!(run(&["dare"]).ends_with("✅ *dare* : 0 Energi"));
    }

    #[test]
    fn an_alias_resolves_and_reports_the_canonical_name() {
        assert_eq!(
            run(&["kkmbg"]),
            "🔋 *DETAIL ENERGI FITUR*\n\n✅ *kalkulatormbg* : 1 Energi"
        );
        assert!(run(&["saldo"]).ends_with("✅ *koin* : 0 Energi"));
    }

    #[test]
    fn an_unknown_command_gets_the_not_found_line() {
        assert_eq!(
            run(&["tidak-ada"]),
            "🔋 *DETAIL ENERGI FITUR*\n\n❌ *tidak-ada* : Tidak ditemukan!"
        );
    }

    #[test]
    fn the_lookup_is_case_insensitive_but_the_report_is_canonical() {
        assert_eq!(run(&["READMORE"]), run(&["readmore"]));
        assert_eq!(run(&["ReadMore"]), run(&["readmore"]));
        assert!(
            run(&["TIDAK-ADA"]).contains("❌ *tidak-ada*"),
            "argumen diturunkan"
        );
    }

    #[test]
    fn many_arguments_are_reported_in_order_and_the_reply_is_trimmed() {
        let out = run(&["readmore", "tidak-ada", "dare"]);
        assert_eq!(
            out,
            concat!(
                "🔋 *DETAIL ENERGI FITUR*\n\n",
                "✅ *readmore* : 1 Energi\n",
                "❌ *tidak-ada* : Tidak ditemukan!\n\n",
                "✅ *dare* : 0 Energi"
            )
        );
        assert!(
            !out.ends_with('\n') && !out.ends_with(' '),
            "`responseText.trim()`"
        );
    }

    #[test]
    fn the_energi_value_always_matches_the_registered_meta() {
        for name in [
            "readmore",
            "dare",
            "truth",
            "cekbaik",
            "soulmatch",
            "kalkulatormbg",
        ] {
            let expected = oktz_plugin_api::find(name).unwrap().meta.energi;
            assert_eq!(
                run(&[name]),
                format!("🔋 *DETAIL ENERGI FITUR*\n\n✅ *{name}* : {expected} Energi"),
                "{name}"
            );
        }
    }

    // -- override `db.setting('capenergi')` ---------------------------------

    #[test]
    fn an_absent_or_empty_capenergi_setting_falls_back_to_the_plugin_meta() {
        let baseline = run(&["readmore", "dare"]);
        for settings in [
            json!({}),
            json!({"capenergi": {}}),
            json!({"capenergi": null}),
            json!({"capenergi": "bukan-peta"}),
        ] {
            assert_eq!(
                run_with(&["readmore", "dare"], settings.clone()),
                baseline,
                "setting harus transparan saat kosong/aneh: {settings}"
            );
        }
    }

    #[test]
    fn capenergi_overrides_the_price_of_that_plugin_only() {
        let out = run_with(
            &["readmore", "dare"],
            json!({"capenergi": {"readmore": 7, "dare": 3}}),
        );
        assert_eq!(
            out,
            concat!(
                "🔋 *DETAIL ENERGI FITUR*\n\n",
                "✅ *readmore* : 7 Energi\n",
                "✅ *dare* : 3 Energi"
            ),
            "override mengalahkan `plugin.config.energi`"
        );
    }

    #[test]
    fn a_capenergi_key_can_also_make_a_free_command_cost_energy() {
        assert_eq!(
            run_with(&["dare"], json!({"capenergi": {"dare": 12}})),
            "🔋 *DETAIL ENERGI FITUR*\n\n✅ *dare* : 12 Energi"
        );
    }

    #[test]
    fn capenergi_is_keyed_by_canonical_name_not_by_alias() {
        // `kkmbg` resolve ke `kalkulatormbg`, jadi override harus pakai nama itu.
        let by_alias = run_with(&["kkmbg"], json!({"capenergi": {"kkmbg": 9}}));
        assert!(
            by_alias.ends_with("✅ *kalkulatormbg* : 1 Energi"),
            "override by alias diabaikan: {by_alias}"
        );
        let by_name = run_with(&["kkmbg"], json!({"capenergi": {"kalkulatormbg": 9}}));
        assert!(by_name.ends_with("✅ *kalkulatormbg* : 9 Energi"));
    }

    #[test]
    fn only_the_probe_branch_carries_a_reaction() {
        // JS bereaksi "🕕" lalu "✅", tapi hanya setelah guard args.length === 0.
        let help_reply = handler(&[], json!({}));
        assert!(matches!(
            help_reply.without_mutations(),
            Reply::Text { .. }
        ));

        let probed = handler(&["readmore"], json!({}));
        let mut cur = probed.without_mutations();
        let mut seen = 0;
        while let Reply::Reaction { emoji, reply } = cur {
            assert_eq!(emoji, "✅");
            seen += 1;
            cur = reply;
        }
        assert_eq!(seen, 1, "hanya reaksi terakhir yang tersimpan");
        assert!(matches!(cur, Reply::Text { .. }), "balasan tetap teks");
        assert!(probed.mutations().is_empty(), "plugin ini hanya membaca");
    }
}