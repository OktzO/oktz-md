//! `plugins/info/whatrolethis.js` — syarat akses tiap command dari registry.
//!
//! `ponytail:` `db.setting('capprem')` adalah override global di store dan
//! `Ctx` tidak membawanya, jadi `isPremium` yang dibaca selalu nilai di
//! `PluginMeta` — sama seperti JS saat override kosong.

use oktz_plugin_api::{Ctx, PluginMeta, Reply, plugin};

use crate::common::{probe_plugin, render_probe};

pub(crate) const HELP: &str = "🔍 *PENGECEKAN AKSES FITUR*\n\nSistem untuk mengetahui detail informasi persyaratan akses sebuah fitur atau beberapa fitur sekaligus.\n\n*PENGGUNAAN:*\n- *.whatrolethis <nama_fitur1> <nama_fitur2> ...*\n\n*CONTOH PENGGUNAAN:*\n- *.whatrolethis hd jpm warn*\n\n*PENJELASAN:*\nMasukkan perintah beserta satu atau banyak nama fitur yang ingin kamu cek. Pisahkan dengan spasi.";

pub(crate) const HEADER: &str = "🔍 *DETAIL AKSES FITUR*\n\n";

/// Baris syarat akses, urutan `if` di `whatrolethis.js:41-52`. Kosong =
/// semua orang.
pub(crate) fn roles_of(m: &PluginMeta) -> Vec<&'static str> {
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
    if m.is_premium {
        roles.push("Premium 💎");
    }
    if roles.is_empty() {
        roles.push("Free / Semua Orang 🆓");
    }
    roles
}

/// `✅ *name*\n` + tiap role di-indentasi dua spasi.
pub(crate) fn render_one(meta: &PluginMeta) -> String {
    let list = roles_of(meta)
        .iter()
        .map(|r| format!("  - {r}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("✅ *{}*\n{list}\n\n", meta.name)
}

probe_plugin!(
    whatrolethis,
    "whatrolethis",
    ["whatrole", "cekrole", "cekakses"],
    "Cek persyaratan akses banyak fitur sekaligus",
    ".whatrolethis <nama_fitur1> <nama_fitur2> ...",
    ".whatrolethis hd jpm warn",
    HELP,
    HEADER,
    |name: &str| match oktz_plugin_api::find(name) {
        Some(p) => render_one(&p.meta),
        None => String::new(),
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Ctx;

    fn run(args: &[&str]) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "whatrolethis".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("whatrolethis").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn meta_of(name: &str) -> PluginMeta {
        oktz_plugin_api::find(name).unwrap().meta.clone()
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
        assert!(!p.meta.is_private && !p.meta.is_group);
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
        assert_eq!(roles_of(&meta_of("dare")), ["Free / Semua Orang 🆓"]);
        assert_eq!(roles_of(&PluginMeta::default()), ["Free / Semua Orang 🆓"]);
    }

    #[test]
    fn each_flag_adds_exactly_its_own_line_in_the_js_order() {
        assert_eq!(roles_of(&meta_of("tagall")), ["Admin Grup 👮", "Grup Only 👥"]);
        assert_eq!(roles_of(&meta_of("listadmin")), ["Grup Only 👥"]);
        assert_eq!(
            roles_of(&meta_of("templateplugin")),
            ["Owner Only 👑"],
            "templateplugin owner-only dan `isPrivate: false`"
        );
        let all = roles_of(&PluginMeta {
            is_owner: true,
            is_admin: true,
            is_bot_admin: true,
            is_group: true,
            is_private: true,
            is_premium: true,
            ..PluginMeta::default()
        });
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
    }

    #[test]
    fn a_group_only_command_never_reports_private_chat() {
        let r = run(&["tagall"]);
        assert!(r.contains("\n  - Admin Grup 👮"));
        assert!(r.contains("\n  - Grup Only 👥"), "baris role terakhir tidak punya newline — balasan sudah di-trim");
        assert!(!r.contains("Private Chat"), "isPrivate=false untuk tagall");
        assert!(!r.contains("Free / Semua Orang"));
    }

    #[test]
    fn rendering_indents_every_role_by_two_spaces_and_blank_lines_follow() {
        let m = meta_of("tagall");
        assert_eq!(render_one(&m), "✅ *tagall*\n  - Admin Grup 👮\n  - Grup Only 👥\n\n");
    }

    // -- loop ---------------------------------------------------------------

    #[test]
    fn an_alias_resolves_and_reports_the_canonical_name() {
        assert!(run(&["everyone"]).starts_with("🔍 *DETAIL AKSES FITUR*\n\n✅ *tagall*"));
        assert!(run(&["tagall"]) == run(&["everyone"]));
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
}
