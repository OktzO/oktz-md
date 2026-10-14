//! `plugins/panel/installtemaenigma.js` — install tema Enigma lewat SSH.
//!
//! `ponytail:` sama seperti dua installer tema lain: guard + validasi dipindah,
//! `execSSHInteractive` (yang menulis `AKSES TOKEN`, pilihan tema, lalu link
//! WA/group/channel) butuh klien SSH yang tidak ada di `Ctx`.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const USAGE: &str = "╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ᴇɴɪɢᴍᴀ* 」\n\
     ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}installtemaenigma <ip>|<password>|<link_wa>|<link_group>|<link_channel>`\n\
     ╰┈┈⬡\n\n\
     > Contoh:\n\
     > `{prefix}installtemaenigma 192.168.1.1|pass|https://wa.me/628xxx|https://t.me/group|https://t.me/channel`";

pub(crate) const BAD_FORMAT: &str =
    "❌ Format salah!\n\n> Gunakan: `ip|password|link_wa|link_group|link_channel`";

/// Lima bagian yang di-`trim`; `None` kalau kurang dari lima (JS:
/// `parts.length < 5`).
pub(crate) fn parse_input(text: &str) -> Option<[String; 5]> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() < 5 {
        return None;
    }
    Some([
        parts[0].trim().to_string(),
        parts[1].trim().to_string(),
        parts[2].trim().to_string(),
        parts[3].trim().to_string(),
        parts[4].trim().to_string(),
    ])
}

#[plugin(
    name = "installtemaenigma",
    alias = ["installthemaenigma", "temaenigma"],
    category = "panel",
    description = "Install tema Enigma untuk panel Pterodactyl via SSH",
    usage = ".installtemaenigma <ip>|<password>|<link_wa>|<link_group>|<link_channel>",
    example = ".installtemaenigma 192.168.1.1|secretpass|https://wa.me/628xxx|https://t.me/group|https://t.me/channel",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn installtemaenigma(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(USAGE.replace("{prefix}", &ctx.prefix));
    }
    let Some([ipvps, ..]) = parse_input(text) else {
        return Reply::text(BAD_FORMAT);
    };
    Reply::text(ssh_unavailable(&ipvps))
}

/// Bentuk pesan yang sama dengan installer lain, tapi teksnya menyebut Enigma.
fn ssh_unavailable(ip: &str) -> String {
    format!(
        "⚠️ *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ᴇɴɪɢᴍᴀ* tidak tersedia di port Rust\n\n\
         > `ssh2` tidak ada di `Ctx`, jadi VPS `{ip}` tidak bisa disentuh.\n\
         > Lima parameter sudah terbaca."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";
    const FIVE: &str = "10.0.0.1|pw|https://wa.me/628xxx|https://t.me/group|https://t.me/channel";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "installtemaenigma".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("installtemaenigma").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("installtemaenigma").unwrap().meta;
        assert_eq!(m.name, "installtemaenigma");
        assert_eq!(m.alias, ["installthemaenigma", "temaenigma"]);
        assert_eq!(m.category, "panel");
        assert_eq!(
            m.description,
            "Install tema Enigma untuk panel Pterodactyl via SSH"
        );
        assert_eq!(
            m.usage,
            ".installtemaenigma <ip>|<password>|<link_wa>|<link_group>|<link_channel>"
        );
        assert_eq!(
            m.example,
            ".installtemaenigma 192.168.1.1|secretpass|https://wa.me/628xxx|https://t.me/group|https://t.me/channel"
        );
        assert_eq!(m.cooldown, 60);
        assert_eq!(m.energi, 0);
        assert!(m.is_owner);
        assert!(!m.is_private && !m.is_group && !m.is_premium);
        for a in ["installthemaenigma", "temaenigma"] {
            assert_eq!(find(a).unwrap().meta.name, "installtemaenigma", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_five_parameter_usage() {
        for empty in ["", "  "] {
            let __balasan = run(empty.clone());
            let t = text_of(&__balasan);
            assert!(t.starts_with("╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ᴇɴɪɢᴍᴀ* 」"), "{t}");
            assert!(t.contains("`.installtemaenigma <ip>|<password>|<link_wa>|<link_group>|<link_channel>`"), "{t}");
            assert!(t.contains("> Contoh:"), "{t}");
        }
    }

    #[test]
    fn fewer_than_five_fields_is_the_only_format_error() {
        assert_eq!(text_of(&run("ip|pw")), BAD_FORMAT);
        assert_eq!(text_of(&run("ip|pw|wa|group")), BAD_FORMAT);
        assert_eq!(text_of(&run("ip")), BAD_FORMAT);
        assert!(!text_of(&run(FIVE.clone())).contains("Format salah"));
    }

    #[test]
    fn all_five_fields_are_trimmed_and_extra_ones_ignored() {
        let parsed = parse_input(" 10.0.0.1 | pw | wa | gr | ch | extra ").unwrap();
        assert_eq!(parsed, ["10.0.0.1", "pw", "wa", "gr", "ch"]);
        assert_eq!(parse_input("a|b|c|d").map(|p| p.len()), None);
    }

    #[test]
    fn a_valid_input_reports_the_unreachable_host_and_writes_nothing() {
        let r = run(FIVE.clone());
        let t = text_of(&r);
        assert!(t.contains("tidak tersedia di port Rust"), "{t}");
        assert!(t.contains("10.0.0.1"), "IP harus tampil: {t}");
        assert!(r.mutations().is_empty());
    }
}