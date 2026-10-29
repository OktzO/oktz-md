//! `plugins/info/fiturpremium.js` — daftar command `isPremium` di registry.
//!
//! `ponytail:` `db.setting('capprem')` adalah override global di store dan
//! `Ctx` tidak membawanya, jadi syaratnya selalu `PluginMeta::is_premium`.

use oktz_plugin_api::{Ctx, Reply, all_plugins, plugin};

const EMPTY: &str =
    "📝 *DAFTAR FITUR PREMIUM*\n\nSaat ini belum ada fitur yang terdaftar sebagai fitur premium eksklusif.";

/// Nama-nama command premium apa adanya (urut registry) — `render` yang
/// mengurutkannya, seperti `premiumFeatures.sort()` di JS.
pub(crate) fn premium_lines() -> Vec<String> {
    all_plugins()
        .iter()
        .filter(|p| p.meta.is_premium)
        .map(|p| p.meta.name.clone())
        .collect()
}

pub(crate) fn render(names: &[String]) -> String {
    if names.is_empty() {
        return EMPTY.to_string();
    }
    // `premiumFeatures.sort()` terjadi sebelum join, jadi render juga mengurutkan
    // — kalau tidak, urutan registry (yang acak) bocor ke balasan.
    let mut sorted = names.to_vec();
    sorted.sort();
    let list = sorted
        .iter()
        .map(|n| format!("- {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "💎 *DAFTAR FITUR PREMIUM*\n\nBerikut adalah seluruh daftar fitur eksklusif yang hanya bisa diakses oleh member berstatus Premium:\n\n{list}\n\n_Untuk berlangganan premium, silakan hubungi owner._"
    )
}

#[plugin(
    name = "fiturpremium",
    alias = ["listpremium", "fiturprem"],
    category = "info",
    description = "Melihat daftar seluruh fitur premium bot",
    usage = ".fiturpremium",
    example = ".fiturpremium",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn fiturpremium(_ctx: Ctx) -> Reply {
    Reply::text(render(&premium_lines()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> String {
        (oktz_plugin_api::find("fiturpremium").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "fiturpremium".into(),
            sender: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
        .text_body()
        .expect("harus ada teks")
        .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("fiturpremium").unwrap();
        assert_eq!(p.meta.name, "fiturpremium");
        assert_eq!(p.meta.alias, ["listpremium", "fiturprem"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Melihat daftar seluruh fitur premium bot");
        assert_eq!(p.meta.usage, ".fiturpremium");
        assert_eq!(p.meta.example, ".fiturpremium");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["listpremium", "fiturprem"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "fiturpremium");
        }
    }

    #[test]
    fn the_empty_branch_is_the_one_the_current_registry_shows() {
        // Cabang yang dipakai ditentukan isi registry, bukan tebakan: kalau
        // ada plugin `is_premium`, balasan WAJIB branche terisi.
        let names = premium_lines();
        let out = run();
        if names.is_empty() {
            assert_eq!(out, EMPTY);
            assert!(EMPTY.starts_with("📝 *DAFTAR FITUR PREMIUM*\n\n"));
            assert!(!EMPTY.contains("💎"), "ikon berbeda antara cabang kosong dan terisi");
        } else {
            assert_eq!(out, render(&names));
            assert!(out.starts_with("💎 *DAFTAR FITUR PREMIUM*"), "{out}");
            for n in &names {
                assert!(out.contains(&format!("- {n}\n")), "nama {n} hilang: {out}");
            }
        }
    }

    #[test]
    fn the_filled_branch_lists_names_sorted_with_a_dash_prefix() {
        let out = render(&["zeta".into(), "alpha".into(), "mid".into()]);
        assert_eq!(
            out,
            concat!(
                "💎 *DAFTAR FITUR PREMIUM*\n\n",
                "Berikut adalah seluruh daftar fitur eksklusif yang hanya bisa diakses oleh member berstatus Premium:\n\n",
                "- alpha\n- mid\n- zeta\n\n",
                "_Untuk berlangganan premium, silakan hubungi owner._"
            )
        );
        assert_eq!(out.matches("\n- ").count(), 3);
    }

    #[test]
    fn a_single_premium_feature_still_uses_the_filled_branch() {
        let out = render(&["satu".into()]);
        assert!(out.starts_with("💎 *DAFTAR FITUR PREMIUM*"));
        assert!(out.contains("- satu"));
    }

    #[test]
    fn premium_lines_agree_with_the_registry_filter() {
        let expected: Vec<String> = all_plugins()
            .iter()
            .filter(|p| p.meta.is_premium)
            .map(|p| p.meta.name.clone())
            .collect();
        assert_eq!(premium_lines().len(), expected.len());
        assert_eq!(run() == EMPTY, expected.is_empty());
    }
}