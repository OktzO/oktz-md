//! `plugins/panel/uninstalltema.js` — uninstall tema Pterodactyl lewat SSH.
//!
//! `ponytail:` `pluginConfig.name` di JS adalah array
//! `['uinstalltema','uninstalltema','removetema','hapustema']` dengan `alias: []`,
//! jadi elemen pertama jadi `name` dan sisanya jadi `alias` — pola yang sama
//! dengan `group/autodl.rs`.
//!
//! Yang dipindah: kedua guard, pembacaan `ip|password`, dan penulisan
//! `global.installtema = { vps, pwvps }` (saya di sini `Ctx::session_set`).
//! `ress.connect()` + `exec` + tulis `skyzodev/2/y/x` ke stream butuh klien SSH
//! yang tidak ada di `Ctx`. `parse_input` dipakai dari `installtemabilling.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::json;

use crate::panel::installtemabilling::{BAD_FORMAT, parse_input};

/// Kunci global yang diisi JS — dipakai plugin lain sebagai sumber kredensial.
pub(crate) const SESSION_KEY: &str = "installtema";

pub(crate) const USAGE: &str = "╭┈┈⬡「 🗑️ *ᴜɴɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ* 」\n\
     ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}uinstalltema <ip>|<password>`\n\
     ╰┈┈⬡\n\n\
     > `Contoh: {prefix}uinstalltema 192.168.1.1|secretpass`";

#[plugin(
    name = "uinstalltema",
    alias = ["uninstalltema", "removetema", "hapustema"],
    category = "panel",
    description = "Uninstall tema Pterodactyl via SSH",
    usage = ".uinstalltema <ip>|<password>",
    example = ".uinstalltema 192.168.1.1|secretpass",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn uninstalltema(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(USAGE.replace("{prefix}", &ctx.prefix));
    }
    let Some((ipvps, passwd)) = parse_input(text) else {
        return Reply::text(BAD_FORMAT);
    };

    // `global.installtema = { vps: ipvps, pwvps: passwd }`
    ctx.session_set(SESSION_KEY, json!({ "vps": ipvps, "pwvps": passwd }));

    Reply::text(format!(
        "⚠️ *ᴜɴɪɴsᴛᴀʟʏ ᴛᴇᴍᴀ* tidak tersedia di port Rust\n\n\
         > `ssh2` tidak ada di `Ctx`, jadi VPS `{ipvps}` tidak bisa disentuh.\n\
         > Kredensial sudah disimpan di session `{SESSION_KEY}`."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = ctx_for(text);
        (find("uinstalltema").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    fn ctx_for(text: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "uinstalltema".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("uinstalltema").unwrap().meta;
        // JS `name` adalah array; elemen pertama jadi name, sisanya alias.
        assert_eq!(m.name, "uinstalltema");
        assert_eq!(m.alias, ["uninstalltema", "removetema", "hapustema"]);
        assert_eq!(m.category, "panel");
        assert_eq!(m.description, "Uninstall tema Pterodactyl via SSH");
        assert_eq!(m.usage, ".uinstalltema <ip>|<password>");
        assert_eq!(m.example, ".uinstalltema 192.168.1.1|secretpass");
        assert_eq!(m.cooldown, 60);
        assert_eq!(m.energi, 0);
        assert!(m.is_owner);
        assert!(!m.is_private && !m.is_group && !m.is_premium);
        for a in ["uninstalltema", "removetema", "hapustema"] {
            assert_eq!(find(a).unwrap().meta.name, "uinstalltema", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_uninstall_usage() {
        for empty in ["", "   "] {
            let __balasan = run(empty.clone());
            let t = text_of(&__balasan);
            assert!(t.starts_with("╭┈┈⬡「 🗑️ *ᴜɴɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ* 」"), "{t}");
            assert!(t.contains("`.uinstalltema <ip>|<password>`"), "{t}");
            assert!(t.contains("Contoh: .uinstalltema 192.168.1.1|secretpass"), "{t}");
        }
    }

    #[test]
    fn a_single_field_is_rejected() {
        assert_eq!(text_of(&run("192.168.1.1")), BAD_FORMAT);
    }

    #[test]
    fn a_valid_pair_is_written_to_the_session_before_replying() {
        let ctx = ctx_for(" 192.168.1.1 | secretpass ");
        let r = (find("uinstalltema").unwrap().handler)(ctx.clone());
        let t = text_of(&r);
        assert!(t.contains("tidak tersedia di port Rust"), "{t}");
        assert!(t.contains("192.168.1.1"), "{t}");
        assert!(r.mutations().is_empty(), "session bukan store mutation");

        // `global.installtema = { vps, pwvps }` dengan nilai yang sudah di-trim.
        let saved = ctx.session_get(SESSION_KEY).expect("session harus terisi");
        assert_eq!(saved, json!({ "vps": "192.168.1.1", "pwvps": "secretpass" }));
    }

    #[test]
    fn a_rejected_input_never_touches_the_session() {
        let ctx = ctx_for("192.168.1.1");
        (find("uinstalltema").unwrap().handler)(ctx.clone());
        assert!(
            ctx.session_get(SESSION_KEY).is_none(),
            "password tidak lengkap tidak boleh disimpan"
        );
    }

    // Perintah dan input stream yang belum bisa dieksekusi — dijaga agar
    // port SSH nanti tidak perlu membuka `uninstalltema.js` lagi.
    const UNINSTALL_CMD: &str =
        "bash <(curl -s https://raw.githubusercontent.com/veryLinh/Theme-Autoinstaller/main/install.sh)";
    const STREAM_INPUTS: [&str; 4] = ["skyzodev", "2", "y", "x"];

    #[test]
    fn the_ssh_command_and_stream_answers_are_kept_for_the_future_port() {
        assert!(UNINSTALL_CMD.starts_with("bash <(curl -s "));
        assert!(UNINSTALL_CMD.contains("Theme-Autoinstaller/main/install.sh"));
        assert_eq!(STREAM_INPUTS, ["skyzodev", "2", "y", "x"]);
    }
}