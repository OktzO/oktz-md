use crate::common::{num_and_self, roll};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `floor(rand * 101)` -> 0..=100.
pub(crate) fn karma_percent() -> u32 {
    roll(101)
}

/// Ambang persis `cekkarma.js:22-26` — semua `>=`, semua kelipatan 20.
pub(crate) fn karma_desc(percent: u32) -> &'static str {
    if percent >= 80 {
        "Karma baik! Surga menantimu~ ✨"
    } else if percent >= 60 {
        "Cukup baik, terus tingkatkan! 🙏"
    } else if percent >= 40 {
        "Netral, perbanyak kebaikan~ ⚖️"
    } else if percent >= 20 {
        "Hati-hati dengan karma buruk! ⚠️"
    } else {
        "Wah perlu banyak tobat nih... 😱"
    }
}

pub(crate) fn karma_text(ctx: &Ctx, percent: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = karma_desc(percent);
    if is_self {
        format!("Hai @{number}\n    \nTingkat kekarmaan kamu *{percent}%\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kekarmaan @{number} yak? \n    \nTingkat kekarmaan dia sebesar *{percent}%\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekkarma",
    alias = ["karma"],
    category = "cek",
    description = "Cek tingkat karma kamu",
    usage = ".cekkarma <nama>",
    example = ".cekkarma Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekkarma(ctx: Ctx) -> Reply {
    Reply::Text {
        text: karma_text(&ctx, karma_percent()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}
