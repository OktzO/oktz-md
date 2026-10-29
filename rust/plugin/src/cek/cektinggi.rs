use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `floor(rand * 50) + 150` -> 150..=199, yaitu rentang Rust `150..200`.
pub(crate) fn tinggi_cm() -> u32 {
    roll(50) + 150
}

/// `cektinggi.js:23-33`.
pub(crate) fn tinggi_desc(cm: u32) -> &'static str {
    if cm >= 190 {
        "TINGGI BANGET! Model basketball! 🏀"
    } else if cm >= 175 {
        "Tinggi ideal! 😎"
    } else if cm >= 165 {
        "Lumayan tinggi 👍"
    } else if cm >= 155 {
        "Standard kok 🙂"
    } else {
        "Imut dan mungil! 🥺"
    }
}

/// Kalimat "orang lain" di JS memang berbeda dari kalimat self: `tingkat
/// ketinggian` vs `Tinggi badan`. Ketidakkonsistenan itu dibiarkan — itu isi
/// file aslinya, bukan bug yang boleh dirapikan diam-diam.
pub(crate) fn tinggi_text(ctx: &Ctx, cm: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = tinggi_desc(cm);
    if is_self {
        format!("Hai @{number}\n    \nTinggi badan kamu *{cm} cm\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat ketinggian @{number} yak? \n    \nTinggi badan dia sebesar *{cm} cm\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cektinggi",
    alias = ["tinggi", "tall"],
    category = "cek",
    description = "Cek tinggi badan random",
    usage = ".cektinggi <nama>",
    example = ".cektinggi Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cektinggi(ctx: Ctx) -> Reply {
    Reply::Text {
        text: tinggi_text(&ctx, tinggi_cm()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
