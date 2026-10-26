//! `plugins/main/ping2.js` — performa dan status sistem bot real-time.
//!
//! Balasan memakai tabel AI (`Reply::tabel`) — padanan `sock.sendTable` JS:
//! judul tabel, baris heading, baris isi, teks sebelum tabel (`headerText`),
//! dan teks setelah tabel (`footer`), isi serta urutannya sama dengan JS.
//! `ponytail:` tabel versi 1 (`tableMetadata`), bukan versi 2 —
//! `unifiedResponse` base64 dan `botMetadata` versi JS v2 belum di-port.
//! `ponytail:` data sistem dibaca dari `/proc` + `std`; `df` tetap dipanggil
//! lewat `std::process::Command` karena `statvfs` tidak ada di `std`.
//! `ponytail:` `process.memoryUsage()`, `process.version`, dan
//! `m.messageTimestamp` tidak ada; Heap = VmRSS/VmSize, Node = `N/A`,
//! WA Roundtrip = 1 ms (cabang `Date.now()` di JS).
//!
//! Urutan kirim meniru handler JS: tabel dulu, BARU reaksi `✅`
//! (`ping2.js:129-141`). Reaksi pembuka `🕕` (`ping2.js:60`) dikirim sebelum
//! tabel dan dibawa `Reply::Sequence`.

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
/// `ctx.store_users`/`store_premium`/`store_groups` dirangkum pipeline dari
/// snapshot store yang sama dengan yang dipakai gate permission.
fn hitung_db(ctx: &Ctx) -> (usize, usize, usize) {
    (ctx.store_users, ctx.store_premium, ctx.store_groups)
}

/// Susun tabel AI — padanan `sock.sendTable` JS (`ping2.js:129-139`).
///
/// Urutan baris sama persis dengan `tableData` JS; tiap `.with_table_row`
/// menambah satu baris.
fn reply(ctx: &Ctx, mulai: Instant) -> Reply {
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

    let mut t = Reply::tabel("⚡ System Performance")
        .with_table_header(["Metric", "Value"])
        .with_table_header_text(format!(
            "{nama_bot} *STATUS*\n\n- 🎄 Dibawah ini adalah statistik bot kita"
        ));

    t = t.with_table_row(["WA Roundtrip", "1 ms"]);
    t = t.with_table_row([
        "Kecepatan Respon bot mu".to_string(),
        format!("{total_exec} ms"),
    ]);
    t = t.with_table_row(["Status", "Online"]);
    t = t.with_table_row(["Hostname".to_string(), baca_hostname()]);
    t = t.with_table_row(["Platform".to_string(), platform()]);
    t = t.with_table_row(["Node".to_string(), "N/A".to_string()]);
    t = t.with_table_row(["CPU".to_string(), potong_model(&model)]);
    t = t.with_table_row(["Cores".to_string(), cores.to_string()]);
    t = t.with_table_row([
        "CPU Load".to_string(),
        format!("{}%", cpu_persen(baca_load(), cores)),
    ]);
    t = t.with_table_row([
        "RAM".to_string(),
        format!(
            "{} / {}",
            fmt_size(total_mem.saturating_sub(bebas_mem)),
            fmt_size(total_mem)
        ),
    ]);
    t = t.with_table_row([
        "Heap".to_string(),
        format!("{} / {}", fmt_size(heap_dipakai), fmt_size(heap_total)),
    ]);
    t = t.with_table_row([
        "Disk".to_string(),
        format!("{} / {}", fmt_size(disk_dipakai), fmt_size(disk_total)),
    ]);
    t = t.with_table_row(["Network".to_string(), antarmuka_jaringan()]);
    t = t.with_table_row(["Users".to_string(), users.to_string()]);
    t = t.with_table_row(["Premium".to_string(), premium.to_string()]);
    t = t.with_table_row(["Groups".to_string(), groups.to_string()]);
    t = t.with_table_row([
        "Uptime Bot".to_string(),
        fmt_up(ctx.uptime_secs as f64),
    ]);
    t = t.with_table_row([
        "Uptime Server".to_string(),
        fmt_up(baca_uptime_server()),
    ]);

    t.with_table_footer("🍃 Realtime Monitoring")
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
    // `await m.react('🕕')` (`ping2.js:60`) lalu tabel lalu `await m.react('✅')`
    // (`:141`). Urutan itu bisa disusun karena `Reply::Sequence` dikirim
    // berurutan: reaksi → tabel → reaksi.
    Reply::sequence(vec![
        Reply::reaction("🕕"),
        reply(&ctx, mulai),
        Reply::reaction("✅"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Balasan `ping2`: satu `Sequence` [reaksi, tabel, reaksi].
    fn isi_sequence(reply: &Reply) -> &[Reply] {
        match reply.without_mutations() {
            Reply::Sequence(list) => list,
            other => panic!("harus Sequence, dapat {other:?}"),
        }
    }

    /// Ambil tabel di dalam sequence.
    fn tabel_dari(reply: &Reply) -> &oktz_plugin_api::Tabel {
        let [_, tabel, _] = isi_sequence(reply) else {
            panic!("harus tiga elemen: reaksi, tabel, reaksi");
        };
        match tabel.tanpa_reaksi() {
            Reply::Tabel { tabel } => tabel,
            other => panic!("harus Tabel, dapat {other:?}"),
        }
    }

    #[test]
    fn ping2_uses_stable_global_counts_and_reaction_after_table() {
        let r = ping2(Ctx::default());
        let list = isi_sequence(&r);

        // Urutan event JS: reaksi pembuka → tabel → reaksi `✅`. Bukan
        // `✅` lebih dulu (`with_reaction` mengirim reaksi SEBELUM balasan).
        assert!(
            matches!(&list[0], Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "elemen pertama harus reaksi 🕕, dapat {:?}",
            list[0]
        );
        assert!(
            matches!(list[1].tanpa_reaksi(), Reply::Tabel { .. }),
            "tabel harus dikirim SETELAH reaksi pembuka, dapat {:?}",
            list[1]
        );
        assert!(
            matches!(&list[2], Reply::Reaction { emoji, .. } if emoji == "✅"),
            "reaksi ✅ harus SESUDAH tabel, dapat {:?}",
            list[2]
        );

        // Angka Users/Premium/Groups dari SELURUH store (`ping2.js:100-102`),
        // jadi tetap stabil walau `user_data`/`group_data` pengirim terisi.
        let ctx = Ctx {
            user_data: serde_json::json!({"isPremium": true}),
            group_data: serde_json::json!({"name": "Grup"}),
            store_users: 12,
            store_premium: 4,
            store_groups: 7,
            ..Default::default()
        };
        let balasan = ping2(ctx);
        let t = tabel_dari(&balasan);
        let baris = |kunci: &str| {
            t.baris
                .iter()
                .find(|b| b[0] == kunci)
                .unwrap_or_else(|| panic!("baris {kunci} hilang"))
                .clone()
        };
        assert_eq!(baris("Users"), vec!["Users", "12"]);
        assert_eq!(baris("Premium"), vec!["Premium", "4"]);
        assert_eq!(baris("Groups"), vec!["Groups", "7"]);
    }

    #[test]
    fn balasan_membentuk_tabel_dengan_isi_dan_urutan_js() {
        let r = ping2(Ctx::default());
        let t = tabel_dari(&r);

        assert_eq!(t.judul, "⚡ System Performance");
        assert_eq!(t.header, vec!["Metric", "Value"]);
        // `tableData` JS punya 18 baris, urutan sama.
        assert_eq!(t.baris.len(), 18);
        assert_eq!(t.baris[0], vec!["WA Roundtrip", "1 ms"]);
        assert_eq!(t.baris[1][0], "Kecepatan Respon bot mu");
        assert_eq!(t.baris[2], vec!["Status", "Online"]);
        assert_eq!(t.baris[17][0], "Uptime Server");

        let kunci: Vec<&str> = t.baris.iter().map(|b| b[0].as_str()).collect();
        assert_eq!(
            kunci,
            vec![
                "WA Roundtrip",
                "Kecepatan Respon bot mu",
                "Status",
                "Hostname",
                "Platform",
                "Node",
                "CPU",
                "Cores",
                "CPU Load",
                "RAM",
                "Heap",
                "Disk",
                "Network",
                "Users",
                "Premium",
                "Groups",
                "Uptime Bot",
                "Uptime Server",
            ]
        );

        assert_eq!(t.footer.as_deref(), Some("🍃 Realtime Monitoring"));
        let header_teks = t.header_teks.as_deref().unwrap_or_default();
        assert!(
            header_teks.ends_with("*STATUS*\n\n- 🎄 Dibawah ini adalah statistik bot kita"),
            "{header_teks}"
        );
    }

    #[test]
    fn tanpa_store_jumlahnya_nol_bukan_satu() {
        // `Ctx::default()` (test plugin langsung) belum lewat pipeline, jadi
        // jumlah store 0 — bukan `user_data.is_object() as usize` yang melapor 1.
        let balasan = ping2(Ctx::default());
        let t = tabel_dari(&balasan);
        for kunci in ["Users", "Premium", "Groups"] {
            assert_eq!(
                t.baris.iter().find(|b| b[0] == kunci).map(|b| b[1].as_str()),
                Some("0"),
                "{kunci}"
            );
        }
    }

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
