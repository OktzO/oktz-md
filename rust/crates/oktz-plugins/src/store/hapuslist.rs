//! `plugins/store/hapuslist.js` — hapus entri dari `storeLists`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::store::listproduk::index_arg;

const EMPTY: &str = "📭 *Belum ada informasi.*\n\nTambahkan informasi terlebih dahulu: `{PREFIX}addlist` ➕";

#[plugin(
    name = "hapuslist",
    alias = ["dellist", "deletelist"],
    category = "store",
    description = "🗑️ Hapus informasi toko",
    usage = ".hapuslist <nomor>",
    example = ".hapuslist 1",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn hapuslist(ctx: Ctx) -> Reply {
    let lists = ctx.setting_list("storeLists");
    if lists.is_empty() {
        return Reply::text(EMPTY.replace("{PREFIX}", &ctx.prefix));
    }

    // Tanpa nomor valid → tampilkan daftar pilihan (`hapuslist.js:36-44`).
    let Some(idx) = index_arg(&ctx, lists.len()) else {
        let mut txt =
            String::from("🗑️ *Pilih Informasi yang Dihapus*\n\nKetik `{PREFIX}hapuslist <nomor>`\n\n");
        for (i, l) in lists.iter().enumerate() {
            let icon = match (l.get("image").is_some_and(|v| !v.is_null()), l.get("video").is_some_and(|v| !v.is_null())) {
                (true, _) => "🖼️",
                (_, true) => "🎬",
                _ => "📝",
            };
            txt.push_str(&format!(
                "{icon} *{}.* {}\n",
                i + 1,
                l.get("name").and_then(Value::as_str).unwrap_or("undefined")
            ));
        }
        return Reply::text(txt.replace("{PREFIX}", &ctx.prefix));
    };

    // `lists.splice(idx, 1)[0]` (`hapuslist.js:47`) — seluruh entri dihapus,
    // bukan hanya field-nya.
    let mut lists: Vec<Value> = lists.to_vec();
    let deleted = lists.remove(idx);
    let name = deleted
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("undefined")
        .to_string();

    // JS: `await m.react('✅')` lalu `m.reply(...)` (hapuslist.js:52-58).
    Reply::text(format!(
        "🗑️ *INFORMASI DIHAPUS*\n\n🏷️ Nama: *{name}*\n\n⚠️ _Informasi telah dihapus secara permanen dan tidak dapat dikembalikan._"
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetSetting {
        key: "storeLists".into(),
        value: Value::Array(lists),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: serde_json::Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hapuslist".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("hapuslist").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
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
        let p = oktz_plugin_api::find("hapuslist").unwrap();
        assert_eq!(p.meta.name, "hapuslist");
        assert_eq!(p.meta.alias, ["dellist", "deletelist"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "🗑️ Hapus informasi toko");
        assert_eq!(p.meta.usage, ".hapuslist <nomor>");
        assert_eq!(p.meta.example, ".hapuslist 1");
        assert_eq!(p.meta.cooldown, 3);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["dellist", "deletelist"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "hapuslist");
        }
    }

    #[test]
    fn an_empty_store_reports_nothing_to_delete_and_writes_nothing() {
        let r = run(json!({}), "1");
        assert!(text_of(&r).starts_with("📭 *Belum ada informasi.*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_bad_number_shows_the_picker_instead_of_deleting() {
        let r = run(
            json!({"storeLists": [{"name": "A"}, {"name": "B", "image": "u"}, {"name": "C", "video": "v"}]}),
            "abc",
        );
        assert!(text_of(&r).contains("*Pilih Informasi yang Dihapus*"));
        assert_eq!(
            text_of(&r).matches("*1.*").count(),
            1,
            "daftar harus tampil"
        );
        assert!(r.mutations().is_empty(), "tidak boleh menghapus apa pun");
    }

    #[test]
    fn the_picker_uses_the_same_media_icons_as_list() {
        let r = run(
            json!({"storeLists": [{"name": "A"}, {"name": "B", "image": "u"}]}),
            "9",
        );
        assert!(text_of(&r).contains("📝 *1.* A"));
        assert!(text_of(&r).contains("🖼️ *2.* B"));
    }

    // Mutasi harus benar-benar mengubah isi daftar, bukan sekadar log.
    #[test]
    fn deleting_an_entry_removes_exactly_that_index() {
        let r = run(
            json!({"storeLists": [{"name": "A"}, {"name": "B"}, {"name": "C"}]}),
            "2",
        );
        let after = stored(&r);
        assert_eq!(after.len(), 2);
        assert_eq!(after[0]["name"].as_str(), Some("A"));
        assert_eq!(after[1]["name"].as_str(), Some("C"), "index 1 (nomor 2) dihapus");
        assert!(text_of(&r).contains("Nama: *B*"));
        // JS: `await m.react('✅')` — reaksi, bukan teks (hapuslist.js:56).
        // Assert emoji di `Reply::Reaction`, bukan di teks balasan.
        assert_eq!(reaction_of(&r), Some("✅"), "reaksi ✅ harus ada");
    }

    #[test]
    fn deleting_the_only_entry_writes_an_empty_list_not_a_missing_key() {
        let r = run(json!({"storeLists": [{"name": "A"}]}), "1");
        let after = stored(&r);
        assert!(after.is_empty(), "harus array kosong, bukan key hilang");
    }
}