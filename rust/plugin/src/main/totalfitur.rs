//! `plugins/main/totalfitur.js` — distribusi jumlah plugin per kategori.
//!
//! Balasan memakai tabel AI (`Reply::tabel`) — padanan `sock.sendTable` JS
//! (`totalfitur.js:62-72`): judul tabel, baris heading, baris isi per
//! kategori, teks sebelum tabel (`headerText`), dan teks setelah tabel
//! (`footer`), isi serta urutannya sama dengan JS.
//! `ponytail:` tabel versi 1 (`tableMetadata`), bukan versi 2 —
//! `unifiedResponse` base64 dan `botMetadata` versi JS v2 belum di-port.
//! `ponytail:` `Ctx` tidak dipakai di plugin ini; jumlah plugin per kategori
//! diambil dari registry plugin (`all_plugins()`), sama seperti
//! `getAllPlugins()` di JS.

use oktz_plugin_api::{Ctx, Reply, all_plugins, plugin};

/// `ICONS` (`totalfitur.js:20-29`) — urutan tidak memengaruhi hasil.
const IKON: [(&str, &str); 37] = [
    ("main", "🏠"),
    ("tools", "🔧"),
    ("downloader", "📥"),
    ("download", "📥"),
    ("sticker", "🎨"),
    ("ai", "🤖"),
    ("media", "📷"),
    ("game", "🎮"),
    ("rpg", "⚔️"),
    ("maker", "🖼️"),
    ("fun", "🎭"),
    ("group", "👥"),
    ("owner", "👑"),
    ("premium", "💎"),
    ("info", "📊"),
    ("search", "🔍"),
    ("canvas", "🎨"),
    ("anime", "🌸"),
    ("nsfw", "🔞"),
    ("utility", "🛠️"),
    ("economy", "💰"),
    ("stalker", "🔎"),
    ("random", "🎲"),
    ("religi", "🕌"),
    ("islamic", "☪️"),
    ("cek", "✅"),
    ("store", "🛒"),
    ("panel", "🖥️"),
    ("convert", "🔄"),
    ("primbon", "🔮"),
    ("tts", "🗣️"),
    ("otp", "🔑"),
    ("vps", "☁️"),
    ("pushkontak", "📱"),
    ("jpm", "🎰"),
    ("ephoto", "📸"),
    ("other", "📦"),
];

struct Rekap {
    kategori: String,
    total: usize,
    aktif: usize,
}

/// Hitung total dan jumlah aktif per kategori, lalu urutkan total menurun.
/// `sort_by` stabil, jadi kategori dengan jumlah sama tetap mengikuti urutan
/// registry seperti `Object.entries(...).sort(...)` di JS.
fn rekap() -> (Vec<Rekap>, usize, usize) {
    let mut daftar: Vec<Rekap> = Vec::new();
    let (mut total, mut aktif) = (0, 0);

    for p in all_plugins() {
        let meta = &p.meta;
        // Plugin tanpa config dilewati di JS; setiap entri registry punya meta.
        let kategori = if meta.category.is_empty() {
            "other"
        } else {
            meta.category.as_str()
        };

        let i = if let Some(i) = daftar.iter().position(|r| r.kategori.as_str() == kategori) {
            i
        } else {
            daftar.push(Rekap {
                kategori: kategori.to_string(),
                total: 0,
                aktif: 0,
            });
            daftar.len() - 1
        };

        daftar[i].total += 1;
        total += 1;
        if meta.is_enabled {
            daftar[i].aktif += 1;
            aktif += 1;
        }
    }

    daftar.sort_by_key(|r| std::cmp::Reverse(r.total));
    (daftar, total, aktif)
}

fn ikon(kategori: &str) -> &'static str {
    IKON
        .iter()
        .find(|(nama, _)| *nama == kategori)
        .map_or("📦", |(_, emoji)| *emoji)
}

/// `((bagian / total) * 100).toFixed(1)` — dibulatkan setengah ke atas seperti
/// `toFixed`, bukan setengah ke genap seperti format float Rust.
fn persen(bagian: usize, total: usize) -> String {
    if total == 0 {
        return "NaN".to_string();
    }
    let sepuluh = (bagian * 1000 + total / 2) / total;
    format!("{}.{}", sepuluh / 10, sepuluh % 10)
}

/// Susun tabel AI — padanan `sock.sendTable` JS (`totalfitur.js:62-72`).
fn reply() -> Reply {
    let (daftar, total, aktif) = rekap();

    let mut t = Reply::tabel("Distribusi Fitur")
        .with_table_header(["Kategori", "Jumlah", "Persen"])
        .with_table_header_text(format!(
            "Total: {total} | Aktif: {aktif} | Kategori: {}",
            daftar.len()
        ));

    for r in &daftar {
        t = t.with_table_row([
            format!("{} {}", ikon(&r.kategori), r.kategori.to_uppercase()),
            r.total.to_string(),
            format!("{}%", persen(r.total, total)),
        ]);
    }

    t.with_table_footer(format!("Total {total} fitur tersedia"))
}

#[plugin(
    name = "totalfitur",
    alias = ["totalfeature", "totalcmd", "countplugin", "distribusi"],
    category = "main",
    description = "Lihat total fitur/command bot",
    usage = ".totalfitur",
    example = ".totalfitur",
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
pub fn totalfitur(_ctx: Ctx) -> Reply {
    reply().with_reaction("📊")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ambil tabel di balik reaksi/mutasi — `with_reaction` membungkus balas
    /// jadi `Mutate` + `Reaction`, jadi keduanya harus ditembus dulu.
    fn tabel_dari(reply: &Reply) -> &oktz_plugin_api::Tabel {
        match reply.without_mutations().tanpa_reaksi() {
            Reply::Tabel { tabel } => tabel,
            other => panic!("harus Tabel, dapat {other:?}"),
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("totalfitur").unwrap();
        assert_eq!(p.meta.name, "totalfitur");
        assert_eq!(
            p.meta.alias,
            ["totalfeature", "totalcmd", "countplugin", "distribusi"]
        );
        assert_eq!(p.meta.category, "main");
        assert_eq!(p.meta.description, "Lihat total fitur/command bot");
        assert_eq!(p.meta.usage, ".totalfitur");
        assert_eq!(p.meta.example, ".totalfitur");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
    }

    #[test]
    fn hitungan_mengikuti_registry() {
        let (daftar, total, aktif) = rekap();
        assert!(total > 0);
        assert_eq!(total, all_plugins().len());
        assert!(aktif <= total);
        assert_eq!(daftar.iter().map(|r| r.total).sum::<usize>(), total);
        assert_eq!(daftar.iter().map(|r| r.aktif).sum::<usize>(), aktif);
        assert!(daftar.iter().any(|r| r.kategori == "main"));
        assert!(daftar.windows(2).all(|w| w[0].total >= w[1].total));
    }

    #[test]
    fn keluaran_membentuk_tabel_dengan_isi_dan_urutan_js() {
        let (daftar, total, aktif) = rekap();
        let r = totalfitur(Ctx::default());
        assert!(
            matches!(r.without_mutations().tanpa_reaksi(), Reply::Tabel { .. }),
            "harus Reply::Tabel"
        );
        let t = tabel_dari(&r);

        assert_eq!(t.judul, "Distribusi Fitur");
        assert_eq!(t.header, vec!["Kategori", "Jumlah", "Persen"]);

        let header_teks = format!("Total: {total} | Aktif: {aktif} | Kategori: {}", daftar.len());
        assert_eq!(t.header_teks.as_deref(), Some(header_teks.as_str()));
        let footer = format!("Total {total} fitur tersedia");
        assert_eq!(t.footer.as_deref(), Some(footer.as_str()));

        // Satu baris per kategori, isi: ikon + kategori.upper, jumlah, persen.
        assert_eq!(t.baris.len(), daftar.len());
        assert!(t.baris.iter().any(|b| b[0] == "🏠 MAIN"));
        assert!(
            t.baris.iter().all(|b| b.len() == 3 && b[2].ends_with('%')),
            "tiap baris punya 3 sel dan persen di akhir: {:?}",
            t.baris
        );
        // Urutan jumlah menurun (persis seperti `sort` JS).
        let jumlah: Vec<u32> = t.baris.iter().map(|b| b[1].parse().unwrap()).collect();
        assert!(jumlah.windows(2).all(|w| w[0] >= w[1]), "{jumlah:?}");
    }

    #[test]
    fn reaksi_tetap_ada() {
        let r = totalfitur(Ctx::default());
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "📊"
        ));
    }

    #[test]
    fn persen_satu_desimal_seperti_tofixed() {
        assert_eq!(persen(1, 16), "6.3");
        assert_eq!(persen(1, 3), "33.3");
        assert_eq!(persen(16, 16), "100.0");
        assert_eq!(persen(0, 0), "NaN");
    }
}
