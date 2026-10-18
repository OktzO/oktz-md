use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `floor(rand * 60) + 40` -> 40..=99, yaitu rentang Rust `40..100`.
pub(crate) fn berat_kg() -> u32 {
    roll(60) + 40
}

/// `cekberat.js:22-32`. Ambang 55 dan 45 (bukan kelipatan 10) — inilah yang
/// membedakannya dari `cek.rs`.
pub(crate) fn berat_desc(kg: u32) -> &'static str {
    if kg >= 90 {
        "Big boy/girl! 💪"
    } else if kg >= 70 {
        "Berisi dan sehat! 😊"
    } else if kg >= 55 {
        "Ideal banget! 👍"
    } else if kg >= 45 {
        "Langsing nih~ 🌸"
    } else {
        "Kurus banget, makan yang banyak! 🍔"
    }
}

pub(crate) fn berat_text(ctx: &Ctx, kg: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = berat_desc(kg);
    if is_self {
        format!("Hai @{number}\n    \nBerat badan kamu *{kg} kg\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek berat badan @{number} yak? \n    \nBerat badan dia sebesar *{kg} kg\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekberat",
    alias = ["berat", "weight"],
    category = "cek",
    description = "Cek berat badan random",
    usage = ".cekberat <nama>",
    example = ".cekberat Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekberat(ctx: Ctx) -> Reply {
    Reply::Text {
        text: berat_text(&ctx, berat_kg()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
