//! `src/scraper/lufemboy.js` — "cek seberapa femboy", acak dari 0 sampai 100.
//!
//! Tidak ada HTTP sama sekali: skrip cuma melempar angka dan memilih salah satu
//! dari lima pasang teks/gambar. `&dyn HttpGet` tetap jadi argumen pertama
//! supaya pemanggil scraper tidak perlu cabang khusus; ditulis `_http`
//! karena tidak dipakai.
//!
//! Semantik yang disalin dari JS:
//! - `if (!nama)` — string kosong itu falsy, jadi `""` juga ditolak.
//! - `Math.floor(Math.random() * 101)` → 0 sampai 100 **inklusif**.
//! - `catch (error) { throw new Error(error.message) }` tidak mengubah teks
//!   pesan, jadi pesannya sama persis.
//! - Emoji di `desc` adalah keluaran program, bukan komentar.

use oktz_plugin_api::{HttpError, HttpGet};
use serde_json::{Value, json};

/// `cekfemboy(nama)` (`lufemboy.js:1-33`).
pub fn cek_femboy(_http: &dyn HttpGet, nama: &str) -> Result<Value, HttpError> {
    if nama.is_empty() {
        return Err(HttpError {
            message: "Masukkan nama dulu dong!".to_string(),
        });
    }

    let percent = rand::random_range(0..101);

    let (desc, img_url) = if percent < 20 {
        (
            "Cowok banget! \u{1F60E}",
            "https://cek-seberapa-femboy.vercel.app/img/normal.gif",
        )
    } else if percent < 40 {
        (
            "Ada aura lembutnya dikit~ \u{1F338}",
            "https://cek-seberapa-femboy.vercel.app/img/dibwh40.gif",
        )
    } else if percent < 60 {
        (
            "Lumayan femboy \u{1F618}",
            "https://cek-seberapa-femboy.vercel.app/img/dibwh60.gif",
        )
    } else if percent < 80 {
        (
            "Femboy sejati \u{1F485}\u{2728}",
            "https://cek-seberapa-femboy.vercel.app/img/dibwh80.gif",
        )
    } else {
        (
            "FEMBOY DEWA \u{1F525}\u{1F496}",
            "https://cek-seberapa-femboy.vercel.app/img/femboyyyy.gif",
        )
    };

    Ok(json!({
        "hasil": format!("{nama}, kamu {percent}% femboy!, {desc}"),
        "gif": img_url,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TanpaHttp;

    impl HttpGet for TanpaHttp {
        fn get(&self, _u: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak memakai jaringan")
        }
        fn post_json(
            &self,
            _u: &str,
            _b: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak memakai jaringan")
        }
        fn post_form(
            &self,
            _u: &str,
            _f: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak memakai jaringan")
        }
        fn post_multipart(
            &self,
            _u: &str,
            _p: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak memakai jaringan")
        }
    }

    const DESC: [(&str, &str); 5] = [
        ("Cowok banget! \u{1F60E}", "normal.gif"),
        ("Ada aura lembutnya dikit~ \u{1F338}", "dibwh40.gif"),
        ("Lumayan femboy \u{1F618}", "dibwh60.gif"),
        ("Femboy sejati \u{1F485}\u{2728}", "dibwh80.gif"),
        ("FEMBOY DEWA \u{1F525}\u{1F496}", "femboyyyy.gif"),
    ];

    #[test]
    fn nama_kosong_melempar_pesan_yang_sama_dengan_js() {
        let galat = cek_femboy(&TanpaHttp, "").unwrap_err();
        assert_eq!(galat.message, "Masukkan nama dulu dong!");
    }

    #[test]
    fn hasil_selalu_cocok_dengan_pasangan_desc_dan_gif() {
        // Rangkuman 500 kali: teks, gif, dan ambang persen harus selalu satu
        // pasangan yang sama — bukan campuran.
        for _ in 0..500 {
            let hasil = cek_femboy(&TanpaHttp, "Budi").unwrap();
            let teks = hasil["hasil"].as_str().unwrap();
            let gif = hasil["gif"].as_str().unwrap();

            let (_, persen) = teks
                .split_once(", kamu ")
                .expect("pola 'nama, kamu N% femboy!, desc'");
            let (angka, _) = persen.split_once("% femboy!").unwrap();
            let n: u32 = angka.parse().unwrap();
            assert!(n <= 100, "persen di luar 0..100: {n}");

            // DESC berurutan menurut ambangnya, jadi pasangan yang cocok harus
            // sesuai dengan angka persen yang tercetak di teks.
            let (i, (_, gif_akhir)) = DESC
                .iter()
                .enumerate()
                .find(|(_, (d, _))| teks.ends_with(*d))
                .expect("desc harus salah satu dari lima");
            assert!(gif.ends_with(*gif_akhir), "gif tidak cocok #{i}: {gif}");
            let batas = [20u32, 40, 60, 80, 101][i];
            assert!(
                n < batas,
                "persen {n} tidak cocok pasangan #{i} (batas {batas})"
            );
        }
    }

    #[test]
    fn nama_ikut_dalam_teks_hasil() {
        let hasil = cek_femboy(&TanpaHttp, "Siti").unwrap();
        assert!(
            hasil["hasil"]
                .as_str()
                .unwrap()
                .starts_with("Siti, kamu "),
            "nama harus jadi awalan: {}",
            hasil["hasil"]
        );
    }
}