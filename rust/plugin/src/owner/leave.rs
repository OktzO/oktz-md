//! `plugins/owner/leave.js` — bot keluar dari grup, via link atau chat aktif.
//!
//! `ponytail:` Tidak ada API grup (`sock.groupLeave`, `sock.groupGetInviteInfo`,
//! `m.groupMetadata`) yang diekspos ke plugin di registry Rust. Bagian grup
//! tidak dapat dicerminkan; handler di bawah hanya menangani parsing argumen
//! dan kasus "tidak ada grup yang ditargetkan", lalu jatuh ke error saat
//! operasi grup harus dieksekusi.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Ekstrak kode undangan dari teks, sama urutan pola dengan JS.
fn extract_invite_code(text: &str) -> Option<String> {
    // Tanpa crate regex, pemindaian manual: cari `chat.whatsapp.com/` atau
    // `wa.me/` diikuti minimal 20 karakter alfanumerik.
    for (pat, prefix_len) in [
        ("chat.whatsapp.com/", "chat.whatsapp.com/".len()),
        ("wa.me/", "wa.me/".len()),
    ] {
        let lower = text.to_ascii_lowercase();
        let mut from = 0;
        while let Some(i) = lower[from..].find(pat) {
            let start = from + i + prefix_len;
            let end = text[start..]
                .char_indices()
                .find(|(_, c)| !c.is_ascii_alphanumeric())
                .map_or(text.len() - start, |(n, _)| n);
            let kode = &text[start..start + end];
            if kode.len() >= 20 {
                return Some(kode.to_string());
            }
            from = start + end;
        }
    }
    None
}

/// Kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🚪 *ʟᴇᴀᴠᴇ ɢʀᴜᴘ*\n\n╭┈┈⬡「 📋 *ᴄᴀʀᴀ ᴘᴀᴋᴜ* 」\n┃ ◦ Di grup: `.leave`\n┃ ◦ Via link: `.leave <link>`\n╰┈┈┈┈┈┈┈┈⬡\n\n`Contoh: {prefix}leave https://chat.whatsapp.com/xxx`"
    )
}

#[plugin(
    name = "leave",
    alias = ["leavegrup", "leavegroup", "keluar"],
    category = "owner",
    description = "Bot keluar dari grup",
    usage = ".leave [link]",
    example = ".leave",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn leave(ctx: Ctx) -> Reply {
    let argumen = ctx.args.join(" ");
    let input = argumen.trim();

    if input.is_empty() {
        if ctx.is_group {
            // `ponytail:` `m.groupMetadata` dan `sock.groupLeave` tidak tersedia
            // di registry; grup aktif tidak dapat ditargetkan.
            return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
        }
        return Reply::text(help(&ctx.prefix));
    }

    let Some(_kode) = extract_invite_code(input) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Link invite tidak valid");
    };

    // `ponytail:` `sock.groupGetInviteInfo` dan `sock.groupLeave` tidak tersedia
    // di registry, jadi operasi grup tidak dapat dieksekusi.
    Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn ekstraksi_kode_undangan_dari_chat_whatsapp() {
        assert_eq!(
            extract_invite_code("https://chat.whatsapp.com/ABCDEFGHIJKLMNOPQRST").as_deref(),
            Some("ABCDEFGHIJKLMNOPQRST")
        );
        assert_eq!(
            extract_invite_code("https://chat.whatsapp.com/abcdefghijklmnopqrst").as_deref(),
            Some("abcdefghijklmnopqrst")
        );
    }

    #[test]
    fn ekstraksi_kode_undangan_dari_wa_me() {
        assert_eq!(
            extract_invite_code("https://wa.me/ABCDEFGHIJKLMNOPQRST").as_deref(),
            Some("ABCDEFGHIJKLMNOPQRST")
        );
    }

    #[test]
    fn kode_pendek_ditolak() {
        assert_eq!(extract_invite_code("https://chat.whatsapp.com/abc123"), None);
    }

    #[test]
    fn tanpa_input_di_bukan_grup_menghasilkan_kartu_pakai() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "leave".into(),
            is_group: false,
            ..Ctx::default()
        };
        let r = leave(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            help(".")
        );
        assert!(matches!(r, Reply::Text { .. }));
    }

    #[test]
    fn link_tidak_valid_menghasilkan_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "leave".into(),
            args: vec!["https://example.com".into()],
            ..Ctx::default()
        };
        let r = leave(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            "❌ *ɢᴀɢᴀʟ*\n\n> Link invite tidak valid"
        );
    }

    #[test]
    fn grup_aktif_tanpa_api_grup_jatuh_ke_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: "123456@g.us".into(),
            prefix: ".".into(),
            command: "leave".into(),
            is_group: true,
            ..Ctx::default()
        };
        let r = leave(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), crate::common::error_template_bawaan(".", "leave"));
        assert!(matches!(r.without_mutations(), Reply::Reaction { .. }));
    }
}
