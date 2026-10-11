//! `plugins/info/howmuchenergy.js` — biaya energi tiap command dari registry.
//!
//! `ponytail:` `db.setting('capenergi')` adalah override global di store.
//! `Ctx` tidak membawanya, jadi override diabaikan dan harga yang dipakai
//! selalu `plugin.config.energi` — sama dengan apa yang terjadi di JS saat
//! override kosong.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{probe_plugin, render_probe};

pub(crate) const HELP: &str = "🔍 *PENGECEKAN ENERGI FITUR*\n\nSistem untuk mengetahui berapa besar potongan energi yang dibutuhkan untuk menggunakan satu atau banyak fitur sekaligus.\n\n*PENGGUNAAN:*\n- *.howmuchenergy <nama_fitur1> <nama_fitur2> ...*\n\n*CONTOH PENGGUNAAN:*\n- *.howmuchenergy hd jpm*\n\n*PENJELASAN:*\nMasukkan perintah beserta satu atau banyak nama fitur yang ingin kamu cek. Pisahkan dengan spasi.";

pub(crate) const HEADER: &str = "🔋 *DETAIL ENERGI FITUR*\n\n";

probe_plugin!(
    howmuchenergy,
    "howmuchenergy",
    [],
    "Mengecek penggunaan energi banyak fitur sekaligus",
    ".howmuchenergy <nama_fitur1> <nama_fitur2> ...",
    ".howmuchenergy hd jpm",
    HELP,
    HEADER,
    |name: &str| {
        let cost = oktz_plugin_api::find(name).map(|p| p.meta.energi).unwrap_or_default();
        format!("✅ *{name}* : {cost} Energi\n")
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Ctx;

    fn run(args: &[&str]) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "howmuchenergy".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("howmuchenergy").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
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
    fn it_has_no_alias_so_it_cannot_shadow_anything() {
        assert_eq!(oktz_plugin_api::find("howmuchenergy").unwrap().meta.alias.len(), 0);
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
        assert!(run(&["TIDAK-ADA"]).contains("❌ *tidak-ada*"), "argumen diturunkan");
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
        assert!(!out.ends_with('\n') && !out.ends_with(' '), "`responseText.trim()`");
    }

    #[test]
    fn the_energi_value_always_matches_the_registered_meta() {
        for name in ["readmore", "dare", "truth", "cekbaik", "soulmatch", "kalkulatormbg"] {
            let expected = oktz_plugin_api::find(name).unwrap().meta.energi;
            assert_eq!(
                run(&[name]),
                format!("🔋 *DETAIL ENERGI FITUR*\n\n✅ *{name}* : {expected} Energi"),
                "{name}"
            );
        }
    }
}