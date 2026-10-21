//! `plugins/main/stats.js` — statistik bot.
//!
//! Balasan memakai tabel AI (`Reply::tabel`): judul tabel, baris heading
//! `Key | Value`, baris isi, dan footer — isi serta urutannya sama dengan JS.
//! `ponytail:` tabel versi 1 (`tableMetadata`), bukan versi 2 —
//! `unifiedResponse` base64 dan `botMetadata` versi JS v2 belum di-port, jadi
//! tabel digabung manual dari pemisah `;;`.
//! `ponytail:` `Ctx` hanya membawa data user/grup pengirim, bukan seluruh
//! store; Users/Premium/Groups dihitung dari data itu, bukan total store.
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

/// Susun tabel AI — padanan `sock.sendTableV2` JS (`stats.js:88-91`).
///
/// JS v2 mengirim `unifiedResponse` base64; tabel v1 yang dipakai di sini
/// butuh isi yang sama, jadi tiap pasangan `Key | Value` dari string
/// ber-pemisah `;;` (`stats.js:79-86`) dipecah jadi satu baris tabel.
fn reply(ctx: &Ctx) -> Reply {
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

    let mut t = Reply::tabel("📊 Berikut ini adalah statistik dari bot kami")
        .with_table_header(["Key", "Value"])
        // "📊 Bot Statistics" ada di `table[0]` JS, yang oleh
        // `toTableMetadataV2` (`rich-messages.js:948`) dibaca sebagai judul
        // tabel (`tableMetadata.title`). V2 tidak pernah mengirim judul itu ke
        // wire — hanya `unified_rows` plus `options.title`/`options.footer` —
        // dan di tabel v1 slot `judul` sudah dipakai `options.title`. Jadi
        // teks paling mirip yang tersisa adalah teks sebelum tabel
        // (`header_teks`): baris pertama tabel akan jadi sel `Key` tanpa
        // pasangan `Value`, jadi grid dua kolom rusak.
        .with_table_header_text("📊 Bot Statistics");

    t = t.with_table_row(["Bot".to_string(), bot.to_string()]);
    t = t.with_table_row(["Version".to_string(), format!("v{versi}")]);
    t = t.with_table_row([
        "Uptime".to_string(),
        format_uptime(ctx.uptime_secs.saturating_mul(1000)),
    ]);
    t = t.with_table_row(["Users".to_string(), users.to_string()]);
    t = t.with_table_row(["Premium".to_string(), premium.to_string()]);
    t = t.with_table_row(["Groups".to_string(), groups.to_string()]);
    t = t.with_table_row(["Platform".to_string(), platform()]);
    t = t.with_table_row(["Node".to_string(), "N/A".to_string()]);
    t = t.with_table_row([
        "CPU Load".to_string(),
        format!("{}%", fixed2(baca_load())),
    ]);
    t = t.with_table_row([
        "RAM".to_string(),
        format!(
            "{} / {}",
            format_bytes(total_mem.saturating_sub(bebas_mem)),
            format_bytes(total_mem)
        ),
    ]);
    t = t.with_table_row([
        "Heap".to_string(),
        format!(
            "{} / {}",
            format_bytes(heap_dipakai),
            format_bytes(heap_total)
        ),
    ]);
    t = t.with_table_row(["Updated".to_string(), jam_jakarta()]);

    // JS `footer: botConfig?.bot?.name` — string kosong berarti tanpa footer.
    if nama_bot.is_empty() {
        t
    } else {
        t.with_table_footer(nama_bot)
    }
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
    reply(&ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ambil tabel dari balasan — `stats` tidak pakai reaksi, jadi cukup
    /// menembus `Mutate` kalau ada.
    fn tabel_dari(reply: &Reply) -> &oktz_plugin_api::Tabel {
        match reply.without_mutations().tanpa_reaksi() {
            Reply::Tabel { tabel } => tabel,
            other => panic!("harus Tabel, dapat {other:?}"),
        }
    }

    #[test]
    fn balasan_membentuk_tabel_dengan_isi_dan_urutan_js() {
        let r = stats(Ctx::default());
        assert!(
            matches!(r.without_mutations().tanpa_reaksi(), Reply::Tabel { .. }),
            "harus Reply::Tabel"
        );
        let t = tabel_dari(&r);

        assert_eq!(t.judul, "📊 Berikut ini adalah statistik dari bot kami");
        assert_eq!(t.header, vec!["Key", "Value"]);
        assert_eq!(t.header_teks.as_deref(), Some("📊 Bot Statistics"));

        let kunci: Vec<&str> = t.baris.iter().map(|b| b[0].as_str()).collect();
        assert_eq!(
            kunci,
            vec![
                "Bot",
                "Version",
                "Uptime",
                "Users",
                "Premium",
                "Groups",
                "Platform",
                "Node",
                "CPU Load",
                "RAM",
                "Heap",
                "Updated",
            ]
        );
        assert_eq!(t.baris.len(), 12);

        // Config bawaan kosong, jadi nama bot jatuh ke "Bot" dan versi ke
        // "1.0.0" (`stats.js:59-60`).
        assert_eq!(t.baris[0], vec!["Bot", "Bot"]);
        assert_eq!(t.baris[1], vec!["Version", "v1.0.0"]);
        assert_eq!(t.baris[8][0], "CPU Load");
        assert!(t.baris[8][1].ends_with('%'));
        // Waktu Asia/Jakarta `HH.MM.SS`.
        assert_eq!(t.baris[11][1].len(), 8, "jam-menit-detik: {}", t.baris[11][1]);
    }

    #[test]
    fn footer_ikut_nama_bot_dan_hilang_kala_kosong() {
        // Nama bot kosong -> tanpa footer (JS `if (footer)`).
        let r = stats(Ctx::default());
        assert_eq!(tabel_dari(&r).footer, None);

        // Nama bot terisi -> footer = nama bot.
        let mut cfg = oktz_plugin_api::config::Runtime::default();
        cfg.bot.name = "Foto-AI".to_string();
        let ctx = Ctx {
            config: std::sync::Arc::new(cfg),
            ..Default::default()
        };
        let r = stats(ctx);
        let t = tabel_dari(&r);
        assert_eq!(t.footer.as_deref(), Some("Foto-AI"));
        assert_eq!(t.baris[0], vec!["Bot", "Foto-AI"]);
    }

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
