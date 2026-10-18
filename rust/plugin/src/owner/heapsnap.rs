//! `plugins/owner/heapsnap.js` — ambil heap snapshot dengan guard RSS.
//!
//! `ponytail:` `takeHeapSnapshot` (v8 `writeHeapSnapshot`) dan
//! `heapSnapshotEligibility` (yang membaca `process.memoryUsage()`) tidak
//! di-port: keduanya butuh V8/heap dan tidak ada di `Ctx`. Yang dipindah:
//! konstanta ceiling, `formatMB`, dan parsing label — semua murni.
//!
//! Catatan meta: alias JS memuat `"heapsnap"` — sama dengan `name`. Dibiarkan
//! apa adanya (lihat laporan batch), bukan dirapikan diam-diam.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `HEAP_SNAP_RSS_CEILING = 400 * 1024 * 1024` (`profiler.js:181`).
pub(crate) const RSS_CEILING: f64 = 400.0 * 1024.0 * 1024.0;
/// `HEAP_SNAP_PROJECTED_CEILING = 800 * 1024 * 1024` (`profiler.js:182`).
pub(crate) const PROJECTED_CEILING: f64 = 800.0 * 1024.0 * 1024.0;

/// `(bytes / 1024 / 1024).toFixed(1) + "MB"` — `toFixed` mengambil integer
/// terdekat dari `nilai * 10` dan mengikat ke atas saat sama saja, jadi
/// `1.99MB` -> `"2.0MB"` (bukan `"1.9MB"` seperti `floor`).
pub(crate) fn format_mb(bytes: f64) -> String {
    let tenths = (bytes / 1024.0 / 1024.0 * 10.0).round() / 10.0;
    format!("{tenths:.1}MB")
}

/// Status tetap menampilkan dua ceiling — keduanya konstanta, bukan hasil
/// pengukuran, jadi nilainya benar meski memoryUsage tidak terbaca.
pub(crate) fn status(prefix: &str) -> String {
    format!(
        "📊 *Status Heap Snapshot*\n\
         RSS: tidak terbaca / {} (ceiling)\n\
         Heap Total: tidak terbaca\n\
         Proyeksi puncak: tidak terbaca / {} (ceiling)\n\
         \n\
         Snapshot: ❌ Tidak bisa dinilai\n\
         > Alasan: `process.memoryUsage()` tidak ada di `Ctx`\n\
         \n\
         Gunakan *{prefix}heapsnap [label]* untuk mengambil snapshot.",
        format_mb(RSS_CEILING),
        format_mb(PROJECTED_CEILING)
    )
}

pub(crate) fn unavailable(label: &str) -> String {
    format!(
        "⚠️ *Heap Snapshot tidak di-port*\n\n\
         > `v8.writeHeapSnapshot` tidak ada di `Ctx`, jadi tidak ada berkas\n\
         > yang ditulis dan guard RSS tidak pernah dievaluasi.\n\
         > Label: `{label}`"
    )
}

#[plugin(
    name = "heapsnap",
    alias = ["heapsnapshot", "heapsnap", "snapheap"],
    category = "owner",
    description = "Ambil heap snapshot (dengan guard RSS — Owner Only)",
    usage = ".heapsnap [label] | .heapsnap status",
    example = ".heapsnap before-ocr",
    cooldown = 30,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn heapsnap(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    if sub == "status" || sub.is_empty() {
        return Reply::text(status(&ctx.prefix));
    }
    Reply::text(unavailable(&sub))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(args: &[&str]) -> String {
        (find("heapsnap").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "heapsnap".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        })
        .text_body()
        .expect("harus ada teks")
        .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("heapsnap").unwrap();
        assert_eq!(p.meta.name, "heapsnap");
        assert_eq!(
            p.meta.alias,
            ["heapsnapshot", "heapsnap", "snapheap"],
            "alias JS memuat nama sendiri; dibiarkan apa adanya"
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Ambil heap snapshot (dengan guard RSS — Owner Only)");
        assert_eq!(p.meta.usage, ".heapsnap [label] | .heapsnap status");
        assert_eq!(p.meta.example, ".heapsnap before-ocr");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_that_repeats_the_name_still_resolves_here() {
        assert_eq!(find("heapsnap").unwrap().meta.name, "heapsnap");
        assert_eq!(find("heapsnapshot").unwrap().meta.name, "heapsnap");
        assert_eq!(find("snapheap").unwrap().meta.name, "heapsnap");
    }

    #[test]
    fn format_mb_keeps_one_decimal_like_to_fixed() {
        assert_eq!(format_mb(0.0), "0.0MB");
        assert_eq!(format_mb(1024.0 * 1024.0), "1.0MB");
        assert_eq!(format_mb(RSS_CEILING), "400.0MB");
        assert_eq!(format_mb(PROJECTED_CEILING), "800.0MB");
        // `toFixed` membulatkan ke terdekat: 1,99 -> "2.0", 1,94 -> "1.9".
        assert_eq!(format_mb(1.99 * 1024.0 * 1024.0), "2.0MB");
        assert_eq!(format_mb(1.94 * 1024.0 * 1024.0), "1.9MB");
        // 1024 * 1024 * 0.05 -> 0,05MB -> "0.1" (0,05 * 10 = 0,5 -> atas).
        assert_eq!(format_mb(0.05 * 1024.0 * 1024.0), "0.1MB");
    }

    #[test]
    fn the_two_ceilings_are_the_ones_from_profiler_js() {
        assert_eq!(RSS_CEILING, 419_430_400.0);
        assert_eq!(PROJECTED_CEILING, 838_860_800.0);
    }

    #[test]
    fn status_shows_both_ceilings_and_says_why_live_numbers_are_missing() {
        let out = run(&["status"]);
        assert!(out.contains("400.0MB (ceiling)"), "{out}");
        assert!(out.contains("800.0MB (ceiling)"), "{out}");
        assert!(out.contains("> Alasan: `process.memoryUsage()` tidak ada di `Ctx`"), "{out}");
        assert!(out.contains("Gunakan *.heapsnap [label]*"), "{out}");
    }

    #[test]
    fn no_argument_takes_the_status_branch() {
        assert_eq!(run(&[]), run(&["status"]));
        assert_eq!(run(&["STATUS"]), run(&["status"]), "lowercase dulu");
    }

    #[test]
    fn any_other_word_becomes_the_snapshot_label() {
        assert!(run(&["before-ocr"]).contains("Label: `before-ocr`"), "label apa adanya");
        assert!(run(&["AFTER"]).contains("Label: `after`"), "cuma argumen yang di-lowercase");
    }

    #[test]
    fn nothing_is_written_to_the_store() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "heapsnap".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: vec!["before-ocr".into()],
            ..Default::default()
        };
        assert!((find("heapsnap").unwrap().handler)(ctx).mutations().is_empty());
    }
}
