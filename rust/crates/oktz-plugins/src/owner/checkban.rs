//! `plugins/owner/checkban.js` — perintah debug yang mencetak keadaan ban
//! untuk satu nomor hardcode.
//!
//! `ponytail:` `config.isOwner(target)` hanya bisa dijawab untuk pengirim di
//! `Ctx`, jadi baris `isOwner()` pada balasannya dicetak sebagai `?`. Selain
//! itu `config.bannedUsers` di JS adalah cache in-memory yang diisi oleh
//! `config.isBanned()`; di sini hanya sumber `db.setting('bannedUsers')` yang
//! dibaca, karena itu satu-satunya yang benar-benar persisten.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// Nomor yang sama dengan di `checkban.js`.
const TARGET: &str = "628979985621@s.whatsapp.net";

/// `normalizeNumber` (`config.js:344-351`).
fn normalize_number(value: &str) -> String {
    value
        .split(':')
        .next()
        .unwrap_or(value)
        .split('@')
        .next()
        .unwrap_or(value)
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

/// `c === n || c.endsWith(n) || n.endsWith(c)` — dipakai `checkban.js`, bukan
/// `matchesNumber` yang dipakai `config.isBanned`.
fn loose_match(clean: &str, clean_banned: &str) -> bool {
    clean == clean_banned
        || clean.ends_with(clean_banned)
        || clean_banned.ends_with(clean)
}

#[plugin(
    name = "checkban",
    alias = [],
    category = "owner",
    description = "Check actual ban state",
    usage = ".checkban",
    example = "",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn checkban(ctx: Ctx) -> Reply {
    let clean = normalize_number(TARGET);
    let saved: Vec<String> = ctx
        .setting_list("bannedUsers")
        .iter()
        .flat_map(|v| joined_strings(std::slice::from_ref(v)))
        .collect();

    // `new Set([...configList, ...savedList])` — dupe pertama yang menang,
    // jadi urutan gabungan = config dulu, baru yang baru dari db.
    let mut combined: Vec<String> = Vec::new();
    for v in saved.iter().cloned() {
        if !combined.contains(&v) {
            combined.push(v);
        }
    }

    let is_banned_direct = combined
        .iter()
        .any(|b| loose_match(&clean, &normalize_number(b)));

    // `config.isBanned` = `!isOwner(n) && bannedList.some(matchesNumber)`,
    // dengan `matchesNumber` = kesamaan ketat setelah normalisasi.
    let is_banned = saved.iter().any(|b| normalize_number(b) == clean);

    Reply::text(format!(
        "DEBUG BAN ({TARGET})\n\
         cleanNumber: {clean}\n\
         bannedList (config): []\n\
         savedBanned (db): {}\n\
         isBannedDirect: {is_banned_direct}\n\
         config.isBanned(): {is_banned}\n\
         isOwner(): ?",
        saved.join(",")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "checkban".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("checkban").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("checkban").unwrap();
        assert_eq!(p.meta.name, "checkban");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Check actual ban state");
        assert_eq!(p.meta.usage, ".checkban");
        // `pluginConfig` JS tidak punya `example` maupun `cooldown`, jadi
        // keduanya memakai default router (`example: ""`, `cooldown: 3`).
        assert_eq!(p.meta.example, "");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn an_empty_list_reports_not_banned_and_writes_nothing() {
        let r = run(json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("DEBUG BAN (628979985621@s.whatsapp.net)"), "{t}");
        assert!(t.contains("cleanNumber: 628979985621"), "{t}");
        assert!(t.contains("isBannedDirect: false"), "{t}");
        assert!(t.contains("config.isBanned(): false"), "{t}");
        assert!(r.mutations().is_empty(), "perintah debug tidak boleh menulis");
    }

    #[test]
    fn the_hardcoded_target_on_the_list_reports_banned() {
        let r = run(json!({ "bannedUsers": ["628979985621"] }));
        let t = text_of(&r);
        assert!(t.contains("isBannedDirect: true"), "{t}");
        assert!(t.contains("config.isBanned(): true"), "{t}");
        assert!(t.contains("savedBanned (db): 628979985621"), "{t}");
    }

    #[test]
    fn a_different_number_on_the_list_reports_not_banned() {
        let r = run(json!({ "bannedUsers": ["628123456789"] }));
        assert!(text_of(&r).contains("config.isBanned(): false"), "{}", text_of(&r));
    }

    #[test]
    fn stored_entries_are_normalised_before_they_are_printed() {
        let r = run(json!({ "bannedUsers": ["628979985621:3@s.whatsapp.net"] }));
        let t = text_of(&r);
        assert!(t.contains("cleanNumber: 628979985621"), "{t}");
        assert!(t.contains("config.isBanned(): true"), "sufiks device dihapus: {t}");
    }

    #[test]
    fn normalisation_drops_device_and_server() {
        assert_eq!(normalize_number("628111:9@s.whatsapp.net"), "628111");
        assert_eq!(normalize_number("+62 811-1"), "628111");
        assert_eq!(normalize_number(""), "");
    }
}