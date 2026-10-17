//! `plugins/info/jadwalbola.js` — jadwal sepak bola dari `api.neoxr.eu`.
//!
//! `ponytail:` reaksi hilang sebagian. JS bereaksi `🕕`, lalu `❌`, `✅`, atau
//! `☢`; `Reply::Reaction` hanya bisa membawa satu emoji, jadi hanya reaksi
//! terakhir yang ikut.
//!
//! `ponytail:` `ctx.http` tidak bisa mengikuti `sendWithRetry`/delay dari
//! plugin lain, dan `jadwalbola.js` memang tidak punya retry — satu `GET`
//! lalu selesai.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;

/// `config.APIkey.neoxr` dibaca dari `ctx.config()` — nilainya sudah
/// `process.env.APIKEY_NEOXR || ""` di `oktz_core::config::ApiKeys`, jadi
/// string kosong di produksi sama dengan `"|| ""` di JS.
pub(crate) fn url(api_key: &str) -> String {
    format!("https://api.neoxr.eu/api/bola?apikey={api_key}")
}

/// `te(prefix, command, pushName)` (`src/lib/error.js:3-9`).
/// `Ctx` tidak membawa `pushName`, jadi `{pushName}` jadi `User` seperti di JS.
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    crate::common::error_template(prefix, command)
}

/// `LEAGUE_EMOJI` (`jadwalbola.js:27-37`) — urutan penting, karena
/// `Object.entries` dipakai dan yang pertama cocok menang.
const LEAGUE_EMOJI: [(&str, &str); 9] = [
    ("liga inggris", "🏴󠁧󠁢󠁥󠁮󠁧󠁿"),
    ("liga italia", "🇮🇹"),
    ("liga spanyol", "🇪🇸"),
    ("la liga spanyol", "🇪🇸"),
    ("liga jerman", "🇩🇪"),
    ("liga prancis", "🇫🇷"),
    ("liga belanda", "🇳🇱"),
    ("liga champions", "🏆"),
    ("bri super league", "🇮🇩"),
];

/// `getLeagueEmoji(league)` — `lower.includes(key) || key.includes(lower)`.
///
/// String kosong ikut mengembalikan emoji pertama, persis seperti
/// `key.includes("")` yang selalu benar di JS.
pub(crate) fn league_emoji(league: &str) -> &'static str {
    let lower = league.to_lowercase();
    for (key, emoji) in LEAGUE_EMOJI {
        if lower.contains(key) || key.contains(&lower) {
            return emoji;
        }
    }
    "⚽"
}

/// `${value}` di JS — `undefined` untuk field yang hilang.
fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Kunci pengelompokan: `match.date || "TBA"`, lalu dipaksa jadi string
/// seperti indeks objek JS.
fn date_key(match_: &Value) -> String {
    match match_.get("date") {
        Some(v) if js_truthy_value(v) => interp(Some(v)),
        _ => String::from("TBA"),
    }
}

/// `.filter(...)` di `jadwalbola.js:63-70` —liga, home, away, atau tanggal
/// mengandung `filter` (sudah lowercase).
pub(crate) fn filter_matches(matches: &[Value], filter: &str) -> Vec<Value> {
    if filter.is_empty() {
        return matches.to_vec();
    }
    matches
        .iter()
        .filter(|m| {
            ["league", "home_team", "away_team", "date"]
                .iter()
                .any(|key| {
                    m.get(key)
                        .and_then(Value::as_str)
                        .is_some_and(|v| v.to_lowercase().contains(filter))
                })
        })
        .cloned()
        .collect()
}

/// Kegagalan saat menyusun balasan, dipetakan ke cabang JS masing-masing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RenderErr {
    /// `matches.length === 0` setelah filter → `❌` + pesan filter.
    NotFound,
    /// `match.league` bukan string → `getLeagueEmoji` melempar → `catch`.
    Throw,
}

/// Teks balasan untuk daftar pertandingan yang sudah difilter.
pub(crate) fn render(matches: &[Value], filter: &str) -> Result<String, RenderErr> {
    if matches.is_empty() {
        return Err(RenderErr::NotFound);
    }

    // `grouped[date]` — urutan sisip, sama seperti `Object.entries` untuk key
    // string. `slice(0, 50)` membatasi yang ikut dirender, tapi `matches.len()`
    // di bawah tetap memakai panjang daftar SEBELUM dipotong.
    let mut groups: Vec<(String, Vec<&Value>)> = Vec::new();
    for match_ in matches.iter().take(50) {
        let key = date_key(match_);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, list)) => list.push(match_),
            None => groups.push((key, vec![match_])),
        }
    }

    let mut text = String::from("⚽ *ᴊᴀᴅᴡᴀʟ ᴘᴇʀᴛᴀɴᴅɪɴɢᴀɴ*\n\n");
    if !filter.is_empty() {
        text.push_str(&format!("> Filter: `{filter}`\n\n"));
    }
    for (date, games) in &groups {
        text.push_str(&format!("📅 *{date}*\n\n"));
        for game in games {
            // `game.league.toLowerCase()` melempar kalau `league` bukan string.
            let league = game
                .get("league")
                .and_then(Value::as_str)
                .ok_or(RenderErr::Throw)?;
            text.push_str(&format!(
                "{} *{}*\n⏰ {}\n🏠 {}\n🆚 {}\n\n",
                league_emoji(league),
                interp(game.get("league")),
                interp(game.get("time")),
                interp(game.get("home_team")),
                interp(game.get("away_team")),
            ));
        }
    }
    text.push_str(&format!("Total: *{}* pertandingan", matches.len()));
    Ok(text)
}

#[plugin(
    name = "jadwalbola",
    alias = ["bola", "football", "soccer", "jadwalsepakbola"],
    category = "info",
    description = "Lihat jadwal pertandingan sepak bola",
    usage = ".jadwalbola [liga]",
    example = ".jadwalbola inggris",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn jadwalbola(ctx: Ctx) -> Reply {
    let filter = ctx.args.join(" ").to_lowercase().trim().to_string();

    let res = match ctx.http.get(&url(&ctx.config().api_keys.neoxr)) {
        Ok(r) => r,
        Err(_) => return error_branch(&ctx),
    };
    // `f()` mengembalikan `null` saat gagal, dan `data?.status` yang null
    // membuat `throw new Error("Tidak ada jadwal tersedia")` di JS.
    let data = res.json().unwrap_or(Value::Null);
    let Some(all) = data
        .get("data")
        .filter(|_| data.get("status").is_some_and(js_truthy_value))
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
    else {
        return error_branch(&ctx);
    };

    match render(&filter_matches(all, &filter), &filter) {
        Ok(text) => Reply::text(text).with_reaction("✅"),
        Err(RenderErr::NotFound) => {
            Reply::text(format!("❌ Tidak ditemukan jadwal untuk: `{filter}`")).with_reaction("❌")
        }
        Err(RenderErr::Throw) => error_branch(&ctx),
    }
}

fn error_branch(ctx: &Ctx) -> Reply {
    Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(filter: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "jadwalbola".into(),
            text: filter.into(),
            args: filter.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("jadwalbola").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    fn sample() -> Vec<Value> {
        vec![
            json!({"league": "Liga Inggris", "time": "20:00", "home_team": "Arsenal", "away_team": "Chelsea", "date": "2026-05-01"}),
            json!({"league": "Liga Inggris", "time": "22:00", "home_team": "Liverpool", "away_team": "Everton", "date": "2026-05-01"}),
            json!({"league": "Liga Italia", "time": "21:45", "home_team": "Milan", "away_team": "Roma", "date": "2026-05-02"}),
        ]
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("jadwalbola").unwrap();
        assert_eq!(p.meta.name, "jadwalbola");
        assert_eq!(
            p.meta.alias,
            ["bola", "football", "soccer", "jadwalsepakbola"]
        );
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Lihat jadwal pertandingan sepak bola");
        assert_eq!(p.meta.usage, ".jadwalbola [liga]");
        assert_eq!(p.meta.example, ".jadwalbola inggris");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["bola", "football", "soccer", "jadwalsepakbola"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "jadwalbola");
        }
    }

    #[test]
    fn a_dead_http_client_lands_on_the_error_template_branch() {
        let r = run("");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.jadwalbola` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), "☢");
    }

    #[test]
    fn the_api_url_carries_the_config_key() {
        assert_eq!(
            url("KUNCI"),
            "https://api.neoxr.eu/api/bola?apikey=KUNCI"
        );
        // `Ctx::default()` config-nya kosong, jadi jalurINI tetap menghasilkan
        // `apikey=` persis seperti `process.env.APIKEY_NEOXR || ""`.
        assert_eq!(url(""), "https://api.neoxr.eu/api/bola?apikey=");
    }

    #[test]
    fn an_empty_list_never_reaches_render() {
        assert_eq!(render(&[], ""), Err(RenderErr::NotFound));
        let after = filter_matches(&sample(), "liga prancis");
        assert!(after.is_empty());
        assert_eq!(render(&after, "liga prancis"), Err(RenderErr::NotFound));
    }

    #[test]
    fn the_filter_matches_league_home_away_or_date() {
        let all = sample();
        // "sample" punya dua laga Liga Inggris, jadi "inggris" memang dua.
        for (f, expect) in [
            ("inggris", 2),
            ("italia", 1),
            ("arsenal", 1),
            ("everton", 1),
            ("2026-05-02", 1),
        ] {
            assert_eq!(filter_matches(&all, f).len(), expect, "filter {f:?} gagal");
        }
        assert_eq!(filter_matches(&all, "liga").len(), 3);
        assert!(filter_matches(&all, "tidak ada").is_empty());
        assert_eq!(
            filter_matches(&all, "").len(),
            3,
            "filter kosong = tanpa filter, bukan 'cocok tidak ada'"
        );
    }

    #[test]
    fn matches_are_grouped_by_date_in_first_seen_order() {
        let all = sample();
        let out = render(&all, "").unwrap();
        let d1 = out.find("📅 *2026-05-01*").expect("hari pertama");
        let d2 = out.find("📅 *2026-05-02*").expect("hari kedua");
        assert!(d1 < d2, "urutan tanggal salah: {out}");
        assert_eq!(
            out.matches("📅 *").count(),
            2,
            "tiga laga harus jadi dua grup: {out}"
        );
        assert!(out.ends_with("Total: *3* pertandingan"), "{out}");
    }

    #[test]
    fn the_total_counts_every_match_not_only_the_first_fifty() {
        let all: Vec<Value> = (0..60)
            .map(|i| {
                json!({"league": "Liga Inggris", "time": "20:00", "home_team": "A", "away_team": "B", "date": format!("2026-05-{:02}", (i % 28) + 1)})
            })
            .collect();
        let out = render(&all, "").unwrap();
        assert_eq!(out.matches("🏠 A\n").count(), 50, "hanya 50 dirender");
        assert!(out.ends_with("Total: *60* pertandingan"), "{out}");
    }

    #[test]
    fn a_match_without_a_date_falls_into_the_tba_group() {
        let all = vec![
            json!({"league": "Liga Inggris", "time": "20:00", "home_team": "A", "away_team": "B"}),
        ];
        assert!(render(&all, "").unwrap().contains("📅 *TBA*"));
        assert_eq!(date_key(&json!({"date": null})), "TBA");
        assert_eq!(date_key(&json!({"date": ""})), "TBA");
        assert_eq!(date_key(&json!({})), "TBA");
    }

    #[test]
    fn the_filter_line_only_appears_when_a_filter_was_given() {
        assert!(!render(&sample(), "").unwrap().contains("Filter:"));
        assert!(
            render(&sample(), "inggris")
                .unwrap()
                .contains("> Filter: `inggris`")
        );
    }

    #[test]
    fn a_league_that_is_not_a_string_is_a_throw_not_a_crash() {
        let all = vec![json!({"league": 7, "time": "20:00"})];
        assert_eq!(render(&all, ""), Err(RenderErr::Throw));
        let all = vec![json!({"time": "20:00"})];
        assert_eq!(render(&all, ""), Err(RenderErr::Throw));
    }

    #[test]
    fn missing_match_fields_are_interpolated_as_undefined() {
        let all = vec![json!({"league": "LigaItalia"})];
        let out = render(&all, "").unwrap();
        assert!(out.contains("⏰ undefined"), "{out}");
        assert!(out.contains("🏠 undefined"), "{out}");
        assert!(out.contains("🆚 undefined"), "{out}");
    }

    #[test]
    fn the_league_emoji_matches_the_js_first_hit_order() {
        assert_eq!(league_emoji("Liga Inggris"), "🏴󠁧󠁢󠁥󠁮󠁧󠁿");
        assert_eq!(league_emoji("BRI SUPER LEAGUE"), "🇮🇩");
        // Kuncinya "liga champions", sedangkan inputnya "champions league":
        // `"champions league".includes("liga champions")` dan
        // `"liga champions".includes("champions league")` sama-sama false,
        // jadi JS jatuh ke `⚽`.
        assert_eq!(league_emoji("Champions League"), "⚽", "urut kata tidak sama");
        // `lower.includes(key)` menang lebih dulu: "Liga Italia" memuat
        // "liga italia", dan urutan peta menempatkan Italia sebelum Spanyol.
        assert_eq!(league_emoji("Liga Italia"), "🇮🇹");
        // `key.includes(lower)` — arah `includes` yang terbalik: "champ" bukan
        // substring dari "liga champions", tapi sebaliknya benar.
        assert_eq!(league_emoji("Champ"), "🏆");
        assert_eq!(
            league_emoji("Liga"),
            "🏴󠁧󠁢󠁥󠁮󠁧󠁿",
            "string pendek → kunci pertama"
        );
        assert_eq!(league_emoji("Liga nowhere"), "⚽", "tidak ada yang cocok");
    }
}
