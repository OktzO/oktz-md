//! `plugins/search/pap.js` — kirim foto dari Pinterest sesuai tipe.
//!
//! `ponytail:` `resolver.resolve("pinterest", { q })` (`src/lib/resolve.js`)
//! belum di-port, jadi `fetch_urls` selalu `Err` dan yang menyala adalah
//! cabang `catch` biasa — pola yang sama dengan `download::pindl`. Logika
//! plugin sendiri dipindah semua: daftar query per tipe, validasi magic
//! number, batas ukuran, dan susunan pesan.
//!
//! `ponytail:` `prepareWAMessageMedia`, `generateWAMessageFromContent`,
//! `viewOnceMessage`, `nativeFlowMessage`, `sock.waUploadToServer`, dan
//! `sock.relayMessage` tidak punya padanan di `Reply`, jadi kartu interaktif
//! yang dikirim JS tidak ikut.

#![allow(dead_code)] // jalur unduh tidak terjangkau tanpa resolver

use oktz_plugin_api::{Ctx, HttpCall, Reply, plugin};


/// `QUERIES` (`pap.js:31-35`) — urutan query per tipe; yang pertama yang
/// menjawab dengan pin dipakai.
pub(crate) const QUERIES_CEWE: [&str; 3] = [
    "cewe cantik indonesia",
    "gadis cantik",
    "wanita indonesia cantik",
];
pub(crate) const QUERIES_COWO: [&str; 3] = [
    "cowo ganteng indonesia",
    "pria tampan",
    "cowok ganteng",
];
pub(crate) const QUERIES_FEMBOY: [&str; 3] = ["femboy", "femboy anime", "femboy aesthetic"];

/// `VALID_TYPES = Object.keys(QUERIES)` (`pap.js:37`) — urutan itu penting
/// karena juga menjadi daftar yang dicetak di pesan error.
pub(crate) const VALID_TYPES: [&str; 3] = ["cewe", "cowo", "femboy"];

/// `QUERIES[type] || []`.
pub(crate) fn queries_for(tipe: &str) -> &'static [&'static str] {
    match tipe {
        "cewe" => &QUERIES_CEWE,
        "cowo" => &QUERIES_COWO,
        "femboy" => &QUERIES_FEMBOY,
        _ => &[],
    }
}

/// `pap.js:146-149` — guard argumen. `m.args?.[0]` di-`trim().toLowerCase()`,
/// jadi spasi excess dan huruf besar tidak menggagalkan.
pub(crate) fn pesan_tipe_invalid() -> String {
    "❌ Pilih salah satu tipe pap yang tersedia: `cewe`, `cowo`, atau `femboy`.\n\nContoh: `.pap cewe`".to_string()
}

/// `pap.js:164` — hasil kosong yang sah.
pub(crate) fn kosong(arg: &str) -> String {
    format!("❌ Waduh, pap {arg} lagi kosong nih. Coba lagi nanti.")
}

/// `pap.js:190` — tidak ada satu pun URL yang bisa diunduh.
pub(crate) const GAMBAR_TIDAK_TERSEDIA: &str = "⚠️ Gambar tidak tersedia. Coba lagi nanti.";

/// `pap.js:201` — `prepareWAMessageMedia` tidak mengembalikan `imageMessage`.
pub(crate) const GAGAL_MENYIAPKAN: &str = "⚠️ Gagal menyiapkan gambar. Coba lagi nanti.";

/// `pap.js:179` — ambang ukuran minimum, terpisah dari magic number supaya
/// both bisa diuji tanpa buffer sungguhan.
pub(crate) const MIN_BYTES: usize = 1000;

/// `isImage(buf)` (`pap.js:43-62`) — magic number JPEG/PNG/WebP.
pub(crate) fn is_image(buf: &[u8]) -> bool {
    if buf.len() < 4 {
        return false;
    }
    // JPEG: FF D8 FF
    if buf[0] == 0xff && buf[1] == 0xd8 && buf[2] == 0xff {
        return true;
    }
    // PNG: 89 50 4E 47
    if buf[0] == 0x89 && buf[1] == 0x50 && buf[2] == 0x4e && buf[3] == 0x47 {
        return true;
    }
    // WebP: "RIFF" .... "WEBP"
    if buf.len() >= 12 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
        return true;
    }
    false
}

/// Gabungan kedua syarat di `pap.js:179`.
pub(crate) fn buffer_terima(buf: &[u8]) -> bool {
    buf.len() > MIN_BYTES && is_image(buf)
}

/// `fetchImageBuffer` (`pap.js:65-72`) — Pinterest menolak tanpa `Referer`.
pub(crate) fn download_request(url: &str) -> HttpCall {
    HttpCall::get(url).header("Referer", "https://www.pinterest.com/")
}

/// `fetchImageUrls` (`pap.js:93-117`) — lapisan kapabilitas belum di-port,
/// jadi selalu melempar. `Err` berarti tidak ada satu pun kapabilitas yang
/// menjawab, persis kondisi yang membuat JS melempar `alasanGagal`.
fn fetch_urls(_tipe: &str) -> Result<Vec<String>, ()> {
    Err(())
}

#[plugin(
    name = "pap",
    alias = ["papcewe", "papcowo", "papfemboy"],
    category = "search",
    description = "Minta pap cewe, cowo, atau femboy dari Pinterest",
    usage = ".pap <cewe/cowo/femboy>",
    example = ".pap cewe",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pap(ctx: Ctx) -> Reply {
    // `typeof raw === "string" ? raw.trim().toLowerCase() : ""` (`:143-144`).
    // `m.args` di `Ctx` selalu `Vec<String>`, jadi non-string tidak mungkin.
    let arg = ctx.args.first().map(|a| a.trim().to_lowercase()).unwrap_or_default();

    if !VALID_TYPES.contains(&arg.as_str()) {
        // `safeReply` menelan error, jadi balasan ini selalu terkirim.
        return Reply::text(pesan_tipe_invalid());
    }

    // `safeReact(m, "🕕")` di `:153` — notifikasi opsional, tanpa teks.
    match fetch_urls(&arg) {
        // `pap.js:161-169` — hasil kosong yang sah.
        Ok(urls) if urls.is_empty() => Reply::text(kosong(&arg)).with_reaction("❌"),
        // `pap.js:171-186` — URL diacak lalu diunduh satu per satu; hanya
        // buffer > 1000 byte dengan magic number JPEG/PNG/WebP yang diterima.
        // Unduhan dan pengiriman/viewOnce tidak ada padanannya di `Reply`.
        Ok(_urls) => Reply::text(GAMBAR_TIDAK_TERSEDIA).with_reaction("❌"),
        // `pap.js:266-273` — `CapabilityError` jadi template error biasa;
        // `m.pushName` tidak ada di `Ctx` sehingga tetap "User".
        Err(()) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pap".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("pap").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pap").unwrap();
        assert_eq!(p.meta.name, "pap");
        assert_eq!(p.meta.alias, ["papcewe", "papcowo", "papfemboy"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Minta pap cewe, cowo, atau femboy dari Pinterest"
        );
        assert_eq!(p.meta.usage, ".pap <cewe/cowo/femboy>");
        assert_eq!(p.meta.example, ".pap cewe");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["papcewe", "papcowo", "papfemboy"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pap");
        }
    }

    #[test]
    fn an_unknown_type_prints_the_three_valid_ones() {
        assert_eq!(
            text_of(&run("ngawur")),
            "❌ Pilih salah satu tipe pap yang tersedia: `cewe`, `cowo`, atau `femboy`.\n\nContoh: `.pap cewe`"
        );
        assert_eq!(text_of(&run("")), text_of(&run("ngawur")));
        assert_eq!(text_of(&run("   ")), text_of(&run("ngawur")));
        assert_eq!(reaction_of(&run("ngawur")), None, "guard sebelum reaksi");
    }

    #[test]
    fn a_valid_type_reaches_the_error_branch_while_the_resolver_is_missing() {
        for tipe in VALID_TYPES {
            let r = run(tipe);
            assert_eq!(text_of(&r), crate::common::error_template(".", "pap"), "tipe {tipe}");
            assert_eq!(reaction_of(&r), Some("☢"), "tipe {tipe}");
        }
    }

    #[test]
    fn odd_and_very_long_types_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", &"c".repeat(5000)] {
            let r = run(input);
            assert!(!text_of(&r).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn the_three_types_keep_their_query_lists() {
        assert_eq!(queries_for("cewe"), &QUERIES_CEWE);
        assert_eq!(queries_for("cowo"), &QUERIES_COWO);
        assert_eq!(queries_for("femboy"), &QUERIES_FEMBOY);
        assert!(queries_for("lain").is_empty());
        assert_eq!(VALID_TYPES, ["cewe", "cowo", "femboy"]);
    }

    #[test]
    fn magic_numbers_accept_only_jpeg_png_and_webp() {
        let jpeg = [0xffu8, 0xd8, 0xff, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0];
        let png = [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0];
        let mut webp = [0u8; 12];
        webp[0..4].copy_from_slice(b"RIFF");
        webp[8..12].copy_from_slice(b"WEBP");
        let gif = [0x47u8, b'I', b'F', b'8', b'9', b'a', 0, 0, 0, 0, 0, 0];
        let html = [0x3cu8, b'!', b'D', b'O', b'C', b'T', b'Y', b'P', b'E', 0, 0, 0];

        assert!(is_image(&jpeg));
        assert!(is_image(&png));
        assert!(is_image(&webp));
        assert!(!is_image(&gif), "GIF bukan salah satu dari tiga");
        assert!(!is_image(&html), "halaman error CDN ditolak");
        assert!(!is_image(&[]));
        assert!(!is_image(&[0x89, b'P', b'N']), "kurang 4 byte");
    }

    #[test]
    fn the_buffer_gate_also_requires_more_than_a_thousand_bytes() {
        // Magic number JPEG di awal, sisanya isian apa saja.
        let kecil = {
            let mut b = vec![b'x'; MIN_BYTES];
            b[0..3].copy_from_slice(&[0xff, 0xd8, 0xff]);
            b
        };
        assert!(is_image(&kecil));
        assert!(!buffer_terima(&kecil), "tepat 1000 byte ditolak");
        let mut besar = kecil.clone();
        besar.push(b'x');
        assert!(buffer_terima(&besar), "1001 byte diterima");
        assert!(!buffer_terima(&vec![b'x'; 5000]), "bukan gambar");
    }

    #[test]
    fn the_empty_and_prepare_failure_messages_are_kept() {
        assert_eq!(
            kosong("cewe"),
            "❌ Waduh, pap cewe lagi kosong nih. Coba lagi nanti."
        );
        assert_eq!(
            GAMBAR_TIDAK_TERSEDIA,
            "⚠️ Gambar tidak tersedia. Coba lagi nanti."
        );
        assert_eq!(
            GAGAL_MENYIAPKAN,
            "⚠️ Gagal menyiapkan gambar. Coba lagi nanti."
        );
    }

    #[test]
    fn the_pinterest_request_carries_the_referer_header() {
        let call = download_request("https://i.pinimg.com/originals/x.jpg");
        assert_eq!(call.method, oktz_plugin_api::HttpMethod::Get);
        assert!(call
            .headers
            .iter()
            .any(|(k, v)| k == "Referer" && v == "https://www.pinterest.com/"));
    }
}