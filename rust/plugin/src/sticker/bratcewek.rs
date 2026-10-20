//! `plugins/sticker/bratcewek.js` — sticker brat pink (`bgColor: '#FFC0CB'`).
//!
//! Yang diport: kartu pakai, penjagaan teks kosong, dan jalur gagal.
//!
//! `ponytail:` render `generateBrat` (`src/lib/brat.js:384-445`) butuh
//! `@napi-rs/canvas` + font ArialNarrow + peta emoji apple; `oktz_plugin_api::gambar`
//! tidak bisa menggambar teks. Jadi jalur teks berakhir di `error_template` +
//! `☢` (`bratcewek.js:37-40`) — sama seperti `sticker/attp2.rs`. Reaksi `🕕`
//! (`bratcewek.js:27`) tidak dikirim: `with_reaction` hanya membawa satu
//! reaksi, dan JS menimpanya dengan `☢` (`bratcewek.js:38`) — di WA reaksi
//! terakhir yang terlihat. Saat renderer teks ada, ganti cabang gagal dengan
//! `generateBrat({ text, theme: "white", bgColor: "#FFC0CB", blur })` →
//! `Reply::sticker(...)` + `✅`. `packname`/`author` (`config.sticker`) tidak
//! ada di `Reply::Sticker`.
//!
//! `NOTE (mod.rs):` tambahkan `pub mod bratcewek;` ke `sticker/mod.rs` secara
//! manual kalau berkas ini didaftarkan (mod.rs bukan milik tugas ini).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Kartu pakai saat teks kosong (`bratcewek.js:23-25`).
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n\
         > Masukkan teks\n\n\
         `Contoh: {prefix}bratcewek Hai semua`"
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

    // `parseBratArgs(text)` + `generateBrat({ bgColor: "#FFC0CB" })`
    // (`bratcewek.js:30-35`) belum bisa — lihat `ponytail:` di atas.
    Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(command: &str, text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: command.to_string(),
            ..Ctx::default()
        }
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("bratcewek").unwrap();
        assert_eq!(p.meta.name, "bratcewek");
        assert_eq!(p.meta.alias, ["cewekbrat", "bratperempuan", "bratgirl"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.description, "Membuat sticker brat");
        assert_eq!(p.meta.usage, ".bratcewek <text>");
        assert_eq!(p.meta.example, ".bratcewek Hai semua");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["cewekbrat", "bratperempuan", "bratgirl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "bratcewek");
        }
    }

    #[test]
    fn tanpa_teks_kartu_pakai_lengkap_dengan_backtick_js() {
        let r = bratcewek(ctx_of("bratcewek", ""));
        assert_eq!(
            teks(&r),
            "🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n> Masukkan teks\n\n`Contoh: .bratcewek Hai semua`"
        );
        assert_eq!(teks(&r), bantuan("."));
        assert_eq!(reaksi(&r), None);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn kartu_pakai_mengikuti_prefix() {
        assert_eq!(
            bantuan("!"),
            "🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n> Masukkan teks\n\n`Contoh: !bratcewek Hai semua`"
        );
    }

    #[test]
    fn dengan_teks_jatuh_ke_error_template_dan_reaksi_bahu() {
        let r = bratcewek(ctx_of("bratcewek", "Hai   semua"));
        assert_eq!(
            teks(&r),
            crate::common::error_template_bawaan(".", "bratcewek")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn error_template_memakai_command_yang_dipanggil() {
        let r = bratcewek(ctx_of("bratgirl", "Hai semua"));
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "bratgirl"));
        assert_eq!(reaksi(&r), Some("☢"));
    }
}
