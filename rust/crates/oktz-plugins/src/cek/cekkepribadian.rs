use crate::common::random_index;
use oktz_plugin_api::{Ctx, Reply, plugin};

/// `(type, title, desc)` — urutan persis seperti `personalities` di JS;
/// indeks hasil undian bergantung urutan ini.
pub(crate) const PERSONALITIES: [(&str, &str, &str); 16] = [
    (
        "INTJ",
        "The Architect",
        "Visioner, strategis, dan independen",
    ),
    ("INTP", "The Logician", "Analitis, inovatif, dan ingin tahu"),
    (
        "ENTJ",
        "The Commander",
        "Tegas, ambisius, dan pemimpin alami",
    ),
    (
        "ENTP",
        "The Debater",
        "Cerdas, penasaran, dan suka tantangan",
    ),
    (
        "INFJ",
        "The Advocate",
        "Idealis, bijaksana, dan penuh empati",
    ),
    ("INFP", "The Mediator", "Kreatif, idealis, dan setia"),
    (
        "ENFJ",
        "The Protagonist",
        "Karismatik, inspiratif, dan peduli",
    ),
    ("ENFP", "The Campaigner", "Antusias, kreatif, dan sosial"),
    (
        "ISTJ",
        "The Logistician",
        "Bertanggung jawab, praktis, dan teliti",
    ),
    ("ISFJ", "The Defender", "Setia, suportif, dan reliable"),
    (
        "ESTJ",
        "The Executive",
        "Terorganisir, tegas, dan tradisional",
    ),
    ("ESFJ", "The Consul", "Peduli, sosial, dan loyal"),
    ("ISTP", "The Virtuoso", "Fleksibel, observan, dan praktis"),
    ("ISFP", "The Adventurer", "Artistik, sensitif, dan spontan"),
    (
        "ESTP",
        "The Entrepreneur",
        "Energik, perceptive, dan berani",
    ),
    ("ESFP", "The Entertainer", "Spontan, energik, dan fun"),
];

#[plugin(
    name = "cekkepribadian",
    alias = ["kepribadian", "personality"],
    category = "cek",
    description = "Cek kepribadian kamu",
    usage = ".cekkepribadian <nama>",
    example = ".cekkepribadian Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekkepribadian(ctx: Ctx) -> Reply {
    let mentioned = ctx.mention_or_sender().to_string();
    let number = mentioned.split('@').next().unwrap_or(&mentioned);
    let (p_type, p_title, p_desc) = PERSONALITIES[random_index(PERSONALITIES.len())];

    let txt = if mentioned == ctx.sender {
        format!(
            "Hai @{number}\n    \nTingkat kepribadian kamu *{p_type} - {p_title}*\n```{p_desc}```"
        )
    } else {
        format!(
            "Kamu ingin ngecek kepribadian @{number} yak? \n    \nKepribadian dia adalah *{p_type} - {p_title}*\n```{p_desc}```"
        )
    };

    Reply::Text {
        text: txt,
        mentions: vec![mentioned],
    }
}
