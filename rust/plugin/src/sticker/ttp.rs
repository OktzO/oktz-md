//! `plugins/sticker/ttp.js` — teks ke gambar/stiker (kanvas 512x512).
//!
//! Yang diport: kartu pakai saat teks kosong dan teks gagal dari JS.
//!
//! `ponytail:` render teks (`ttp.js:30-70`: `createCanvas`, `measureText`,
//! `fillText`, `canvas.encode("png")`) butuh rasterizer teks + font;
//! `oktz_plugin_api::gambar` hanya bisa resize/grayscale/encode. Jadi jalur
//! teks berakhir di teks error JS (`ttp.js:82`) + `☢`, sama seperti
//! `sticker/attp2.rs`. Reaksi `🕕` (`ttp.js:27`) tidak dikirim:
//! `with_reaction` hanya membawa satu reaksi dan JS menimpanya dengan `☢`
//! (`ttp.js:81`). `console.error("[TTP Maker]", err.message)` (`ttp.js:80`)
//! tidak punya padanan log di kontrak plugin. Saat rasterizer teks ada:
//! kanvas 512x512 → `Reply::sticker(...)` + `✅`. `packname`/`author`
//! (`config.sticker`) tidak ada di `Reply::Sticker`.
//!
//! Kategori plugin ini `maker` (`ttp.js:7`), tapi berkasnya ada di
//! `plugin/src/sticker/` mengikuti `plugins/sticker/ttp.js`.
//!
//! `NOTE (mod.rs):` tambahkan `pub mod ttp;` ke `sticker/mod.rs` secara
//! manual kalau berkas ini didaftarkan (mod.rs bukan milik tugas ini).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Kartu pakai saat teks kosong (`ttp.js:24`).
///
/// Prefix di JS TIDAK diinterpolasi — teksnya literal `.ttp`, bukan
/// `${m.prefix}ttp`, jadi biarkan apa adanya.
pub(crate) const PAKAI: &str = "❌ *Waduh, teksnya mana nih?*\n\n\
Kamu harus memasukkan teks yang ingin dijadikan stiker.\n\n\
Contoh: `.ttp Hai Cantik`";

/// Teks gagal render (`ttp.js:82`) — perhatikan spasi sebelum `\n` pertama,
/// JS menulis `* \n\n`.
pub(crate) const GAGAL: &str = "😔 *Terjadi masalah di sistem kami.* \n\n\
Sistem gagal menghubungi server pembuat stiker. Silakan coba beberapa saat lagi ya.";

#[plugin(
    name = "ttp",
    alias = ["texttopicture"],
    category = "maker",
    description = "Membuat stiker keren dari teks",
    usage = ".ttp <teks>",
    example = ".ttp Hai Cantik",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ttp(ctx: Ctx) -> Reply {
    // `m.args.join(" ") || m.text?.trim()` (`ttp.js:21`).
    let dari_args = ctx.args.join(" ");
    let text = if dari_args.is_empty() {
        ctx.text.trim().to_string()
    } else {
        dari_args
    };

    if text.is_empty() {
        return Reply::text(PAKAI);
    }

    // Rasterizer teks belum ada di Rust — lihat `ponytail:` di atas.
    Reply::text(GAGAL).with_reaction("☢")
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
        let p = oktz_plugin_api::find("ttp").unwrap();
        assert_eq!(p.meta.name, "ttp");
        assert_eq!(p.meta.alias, ["texttopicture"]);
        assert_eq!(p.meta.category, "maker");
        assert_eq!(p.meta.description, "Membuat stiker keren dari teks");
        assert_eq!(p.meta.usage, ".ttp <teks>");
        assert_eq!(p.meta.example, ".ttp Hai Cantik");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn alias_texttopicture_menunjuk_ke_plugin_ini() {
        assert_eq!(
            oktz_plugin_api::find("texttopicture").unwrap().meta.name,
            "ttp"
        );
    }

    #[test]
    fn tanpa_teks_kartu_pakai_literal_dot_ttp() {
        let r = ttp(ctx_of("ttp", ""));
        assert_eq!(
            teks(&r),
            "❌ *Waduh, teksnya mana nih?*\n\n\
             Kamu harus memasukkan teks yang ingin dijadikan stiker.\n\n\
             Contoh: `.ttp Hai Cantik`"
        );
        assert_eq!(reaksi(&r), None);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn kartu_pakai_tidak_mengikuti_prefix_karena_js_literal() {
        let mut c = ctx_of("ttp", "");
        c.prefix = "!".into();
        assert_eq!(teks(&ttp(c)), PAKAI);
        assert!(PAKAI.contains("`.ttp Hai Cantik`"));
    }

    #[test]
    fn teks_jatuh_ke_pesan_gagal_dan_reaksi_bahu() {
        let r = ttp(ctx_of("ttp", "Hai Cantik"));
        assert_eq!(
            teks(&r),
            "😔 *Terjadi masalah di sistem kami.* \n\n\
             Sistem gagal menghubungi server pembuat stiker. Silakan coba beberapa saat lagi ya."
        );
        assert_eq!(teks(&r), GAGAL);
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn fallback_ke_text_mentah_saat_args_kosong() {
        // `m.args.join(" ") || m.text?.trim()`: args kosong tapi `text` terisi
        // tetap dianggap ada teks.
        let mut c = ctx_of("ttp", "");
        c.text = "Hai Cantik".into();
        assert_eq!(teks(&ttp(c)), GAGAL);
    }
}
