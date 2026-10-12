//! `plugins/group/delantilink.js` — `groups[chat].antilinkList` hapus satu.
//!
//! Tanpa argumen plugin ini BERBEDA dari `addantilink`: ia menampilkan
//! daftar yang ada, bukan bantuan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.args.join(' ')?.trim()?.toLowerCase()` — token digabung dengan spasi
/// dulu, lalu dipangkas. Berbeda dari `addantilink` yang tidak `trim`.
pub(crate) fn link(ctx: &Ctx) -> String {
    ctx.args.join(" ").trim().to_lowercase()
}

pub(crate) fn empty_list() -> &'static str {
    "📋 Daftar antilink kosong!"
}

/// Daftar bernomor + ajakan hapus. `i + 1` mulai dari satu.
pub(crate) fn render_list(ctx: &Ctx, list: &[serde_json::Value]) -> String {
    let mut txt = String::from("🔗 *ᴅᴀꜰᴛᴀʀ ᴀɴᴛɪʟɪɴᴋ*\n\n");
    for (i, l) in list.iter().enumerate() {
        txt.push_str(&format!("> {}. `{}`\n", i + 1, l.as_str().unwrap_or("")));
    }
    txt.push_str(&format!("\n> Total: *{}* link", list.len()));
    txt.push_str(&format!(
        "\n\n`{}delantilink <domain>` untuk hapus",
        ctx.prefix
    ));
    txt
}

pub(crate) fn not_found(link: &str) -> String {
    format!("⚠️ Link `{link}` tidak ditemukan di daftar antilink!")
}

pub(crate) fn removed(link: &str, left: usize) -> String {
    format!("✅ *ᴀɴᴛɪʟɪɴᴋ ᴅɪʜᴀᴘᴜs*\n\n> Link: `{link}`\n> Sisa: *{left}* link")
}

#[plugin(
    name = "delantilink",
    alias = ["delalink", "delblocklink", "remantilink"],
    category = "group",
    description = "Menghapus link dari daftar antilink",
    usage = ".delantilink <domain/pattern>",
    example = ".delantilink tiktok.com",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn delantilink(ctx: Ctx) -> Reply {
    let list = ctx.group_list("antilinkList");
    let link = link(&ctx);

    if link.is_empty() {
        return Reply::text(if list.is_empty() {
            empty_list().to_string()
        } else {
            render_list(&ctx, list)
        });
    }

    // `findIndex(l => l === link)` — perbandingan identity string, bukan
    // substring dan bukan case-insensitive lagi (link sudah lowercase).
    let Some(index) = list.iter().position(|v| v.as_str() == Some(link.as_str())) else {
        return Reply::text(not_found(&link));
    };

    let mut next = list.to_vec();
    next.remove(index);
    let left = next.len();

    Reply::text(removed(&link, left)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "antilinkList": next }),
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
            command: "delantilink".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("delantilink").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("delantilink").unwrap();
        assert_eq!(p.meta.name, "delantilink");
        assert_eq!(p.meta.alias, ["delalink", "delblocklink", "remantilink"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Menghapus link dari daftar antilink");
        assert_eq!(p.meta.usage, ".delantilink <domain/pattern>");
        assert_eq!(p.meta.example, ".delantilink tiktok.com");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delalink", "delblocklink", "remantilink"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "delantilink");
        }
    }

    #[test]
    fn no_argument_on_an_empty_list_says_the_list_is_empty() {
        for g in [json!(null), json!({}), json!({"antilinkList": []}), json!({"antilinkList": null})] {
            let r = run(g.clone(), "");
            assert_eq!(text_of(&r), empty_list(), "input {g}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn no_argument_on_a_populated_list_numbers_every_entry_from_one() {
        let r = run(json!({"antilinkList": ["a.com", "b.com", "c.com"]}), "");
        let t = text_of(&r);
        assert!(t.starts_with("🔗 *ᴅᴀꜰᴛᴀʀ ᴀɴᴛɪʟɪɴᴋ*"));
        assert!(t.contains("> 1. `a.com`\n> 2. `b.com`\n> 3. `c.com`"), "{t}");
        assert!(t.contains("> Total: *3* link"));
        assert!(t.ends_with("`.delantilink <domain>` untuk hapus"));
        assert!(r.mutations().is_empty(), "listing tidak boleh menulis");
    }

    #[test]
    fn removing_the_first_entry_keeps_the_rest_in_order() {
        let r = run(json!({"antilinkList": ["a.com", "b.com", "c.com"]}), "a.com");
        let list = list_write(&r);
        assert_eq!(list, &vec![json!("b.com"), json!("c.com")]);
        assert!(text_of(&r).contains("> Sisa: *2* link"));
    }

    #[test]
    fn removing_the_last_entry_leaves_an_empty_but_present_list() {
        let r = run(json!({"antilinkList": ["only.com"]}), "only.com");
        assert!(list_write(&r).is_empty(), "list jadi array kosong, bukan hilang");
        assert!(text_of(&r).contains("> Sisa: *0* link"));
    }

    #[test]
    fn a_missing_link_is_reported_and_nothing_is_written() {
        let r = run(json!({"antilinkList": ["a.com"]}), "tiktok.com");
        assert_eq!(text_of(&r), not_found("tiktok.com"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn matching_is_whole_string_not_substring() {
        // `l === link`, bukan `includes` — "com" tidak boleh menghapus
        // "tiktok.com".
        let r = run(json!({"antilinkList": ["tiktok.com"]}), "com");
        assert!(text_of(&r).starts_with("⚠️"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_argument_is_joined_trimmed_and_lowercased() {
        // `m.args.join(' ').trim().toLowerCase()` — tiga transformasi, dan
        // trim-nya TIDAK ada di `addantilink`.
        let r = run(json!({"antilinkList": ["a.com", "b.com"]}), "  A.com  ");
        assert_eq!(list_write(&r), &vec![json!("b.com")]);
    }

    #[test]
    fn a_non_string_entry_is_skipped_not_matched() {
        let r = run(json!({"antilinkList": [1, null, "a.com"]}), "1");
        // `1` adalah number, bukan string "1" — `l === link` tidak cocok.
        assert!(text_of(&r).starts_with("⚠️"), "angka tidak boleh ikut terhapus");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_removed_reply_is_paired_with_the_write() {
        let r = run(json!({"antilinkList": ["a.com"]}), "a.com");
        assert!(text_of(&r).contains("ᴅɪʜᴀᴘᴜs"));
        assert_eq!(r.mutations().len(), 1);
    }

    /// Kunci add/del harus tetap terpisah supaya `find()` tidak ambigu.
    #[test]
    fn add_and_delete_antilink_keep_separate_keys() {
        for (name, keys) in [
        ("addantilink", &["addantilink", "addalink", "addblocklink"][..]),
        (
            "delantilink",
            &["delantilink", "delalink", "delblocklink", "remantilink"][..],
        ),
    ] {
            for k in keys {
                let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(k))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, [name], "key {k} diklaim oleh {owners:?}");
            }
        }
    }
}