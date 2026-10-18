//! `plugins/panel/installtemastellar.rs` — install tema Stellar lewat SSH.
//!
//! `ponytail:` guard + validasi dipindah; tiga `execSSH` (deps, `flavor.sh`,
//! build assets) butuh klien SSH yang tidak ada di `Ctx`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::panel::installtemabilling::{BAD_FORMAT, parse_input, ssh_unavailable};

pub(crate) const USAGE: &str = "╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ sᴛᴇʟʟᴀʀ* 」\n\
     ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}installtemastellar <ip>|<password>`\n\
     ╰┈┈⬡\n\n\
     > `Contoh: {prefix}installtemastellar 192.168.1.1|secretpass`";

#[plugin(
    name = "installtemastellar",
    alias = ["installthemastellar", "temastellar"],
    category = "panel",
    description = "Install tema Stellar untuk panel Pterodactyl via SSH",
    usage = ".installtemastellar <ip>|<password>",
    example = ".installtemastellar 192.168.1.1|secretpass",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn installtemastellar(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(USAGE.replace("{prefix}", &ctx.prefix));
    }
    let Some((ipvps, _passwd)) = parse_input(text) else {
        return Reply::text(BAD_FORMAT);
    };
    Reply::text(ssh_unavailable(&ipvps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "installtemastellar".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("installtemastellar").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("installtemastellar").unwrap().meta;
        assert_eq!(m.name, "installtemastellar");
        assert_eq!(m.alias, ["installthemastellar", "temastellar"]);
        assert_eq!(m.category, "panel");
        assert_eq!(
            m.description,
            "Install tema Stellar untuk panel Pterodactyl via SSH"
        );
        assert_eq!(m.usage, ".installtemastellar <ip>|<password>");
        assert_eq!(m.example, ".installtemastellar 192.168.1.1|secretpass");
        assert_eq!(m.cooldown, 60);
        assert_eq!(m.energi, 0);
        assert!(m.is_owner);
        assert!(!m.is_private && !m.is_group && !m.is_premium);
        for a in ["installthemastellar", "temastellar"] {
            assert_eq!(find(a).unwrap().meta.name, "installtemastellar", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_stellar_usage_with_the_live_prefix() {
        for empty in ["", " "] {
            let __balasan = run(empty);
            let t = text_of(&__balasan);
            assert!(t.starts_with("╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ sᴛᴇʟʟᴀʀ* 」"), "{t}");
            assert!(t.contains("`.installtemastellar <ip>|<password>`"), "{t}");
        }
    }

    #[test]
    fn a_single_field_is_rejected() {
        assert_eq!(text_of(&run("192.168.1.1")), BAD_FORMAT);
    }

    #[test]
    fn a_valid_pair_reports_the_unreachable_host_and_writes_nothing() {
        let r = run("192.168.1.1|secretpass");
        assert!(text_of(&r).contains("tidak tersedia di port Rust"));
        assert!(r.mutations().is_empty());
    }
}