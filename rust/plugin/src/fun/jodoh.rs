//! `plugins/fun/jodoh.js` — jodohkan dua member grup acak.
//!
//! `ponytail:` dua hal tidak bisa dicerminkan persis:
//!
//! 1. `sock.user.id` tidak ada di `Ctx`, jadi bot sendiri TIDAK bisa dibuang
//!    dari kandidat. Kalau bot adalah anggota grup, dia bisa jadi hasil
//!    jodoh. Tambahkan `bot_jid: String` ke `Ctx` kalau router mau
//!    membuangnya.
//! 2. `sort(() => Math.random() - 0.5)` di JS adalah shuffle bias. Di sini
//!    Fisher-Yates — tujuannya "ambil dua orang berbeda", dan keduanya
//!    memenuhi itu.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, random_index};

const LOVE_QUOTES: [&str; 8] = [
    "Cinta sejati tidak pernah mengenal jarak 💕",
    "Dua hati yang bersatu takkan terpisahkan 💗",
    "Kalian seperti puzzle yang sempurna 🧩",
    "Match made in heaven! ✨",
    "Chemistry-nya kuat banget! 🔥",
    "Couple goals banget sih kalian 💑",
    "Destiny brought you together 🌟",
    "Perfect match detected! 💘",
];

pub(crate) const ERR_META: &str = "❌ *ɢᴀɢᴀʟ*";
pub(crate) const ERR_MIN_TWO: &str = "> Minimal ada 2 member untuk dijodohkan!";
pub(crate) const ERR_REGISTRATION: &str =
    "> Mode wajib daftar aktif. Minimal harus ada 2 member yang sudah terdaftar di grup ini!";

/// `compatibilityEmoji` — 5 ambang dari `>= 90` turun ke `>= 30`.
pub(crate) fn compatibility_emoji(percent: u32) -> &'static str {
    match percent {
        90.. => "💕💕💕💕💕",
        70..=89 => "💕💕💕💕",
        50..=69 => "💕💕💕",
        30..=49 => "💕💕",
        _ => "💕",
    }
}

/// `compatibilityText`.
pub(crate) fn compatibility_text(percent: u32) -> &'static str {
    match percent {
        90.. => "JODOH SEJATI! 💍",
        70..=89 => "Sangat Cocok! 💖",
        50..=69 => "Lumayan Cocok 💗",
        30..=49 => "Bisa Dicoba 💓",
        _ => "Butuh Usaha Lebih 💔",
    }
}

/// `filled = floor(percent / 10)`, `empty = 10 - filled`.
pub(crate) fn progress_bar(percent: u32) -> String {
    let filled = (percent / 10) as usize;
    "█".repeat(filled) + &"░".repeat(10 - filled)
}

/// Truthiness JavaScript untuk `user.isRegistered`. Objek dan array kosong
/// tetap truthy di JS, jadi keduanya tidak boleh ikut jadi `false`.
fn truthy(v: Option<&serde_json::Value>) -> bool {
    match v {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(serde_json::Value::String(s)) => !s.is_empty(),
        Some(serde_json::Value::Array(_)) | Some(serde_json::Value::Object(_)) => true,
    }
}

fn gender(data: &serde_json::Value) -> Option<&str> {
    data.get("regGender").and_then(serde_json::Value::as_str)
}

/// `allUsers[clean].regName` — hanya dipakai kalau truthy.
fn reg_name(data: &serde_json::Value) -> Option<&str> {
    match data.get("regName") {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

fn is_registered(data: &serde_json::Value) -> bool {
    truthy(data.get("isRegistered"))
}

/// Fisher-Yates untuk `sort(() => Math.random() - 0.5)`.
fn shuffle(items: &[String]) -> Vec<String> {
    use rand::Rng as _;
    let mut out = items.to_vec();
    let mut rng = rand::rng();
    for i in (1..out.len()).rev() {
        out.swap(i, rng.random_range(0..=i));
    }
    out
}

/// `Math.floor(Math.random() * 100) + 1` → `1..=100`.
pub(crate) fn roll_compatibility() -> u32 {
    use rand::Rng as _;
    rand::rng().random_range(1..=100)
}

/// Label gender, dengan fallback acak persis seperti JS.
fn label_for(data: &serde_json::Value, fallback: Option<&str>) -> &'static str {
    match gender(data) {
        Some("Laki-laki") => "👨",
        Some("Perempuan") => "👩",
        _ => match fallback {
            Some("👩") => "👨",
            _ => {
                use rand::Rng as _;
                if rand::rng().random_bool(0.5) { "👩" } else { "👨" }
            }
        },
    }
}

fn display_name(jid: &str, data: &serde_json::Value) -> String {
    let num = jid_number(jid);
    match reg_name(data) {
        Some(n) => format!("*{n}* (@{num})"),
        None => format!("@{num}"),
    }
}

/// Badan pesan, terpisah supaya bisa diuji tanpa `Ctx`.
pub(crate) fn render(
    p1: (&str, &serde_json::Value),
    p2: (&str, &serde_json::Value),
    compatibility: u32,
    used_registration: bool,
    registration_required: bool,
    quote: &str,
) -> String {
    let (person1, data1) = p1;
    let (person2, data2) = p2;
    let label1 = label_for(data1, None);
    let label2 = label_for(data2, Some(label1));
    let name1 = display_name(person1, data1);
    let name2 = display_name(person2, data2);

    let mut t = String::from("💘 *ᴊᴏᴅᴏʜ ʀᴀɴᴅᴏᴍ*\n\n");
    t.push_str("╭┈┈⬡「 💑 *ᴘᴀsᴀɴɢᴀɴ* 」\n");
    t.push_str(&format!("┃ {label1} {name1}\n"));
    t.push_str("┃ ❤️\n");
    t.push_str(&format!("┃ {label2} {name2}\n"));
    t.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    t.push_str("╭┈┈⬡「 📊 *ᴋᴇᴄᴏᴄᴏᴋᴀɴ* 」\n");
    t.push_str(&format!(
        "┃ {} *{compatibility}%*\n",
        progress_bar(compatibility)
    ));
    t.push_str(&format!("┃ {}\n", compatibility_emoji(compatibility)));
    t.push_str(&format!(
        "┃ Status: *{}*\n",
        compatibility_text(compatibility)
    ));
    t.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    if used_registration {
        t.push_str("> ✨ _Dijodohkan berdasarkan data registrasi_\n");
    }
    if registration_required {
        t.push_str("> 🔒 _Mode wajib daftar aktif, hanya member terdaftar yang dipilih_\n");
    }
    t.push_str(&format!("> _\"{quote}\"_"));
    t
}

#[plugin(
    name = "jodoh",
    alias = ["match", "shipcouple", "ship"],
    category = "fun",
    description = "Jodohkan 2 member random dengan kecocokan",
    usage = ".jodoh",
    example = ".jodoh",
    cooldown = 10,
    energi = 1,
    is_group = true,
    is_private = false
)]
pub fn jodoh(ctx: Ctx) -> Reply {
    let members: Vec<String> = ctx.roster.member_jids();
    if members.len() < 2 {
        return Reply::text(format!("{ERR_META}\n\n{ERR_MIN_TWO}"));
    }

    let registered_members: Vec<String> = ctx
        .roster
        .participants
        .iter()
        .filter(|p| is_registered(&p.data))
        .map(|p| p.jid.clone())
        .collect();

    let registration_required = ctx.setting_bool("registrationRequired", false);
    if registration_required && registered_members.len() < 2 {
        return Reply::text(format!("{ERR_META}\n\n{ERR_REGISTRATION}"));
    }

    let registered_in_group: Vec<String> = ctx
        .roster
        .participants
        .iter()
        .filter(|p| is_registered(&p.data) && gender(&p.data).is_some_and(|g| !g.is_empty()))
        .map(|p| p.jid.clone())
        .collect();

    let data_of = |jid: &String| -> serde_json::Value {
        ctx.roster
            .participants
            .iter()
            .find(|p| &p.jid == jid)
            .map(|p| p.data.clone())
            .unwrap_or(serde_json::Value::Null)
    };

    let mut pair: Option<(String, String)> = None;
    let mut used_registration = false;

    if registered_in_group.len() >= 2 {
        let mut males = Vec::new();
        let mut females = Vec::new();
        for jid in &registered_in_group {
            match gender(&data_of(jid)) {
                Some("Laki-laki") => males.push(jid.clone()),
                Some("Perempuan") => females.push(jid.clone()),
                _ => {}
            }
        }

        if !males.is_empty() && !females.is_empty() {
            pair = Some((
                males[random_index(males.len())].clone(),
                females[random_index(females.len())].clone(),
            ));
        } else {
            let shuffled = shuffle(&registered_in_group);
            pair = Some((shuffled[0].clone(), shuffled[1].clone()));
        }
        used_registration = true;
    }

    let pair = pair.unwrap_or_else(|| {
        let candidates = if registration_required {
            &registered_members
        } else {
            &members
        };
        let shuffled = shuffle(candidates);
        (shuffled[0].clone(), shuffled[1].clone())
    });

    let (person1, person2) = pair;
    let compatibility = roll_compatibility();
    let quote = LOVE_QUOTES[random_index(LOVE_QUOTES.len())];
    let text = render(
        (&person1, &data_of(&person1)),
        (&person2, &data_of(&person2)),
        compatibility,
        used_registration,
        registration_required,
        quote,
    );

    Reply::Text {
        text,
        mentions: vec![person1, person2],
    }
    .with_reaction("💕")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use serde_json::{Value};

    fn member(jid: &str, data: Value) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: false,
            is_owner: false,
            data,
        }
    }

    fn ctx_of(members: Vec<Participant>, settings: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "jodoh".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants: members,
                is_bot_admin: false,
            },
            settings,
            ..Default::default()
        }
    }

    fn plain(n: usize) -> Vec<Participant> {
        (0..n)
            .map(|i| member(&format!("62811{i}@s.whatsapp.net"), serde_json::json!({})))
            .collect()
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("jodoh").unwrap().handler)(ctx)
    }

    /// `Reply::Text` polos untuk cabang guard (JS reply tanpa `m.react`),
    /// `Reply::Reaction` untuk cabang yang memutar botol.
    fn split(r: &Reply) -> (String, Vec<String>) {
        let inner = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            other => other,
        };
        match inner {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("jodoh").unwrap();
        assert_eq!(p.meta.name, "jodoh");
        assert_eq!(p.meta.alias, ["match", "shipcouple", "ship"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Jodohkan 2 member random dengan kecocokan");
        assert_eq!(p.meta.usage, ".jodoh");
        assert_eq!(p.meta.example, ".jodoh");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["jodoh", "match", "shipcouple", "ship"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("jodoh"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn fewer_than_two_members_takes_the_guard_branch() {
        for n in [0usize, 1] {
            let (text, mentions) = split(&run(ctx_of(plain(n), serde_json::json!({}))));
            assert_eq!(text, format!("{ERR_META}\n\n{ERR_MIN_TWO}"), "n={n}");
            assert!(mentions.is_empty(), "n={n}");
        }
    }

    #[test]
    fn the_normal_branch_mentions_two_different_members_of_the_roster() {
        let roster = ctx_of(plain(5), serde_json::json!({}));
        let jids: Vec<String> = roster.roster.member_jids();
        for _ in 0..60 {
            let (text, mentions) = split(&run(roster.clone()));
            assert_eq!(mentions.len(), 2, "{text}");
            assert_ne!(mentions[0], mentions[1], "dua orang harus berbeda");
            for m in &mentions {
                assert!(jids.contains(m), "mention di luar roster: {m}");
            }
            assert!(text.starts_with("💘 *ᴊᴏᴅᴏʜ ʀᴀɴᴅᴏᴍ*\n\n"), "{text}");
        }
    }

    #[test]
    fn the_progress_bar_and_bands_agree_with_the_score() {
        for pct in [1u32, 9, 10, 29, 30, 49, 50, 69, 70, 89, 90, 100] {
            let bar = progress_bar(pct);
            assert_eq!(bar.chars().count(), 10, "pct {pct}");
            assert_eq!(bar.chars().filter(|c| *c == '█').count(), (pct / 10) as usize);
            assert_eq!(bar.chars().filter(|c| *c == '░').count(), 10 - (pct / 10) as usize);
            assert!(text_has_band(pct), "pct {pct}");
        }
    }

    fn text_has_band(pct: u32) -> bool {
        let out = render(
            ("1@s.whatsapp.net", &serde_json::json!({})),
            ("2@s.whatsapp.net", &serde_json::json!({})),
            pct,
            false,
            false,
            LOVE_QUOTES[0],
        );
        out.contains(compatibility_text(pct)) && out.contains(compatibility_emoji(pct))
    }

    #[test]
    fn the_compatibility_roll_stays_in_one_to_one_hundred() {
        let mut seen = vec![0u32; 101];
        for _ in 0..5000 {
            let v = roll_compatibility();
            assert!((1..=100).contains(&v), "di luar 1..=100: {v}");
            seen[v as usize] += 1;
        }
        assert!(seen[1] > 0, "nilai 1 tidak pernah muncul");
        assert!(seen[100] > 0, "nilai 100 tidak pernah muncul");
    }

    #[test]
    fn registered_members_of_both_genders_always_get_paired_across() {
        let members = vec![
            member("1@s.whatsapp.net", serde_json::json!({"isRegistered": true, "regGender": "Laki-laki", "regName": "Budi"})),
            member("2@s.whatsapp.net", serde_json::json!({"isRegistered": true, "regGender": "Perempuan", "regName": "Sari"})),
            member("3@s.whatsapp.net", serde_json::json!({})),
        ];
        for _ in 0..40 {
            let (text, mentions) = split(&run(ctx_of(members.clone(), serde_json::json!({}))));
            assert!(
                text.contains("> ✨ _Dijodohkan berdasarkan data registrasi_"),
                "{text}"
            );
            let (a, b) = (&mentions[0], &mentions[1]);
            let g = |jid: &String| {
                if jid == "1@s.whatsapp.net" { "Laki-laki" } else { "Perempuan" }
            };
            assert_ne!(g(a), g(b), "laki-laki harus dipasangkan dengan perempuan: {text}");
            assert!(text.contains("*Budi* (@1)") || text.contains("*Sari* (@2)"), "{text}");
        }
    }

    #[test]
    fn a_single_gender_falls_back_to_a_shuffle_of_the_registered_pool() {
        let members = vec![
            member("1@s.whatsapp.net", serde_json::json!({"isRegistered": true, "regGender": "Laki-laki"})),
            member("2@s.whatsapp.net", serde_json::json!({"isRegistered": true, "regGender": "Laki-laki"})),
            member("3@s.whatsapp.net", serde_json::json!({})),
        ];
        let (text, mentions) = split(&run(ctx_of(members, serde_json::json!({}))));
        assert!(text.contains("Dijodohkan berdasarkan data registrasi"), "{text}");
        assert!(mentions.iter().all(|m| m.ends_with("1@s.whatsapp.net") || m.ends_with("2@s.whatsapp.net")), "{text}");
    }

    #[test]
    fn registration_mode_blocks_unregistered_groups() {
        let members = vec![
            member("1@s.whatsapp.net", serde_json::json!({"isRegistered": true})),
            member("2@s.whatsapp.net", serde_json::json!({})),
        ];
        let (text, mentions) = split(&run(ctx_of(members, serde_json::json!({"registrationRequired": true}))));
        assert_eq!(text, format!("{ERR_META}\n\n{ERR_REGISTRATION}"));
        assert!(mentions.is_empty());
    }

    #[test]
    fn registration_mode_pairs_from_the_registered_pool_only() {
        let members = vec![
            member("1@s.whatsapp.net", serde_json::json!({"isRegistered": true})),
            member("2@s.whatsapp.net", serde_json::json!({"isRegistered": true})),
            member("3@s.whatsapp.net", serde_json::json!({})),
        ];
        let (text, mentions) = split(&run(ctx_of(members, serde_json::json!({"registrationRequired": true}))));
        assert!(mentions.iter().all(|m| m.ends_with("1@s.whatsapp.net") || m.ends_with("2@s.whatsapp.net")), "{text}");
        assert!(text.contains("> 🔒 _Mode wajib daftar aktif, hanya member terdaftar yang dipilih_"), "{text}");
    }

    #[test]
    fn labels_and_names_follow_the_registered_gender() {
        let out = render(
            (
                "628111@s.whatsapp.net",
                &serde_json::json!({"regGender": "Laki-laki", "regName": "Budi"}),
            ),
            ("628222@s.whatsapp.net", &serde_json::json!({"regGender": "Perempuan"})),
            74,
            false,
            false,
            LOVE_QUOTES[0],
        );
        assert!(out.contains("┃ 👨 *Budi* (@628111)"), "{out}");
        assert!(out.contains("┃ 👩 @628222"), "{out}");
        assert!(out.ends_with(&format!("> _\"{}\"_", LOVE_QUOTES[0])), "{out}");
    }

    #[test]
    fn the_reply_only_reads_the_roster() {
        assert!(run(ctx_of(plain(4), serde_json::json!({}))).mutations().is_empty());
    }
}
