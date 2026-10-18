//! `plugins/sticker/bratcewek.js` — membuat sticker brat.
//!
//! Catatan: `mod.rs` kategori `sticker` sudah memuat plugin `swm` yang bukan
//! milik tugas ini, jadi baris `pub mod bratcewek;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/sticker/mod.rs`.
//!
//! `generateBrat` / `parseBratArgs` dari `src/lib/brat.js` dan
//! `config.sticker` tidak punya padanan di `Ctx`. Handler memeriksa apakah
//! teks tersedia; jika ada, melaporkan bahwa sticker brat akan dibuat
//! (tanpa gambar sticker sebenarnya karena fungsi generate tidak tersedia
//! di konteks plugin).

use oktz_plugin_api::{Ctx, Reply, plugin};

fn bantuan(prefix: &str) -> String {
    format!(
        "🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n\
         > Masukkan teks\n\n\
         Contoh: {prefix}bratcewek Hai semua"
    )
}

#[plugin(
    name = "bratcewek",
    alias = ["cewekbrat", "bratperempuan", "bratgirl"],
    category = "sticker",
    description = "Membuat sticker brat",
    usage = ".bratcewek <text>",
    example = ".bratcewek Hai semua",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bratcewek(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(bantuan(&ctx.prefix));
    }

    // JS: parseBratArgs(text) + generateBrat({ text, theme: 'white',
    // bgColor: '#FFC0CB', blur }) → buffer → sendImageAsSticker.
    // Tidak ada padanan generateBrat di Ctx; melaporkan keberhasilan
    // simbolis tanpa media.
    Reply::text(format!(
        "🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n\
         > Teks diterima: *{text}*\n\
         > Sticker brat sedang diproses (generateBrat belum tersedia di konteks Rust)."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "bratcewek".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("bratcewek").unwrap();
        assert_eq!(p.meta.name, "bratcewek");
        assert_eq!(
            p.meta.alias,
            ["cewekbrat", "bratperempuan", "bratgirl"]
        );
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_teks_mengembalikan_bantuan() {
        let r = bratcewek(ctx());
        assert!(text_of(&r).contains("🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*"));
        assert!(text_of(&r).contains("Masukkan teks"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn dengan_teks_menerima_input() {
        let mut c = ctx();
        c.args = vec!["Hai".into(), "semua".into()];
        let r = bratcewek(c);
        assert!(text_of(&r).contains("Teks diterima: *Hai semua*"));
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["cewekbrat", "bratperempuan", "bratgirl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "bratcewek");
        }
    }
}
