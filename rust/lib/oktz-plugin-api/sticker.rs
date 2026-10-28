//! Sticker options, WebP EXIF, and `.s` conversion helpers.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use image::imageops::FilterType;
use image::{ColorType, DynamicImage, ImageEncoder, RgbaImage};
use serde_json::json;

pub const STICKER_WEBP_VF: &str = "fps=12,scale='min(512,iw)':'min(512,ih)':force_original_aspect_ratio=decrease,format=rgba,pad=512:512:(ow-iw)/2:(oh-ih)/2:color=0x00000000,setsar=1";
pub const EXIF_PACK_ID: &str = "com.foto.sticker";
pub const EXIF_EMOJIS: [&str; 1] = ["🤖"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpsiS {
    Crop,
    Resize(u32, u32),
    Circle,
    Rounded,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StickerOpsi {
    pub packname: Option<String>,
    pub author: Option<String>,
    pub emojis: Vec<String>,
    pub forwarded: bool,
    pub forwarding_score: Option<u32>,
    pub premium: bool,
    pub animated: bool,
}

impl StickerOpsi {
    pub fn gambar() -> Self {
        Self {
            forwarded: true,
            forwarding_score: Some(1),
            premium: true,
            ..Default::default()
        }
    }

    pub fn video() -> Self {
        Self {
            forwarded: true,
            forwarding_score: Some(999),
            animated: true,
            ..Default::default()
        }
    }

    pub fn with_packname(mut self, packname: impl Into<String>) -> Self {
        let value = packname.into();
        if !value.is_empty() {
            self.packname = Some(value);
        }
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        let value = author.into();
        if !value.is_empty() {
            self.author = Some(value);
        }
        self
    }
}

fn is_webp(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP"
}

fn is_resize_value(value: &str) -> bool {
    if let Some((w, h)) = value.split_once('x') {
        return !w.is_empty()
            && !h.is_empty()
            && w.chars().all(|c| c.is_ascii_digit())
            && h.chars().all(|c| c.is_ascii_digit());
    }
    false
}

pub fn opsi_s(teks: &str) -> (Vec<OpsiS>, Option<String>, Option<String>) {
    let mut opsi: Vec<OpsiS> = Vec::new();
    let mut bebas: Vec<String> = Vec::new();
    let mut i = 0usize;
    let tokens: Vec<&str> = teks.split_whitespace().collect();

    while i < tokens.len() {
        let token = tokens[i];
        match token {
            "--crop" | "-c" => opsi.push(OpsiS::Crop),
            "--circle" => opsi.push(OpsiS::Circle),
            "--rounded" => opsi.push(OpsiS::Rounded),
            "--resize" | "-r" => {
                if let Some(nilai) = tokens.get(i + 1).filter(|n| is_resize_value(n)) {
                    let (w, h) = nilai.split_once('x').unwrap();
                    if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                        opsi.push(OpsiS::Resize(w, h));
                    }
                    i += 2;
                    continue;
                }
            }
            lain if !lain.starts_with('-') => bebas.push(lain.to_string()),
            _ => {}
        }
        i += 1;
    }

    let packname = bebas.first().cloned();
    let author = bebas.get(1).cloned();
    (opsi, packname, author)
}

fn to_rgba(data: &[u8]) -> Result<RgbaImage, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("gagal decode gambar: {e}"))?;
    Ok(img.to_rgba8())
}

fn contain(img: &RgbaImage, target_w: u32, target_h: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return RgbaImage::new(target_w, target_h);
    }
    let scale = (target_w as f32 / w as f32).min(target_h as f32 / h as f32);
    let new_w = ((w as f32 * scale).round() as u32).clamp(1, target_w);
    let new_h = ((h as f32 * scale).round() as u32).clamp(1, target_h);
    let scaled = DynamicImage::ImageRgba8(img.clone()).resize_exact(new_w, new_h, FilterType::Triangle);
    let mut out = RgbaImage::new(target_w, target_h);
    let x = (target_w.saturating_sub(new_w)) / 2;
    let y = (target_h.saturating_sub(new_h)) / 2;
    image::imageops::overlay(&mut out, &scaled.to_rgba8(), x as i64, y as i64);
    RgbaImage::from_raw(target_w, target_h, out.into_raw().to_vec()).unwrap_or_else(|| {
        let mut fallback = RgbaImage::new(target_w, target_h);
        image::imageops::overlay(&mut fallback, &scaled.to_rgba8(), x as i64, y as i64);
        fallback
    })
}

fn crop_center(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let side = w.min(h);
    let x = (w.saturating_sub(side)) / 2;
    let y = (h.saturating_sub(side)) / 2;
    image::imageops::crop_imm(img, x, y, side, side).to_image()
}

fn circle_mask(img: &mut RgbaImage) {
    let (w, h) = img.dimensions();
    let radius = (w.min(h) as f32) / 2.0;
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    for (x, y, px) in img.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - cx;
        let dy = y as f32 + 0.5 - cy;
        if (dx * dx + dy * dy).sqrt() > radius {
            px[3] = 0;
        }
    }
}

fn rounded_mask(img: &mut RgbaImage, radius: u32) {
    let (w, h) = img.dimensions();
    let radius = radius.min(w / 2).min(h / 2) as f32;
    for (x, y, px) in img.enumerate_pixels_mut() {
        let near_x = x < radius as u32 || x + 1 + radius as u32 > w;
        let near_y = y < radius as u32 || y + 1 + radius as u32 > h;
        if !(near_x && near_y) {
            continue;
        }
        let corner_x = if x < radius as u32 { radius - x as f32 } else { x as f32 + 1.0 - (w as f32 - radius) };
        let corner_y = if y < radius as u32 { radius - y as f32 } else { y as f32 + 1.0 - (h as f32 - radius) };
        if corner_x * corner_x + corner_y * corner_y > radius * radius {
            px[3] = 0;
        }
    }
}

fn encode_webp(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    #[allow(deprecated)]
    let encoder = image::codecs::webp::WebPEncoder::new_with_quality(
        &mut buf,
        image::codecs::webp::WebPQuality::lossy(80),
    );
    encoder
        .write_image(img.as_raw(), img.width(), img.height(), ColorType::Rgba8)
        .map_err(|e| format!("gagal encode webp: {e}"))?;
    Ok(buf)
}

pub fn gambar_ke_stiker_webp(data: &[u8], opsi: &[OpsiS]) -> Result<Vec<u8>, String> {
    if is_webp(data) {
        return Ok(data.to_vec());
    }

    let mut img = to_rgba(data)?;
    if opsi.contains(&OpsiS::Crop) {
        img = crop_center(&img);
    }
    let mut target = (512u32, 512u32);
    for op in opsi {
        if let OpsiS::Resize(w, h) = op {
            target = (*w, *h);
        }
    }
    img = contain(&img, target.0, target.1);
    img = contain(&img, 512, 512);
    if opsi.contains(&OpsiS::Circle) {
        circle_mask(&mut img);
    }
    if opsi.contains(&OpsiS::Rounded) {
        rounded_mask(&mut img, 50);
    }
    encode_webp(&img)
}

pub fn video_ke_stiker_webp(data: &[u8], opsi: &[OpsiS]) -> Result<Vec<u8>, String> {
    if is_webp(data) {
        return Ok(data.to_vec());
    }

    let mut prefix = String::from("fps=12,");
    if opsi.contains(&OpsiS::Crop) {
        prefix.push_str("crop='min(iw,ih)':'min(iw,ih)',");
    }
    for op in opsi {
        if let OpsiS::Resize(w, h) = op {
            prefix.push_str(&format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=0x00000000,setsar=1,"
            ));
        }
    }
    let vf = format!("{prefix}{STICKER_WEBP_VF}");
    let args = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-i",
        "pipe:0",
        "-t",
        "6",
        "-vf",
        vf.as_str(),
        "-c:v",
        "libwebp",
        "-q:v",
        "50",
        "-an",
        "-f",
        "webp",
        "pipe:1",
    ];
    crate::ffmpeg::jalankan_dengan_masukan(&args, data).map_err(|e| e.to_string())
}

pub fn sticker_metadata_json(packname: &str, author: &str, emojis: &[&str]) -> Vec<u8> {
    let value = json!({
        "sticker-pack-id": EXIF_PACK_ID,
        "sticker-pack-name": packname,
        "sticker-pack-publisher": author,
        "emojis": emojis,
        "is-avatar-sticker": 0,
        "android-app-store-link": "",
        "ios-app-store-link": "",
    });
    serde_json::to_vec(&value).unwrap_or_default()
}

fn exif_payload(metadata_json: &[u8]) -> Vec<u8> {
    let mut exif = vec![
        0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x41, 0x57, 0x07, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
    ];
    let len = metadata_json.len() as u32;
    exif[14..18].copy_from_slice(&len.to_le_bytes());
    exif.extend_from_slice(metadata_json);
    if exif.len() % 2 == 1 {
        exif.push(0);
    }
    exif
}

fn put_chunk(out: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(id);
    let len = payload.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn parse_chunks(data: &[u8]) -> Option<Vec<([u8; 4], Vec<u8>)>> {
    if !is_webp(data) {
        return None;
    }
    let mut pos = 12usize;
    let mut chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    while pos + 8 <= data.len() {
        let mut id = [0u8; 4];
        id.copy_from_slice(&data[pos..pos + 4]);
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size);
        if end > data.len() {
            return None;
        }
        chunks.push((id, data[start..end].to_vec()));
        pos = end + (size % 2);
        if pos > data.len() {
            return None;
        }
    }
    Some(chunks)
}

pub fn tempel_metadata_stiker(
    webp: &[u8],
    packname: &str,
    author: &str,
    emojis: &[&str],
) -> Result<Vec<u8>, String> {
    if !is_webp(webp) {
        return Err("bukan WebP".to_string());
    }
    let chunks = parse_chunks(webp).ok_or_else(|| "WebP rusak".to_string())?;
    let exif = exif_payload(&sticker_metadata_json(packname, author, emojis));

    let has_vp8x = chunks.iter().any(|(id, _)| id == b"VP8X");
    let has_alpha = chunks.iter().any(|(id, _)| id == b"ALPH");
    let (mut width, mut height) = (0u32, 0u32);
    if let Some((_, payload)) = chunks.iter().find(|(id, _)| id == b"VP8X") {
        if payload.len() >= 10 {
            let packed = ((payload[6] as u32) << 16)
                | ((payload[7] as u32) << 8)
                | payload[8] as u32;
            let packed_h = ((payload[9] as u32) << 24)
                | 0; // height occupies low 3 bytes after first byte
            width = packed + 1;
            height = packed_h.max(payload[9] as u32) + 1;
        }
    }

    let mut body: Vec<u8> = Vec::new();
    let flags = if has_alpha { 0x18 } else { 0x18 };
    if !has_vp8x {
        let mut vp8x = vec![flags, 0, 0, 0];
        let w = width.saturating_sub(1).min(0xFF_FFFF);
        let h = height.saturating_sub(1).min(0xFF_FFFF);
        vp8x.push(((w >> 16) & 0xFF) as u8);
        vp8x.push(((w >> 8) & 0xFF) as u8);
        vp8x.push((w & 0xFF) as u8);
        vp8x.push(((h >> 16) & 0xFF) as u8);
        vp8x.push(((h >> 8) & 0xFF) as u8);
        vp8x.push((h & 0xFF) as u8);
        put_chunk(&mut body, b"VP8X", &vp8x);
    } else {
        let mut vp8x = vec![flags, 0, 0, 0];
        let w = width.saturating_sub(1).min(0xFF_FFFF);
        let h = height.saturating_sub(1).min(0xFF_FFFF);
        vp8x.push(((w >> 16) & 0xFF) as u8);
        vp8x.push(((w >> 8) & 0xFF) as u8);
        vp8x.push((w & 0xFF) as u8);
        vp8x.push(((h >> 16) & 0xFF) as u8);
        vp8x.push(((h >> 8) & 0xFF) as u8);
        vp8x.push((h & 0xFF) as u8);
        put_chunk(&mut body, b"VP8X", &vp8x);
    }
    for (id, payload) in chunks.iter().filter(|(id, _)| id != b"EXIF" && id != b"XMP ") {
        put_chunk(&mut body, id, payload);
    }
    put_chunk(&mut body, b"EXIF", &exif);

    let mut out = Vec::with_capacity(12 + body.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(4 + body.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn buat_kunci_media() -> [u8; 32] {
    rand::random()
}

pub fn nama_unik(prefix: &str, ext: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let mut dir = std::env::temp_dir();
    dir.push(format!("{prefix}-{}-{nanos}.{ext}", std::process::id()));
    dir
}

pub fn tulis_berkas_sementara(prefix: &str, ext: &str, data: &[u8]) -> Result<std::path::PathBuf, String> {
    let path = nama_unik(prefix, ext);
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(data))
        .map_err(|e| format!("gagal menulis berkas sementara: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use image::Rgba;

    fn vp8l_riff() -> Vec<u8> {
        let payload = [0x2f, 0x01, 0x02, 0x03, 0x04];
        let mut body = Vec::new();
        put_chunk(&mut body, b"VP8L", &payload);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(4 + body.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WEBP");
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn option_parser_matches_js_first_two_named_tokens() {
        let (opsi, packname, author) = opsi_s("--resize 256x256 Pack Author ketiga");
        assert_eq!(opsi, vec![OpsiS::Resize(256, 256)]);
        assert_eq!(packname.as_deref(), Some("Pack"));
        assert_eq!(author.as_deref(), Some("Author"));

        let (opsi, packname, author) = opsi_s("--resize 256x256 -c Pack Author");
        assert_eq!(opsi, vec![OpsiS::Resize(256, 256), OpsiS::Crop]);
        assert_eq!(packname.as_deref(), Some("Pack"));
        assert_eq!(author.as_deref(), Some("Author"));

        let (opsi, packname, _) = opsi_s("--resize 256x256 Pack");
        assert_eq!(opsi, vec![OpsiS::Resize(256, 256)]);
        assert_eq!(packname.as_deref(), Some("Pack"));

        let (opsi, packname, author) = opsi_s("-r bogus Pack Author");
        assert_eq!(opsi, Vec::new());
        assert_eq!(packname.as_deref(), Some("bogus"));
        assert_eq!(author.as_deref(), Some("Pack"));
    }

    #[test]
    fn existing_webp_input_is_not_re_encoded() {
        let input = vp8l_riff();
        let out = gambar_ke_stiker_webp(&input, &[OpsiS::Circle]).unwrap();
        assert_eq!(out, input);

        let chunks = parse_chunks(&out).unwrap();
        let vp8l = chunks
            .iter()
            .find(|(id, _)| id == b"VP8L")
            .map(|(_, payload)| payload.clone())
            .unwrap();
        assert_eq!(vp8l, [0x2f, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn exif_writes_pack_name_publisher_emojis() {
        let input = vp8l_riff();
        let out = tempel_metadata_stiker(&input, "𝗙𝗢𝗧𝗢 𝗗𝗘𝗟𝗨𝗫𝗘", "Zann", &EXIF_EMOJIS).unwrap();
        assert!(is_webp(&out));
        let chunks = parse_chunks(&out).unwrap();
        assert!(chunks.iter().any(|(id, _)| id == b"VP8X"));
        assert!(chunks.iter().any(|(id, _)| id == b"EXIF"));
        let exif = chunks
            .iter()
            .find(|(id, _)| id == b"EXIF")
            .map(|(_, payload)| payload.clone())
            .unwrap();
        assert_eq!(&exif[..4], &[0x49, 0x49, 0x2A, 0x00]);
        let len = u32::from_le_bytes([exif[14], exif[15], exif[16], exif[17]]) as usize;
        let json = &exif[22..22 + len];
        let value: serde_json::Value = serde_json::from_slice(json).unwrap();
        assert_eq!(value["sticker-pack-name"], "𝗙𝗢𝗧𝗢 𝗗𝗘𝗟𝗨𝗫𝗘");
        assert_eq!(value["sticker-pack-publisher"], "Zann");
        assert_eq!(value["emojis"][0], "🤖");
        assert_eq!(value["sticker-pack-id"], "com.foto.sticker");
    }

    #[test]
    fn image_sticker_normalizes_to_512_transparent_contain() {
        let img = RgbaImage::from_fn(800, 400, |x, y| {
            if x < 200 && y < 100 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let mut encoded = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut encoded), image::ImageOutputFormat::Png)
            .unwrap();
        let out = gambar_ke_stiker_webp(&encoded, &[]).unwrap();
        assert!(is_webp(&out));
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!(decoded.dimensions(), (512, 512));
    }

    #[test]
    fn vp8l_flag_and_exif_chunk_size_are_valid() {
        let input = vp8l_riff();
        let out = tempel_metadata_stiker(&input, "P", "A", &["🤖"]).unwrap();
        let riff_size = u32::from_le_bytes([out[4], out[5], out[6], out[7]]) as usize;
        assert_eq!(riff_size + 8, out.len());
        let chunks = parse_chunks(&out).unwrap();
        assert!(chunks.iter().any(|(id, _)| id == b"VP8X"));
        assert!(chunks.iter().any(|(id, _)| id == b"EXIF"));
    }
}
