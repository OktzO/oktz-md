//! `plugins/main/rules.js` — peraturan bot, dari setting atau bawaan.
//!
//! `ponytail:` `getAssetBuffer("rules")` membaca gambar dari disk lalu
//! mengirimnya sebagai `image` + caption. Plugin tidak punya `fs`, dan
//! `MediaSource::Path` butuh path yang sudah diketahui router — sedangkan
//! `getAssetBuffer` tidak pernah mengembalikan path, hanya buffer. Jadi
//! gambar tidak bisa dikirim dan balasan selalu teks.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// `config.bot?.name || "Bot"` — `Ctx` tidak membawa config.
pub(crate) const BOT_NAME: &str = "Oktz Bot";

/// `buildDefaultRules` (`rules.js:21`) dengan `botName` dan `prefix` diisi.
pub(crate) fn build_default_rules(prefix: &str) -> String {
    let bot = BOT_NAME.to_uppercase();
    format!(
        "📜 *PERATURAN {bot}*\n\n\
         Hai! Sebelum kamu menggunakan semua fitur yang tersedia, pastikan kamu memahami dan mematuhi peraturan berikut ini ya. Peraturan ini dibuat supaya semua pengguna bisa nyaman dan bot bisa berjalan dengan lancar.\n\n\
         🔹 *ATURAN UMUM*\n\n\
         - Dilarang keras melakukan *spam command* secara berulang-ulang dalam waktu singkat. Sistem anti-spam aktif dan pelanggar bisa langsung dibanned otomatis oleh bot.\n\
         - Gunakan semua fitur bot dengan *bijak dan bertanggung jawab*. Jangan menyalahgunakan fitur untuk hal-hal yang merugikan orang lain.\n\
         - Dilarang menggunakan bot untuk menyebarkan *konten SARA, ujaran kebencian, atau konten ilegal* dalam bentuk apapun.\n\
         - *Hormati sesama pengguna* bot. Jangan menggunakan fitur untuk mengganggu, membully, atau merugikan pengguna lain.\n\
         - Jangan mengirimkan *konten NSFW* di grup tanpa persetujuan admin grup terlebih dahulu.\n\n\
         🔹 *ATURAN PENGGUNAAN FITUR*\n\n\
         - Setiap penggunaan command membutuhkan *energi*. Gunakan energi kamu dengan bijak supaya tidak cepat habis.\n\
         - Fitur *premium* hanya bisa diakses oleh pengguna yang sudah berlangganan. Ketik *{prefix}benefitpremium* untuk info lengkap.\n\
         - Jika menemukan *bug atau error*, segera laporkan ke owner bot melalui *{prefix}owner*. Jangan disalahgunakan.\n\
         - Dilarang melakukan *request fitur yang tidak masuk akal* atau memaksa owner untuk menambahkan fitur tertentu.\n\n\
         🔹 *KONSEKUENSI PELANGGARAN*\n\n\
         - Pelanggaran ringan akan mendapat *peringatan* dari admin atau owner\n\
         - Pelanggaran berat atau berulang dapat mengakibatkan *banned permanen* dari penggunaan bot\n\
         - Owner berhak memutuskan sanksi tanpa pemberitahuan terlebih dahulu\n\n\
         _Dengan menggunakan bot ini, kamu dianggap sudah membaca dan menyetujui seluruh peraturan di atas._"
    )
}

/// Tiga cabang `rules.js:55-65`: string non-kosong → dipakai utuh; array
/// non-kosong → jadi daftar bernomor; selain itu → teks bawaan.
pub(crate) fn render(custom: Option<&serde_json::Value>, prefix: &str) -> String {
    match custom {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.clone(),
        Some(serde_json::Value::Array(items)) if !items.is_empty() => {
            let mut out = format!("📜 *PERATURAN {}*\n\n", BOT_NAME.to_uppercase());
            for (i, rule) in joined_strings(items).iter().enumerate() {
                out.push_str(&format!("{}. {rule}\n", i + 1));
            }
            out
        }
        _ => build_default_rules(prefix),
    }
}

#[plugin(
    name = "rules",
    alias = ["aturanbot", "botrules", "peraturanbot"],
    category = "main",
    description = "Menampilkan rules dan aturan penggunaan bot secara lengkap",
    usage = ".rules",
    example = ".rules",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn rules(ctx: Ctx) -> Reply {
    Reply::text(render(ctx.setting("botRules"), &ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(setting: Option<serde_json::Value>, prefix: &str) -> String {
        let settings = match setting {
            Some(v) => serde_json::json!({ "botRules": v }),
            None => serde_json::json!({}),
        };
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "rules".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("rules").unwrap().handler)(ctx);
        r.text_body().expect("harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("rules").unwrap();
        assert_eq!(p.meta.name, "rules");
        assert_eq!(p.meta.alias, ["aturanbot", "botrules", "peraturanbot"]);
        assert_eq!(p.meta.category, "main");
        assert_eq!(
            p.meta.description,
            "Menampilkan rules dan aturan penggunaan bot secara lengkap"
        );
        assert_eq!(p.meta.usage, ".rules");
        assert_eq!(p.meta.example, ".rules");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["rules", "aturanbot", "botrules", "peraturanbot"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("rules"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn a_custom_string_is_used_verbatim() {
        assert_eq!(
            run(Some(serde_json::json!("Aturan kustom dari owner")), "."),
            "Aturan kustom dari owner",
            "custom string tidak boleh di-render ulang"
        );
    }

    #[test]
    fn a_custom_array_becomes_a_numbered_list() {
        let out = run(Some(serde_json::json!(["Satu", "Dua", 7])), ".");
        assert_eq!(out, "📜 *PERATURAN OKTZ BOT*\n\n1. Satu\n2. Dua\n3. 7\n");
    }

    #[test]
    fn the_fallback_branch_renders_the_built_in_rules_with_the_live_prefix() {
        for setting in [
            None,
            Some(serde_json::json!({})),
            Some(serde_json::json!("")),
            Some(serde_json::json!("   ")),
            Some(serde_json::json!([])),
            Some(serde_json::json!(7)),
        ] {
            let out = run(setting.clone(), ".");
            assert!(
                out.starts_with("📜 *PERATURAN OKTZ BOT*\n\nHai!"),
                "{setting:?} -> {out}"
            );
            assert!(out.contains(".benefitpremium"), "{setting:?} -> {out}");
            assert!(out.contains(".owner"), "{setting:?} -> {out}");
            assert!(
                out.ends_with("_Dengan menggunakan bot ini, kamu dianggap sudah membaca dan menyetujui seluruh peraturan di atas._"),
                "{setting:?} -> {out}"
            );
        }
    }

    #[test]
    fn the_default_rules_quote_the_live_prefix() {
        assert!(run(None.clone(), "!").contains("!benefitpremium"));
        assert!(run(None.clone(), "#").contains("#owner"));
    }

    #[test]
    fn the_built_in_rules_keep_all_three_sections() {
        let out = run(None.clone(), ".");
        for section in [
            "🔹 *ATURAN UMUM*",
            "🔹 *ATURAN PENGGUNAAN FITUR*",
            "🔹 *KONSEKUENSI PELANGGARAN*",
        ] {
            assert!(out.contains(section), "bagian hilang: {section}");
        }
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = Ctx {
            settings: serde_json::json!({"botRules": ["a"]}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("rules").unwrap().handler)(ctx);
        assert!(r.mutations().is_empty());
    }
}
