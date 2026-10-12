//! `plugins/group/addantilink.js` — `groups[chat].antilinkList` tambah satu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.text?.toLowerCase()` — TANPA `trim()`. Jadi `"  tiktok.com  "` masuk
/// daftar dengan spasi ikut tersimpan, dan tidak akan cocok dengan
/// `listantilink` yang membandingkan persis. Menambah `trim()` di sini akan
/// "memperbaiki" perilaku yang justru perlu dicerminkan.
pub(crate) fn link(ctx: &Ctx) -> String {
    ctx.text.to_lowercase()
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔗 *ᴀᴅᴅ ᴀɴᴛɪʟɪɴᴋ*\n\n\
         > Masukkan domain/pattern link yang ingin diblokir\n\n\
         `Contoh:`\n\
         `{p}addantilink tiktok.com`\n\
         `{p}addantilink chat.whatsapp.com`\n\
         `{p}addantilink instagram.com`",
        p = prefix
    )
}

pub(crate) fn already_exists(link: &str) -> String {
    format!("⚠️ Link `{link}` sudah ada di daftar antilink!")
}

pub(crate) fn added(prefix: &str, link: &str, total: usize) -> String {
    format!(
        "✅ *ᴀɴᴛɪʟɪɴᴋ ᴅɪᴛᴀᴍʘᴀʜ*\n\n\
         > Link: `{link}`\n\
         > Total: *{total}* link\n\n\
         > Gunakan `{p}listantilink` untuk melihat daftar",
        p = prefix
    )
}

/// Daftar yang akan ditulis: salinan store + satu elemen baru.
pub(crate) fn appended(ctx: &Ctx, link: &str) -> Vec<serde_json::Value> {
    let mut list = ctx.group_list("antilinkList").to_vec();
    list.push(serde_json::Value::String(link.to_string()));
    list
}

#[plugin(
    name = "addantilink",
    alias = ["addalink", "addblocklink"],
    category = "group",
    description = "Menambah link ke daftar antilink",
    usage = ".addantilink <domain/pattern>",
    example = ".addantilink tiktok.com",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn addantilink(ctx: Ctx) -> Reply {
    let link = link(&ctx);
    if link.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let existing = ctx.group_list("antilinkList");
    if existing
        .iter()
        .any(|v| v.as_str() == Some(link.as_str()))
    {
        return Reply::text(already_exists(&link));
    }

    let list = appended(&ctx, &link);
    let n = list.len();
    Reply::text(added(&ctx.prefix, &link, n)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "antilinkList": list }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use serde_json::json;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addantilink".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("addantilink").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn list_write(r: &Reply) -> &Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data["antilinkList"].as_array().expect("harus array")
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addantilink").unwrap();
        assert_eq!(p.meta.name, "addantilink");
        assert_eq!(p.meta.alias, ["addalink", "addblocklink"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Menambah link ke daftar antilink");
        assert_eq!(p.meta.usage, ".addantilink <domain/pattern>");
        assert_eq!(p.meta.example, ".addantilink tiktok.com");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["addalink", "addblocklink"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addantilink");
        }
    }

    #[test]
    fn an_empty_link_shows_help_and_writes_nothing() {
        for g in [json!(null), json!({}), json!({"antilinkList": ["a.com"]})] {
            let r = run(g.clone(), "");
            assert!(text_of(&r).starts_with("🔗 *ᴀᴅᴅ ᴀɴᴛɪʟɪɴᴋ*"), "input {g}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_new_link_is_appended_and_the_count_reported_matches_the_write() {
        let r = run(json!({"antilinkList": ["a.com"]}), "tiktok.com");
        let list = list_write(&r);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], json!("a.com"), "entri lama harus utuh");
        assert_eq!(list[1], json!("tiktok.com"));
        assert!(text_of(&r).contains("> Total: *2* link"), "{}", text_of(&r));
    }

    #[test]
    fn an_absent_list_starts_from_empty_and_reports_one() {
        let r = run(json!({}), "tiktok.com");
        assert_eq!(list_write(&r).len(), 1);
        assert!(text_of(&r).contains("> Total: *1* link"));
    }

    #[test]
    fn an_existing_link_is_refused_and_nothing_is_written() {
        let r = run(json!({"antilinkList": ["tiktok.com"]}), "tiktok.com");
        assert_eq!(text_of(&r), already_exists("tiktok.com"));
        assert!(r.mutations().is_empty(), "duplikat tidak boleh ditulis");
    }

    #[test]
    fn the_link_is_lowercased_before_the_duplicate_check() {
        // `m.text?.toLowerCase()` terjadi SEBELUM `includes`, jadi "TikTok.com"
        // dianggap duplikat dari "tiktok.com".
        let r = run(json!({"antilinkList": ["tiktok.com"]}), "TikTok.com");
        assert!(text_of(&r).starts_with("⚠️ Link"));
        assert!(r.mutations().is_empty());

        // Dan yang baru disimpan dalam huruf kecil.
        let r = run(json!({}), "TikTok.COM");
        assert_eq!(list_write(&r)[0], json!("tiktok.com"));
    }

    #[test]
    fn the_link_is_not_trimmed_so_padded_input_is_stored_verbatim() {
        // JS `m.text?.toLowerCase()` tanpa `.trim()` — spasi ikut tersimpan.
        // Menambah trim di sini akan membuatnya cocok dengan `delantilink`,
        // padahal di JS tidak.
        let r = run(json!({}), "  spaced.com  ");
        assert_eq!(list_write(&r)[0], json!("  spaced.com  "));
    }

    #[test]
    fn a_non_string_entry_does_not_match_and_is_preserved_on_write() {
        let g = json!({"antilinkList": [1, null, "x.com"]});
        let r = run(g, "x.com");
        assert!(text_of(&r).starts_with("⚠️"), "string ke-3 tetap duplikat");

        let r2 = run(json!({"antilinkList": [1, null]}), "x.com");
        let list = list_write(&r2);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], json!(1), "entri non-string harus ikut tersimpan");
        assert_eq!(list[1], json!(null));
    }

    #[test]
    fn a_non_array_antilink_list_is_treated_as_empty() {
        // `groupData.antilinkList || []` — objek/string bukan array, lalu
        // `.includes` di JS akan berperilaku aneh. Yang aman dan konsisten dengan
        // store: perlakukan sebagai kosong.
        let r = run(json!({"antilinkList": "bukan-array"}), "x.com");
        assert_eq!(list_write(&r).len(), 1);
        assert!(text_of(&r).contains("Total: *1*"));
    }

    #[test]
    fn the_added_reply_points_at_listantilink_with_the_live_prefix() {
        let r = run(json!({}), "x.com");
        assert!(text_of(&r).contains("`.listantilink`"));
    }
}