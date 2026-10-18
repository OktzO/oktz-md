//! `plugins/tools/dafont.js` — cari dan download font dari DaFont.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔤 *DaFont Search*\n\nCari font dari DaFont, lalu reply nomor buat download.\n\n*PENGGUNAAN:*\n> *{prefix}dafont <nama font>*\n\n*CONTOH:*\n> *{prefix}dafont arial*\n> *{prefix}dafont horror*\n\n_Setelah daftar muncul, reply pesan bot dengan nomor font buat download file-nya_",
        prefix = prefix
    )
}

#[plugin(
    name = "dafont",
    alias = ["daffont", "carifont"],
    category = "tools",
    description = "Cari dan download font dari DaFont",
    usage = ".dafont <nama font>",
    example = ".dafont arial",
    cooldown = 10,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dafont(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `dafont.js` butuh:
    //   1. `DaFont(text)` dari `src/scraper/dafont.js` — tersedia sebagai
    //      `oktz_scraper::dafont::dafont(&ctx.http, &text)`, mengembalikan
    //      `{ status, results, count }`.
    //   2. `global.dafontSessions` — simpan hasil pencarian per sender untuk
    //      dibaca oleh `dafontAnswerEngine` di pesan berikutnya. Tidak ada
    //      mekanisme sesi interaktif yang bertahan antar pesan di `Ctx`.
    //   3. `sock.sendMedia(..., { type: 'image' })` untuk preview, dan
    //      `sock.sendMessage({ document: { url } })` untuk file zip — kirim
    //      media/document bisa `Reply::Image`/`Reply::Document`, tapi butuh
    //      byte atau URL yang sudah diketahui.
    //
    // `dafontAnswerHandler` juga adalah `answerHandler` (entry point kedua)
    // yang belum terdaftar di sini.
    //
    // `ponytail:` tambahkan sesi per sender (mis. lewat `ctx.session_set` /
    // `session_get` — tapi itu `Value` global, bukan per-plugin) dan
    // daftarkan `answer_handler` untuk nomor balasan.
    Reply::text(format!(
        "🔤 *DaFont — ? Font Ditemukan*\n\n> Pencarian: *{}*\n\n_(Hasil pencarian perlu sesi DaFont yang belum tersedia di Rust)_",
        text
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "dafont".into(),
            ..Default::default()
        }
    }

    #[test]
    fn tanpa_argumen_mengembalikan_bantuan() {
        let r = dafont(ctx_of(""));
        assert_eq!(r.text_body().unwrap(), help("."));
    }

    #[test]
    fn pencarian_tetap_mengembalikan_teks_tanpa_sesi() {
        let r = dafont(ctx_of("arial"));
        let t = r.text_body().unwrap();
        assert!(t.contains("DaFont"));
        assert!(t.contains("arial"));
    }
}
