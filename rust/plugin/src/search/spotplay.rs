//! `plugins/search/spotplay.js` — putar musik dari Spotify.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "spotplay",
    alias = ["splay"],
    category = "search",
    description = "Putar musik dari Spotify",
    usage = ".spotplay <query>",
    example = ".spotplay neffex grateful",
    cooldown = 15,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn spotplay(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();
    if query.is_empty() {
        return Reply::text(format!(
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴜɪ*\n\n> `{prefix}spotplay <query>`",
            prefix = ctx.prefix
        ));
    }

    // `spotplay.js` perlu dua jalur:
    //  1. `resolver.resolve("spotify", { q: query })` — cari track, ambil yang
    //     url-nya `open.spotify.com/track/...`.
    //  2. `resolver.resolve("spotify", { url: <track url> })` — ambil download link.
    //
    // Di Rust, `oktz_scraper::spotify::download_spotify` hanya menyediakan
    // unduhan dari URL track yang sudah diketahui; pencarian per kata kunci
    // (`{ q }`) ada di capability resolver JS (`src/capabilities/spotify.js`)
    // yang belum di-port ke crate Rust. Tanpa search per kata kunci, plugin
    // ini tidak bisa di-port sepenuhnya.
    //
    // `ponytail:` tambahkan fungsi pencarian Spotify ke `oktz_scraper` (atau
    // panggil capability search lewat HTTP jika tersedia), lalu kirim audio
    // dengan `Reply::audio(MediaSource::url(download_url))`.
    Reply::text(format!(
        "❌ Lagu Spotify tidak ditemukan.\n\n`{prefix}spotplay <query>`",
        prefix = ctx.prefix
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
            command: "spotplay".into(),
            ..Default::default()
        }
    }

    #[test]
    fn query_kosong_mengembalikan_kartu_pakai() {
        let r = spotplay(ctx_of(""));
        let t = r.text_body().unwrap();
        assert!(t.contains("ᴄᴀʀᴀ ᴘᴀᴋᴜɪ"));
        assert!(t.contains("`.spotplay <query>`"));
    }

    #[test]
    fn query_isi_tetap_mengembalikan_tidak_ditemukan_belum_ada_search() {
        let r = spotplay(ctx_of("neffex grateful"));
        let t = r.text_body().unwrap();
        assert!(t.contains("Lagu Spotify tidak ditemukan"));
    }
}
