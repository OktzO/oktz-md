//! Brat static image renderer.

use ab_glyph::{Font, FontArc, ScaleFont};
use image::{DynamicImage, RgbaImage};

const FONT: &[u8] = include_bytes!("../../assets/fonts/arialnarrow.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BratTema {
    Putih,
    Hijau,
    Anime,
    Cewek,
    Hitam,
}

impl BratTema {
    pub fn warna_latar(self) -> [u8; 4] {
        match self {
            Self::Putih => [255, 255, 255, 255],
            Self::Hijau => [138, 206, 0, 255],
            Self::Anime => [255, 105, 180, 255],
            Self::Cewek => [255, 192, 203, 255],
            Self::Hitam => [0, 0, 0, 255],
        }
    }

    pub fn warna_teks(self) -> [u8; 4] {
        match self {
            Self::Hitam => [255, 255, 255, 255],
            _ => [0, 0, 0, 255],
        }
    }
}

pub fn render_brat(teks: &str, tema: BratTema, _blur: u8) -> Result<Vec<u8>, String> {
    let Ok(font) = FontArc::try_from_slice(FONT) else {
        return Err("font brat tidak bisa dibaca".to_string());
    };
    let mut canvas = RgbaImage::new(1000, 1000);
    let latar = BratTema::warna_latar(tema);
    for p in canvas.pixels_mut() {
        *p = image::Rgba(latar);
    }

    let ukuran = 180.0;
    let scaled = font.as_scaled(ukuran);
    let mut baris: Vec<String> = Vec::new();
    let mut saat_ini = String::new();
    for kata in teks.split_whitespace() {
        let coba = if saat_ini.is_empty() {
            kata.to_string()
        } else {
            format!("{saat_ini} {kata}")
        };
        if coba.chars().count() > 24 && !saat_ini.is_empty() {
            baris.push(saat_ini.clone());
            saat_ini = kata.to_string();
        } else {
            saat_ini = coba;
        }
    }
    if !saat_ini.is_empty() {
        baris.push(saat_ini);
    }

    let line_h = scaled.ascent() - scaled.descent() + 20.0;
    let tinggi = baris.len() as f32 * line_h;
    let mut y = ((1000.0 - tinggi) / 2.0).max(80.0);
    let warna_teks = BratTema::warna_teks(tema);
    for baris in &baris {
        let mut x = 80.0;
        for karakter in baris.chars() {
            let glyph = font
                .glyph_id(karakter)
                .with_scale_and_position(ukuran, ab_glyph::point(x, y + scaled.ascent()));
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|px, py, coverage| {
                    let x = bounds.min.x as u32 + px;
                    let y = bounds.min.y as u32 + py;
                    if x < 1000 && y < 1000 {
                        let a = (warna_teks[3] as f32 * coverage).round() as u8;
                        canvas.put_pixel(x, y, image::Rgba([warna_teks[0], warna_teks[1], warna_teks[2], a]));
                    }
                });
            }
            x += scaled.h_advance(font.glyph_id(karakter));
        }
        y += line_h;
    }

    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png)
        .map_err(|e| format!("gagal encode brat: {e}"))?;
    Ok(buf)
}

pub fn is_blank(buf: &[u8]) -> bool {
    if buf.len() < 100 {
        return true;
    }
    let Ok(img) = image::load_from_memory(buf) else {
        return true;
    };
    let img = img.to_rgba8();
    let small = DynamicImage::ImageRgba8(img).resize_exact(50, 50, image::imageops::FilterType::Triangle);
    let small = small.to_rgba8();
    let pertama = small.pixels().next().copied();
    let Some(pertama) = pertama else {
        return true;
    };
    small.pixels().all(|p| p == &pertama)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_brat_is_1000x1000_nonblank() {
        let png = render_brat("halo brat", BratTema::Putih, 0).unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.dimensions(), (1000, 1000));
        use image::GenericImageView;
        assert!(!is_blank(&png));
    }

    #[test]
    fn is_blank_rejects_uniform_image() {
        let img = image::RgbaImage::from_fn(100, 100, |_x, _y| image::Rgba([255, 255, 255, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png)
            .unwrap();
        assert!(is_blank(&buf));
        assert!(!is_blank(&render_brat("halo", BratTema::Putih, 0).unwrap()));
    }

    #[test]
    fn invalid_bytes_are_blank() {
        assert!(is_blank(b"bukan gambar"));
        assert!(is_blank(&[0u8; 10]));
    }
}
