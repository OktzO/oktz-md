//! `plugins/owner/public.js` — set mode bot ke `public`.
//!
//! `ponytail:` dua sumber runtime tidak ada di `Ctx`:
//! 1. `config.mode` — variabel mutable di `config.js`. Yang dipakai adalah
//!    `db.setting('botMode')`, cermin durable-nya yang juga dibaca router
//!    (`mode_settings_from`, `pipeline.rs:250`).
//! 2. `m.fromMe` — tidak ada di `Ctx`, jadi jalur `if (m.fromMe) return true`
//!    hilang; owner tetap harus lolos cek nomor di bawah.
//!
//! `OWNER_NUMBERS` ikut dipakai `self.rs`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::Value;

/// `config.owner.number` (`config.js:16`) — cek kedua setelah `isOwner`.
pub(crate) const OWNER_NUMBERS: [&str; 1] = ["6285608953677"];

pub(crate) const DENIED: &str = "🚫 *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*\n\n> Hanya owner yang bisa mengubah mode bot!";
pub(crate) const PUBLIC_TEXT: &str = "🌐 *ᴍᴏᴅᴇ ᴘᴜʙʟɪᴄ ᴀᴋᴛɪꜰ*\n\n> Bot sekarang merespon semua user!\n\n_Gunakan .self untuk menutup akses_";
pub(crate) const ALREADY: &str = "ℹ️ Bot sudah dalam mode *public*";

/// `db.setting('botMode') || config.mode || 'public'` — nilai kosong dibiarkan
/// jatuh ke default, sama seperti `mode.rs:73-77`.
pub(crate) fn current_mode(ctx: &Ctx) -> String {
    ctx.setting("botMode")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("public")
        .to_string()
}

/// `validateOwner` (`public.js:57-70`) — `m.fromMe` tidak ada di `Ctx`.
pub(crate) fn is_real_owner(ctx: &Ctx) -> bool {
    if !ctx.is_owner {
        return false;
    }
    let sender: String = ctx.sender.chars().filter(char::is_ascii_digit).collect();
    let in_list = OWNER_NUMBERS.iter().any(|owner| {
        let clean: String = owner.chars().filter(char::is_ascii_digit).collect();
        sender.contains(&clean) || clean.contains(&sender)
    });
    if !in_list {
        return false;
    }
    ctx.sender.contains('@')
}

#[plugin(
    name = "public",
    alias = ["publicmode"],
    category = "owner",
    description = "Mengaktifkan mode public (semua user bisa akses)",
    usage = ".public",
    example = ".public",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn public(ctx: Ctx) -> Reply {
    if !is_real_owner(&ctx) {
        return Reply::text(DENIED);
    }
    if current_mode(&ctx) == "public" {
        return Reply::text(ALREADY);
    }
    Reply::text(PUBLIC_TEXT).with_mutation(Mutation::SetSetting {
        key: "botMode".into(),
        value: Value::String("public".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sender: &str, is_owner: bool, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "public".into(),
            sender: sender.into(),
            chat: sender.into(),
            is_owner,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("public").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written_mode(r: &Reply) -> Option<String> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "botMode" => {
                value.as_str().map(str::to_string)
            }
            _ => None,
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("public").unwrap();
        assert_eq!(p.meta.name, "public");
        assert_eq!(p.meta.alias, ["publicmode"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengaktifkan mode public (semua user bisa akses)"
        );
        assert_eq!(p.meta.usage, ".public");
        assert_eq!(p.meta.example, ".public");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("publicmode").unwrap().meta.name, "public");
    }

    #[test]
    fn the_owner_switches_the_mode_and_writes_botmode() {
        let r = run("6285608953677@s.whatsapp.net", true, serde_json::json!({"botMode": "self"}));
        assert_eq!(text_of(&r), PUBLIC_TEXT);
        assert_eq!(written_mode(&r), Some("public".to_string()));
    }

    #[test]
    fn a_stored_public_mode_is_only_reported_never_rewritten() {
        let r = run("6285608953677@s.whatsapp.net", true, serde_json::json!({"botMode": "public"}));
        assert_eq!(text_of(&r), ALREADY);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_missing_or_empty_setting_falls_back_to_the_js_default_mode() {
        for settings in [serde_json::json!({}), serde_json::json!({"botMode": ""})] {
            let r = run("6285608953677@s.whatsapp.net", true, settings);
            assert_eq!(text_of(&r), ALREADY, "config.mode default 'public'");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_non_owner_is_rejected_even_when_the_number_matches() {
        let r = run("6285608953677@s.whatsapp.net", false, serde_json::json!({}));
        assert_eq!(text_of(&r), DENIED);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_owner_flagged_sender_outside_the_number_list_is_rejected() {
        // `isOwner` di JS sudah berarti cfg, tapi `validateOwner` cek ulang —
        // dua-duanya harus benar.
        let r = run("628999999999@s.whatsapp.net", true, serde_json::json!({}));
        assert_eq!(text_of(&r), DENIED);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_sender_without_a_server_suffix_is_rejected() {
        assert!(!is_real_owner(&Ctx {
            sender: "6285608953677".into(),
            is_owner: true,
            ..Default::default()
        }));
    }
}