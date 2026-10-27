//! `plugins/main/stats.js` — statistik bot.
//!
//! Balasan memakai tabel V2 (`Reply::tabel_v2`) — padanan `sock.sendTableV2`
//! JS (`stats.js:88-91`): baris heading, baris isi, dan footer, isi serta
//! urutannya sama dengan JS.
//!
//! Judul input `📊 Bot Statistics` (`table[0]`) sengaja TIDAK dikirim ke wire:
//! `generateTableContentV2` (`rich-messages.js:978-993`) hanya memakainya
//! sebagai `tableMetadata.title` dan menulis section heading dari
//! `options.headerText || options.title`. Tidak ada `headerText` di `.stats`,
//! jadi yang mendarat di wire adalah `title` (`📊 Berikut ini adalah statistik
//! dari bot kami`) dan `table[0]` hilang — persis seperti produksi.
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
/// `ctx.store_users`/`store_groups` dirangkum pipeline dari snapshot store yang
/// sama dengan yang dipakai gate permission.
///
/// Quirk produksi yang disalin apa adanya: `.stats` memfilter `u.premium`
/// (`stats.js:56`) padahal skema store memakai `isPremium`, jadi hasilnya
/// SELALU `0` — beda dengan `.ping2` yang memang membaca `isPremium`.
fn hitung_db(ctx: &Ctx) -> (usize, usize, usize) {
    (ctx.store_users, 0, ctx.store_groups)
}

/// `new Date().toLocaleTimeString("id-ID", { timeZone: "Asia/Jakarta" })`.
fn jam_jakarta() -> String {
    let (_, _, _, jam, menit, detik) = crate::common::jakarta_parts(crate::common::now_ms());
    format!("{jam:02}.{menit:02}.{detik:02}")
}

/// Susun tabel V2 — padanan `sock.sendTableV2` JS (`stats.js:88-91`).
///
/// `toTableMetadataV2` (`rich-messages.js:944-977`) membaca `table[0]` sebagai
/// `title`, tapi `generateTableContentV2` hanya mengirim `unified_rows` plus
/// section heading `headerText || title` dan section `footer`. Karena itu
/// judul di sini (`table[0]` JS) tidak pernah mendarat di wire, dan heading
/// yang terkirim adalah `options.title`.
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

    let mut t = Reply::tabel_v2("📊 Bot Statistics")
        .with_table_header(["Key", "Value"])
        // `options.title` JS — satu-satunya teks yang mendarat di wire sebagai
        // section heading (`rich-messages.js:982-983`: `headerText || title`).
        .with_table_header_text("📊 Berikut ini adalah statistik dari bot kami");

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

    /// Ambil tabel V2 dari balasan — `stats` tidak pakai reaksi, jadi cukup
    /// menembus `Mutate` kalau ada.
    fn tabel_dari(reply: &Reply) -> &oktz_plugin_api::RichV2 {
        match reply.without_mutations().tanpa_reaksi() {
            Reply::TabelV2(v2) => v2,
            other => panic!("harus TabelV2, dapat {other:?}"),
        }
    }

    #[test]
    fn stats_uses_table_v2_exact_heading_footer() {
        // `sock.sendTableV2(m.chat, table, m, { title, footer })`
        // (`stats.js:88-91`) — judul input `📊 Bot Statistics` (`table[0]`)
        // hanya jadi `tableMetadata.title` dan TIDAK dikirim ke wire; yang
        // mendarat sebagai section heading adalah `title`.
        let r = stats(Ctx::default());
        assert!(
            matches!(r.without_mutations().tanpa_reaksi(), Reply::TabelV2 { .. }),
            "harus Reply::TabelV2"
        );
        let t = tabel_dari(&r);

        assert_eq!(t.judul, "📊 Bot Statistics");
        assert_eq!(
            t.heading.as_deref(),
            Some("📊 Berikut ini adalah statistik dari bot kami")
        );
        // Config bawaan kosong, jadi footer hilang (`if (footer)`).
        assert_eq!(t.footer, None);

        assert_eq!(t.header, vec!["Key", "Value"]);
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
    fn jumlah_users_premium_groups_dari_seluruh_store_bukan_pengirim() {
        // `Object.keys(db.data.users/groups)` — SELURUH store. Data pengirim
        // saja tidak boleh dihitung sebagai jumlah user.
        //
        // Quirk `.stats`: `Object.values(users).filter((u) => u.premium)`
        // membaca field `premium` yang tidak ada di store (`isPremium` yang
        // ada), jadi produksi selalu menulis `0` — diuji apa adanya.
        let ctx = Ctx {
            user_data: serde_json::json!({"isPremium": true}),
            group_data: serde_json::json!({"name": "Grup"}),
            store_users: 12,
            store_premium: 4,
            store_groups: 7,
            ..Default::default()
        };
        let balasan = stats(ctx);
        let t = tabel_dari(&balasan);
        let baris = |kunci: &str| {
            t.baris
                .iter()
                .find(|b| b[0] == kunci)
                .unwrap_or_else(|| panic!("baris {kunci} hilang"))
                .clone()
        };
        assert_eq!(baris("Users"), vec!["Users", "12"]);
        assert_eq!(baris("Premium"), vec!["Premium", "0"]);
        assert_eq!(baris("Groups"), vec!["Groups", "7"]);
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
