//! `plugins/main/owner.js` — kartu identitas owner bot.
//!
//! `ponytail:` dua bagian yang hilang.
//!
//! 1. `ownerType === 2` mengirim contact card (`contacts`/`vcard`) lewat
//!    `sock.sendMessage`; `Reply` tidak punya varian vcard. Cabang itu tidak
//!    ada di sini —Owner hanya membaca daftar owner.
//! 2. `botConfig.owner?.number` berasal dari `config.js` dan tidak ada di
//!    `Ctx`. Daftar owner diambil dari `settings.owner`, yang juga tempat
//!    `owner/*` menulis. Nama bot dan nama owner default di-hardcode dari
//!    `config.js:15,38` — sama seperti `info/benefitpartner.rs`.
//!
//! Contact card per-owner pada cabang `else` juga tidak bisa dikirim, tapi
//!    teksnya utuh.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

pub(crate) const BOT_NAME: &str = "Oktz Bot";
pub(crate) const OWNER_NAME: &str = "Oktz";

/// `getOwnerName` (`config.js:537`): peta `ownerNames`, lalu nama utama.
pub(crate) fn owner_name(ctx: &Ctx, number: &str) -> String {
    let clean: String = number.chars().filter(char::is_ascii_digit).collect();
    if clean.is_empty() {
        return OWNER_NAME.to_string();
    }
    ctx.setting("ownerNames")
        .and_then(|m| m.get(&clean))
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(OWNER_NAME)
        .to_string()
}

/// `[...new Set([...configOwners, ...dbOwners])]` — urutan pertama-tetap,
/// duplikat dibuang per nomor BERSIH (`+62 811` dan `62811` itu satu orang).
pub(crate) fn owner_numbers(ctx: &Ctx) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for raw in joined_strings(ctx.setting_list("owner")) {
        let clean: String = raw.chars().filter(char::is_ascii_digit).collect();
        if clean.is_empty() || seen.contains(&clean) {
            continue;
        }
        seen.push(clean.clone());
        out.push(raw);
    }
    out
}

pub(crate) fn owner_text(ctx: &Ctx, numbers: &[String]) -> String {
    let names = numbers
        .iter()
        .map(|n| owner_name(ctx, n))
        .collect::<Vec<String>>()
        .join(", ");
    format!(
        "👑 *ᴏᴡɴᴇʀ ɪɴꜰᴏʀᴍᴀᴛɪᴏɴ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ ㊗ ɴᴀᴍᴀ: *{names}*\n\
         ┃ ㊗ ʙᴏᴛ: *{BOT_NAME}*\n\
         ┃ ㊗ sᴛᴀᴛᴜs: *🟢 Online*\n\
         ╰┈┈⬡\n\n\
         > _Jika ada pertanyaan atau kendala,_\n\
         > _silakan hubungi owner di atas!_\n\
         > _📞 Contact card di bawah._"
    )
}

#[plugin(
    name = "owner",
    alias = ["creator", "dev", "developer"],
    category = "main",
    description = "Menampilkan kontak owner bot",
    usage = ".owner",
    example = ".owner",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn owner(ctx: Ctx) -> Reply {
    let numbers = owner_numbers(&ctx);
    Reply::text(owner_text(&ctx, &numbers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "owner".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("owner").unwrap().handler)(ctx);
        r.text_body().expect("harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("owner").unwrap();
        assert_eq!(p.meta.name, "owner");
        assert_eq!(p.meta.alias, ["creator", "dev", "developer"]);
        assert_eq!(p.meta.category, "main");
        assert_eq!(p.meta.description, "Menampilkan kontak owner bot");
        assert_eq!(p.meta.usage, ".owner");
        assert_eq!(p.meta.example, ".owner");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["owner", "creator", "dev", "developer"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("owner"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_renders_one_name_per_owner() {
        let out = run(serde_json::json!({"owner": ["6285608953677", "628999"]}));
        assert!(out.starts_with("👑 *ᴏᴡɴᴇʀ ɪɴꜰᴏʀᴍᴀᴛɪᴏɴ*\n\n"), "{out}");
        assert!(out.contains("┃ ㊗ ɴᴀᴍᴀ: *Oktz, Oktz*"), "satu nama per owner: {out}");
        assert!(out.contains("┃ ㊗ ʙᴏᴛ: *Oktz Bot*"), "{out}");
        assert!(out.contains("┃ ㊗ sᴛᴀᴛᴜs: *🟢 Online*"), "{out}");
        assert!(out.ends_with("> _📞 Contact card di bawah._"), "{out}");
    }

    #[test]
    fn a_named_owner_replaces_the_default_name() {
        let out = run(serde_json::json!({
            "owner": ["6285608953677"],
            "ownerNames": {"6285608953677": "Lyoraaa"}
        }));
        assert!(out.contains("┃ ㊗ ɴᴀᴍᴀ: *Lyoraaa*"), "{out}");
    }

    #[test]
    fn owner_name_falls_back_to_the_main_owner_name() {
        let ctx = Ctx {
            settings: serde_json::json!({"ownerNames": {"628999": "Danzzz"}}),
            ..Default::default()
        };
        assert_eq!(owner_name(&ctx, "6285608953677"), OWNER_NAME);
        assert_eq!(owner_name(&ctx, "628999"), "Danzzz");
        assert_eq!(owner_name(&ctx, ""), OWNER_NAME);
        assert_eq!(owner_name(&ctx, "  "), OWNER_NAME);
    }

    #[test]
    fn duplicate_owners_are_collapsed_per_clean_number() {
        let ctx = Ctx {
            settings: serde_json::json!({"owner": ["+62 811-1", "628111", "628111"]}),
            ..Default::default()
        };
        assert_eq!(owner_numbers(&ctx), ["+62 811-1".to_string()]);
    }

    #[test]
    fn the_fallback_branch_renders_an_empty_name_list() {
        for settings in [serde_json::json!({}), serde_json::json!({"owner": []}), serde_json::json!({"owner": "x"})] {
            let out = run(settings.clone());
            assert!(out.contains("┃ ㊗ ɴᴀᴍᴀ: **\n"), "{settings} -> {out}");
        }
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = Ctx {
            settings: serde_json::json!({"owner": ["628111"]}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("owner").unwrap().handler)(ctx);
        assert!(r.mutations().is_empty());
    }
}
