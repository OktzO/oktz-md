//! Kapabilitas berkas (sistem berkas) untuk plugin — padanan `fs` Node.
//!
//! Semua jalur yang datang dari plugin diperlakukan relatif terhadap `root`
//! dan wajib divalidasi supaya tidak menembus keluar akar (`..`, absolut,
//! symlink yang menembus keluar). Tolak dengan `Err`, jangan panic.
//!
//! Catatan kepala: pemakaian `fs` di 32 plugin JS mencakup baca daftar berkas,
//! tulis database, buat folder, dan hapus berkas — semuanya tercakup di trait
//! ini. Tidak ada metode tambahan.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

/// Kontak operasi berkas yang boleh dipakai plugin.
///
/// Implementasi: [`BerkasNyata`] (disk) dan [`BerkasPalsu`] (memori, test).
pub trait Berkas: Send + Sync + std::fmt::Debug {
    /// `fs.existsSync(jalur)`.
    fn ada(&self, jalur: &str) -> bool;
    /// `fs.readFileSync(jalur)` — byte mentah.
    fn baca(&self, jalur: &str) -> Result<Vec<u8>, String>;
    /// `fs.readFileSync(jalur)` sebagai teks UTF-8.
    fn baca_teks(&self, jalur: &str) -> Result<String, String>;
    /// `fs.writeFileSync(jalur, isi)` — menimpa isi lama.
    fn tulis(&self, jalur: &str, isi: &[u8]) -> Result<(), String>;
    /// `fs.writeFileSync(jalur, isi)` sebagai teks UTF-8.
    fn tulis_teks(&self, jalur: &str, isi: &str) -> Result<(), String>;
    /// `fs.appendFileSync(jalur, isi)` — menambahkan di akhir.
    fn tambah_teks(&self, jalur: &str, isi: &str) -> Result<(), String>;
    /// `fs.readdirSync(folder)` — nama entri saja, tanpa jalur penuh.
    fn daftar(&self, folder: &str) -> Result<Vec<String>, String>;
    /// `fs.unlinkSync(jalur)`.
    fn hapus(&self, jalur: &str) -> Result<(), String>;
    /// `fs.mkdirSync(jalur, { recursive: true })` — tidak error kalau sudah ada.
    fn buat_folder(&self, jalur: &str) -> Result<(), String>;
    /// `fs.statSync(jalur).size`.
    fn ukuran(&self, jalur: &str) -> Result<u64, String>;
    /// `fs.copyFileSync(dari, ke)`.
    fn salin(&self, dari: &str, ke: &str) -> Result<(), String>;
}

/// Validasi jalur mentah dari plugin: tolak absolut dan komponen `..`.
///
/// Hanya memeriksa string jalur, tidak menyentuh disk. Digunakan oleh kedua
/// implementasi sebelum menambah `root`.
fn validasi_jalur(jalur: &str) -> Result<PathBuf, String> {
    let path = Path::new(jalur);
    if path.is_absolute() {
        return Err(format!("jalur absolut dilarang: {jalur}"));
    }
    for comp in path.components() {
        if let Component::ParentDir = comp {
            return Err(format!("jalur '..' dilarang: {jalur}"));
        }
    }
    Ok(path.to_path_buf())
}

/// Implementasi nyata di atas `std::fs`. Semua jalur relatif terhadap `root`.
///
/// `root` dikanonkan saat konstruksi; tiap operasi memeriksa bahwa tujuan
/// (atau induknya, kalau belum ada) masih berada di dalam `root_kanonis`,
/// sehingga symlink yang menembus keluar juga ditolak.
#[derive(Debug)]
pub struct BerkasNyata {
    root: PathBuf,
    root_kanonis: PathBuf,
}

impl BerkasNyata {
    /// Buat kapasitas berkas dengan akar `root`.
    pub fn baru(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let root_kanonis = root.canonicalize().unwrap_or_else(|_| root.clone());
        Self {
            root,
            root_kanonis,
        }
    }

    /// Gabungkan `root` dengan jalur relatif `jalur`, setelah validasi dasar.
    fn gabung(&self, jalur: &str) -> Result<PathBuf, String> {
        let rel = validasi_jalur(jalur)?;
        Ok(self.root.join(rel))
    }

    /// Kembalikan jalur yang dipastikan berada di dalam `root`.
    ///
    /// Kalau `tidak_ada_ok` benar, target boleh belum ada (cocok untuk
    /// `tulis` dan `buat_folder`); maka induknya yang dikanonkan dan
    /// diperiksa. Kalau salah, target harus sudah ada.
    fn jalur_dalam_root(&self, jalur: &str, tidak_ada_ok: bool) -> Result<PathBuf, String> {
        let calon = self.gabung(jalur)?;
        if calon.exists() {
            let kanonis = calon
                .canonicalize()
                .map_err(|e| format!("tidak dapat mengkanoniskan {jalur}: {e}"))?;
            if !kanonis.starts_with(&self.root_kanonis) {
                return Err(format!("jalur menembus akar: {jalur}"));
            }
            return Ok(calon);
        }
        if tidak_ada_ok {
            match calon.parent() {
                Some(induk) => {
                    let kanonis_induk = induk
                        .canonicalize()
                        .map_err(|e| format!("tidak dapat mengkanoniskan induk {jalur}: {e}"))?;
                    if !kanonis_induk.starts_with(&self.root_kanonis) {
                        return Err(format!("jalur menembus akar: {jalur}"));
                    }
                    return Ok(calon);
                }
                None => {
                    return Err(format!("jalur tidak memiliki induk: {jalur}"));
                }
            }
        }
        Err(format!("berkas tidak ada: {jalur}"))
    }
}

impl Berkas for BerkasNyata {
    fn ada(&self, jalur: &str) -> bool {
        match self.gabung(jalur) {
            Ok(calon) => calon.exists(),
            Err(_) => false,
        }
    }

    fn baca(&self, jalur: &str) -> Result<Vec<u8>, String> {
        let target = self.jalur_dalam_root(jalur, false)?;
        std::fs::read(&target).map_err(|e| format!("baca {jalur} gagal: {e}"))
    }

    fn baca_teks(&self, jalur: &str) -> Result<String, String> {
        let byte = self.baca(jalur)?;
        String::from_utf8(byte)
            .map_err(|e| format!("baca_teks {jalur} bukan UTF-8: {e}"))
    }

    fn tulis(&self, jalur: &str, isi: &[u8]) -> Result<(), String> {
        let target = self.jalur_dalam_root(jalur, true)?;
        if let Some(induk) = target.parent() {
            let _ = std::fs::create_dir_all(induk);
        }
        std::fs::write(&target, isi).map_err(|e| format!("tulis {jalur} gagal: {e}"))
    }

    fn tulis_teks(&self, jalur: &str, isi: &str) -> Result<(), String> {
        self.tulis(jalur, isi.as_bytes())
    }

    fn tambah_teks(&self, jalur: &str, isi: &str) -> Result<(), String> {
        let target = self.jalur_dalam_root(jalur, true)?;
        if let Some(induk) = target.parent() {
            let _ = std::fs::create_dir_all(induk);
        }
        let mut berkas = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&target)
            .map_err(|e| format!("tambah_teks {jalur} gagal membuka: {e}"))?;
        berkas
            .write_all(isi.as_bytes())
            .map_err(|e| format!("tambah_teks {jalur} gagal menulis: {e}"))
    }

    fn daftar(&self, folder: &str) -> Result<Vec<String>, String> {
        let target = self.jalur_dalam_root(folder, false)?;
        if !target.is_dir() {
            return Err(format!("bukan folder: {folder}"));
        }
        let mut entri: Vec<String> = std::fs::read_dir(&target)
            .map_err(|e| format!("daftar {folder} gagal: {e}"))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        entri.sort();
        Ok(entri)
    }

    fn hapus(&self, jalur: &str) -> Result<(), String> {
        let target = self.jalur_dalam_root(jalur, false)?;
        std::fs::remove_file(&target).map_err(|e| format!("hapus {jalur} gagal: {e}"))
    }

    fn buat_folder(&self, jalur: &str) -> Result<(), String> {
        // `mkdir recursive`: induk belum tentu ada, jadi tidak bisa mengkanonkan
        // induk seperti `jalur_dalam_root`. `gabung` sudah menolak jalur absolut
        // dan komponen `..`.
        // ponytail: symlink di dalam root yang menunjuk keluar belum dideteksi;
        // tambahkan kanonis per komponen kalau plugin boleh membuat symlink.
        let target = self.gabung(jalur)?;
        std::fs::create_dir_all(&target).map_err(|e| format!("buat_folder {jalur} gagal: {e}"))
    }

    fn ukuran(&self, jalur: &str) -> Result<u64, String> {
        let target = self.jalur_dalam_root(jalur, false)?;
        std::fs::metadata(&target)
            .map(|m| m.len())
            .map_err(|e| format!("ukuran {jalur} gagal: {e}"))
    }

    fn salin(&self, dari: &str, ke: &str) -> Result<(), String> {
        let sumber = self.jalur_dalam_root(dari, false)?;
        let tujuan = self.jalur_dalam_root(ke, true)?;
        if let Some(induk) = tujuan.parent() {
            let _ = std::fs::create_dir_all(induk);
        }
        std::fs::copy(&sumber, &tujuan)
            .map(|_| ())
            .map_err(|e| format!("salin {dari} ke {ke} gagal: {e}"))
    }
}

/// Implementasi tiruan di memori untuk test.
///
/// Simpan byte di `HashMap<PathBuf, Vec<u8>>` di balik `Mutex`. Perilaku
/// metodanya sama dengan [`BerkasNyata`], tanpa menyentuh disk.
#[derive(Debug)]
pub struct BerkasPalsu {
    berkas: Mutex<HashMap<PathBuf, Vec<u8>>>,
}

impl BerkasPalsu {
    /// Buat kapasitas berkas tiruan kosong.
    pub fn baru() -> Self {
        Self {
            berkas: Mutex::new(HashMap::new()),
        }
    }

    /// Kunci `Mutex`, dengan pesan kesalahan jelas kalau terkena panah panik.
    fn kunci(&self) -> Result<std::sync::MutexGuard<'_, HashMap<PathBuf, Vec<u8>>>, String> {
        self.berkas
            .lock()
            .map_err(|e| format!("kunci berkas palsu rusak: {e}"))
    }
}

impl Berkas for BerkasPalsu {
    fn ada(&self, jalur: &str) -> bool {
        match validasi_jalur(jalur) {
            Ok(rel) => match self.kunci() {
                Ok(guard) => guard.contains_key(&rel),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    fn baca(&self, jalur: &str) -> Result<Vec<u8>, String> {
        let rel = validasi_jalur(jalur)?;
        let guard = self.kunci()?;
        guard
            .get(&rel)
            .cloned()
            .ok_or_else(|| format!("berkas tidak ada: {jalur}"))
    }

    fn baca_teks(&self, jalur: &str) -> Result<String, String> {
        let byte = self.baca(jalur)?;
        String::from_utf8(byte).map_err(|e| format!("baca_teks {jalur} bukan UTF-8: {e}"))
    }

    fn tulis(&self, jalur: &str, isi: &[u8]) -> Result<(), String> {
        let rel = validasi_jalur(jalur)?;
        let mut guard = self.kunci()?;
        guard.insert(rel, isi.to_vec());
        Ok(())
    }

    fn tulis_teks(&self, jalur: &str, isi: &str) -> Result<(), String> {
        self.tulis(jalur, isi.as_bytes())
    }

    fn tambah_teks(&self, jalur: &str, isi: &str) -> Result<(), String> {
        let rel = validasi_jalur(jalur)?;
        let mut guard = self.kunci()?;
        let isi_byte = isi.as_bytes();
        match guard.get_mut(&rel) {
            Some(sedang) => {
                sedang.extend_from_slice(isi_byte);
                Ok(())
            }
            None => {
                guard.insert(rel, isi_byte.to_vec());
                Ok(())
            }
        }
    }

    fn daftar(&self, folder: &str) -> Result<Vec<String>, String> {
        let folder_path = validasi_jalur(folder)?;
        let guard = self.kunci()?;
        let mut nama: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        for key in guard.keys() {
            let Ok(sisa) = key.strip_prefix(&folder_path) else {
                continue;
            };
            let mut comps = sisa.components();
            let Some(Component::Normal(pertama)) = comps.next() else {
                continue;
            };
            let nama_berkas = pertama.to_string_lossy().to_string();
            if seen.insert(nama_berkas.clone()) {
                nama.push(nama_berkas);
            }
        }
        nama.sort();
        Ok(nama)
    }

    fn hapus(&self, jalur: &str) -> Result<(), String> {
        let rel = validasi_jalur(jalur)?;
        let mut guard = self.kunci()?;
        if guard.remove(&rel).is_some() {
            Ok(())
        } else {
            Err(format!("berkas tidak ada: {jalur}"))
        }
    }

    fn buat_folder(&self, jalur: &str) -> Result<(), String> {
        // Folder bersifat implisit (diwakili oleh berkas di dalamnya), jadi
        // selalu berhasil dan idempoten, mirip `mkdir recursive`.
        let _ = validasi_jalur(jalur)?;
        Ok(())
    }

    fn ukuran(&self, jalur: &str) -> Result<u64, String> {
        let byte = self.baca(jalur)?;
        Ok(byte.len() as u64)
    }

    fn salin(&self, dari: &str, ke: &str) -> Result<(), String> {
        let dari_path = validasi_jalur(dari)?;
        let ke_path = validasi_jalur(ke)?;
        let mut guard = self.kunci()?;
        let isi = guard
            .get(&dari_path)
            .cloned()
            .ok_or_else(|| format!("berkas tidak ada: {dari}"))?;
        guard.insert(ke_path, isi);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Buat `BerkasNyata` yang menunjuk ke direktori sementara unik.
    fn nyata_sementara() -> (BerkasNyata, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oktz-berkas-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        (BerkasNyata::baru(&root), root)
    }

    #[test]
    fn jalur_larus_ditolak_nyata() {
        let (b, _root) = nyata_sementara();
        assert!(!b.ada(".."));
        assert!(!b.ada("../keluar"));
        assert!(!b.ada("/etc/passwd"));
        assert!(!b.ada("/tmp/../x"));
        assert!(b.tulis("../x", &[]).is_err());
        assert!(b.tulis("/abs", &[]).is_err());
        assert!(b.baca("../x").is_err());
        assert!(b.hapus("../x").is_err());
        assert!(b.buat_folder("../x").is_err());
    }

    #[test]
    fn jalur_larus_ditolak_palsu() {
        let b = BerkasPalsu::baru();
        assert!(!b.ada(".."));
        assert!(!b.ada("../keluar"));
        assert!(!b.ada("/etc/passwd"));
        assert!(b.tulis("../x", &[]).is_err());
        assert!(b.tulis("/abs", &[]).is_err());
        assert!(b.baca("../x").is_err());
    }

    #[test]
    fn buat_folder_idempoten() {
        let (b, _root) = nyata_sementara();
        assert!(b.buat_folder("a").is_ok());
        assert!(b.buat_folder("a").is_ok(), "buat_folder ulang tidak boleh error");
        assert!(b.buat_folder("a/b/c").is_ok());
        assert!(b.ada("a"));
        let p = BerkasPalsu::baru();
        assert!(p.buat_folder("a").is_ok());
        assert!(p.buat_folder("a").is_ok());
        assert!(p.buat_folder("a/b/c").is_ok());
    }

    #[test]
    fn berkas_tidak_ada_kembalikan_err_bukan_panic() {
        let (b, _root) = nyata_sementara();
        let hasil = std::panic::catch_unwind(|| b.baca("tidak-ada"));
        assert!(hasil.is_ok(), "baca berkas tidak ada tidak boleh panik");
        assert!(b.baca("tidak-ada").is_err());
        assert!(b.hapus("tidak-ada").is_err());
        assert!(b.ukuran("tidak-ada").is_err());

        let p = BerkasPalsu::baru();
        assert!(p.baca("tidak-ada").is_err());
        assert!(p.hapus("tidak-ada").is_err());
        assert!(p.ukuran("tidak-ada").is_err());
    }

    #[test]
    fn siklus_baca_tulis_hapus() {
        let (b, _root) = nyata_sementara();
        assert!(!b.ada("contoh.txt"));
        assert!(b.tulis_teks("contoh.txt", "halo").is_ok());
        assert!(b.ada("contoh.txt"));
        assert_eq!(b.baca_teks("contoh.txt").unwrap(), "halo");
        assert_eq!(b.ukuran("contoh.txt").unwrap(), 4);
        assert!(b.hapus("contoh.txt").is_ok());
        assert!(!b.ada("contoh.txt"));
    }

    #[test]
    fn tambah_teks_menambahkan_akhir() {
        let (b, _root) = nyata_sementara();
        assert!(b.tulis_teks("log.txt", "baris1\n").is_ok());
        assert!(b.tambah_teks("log.txt", "baris2\n").is_ok());
        assert_eq!(b.baca_teks("log.txt").unwrap(), "baris1\nbaris2\n");
    }

    #[test]
    fn daftar_mengembalikan_nama_saja() {
        let (b, _root) = nyata_sementara();
        assert!(b.buat_folder("sub").is_ok());
        assert!(b.tulis_teks("sub/x.txt", "x").is_ok());
        assert!(b.tulis_teks("sub/y.txt", "y").is_ok());
        let mut entri = b.daftar("sub").unwrap();
        entri.sort();
        assert_eq!(entri, vec!["x.txt".to_string(), "y.txt".to_string()]);
    }

    #[test]
    fn salin_berkas() {
        let (b, _root) = nyata_sementara();
        assert!(b.tulis_teks("asli.txt", "isi asli").is_ok());
        assert!(b.salin("asli.txt", "salinan.txt").is_ok());
        assert_eq!(b.baca_teks("salinan.txt").unwrap(), "isi asli");
    }

    #[test]
    fn palsu_berperilaku_sama_seperti_nyata() {
        let (b, _root) = nyata_sementara();
        let p = BerkasPalsu::baru();

        // keduanya mulai kosong
        assert!(!b.ada("a.txt"));
        assert!(!p.ada("a.txt"));

        // tulis + baca
        assert!(b.tulis_teks("a.txt", "hai").is_ok());
        assert!(p.tulis_teks("a.txt", "hai").is_ok());
        assert_eq!(b.baca_teks("a.txt").unwrap(), p.baca_teks("a.txt").unwrap());
        assert_eq!(b.ukuran("a.txt").unwrap(), p.ukuran("a.txt").unwrap());

        // tambah
        assert!(b.tambah_teks("a.txt", " lanjut").is_ok());
        assert!(p.tambah_teks("a.txt", " lanjut").is_ok());
        assert_eq!(b.baca_teks("a.txt").unwrap(), p.baca_teks("a.txt").unwrap());

        // hapus
        assert!(b.hapus("a.txt").is_ok());
        assert!(p.hapus("a.txt").is_ok());
        assert!(!b.ada("a.txt"));
        assert!(!p.ada("a.txt"));

        // daftar kosong
        assert_eq!(b.daftar(".").unwrap(), p.daftar(".").unwrap());

        // buat_folder idempoten keduanya
        assert!(b.buat_folder("d").is_ok());
        assert!(p.buat_folder("d").is_ok());
        assert!(b.buat_folder("d").is_ok());
        assert!(p.buat_folder("d").is_ok());
    }
}
