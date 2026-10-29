//! `plugins/store/editlist.js` — ubah satu field entri `storeLists`.
//!
//! `ponytail:` field `gambar` dan `video` tidak bisa dipindah — keduanya
//! butuh reply/download media lalu upload, dan `Ctx` tidak membawa media.
//! Percabangan itu dikembalikan sebagai pesan eksplisit (branch yang sama
//! seperti di JS ketika tidak ada media), bukan diam-diam menyatakan
//! berhasil. Kalau `Ctx` nanti membawa `quoted`, tinggal isi dua cabang itu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;


const NO_MEDIA: &str = "🖼️ *Reply atau kirim gambar baru.*\n\nKirim gambar lalu reply dengan command ini.";

#[plugin(
    name = "editlist",
    alias = ["editinfo"],
    category = "store",
    description = "✏️ Edit informasi toko (hanya di private chat)",
    usage = ".editlist <nomor> <field> <nilai>",
    example = ".editlist 1 isi Konten baru di sini",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = true
)]
pub fn editlist(ctx: Ctx) -> Reply {
    if ctx.is_group {
        return Reply::text("🚫 *Akses Ditolak*\n\nUntuk menjaga keamanan data 🛡️, pengeditan informasi hanya dapat dilakukan di *private chat*.\n\nSilakan chat bot secara langsung 📱");
    }

    let lists = ctx.setting_list("storeLists");
    if lists.is_empty() {
        return Reply::text(format!(
            "📭 *Belum ada informasi.*\n\nTambahkan informasi terlebih dahulu: `{}addlist` ➕",
            ctx.prefix
        ));
    }

    let text = ctx.text.trim();
    // `^(\d+)\s+(nama|isi|deskripsi|gambar|video)\s*(.*)$` (case-insensitive),
    // `editlist.js:43`.
    let Some((num, field, value)) = parse_edit_command(text) else {
        return Reply::text(usage(&ctx.prefix));
    };

    let Some(idx) = num
        .checked_sub(1)
        .filter(|i| *i >= 0 && (*i as usize) < lists.len())
        .map(|i| i as usize)
    else {
        return Reply::text(format!(
            "❌ *Nomor tidak valid.*\n\nRentang: 1-{} 📋",
            lists.len()
        ));
    };

    let mut item = lists[idx].clone();
    let obj = item.as_object_mut().expect("entri store objek");

    match field {
        "nama" => {
            if value.chars().count() < 2 {
                return Reply::text("❌ *Nama terlalu pendek.* Minimal 2 karakter 🏷️");
            }
            obj.insert("name".into(), Value::from(value));
        }
        "isi" => {
            if value.chars().count() < 3 {
                return Reply::text("❌ *Isi terlalu pendek.* Minimal 3 karakter 📝");
            }
            let content = value.replace(";;", "\n");
            let preview = content.chars().take(80).collect::<String>().replace('\n', " ");
            obj.insert("content".into(), Value::from(content));
            // `editlist.js:100-101`: `description` ikut dihitung ulang.
            obj.insert("description".into(), Value::from(preview));
        }
        "deskripsi" => {
            obj.insert("description".into(), Value::from(value.replace(";;", " ")));
        }
        // Dua cabang ini di JS butuh media; lihat `ponytail:` modul.
        "gambar" => return Reply::text(NO_MEDIA),
        "video" => {
            return Reply::text(
                "🎬 *Reply atau kirim video baru.*\n\nKirim video lalu reply dengan command ini.",
            );
        }
        _ => unreachable!("pola sudah membatasi lima field"),
    }

    let mut lists: Vec<Value> = lists.to_vec();
    let name = item["name"].as_str().unwrap_or("undefined").to_string();
    lists[idx] = item;

    let mut reply = String::from("✅ *INFORMASI DIPERBARUI*\n\n");
    reply.push_str(&format!("🏷️ Nama: *{name}*\n"));
    if field == "isi" {
        reply.push_str(&format!("📝 Isi:\n{}\n\n", lists[idx]["content"].as_str().unwrap_or("")));
    }
    reply.push_str(&format!("\n👀 _Lihat perubahan: `{}list`_", ctx.prefix));

    Reply::text(reply).with_mutation(Mutation::SetSetting {
        key: "storeLists".into(),
        value: Value::Array(lists),
    })
}

/// `(\d+)\s+(nama|isi|deskripsi|gambar|video)\s*(.*)` — `editlist.js:43`.
/// Field dikembalikan lowercase karena JS memakai flag `i`.
fn parse_edit_command(text: &str) -> Option<(i64, &'static str, String)> {
    let t = text.trim_start();
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = t[digits.len()..].trim_start();
    let lower = rest.to_ascii_lowercase();
    for field in ["nama", "isi", "deskripsi", "gambar", "video"] {
        if lower.strip_prefix(field).is_some() {
            // `\s*` — nol spasi pun sah (`editlist.js:43`).
            return Some((
                digits.parse().ok()?,
                field,
                // Sisanya diambil dari `rest` (bukan `lower`) supaya kapitalisasi
                // nilai asli tidak hilang.
                rest[field.len()..].trim().to_string(),
            ));
        }
    }
    None
}

fn usage(prefix: &str) -> String {
    format!(
        "✏️ *EDIT INFORMASI TOKO*\n\n📋 Format: `{prefix}editlist <nomor> <field> <nilai>`\n\n\
         📌 *Field yang bisa diedit:*\n\
         • *nama* 🏷️ — Judul informasi\n\
         • *isi* 📝 — Konten informasi (gunakan `;;` untuk baris baru)\n\
         • *deskripsi* 📋 — Deskripsi singkat (preview di daftar)\n\
         • *gambar* 🖼️ — Upload gambar baru (reply gambar)\n\
         • *video* 🎬 — Upload video baru (reply video)\n\n\
         📝 *Contoh:*\n\
         `{prefix}editlist 1 isi Syarat baru: blablabla;;Ketentuan: blablabla`\n\
         `{prefix}editlist 1 nama FAQ Pembayaran`\n\n\
         _Gunakan `;;` untuk baris baru dalam isi_ ✍️"
    )
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run(settings: Value, text: &str, is_group: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "editlist".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_group,
            text: text.into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("editlist").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "storeLists");
                value.as_array().unwrap().clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("editlist").unwrap();
        assert_eq!(p.meta.name, "editlist");
        assert_eq!(p.meta.alias, ["editinfo"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(
            p.meta.description,
            "✏️ Edit informasi toko (hanya di private chat)"
        );
        assert_eq!(p.meta.usage, ".editlist <nomor> <field> <nilai>");
        assert_eq!(p.meta.example, ".editlist 1 isi Konten baru di sini");
        assert_eq!(p.meta.cooldown, 3);
        assert!(p.meta.is_owner && p.meta.is_private && !p.meta.is_group);
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(oktz_plugin_api::find("editinfo").unwrap().meta.name, "editlist");
    }

    #[test]
    fn the_group_branch_is_rejected_and_writes_nothing() {
        let r = run(serde_json::json!({"storeLists": [{"name": "A"}]}), "1 nama B", true);
        assert!(text_of(&r).starts_with("🚫 *Akses Ditolak*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_empty_store_short_circuits_before_parsing() {
        let r = run(serde_json::json!({}), "1 nama B", false);
        assert!(text_of(&r).starts_with("📭 *Belum ada informasi.*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn text_that_does_not_match_the_pattern_shows_usage() {
        for bad in ["bukan perintah", "nama erah"] {
            let r = run(serde_json::json!({"storeLists": [{"name": "A"}]}), bad, false);
            assert!(
                text_of(&r).starts_with("✏️ *EDIT INFORMASI TOKO*"),
                "input {bad:?} harus menampilkan usage"
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn an_out_of_range_number_is_rejected_before_writing() {
        let r = run(serde_json::json!({"storeLists": [{"name": "A"}]}), "5 nama B", false);
        assert!(text_of(&r).contains("Nomor tidak valid."));
        assert!(text_of(&r).contains("Rentang: 1-1"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn editing_the_name_lands_in_the_store() {
        let r = run(
            serde_json::json!({"storeLists": [{"name": "Lama", "content": "isi"}]}),
            "1 nama FAQ Pembayaran",
            false,
        );
        let after = stored(&r);
        assert_eq!(after[0]["name"].as_str(), Some("FAQ Pembayaran"));
        assert_eq!(after[0]["content"].as_str(), Some("isi"), "field lain utuh");
        assert!(text_of(&r).contains("✅ *INFORMASI DIPERBARUI*"));
    }

    // `editlist.js:100`: mengubah `isi` harus MENGHITUNG ULANG `description`.
    #[test]
    fn editing_content_also_recomputes_the_description_preview() {
        let r = run(
            serde_json::json!({"storeLists": [{"name": "A", "content": "lama", "description": "lama"}]}),
            "1 isi baru;;baris dua",
            false,
        );
        let after = stored(&r);
        assert_eq!(after[0]["content"].as_str(), Some("baru\nbaris dua"));
        assert_eq!(
            after[0]["description"].as_str(),
            Some("baru baris dua"),
            "preview harus datar (\\n -> spasi)"
        );
        assert!(text_of(&r).contains("📝 Isi:\nbaru\nbaris dua"));
    }

    #[test]
    fn editing_the_description_does_not_touch_content() {
        let r = run(
            serde_json::json!({"storeLists": [{"name": "A", "content": "isi asli"}]}),
            "1 deskripsi preview;;baru",
            false,
        );
        let after = stored(&r);
        assert_eq!(after[0]["description"].as_str(), Some("preview baru"));
        assert_eq!(after[0]["content"].as_str(), Some("isi asli"));
    }

    #[test]
    fn too_short_name_and_content_are_rejected_without_writing() {
        let settings = serde_json::json!({"storeLists": [{"name": "A", "content": "isi"}]});
        let short_name = run(settings.clone(), "1 nama B", false);
        assert!(text_of(&short_name).contains("Nama terlalu pendek."));
        assert!(short_name.mutations().is_empty());

        let short_body = run(settings.clone(), "1 isi x", false);
        assert!(text_of(&short_body).contains("Isi terlalu pendek."));
        assert!(short_body.mutations().is_empty());
    }

    #[test]
    fn media_fields_report_that_media_is_needed_and_write_nothing() {
        let settings = serde_json::json!({"storeLists": [{"name": "A", "image": "u"}]});
        let img = run(settings.clone(), "1 gambar", false);
        assert!(text_of(&img).contains("🖼️ *Reply atau kirim gambar baru.*"));
        assert!(img.mutations().is_empty(), "tidak boleh menghapus media lama");

        let vid = run(settings.clone(), "1 video", false);
        assert!(text_of(&vid).contains("🎬 *Reply atau kirim video baru.*"));
        assert!(vid.mutations().is_empty());
    }

    #[test]
    fn field_matching_is_case_insensitive_like_the_js_regex() {
        let (_, field, value) = parse_edit_command("1 NAMA Baru").unwrap();
        assert_eq!(field, "nama");
        assert_eq!(value, "Baru", "nilai harus mempertahankan kapitalisasi");
        assert_eq!(parse_edit_command("2  Isi   halo").unwrap().1, "isi");
        assert_eq!(parse_edit_command("3isi halo").unwrap().1, "isi", "\\s* nol spasi");
    }
}