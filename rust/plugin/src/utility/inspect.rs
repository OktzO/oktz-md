//! `plugins/utility/inspect.js` — info grup atau saluran WhatsApp via link.
//!
//! `ponytail:` `sock.groupGetInviteInfo` dan `sock.newsletterMsg` butuh I/O
//! Baileys yang tidak ada di `Ctx`; handler untuk teks tidak kosong diakhiri
//! dengan `error_template` + `☢`, pola `swm.rs`. Fungsi pemola pola (regex)
//! dan teks pakai di-port agar bisa diuji. Saat `sock` tersedia, tambahkan
//! `pub mod inspect;` di `src/utility/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `m.react('🔍')` tidak tersedia sebagai reaksi tunggal terpisah dari balasan;
// dicatat di `ponytail:` di atas.
/// Kartu pakai (`inspect.js:22-28`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🔍 *ɪɴsᴘᴇᴄᴛ*\n\n\
         > Cek info grup atau saluran via link\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}inspect https://chat.whatsapp.com/xxx`\n\
         > `{prefix}inspect https://whatsapp.com/channel/xxx`"
    )
}

/// `text.match(grupPattern)[1]` — kode undangan grup, `None` kalau tidak cocok.
#[cfg(test)]
pub(crate) fn kode_undangan_grup(text: &str) -> Option<String> {
    // Regex sederhana tanpa dependency: cari `chat.whatsapp.com/` lalu ambil
    // token [\w\d]* setelahnya, sama seperti `([\w\d]*)`.
    let marker = "chat.whatsapp.com/";
    let pos = text.find(marker)?;
    let setelah = &text[pos + marker.len()..];
    let mut kode = String::new();
    for c in setelah.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            kode.push(c);
        } else {
            break;
        }
    }
    if kode.is_empty() {
        None
    } else {
        Some(kode)
    }
}

/// `text.match(saluranPattern)[1]` — ID saluran, `None` kalau tidak cocok.
#[cfg(test)]
pub(crate) fn id_saluran(text: &str) -> Option<String> {
    let marker = "whatsapp.com/channel/";
    let pos = text.find(marker)?;
    let setelah = &text[pos + marker.len()..];
    let mut id = String::new();
    for c in setelah.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            id.push(c);
        } else {
            break;
        }
    }
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// `!isNaN(text)` (`inspect.js:88`) — `Number(text)` bukan `NaN`. String kosong
/// dan spasi jadi 0 di JS, jadi ikut `true`.
#[cfg(test)]
pub(crate) fn adalah_angka_js(text: &str) -> bool {
    let bersih = text.trim();
    if bersih.is_empty() {
        return true;
    }
    bersih.parse::<f64>().is_ok()
}

/// Saluran valid: cocok pola, atau berakhir `@newsletter`, atau angka (`inspect.js:88`).
#[cfg(test)]
pub(crate) fn saluran_valid(text: &str) -> bool {
    id_saluran(text).is_some()
        || text.ends_with("@newsletter")
        || adalah_angka_js(text)
}

#[plugin(
    name = "inspect",
    alias = ["cekgrup", "ceksaluran", "channelinfo"],
    category = "utility",
    description = "Inspect info grup atau saluran WhatsApp via link",
    usage = ".inspect <link grup/saluran>",
    example = ".inspect https://chat.whatsapp.com/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn inspect(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    // `sock.groupGetInviteInfo` / `sock.newsletterMsg` tidak tersedia di Ctx.
    Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        // JS `getPlugin` mengecek map `commands` (nama) SEBELUM `aliases`
        // (`src/lib/plugins.js:334-341`), jadi `.inspect` menunjuk plugin
        // ini walau `q` juga memakai `inspect` sebagai alias
        // (`plugins/owner/q.js:4`). `find()` Rust mencocokkan nama+alias urut
        // registrasi, jadi lookup di sini harus lewat nama.
        let p = oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|p| p.meta.name == "inspect")
            .unwrap();
        assert_eq!(p.meta.name, "inspect");
        assert_eq!(p.meta.alias, ["cekgrup", "ceksaluran", "channelinfo"]);
        assert_eq!(p.meta.category, "utility");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_text_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "inspect".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = inspect(ctx);
        let teks = r.without_mutations().text_body().unwrap();
        assert_eq!(teks, pakai("."));
    }

    #[test]
    fn kode_undangan_grup_ambil_token_setelah_marker() {
        assert_eq!(
            kode_undangan_grup("https://chat.whatsapp.com/Abc123"),
            Some("Abc123".to_string())
        );
        assert_eq!(
            kode_undangan_grup("https://chat.whatsapp.com/Abc123&extra"),
            Some("Abc123".to_string())
        );
        assert_eq!(kode_undangan_grup("https://example.com/x"), None);
        assert_eq!(kode_undangan_grup("chat.whatsapp.com/"), None);
    }

    #[test]
    fn id_saluran_ambil_token_setelah_marker() {
        assert_eq!(
            id_saluran("https://whatsapp.com/channel/0029Vb8z7S0KrWQvCa8i0J0I"),
            Some("0029Vb8z7S0KrWQvCa8i0J0I".to_string())
        );
        assert_eq!(id_saluran("https://chat.whatsapp.com/x"), None);
    }

    #[test]
    fn saluran_valid_miripkan_js() {
        assert!(saluran_valid("https://whatsapp.com/channel/0029Vb8z7S0KrWQvCa8i0J0I"));
        assert!(saluran_valid("120363411617217441@newsletter"));
        assert!(saluran_valid("123"));
        assert!(saluran_valid(""));
        assert!(!saluran_valid("abc"));
        assert!(!saluran_valid("https://chat.whatsapp.com/xxx"));
    }

    #[test]
    fn adalah_angka_js_serupa_global_is_nan() {
        assert!(adalah_angka_js("123"));
        assert!(adalah_angka_js(""));
        assert!(adalah_angka_js(" "));
        assert!(!adalah_angka_js("abc"));
        assert!(!adalah_angka_js("12abc"));
    }

    #[test]
    fn link_grup_atau_saluran_tanpa_sock_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "inspect".into(),
            text: "https://chat.whatsapp.com/Abc123".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = inspect(ctx);
        let teks = r.without_mutations().text_body().unwrap();
        assert_eq!(teks, crate::common::error_template_bawaan(".", "inspect"));
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            ),
            "harus bereaksi ☢"
        );
    }
}
