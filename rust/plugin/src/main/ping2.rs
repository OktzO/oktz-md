//! `plugins/main/ping2.js` — performa dan status sistem bot real-time.
//!
//! `ponytail:` `sendTable` belum punya varian di `Reply`; header, tabel, dan
//! footer dirangkai sebagai teks dengan isi serta urutan yang sama (pola
//! `main/totalfitur.rs`).
//! `ponytail:` data sistem dibaca dari `/proc` + `std`; `df` tetap dipanggil
//! lewat `std::process::Command` karena `statvfs` tidak ada di `std`.
//! `ponytail:` `Ctx` hanya membawa data user/grup pengirim, bukan seluruh
//! store; Users/Premium/Groups dihitung dari data itu, bukan total.
//! `ponytail:` `process.memoryUsage()`, `process.version`, dan
//! `m.messageTimestamp` tidak ada; Heap = VmRSS/VmSize, Node = `N/A`,
//! WA Roundtrip = 1 ms (cabang `Date.now()` di JS).

use std::fs;
use std::process::Command;
use std::time::Instant;

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `os.platform()` + `os.arch()` dengan nama versi Node (`darwin`, `x64`).
fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        lain => lain,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        "arm" => "arm",
        lain => lain,
    };
    format!("{os} {arch}")
}

/// `fmtSize` (`ping2.js:24-29`) — satu desimal, `0 B` untuk nol; di luar `TB`
/// JS mencetak `undefined`.
fn fmt_size(b: u64) -> String {
    if b == 0 {
        return "0 B".to_string();
    }
    let satuan = ["B", "KB", "MB", "GB", "TB"];
    let angka = b as f64;
    let i = (angka.ln() / 1024f64.ln()).floor() as usize;
    let nilai = angka / 1024f64.powi(i as i32);
    format!(
        "{:.1} {}",
        (nilai * 10.0).round() / 10.0,
        satuan.get(i).copied().unwrap_or("undefined")
    )
}

/// `fmtUp` (`ping2.js:31-37`) — detik, tanpa satuan nol di bawahnya.
fn fmt_up(s: f64) -> String {
    let d = (s / 86_400.0).floor();
    let h = (s % 86_400.0 / 3600.0).floor();
    let m = (s % 3600.0 / 60.0).floor();
    let sc = (s % 60.0).floor();
    if d > 0.0 {
        format!("{}d {}h {}m", d as i64, h as i64, m as i64)
    } else if h > 0.0 {
        format!("{}h {}m {}s", h as i64, m as i64, sc as i64)
    } else {
        format!("{}m {}s", m as i64, sc as i64)
    }
}

/// `os.loadavg()[0]` dari `/proc/loadavg`.
fn baca_load() -> f64 {
    fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|teks| {
            teks.split_whitespace()
                .next()
                .and_then(|nilai| nilai.parse().ok())
        })
        .unwrap_or(0.0)
}

/// `os.cpus()` — jumlah dan `model` CPU pertama dari `/proc/cpuinfo`;
/// fallback jumlah ke `available_parallelism`.
fn baca_cpu() -> (usize, String) {
    let teks = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let (mut jumlah, mut model) = (0usize, String::new());
    for baris in teks.lines() {
        let Some((kunci, nilai)) = baris.split_once(':') else {
            continue;
        };
        match kunci.trim() {
            "processor" => jumlah += 1,
            "model name" if model.is_empty() => model = nilai.trim().to_string(),
            _ => {}
        }
    }
    if jumlah == 0 {
        jumlah = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
    }
    (jumlah, model)
}

/// `cpus[0]?.model?.slice(0, 25)`; model kosong jadi `N/A`.
fn potong_model(model: &str) -> String {
    if model.is_empty() {
        "N/A".to_string()
    } else {
        model.chars().take(25).collect()
    }
}

/// `Math.max(1, Math.min(100, os.loadavg()[0] / cpus.length * 100)).toFixed(1)`.
fn cpu_persen(load: f64, cores: usize) -> String {
    let mentah = load / cores as f64 * 100.0;
    let pct = mentah.clamp(1.0, 100.0);
    format!("{:.1}", (pct * 10.0).round() / 10.0)
}

/// `os.totalmem()` / `os.freemem()` dari `/proc/meminfo` (kB -> byte).
fn baca_memori() -> (u64, u64) {
    let teks = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let (mut total, mut bebas) = (0u64, 0u64);
    for baris in teks.lines() {
        let Some((kunci, nilai)) = baris.split_once(':') else {
            continue;
        };
        let kb = nilai
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);
        match kunci.trim() {
            "MemTotal" => total = kb.saturating_mul(1024),
            "MemFree" => bebas = kb.saturating_mul(1024),
            _ => {}
        }
    }
    (total, bebas)
}

/// `process.memoryUsage().heapUsed/.heapTotal` — `ponytail:` padanan terdekat
/// adalah VmRSS/VmSize proses ini (byte), bukan heap V8.
fn baca_heap() -> (u64, u64) {
    let teks = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let (mut dipakai, mut total) = (0u64, 0u64);
    for baris in teks.lines() {
        let Some((kunci, nilai)) = baris.split_once(':') else {
            continue;
        };
        let kb = nilai
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);
        match kunci.trim() {
            "VmRSS" => dipakai = kb.saturating_mul(1024),
            "VmSize" => total = kb.saturating_mul(1024),
            _ => {}
        }
    }
    (dipakai, total)
}

/// `os.uptime()` dari `/proc/uptime` (detik pecahan).
fn baca_uptime_server() -> f64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|teks| {
            teks.split_whitespace()
                .next()
                .and_then(|nilai| nilai.parse().ok())
        })
        .unwrap_or(0.0)
}

/// `os.hostname()` dari `/proc/sys/kernel/hostname`.
fn baca_hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|teks| teks.trim().to_string())
        .filter(|teks| !teks.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "N/A".to_string())
}

/// `df -k --output=size,used /` (kiB di JS, byte di sini); 0 kalau perintah
/// tidak tersedia. Cabang `wmic` Windows tidak diport.
fn baca_disk() -> (u64, u64) {
    let keluaran = Command::new("df")
        .args(["-k", "--output=size,used", "/"])
        .output()
        .ok();
    let Some(keluaran) = keluaran else {
        return (0, 0);
    };
    let teks = String::from_utf8_lossy(&keluaran.stdout);
    let Some(baris) = teks.lines().nth(1) else {
        return (0, 0);
    };
    let mut kolom = baris.split_whitespace();
    let total = kolom.next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
    let dipakai = kolom.next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
    (dipakai.saturating_mul(1024), total.saturating_mul(1024))
}

/// `os.networkInterfaces()` — nama interface IPv4 non-loopback terakhir;
/// IPv4 dideteksi dari tabel rute `/proc/net/route`.
fn antarmuka_jaringan() -> String {
    let rute = fs::read_to_string("/proc/net/route").unwrap_or_default();
    let dev = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let beripv4: Vec<&str> = rute
        .lines()
        .skip(1)
        .filter_map(|baris| baris.split_whitespace().next())
        .collect();

    let mut aktif = "N/A".to_string();
    for baris in dev.lines().skip(2) {
        let nama = baris.split(':').next().unwrap_or("").trim();
        if nama.is_empty() || nama.to_lowercase().contains("lo") {
            continue;
        }
        if beripv4.contains(&nama) {
            aktif = nama.to_string();
        }
    }
    aktif
}

/// `Object.keys(db.data.users/groups)` dan `u.isPremium` tidak ada di `Ctx`;
/// yang tersedia hanya data pengirim, jadi itulah yang dihitung.
fn hitung_db(ctx: &Ctx) -> (usize, usize, usize) {
    let users = ctx.user_data.is_object() as usize;
    let groups = ctx.group_data.is_object() as usize;
    let premium = crate::common::js_truthy(ctx.user_data.get("isPremium")) as usize;
    (users, premium, groups)
}

fn teks(ctx: &Ctx, mulai: Instant) -> String {
    let cfg = ctx.config();
    let nama_bot = if cfg.bot.name.is_empty() {
        "Foto-AI"
    } else {
        cfg.bot.name.as_str()
    };

    let (cores, model) = baca_cpu();
    let (total_mem, bebas_mem) = baca_memori();
    let (heap_dipakai, heap_total) = baca_heap();
    let (disk_dipakai, disk_total) = baca_disk();
    let (users, premium, groups) = hitung_db(ctx);
    let total_exec = (mulai.elapsed().as_secs_f64() * 1000.0).round() as u64;

    let mut out = format!(
        "{nama_bot} *STATUS*\n\n\
         - 🎄 Dibawah ini adalah statistik bot kita\n\n\
         ⚡ System Performance\n\
         Metric | Value\n"
    );
    out.push_str("WA Roundtrip | 1 ms\n");
    out.push_str(&format!("Kecepatan Respon bot mu | {total_exec} ms\n"));
    out.push_str("Status | Online\n");
    out.push_str(&format!("Hostname | {}\n", baca_hostname()));
    out.push_str(&format!("Platform | {}\n", platform()));
    out.push_str("Node | N/A\n");
    out.push_str(&format!("CPU | {}\n", potong_model(&model)));
    out.push_str(&format!("Cores | {cores}\n"));
    out.push_str(&format!(
        "CPU Load | {}%\n",
        cpu_persen(baca_load(), cores)
    ));
    out.push_str(&format!(
        "RAM | {} / {}\n",
        fmt_size(total_mem.saturating_sub(bebas_mem)),
        fmt_size(total_mem)
    ));
    out.push_str(&format!(
        "Heap | {} / {}\n",
        fmt_size(heap_dipakai),
        fmt_size(heap_total)
    ));
    out.push_str(&format!(
        "Disk | {} / {}\n",
        fmt_size(disk_dipakai),
        fmt_size(disk_total)
    ));
    out.push_str(&format!("Network | {}\n", antarmuka_jaringan()));
    out.push_str(&format!("Users | {users}\n"));
    out.push_str(&format!("Premium | {premium}\n"));
    out.push_str(&format!("Groups | {groups}\n"));
    out.push_str(&format!(
        "Uptime Bot | {}\n",
        fmt_up(ctx.uptime_secs as f64)
    ));
    out.push_str(&format!("Uptime Server | {}\n", fmt_up(baca_uptime_server())));
    out.push_str("\n🍃 Realtime Monitoring");
    out
}

#[plugin(
    name = "ping2",
    alias = ["speed2", "p2", "latency2", "sys2", "status2"],
    category = "main",
    description = "Cek performa dan status sistem bot secara real-time",
    usage = ".ping2",
    example = ".ping2",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ping2(ctx: Ctx) -> Reply {
    let mulai = Instant::now();
    // `ponytail:` reaksi pembuka `🕕` hilang; `Reply` hanya membawa satu
    // reaksi, jadi yang dikirim reaksi akhir `✅` seperti JS.
    Reply::text(teks(&ctx, mulai)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_size_satu_desimal_seperti_js() {
        assert_eq!(fmt_size(0), "0 B");
        assert_eq!(fmt_size(1), "1.0 B");
        assert_eq!(fmt_size(1023), "1023.0 B");
        assert_eq!(fmt_size(1024), "1.0 KB");
        assert_eq!(fmt_size(1536), "1.5 KB");
        assert_eq!(fmt_size(1_048_576), "1.0 MB");
        assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
        assert_eq!(fmt_size(1_099_511_627_776), "1.0 TB");
        assert_eq!(fmt_size(123_456_789), "117.7 MB");
    }

    #[test]
    fn fmt_up_sesuai_cabang_js() {
        assert_eq!(fmt_up(0.0), "0m 0s");
        assert_eq!(fmt_up(59.0), "0m 59s");
        assert_eq!(fmt_up(60.0), "1m 0s");
        assert_eq!(fmt_up(3599.0), "59m 59s");
        assert_eq!(fmt_up(3600.0), "1h 0m 0s");
        assert_eq!(fmt_up(86_399.0), "23h 59m 59s");
        assert_eq!(fmt_up(86_400.0), "1d 0h 0m");
        assert_eq!(fmt_up(90_000.0), "1d 1h 0m");
        assert_eq!(fmt_up(90_061.0), "1d 1h 1m");
    }

    #[test]
    fn cpu_persen_selalu_1_sampai_100() {
        assert_eq!(cpu_persen(1.0, 4), "25.0");
        assert_eq!(cpu_persen(0.5, 4), "12.5");
        assert_eq!(cpu_persen(8.0, 4), "100.0");
        assert_eq!(cpu_persen(0.0, 4), "1.0");
        assert_eq!(cpu_persen(0.0, 0), "NaN");
    }

    #[test]
    fn model_cpu_dipotong_25_karakter() {
        assert_eq!(potong_model(""), "N/A");
        assert_eq!(
            potong_model("abcdefghijklmnopqrstuvwxyz"),
            "abcdefghijklmnopqrstuvwxy"
        );
        assert_eq!(
            potong_model("Intel(R) Xeon(R) CPU E5-2670 v3 @ 2.30GHz").len(),
            25
        );
    }
}
