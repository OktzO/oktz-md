use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// Tiga undian terpisah — `ceksisaumur.js:22-24`. Semuanya 0..=n-1; hanya
/// tahun yang diberi offset.
pub(crate) fn sisaumur_tahun() -> u32 {
    roll(80) + 20
}

pub(crate) fn sisaumur_bulan() -> u32 {
    roll(12)
}

pub(crate) fn sisaumur_hari() -> u32 {
    roll(30)
}

/// SATU-SATUNYA plugin di batch ini yang memakai `>` bukan `>=`
/// (`ceksisaumur.js:27-34`).
///
/// Konsekuensi: `tahun == 80` -> tier kedua, `tahun == 60` -> tier ketiga,
/// `tahun == 40` -> tier keempat. Kalau dibikin `>=`, tiga nilai itu lompat satu
/// tingkat — dan bukan probabilitas kecil (masing-masing 1 dari 80 nilai).
pub(crate) fn sisaumur_desc(tahun: u32) -> &'static str {
    if tahun > 80 {
        "Panjang umur banget! 🎉"
    } else if tahun > 60 {
        "Lumayan panjang~ ✨"
    } else if tahun > 40 {
        "Cukup lah ya 😊"
    } else {
        "Jaga kesehatan ya! 🙏"
    }
}

pub(crate) fn sisaumur_text(ctx: &Ctx, tahun: u32, bulan: u32, hari: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = sisaumur_desc(tahun);
    let durasi = format!("{tahun} Tahun {bulan} Bulan {hari} Hari");
    if is_self {
        format!("Hai @{number}\n    \nSisa umur kamu *{durasi}\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kesisaumuran @{number} yak? \n    \nSisa umur dia sebesar *{durasi}\n```{desc}```"
        )
    }
}

#[plugin(
    name = "ceksisaumur",
    alias = ["sisaumur"],
    category = "cek",
    description = "Cek sisa umur kamu",
    usage = ".ceksisaumur <nama>",
    example = ".ceksisaumur Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn ceksisaumur(ctx: Ctx) -> Reply {
    Reply::Text {
        text: sisaumur_text(&ctx, sisaumur_tahun(), sisaumur_bulan(), sisaumur_hari()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
