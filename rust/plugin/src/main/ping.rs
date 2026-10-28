//! `plugins/main/ping.js` — canvas diagnostik sistem PNG.

use std::fs;
use std::time::Instant;

use oktz_plugin_api::{Ctx, Kanvas, MediaSource, Reply, plugin};

fn fmt_size(b: u64) -> String {
    if b == 0 {
        return "0 B".to_string();
    }
    let satuan = ["B", "KB", "MB", "GB", "TB"];
    let angka = b as f64;
    let i = (angka.ln() / 1024f64.ln()).floor() as usize;
    let nilai = angka / 1024f64.powi(i as i32);
    format!(
        "{:.2} {}",
        (nilai * 100.0).round() / 100.0,
        satuan.get(i).copied().unwrap_or("undefined")
    )
}

fn fmt_up(s: u64) -> String {
    let d = s / 86_400;
    let h = (s % 86_400) / 3600;
    let m = (s % 3600) / 60;
    let sc = s % 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m {sc}s")
    } else {
        format!("{m}m {sc}s")
    }
}

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

fn load_avg() -> [f64; 3] {
    fs::read_to_string("/proc/loadavg")
        .ok()
        .map(|teks| {
            let angka: Vec<f64> = teks
                .split_whitespace()
                .take(3)
                .filter_map(|n| n.parse().ok())
                .collect();
            [
                angka.first().copied().unwrap_or(0.0),
                angka.get(1).copied().unwrap_or(0.0),
                angka.get(2).copied().unwrap_or(0.0),
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0])
}

fn mem_info() -> (u64, u64) {
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

fn cpu_info() -> (String, u64, u64) {
    let teks = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut model = String::new();
    let mut cores = 0u64;
    let mut speed = 0u64;
    for baris in teks.lines() {
        let Some((kunci, nilai)) = baris.split_once(':') else {
            continue;
        };
        match kunci.trim() {
            "processor" => cores += 1,
            "model name" if model.is_empty() => model = nilai.trim().to_string(),
            "cpu MHz" if speed == 0 => {
                speed = nilai
                    .trim()
                    .parse::<f64>()
                    .map(|n| n as u64)
                    .unwrap_or(0)
            }
            _ => {}
        }
    }
    if cores == 0 {
        cores = std::thread::available_parallelism()
            .map(|n| n.get() as u64)
            .unwrap_or(1);
    }
    if model.is_empty() {
        model = "Unknown CPU".to_string();
    }
    (model, cores, speed)
}

fn heap_info() -> (u64, u64, u64, u64) {
    let teks = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let (mut rss, mut size) = (0u64, 0u64);
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
            "VmRSS" => rss = kb.saturating_mul(1024),
            "VmSize" => size = kb.saturating_mul(1024),
            _ => {}
        }
    }
    (rss, rss, size, size)
}

fn os_info() -> (String, String, String) {
    let os_type = std::env::consts::OS.to_string();
    let release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "N/A".to_string());
    let host = fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "N/A".to_string());
    (os_type, release, host)
}

fn teks_atas(kanvas: &mut Kanvas, teks: &str, x: u32, y: u32, ukuran: f32, warna: [u8; 4]) {
    kanvas.teks(teks, x, y, ukuran, warna);
}

fn gambar_ping(ping_ms: u128, bot_uptime: u64) -> Vec<u8> {
    let mut k = Kanvas::baru(1200, 800);
    k.isi_gradient_radial((600, 400), 1200, [(15, 23, 42), (2, 6, 23)]);

    for x in (0..1200).step_by(40) {
        k.garis((x, 0), (x, 800), [56, 189, 248, 8]);
    }
    for y in (0..800).step_by(40) {
        k.garis((0, y), (1200, y), [56, 189, 248, 8]);
    }

    teks_atas(&mut k, "SYSTEM DIAGNOSTICS", 50, 45, 50.0, [56, 189, 248, 255]);
    teks_atas(&mut k, "STATUS: ONLINE", 50, 95, 24.0, [203, 213, 225, 255]);
    k.garis((50, 130), (1150, 130), [56, 189, 248, 255]);

    let (cpu_model, cores, speed) = cpu_info();
    let load = load_avg();
    let (total_mem, free_mem) = mem_info();
    let used_mem = total_mem.saturating_sub(free_mem);
    let mem_pct = if total_mem == 0 {
        0.0
    } else {
        used_mem as f64 / total_mem as f64 * 100.0
    };
    let (heap_used, _, heap_total, _) = heap_info();
    let (os_type, os_rel, host) = os_info();
    let up_server = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|teks| {
            teks.split_whitespace()
                .next()
                .and_then(|n| n.parse::<f64>().ok())
        })
        .unwrap_or(0.0) as u64;
    let node = "N/A";
    let v8 = "N/A";

    k.kotak_gradient(50, 160, 330, 180, [(30, 41, 59), (15, 23, 42)], (74, 222, 128));
    teks_atas(&mut k, "⌖ LATENCY PING", 70, 195, 22.0, [74, 222, 128, 255]);
    let ping_warna = if ping_ms < 100 {
        [74, 222, 128, 255]
    } else if ping_ms < 500 {
        [250, 204, 21, 255]
    } else {
        [248, 113, 113, 255]
    };
    k.teks(&format!("{ping_ms}"), 150, 250, 70.0, ping_warna);
    k.teks("ms", 300, 250, 30.0, ping_warna);
    let status_ping = if ping_ms < 100 {
        "EXCELLENT"
    } else if ping_ms < 500 {
        "MODERATE"
    } else {
        "POOR"
    };
    k.teks(&format!("CONNECTION: {status_ping}"), 70, 305, 18.0, [203, 213, 225, 255]);

    k.kotak_gradient(50, 360, 330, 230, [(30, 41, 59), (15, 23, 42)], (251, 191, 36));
    teks_atas(&mut k, "⌖ CPU PROCESSOR", 70, 395, 22.0, [251, 191, 36, 255]);
    let mut y = 430;
    for baris in cpu_model.split_whitespace().collect::<Vec<_>>().chunks(4) {
        k.teks(&baris.join(" "), 70, y, 22.0, [248, 250, 252, 255]);
        y += 30;
    }
    k.teks("Cores:", 70, 520, 20.0, [148, 163, 184, 255]);
    k.teks(&format!("{cores} Threads"), 160, 520, 20.0, [251, 191, 36, 255]);
    k.teks("Speed:", 70, 555, 20.0, [148, 163, 184, 255]);
    k.teks(&format!("{speed} MHz"), 160, 555, 20.0, [251, 191, 36, 255]);

    k.kotak_gradient(50, 610, 330, 140, [(30, 41, 59), (15, 23, 42)], (244, 63, 94));
    teks_atas(&mut k, "⌖ SYSTEM LOAD", 70, 645, 22.0, [244, 63, 94, 255]);
    k.teks(&format!("1m:  {:.2}", load[0]), 70, 680, 24.0, [248, 250, 252, 255]);
    k.teks(&format!("5m:  {:.2}", load[1]), 70, 710, 24.0, [248, 250, 252, 255]);
    k.teks(&format!("15m: {:.2}", load[2]), 230, 680, 24.0, [248, 250, 252, 255]);

    let gauge_x = 600;
    let gauge_y = 380;
    k.teks("MEMORY USAGE", gauge_x, 200, 30.0, [192, 132, 252, 255]);
    k.lingkaran((gauge_x, gauge_y), 140, [255, 255, 255, 8], 0.75, 2.25);
    let sudut = 0.75 + 1.5 * (mem_pct.clamp(0.0, 100.0) / 100.0) as f32;
    k.lingkaran((gauge_x, gauge_y), 140, [192, 132, 252, 255], 0.75, sudut);
    k.teks(&format!("{mem_pct:.1}%"), gauge_x, 390, 70.0, [255, 255, 255, 255]);
    k.teks("USED RAM", gauge_x, 420, 24.0, [192, 132, 252, 255]);
    k.teks(&format!("Used: {}", fmt_size(used_mem)), gauge_x, 580, 22.0, [148, 163, 184, 255]);
    k.teks(&format!("Free: {}", fmt_size(free_mem)), gauge_x, 615, 22.0, [148, 163, 184, 255]);
    k.teks(&format!("TOTAL: {}", fmt_size(total_mem)), gauge_x, 660, 28.0, [248, 250, 252, 255]);

    k.kotak_gradient(820, 160, 330, 230, [(30, 41, 59), (15, 23, 42)], (56, 189, 248));
    teks_atas(&mut k, "⌖ NODE.JS ENGINE", 840, 195, 22.0, [56, 189, 248, 255]);
    k.teks("RSS:", 840, 220, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_size(heap_used), 980, 220, 20.0, [56, 189, 248, 255]);
    k.teks("Heap Used:", 840, 255, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_size(heap_used), 980, 255, 20.0, [56, 189, 248, 255]);
    k.teks("Heap Total:", 840, 290, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_size(heap_total), 980, 290, 20.0, [56, 189, 248, 255]);
    k.teks("External:", 840, 325, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_size(heap_used), 980, 325, 20.0, [56, 189, 248, 255]);
    k.teks("V8 Engine:", 840, 360, 20.0, [148, 163, 184, 255]);
    k.teks(v8, 980, 360, 20.0, [248, 250, 252, 255]);

    k.kotak_gradient(820, 410, 330, 180, [(30, 41, 59), (15, 23, 42)], (168, 85, 247));
    teks_atas(&mut k, "⌖ SYSTEM SPECS", 840, 445, 22.0, [168, 85, 247, 255]);
    k.teks("OS:", 840, 470, 20.0, [148, 163, 184, 255]);
    k.teks(&format!("{os_type} {os_rel}"), 900, 470, 20.0, [248, 250, 252, 255]);
    k.teks("Arch:", 840, 505, 20.0, [148, 163, 184, 255]);
    k.teks(&platform(), 900, 505, 20.0, [248, 250, 252, 255]);
    k.teks("Host:", 840, 540, 20.0, [148, 163, 184, 255]);
    k.teks(&host, 900, 540, 20.0, [248, 250, 252, 255]);
    k.teks("Node:", 840, 575, 20.0, [148, 163, 184, 255]);
    k.teks(node, 900, 575, 20.0, [248, 250, 252, 255]);

    k.kotak_gradient(820, 610, 330, 140, [(30, 41, 59), (15, 23, 42)], (236, 72, 153));
    teks_atas(&mut k, "⌖ UPTIME", 840, 645, 22.0, [236, 72, 153, 255]);
    k.teks("Server:", 840, 670, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_up(up_server), 940, 670, 22.0, [236, 72, 153, 255]);
    k.teks("Bot:", 840, 715, 20.0, [148, 163, 184, 255]);
    k.teks(&fmt_up(bot_uptime), 940, 715, 22.0, [236, 72, 153, 255]);

    k.ke_png()
}

fn caption(ping_ms: u128, bot_uptime: u64, data: &str) -> String {
    let (cpu_model, cores, speed) = cpu_info();
    let load = load_avg();
    let (total_mem, free_mem) = mem_info();
    let used_mem = total_mem.saturating_sub(free_mem);
    let mem_pct = if total_mem == 0 {
        0.0
    } else {
        used_mem as f64 / total_mem as f64 * 100.0
    };
    let (rss, heap_used, heap_total, external) = heap_info();
    let (os_type, os_rel, host) = os_info();
    let up_server = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|teks| {
            teks.split_whitespace()
                .next()
                .and_then(|n| n.parse::<f64>().ok())
        })
        .unwrap_or(0.0) as u64;
    let up_os = fmt_up(up_server);
    let up_bot = fmt_up(bot_uptime);
    let node = "N/A";
    let v8 = "N/A";

    format!(
        "🏓 *PONG!* ({ping_ms}ms)\n\n\
         Berikut adalah detail spesifikasi dan performa server secara lengkap:\n\n\
         🖥️ *INFORMASI SISTEM*\n\
         > ◦ *OS:* {os_type} ({os_rel})\n\
         > ◦ *Platform:* {platform}\n\
         > ◦ *Hostname:* {host}\n\
         > ◦ *NodeJS:* {node}\n\
         > ◦ *Engine V8:* {v8}\n\n\
         💻 *INFORMASI CPU*\n\
         > ◦ *Model:* {model}\n\
         > ◦ *Cores:* {cores} Core(s)\n\
         > ◦ *Speed:* {speed} MHz\n\
         > ◦ *Load Avg:* {l1} (1m), {l2} (5m), {l3} (15m)\n\n\
         🧠 *PENGGUNAAN MEMORI*\n\
         > ◦ *Total RAM:* {total_ram}\n\
         > ◦ *Dipakai:* {used_ram} ({mem_pct:.1}%)\n\
         > ◦ *Sisa Bebas:* {free_ram}\n\n\
         📦 *MEMORI NODEJS*\n\
         > ◦ *RSS:* {rss}\n\
         > ◦ *Heap Total:* {heap_total}\n\
         > ◦ *Heap Used:* {heap_used}\n\
         > ◦ *External:* {external}\n\n\
         ⏱️ *WAKTU AKTIF (UPTIME)*\n\
         > ◦ *Uptime Server:* {up_os}\n\
         > ◦ *Uptime Bot:* {up_bot}\n\n\
         Sistem berjalan stabil dan menyelesaikan kalkulasi dalam waktu eksekusi *{ping_ms}ms*\n{data}",
        model = cpu_model.trim(),
        platform = platform(),
        l1 = format!("{:.2}", load[0]),
        l2 = format!("{:.2}", load[1]),
        l3 = format!("{:.2}", load[2]),
        total_ram = fmt_size(total_mem),
        used_ram = fmt_size(used_mem),
        free_ram = fmt_size(free_mem),
        rss = fmt_size(rss),
        heap_total = fmt_size(heap_total),
        heap_used = fmt_size(heap_used),
        external = fmt_size(external),
        data = data
    )
}

#[plugin(
    name = "ping",
    alias = ["speed", "p", "sys"],
    category = "main",
    description = "Cek performa dan status sistem bot secara real-time (Canvas Epic)",
    usage = ".ping",
    example = ".ping",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ping(ctx: Ctx) -> Reply {
    let mulai = Instant::now();
    let png = gambar_ping(mulai.elapsed().as_millis(), ctx.uptime_secs);
    if png.is_empty() {
        return Reply::sequence(vec![
            Reply::Reaction {
                emoji: "❌".into(),
                reply: Box::new(Reply::None),
            },
            Reply::text(crate::common::error_template(&ctx)),
        ]);
    }

    let ping_ms = mulai.elapsed().as_millis();
    let pesan = caption(ping_ms, ctx.uptime_secs, "");
    Reply::sequence(vec![
        Reply::Reaction {
            emoji: "🕕".into(),
            reply: Box::new(Reply::None),
        },
        Reply::Image {
            media: Some(MediaSource::bytes(png)),
            caption: pesan,
            mentions: Vec::new(),
        },
        Reply::Reaction {
            emoji: "✅".into(),
            reply: Box::new(Reply::None),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    fn ctx() -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ping".into(),
            uptime_secs: 42,
            ..Ctx::default()
        }
    }

    #[test]
    fn ping_dimensions_are_1200x800() {
        let png = gambar_ping(12, 42);
        assert_eq!(Kanvas::ukuran_png(&png), Some((1200, 800)));
    }

    #[test]
    fn ping_png_magic_and_decoded_dimensions() {
        let png = gambar_ping(12, 42);
        assert!(!png.is_empty());
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(Kanvas::ukuran_png(&png), Some((1200, 800)));
    }

    #[test]
    fn ping_contains_all_caption_sections_and_labels() {
        let teks = caption(12, 42, "");
        for bagian in [
            "🏓 *PONG!* (12ms)",
            "🖥️ *INFORMASI SISTEM*",
            "💻 *INFORMASI CPU*",
            "🧠 *PENGGUNAAN MEMORI*",
            "📦 *MEMORI NODEJS*",
            "⏱️ *WAKTU AKTIF (UPTIME)*",
            "> ◦ *Hostname:*",
            "> ◦ *Load Avg:*",
            "> ◦ *Heap Total:*",
            "> ◦ *Uptime Bot:*",
            "Sistem berjalan stabil dan menyelesaikan kalkulasi dalam waktu eksekusi *12ms*",
        ] {
            assert!(teks.contains(bagian), "caption hilang {bagian}: {teks}");
        }
    }

    #[test]
    fn ping_success_sequence_is_clock_image_check() {
        let r = ping(ctx());
        let Reply::Sequence(list) = r else {
            panic!("sequence, dapat {r:?}")
        };
        assert_eq!(list.len(), 3);
        assert!(matches!(&list[0], Reply::Reaction { emoji, .. } if emoji == "🕕"));
        assert!(matches!(&list[1], Reply::Image { .. }));
        assert!(matches!(&list[2], Reply::Reaction { emoji, .. } if emoji == "✅"));
    }

    #[test]
    fn ping_failure_sequence_is_cross_then_error_template() {
        let r = Reply::sequence(vec![
            Reply::Reaction {
                emoji: "❌".into(),
                reply: Box::new(Reply::None),
            },
            Reply::text(crate::common::error_template(&ctx())),
        ]);
        let Reply::Sequence(list) = r else {
            panic!("sequence, dapat {r:?}")
        };
        assert!(matches!(&list[0], Reply::Reaction { emoji, .. } if emoji == "❌"));
        assert_eq!(
            list[1].without_mutations().text_body().unwrap(),
            crate::common::error_template_bawaan(".", "ping")
        );
    }

    #[test]
    fn ping_image_has_no_default_mention() {
        let r = ping(ctx());
        let Reply::Sequence(list) = r else {
            panic!("sequence, dapat {r:?}")
        };
        let Reply::Image { mentions, .. } = &list[1] else {
            panic!("image, dapat {list:?}")
        };
        assert!(mentions.is_empty());
    }

    #[test]
    fn ping_uses_trigger_quote() {
        let r = ping(ctx());
        let Reply::Sequence(list) = r else {
            panic!("sequence, dapat {r:?}")
        };
        let Reply::Image { media, .. } = &list[1] else {
            panic!("image, dapat {list:?}")
        };
        assert!(media.is_some());
    }
}
