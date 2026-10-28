//! `plugins/owner/unban.js` — keluarkan satu nomor dari `bannedUsers`.
//!
//! `ponytail:` dua hal tidak ada di `Ctx` dan sengaja tidak ditebak:
//! 1. `isLid`/`lidToJid` (`src/lib/lid.js`) butuh cache di disk. Tanpa cache,
//!    `lidToJid` tinggal mengganti sufiks `@lid` — dan karena `resolveTarget`
//!    membuang semua non-digit sesudahnya, digit hasilnya tetap sama.
//! 2. `config.bannedUsers = bannedList` menulis cache in-memory `config.js`;
//!    sumber yang dibaca `config.isBanned` adalah `db.setting('bannedUsers')`,
//!    jadi mutasi setting sudah cukup.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

pub(crate) const NO_TARGET: &str = "✅ *ᴜɴʃᴀɴ ᴅsᴇʀ*\n\n> Masukkan nomor atau tag user";
pub(crate) const NOT_BANNED: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Nomor `";

/// `raw.replace(/[^0-9]/g, '')` — semua non-digit dibuang, `@lid` dan
/// `@s.whatsapp.net` lenyap tanpa sisa.
pub(crate) fn digits_of(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}

/// `isLid(raw) ? lidToJid(raw) : raw` tanpa cache disk — hanya ganti sufiks,
/// dan hasilnya belum dipakai sampai `digits_of` membuang semua non-digit.
pub(crate) fn strip_lid(raw: &str) -> &str {
    raw.strip_suffix("@lid").unwrap_or(raw)
}

/// Normalisasi nomor Indonesia: `08…` → `62…`, lalu `0…` → `62…`.
pub(crate) fn normalize(mut num: String) -> String {
    if num.starts_with("08") {
        num = format!("62{}", &num[1..]);
    }
    if num.starts_with('0') {
        num = format!("62{}", &num[1..]);
    }
    num
}

/// `resolveTarget` — reply lebih dulu, lalu mention, lalu argumen pertama.
/// Reply yang ada tapi pengirimnya kosong TETAP dipakai: `m.quoted.sender || ''`
/// mengembalikan `''`, dan `if (!raw) return ''` keluar tanpa mencoba
/// mention atau argumen.
pub(crate) fn resolve_target(ctx: &Ctx) -> String {
    let raw = if let Some(q) = ctx.quoted() {
        q.sender.clone()
    } else if let Some(jid) = ctx.mentioned.first() {
        jid.clone()
    } else {
        ctx.args.first().cloned().unwrap_or_default()
    };
    if raw.is_empty() {
        return String::new();
    }
    normalize(digits_of(strip_lid(&raw)))
}

/// `String(b)` — elemen non-string dirender sebagai teksnya (`${el}` di JS).
fn stringify(v: &Value) -> String {
    match v.as_str() {
        Some(s) => s.to_string(),
        None => v.to_string(),
    }
}

/// `findIndex` JS: cocok kalau sama persis, atau salah satunya sufiks yang lain.
pub(crate) fn matches(entry: &Value, target: &str) -> bool {
    let c = digits_of(&stringify(entry));
    c == target || (!c.is_empty() && c.ends_with(target)) || (!target.is_empty() && target.ends_with(&c))
}

pub(crate) fn detail(target: &str, remaining: usize) -> String {
    format!(
        "✅ *ᴜsᴇʀ ᴅɪᴜɴʃᴀɴ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 📱 ɴᴏᴍᴏʀ: `{target}`\n\
         ┃ ✅ sᴛᴀᴛᴜs: `Unbanned`\n\
         ┃ 📊 ᴛᴏᴛᴀʟ: `{remaining}` ᴜsᴇʀ\n\
         ╰┈┈┈┈┈┈┈┈⬡"
    )
}

#[plugin(
    name = "unban",
    alias = ["delban", "unblock"],
    category = "owner",
    description = "Menghapus user dari daftar banned",
    usage = ".unban <nomor/@tag>",
    example = ".unban 6281234567890",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn unban(ctx: Ctx) -> Reply {
    let target = resolve_target(&ctx);
    if target.len() < 10 || target.len() > 15 {
        return Reply::text(format!(
            "{NO_TARGET}\n\n`Contoh: {}unban 6281234567890`",
            ctx.prefix
        ));
    }

    let banned = ctx.setting_list("bannedUsers");
    let Some(index) = banned.iter().position(|b| matches(b, &target)) else {
        return Reply::text(format!("{NOT_BANNED}{target}` tidak dalam daftar banned"));
    };

    // JS `bannedList.splice(index, 1)` lalu tulis ulang seluruh list.
    let mut stored = banned.to_vec();
    stored.remove(index);

    Reply::text(detail(&target, stored.len()))
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "bannedUsers".into(),
            value: Value::Array(stored),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn run(args: &str, mentioned: &[&str], quoted_from: Option<&str>, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "unban".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.iter().map(|s| s.to_string()).collect(),
            quoted: quoted_from.map(|s| QuotedMessage {
                id: "MSG1".into(),
                sender: s.into(),
                chat: "628111@s.whatsapp.net".into(),
                text: "hi".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            timestamp: 0,
        }),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("unban").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    fn stored_list(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "bannedUsers");
                value.as_array().cloned().expect("harus array")
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("unban").unwrap();
        assert_eq!(p.meta.name, "unban");
        assert_eq!(p.meta.alias, ["delban", "unblock"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Menghapus user dari daftar banned");
        assert_eq!(p.meta.usage, ".unban <nomor/@tag>");
        assert_eq!(p.meta.example, ".unban 6281234567890");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `unblock` juga nama `plugins/owner/unblock.js`; nama didahulukan
        // (`plugins.js:539`), jadi alias `unblock` bukan milik unban.
        assert_eq!(oktz_plugin_api::find("delban").unwrap().meta.name, "unban");
        assert_eq!(oktz_plugin_api::find("unblock").unwrap().meta.name, "unblock");
    }

    #[test]
    fn a_listed_number_is_removed_and_the_remaining_list_is_written() {
        let r = run(
            "6281234567890",
            &[],
            None,
            serde_json::json!({"bannedUsers": ["628111111111", "6281234567890"]}),
        );
        assert_eq!(stored_list(&r), vec![serde_json::json!("628111111111")]);
        assert!(text_of(&r).contains("┃ ✅ sᴛᴀᴛᴜs: `Unbanned`"));
        assert!(text_of(&r).contains("┃ 📊 ᴛᴏᴛᴀʟ: `1` ᴜsᴇʀ"));
        assert_eq!(reaction_of(&r), "✅");
    }

    #[test]
    fn a_mention_or_a_quote_can_supply_the_number() {
        let list = serde_json::json!({"bannedUsers": ["6289998887776"]});
        assert!(
            stored_list(&run(
                "",
                &["6289998887776@s.whatsapp.net"],
                None,
                list.clone()
            ))
            .is_empty()
        );
        assert!(
            stored_list(&run(
                "",
                &[],
                Some("6289998887776@s.whatsapp.net"),
                list
            ))
            .is_empty()
        );
    }

    #[test]
    fn a_quoted_wins_over_the_argument() {
        let r = run(
            "6280000000000",
            &[],
            Some("6289998887776@s.whatsapp.net"),
            serde_json::json!({"bannedUsers": ["6289998887776"]}),
        );
        assert!(
            stored_list(&r).is_empty(),
            "yang dihapus harus yang di-reply, bukan argumen"
        );
        assert!(text_of(&r).contains("6289998887776"));
    }

    #[test]
    fn a_number_outside_the_length_bounds_is_rejected_before_any_lookup() {
        for bad in ["", "628", "628123456789012345678"] {
            let r = run(bad, &[], None, serde_json::json!({"bannedUsers": ["6281234567890"]}));
            assert!(text_of(&r).starts_with(NO_TARGET), "arg {bad:?}");
            assert!(r.mutations().is_empty(), "arg {bad:?}");
        }
    }

    #[test]
    fn an_unlisted_number_is_reported_and_writes_nothing() {
        let r = run(
            "6280000000000",
            &[],
            None,
            serde_json::json!({"bannedUsers": ["628111111111"]}),
        );
        assert!(text_of(&r).starts_with(NOT_BANNED));
        assert!(text_of(&r).ends_with("` tidak dalam daftar banned"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_usage_hint_carries_the_live_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "unban".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("unban").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("`Contoh: !unban 6281234567890`"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn indonesian_number_formats_are_normalized_before_matching() {
        assert_eq!(normalize("081234567890".into()), "6281234567890");
        assert_eq!(normalize("01234567890".into()), "621234567890");
        assert_eq!(normalize("6281234567890".into()), "6281234567890");
        // dua transform berurutan: "08" -> "62" dulu, jadi tidak jadi "628".
        assert_eq!(normalize("0812".into()), "62812");
    }

    #[test]
    fn suffix_matching_is_bidirectional() {
        assert!(matches(&serde_json::json!("6281234567890"), "6281234567890"));
        // nomor tersimpan boleh punya awalan/akhiran lain; yang dicari adalah
        // salah satunya sufiks yang lain, persis `findIndex` di JS.
        assert!(matches(&serde_json::json!("126281234567890"), "6281234567890"));
        assert!(matches(&serde_json::json!("234567890"), "6281234567890"));
        assert!(matches(&serde_json::json!("+62 812-3456-7890"), "6281234567890"));
        assert!(!matches(&serde_json::json!("6289999999999"), "6281234567890"));
    }

    // `"6281…".endsWith("")` = true di JS, jadi entri kosong di daftar banned
    // akan mencocoki semua nomor. Bukan bug port — `findIndex` memang begitu.
    // Dikunci di sini supaya tidak "dibetulkan" diam-diam; `ban.js` tidak
    // pernah menulis entri kosong.
    #[test]
    fn an_empty_entry_matches_everything_just_like_in_javascript() {
        assert!(matches(&serde_json::json!(""), "6281234567890"));
        assert!(matches(&serde_json::json!("   "), "6281234567890"));
    }

    #[test]
    fn a_lid_suffix_leaves_no_digits_behind() {
        assert_eq!(strip_lid("12345@lid"), "12345");
        assert_eq!(strip_lid("12345@s.whatsapp.net"), "12345@s.whatsapp.net");
        assert_eq!(digits_of(strip_lid("12345@lid")), "12345");
    }
}