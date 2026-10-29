//! Canvas RGBA minimal untuk gambar diagnostik Rust.

use ab_glyph::{Font, FontArc, ScaleFont, point};
use image::{Rgba, RgbaImage};

const FONT: &[u8] = include_bytes!("../../assets/font.ttf");

#[derive(Clone)]
pub struct Kanvas {
    lebar: u32,
    tinggi: u32,
    piksel: RgbaImage,
}

impl Kanvas {
    pub fn baru(lebar: u32, tinggi: u32) -> Self {
        Self {
            lebar,
            tinggi,
            piksel: RgbaImage::new(lebar, tinggi),
        }
    }

    pub fn ukuran(&self) -> (u32, u32) {
        (self.lebar, self.tinggi)
    }

    pub fn ukuran_png(data: &[u8]) -> Option<(u32, u32)> {
        if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" {
            return None;
        }
        if &data[12..16] != b"IHDR" {
            return None;
        }
        let lebar = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let tinggi = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        Some((lebar, tinggi))
    }

    fn campur(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
        let t = t.clamp(0.0, 1.0);
        let kanal = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
        Rgba([
            kanal(a[0], b[0]),
            kanal(a[1], b[1]),
            kanal(a[2], b[2]),
            kanal(a[3], b[3]),
        ])
    }

    fn warnakan(&mut self, x: u32, y: u32, warna: Rgba<u8>) {
        if x >= self.lebar || y >= self.tinggi {
            return;
        }
        let latar = self.piksel.get_pixel(x, y);
        let a = warna[3] as f32 / 255.0;
        let hasil = [
            (latar[0] as f32 * (1.0 - a) + warna[0] as f32 * a).round() as u8,
            (latar[1] as f32 * (1.0 - a) + warna[1] as f32 * a).round() as u8,
            (latar[2] as f32 * (1.0 - a) + warna[2] as f32 * a).round() as u8,
            255,
        ];
        self.piksel.put_pixel(x, y, Rgba(hasil));
    }

    pub fn isi_gradient_radial(
        &mut self,
        tengah: (u32, u32),
        radius: u32,
        warna: [(u8, u8, u8); 2],
    ) {
        let radius = radius.max(1) as f32;
        let (cx, cy) = (tengah.0 as f32, tengah.1 as f32);
        for y in 0..self.tinggi {
            for x in 0..self.lebar {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let jarak = (dx * dx + dy * dy).sqrt() / radius;
                let warna = Self::campur(
                    Rgba([warna[0].0, warna[0].1, warna[0].2, 255]),
                    Rgba([warna[1].0, warna[1].1, warna[1].2, 255]),
                    jarak,
                );
                self.piksel.put_pixel(x, y, warna);
            }
        }
    }

    pub fn kotak_gradient(
        &mut self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        warna: [(u8, u8, u8); 2],
        glow: (u8, u8, u8),
    ) {
        let h = h.max(1);
        for baris in 0..h {
            let t = baris as f32 / (h - 1) as f32;
            let warna = Self::campur(
                Rgba([warna[0].0, warna[0].1, warna[0].2, 217]),
                Rgba([warna[1].0, warna[1].1, warna[1].2, 173]),
                t,
            );
            for kolom in 0..w {
                self.warnakan(x + kolom, y + baris, warna);
            }
        }
        let glow = Rgba([glow.0, glow.1, glow.2, 235]);
        for kolom in 0..w {
            self.warnakan(x + kolom, y, glow);
        }
    }

    pub fn garis(&mut self, dari: (u32, u32), ke: (u32, u32), warna: [u8; 4]) {
        let (x0, y0) = (dari.0 as i32, dari.1 as i32);
        let (x1, y1) = (ke.0 as i32, ke.1 as i32);
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;
        loop {
            if x >= 0 && y >= 0 {
                self.warnakan(x as u32, y as u32, Rgba(warna));
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn lingkaran(
        &mut self,
        tengah: (u32, u32),
        radius: u32,
        warna: [u8; 4],
        mulai: f32,
        selesai: f32,
    ) {
        let radius = radius as f32;
        let n = (radius * 6.5).max(96.0) as u32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let sudut = mulai + (selesai - mulai) * t;
            let x = tengah.0 as f32 + radius * sudut.cos();
            let y = tengah.1 as f32 + radius * sudut.sin();
            if x >= 0.0 && y >= 0.0 {
                self.warnakan(x as u32, y as u32, Rgba(warna));
            }
        }
    }

    pub fn teks(&mut self, teks: &str, x: u32, y: u32, ukuran: f32, warna: [u8; 4]) {
        let Ok(font) = FontArc::try_from_slice(FONT) else {
            return;
        };
        let scaled = font.as_scaled(ukuran);
        let mut kursor_x = x as f32;
        let baseline = y as f32 + scaled.ascent();
        for karakter in teks.chars() {
            if karakter == '\n' {
                kursor_x = x as f32;
                continue;
            }
            let glyph = font.glyph_id(karakter).with_scale_and_position(ukuran, point(kursor_x, baseline));
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|px, py, coverage| {
                    self.warnakan(
                        bounds.min.x as u32 + px,
                        bounds.min.y as u32 + py,
                        Rgba([
                            warna[0],
                            warna[1],
                            warna[2],
                            (warna[3] as f32 * coverage).round() as u8,
                        ]),
                    );
                });
            }
            let advance = scaled.h_advance(font.glyph_id(karakter));
            kursor_x += advance;
        }
    }

    pub fn ke_png(self) -> Vec<u8> {
        let mut buf = Vec::new();
        if image::DynamicImage::ImageRgba8(self.piksel)
            .write_to(
                &mut std::io::Cursor::new(&mut buf),
                image::ImageOutputFormat::Png,
            )
            .is_err()
        {
            return Vec::new();
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    fn kanvas_text_rasterizes_nonempty_pixels() {
        let mut kanvas = Kanvas::baru(400, 120);
        let kosong = kanvas.piksel.clone();
        kanvas.teks("PING", 20, 40, 42.0, [255, 255, 255, 255]);
        assert_ne!(kanvas.piksel, kosong);
    }

    #[test]
    fn kanvas_outputs_png_magic_and_dimensions() {
        let mut kanvas = Kanvas::baru(32, 24);
        kanvas.teks("ok", 2, 2, 12.0, [255, 255, 255, 255]);
        let png = kanvas.ke_png();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.dimensions(), (32, 24));
    }

    #[test]
    fn canvas_new_is_transparent_before_drawing() {
        let kanvas = Kanvas::baru(4, 4);
        for piksel in kanvas.piksel.pixels() {
            assert_eq!(piksel.0, [0, 0, 0, 0]);
        }
    }
}
