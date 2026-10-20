//! `plugins/owner/ban.js` — masukkan nomor ke `setting('bannedUsers')`.
//!
//! `ponytail:` `config.bannedUsers = bannedList` di JS hanya mengisi objek
//! config in-memory (dipakai `config.isBanned`), bukan store — jadi tidak ada
//! mutasi untuk itu. `config.isOwner(target)` juga tidak ikut: `Ctx` hanya
//! membawa satu flag untuk pengirim, bukan untuk sembarang nomor, sehingga
//! ban owner tidak dicegah di port ini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

fn bad_input(prefix: &str) -> String {
    format!(
        "🚫 *ʙᴀɴ ᴅsᴇʀ*\n\n> Masukkan nomor atau tag user\n\n`Contoh: {p}ban 6281234567890`",
        p = prefix
    )
}

/// `m.quoted` → `mentionedJid[0]` → `args[0]`; lalu `lidToJid` kalau berakhiran
/// `@lid` — tanpa cache, `lidToJid` hanya mengganti suffix itu
/// (`src/lib/lid.js:178-190`), jadi cukup mengganti akhirannya.
fn raw_target(ctx: &Ctx) -> String {
    let raw = if let Some(q) = ctx.quoted() {
        q.sender.clone()
    } else if let Some(jid) = ctx.mentioned.first() {
        jid.clone()
    } else {
        ctx.args.first().cloned().unwrap_or_default()
    };
    match raw.strip_suffix("@lid") {
        Some(head) => format!("{head}@s.whatsapp.net"),
        None => raw,
    }
}

/// Digit saja, lalu dua cabang `'62' + num.slice(1)` — `slice(1)` membuang
/// SATU karakter, jadi `0812…` jadi `62812…`, bukan `6212…`.
fn normalize_number(raw: &str) -> String {
    let mut num: String = raw.chars().filter(char::is_ascii_digit).collect();
    if num.starts_with("08") {
        num = format!("62{}", &num[1..]);
    }
    if num.starts_with('0') {
        num = format!("62{}", &num[1..]);
    }
    num
}

/// `String(banned).replace(/[^0-9]/g, '')` — entri bisa string atau objek.
fn digits_of(entry: &serde_json::Value) -> String {
    crate::common::joined_strings(std::slice::from_ref(entry))
        .into_iter()
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

/// `c === n || c.endsWith(n) || n.endsWith(c)`.
fn already_banned(list: &[serde_json::Value], target: &str) -> bool {
    list.iter().any(|b| {
        let c = digits_of(b);
        c == target || c.ends_with(target) || target.ends_with(&c)
    })
}

fn banned_detail(number: &str, total: usize) -> String {
    format!(
        "🚫 *ᴜsᴇʀ ᴅɪʙᴀɴɴᴇᴅ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 📱 ɴᴏᴍᴏʀ: `{number}`\n\
         ┃ 🚫 sᴛᴀᴛᴜs: `Banned`\n\
         ┃ 📊 ᴛᴏᴛᴀʟ: `{total}` ᴜsᴇʀ\n\
         ╰┈┈⬡"
    )
}

#[plugin(
    name = "ban",
    alias = ["addban", "block"],
    category = "owner",
    description = "Memblokir user dari menggunakan bot",
    usage = ".ban <nomor/@tag>",
    example = ".ban 6281234567890",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ban(ctx: Ctx) -> Reply {
    let number = normalize_number(&raw_target(&ctx));
    let len = number.chars().count();
    if !(10..=15).contains(&len) {
        return Reply::text(bad_input(&ctx.prefix));
    }

    let list = ctx.setting_list("bannedUsers").to_vec();
    if already_banned(&list, &number) {
        return Reply::text(format!(
            "❌ *ɢᴀɢᴀʟ*\n\n> Nomor `{number}` sudah dibanned"
        ));
    }

    let mut next = list;
    next.push(serde_json::Value::String(number.clone()));
    Reply::text(banned_detail(&number, next.len()))
        .with_reaction("🚫")
        .with_mutation(Mutation::SetSetting {
            key: "bannedUsers".into(),
            value: serde_json::Value::Array(next),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(text: &str, mentioned: Option<&str>, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ban".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("ban").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 1, "satu write untuk daftar ban");
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "bannedUsers");
                value.as_array().expect("harus array").clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ban").unwrap();
        assert_eq!(p.meta.name, "ban");
        assert_eq!(p.meta.alias, ["addban", "block"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Memblokir user dari menggunakan bot");
        assert_eq!(p.meta.usage, ".ban <nomor/@tag>");
        assert_eq!(p.meta.example, ".ban 6281234567890");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `block` juga nama `plugins/owner/block.js`; nama didahulukan
        // (`plugins.js:539`), jadi alias `block` bukan milik ban.
        assert_eq!(oktz_plugin_api::find("addban").unwrap().meta.name, "ban");
        assert_eq!(oktz_plugin_api::find("block").unwrap().meta.name, "block");
    }

    #[test]
    fn a_number_shorter_than_ten_or_longer_than_fifteen_is_refused() {
        for text in ["", "123", "6281234567890123456"] {
            let r = run(text, None, serde_json::json!({}));
            assert!(text_of(&r).starts_with("🚫 *ʙᴀɴ ᴅsᴇʀ*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn exactly_ten_and_exactly_fifteen_digits_are_accepted() {
        for text in ["1234567890", "123456789012345"] {
            assert_eq!(stored(&run(text, None, serde_json::json!({}))).len(), 1, "input {text:?}");
        }
    }

    #[test]
    fn a_mention_wins_over_the_argument() {
        let r = run("628000000000", Some("6289991234567@s.whatsapp.net"), serde_json::json!({}));
        assert_eq!(stored(&r)[0], serde_json::json!("6289991234567"));
    }

    #[test]
    fn a_leading_zero_is_rewritten_to_sixty_two_once() {
        assert_eq!(normalize_number("081234567890"), "6281234567890");
        // `slice(1)` membuang SATU karakter: "0812" → "62" + "812" = "62812".
        assert_eq!(normalize_number("0812"), "62812");
        assert_eq!(normalize_number("0 812-3456-7890"), "6281234567890");
    }

    #[test]
    fn a_lid_target_loses_its_suffix_before_normalisation() {
        let ctx = Ctx {
            mentioned: vec!["123456789012345@lid".into()],
            ..Default::default()
        };
        assert_eq!(normalize_number(&raw_target(&ctx)), "123456789012345");
    }

    #[test]
    fn a_number_already_on_the_list_is_refused_and_writes_nothing() {
        let existing = serde_json::json!({ "bannedUsers": ["6281234567890"] });
        let r = run("6281234567890", None, existing);
        assert!(text_of(&r).contains("sudah dibanned"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_suffix_or_prefix_overlap_counts_as_already_banned() {
        // `c === n || c.endsWith(n) || n.endsWith(c)` — perbandingan SUFFIX,
        // bukan normalisasi nomor. Jadi hanya entri yang benar-benar berakhian
        // sama yang cocok; "081234567890" TIDAK cocok dengan "6281234567890".
        for existing in ["81234567890", "126281234567890", "6281234567890"] {
            let settings = serde_json::json!({ "bannedUsers": [existing] });
            let r = run("6281234567890", None, settings);
            assert!(
                text_of(&r).contains("sudah dibanned"),
                "entri {existing} harus cocok: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_prefix_overlap_that_is_not_a_real_suffix_does_not_block_the_ban() {
        // Guard batas: "081234567890" share digit AWAL dengan target tapi bukan
        // suffix, jadi JS tetap memban. Menahan ini menjaga `endsWith` dari
        // berubah jadi `startsWith` atau normalisasi diam-diam.
        let settings = serde_json::json!({ "bannedUsers": ["081234567890"] });
        let r = run("6281234567890", None, settings);
        assert_eq!(stored(&r).len(), 2);
    }

    #[test]
    fn banning_appends_to_the_existing_list_and_reports_the_new_total() {
        let existing = serde_json::json!({ "bannedUsers": ["628111111111"] });
        let r = run("628999999999", None, existing);
        assert_eq!(
            stored(&r),
            vec![serde_json::json!("628111111111"), serde_json::json!("628999999999")]
        );
        assert!(text_of(&r).contains("┃ 📊 ᴛᴏᴛᴀʟ: `2` ᴜsᴇʀ"), "{}", text_of(&r));
    }

    #[test]
    fn the_applied_reply_reacts_and_is_paired_with_the_write() {
        let r = run("628999999999", None, serde_json::json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🚫"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}