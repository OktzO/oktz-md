//! `plugins/canvas/balogo.js` — logo Blue Archive dua kolom dari maker API.
//!
//! Balasan berupa gambar yang diambil dari URL maker; tidak ada HTTP dari
//! plugin ini — pipeline yang mengunduh dan meng-upload-nya.
//!
//! `ponytail:` `catch` di JS (`balogo.js:39-42`) hanya menempel pada
//! `sock.sendMedia`, satu-satunya operasi yang bisa melempar. Di sini media
//! cuma URL dan upload-nya dikerjakan pipeline, jadi cabang `☢` +
//! `src/lib/error.js` tidak punya sumber kegagalan dan tidak ditulis.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

const MAKER_URL: &str = "https://api.nexray.web.id/maker/balogo?text=";

/// `input.split(/[&,]/).map(s => s.trim()).filter(s => s)`
/// (`balogo.js:20`) — pemisah bisa `&` atau `,`, bagian kosong dibuang.
fn split_parts(input: &str) -> Vec<&str> {
    input
        .split(['&', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// `balogo.js:23`.
fn help(prefix: &str) -> String {
    format!(
        "🎮 *ʙʟᴜᴇ ᴀʀᴄʜɪᴠᴇ ʟᴏɢᴏ*\n\n\
         > Masukkan 2 teks untuk logo\n\n\
         > Contoh: {prefix}balogo Blue & Archive"
    )
}

#[plugin(
    name = "balogo",
    alias = ["bluearchivelogo"],
    category = "canvas",
    description = "Membuat logo Blue Archive style",
    usage = ".balogo <textL> & <textR>",
    example = ".balogo Blue & Archive",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn balogo(ctx: Ctx) -> Reply {
    let parts = split_parts(ctx.text.trim());
    if parts.len() < 2 {
        return Reply::text(help(&ctx.prefix));
    }

    // Spasi di antara kedua query yang di-encode itu literal di `balogo.js:33`.
    let url = format!(
        "{MAKER_URL}{} {}",
        encode_uri_component(parts[0]),
        encode_uri_component(parts[1])
    );
    Reply::image(MediaSource::url(url), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "balogo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("balogo").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("balogo").unwrap();
        assert_eq!(p.meta.name, "balogo");
        assert_eq!(p.meta.alias, ["bluearchivelogo"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.description, "Membuat logo Blue Archive style");
        assert_eq!(p.meta.usage, ".balogo <textL> & <textR>");
        assert_eq!(p.meta.example, ".balogo Blue & Archive");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["balogo", "bluearchivelogo"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("balogo"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn two_columns_produce_the_maker_url_without_a_caption() {
        let r = run("Blue & Archive");
        assert!(r.has_media());
        // `balogo.js:33` menyisipkan spasi LITERAL di antara dua hasil
        // `encodeURIComponent`, jadi kolom yang tanpa spasi tetap terpisah
        // oleh spasi kosong: `Blue` + " " + `Archive`.
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url(
                "https://api.nexray.web.id/maker/balogo?text=Blue Archive"
            ))
        );
        assert_eq!(body(&r), "", "caption `null` berarti kosong");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_comma_also_separates_the_two_columns() {
        let r = run("Blue, Archive");
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url(
                "https://api.nexray.web.id/maker/balogo?text=Blue Archive"
            ))
        );
    }

    #[test]
    fn only_the_first_two_usable_parts_are_used() {
        // `A &  & B & C` -> ["A","B","C"]; teks ketiga dibuang seperti JS.
        let r = run("A &  & B & C");
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url(
                "https://api.nexray.web.id/maker/balogo?text=A B"
            ))
        );
    }

    #[test]
    fn empty_or_single_column_input_shows_the_help_card() {
        for text in ["", "   ", "Blue", " & ", ",,,"] {
            let r = run(text);
            assert!(!r.has_media(), "{text:?} tidak boleh kirim gambar");
            assert_eq!(
                body(&r),
                "🎮 *ʙʟᴜᴇ ᴀʀᴄʜɪᴠᴇ ʟᴏɢᴏ*\n\n> Masukkan 2 teks untuk logo\n\n> Contoh: .balogo Blue & Archive",
                "{text:?}"
            );
        }
    }

    #[test]
    fn spasi_dalam_setiap_kolom_di_encode_sepenuhnya() {
        let r = run("Blue Archive & Blue Archive School");
        let Some(MediaSource::Url { url, .. }) = r.media_source() else {
            panic!("harus media URL");
        };
        // Tiap kolom di-encode utuh (`%20`), tapi spasi pemisah keduanya tetap
        // spasi mentah — itulah yang dilihat maker.
        assert_eq!(
            url,
            "https://api.nexray.web.id/maker/balogo?text=Blue%20Archive Blue%20Archive%20School"
        );
    }
}