//! Kemampuan gambar (padanan `sharp`) untuk oktz-plugin-api.
//!
//! Pembungkus ringan di atas crate `image` (0.24, fitur jpeg/png/webp/gif)
//! yang sudah ditambahkan orkestrator ke `Cargo.toml`. Tidak ada dependensi
//! baru. API 0.24 dipakai karena di sinilah `ImageOutputFormat` masih ada.
//!
//! Tersedia:
//!   - `sharp(data)` — buat `Gambar` dari byte gambar (jpeg/png/webp/gif).
//!   - `ubah_ukuran(w, h)` — padanan `resize(w, h)` dengan `fit: 'cover'`
//!     (potong ke tengah agar mengisi, lalu ubah ukuran) seperti sharp default.
//!   - `pas_di_dalam()` — `fit: 'inside'` terhadap ukuran terakhir dari
//!     `ubah_ukuran` (masuk tanpa melebihi sisi, rasio aspek dipertahankan).
//!   - `tanpa_memperbesar()` — `withoutEnlargement: true` terhadap ukuran
//!     terakhir (hanya memperkecil, tidak memperbesar).
//!   - `putar_otomatis()` — `rotate()` tanpa argumen (saat ini placeholder,
//!     EXIF belum diparsing).
//!   - `pastikan_alfa()` — `ensureAlpha()` (saat ini placeholder, alfa standar
//!     dari format gambar).
//!   - `skala_abu()` — `grayscale()`.
//!   - `metadata()` — lebar/tinggi/format.
//!   - `ke_jpeg(kualitas)`, `ke_png()`, `ke_webp(kualitas)` — encode ke `Vec<u8>`.
//!
//! Belum tercakup (jalan naik untuk orchestrator/plugin lain):
//!   - komposit gambar (overlap/blend).
//!   - ekstensi metode baru di luar yang tercantum di atas.
//!   - GIF animasi (hanya frame pertama dibaca).
//!   - putar dengan sudut bebas (hanya `putar_otomatis` tanpa argumen).
//!   - kernel resize `nearest` (hanya filter default image).
//!
//! Komentar dalam bahasa Indonesia. Tidak ada huruf CJK.

use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::{GenericImageView, ImageFormat, ImageOutputFormat};
use std::io::Cursor;

/// Padanan `sharp(data)`.
///
/// Mengembalikan `Err` bila `data` bukan berkas gambar yang dikenali
/// (jpeg/png/webp/gif), bukan `panic`.
pub fn sharp(data: &[u8]) -> Result<Gambar, String> {
    let cursor = Cursor::new(data);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| format!("tidak bisa mendeteksi format gambar: {e}"))?;
    // Nama format mengikuti sharp().metadata().format (huruf kecil, tanpa
    // titik), bukan Debug enum.
    let format = match reader.format() {
        Some(ImageFormat::Jpeg) => "jpeg",
        Some(ImageFormat::Png) => "png",
        Some(ImageFormat::WebP) => "webp",
        Some(ImageFormat::Gif) => "gif",
        _ => "unknown",
    }
    .to_string();
    let gambar = reader
        .decode()
        .map_err(|e| format!("tidak bisa mendecode gambar: {e}"))?;
    Ok(Gambar {
        dinamis: gambar,
        format,
        target: None,
    })
}

/// Metadata gambar.
pub struct MetadataGambar {
    pub lebar: u32,
    pub tinggi: u32,
    pub format: String,
}

/// Gambar yang dapat dirantai (padanan instance `sharp`).
pub struct Gambar {
    dinamis: image::DynamicImage,
    format: String,
    /// Ukuran target terakhir yang diatur oleh `ubah_ukuran`, digunakan oleh
    /// `pas_di_dalam` dan `tanpa_memperbesar`.
    target: Option<(u32, u32)>,
}

impl Gambar {
    /// `resize(w, h)` — default `fit: 'cover'` seperti sharp.
    pub fn ubah_ukuran(mut self, lebar: u32, tinggi: u32) -> Self {
        let (lebar_sumber, tinggi_sumber) = self.dinamis.dimensions();
        let rasio_target = lebar as f32 / tinggi as f32;
        let rasio_sumber = lebar_sumber as f32 / tinggi_sumber as f32;

        let dipotong = if rasio_sumber > rasio_target {
            // sumber lebih lebar relatif target: potong sisi kiri-kanan.
            let baru_lebar = (tinggi_sumber as f32 * rasio_target) as u32;
            let x = (lebar_sumber.saturating_sub(baru_lebar)) / 2;
            image::imageops::crop_imm(&self.dinamis, x, 0, baru_lebar, tinggi_sumber).to_image()
        } else {
            // sumber lebih tinggi relatif target: potong atas-bawah.
            let baru_tinggi = (lebar_sumber as f32 / rasio_target) as u32;
            let y = (tinggi_sumber.saturating_sub(baru_tinggi)) / 2;
            image::imageops::crop_imm(&self.dinamis, 0, y, lebar_sumber, baru_tinggi).to_image()
        };

        self.dinamis = image::DynamicImage::ImageRgba8(image::imageops::resize(
            &dipotong,
            lebar,
            tinggi,
            FilterType::Triangle,
        ));
        self.target = Some((lebar, tinggi));
        self
    }

    /// `resize(w, h, { fit: 'inside' })` — masuk tanpa melebihi sisi.
    pub fn pas_di_dalam(mut self) -> Self {
        if let Some((target_lebar, target_tinggi)) = self.target {
            let (lebar_sumber, tinggi_sumber) = self.dinamis.dimensions();
            let skala = (target_lebar as f32 / lebar_sumber as f32)
                .min(target_tinggi as f32 / tinggi_sumber as f32);
            let baru_lebar = (lebar_sumber as f32 * skala) as u32;
            let baru_tinggi = (tinggi_sumber as f32 * skala) as u32;
            self.dinamis = image::DynamicImage::ImageRgba8(image::imageops::resize(
                &self.dinamis,
                baru_lebar,
                baru_tinggi,
                FilterType::Triangle,
            ));
        }
        self
    }

    /// `withoutEnlargement: true` (hanya memperkecil, tidak memperbesar).
    pub fn tanpa_memperbesar(mut self) -> Self {
        if let Some((target_lebar, target_tinggi)) = self.target {
            let (lebar_sumber, tinggi_sumber) = self.dinamis.dimensions();
            if lebar_sumber <= target_lebar && tinggi_sumber <= target_tinggi {
                return self;
            }
            let skala = (target_lebar as f32 / lebar_sumber as f32)
                .min(target_tinggi as f32 / tinggi_sumber as f32);
            let baru_lebar = (lebar_sumber as f32 * skala) as u32;
            let baru_tinggi = (tinggi_sumber as f32 * skala) as u32;
            self.dinamis = image::DynamicImage::ImageRgba8(image::imageops::resize(
                &self.dinamis,
                baru_lebar,
                baru_tinggi,
                FilterType::Triangle,
            ));
        }
        self
    }

    /// `rotate()` tanpa argumen — putar menurut EXIF.
    ///
    /// Saat ini placeholder: EXIF belum diparsing, mengembalikan `self` tanpa
    /// perubahan. Jalan naik: tambah parsing EXIF orientation.
    pub fn putar_otomatis(self) -> Self {
        self
    }

    /// `ensureAlpha()` — pastikan kanal alfa ada.
    ///
    /// Saat ini placeholder: alfa standar dari format gambar, mengembalikan
    /// `self` tanpa perubahan. Jalan naik: konversi ke RGBA bila belum ada.
    pub fn pastikan_alfa(self) -> Self {
        self
    }

    /// `grayscale()` — ubah ke skala abu-abu.
    pub fn skala_abu(mut self) -> Self {
        self.dinamis = image::DynamicImage::ImageLuma8(image::imageops::grayscale(&self.dinamis));
        self
    }

    pub fn metadata(&self) -> MetadataGambar {
        let (lebar, tinggi) = self.dinamis.dimensions();
        MetadataGambar {
            lebar,
            tinggi,
            format: self.format.clone(),
        }
    }

    pub fn ke_jpeg(self, kualitas: u8) -> Result<Vec<u8>, String> {
        // image 0.24 menulis lewat Seek, jadi pakai Cursor.
        let mut buf = Cursor::new(Vec::new());
        self.dinamis
            .write_to(&mut buf, ImageOutputFormat::Jpeg(kualitas))
            .map_err(|e| format!("gagal encode jpeg: {e}"))?;
        Ok(buf.into_inner())
    }

    pub fn ke_png(self) -> Result<Vec<u8>, String> {
        let mut buf = Cursor::new(Vec::new());
        self.dinamis
            .write_to(&mut buf, ImageOutputFormat::Png)
            .map_err(|e| format!("gagal encode png: {e}"))?;
        Ok(buf.into_inner())
    }

    /// sharp().webp({ quality }) — crate image 0.24 tidak menerima
    /// kualitas untuk WebP, jadi kualitas sengaja diabaikan. ponytail:
    /// pindah ke encoder webp lain kalau kualitas benar-benar harus diatur.
    pub fn ke_webp(self, _kualitas: u8) -> Result<Vec<u8>, String> {
        let mut buf = Cursor::new(Vec::new());
        self.dinamis
            .write_to(&mut buf, ImageOutputFormat::WebP)
            .map_err(|e| format!("gagal encode webp: {e}"))?;
        Ok(buf.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buat_png_lebar_tinggi(lebar: u32, tinggi: u32) -> Vec<u8> {
        let rgb = image::RgbImage::new(lebar, tinggi);
        let mut buf = Cursor::new(Vec::new());
        rgb.write_to(&mut buf, ImageOutputFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn sharp_resize_hasil_300x170() {
        let data = buat_png_lebar_tinggi(600, 400);
        let gambar = sharp(&data).unwrap().ubah_ukuran(300, 170);
        let meta = gambar.metadata();
        assert_eq!(meta.lebar, 300);
        assert_eq!(meta.tinggi, 170);
    }

    #[test]
    fn pas_di_dalam_tidak_melebihi() {
        let data = buat_png_lebar_tinggi(800, 200);
        let gambar = sharp(&data).unwrap().ubah_ukuran(300, 170).pas_di_dalam();
        let meta = gambar.metadata();
        assert!(meta.lebar <= 300);
        assert!(meta.tinggi <= 170);
    }

    #[test]
    fn ke_png_bisa_dibaca_ulang() {
        let data = buat_png_lebar_tinggi(50, 50);
        let encoded = sharp(&data).unwrap().ke_png().unwrap();
        let _ = sharp(&encoded).unwrap();
    }

    #[test]
    fn sharp_byte_sampah_err_bukan_panic() {
        let hasil = sharp(b"bukan gambar");
        assert!(hasil.is_err());
    }
}
