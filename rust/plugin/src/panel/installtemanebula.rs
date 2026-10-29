//! `plugins/panel/installtemanebula.rs` — install tema Nebula (AtasBawahCantik)
//! lewat SSH.
//!
//! `ponytail:` sama seperti `installtemabilling.rs`: guard + validasi dipindah,
//! tiga `execSSH` (Node 22, Blueprint, `blueprint -install nebula`) tidak bisa
//! jalan tanpa klien SSH yang tidak ada di `Ctx`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::panel::installtemabilling::{BAD_FORMAT, parse_input, ssh_unavailable};

pub(crate) const USAGE: &str = "╭┈┈⬡「 🌌 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ɴᴇʙᴜʟᴀ* 」\n\
     ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}installtemanebula <ip>|<password>`\n\
     ╰┈┈⬡\n\n\
     > `Contoh: {prefix}installtemanebula 192.168.1.1|secretpass`";

#[plugin(
    name = "installtemanebula",
    alias = ["installthemanebula", "temanebula", "nebulatheme"],
    category = "panel",
    description = "Install tema Nebula (AtasBawahCantik) untuk panel Pterodactyl via SSH",
    usage = ".installtemanebula <ip>|<password>",
    example = ".installtemanebula 192.168.1.1|secretpass",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn installtemanebula(ctx: Ctx) -> Reply {
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
            command: "installtemanebula".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("installtemanebula").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("installtemanebula").unwrap().meta;
        assert_eq!(m.name, "installtemanebula");
        assert_eq!(
            m.alias,
            ["installthemanebula", "temanebula", "nebulatheme"]
        );
        assert_eq!(m.category, "panel");
        assert_eq!(
            m.description,
            "Install tema Nebula (AtasBawahCantik) untuk panel Pterodactyl via SSH"
        );
        assert_eq!(m.usage, ".installtemanebula <ip>|<password>");
        assert_eq!(m.example, ".installtemanebula 192.168.1.1|secretpass");
        assert_eq!(m.cooldown, 60);
        assert_eq!(m.energi, 0);
        assert!(m.is_owner);
        assert!(!m.is_private && !m.is_group && !m.is_premium);
        for a in ["installthemanebula", "temanebula", "nebulatheme"] {
            assert_eq!(find(a).unwrap().meta.name, "installtemanebula", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_nebula_usage_with_the_live_prefix() {
        for empty in ["", "   "] {
            let __balasan = run(empty);
            let t = text_of(&__balasan);
            assert!(t.starts_with("╭┈┈⬡「 🌌 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ɴᴇʙᴜʟᴀ* 」"), "{t}");
            assert!(t.contains("`.installtemanebula <ip>|<password>`"), "{t}");
            assert!(t.contains("Contoh: .installtemanebula 192.168.1.1|secretpass"), "{t}");
        }
    }

    #[test]
    fn a_single_field_is_rejected() {
        assert_eq!(text_of(&run("192.168.1.1")), BAD_FORMAT);
    }

    #[test]
    fn a_valid_pair_reports_the_unreachable_host_and_writes_nothing() {
        let r = run(" 10.0.0.5 | pw ");
        let t = text_of(&r);
        assert!(t.contains("tidak tersedia di port Rust"), "{t}");
        assert!(t.contains("10.0.0.5"), "IP harus tampil: {t}");
        assert!(r.mutations().is_empty());
    }
}