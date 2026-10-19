//! `plugins/sticker/bratvid.js` — sticker brat animated (MP4).
//!
//! Yang diport: kartu pakai, penjagaan teks kosong, dan jalur gagal.
//!
//! `ponytail:` render (`brat-canvas/video`, `bratvid.js:12-17` + `:58-61`)
//! tidak punya padanan di Rust: tidak ada `bratVid`, tidak ada API-nya, dan
//! `oktz_plugin_api::ffmpeg` cuma menjalankan ffmpeg — tanpa bingkai video
//! tidak ada yang bisa diremux. Jadi jalur teks berakhir di `error_template`
//! + `☢` (`bratvid.js:67-69`), sama seperti `sticker/attp2.rs`. Reaksi `🕕`
//! (`bratvid.js:50`) tidak dikirim: `with_reaction` hanya membawa satu reaksi
//! dan JS menimpanya dengan `☢` (`bratvid.js:68`). Saat `bratVid` tersedia:
//! render → `Reply::video(...)` + `✅`. Gerbang RAM `bratvidRssBlocked`
//! (`src/lib/bratvid-gate.js:5-8`) tidak ikut karena `Ctx` tidak punya RSS
//! proses, dan di Rust tidak ada import `brat-canvas` 80MB yang perlu ditahan
//! (alasan gerbang itu ada di JS). Berkas sementara `fs`/`os.tmpdir`
//! (`bratvid.js:52,61,71`) juga tidak perlu karena byte tidak pernah ditulis
//! ke disk. `isBratVidLoaded` (`bratvid.js:19-21`) tidak punya padanan:
//! tidak ada modul yang di-lazy-load. `packname`/`author` (`config.sticker`)
//! tidak ada di `Reply::Video`.
//!
//! `NOTE (mod.rs):` tambahkan `pub mod bratvid;` ke `sticker/mod.rs` secara
//! manual kalau berkas ini didaftarkan (mod.rs bukan milik tugas ini).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Kartu pakai saat teks kosong (`bratvid.js:42`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎬 *ʙʀᴀᴛ ᴀɴɪᴍᴀᴛᴇᴅ*\n\n\
         > Masukkan teks\n\n\
         `Contoh: {prefix}bratvid Hai semua`"
    )
}

#[plugin(
    name = "bratvid",
    alias = ["bratgif", "bratvideo"],
    category = "sticker",
    description = "Membuat sticker brat animated",
    usage = ".bratvid <text>",
    example = ".bratvid Hai semua",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bratvid(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    // Render `brat-canvas/video` (`bratvid.js:58`) belum ada di Rust —
    // lihat `ponytail:` di atas.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
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
        let p = oktz_plugin_api::find("bratvid").unwrap();
        assert_eq!(p.meta.name, "bratvid");
        assert_eq!(p.meta.alias, ["bratgif", "bratvideo"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.description, "Membuat sticker brat animated");
        assert_eq!(p.meta.usage, ".bratvid <text>");
        assert_eq!(p.meta.example, ".bratvid Hai semua");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["bratgif", "bratvideo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "bratvid");
        }
    }

    #[test]
    fn tanpa_teks_kartu_pakai_lengkap_dengan_backtick_js() {
        let r = bratvid(ctx_of("bratvid", ""));
        assert_eq!(
            teks(&r),
            "🎬 *ʙʀᴀᴛ ᴀɴɪᴍᴀᴛᴇᴅ*\n\n> Masukkan teks\n\n`Contoh: .bratvid Hai semua`"
        );
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn kartu_pakai_mengikuti_prefix() {
        assert_eq!(
            pakai("!"),
            "🎬 *ʙʀᴀᴛ ᴀɴɪᴍᴀᴛᴇᴅ*\n\n> Masukkan teks\n\n`Contoh: !bratvid Hai semua`"
        );
    }

    #[test]
    fn dengan_teks_jatuh_ke_error_template_dan_reaksi_bahu() {
        let r = bratvid(ctx_of("bratvid", "Hai semua"));
        assert_eq!(teks(&r), crate::common::error_template(".", "bratvid"));
        assert_eq!(reaksi(&r), Some("☢"));
    }
}
