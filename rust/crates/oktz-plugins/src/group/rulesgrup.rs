//! `plugins/group/rulesgrup.js` — `groups[chat].groupRules` + teks default.
//!
//! `ponytail:` JS mengirim gambar asset `rules` + caption kalau
//! `getAssetBuffer('rules')` ada, dan teks polos kalau tidak. `Ctx` tidak punya
//! jalur asset, jadi port ini selalu cabang teks. Naik ke
//! `Reply::Image` + `MediaSource::path` begitu router punya asset store.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const DEFAULT_GROUP_RULES: &str = "📜 *ATURAN GRUP*\n\nSelamat datang di grup ini! Supaya suasana tetap nyaman dan kondusif untuk semua anggota, harap patuhi aturan-aturan berikut ini ya.\n\n🔹 *ETIKA BERKOMUNIKASI*\n\n- Gunakan *bahasa yang sopan* dan saling menghargai satu sama lain. Tidak ada toleransi untuk kata-kata kasar atau ujaran kebencian.\n- Dilarang melakukan *spam* pesan berulang-ulang, termasuk spam sticker, gambar, dan voice note secara berlebihan.\n- Dilarang menyebarkan *konten SARA, pornografi, dan kekerasan* dalam bentuk apapun.\n- Jangan *menghina, membully, atau mempermalukan* anggota grup lainnya.\n\n🔹 *KONTEN DAN LINK*\n\n- Dilarang *promosi* produk, jasa, atau akun media sosial tanpa izin admin grup.\n- Dilarang *menyebarkan link* apapun tanpa persetujuan admin, termasuk link grup WhatsApp lain.\n- Dilarang menyebarkan *berita hoax* atau informasi yang belum terverifikasi kebenarannya.\n- Dilarang mengirim *dokumen atau file mencurigakan* yang berpotensi mengandung virus.\n\n🔹 *PENGGUNAAN BOT*\n\n- Gunakan command bot *secukupnya*, jangan spam command berulang-ulang.\n- *Patuhi instruksi admin* grup kapan saja. Keputusan admin adalah final.\n- Jika ada masalah, sampaikan dengan baik-baik ke admin, bukan di grup.\n\n🔹 *KONSEKUENSI*\n\n- Pelanggaran ringan mendapat *peringatan (warning)* dari admin\n- Pelanggaran berat atau berulang akan di-*kick* dari grup\n- Admin berhak mengeluarkan member kapan saja tanpa pemberitahuan\n\n_Dengan bergabung di grup ini, kamu dianggap sudah menyetujui semua aturan di atas._";

/// `groupData.groupRules || DEFAULT_GROUP_RULES`.
pub(crate) fn rules_text(ctx: &Ctx) -> String {
    match ctx.group_field("groupRules").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => DEFAULT_GROUP_RULES.to_string(),
    }
}

#[plugin(
    name = "rulesgrup",
    alias = ["grouprules", "aturangrup", "grules"],
    category = "group",
    description = "Menampilkan rules dan aturan grup secara lengkap",
    usage = ".rulesgrup",
    example = ".rulesgrup",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn rulesgrup(ctx: Ctx) -> Reply {
    Reply::text(rules_text(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: serde_json::Value) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rulesgrup".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("rulesgrup").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("rulesgrup").unwrap();
        assert_eq!(p.meta.name, "rulesgrup");
        assert_eq!(p.meta.alias, ["grouprules", "aturangrup", "grules"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Menampilkan rules dan aturan grup secara lengkap"
        );
        assert_eq!(p.meta.usage, ".rulesgrup");
        assert_eq!(p.meta.example, ".rulesgrup");
        assert_eq!(p.meta.cooldown, 10);
        assert!(p.meta.is_group && !p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["grouprules", "aturangrup", "grules"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "rulesgrup");
        }
    }

    #[test]
    fn absent_group_data_sends_the_full_default_rules() {
        for g in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"groupRules": ""}), serde_json::json!({"groupRules": 5})] {
            let out = run(g.clone());
            assert_eq!(out, DEFAULT_GROUP_RULES, "input {g}");
        }
    }

    #[test]
    fn the_default_rules_keep_all_four_sections() {
        for section in [
            "📜 *ATURAN GRUP*",
            "🔹 *ETIKA BERKOMUNIKASI*",
            "🔹 *KONTEN DAN LINK*",
            "🔹 *PENGGUNAAN BOT*",
            "🔹 *KONSEKUENSI*",
        ] {
            assert!(DEFAULT_GROUP_RULES.contains(section), "bagian hilang: {section}");
        }
        assert!(DEFAULT_GROUP_RULES.ends_with(
            "_Dengan bergabung di grup ini, kamu dianggap sudah menyetujui semua aturan di atas._"
        ));
        assert!(!DEFAULT_GROUP_RULES.contains("@"), "tidak ada placeholder");
    }

    #[test]
    fn a_custom_rule_replaces_the_default_verbatim() {
        let out = run(serde_json::json!({"groupRules": "1. Jangan spam\n2. Sopan"}));
        assert_eq!(out, "1. Jangan spam\n2. Sopan");
    }

    #[test]
    fn the_reply_is_plain_text_with_no_mentions() {
        let ctx = Ctx {
            prefix: ".".into(),
            chat: "120363@g.us".into(),
            sender: "628111@s.whatsapp.net".into(),
            group_data: serde_json::json!({"groupRules": "x"}),
            ..Default::default()
        };
        match (oktz_plugin_api::find("rulesgrup").unwrap().handler)(ctx) {
            Reply::Text { mentions, .. } => assert!(mentions.is_empty()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }
}