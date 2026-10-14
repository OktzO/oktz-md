//! `plugins/store/addlist.js` — tambah entri ke `db.setting('storeLists')`.
//!
//! `ponytail:` cabang upload media (`m.quoted?.isMedia` / `m.isMedia` +
//! `uploadImage`) TIDAK dipindah — `Ctx` tidak membawa media terkirim dan
//! tidak ada API upload. Tanpa media, `imageUrl`/`videoUrl` tetap `null`
//! dan plugin tetap menyimpan produk/informasi TEKS dengan benar; yang hilang
//! hanya thumbnail. Reply yang menyebut media pun tidak muncul karena
//! keduanya `null`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const DENIED: &str = "🚫 *Akses Ditolak*\n\nUntuk menjaga keamanan data 🛡️, penambahan informasi hanya dapat dilakukan di *private chat*.\n\nSilakan chat bot secara langsung 📱, lalu ketik:\n`{PREFIX}addlist <nama>|<isi>`";

fn denied(prefix: &str) -> String {
    DENIED.replace("{PREFIX}", prefix)
}

fn usage(prefix: &str) -> String {
    format!(
        "➕ *TAMBAH INFORMASI TOKO*\n\n\
         📋 Format:\n`{prefix}addlist <nama>|<isi>`\n\n\
         📌 *Parameter:*\n\
         • *nama* — Judul informasi (min. 2 karakter)\n\
         • *isi* — Konten informasi (gunakan `;;` untuk baris baru)\n\n\
         📝 *Contoh:*\n\
         `{prefix}addlist Syarat & Ketentuan|1. Pembelian tidak bisa dibatalkan;;2. Garansi 7 hari;;3. Hubungi admin untuk klaim`\n\
         `{prefix}addlist Cara Order|1. Ketik .listproduk;;2. Pilih produk;;3. Ketik .beli <nomor>`\n\n\
         🖼️ *Tips:*\n\
         • Gunakan `;;` untuk membuat baris baru dalam isi informasi ✍️\n\
         • Informasi ini bisa dilihat semua orang melalui `{prefix}list` 👥"
    )
}

#[plugin(
    name = "addlist",
    alias = ["addinfo"],
    category = "store",
    description = "➕ Tambah informasi toko baru (hanya di private chat)",
    usage = ".addlist <nama>|<isi>",
    example = ".addlist Syarat & Ketentuan|1. Pembelian tidak bisa dibatalkan;;2. Garansi 7 hari",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = true
)]
pub fn addlist(ctx: Ctx) -> Reply {
    // `m.isGroup` diperiksa TERLEBIH DAHULU, sebelum store dibaca
    // (`addlist.js:24`) — jadi command di grup tidak pernah menyentuh data.
    if ctx.is_group {
        return Reply::text(denied(&ctx.prefix));
    }

    let text = ctx.text.trim();
    let Some(pipe) = text.find('|') else {
        return Reply::text(usage(&ctx.prefix));
    };

    let name = text[..pipe].trim();
    // `;;` → baris baru (`addlist.js:50`).
    let content = text[pipe + 1..].trim().replace(";;", "\n");

    if name.chars().count() < 2 {
        return Reply::text("❌ *Nama terlalu pendek.*\n\nMinimal 2 karakter diperlukan agar mudah dikenali 📝");
    }
    if content.chars().count() < 3 {
        return Reply::text("❌ *Isi informasi terlalu pendek.*\n\nMinimal 3 karakter diperlukan ✍️");
    }

    let mut lists: Vec<serde_json::Value> = ctx.setting_list("storeLists").to_vec();
    let item = serde_json::json!({
        "name": name,
        "content": content,
        // `content.substring(0, 80)` dihitung dari character code unit JS;
        // untuk teks ASCII sama dengan `chars().take(80)`.
        "description": content.chars().take(80).collect::<String>().replace('\n', " "),
        // `image`/`video` tetap null — lihat catatan `ponytail:` di modul.
        "image": serde_json::Value::Null,
        "video": serde_json::Value::Null,
    });
    lists.push(item);
    let n = lists.len();

    let mut reply = String::from("✅ *INFORMASI DITAMBAHKAN*\n\n");
    reply.push_str(&format!("🏷️ Nama: *{name}*\n"));
    reply.push_str("🖼️ Media: *tidak ada*\n");
    reply.push_str(&format!("📝 Isi:\n{content}\n\n"));
    reply.push_str(&format!("📋 _Lihat daftar: `{}list`_\n", ctx.prefix));
    reply.push_str(&format!("✏️ _Edit: `{}editlist {n}`_", ctx.prefix));

    Reply::text(reply).with_mutation(Mutation::SetSetting {
        key: "storeLists".into(),
        value: serde_json::Value::Array(lists),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: serde_json::Value, text: &str, is_group: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addlist".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_group,
            text: text.into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("addlist").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply, key: &str) -> serde_json::Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetSetting { key: k, value } => {
                assert_eq!(k, key);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addlist").unwrap();
        assert_eq!(p.meta.name, "addlist");
        assert_eq!(p.meta.alias, ["addinfo"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(
            p.meta.description,
            "➕ Tambah informasi toko baru (hanya di private chat)"
        );
        assert_eq!(p.meta.usage, ".addlist <nama>|<isi>");
        assert_eq!(
            p.meta.example,
            ".addlist Syarat & Ketentuan|1. Pembelian tidak bisa dibatalkan;;2. Garansi 7 hari"
        );
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner, "isOwner: true");
        assert!(p.meta.is_private && !p.meta.is_group);
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(oktz_plugin_api::find("addinfo").unwrap().meta.name, "addlist");
    }

    // --- cabang yang tidak menulis apa pun ------------------------------

    #[test]
    fn the_group_branch_is_rejected_and_writes_nothing() {
        let r = run(json!({}), "S&K|isi", true);
        assert!(text_of(&r).starts_with("🚫 *Akses Ditolak*"));
        assert!(r.mutations().is_empty(), "command di grup tidak boleh menulis");
    }

    #[test]
    fn a_missing_pipe_shows_usage_and_writes_nothing() {
        let r = run(json!({}), "cuma nama", false);
        assert!(text_of(&r).starts_with("➕ *TAMBAH INFORMASI TOKO*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_too_short_name_or_body_is_rejected_before_touching_the_store() {
        let short_name = run(json!({"storeLists": [{"name": "lama"}]}), "a|isi panjang", false);
        assert!(text_of(&short_name).contains("Nama terlalu pendek."));
        assert!(short_name.mutations().is_empty());

        let short_body = run(json!({"storeLists": [{"name": "lama"}]}), "Nama|ab", false);
        assert!(text_of(&short_body).contains("Isi informasi terlalu pendek."));
        assert!(short_body.mutations().is_empty());
    }

    // --- cabang yang menulis -------------------------------------------

    #[test]
    fn a_new_entry_is_appended_and_the_whole_list_is_written_back() {
        let r = run(json!({"storeLists": [{"name": "lama", "content": "x"}]}), "S&K|baris1;;baris2", false);
        let stored = setting_of(&r, "storeLists");
        assert_eq!(stored.as_array().unwrap().len(), 2, "entri lama harus utuh");
        let added = &stored.as_array().unwrap()[1];
        assert_eq!(added["name"].as_str(), Some("S&K"));
        assert_eq!(
            added["content"].as_str(),
            Some("baris1\nbaris2"),
            "`;;` harus jadi baris baru"
        );
        assert_eq!(added["image"], serde_json::Value::Null);
        assert!(text_of(&r).contains("✅ *INFORMASI DITAMBAHKAN*"));
        assert!(text_of(&r).contains("🖼️ Media: *tidak ada*"));
    }

    // Inilah yang membuat `store/addlist` tadinya terblokir: tanpa
    // kemampuan menulis, reply "ditambahkan" hanya akan berbohong.
    #[test]
    fn the_reply_claiming_success_is_paired_with_the_write_that_makes_it_true() {
        let r = run(json!({}), "S&K|isi memanjang", false);
        assert!(text_of(&r).contains("✅ *INFORMASI DITAMBAHKAN*"));
        assert_eq!(
            r.mutations().len(),
            1,
            "pesan 'ditambahkan' tanpa mutasi = klaim palsu"
        );
    }

    #[test]
    fn an_absent_setting_starts_from_an_empty_list() {
        let r = run(json!({}), "S&K|isi memanjang", false);
        let stored = setting_of(&r, "storeLists");
        assert_eq!(stored.as_array().unwrap().len(), 1);
        assert!(text_of(&r).contains("✏️ _Edit: `.editlist 1`_"));
    }

    #[test]
    fn the_description_preview_flattens_newlines_and_caps_length() {
        let long = "x".repeat(100);
        let r = run(json!({}), &format!("Nama|{long}"), false);
        let stored = setting_of(&r, "storeLists");
        let desc = stored[0]["description"].as_str().unwrap();
        assert_eq!(desc.chars().count(), 80, "preview dibatasi 80 karakter");
    }
}