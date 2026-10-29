use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `floor(rand * 80) + 5` -> 5..=84. Offset ditambahkan SETELAH floors.
pub(crate) fn umur_percent() -> u32 {
    roll(80) + 5
}

/// Empat tier — `cekumur.js:22-25`, semua `>=`.
pub(crate) fn umur_desc(percent: u32) -> &'static str {
    if percent >= 60 {
        "Bijaksana seperti orang tua! 🧓"
    } else if percent >= 40 {
        "Dewasa dan matang~ 🧑"
    } else if percent >= 20 {
        "Jiwa muda! 🧒"
    } else {
        "Masih seperti anak kecil~ 👶"
    }
}

pub(crate) fn umur_text(ctx: &Ctx, percent: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = umur_desc(percent);
    if is_self {
        format!("Hai @{number}\n    \nTingkat keumuran kamu *{percent}%\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat keumuran @{number} yak? \n    \nTingkat keumuran dia sebesar *{percent}%\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekumur",
    alias = ["umur", "age"],
    category = "cek",
    description = "Cek umur mental kamu",
    usage = ".cekumur <nama>",
    example = ".cekumur Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekumur(ctx: Ctx) -> Reply {
    Reply::Text {
        text: umur_text(&ctx, umur_percent()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
