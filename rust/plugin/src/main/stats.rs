//! `plugins/main/stats.js` — statistik bot.
//!
//! `ponytail:` `sendTableV2` (tabel AI) belum punya varian di `Reply`; header,
//! baris, dan footer dirangkai sebagai teks dengan isi serta urutan yang sama
//! (pola `main/totalfitur.rs`).
//! `ponytail:` `Ctx` hanya membawa data user/grup pengirim, bukan seluruh
//! store; Users/Premium/Groups dihitung dari data itu, bukan total.
//! `ponytail:` `process.memoryUsage()` tidak ada; Heap memakai VmRSS/VmSize
//! dari `/proc/self/status` — padanan terdekat.
//! `ponytail:` `process.version` tidak punya padanan di Rust; kolom Node
//! diisi `N/A`.

use std::fs;

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

/// `Number.prototype.toFixed(2)` — setengah menjauhi nol seperti JS, bukan
/// setengah ke genap seperti format float bawaan Rust.
fn fixed2(nilai: f64) -> String {
    format!("{:.2}", (nilai * 100.0).round() / 100.0)
}

/// `parseFloat(x.toFixed(2))` — nol ekor dan titik sisa dibuang.
fn buang_nol_ekor(teks: &str) -> String {
    let bersih = teks.trim_end_matches('0').trim_end_matches('.');
    if bersih.is_empty() {
        "0".to_string()
    } else {
        bersih.to_string()
    }
}

/// `formatBytes` (`stats.js:20-26`) — satuan sampai `GB`; di luar itu JS
/// mencetak `undefined`.
fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let satuan = ["B", "KB", "MB", "GB"];
    let angka = bytes as f64;
    let i = (angka.ln() / 1024f64.ln()).floor() as usize;
    let nilai = angka / 1024f64.powi(i as i32);
    format!(
        "{} {}",
        buang_nol_ekor(&fixed2(nilai)),
        satuan.get(i).copied().unwrap_or("undefined")
    )
}

/// `formatUptime` (`stats.js:28-42`) — hari/jam/menit/detik, satuan nol
/// dibuang; kalau semua nol tetap `0s`.
fn format_uptime(ms: u64) -> String {
    let detik = ms / 1000;
    let hari = detik / 86_400;
    let jam = (detik % 86_400) / 3600;
    let menit = (detik % 3600) / 60;
    let sisa = detik % 60;

    let mut bagian: Vec<String> = Vec::new();
    if hari > 0 {
        bagian.push(format!("{hari}d"));
    }
    if jam > 0 {
        bagian.push(format!("{jam}h"));
    }
    if menit > 0 {
        bagian.push(format!("{menit}m"));
    }
    if sisa > 0 || bagian.is_empty() {
        bagian.push(format!("{sisa}s"));
    }
    bagian.join(" ")
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

/// `Object.keys(db.db.data.users/groups)` dan `u.premium` tidak ada di `Ctx`;
/// yang tersedia hanya data pengirim, jadi itulah yang dihitung.
fn hitung_db(ctx: &Ctx) -> (usize, usize, usize) {
    let users = ctx.user_data.is_object() as usize;
    let groups = ctx.group_data.is_object() as usize;
    let premium = crate::common::js_truthy(ctx.user_data.get("premium")) as usize;
    (users, premium, groups)
}

/// `new Date().toLocaleTimeString("id-ID", { timeZone: "Asia/Jakarta" })`.
fn jam_jakarta() -> String {
    let (_, _, _, jam, menit, detik) = crate::common::jakarta_parts(crate::common::now_ms());
    format!("{jam:02}.{menit:02}.{detik:02}")
}

fn teks(ctx: &Ctx) -> String {
    let cfg = ctx.config();
    let nama_bot = cfg.bot.name.as_str();
    let bot = if nama_bot.is_empty() { "Bot" } else { nama_bot };
    let versi = if cfg.bot.version.is_empty() {
        "1.0.0"
    } else {
        cfg.bot.version.as_str()
    };

    let (users, premium, groups) = hitung_db(ctx);
    let (total_mem, bebas_mem) = baca_memori();
    let (heap_dipakai, heap_total) = baca_heap();

    let mut out =
        String::from("📊 Berikut ini adalah statistik dari bot kami\n\nKey | Value\n");
    out.push_str(&format!("Bot | {bot}\n"));
    out.push_str(&format!("Version | v{versi}\n"));
    out.push_str(&format!(
        "Uptime | {}\n",
        format_uptime(ctx.uptime_secs.saturating_mul(1000))
    ));
    out.push_str(&format!("Users | {users}\n"));
    out.push_str(&format!("Premium | {premium}\n"));
    out.push_str(&format!("Groups | {groups}\n"));
    out.push_str(&format!("Platform | {}\n", platform()));
    out.push_str("Node | N/A\n");
    out.push_str(&format!("CPU Load | {}%\n", fixed2(baca_load())));
    out.push_str(&format!(
        "RAM | {} / {}\n",
        format_bytes(total_mem.saturating_sub(bebas_mem)),
        format_bytes(total_mem)
    ));
    out.push_str(&format!(
        "Heap | {} / {}\n",
        format_bytes(heap_dipakai),
        format_bytes(heap_total)
    ));
    out.push_str(&format!("Updated | {}", jam_jakarta()));
    if !nama_bot.is_empty() {
        out.push_str(&format!("\n\n{nama_bot}"));
    }
    out
}

#[plugin(
    name = "stats",
    alias = ["botstats", "stat"],
    category = "main",
    description = "Menampilkan statistik bot",
    usage = ".stats",
    example = ".stats",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stats(ctx: Ctx) -> Reply {
    Reply::text(teks(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_sama_dengan_js() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1 MB");
        assert_eq!(format_bytes(1_073_741_824), "1 GB");
        assert_eq!(format_bytes(123_456_789), "117.74 MB");
    }

    #[test]
    fn format_uptime_sama_dengan_js() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(999), "0s");
        assert_eq!(format_uptime(1000), "1s");
        assert_eq!(format_uptime(59_000), "59s");
        assert_eq!(format_uptime(60_000), "1m");
        assert_eq!(format_uptime(61_000), "1m 1s");
        assert_eq!(format_uptime(3_600_000), "1h");
        assert_eq!(format_uptime(3_661_000), "1h 1m 1s");
        assert_eq!(format_uptime(86_400_000), "1d");
        assert_eq!(format_uptime(86_399_999), "23h 59m 59s");
        assert_eq!(format_uptime(90_061_000), "1d 1h 1m 1s");
        assert_eq!(format_uptime(86_461_000), "1d 1m 1s");
        assert_eq!(format_uptime(176_461_000), "2d 1h 1m 1s");
    }

    #[test]
    fn fixed2_membulat_setengah_menjauhi_nol() {
        assert_eq!(fixed2(0.125), "0.13");
        assert_eq!(fixed2(1.0), "1.00");
        assert_eq!(fixed2(117.737_564_086_914_06), "117.74");
    }
}
