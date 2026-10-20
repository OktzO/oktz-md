//! `plugins/main/totalfitur.js` — distribusi jumlah plugin per kategori.
//!
//! `ponytail:` `sock.sendTable` (`src/lib/socket.js:1300`) mengirim rich message
//! `botForwardedMessage.richResponseMessage` (`src/lib/rich-messages.js:609`)
//! yang protonya tidak ada di `wacore-0.7.0`; header, tabel, dan footer
//! dirangkai sebagai teks dengan isi serta urutan yang sama. Ganti ke
//! `Reply::Interactive` begitu proto rich message tersedia.

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

fn teks() -> String {
    let (daftar, total, aktif) = rekap();

    let mut out = format!(
        "Total: {total} | Aktif: {aktif} | Kategori: {}\n\n\
         Distribusi Fitur\n\n\
         Kategori | Jumlah | Persen\n",
        daftar.len()
    );

    for r in &daftar {
        out.push_str(&format!(
            "{} {} | {} | {}%\n",
            ikon(&r.kategori),
            r.kategori.to_uppercase(),
            r.total,
            persen(r.total, total)
        ));
    }

    out.push_str(&format!("\nTotal {total} fitur tersedia"));
    out
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
    Reply::text(teks()).with_reaction("📊")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn keluaran_memuat_total_dan_kategori_main() {
        let (_, total, aktif) = rekap();
        let r = totalfitur(Ctx::default());
        let t = r.without_mutations().text_body().unwrap();

        assert!(t.contains(&format!(
            "Total: {total} | Aktif: {aktif} | Kategori: "
        )));
        assert!(t.contains("Distribusi Fitur"));
        assert!(t.contains("Kategori | Jumlah | Persen"));
        assert!(t.contains("🏠 MAIN"));
        assert!(t.ends_with(&format!("Total {total} fitur tersedia")));
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
