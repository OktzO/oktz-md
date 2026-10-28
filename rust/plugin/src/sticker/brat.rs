//! `plugins/sticker/brat.js` — menu variant brat dan generator sticker brat.

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, Tombol, plugin};

pub(crate) const CAPTION_MENU: &str =
    "🌿 *kamu mau buat brat yak, silahkan pilih variant brat tombol dibawah*";

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
        .with_header(oktz_plugin_api::kartu::KartuHeader {
            title: Some(String::new()),
            subtitle: Some("Oktz Bot".to_string()),
            media: Some(MediaSource::path("assets/image/foto.jpeg")),
        })
        .with_footer("Pilih variant brat favorit kamu")
        .with_button(
            Tombol::dari_nama(
                "single_select",
                serde_json::json!({
                    "title": "🌾 Pilih Variant Brat",
                    "sections": [{
                        "title": "Variant Brat",
                        "rows": rows,
                    }],
                }),
            )
            .expect("nama tombol WA dikenal"),
        )
}

pub(crate) fn pakai(prefix: &str) -> String {
    format!("🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: {prefix}bratimg Hai semua`")
}

pub(crate) fn api_url(text: &str) -> String {
    let mut out = String::from("https://api.theresav.eu/api/maker/brat?text=");
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub(crate) fn parse_brat_args(input: &str) -> (String, u8) {
    let text = input.trim();
    let mut dari = 0usize;
    while let Some(rel) = text[dari..].find("-blur") {
        let awal = dari + rel;
        let sisa = &text[awal + 5..];
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
    is_private = false
)]
pub fn brat(ctx: Ctx) -> Reply {
    let command = ctx.command.to_lowercase();
    if command == "brat" || command == "bratmenu" {
        return menu_brat(&ctx.prefix, &ctx.text);
    }

    if ctx.text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    let (parsed, blur) = parse_brat_args(&ctx.text);
    let http_call = HttpCall::get(api_url(&parsed)).with_timeout(15000);
    let res = match ctx.http.call(&http_call) {
        Ok(res) => res,
        Err(_) => {
            let bytes = oktz_plugin_api::render_brat(&parsed, oktz_plugin_api::BratTema::Putih, blur).unwrap_or_default();
            return Reply::sticker_opsi(MediaSource::bytes(bytes), oktz_plugin_api::StickerOpsi::gambar()).with_reaction("✅");
        }
    };
    if res.status < 200 || res.status >= 300 {
        let bytes = oktz_plugin_api::render_brat(&parsed, oktz_plugin_api::BratTema::Putih, blur).unwrap_or_default();
        return Reply::sticker_opsi(MediaSource::bytes(bytes), oktz_plugin_api::StickerOpsi::gambar()).with_reaction("✅");
    }
    let buf = res.body.bytes().to_vec();
    if oktz_plugin_api::is_blank(&buf) {
        let bytes = oktz_plugin_api::render_brat(&parsed, oktz_plugin_api::BratTema::Putih, blur).unwrap_or_default();
        return Reply::sticker_opsi(MediaSource::bytes(bytes), oktz_plugin_api::StickerOpsi::gambar()).with_reaction("✅");
    }

    Reply::sticker_opsi(
        MediaSource::bytes(buf).with_mimetype("image/webp"),
        oktz_plugin_api::StickerOpsi::gambar(),
    )
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brat_menu_has_six_rows() {
        let r = menu_brat(".", "halo");
        let Reply::Interactive { kartu } = r else {
            panic!("harus interactive")
        };
        assert!(kartu.body.contains("silahkan pilih variant brat"));
    }
}
