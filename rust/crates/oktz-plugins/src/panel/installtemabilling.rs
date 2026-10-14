//! `plugins/panel/installtemabilling.js` — install tema Billing Pterodactyl
//! lewat SSH.
//!
//! `ponytail:` hanya guard + validasinya yang dipindah. Langkah JS setelah
//! `m.react('🕕')` — `new ssh2.Client()`, `.connect()`, tiga `execSSH` — butuh
//! soket TCP yang tidak ada di `Ctx`, jadi tidak bisa di-port; balasannya
//! gantinya memberi tahu host mana yang tidak bisa disentuh. `parse_input` dipakai
//! juga oleh `uninstalltema.rs` (lihat laporan batch).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const USAGE: &str = "╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ʙɪʟʟɪɴɢ* 」\n\
     ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}installtemabilling <ip>|<password>`\n\
     ╰┈┈⬡\n\n\
     > `Contoh: {prefix}installtemabilling 192.168.1.1|secretpass`";

pub(crate) const BAD_FORMAT: &str = "❌ Format salah! Gunakan: `ip|password`";

/// `text.split('|')` — JS tidak melimitasi jumlah bagian, hanya mensyaratkan
/// minimal dua. bagianbeyond index 1 diabaikan.
pub(crate) fn parse_input(text: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() < 2 {
        return None;
    }
    Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

/// Balasan untuk langkah yang tidak bisa di-port: format sudah sah, IP sudah
/// dibaca, tapi tidak ada klien SSH di `Ctx`.
pub(crate) fn ssh_unavailable(ip: &str) -> String {
    format!(
        "⚠️ *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ʙɪʟʟɪɴɢ* tidak tersedia di port Rust\n\n\
         > `ssh2` tidak ada di `Ctx`, jadi VPS `{ip}` tidak bisa disentuh.\n\
         > Format `ip|password` sudah benar."
    )
}

#[plugin(
    name = "installtemabilling",
    alias = ["installthemabilling", "temabilling"],
    category = "panel",
    description = "Install tema Billing untuk panel Pterodactyl via SSH",
    usage = ".installtemabilling <ip>|<password>",
    example = ".installtemabilling 192.168.1.1|secretpass",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn installtemabilling(ctx: Ctx) -> Reply {
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
            command: "installtemabilling".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("installtemabilling").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("installtemabilling").unwrap().meta;
        assert_eq!(m.name, "installtemabilling");
        assert_eq!(m.alias, ["installthemabilling", "temabilling"]);
        assert_eq!(m.category, "panel");
        assert_eq!(
            m.description,
            "Install tema Billing untuk panel Pterodactyl via SSH"
        );
        assert_eq!(m.usage, ".installtemabilling <ip>|<password>");
        assert_eq!(m.example, ".installtemabilling 192.168.1.1|secretpass");
        assert_eq!(m.cooldown, 60, "SSH — cooldown panjang seperti di JS");
        assert_eq!(m.energi, 0);
        assert!(m.is_owner, "isOwner: true di JS");
        assert!(!m.is_private && !m.is_group && !m.is_premium);
        for a in ["installthemabilling", "temabilling"] {
            assert_eq!(
                find(a).unwrap().meta.name,
                "installtemabilling",
                "alias {a}"
            );
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_with_the_live_prefix() {
        for empty in ["", "   "] {
            let __balasan = run(empty.clone());
            let t = text_of(&__balasan);
            assert!(t.starts_with("╭┈┈⬡「 🎨 *ɪɴsᴛᴀʟʟ ᴛᴇᴍᴀ ʙɪʟʟɪɴɢ* 」"), "{t}");
            assert!(t.contains("`.installtemabilling <ip>|<password>`"), "{t}");
            assert!(t.contains("Contoh: .installtemabilling 192.168.1.1|secretpass"), "{t}");
        }
    }

    #[test]
    fn a_single_field_is_rejected_before_anything_else_happens() {
        for bad in ["192.168.1.1", "  1.2.3.4  "] {
            assert_eq!(text_of(&run(bad.clone())), BAD_FORMAT, "input {bad:?}");
        }
    }

    #[test]
    fn both_fields_are_trimmed_and_extra_parts_are_ignored() {
        assert_eq!(
            parse_input("  10.0.0.1 | secret |extra"),
            Some(("10.0.0.1".to_string(), "secret".to_string()))
        );
        assert_eq!(
            parse_input("10.0.0.1|"),
            Some(("10.0.0.1".to_string(), String::new())),
            "password kosong tetap format sah"
        );
        assert_eq!(parse_input("ip"), None);
    }

    #[test]
    fn a_valid_pair_names_the_host_that_cannot_be_reached() {
        let __balasan = run("192.168.1.1|secretpass");
        let t = text_of(&__balasan);
        assert!(t.contains("tidak tersedia di port Rust"), "{t}");
        assert!(t.contains("`ssh2` tidak ada di `Ctx`"), "{t}");
        assert!(t.contains("192.168.1.1"), "IP harus tampil: {t}");
    }

    #[test]
    fn the_plugin_never_writes_to_the_store() {
        assert!(run("192.168.1.1|secretpass").mutations().is_empty());
        assert!(run("").mutations().is_empty());
        assert!(run("salah").mutations().is_empty());
    }
}