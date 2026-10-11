use oktz_plugin_api::{Ctx, Reply, plugin};
use rand::Rng as _;

pub(crate) const ELEMENTS: [&str; 8] = [
    "Api 🔥",
    "Air 💧",
    "Tanah 🌍",
    "Angin 🌪️",
    "Petir ⚡",
    "Es ❄️",
    "Cahaya ✨",
    "Bayangan 🌑",
];

pub(crate) const ZODIAC: [&str; 12] = [
    "♈ Aries",
    "♉ Taurus",
    "♊ Gemini",
    "♋ Cancer",
    "♌ Leo",
    "♍ Virgo",
    "♎ Libra",
    "♏ Scorpio",
    "♐ Sagittarius",
    "♑ Capricorn",
    "♒ Aquarius",
    "♓ Pisces",
];

pub(crate) const SOUL_TYPES: [&str; 9] = [
    "Pemimpin Yang Berani",
    "Penyeimbang Bijaksana",
    "Kreator Ekspresif",
    "Pembangun Solid",
    "Petualang Bebas",
    "Pelindung Setia",
    "Pemikir Mistis",
    "Penakluk Kuat",
    "Humanitarian Murni",
];

pub(crate) const HELP_SOULMATCH: &str = "💫 *sᴏᴜʟ ᴍᴀᴛᴄʜ*\n\n> Cek kecocokan jiwa 2 orang!\n\n*Format:*\n> `.soulmatch nama1|nama2`\n\n*Contoh:*\n> `.soulmatch Raiden|Mei`";

/// Jumlah sum of char codes — `Array.from(s.toLowerCase()).reduce((a,c)=>a+c.charCodeAt(0),0)`.
pub(crate) fn name_value(name: &str) -> u32 {
    name.to_lowercase().chars().map(|c| c as u32).sum()
}

/// `(element, zodiac, soulType)` untuk satu nama + seed.
pub(crate) fn soul_data(name: &str, seed: u32) -> (&'static str, &'static str, &'static str) {
    let n = name_value(name);
    let idx =
        |len: usize, mul: u32| ((n.wrapping_add(seed.wrapping_mul(mul))) % len as u32) as usize;
    (
        ELEMENTS[idx(ELEMENTS.len(), 1)],
        ZODIAC[idx(ZODIAC.len(), 2)],
        SOUL_TYPES[idx(SOUL_TYPES.len(), 3)],
    )
}

/// `(baseScore % 51) + 50` → selalu di `[50, 100]`.
pub(crate) fn compatibility(nama1: &str, nama2: &str) -> u32 {
    (name_value(&format!("{nama1}{nama2}")) % 51) + 50
}

pub(crate) fn match_description(score: u32) -> &'static str {
    if score >= 90 {
        "💫 Takdir Sejati"
    } else if score >= 80 {
        "✨ Harmoni Sempurna"
    } else if score >= 70 {
        "🌟 Koneksi Kuat"
    } else if score >= 60 {
        "⭐ Potensi Bagus"
    } else if score >= 50 {
        "🌙 Perlu Perjuangan"
    } else {
        "🌑 Tantangan Berat"
    }
}

pub(crate) fn reading(score: u32) -> &'static str {
    if score >= 80 {
        "Jiwa kalian memiliki koneksi yang sangat istimewa dan langka. Takdir telah merencanakan pertemuan ini."
    } else if score >= 60 {
        "Ada chemistry yang kuat di antara kalian. Perbedaan kalian justru menciptakan harmoni."
    } else if score >= 40 {
        "Butuh waktu untuk saling memahami. Setiap tantangan akan memperkuat ikatan kalian."
    } else {
        "Perbedaan signifikan dalam energi jiwa. Butuh banyak adaptasi dan pengertian."
    }
}

/// Seed acak 0..100 — JS memakai `Date.now() % 100`; `Date.now()` tidak ada di
/// `Ctx`, jadi disamakan dengan undian seragam.
pub(crate) fn soul_seed() -> u32 {
    rand::rng().random_range(0..100)
}

pub(crate) fn render_soulmatch(
    nama1: &str,
    nama2: &str,
    s1: (&str, &str, &str),
    s2: (&str, &str, &str),
    score: u32,
) -> String {
    let mut t = String::new();
    t.push_str("╭═══❯ *💫 SOUL MATCH* ❮═══\n");
    t.push_str("│\n");
    t.push_str(&format!("│ 👤 *{nama1}*\n"));
    t.push_str(&format!("│ ├ 🔮 Soul: {}\n", s1.2));
    t.push_str(&format!("│ ├ 🌟 Element: {}\n", s1.0));
    t.push_str(&format!("│ └ 🎯 Zodiac: {}\n", s1.1));
    t.push_str("│\n");
    t.push_str(&format!("│ 👤 *{nama2}*\n"));
    t.push_str(&format!("│ ├ 🔮 Soul: {}\n", s2.2));
    t.push_str(&format!("│ ├ 🌟 Element: {}\n", s2.0));
    t.push_str(&format!("│ └ 🎯 Zodiac: {}\n", s2.1));
    t.push_str("│\n");
    t.push_str("│ 💕 *COMPATIBILITY*\n");
    t.push_str(&format!("│ ├ 📊 Score: *{score}%*\n"));
    t.push_str(&format!("│ └ 🎭 Status: {}\n", match_description(score)));
    t.push_str("│\n");
    t.push_str("│ 🔮 *Reading:*\n");
    t.push_str(&format!("│ {}\n", reading(score)));
    t.push_str("│\n");
    t.push_str("╰════════════════════");
    t
}

#[plugin(
    name = "soulmatch",
    alias = [],
    category = "fun",
    description = "Cek kecocokan jiwa dengan seseorang",
    usage = ".soulmatch nama1|nama2",
    example = ".soulmatch Raiden|Mei",
    cooldown = 15,
    energi = 1,
    is_private = false
)]
pub fn soulmatch(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() || !text.contains('|') {
        return Reply::text(HELP_SOULMATCH);
    }

    let mut names = text.split('|').map(str::trim);
    let nama1 = names.next().unwrap_or("");
    let nama2 = names.next().unwrap_or("");
    if nama1.is_empty() || nama2.is_empty() {
        return Reply::text(format!(
            "❌ Masukkan 2 nama dengan format: `{}soulmatch nama1|nama2`",
            ctx.prefix
        ));
    }

    let s1 = soul_data(nama1, soul_seed());
    let s2 = soul_data(nama2, soul_seed());
    Reply::text(render_soulmatch(
        nama1,
        nama2,
        s1,
        s2,
        compatibility(nama1, nama2),
    ))
}
