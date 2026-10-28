//! Kemampuan ffmpeg untuk oktz-plugin-api.
//!
//! Menyediakan pembungkus ringan di atas `std::process::Command` untuk
//! menjalankan biner ffmpeg tanpa menambah dependensi baru.
//!
//! Urutan resolusi biner:
//!   1. env `OTKZ_FFMPEG_PATH` (hormati bila berkasnya ada dan dieksekusi)
//!   2. `ffmpeg` yang tersedia di `PATH`

use std::path::PathBuf;
use std::process::Command;
use std::io::Write;

/// Jalur biner ffmpeg.
/// Urutan: env `OTKZ_FFMPEG_PATH`, lalu `ffmpeg` di `PATH`.
pub fn jalur_ffmpeg() -> Option<PathBuf> {
    // 1. override eksplisit dari operator melalui env.
    if let Some(nama) = std::env::var_os("OTKZ_FFMPEG_PATH") {
        let calon = PathBuf::from(nama);
        if calon.is_file() {
            return Some(calon);
        }
    }

    // 2. biner yang sudah ada di PATH.
    cari_di_path("ffmpeg")
}

/// Jalur biner ffprobe.
/// Urutan: env `OTKZ_FFPROBE_PATH`, lalu `ffprobe` di `PATH`.
pub fn jalur_ffprobe() -> Option<PathBuf> {
    if let Some(nama) = std::env::var_os("OTKZ_FFPROBE_PATH") {
        let calon = PathBuf::from(nama);
        if calon.is_file() {
            return Some(calon);
        }
    }
    cari_di_path("ffprobe")
}

/// Durasi media dalam detik dari stdin (`pipe:0`).
///
/// `None` kalau ffprobe tidak tersedia atau keluarannya bukan angka — sama
/// seperti cabang `catch {}` JS yang melanjutkan tanpa menolak video.
pub fn durasi_dari_masukan(masukan: &[u8]) -> Option<f64> {
    let biner = jalur_ffprobe()?;
    let args = [
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        "pipe:0",
    ];
    let keluaran = Command::new(&biner)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut keluaran = keluaran;
    if let Some(stdin) = keluaran.stdin.as_mut() {
        stdin.write_all(masukan).ok()?;
    }
    let hasil = keluaran.wait_with_output().ok()?;
    if !hasil.status.success() {
        return None;
    }
    let teks = String::from_utf8_lossy(&hasil.stdout);
    teks.trim().parse::<f64>().ok()
}

/// Jalankan ffmpeg dengan argumen mentah, kembalikan stdout biner.
/// `Err` memuat stderr supaya bisa ditampilkan seperti JS.
pub fn jalankan(argumen: &[&str]) -> Result<Vec<u8>, String> {
    let biner = jalur_ffmpeg().unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let keluaran = Command::new(&biner)
        .args(argumen)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("gagal menjalankan ffmpeg: {e}"))?;

    if keluaran.status.success() {
        Ok(keluaran.stdout)
    } else {
        Err(String::from_utf8_lossy(&keluaran.stderr).to_string())
    }
}

/// Sama seperti `jalankan`, tapi stdin diisi `masukan` (ffmpeg baca dari `pipe:0`).
pub fn jalankan_dengan_masukan(argumen: &[&str], masukan: &[u8]) -> Result<Vec<u8>, String> {
    let biner = jalur_ffmpeg().unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let mut anak = Command::new(&biner)
        .args(argumen)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("gagal menjalankan ffmpeg: {e}"))?;

    {
        let stdin = anak
            .stdin
            .as_mut()
            .ok_or_else(|| "tidak bisa membuka stdin ffmpeg".to_string())?;
        stdin
            .write_all(masukan)
            .map_err(|e| format!("gagal menulis ke stdin ffmpeg: {e}"))?;
    }

    let keluaran = anak
        .wait_with_output()
        .map_err(|e| format!("gagal menunggu ffmpeg: {e}"))?;

    if keluaran.status.success() {
        Ok(keluaran.stdout)
    } else {
        Err(String::from_utf8_lossy(&keluaran.stderr).to_string())
    }
}

/// `-y -i <masuk> ...` dengan keluaran ditulis ke `keluar` (berkas terakhir
/// dalam `argumen`). Mengembalikan `()` saat berhasil, `Err` memuat stderr.
pub fn jalankan_ke_berkas(argumen: &[&str]) -> Result<(), String> {
    let biner = jalur_ffmpeg().unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let keluaran = Command::new(&biner)
        .args(argumen)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("gagal menjalankan ffmpeg: {e}"))?;

    if keluaran.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&keluaran.stderr).to_string())
    }
}

/// Cari `nama` biner di semua direktori `PATH`.
/// Pada Windows mencoba `nama.exe` terlebih dahulu.
fn cari_di_path(nama: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").ok()?;
    for direktori in std::env::split_paths(&path) {
        let mut calon = direktori.join(nama);
        if cfg!(target_os = "windows") && !calon.to_string_lossy().ends_with(".exe") {
            calon.set_extension("exe");
        }
        if calon.is_file() {
            return Some(calon);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::sync::Mutex;

    /// Test memutasi env global; jalankan berurutan supaya tidak balapan.
    static ENV_KUNCI: Mutex<()> = Mutex::new(());

    /// Buat berkas dummy bernama `ffmpeg` di direktori temp, kembalikan
    /// (direktori, path_berkas, PATH_asal).
    fn siapkan_env() -> (std::path::PathBuf, std::path::PathBuf, String) {
        let dir = env::temp_dir().join(format!("oktz-ffmpeg-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("ffmpeg");
        File::create(&bin).unwrap().write_all(b"# dummy ffmpeg").unwrap();
        let path_asal = env::var("PATH").unwrap_or_default();
        (dir, bin, path_asal)
    }

    fn bersihkan_env(dir: &std::path::Path, path_asal: &str) {
        let _ = fs::remove_dir_all(dir);
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::set_var("PATH", path_asal); }
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::remove_var("OTKZ_FFMPEG_PATH"); }
    }

    #[test]
    fn jalur_ffmpeg_hormati_env_bila_berkas_ada() {
        let _kunci = ENV_KUNCI.lock().unwrap_or_else(|e| e.into_inner());
        let (dir, bin, path_asal) = siapkan_env();
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::set_var("OTKZ_FFMPEG_PATH", &bin); }
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::set_var("PATH", ""); }
        let hasil = jalur_ffmpeg();
        assert_eq!(hasil.as_deref(), Some(Path::new(&bin)));
        bersihkan_env(&dir, &path_asal);
    }

    #[test]
    fn jalur_ffmpeg_none_bila_env_salah_dan_tidak_ada_di_path() {
        let _kunci = ENV_KUNCI.lock().unwrap_or_else(|e| e.into_inner());
        let (dir, _bin, path_asal) = siapkan_env();
        // PATH diarahkan ke direktori yang hanya berisi dummy, bukan ffmpeg asli,
        // tapi dummy itu sendiri bernama ffmpeg — gunakan direktori kosong lain.
        let kosong = dir.join("kosong");
        fs::create_dir_all(&kosong).unwrap();
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::set_var("PATH", &kosong); }
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::remove_var("OTKZ_FFMPEG_PATH"); }
        assert!(jalur_ffmpeg().is_none());
        bersihkan_env(&dir, &path_asal);
    }

    #[test]
    fn jalur_ffmpeg_cari_di_path_saat_env_kosong() {
        let _kunci = ENV_KUNCI.lock().unwrap_or_else(|e| e.into_inner());
        let (dir, bin, path_asal) = siapkan_env();
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::remove_var("OTKZ_FFMPEG_PATH"); }
        // Mutasi env global: hanya aman karena test dijalankan sinkron di modul ini.
        unsafe { env::set_var("PATH", &dir); }
        let hasil = jalur_ffmpeg();
        assert_eq!(hasil.as_deref(), Some(Path::new(&bin)));
        bersihkan_env(&dir, &path_asal);
    }

    #[test]
    fn argumen_audio_ke_mp3_tidak_didefinisikan() {
        // Tidak ada pola argumen ffmpeg yang berulang di seluruh plugin
        // (toaudio mp3, tovn opus/ogg, sticker -vf, ttmp3 fluent-ffmpeg),
        // jadi helper argumen_audio_ke_mp3 sengaja tidak dibuat.
    }
}
