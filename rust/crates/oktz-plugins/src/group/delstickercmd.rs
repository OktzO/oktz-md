//! `plugins/group/delstickercmd.js` — hapus satu entri sticker-command.
//!
//! `ponytail:` dua hal TIDAK di-port. Pertama, `getQuotedStickerHash(m)`
//! membaca `fileSha256`/`mediaKey` dari `quoted.message.stickerMessage`, dan
//! `QuotedMessage` tidak punya keduanya — jadi cabang "reply sticker" selalu
//! jatuh ke pesan "Gagal menghapus!". Kedua, `src/lib/sticker-command.js`
//! menyimpan ke `database/stickerCommands.json`; satu-satunya operasi tulis
//! yang tersedia di `Mutation` adalah `SetSetting`, jadi pemetaannya disimpan
//! di `setting("stickerCommands")` dengan bentuk yang sama (hash →
//! `{command, addedBy, addedAt}`). Plugin yang menambah entri
//! (`addcmdsticker`) harus menulis ke key yang sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `database/stickerCommands.json` → `setting("stickerCommands")`.
const STORE: &str = "stickerCommands";

const EMPTY: &str = "🖼️ *sᴛɪᴄᴋᴇʀ ᴄᴏᴍᴍᴀɴᴅs*\n\n\
     > Tidak ada sticker command yang terdaftar.\n\
     > Tambahkan dengan `.addcmdsticker`";

fn list_card(names: &[String]) -> String {
    let mut txt = String::from("🖼️ *sᴛɪᴄᴋᴇʀ ᴄᴏᴍᴍᴀɴᴅs*\n\n╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ* 」\n");
    for cmd in names {
        txt.push_str(&format!("┃ 🖼️ → `.{cmd}`\n"));
    }
    txt.push_str(
        "╰┈┈┈┈┈┈┈┈⬡\n\n*Hapus dengan:*\n\
         > `.delstickercmd <command>`\n\
         > atau reply sticker + `.delstickercmd`",
    );
    txt
}

fn not_found(clean: &str) -> String {
    format!(
        "❌ Sticker command `{clean}` tidak ditemukan!\n\n\
         > Lihat daftar dengan `.delstickercmd`"
    )
}

fn deleted_msg(cmd: &str) -> String {
    format!("✅ *sᴛɪᴄᴋᴇʀ ᴄᴏᴍᴍᴀɴᴅ ᴅɪʜᴀᴘᴜs*\n\n> 🗑️ `{cmd}` telah dihapus.")
}

const FAILED: &str = "❌ Gagal menghapus!\n\n\
     > Reply sticker yang ingin dihapus, atau\n\
     > Ketik nama command: `.delstickercmd menu`";

/// `commandName.toLowerCase().replace(/^\./, '')` — satu titik di depan saja.
fn clean_cmd(raw: &str) -> String {
    let lower = raw.to_lowercase();
    lower.strip_prefix('.').unwrap_or(&lower).to_string()
}

/// Nama command yang terdaftar, sesuai urutan key di store (persis urutan
/// iterasi `Object.entries` di JS).
fn registered(ctx: &Ctx) -> Vec<String> {
    ctx.setting_entries(STORE)
        .map(|(_, v)| {
            v.get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

/// Hash pertama yang punya `command == clean`, atau `None` (`findByCommand`).
fn find_hash(ctx: &Ctx, clean: &str) -> Option<String> {
    ctx.setting_entries(STORE)
        .find(|(_, v)| v.get("command").and_then(|c| c.as_str()) == Some(clean))
        .map(|(k, _)| k.to_string())
}

#[plugin(
    name = "delstickercmd",
    alias = ["delcmdsticker", "removesticker", "unsticker"],
    category = "group",
    description = "Hapus sticker command",
    usage = ".delstickercmd <command> atau reply sticker",
    example = ".delstickercmd menu",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn delstickercmd(ctx: Ctx) -> Reply {
    let command_name = ctx.args.first().cloned();
    let quoted = ctx.quoted().is_some();

    if command_name.is_none() && !quoted {
        let names = registered(&ctx);
        return Reply::text(if names.is_empty() {
            EMPTY.to_string()
        } else {
            list_card(&names)
        });
    }

    // Cabang quoted tidak bisa di-port (lihat `ponytail:` modul) — hanya
    // jalur nama yang bisa menghapus apa pun.
    if let Some(raw) = &command_name {
        let clean = clean_cmd(raw);
        let Some(hash) = find_hash(&ctx, &clean) else {
            return Reply::text(not_found(&clean));
        };
        let mut next = ctx.setting(STORE).cloned().unwrap_or_default();
        if let Some(map) = next.as_object_mut() {
            map.remove(&hash);
        }
        return Reply::text(deleted_msg(&clean))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: STORE.into(),
                value: next,
            });
    }

    Reply::text(FAILED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(args: &[&str], settings: serde_json::Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delstickercmd".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("delstickercmd").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn store_after(r: &Reply) -> serde_json::Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, STORE);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn no_mutation(r: &Reply) -> bool {
        r.mutations().is_empty()
    }

    fn full() -> serde_json::Value {
        json!({"stickerCommands": {
            "hashaaa": {"command": "menu", "addedBy": "628111", "addedAt": 1},
            "hashbbb": {"command": "promo", "addedBy": "628222", "addedAt": 2}
        }})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delstickercmd").unwrap();
        assert_eq!(p.meta.name, "delstickercmd");
        assert_eq!(
            p.meta.alias,
            ["delcmdsticker", "removesticker", "unsticker"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Hapus sticker command");
        assert_eq!(p.meta.usage, ".delstickercmd <command> atau reply sticker");
        assert_eq!(p.meta.example, ".delstickercmd menu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["delcmdsticker", "removesticker", "unsticker"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("delstickercmd")
            );
        }
    }

    #[test]
    fn no_argument_lists_the_registered_commands_in_store_order() {
        let __balasan = run(&[], full());
        let t = body(&__balasan);
        assert!(t.starts_with("🖼️ *sᴛɪᴄᴋᴇʀ ᴄᴏᴍᴍᴀɴᴅs*"), "{t}");
        assert!(t.contains("┃ 🖼️ → `.menu`\n"), "{t}");
        assert!(t.contains("┃ 🖼️ → `.promo`\n"), "{t}");
        assert!(
            t.find(".menu").unwrap() < t.find(".promo").unwrap(),
            "urutan isi harus urutan store: {t}"
        );
        assert!(t.contains("> atau reply sticker + `.delstickercmd`"), "{t}");
        assert!(no_mutation(&run(&[], full())));
    }

    #[test]
    fn an_absent_store_lists_nothing_registered() {
        for s in [json!({}), json!({"stickerCommands": {}})] {
            let __balasan = run(&[], s.clone());
            let t = body(&__balasan);
            assert!(t.starts_with("🖼️ *sᴛɪᴄᴋᴇʀ ᴄᴏᴍᴍᴀɴᴅs*"), "{s}: {t}");
            assert!(t.contains("> Tidak ada sticker command yang terdaftar."), "{t}");
            assert!(t.contains("`.addcmdsticker`"), "{t}");
            assert!(no_mutation(&run(&[], s)));
        }
    }

    #[test]
    fn a_known_command_is_deleted_from_the_store() {
        let r = run(&["menu"], full());
        assert_eq!(body(&r), deleted_msg("menu"));
        // `SetSetting` menulis peta hash→entri itu sendiri sebagai nilai
        // setting, jadi tidak ada lapisan `stickerCommands` kedua.
        let after = store_after(&r);
        assert_eq!(after.as_object().unwrap().len(), 1, "satu entri harus tersisa");
        assert!(after.get("hashaaa").is_none(), "hash yang dihapus: {after}");
        assert!(after.get("hashbbb").is_some());
    }

    #[test]
    fn the_lookup_is_case_insensitive_and_strips_one_leading_dot() {
        assert_eq!(clean_cmd("Menu"), "menu");
        assert_eq!(clean_cmd(".PROMO"), "promo");
        assert_eq!(clean_cmd("menu."), "menu.");
        for input in ["MENU", ".Menu"] {
            let r = run(&[input], full());
            assert_eq!(body(&r), deleted_msg("menu"), "input {input}");
            assert_eq!(
                store_after(&r)["hashbbb"],
                json!({"command": "promo", "addedBy": "628222", "addedAt": 2}),
                "input {input}: `menu` yang terhapus, `promo` yang tersisa"
            );
        }
    }

    #[test]
    fn an_unknown_command_is_reported_and_nothing_is_written() {
        for input in ["tidakada", ".tidakada"] {
            let r = run(&[input], full());
            assert!(
                body(&r).starts_with("❌ Sticker command `tidakada` tidak ditemukan!"),
                "input {input}: {}",
                body(&r)
            );
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }
}
