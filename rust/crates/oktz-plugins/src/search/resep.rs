//! `plugins/search/resep.js` — pencarian resep makanan lewat Neoxr dengan sesi.
//!
//! `ponytail:` `getDatabase()` (`db.getUser`/`db.save`) dan `resepAnswerHandler`
//! (`answer_handler`) butuh store + pendaftaran answer handler yang tidak tersedia
//! di `Ctx`; handler pencarian untuk teks tidak kosong diakhiri dengan
//! `error_template` + `☢`. Fungsi URL dan rendering daftar/detail di-port agar
//! bisa diuji. Saat store + answer handler tersedia, tambahkan
//! `pub mod resep;` di `src/search/mod.rs` secara manual (mod.rs milik plugin
//! lain — dilarang menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const API: &str = "https://api.neoxr.eu/api";

/// Hasil satu pencarian resep (`resep.js:51-54`).
pub(crate) struct ResepHasil {
    pub name: String,
    pub url: String,
}

/// `searchUrl` (`resep.js:33`).
pub(crate) fn search_url(query: &str, apikey: &str) -> String {
    format!(
        "{API}/resep-search?q={}&apikey={}",
        crate::tools::phisingataubukan::encode_uri_component(query),
        apikey
    )
}

/// `detailUrl` (`resep.js:115`).
pub(crate) fn detail_url(resep_url: &str, apikey: &str) -> String {
    format!(
        "{API}/resep?url={}&apikey={}",
        crate::tools::phisingataubukan::encode_uri_component(resep_url),
        apikey
    )
}

/// Apakah respons pencarian valid (`resep.js:38`).
pub(crate) fn pencarian_valid(data: &Value) -> bool {
    crate::common::js_truthy(data.get("status"))
        && data
            .get("data")
            .is_some_and(|d| d.is_array() && !d.as_array().unwrap().is_empty())
}

/// `listTxt` daftar hasil (`resep.js:44-58`).
pub(crate) fn render_daftar(text: &str, hasil: &[ResepHasil]) -> String {
    let mut out = format!(
        "🍳 *HASIL PENCARIAN RESEP: {}*\n\nDitemukan beberapa resep nih, pilih salah satu ya:\n\n",
        text.to_uppercase()
    );
    for (i, h) in hasil.iter().enumerate() {
        out.push_str(&format!("*{}.* {}\n", i + 1, h.name));
    }
    out.push_str(
        "\n> 💡 *Kirim angka (contoh: 1)* untuk melihat detail resepnya, atau ketik `batal` untuk membatalkan pencarian.",
    );
    out
}

/// Teks tidak ditemukan (`resep.js:40`).
pub(crate) fn tidak_ditemukan(text: &str) -> String {
    format!("Maaf, resep untuk \"{text}\" tidak ditemukan. Coba kata kunci lain.")
}

/// `renderDetail` resep (`resep.js:125-142`).
pub(crate) fn render_detail(recipe: &Value) -> String {
    let mut out = format!(
        "👨‍🍳 *{}* 👩‍🍳\n\n⏱️ *Waktu:* {}\n🍽️ *Porsi:* {}\n\n",
        recipe
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_uppercase(),
        recipe.get("timeout").and_then(Value::as_str).unwrap_or("-"),
        recipe.get("portion").and_then(Value::as_str).unwrap_or("-")
    );
    if let Some(bahan) = recipe.get("ingredients").and_then(Value::as_array) {
        if !bahan.is_empty() {
            out.push_str("*🥬 BAHAN-BAHAN:*\n");
            for b in bahan {
                out.push_str(&format!("- {}\n", b.as_str().unwrap_or(&b.to_string())));
            }
            out.push('\n');
        }
    }
    if let Some(steps) = recipe.get("steps").and_then(Value::as_array) {
        if !steps.is_empty() {
            out.push_str("*🍳 CARA MEMBUAT:*\n");
            for (i, s) in steps.iter().enumerate() {
                out.push_str(&format!(
                    "*{}.* {}\n\n",
                    i + 1,
                    s.as_str().unwrap_or(&s.to_string())
                ));
            }
        }
    }
    out
}

/// Kartu pakai (`resep.js:25-27`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🍳 *PENCARIAN RESEP*\n\nKetik nama makanan yang mau dicari resepnya.\n*Contoh:* {prefix}resep Nasi Goreng"
    )
}

#[plugin(
    name = "resep",
    alias = ["resepmasak", "resepmasakan", "caramasak"],
    category = "search",
    description = "Cari resep makanan yang lengkap dan enak",
    usage = ".resep <nama makanan>",
    example = ".resep ayam geprek",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn resep(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    // `getDatabase()` + `db.getUser`/`db.save` (untuk `user.resep_session`)
    // tidak tersedia di Ctx, dan `resepAnswerHandler` butuh pendaftaran
    // answer_handler. Diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("resep").unwrap();
        assert_eq!(p.meta.name, "resep");
        assert_eq!(p.meta.alias, ["resepmasak", "resepmasakan", "caramasak"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn search_url_dan_detail_url() {
        assert_eq!(
            search_url("ayam geprek", "KUNCI"),
            "https://api.neoxr.eu/api/resep-search?q=ayam%20geprek&apikey=KUNCI"
        );
        assert_eq!(
            detail_url("https://x/resep/1", "KUNCI"),
            "https://api.neoxr.eu/api/resep?url=https%3A%2F%2Fx%2Fresep%2F1&apikey=KUNCI"
        );
    }

    #[test]
    fn pencarian_valid_miripkan_js() {
        assert!(!pencarian_valid(&serde_json::json!({})));
        assert!(!pencarian_valid(&serde_json::json!({ "status": true })));
        assert!(!pencarian_valid(&serde_json::json!({ "status": true, "data": null })));
        assert!(!pencarian_valid(&serde_json::json!({ "status": true, "data": [] })));
        assert!(!pencarian_valid(&serde_json::json!({ "status": false, "data": [{ "name": "x" }] })));
        assert!(pencarian_valid(&serde_json::json!({ "status": true, "data": [{ "name": "x" }] })));
    }

    #[test]
    fn render_daftar_bentuk_sama_dengan_js() {
        let hasil = [
            ResepHasil {
                name: "Ayam Geprek".into(),
                url: "https://x/1".into(),
            },
            ResepHasil {
                name: "Nasi Goreng".into(),
                url: "https://x/2".into(),
            },
        ];
        let out = render_daftar("ayam", &hasil);
        assert!(out.starts_with("🍳 *HASIL PENCARIAN RESEP: AYAM*\n\n"));
        assert!(out.contains("*1.* Ayam Geprek"));
        assert!(out.contains("*2.* Nasi Goreng"));
        assert!(out.contains("Kirim angka (contoh: 1)"));
        assert!(out.contains("batal"));
    }

    #[test]
    fn tidak_ditemukan_serupa_js() {
        assert_eq!(tidak_ditemukan("ayam"), "Maaf, resep untuk \"ayam\" tidak ditemukan. Coba kata kunci lain.");
    }

    #[test]
    fn render_detail_judul_waktu_porsi() {
        let recipe = serde_json::json!({
            "title": "Ayam Geprek",
            "timeout": "30 menit",
            "portion": "2 porsi",
            "ingredients": ["ayam", "tepung"],
            "steps": ["siapkan ayam", "baluri tepung", "goreng"],
            "thumbnail": "https://x.jpg"
        });
        let out = render_detail(&recipe);
        assert!(out.starts_with("👨‍🍳 *AYAM GEPREK* 👩‍🍳\n\n"));
        assert!(out.contains("⏱️ *Waktu:* 30 menit"));
        assert!(out.contains("🍽️ *Porsi:* 2 porsi"));
        assert!(out.contains("*🥬 BAHAN-BAHAN:*\n- ayam\n- tepung"));
        assert!(out.contains("*🍳 CARA MEMBUAT:*\n*1.* siapkan ayam\n\n*2.* baluri tepung\n\n*3.* goreng\n\n"));
    }

    #[test]
    fn render_detail_tanpa_field_gunakan_tanda_hubung() {
        let out = render_detail(&serde_json::json!({}));
        assert!(out.contains("⏱️ *Waktu:* -"));
        assert!(out.contains("🍽️ *Porsi:* -"));
    }

    #[test]
    fn tanpa_text_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resep".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = resep(ctx);
        let teks = r.without_mutations().text_body().unwrap();
        assert_eq!(teks, pakai("."));
    }

    #[test]
    fn pencarian_dengan_text_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resep".into(),
            text: "ayam geprek".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = resep(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "resep")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
