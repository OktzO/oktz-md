use crate::common::random_index;
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const RATE: [(&str, &str); 20] = [
    ("10/10", "Sempurna! Nggak ada duanya!"),
    ("9/10", "Hampir sempurna! Keren banget!"),
    ("8/10", "Bagus banget! Mantap!"),
    ("7/10", "Cukup bagus, di atas rata-rata!"),
    ("6/10", "Lumayan, bisa lebih baik lagi."),
    ("5/10", "Biasa aja sih, standar."),
    ("4/10", "Hmm, kurang sedikit."),
    ("3/10", "Perlu banyak perbaikan."),
    ("2/10", "Aduh, masih jauh dari bagus."),
    ("1/10", "Maaf, tapi ini parah."),
    ("100/10", "LEGEND! Beyond perfect!"),
    ("11/10", "Melebihi ekspektasi!"),
    ("69/100", "Nice..."),
    ("420/10", "BLAZING!"),
    ("∞/10", "Gacor kang"),
    ("7.5/10", "Solid! Good job!"),
    ("8.5/10", "Impressive!"),
    ("9.5/10", "Near perfection!"),
    ("-1/10", "Aku nggak tau harus ngomong apa..."),
    ("???/10", "Error 404: Rating not found."),
];

pub(crate) const HELP_RATE: &str =
    "⭐ *ʀᴀᴛᴇ*\n\n> Masukkan sesuatu untuk Dinilai!\n\n*Contoh:*\n> .rate wajahku";

pub(crate) fn render_rate(score: &str, comment: &str) -> String {
    format!("Rating dari aku: *{score}*\n{comment}")
}

#[plugin(
    name = "rate",
    alias = ["nilai", "rating"],
    category = "fun",
    description = "Minta bot memberi rating sesuatu",
    usage = ".rate <sesuatu>",
    example = ".rate wajahku",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn rate(ctx: Ctx) -> Reply {
    if ctx.text.trim().is_empty() {
        return Reply::text(HELP_RATE);
    }
    let (score, comment) = RATE[random_index(RATE.len())];
    Reply::text(render_rate(score, comment))
}
