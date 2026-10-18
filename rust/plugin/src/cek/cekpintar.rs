use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `floor(rand * 100) + 70` -> 70..=169, yaitu rentang Rust `70..170`.
///
/// Offset +70 inilah yang membuat kelima tier terjangkau: tanpa itu, range
/// 0..=99 membuat tier `>= 150` dan `>= 130` jadi dead branch permanen.
pub(crate) fn pintar_iq() -> u32 {
    roll(100) + 70
}

/// `cekpintar.js:23-33`.
pub(crate) fn pintar_desc(iq: u32) -> &'static str {
    if iq >= 150 {
        "JENIUS! Einstein level! 🧠✨"
    } else if iq >= 130 {
        "Sangat cerdas! 🎓"
    } else if iq >= 110 {
        "Di atas rata-rata! 👍"
    } else if iq >= 90 {
        "Normal, rata-rata 😊"
    } else {
        "Tetap semangat belajar! 📚"
    }
}

pub(crate) fn pintar_text(ctx: &Ctx, iq: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = pintar_desc(iq);
    if is_self {
        format!("Hai @{number}\n    \nKepintaran (IQ) kamu *{iq}\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek kepintaran @{number} yak? \n    \nKepintaran (IQ) dia sebesar *{iq}\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekpintar",
    alias = ["pintar", "iq", "smart"],
    category = "cek",
    description = "Cek seberapa pintar kamu",
    usage = ".cekpintar <nama>",
    example = ".cekpintar Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekpintar(ctx: Ctx) -> Reply {
    Reply::Text {
        text: pintar_text(&ctx, pintar_iq()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
