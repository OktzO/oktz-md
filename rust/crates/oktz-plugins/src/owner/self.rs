//! `plugins/owner/self.js` — set mode bot ke `self`.
//!
//! `ponytail:` sama seperti `public.rs`: `config.mode` dibaca lewat
//! `db.setting('botMode')` dan `m.fromMe` tidak ada di `Ctx`. Cek owner dan
//! pembacaan mode dipinjam dari `public.rs` supaya kedua plugin tidak
//! menyimpang diam-diam.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::Value;

use crate::owner::public::{DENIED, current_mode, is_real_owner};

pub(crate) const SELF_TEXT: &str = "🔒 *ᴍᴏᴅᴇ sᴇʟꜰ ᴀᴋᴛɪꜰ*\n\n> Bot sekarang hanya merespon:\n> • Owner bot\n> • Bot sendiri (fromMe)\n\n_Gunakan .public untuk membuka akses_";
pub(crate) const ALREADY_SELF: &str = "ℹ️ Bot sudah dalam mode *self*";

#[plugin(
    name = "self",
    alias = ["selfmode", "private-mode"],
    category = "owner",
    description = "Mengaktifkan mode self (hanya owner & bot yang bisa akses)",
    usage = ".self",
    example = ".self",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn self_mode(ctx: Ctx) -> Reply {
    if !is_real_owner(&ctx) {
        return Reply::text(DENIED);
    }
    if current_mode(&ctx) == "self" {
        return Reply::text(ALREADY_SELF);
    }
    Reply::text(SELF_TEXT).with_mutation(Mutation::SetSetting {
        key: "botMode".into(),
        value: Value::String("self".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(sender: &str, is_owner: bool, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "self".into(),
            sender: sender.into(),
            chat: sender.into(),
            is_owner,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("self").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("self").unwrap();
        assert_eq!(p.meta.name, "self");
        assert_eq!(p.meta.alias, ["selfmode", "private-mode"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengaktifkan mode self (hanya owner & bot yang bisa akses)"
        );
        assert_eq!(p.meta.usage, ".self");
        assert_eq!(p.meta.example, ".self");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["selfmode", "private-mode"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "self");
        }
    }

    #[test]
    fn the_owner_switches_the_mode_and_writes_botmode() {
        let r = run("6285608953677@s.whatsapp.net", true, json!({"botMode": "public"}));
        assert_eq!(text_of(&r), SELF_TEXT);
        assert_eq!(written_mode(&r), Some("self".to_string()));
    }

    #[test]
    fn a_stored_self_mode_is_only_reported_never_rewritten() {
        let r = run("6285608953677@s.whatsapp.net", true, json!({"botMode": "self"}));
        assert_eq!(text_of(&r), ALREADY_SELF);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn self_and_public_share_the_denied_reply() {
        let denied = run("628999999999@s.whatsapp.net", true, json!({}));
        assert_eq!(text_of(&denied), DENIED);
        assert!(denied.mutations().is_empty());
    }
}