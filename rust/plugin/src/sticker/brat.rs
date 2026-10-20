//! `plugins/sticker/brat.js` — menu variant brat dan generator sticker brat.
//!
//! Yang diport apa adanya: pembagian command menu vs generator, kartu pakai,
//! `parseBratArgs` (`src/lib/brat.js:466-472`), dan jalur API
//! `fetchBratFromAPI` (`src/lib/brat.js:447-463`) yang hasilnya dikirim
//! sebagai sticker (`brat.js:148-153`).
//!
//! `ponytail:` yang belum bisa, beserta alasannya:
//! - Menu `brat`/`bratmenu` (`brat.js:65-119`) dikirim sebagai kartu
//!   interaktif dengan `single_select` + footer persis JS; yang belum bisa
//!   hanya header gambarnya (`getAssetBuffer("foto")` +
//!   `prepareWAMessageMedia` — tidak ada uploader di `Reply`).
//! - Fallback `generateBrat` (canvas, `src/lib/brat.js:384-445`): tidak ada
//!   renderer teks di Rust. Saat API gagal atau gambarnya blank, alurnya
//!   berhenti di `error_template` + `☢` (`brat.js:154-157`), bukan render
//!   ulang. Tambahkan renderer teks + `is_blank` penuh saat canvas tersedia.
//! - `packname`/`author` (`config.sticker`) tidak ada di `Reply::Sticker`.
//!
//! `NOTE (mod.rs):` tambahkan `pub mod brat;` ke `sticker/mod.rs` secara
//! manual kalau berkas ini didaftarkan (mod.rs bukan milik tugas ini).

use oktz_plugin_api::registry::InteractiveButton;
use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

/// Caption menu (`brat.js:66-67`).
pub(crate) const CAPTION_MENU: &str =
    "🌿 *kamu mau buat brat yak, silahkan pilih variant brat tombol dibawah*";

/// `BRAT_VARIANTS` + `buildVariantRows` (`brat.js:24-63`) — isi baris
/// `single_select` menu.
pub(crate) fn baris_varian(prefix: &str, text: &str) -> Vec<(String, String, String)> {
    [
        ("Brat Default", "Sticker brat versi biasa", "bratimg"),
        ("Brat Green", "Variant brat warna hijau", "bratgreen"),
        ("Brat Cewek", "Variant brat cewek", "bratcewek"),
        ("Brat Anime", "Variant brat anime", "animebrat"),
        ("Brat Video", "Sticker brat animated", "bratvid"),
        ("Brat Video V2", "Sticker brat video v2", "bratvid2"),
    ]
    .iter()
    .map(|(judul, des, cmd)| {
        (
            (*judul).to_string(),
            format!("{des} • .{cmd} <text>"),
            format!("{prefix}{cmd} {text}").trim().to_string(),
        )
    })
    .collect()
}

/// `sendBratMenu` (`brat.js:65-119`) tanpa header gambar.
fn menu_brat(prefix: &str, text: &str) -> Reply {
    let rows: Vec<serde_json::Value> = baris_varian(prefix, text)
        .into_iter()
        .map(|(title, description, id)| {
            serde_json::json!({
                "title": title,
                "description": description,
                "id": id,
            })
        })
        .collect();

    Reply::interactive(CAPTION_MENU)
        .with_footer("Pilih variant brat favorit kamu")
        .with_button(InteractiveButton::new(
            "single_select",
            serde_json::json!({
                "title": "🌾 Pilih Variant Brat",
                "sections": [{
                    "title": "Variant Brat",
                    "rows": rows,
                }],
            }),
        ))
}

/// Kartu pakai saat teks kosong (`brat.js:130-134`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!("🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: {prefix}bratimg Hai semua`")
}

/// URL API (`src/lib/brat.js:448`).
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.theresav.eu/api/maker/brat?text={}",
        crate::tools::phisingataubukan::encode_uri_component(text)
    )
}

/// `parseBratArgs` (`src/lib/brat.js:466-472`) — `-blur N` (N = 0..3)
/// dibuang dari teks, N dikembalikan. Regex JS (`/-blur\s*([0-3])/`) tanpa
/// flag `g` mengambil kemunculan PALING KIRI yang cocok; kalau di situ
/// angkanya bukan 0..3, regex lanjut mencari `-blur` berikutnya. Flag tak
/// dikenal dibiarkan sebagai teks.
pub(crate) fn parse_brat_args(input: &str) -> (String, u8) {
    let text = input.trim();
    let mut dari = 0usize;
    while let Some(rel) = text[dari..].find("-blur") {
        let awal = dari + rel;
        let sisa = &text[awal + 5..];
        // `\s*` — spasi sebelum angka boleh nol atau lebih.
        let spasi = sisa.len() - sisa.trim_start().len();
        if let Some(d @ b'0'..=b'3') = sisa.as_bytes().get(spasi).copied() {
            let akhir = awal + 5 + spasi + 1;
            let bersih = format!("{}{}", &text[..awal], &text[akhir..])
                .trim()
                .to_string();
            return (bersih, d - b'0');
        }
        dari = awal + 5;
    }
    (text.to_string(), 0)
}

/// `isBlankImage` (`src/lib/brat.js:362-380`).
///
/// JS menyusutkan gambar ke 50x50 lalu membandingkan seluruh piksel; itu tidak
/// mungkin di sini (`oktz_plugin_api::gambar::Gambar` tidak mengekspos data
/// mentah). Yang ditiru: buffer di bawah 100 byte (`brat.js:363`) dan payload
/// yang bukan gambar — padanan cabang `catch` JS ("bukan gambar valid →
/// anggap blank/rusak", `brat.js:377-379`).
pub(crate) fn is_blank(buf: &[u8]) -> bool {
    if buf.len() < 100 {
        return true;
    }
    let webp = buf.starts_with(b"RIFF") && buf.len() >= 12 && buf[8..].starts_with(b"WEBP");
    !(buf.starts_with(b"\x89PNG\r\n\x1a\n")
        || buf.starts_with(&[0xFF, 0xD8, 0xFF])
        || webp
        || buf.starts_with(b"GIF87a")
        || buf.starts_with(b"GIF89a"))
}

#[plugin(
    name = "brat",
    alias = ["bratmenu", "bratimg", "brattext"],
    category = "sticker",
    description = "Menu variant brat dan generator sticker brat",
    usage = ".brat | .bratimg <text>",
    example = ".bratimg Hai semua",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn brat(ctx: Ctx) -> Reply {
    let command = ctx.command.to_lowercase();
    if command == "brat" || command == "bratmenu" {
        return menu_brat(&ctx.prefix, &ctx.text);
    }

    if ctx.text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    // `blur` hanya dipakai `generateBrat` (canvas), jadi tidak diteruskan ke
    // API — sama seperti JS yang memanggil `fetchBratFromAPI(parsed)`.
    let (parsed, _blur) = parse_brat_args(&ctx.text);

    let Ok(res) = ctx.http.get(&api_url(&parsed)) else {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    };
    // `if (!res.ok) throw` (`src/lib/brat.js:455`).
    if !(200..300).contains(&res.status) {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    }
    let buf = res.body.bytes().to_vec();
    // `if (await isBlankImage(buffer)) throw new Error("API returned blank
    // image")` (`brat.js:143`) — di JS memicu `generateBrat`, di sini tidak
    // ada padanannya.
    if is_blank(&buf) {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    }

    Reply::sticker(MediaSource::bytes(buf)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        status: u16,
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn ok(bytes: &[u8]) -> Arc<Stub> {
            Arc::new(Stub {
                status: 200,
                balasan: Some(bytes.to_vec()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn status(status: u16) -> Arc<Stub> {
            Arc::new(Stub {
                status,
                balasan: Some(b"halaman error".to_vec()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                status: 0,
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: self.status,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    /// PNG palsu yang lewat ambang 100 byte (`canvas/fakebankjago.rs` dan
    /// `ai/text2img2.rs` memakai pola yang sama).
    fn png_palsu() -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.resize(200, 0);
        v
    }

    fn run(http: Arc<Stub>, command: &str, text: &str) -> Reply {
        let ctx = Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: command.to_string(),
            http,
            ..Ctx::default()
        };
        brat(ctx)
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
        let p = oktz_plugin_api::find("brat").unwrap();
        assert_eq!(p.meta.name, "brat");
        assert_eq!(p.meta.alias, ["bratmenu", "bratimg", "brattext"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.description, "Menu variant brat dan generator sticker brat");
        assert_eq!(p.meta.usage, ".brat | .bratimg <text>");
        assert_eq!(p.meta.example, ".bratimg Hai semua");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["brat", "bratmenu", "bratimg", "brattext"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "brat");
        }
    }

    #[test]
    fn menu_brat_mengirim_kartu_tombol_tanpa_request() {
        for cmd in ["brat", "bratmenu"] {
            let http = Stub::offline();
            let r = run(http.clone(), cmd, "Hai semua");
            let Reply::Interactive {
                body,
                footer,
                buttons,
            } = r.without_mutations()
            else {
                panic!("command {cmd}: harus kartu interaktif");
            };
            assert_eq!(body, CAPTION_MENU, "command {cmd}");
            assert_eq!(footer.as_deref(), Some("Pilih variant brat favorit kamu"));
            assert_eq!(buttons.len(), 1);
            assert_eq!(buttons[0].name, "single_select");
            assert_eq!(buttons[0].params["title"], "🌾 Pilih Variant Brat");
            assert_eq!(buttons[0].params["sections"][0]["title"], "Variant Brat");
            let rows = buttons[0].params["sections"][0]["rows"].as_array().unwrap();
            assert_eq!(rows.len(), 6);
            assert_eq!(rows[0]["title"], "Brat Default");
            assert_eq!(rows[0]["id"], ".bratimg Hai semua");
            assert_eq!(reaksi(&r), None, "menu JS tidak bereaksi");
            assert!(http.seen.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn isi_baris_menu_mencerminkan_js() {
        let kosong = baris_varian(".", "");
        assert_eq!(kosong.len(), 6);
        assert_eq!(
            kosong[0],
            (
                "Brat Default".to_string(),
                "Sticker brat versi biasa • .bratimg <text>".to_string(),
                ".bratimg".to_string()
            )
        );
        assert_eq!(kosong[3].0, "Brat Anime");
        assert_eq!(kosong[5].2, ".bratvid2");

        let dengan_teks = baris_varian(".", "Hai semua");
        assert_eq!(dengan_teks[1].2, ".bratgreen Hai semua");
        assert_eq!(dengan_teks[1].1, "Variant brat warna hijau • .bratgreen <text>");
    }

    #[test]
    fn tanpa_teks_mengembalikan_kartu_pakai() {
        let http = Stub::offline();
        let r = run(http.clone(), "bratimg", "");
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(
            pakai("."),
            "🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: .bratimg Hai semua`"
        );
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn parse_brat_args_membuang_flag_blur() {
        assert_eq!(parse_brat_args("halo"), ("halo".to_string(), 0));
        assert_eq!(parse_brat_args("  halo  "), ("halo".to_string(), 0));
        assert_eq!(parse_brat_args("halo -blur 2"), ("halo".to_string(), 2));
        assert_eq!(parse_brat_args("halo -blur2"), ("halo".to_string(), 2));
        assert_eq!(parse_brat_args("-blur 3"), (String::new(), 3));
        assert_eq!(parse_brat_args("-blur\t1 halo"), ("halo".to_string(), 1));
        // Flag tak dikenal dibiarkan sebagai teks.
        assert_eq!(parse_brat_args("halo -blur 9"), ("halo -blur 9".to_string(), 0));
        assert_eq!(parse_brat_args("halo -blurx 3"), ("halo -blurx 3".to_string(), 0));
        // Regex tanpa `g` lanjut mencari `-blur` berikutnya kalau yang pertama
        // tidak diikuti angka 0..3.
        assert_eq!(
            parse_brat_args("-blur 9 -blur 2"),
            ("-blur 9".to_string(), 2)
        );
        // `[0-3]` satu digit: sisa digitnya tertinggal di teks.
        assert_eq!(parse_brat_args("-blur 01"), ("1".to_string(), 0));
    }

    #[test]
    fn url_api_mengemudikan_teks() {
        assert_eq!(
            api_url("Hai semua"),
            "https://api.theresav.eu/api/maker/brat?text=Hai%20semua"
        );
        assert_eq!(
            api_url("😂"),
            "https://api.theresav.eu/api/maker/brat?text=%F0%9F%98%82"
        );
    }

    #[test]
    fn is_blank_mengikuti_ambang_js() {
        assert!(is_blank(b""));
        assert!(is_blank(&[0x89, 0x50, 0x4E, 0x47])); // < 100 byte
        assert!(!is_blank(&png_palsu()));
        // HTML dari proxy dengan status 200: bukan gambar.
        assert!(is_blank(&[b'<'; 200]));
        // WebP/GIF/JPEG asli dilewatkan.
        let mut webp = b"RIFF".to_vec();
        webp.resize(8, 0);
        webp.extend_from_slice(b"WEBP");
        webp.resize(120, 0);
        assert!(!is_blank(&webp));
    }

    #[test]
    fn sukses_api_mengirim_sticker_byte_dan_reaksi_centang() {
        let png = png_palsu();
        let http = Stub::ok(&png);
        let r = run(http.clone(), "bratimg", "halo -blur 2");
        assert_eq!(reaksi(&r), Some("✅"));
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        };
        match isi {
            Reply::Sticker { media } => assert_eq!(
                media,
                Some(MediaSource::Bytes {
                    data: Arc::from(png),
                    mimetype: None,
                })
            ),
            lain => panic!("harus sticker, dapat {lain:?}"),
        }
        // `parseBratArgs` sudah membuang `-blur 2` sebelum request.
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.theresav.eu/api/maker/brat?text=halo".to_string()]
        );
    }

    #[test]
    fn jalur_gagal_selalu_error_template_dan_reaksi_bahu() {
        let gagal: Vec<Arc<Stub>> = vec![
            Stub::offline(),
            Stub::status(500),
            Stub::status(404),
            Stub::ok(b"<html>blank</html>"),
            Stub::ok(&[0x89, 0x50, 0x4E, 0x47]),
        ];
        for http in gagal {
            let r = run(http, "bratimg", "halo");
            assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "bratimg"));
            assert_eq!(reaksi(&r), Some("☢"));
        }
    }

    #[test]
    fn kartu_pakai_selalu_menyebut_bratimg_walau_command_lain() {
        let http = Stub::offline();
        let r = run(http, "brattext", "");
        assert_eq!(
            teks(&r),
            "🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: .bratimg Hai semua`"
        );
    }
}
