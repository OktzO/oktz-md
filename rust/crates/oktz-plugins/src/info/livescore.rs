//! `plugins/info/livescore.js` — live score sepak bola dari `goal.com`.
//!
//! Sumbernya halaman HTML dengan satu blok `<script id="__NEXT_DATA__">`
//! berisi JSON. Blok itu diambil persis seperti regex JS
//! `/__NEXT_DATA__" type="application\/json">(.*?)<\/script>/s` — penanda
//! tetap, lalu berhenti di `</script>` pertama. Tidak ada parser HTML di
//! `oktz-plugin-api`, dan tidak boleh menambah dependency baru, jadi ini
//! satu-satunya potongan HTML yang memang dibutuhkan plugin ini.
//!
//! `ponytail:` reaksi hilang sebagian. JS bereaksi `🕕`, lalu `✅` atau `❌`;
//! `Reply::Reaction` hanya bisa membawa satu emoji, jadi hanya reaksi
//! terakhir yang ikut.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;

pub(crate) const URL: &str = "https://www.goal.com/id/livescore";

/// Penanda sebelum payload JSON — sama persis dengan regex JS.
const MARKER: &str = "__NEXT_DATA__\" type=\"application/json\">";

pub(crate) const NO_MATCH: &str = "⚽ *LIVESCORE BOLA* ⚽\n\n\
    Saat ini tidak ada pertandingan yang sedang berlangsung atau dijadwalkan.";

#[allow(dead_code)] // teks dipakai dari test saja
pub(crate) const NO_SCORE: &str = "❌ *GAGAL MENGAMBIL SKOR*\n\n\
    Maaf, sistem gagal mengambil data live score saat ini.";

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk null.
fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `_statusIndo(status)` (`livescore.js:47-55`) — kunci yang tidak dikenal
/// diteruskan apa adanya.
pub(crate) fn status_indo(status: Option<&Value>) -> String {
    let key = match status {
        Some(Value::String(s)) => s.as_str(),
        _ => return interp(status),
    };
    match key {
        "FIXTURE" => "Belum Mulai",
        "LIVE" => "Berlangsung",
        "FINISHED" => "Selesai",
        "POSTPONED" => "Ditunda",
        "CANCELLED" => "Dibatalkan",
        "HALF_TIME" => "Turun Minum",
        other => other,
    }
    .to_string()
}

/// Rantai `??` pada `m.score.teamA` (`livescore.js:64`).
///
/// objek → `current` / `normalTime` / `score` / `'-'`; selain itu nilai itu
/// sendiri, atau `'-'` kalau null/undefined.
fn score_side(m: &Value, side: &str) -> String {
    match m.get("score").and_then(|s| s.get(side)) {
        Some(Value::Object(o)) => ["current", "normalTime", "score"]
            .iter()
            .find_map(|k| match o.get(*k) {
                Some(Value::Null) | None => None,
                Some(v) => Some(interp(Some(v))),
            })
            .unwrap_or_else(|| "-".to_string()),
        Some(Value::Null) | None => "-".to_string(),
        Some(v) => interp(Some(v)),
    }
}

/// `m.redCards?.teamA ?? 0`, dipakai hanya lewat `> 0`.
fn red_cards(m: &Value, side: &str) -> i64 {
    m.get("redCards")
        .and_then(|r| r.get(side))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

/// `m.period` (`livescore.js:66`): objek → `name`/`short`/`long`/`'-'`,
/// selain itu `m.period || ''`.
pub(crate) fn period(m: &Value) -> String {
    match m.get("period") {
        Some(Value::Object(o)) => ["name", "short", "long"]
            .iter()
            .find_map(|k| match o.get(*k) {
                Some(Value::Null) | None => None,
                Some(v) => Some(interp(Some(v))),
            })
            .unwrap_or_else(|| "-".to_string()),
        Some(v) if js_truthy_value(v) => interp(Some(v)),
        _ => String::new(),
    }
}

/// Ambil `props.pageProps.content.liveScores` dari HTML `goal.com`.
///
/// Pesan `Err` mengikuti `throw` di `ambilLivescore` dan dipakai verbatim di
/// teks `catch`.
pub(crate) fn extract(html: &str) -> Result<Value, &'static str> {
    let start = html.find(MARKER).ok_or("Gagal menemukan data skor")? + MARKER.len();
    let rest = &html[start..];
    let end = rest.find("</script>").ok_or("Gagal menemukan data skor")?;
    let parsed: Value =
        serde_json::from_str(rest[..end].trim()).map_err(|_| "Gagal menemukan data skor")?;
    parsed
        .get("props")
        .and_then(|p| p.get("pageProps"))
        .and_then(|p| p.get("content"))
        .and_then(|c| c.get("liveScores"))
        .filter(|v| js_truthy_value(v))
        .cloned()
        .ok_or("Data liveScores kosong")
}

/// Satu blok kompetisi, sudah diterjemahkan ke baris caption yang dipakai
/// `handler`.
fn render_competition(group: &Value) -> (String, Vec<String>) {
    let name = group
        .get("competition")
        .filter(|c| js_truthy_value(c))
        .and_then(|c| c.get("name"))
        .filter(|n| js_truthy_value(n))
        .map_or_else(|| "-".to_string(), |n| interp(Some(n)));
    let matches = group.get("matches").and_then(Value::as_array);

    let mut lines = Vec::new();
    for m in matches.map(Vec::as_slice).unwrap_or(&[]) {
        let team_a = m.get("teamA").and_then(|t| t.get("name"));
        let team_b = m.get("teamB").and_then(|t| t.get("name"));
        let red_a = red_cards(m, "teamA");
        let red_b = red_cards(m, "teamB");
        let red_a = if red_a > 0 {
            format!(" 🟥{red_a}")
        } else {
            String::new()
        };
        let red_b = if red_b > 0 {
            format!(" 🟥{red_b}")
        } else {
            String::new()
        };
        let mut line = format!(
            "▪️ {}{red_a} *[ {} - {} ]* {}{red_b}\n",
            interp(team_a),
            score_side(m, "teamA"),
            score_side(m, "teamB"),
            interp(team_b),
        );
        let periode = period(m);
        line.push_str(&format!(
            "   └ ⏳ _Status: {}_",
            status_indo(m.get("status"))
        ));
        if !periode.is_empty() {
            line.push_str(&format!(" | ⏱️ _{periode}_"));
        }
        line.push('\n');
        lines.push(line);
    }
    (name, lines)
}

/// Caption untuk `liveScores` yang sudah lolos `extract`.
pub(crate) fn render(live_scores: &[Value]) -> String {
    let mut caption = String::from("⚽ *LIVE SCORE BOLA HARI INI* ⚽\n\n");
    let mut count = 0usize;
    for group in live_scores {
        if count >= 15 {
            break;
        }
        let (name, lines) = render_competition(group);
        if lines.is_empty() {
            continue;
        }
        caption.push_str(&format!("🏆 *{}*\n", name.to_uppercase()));
        for line in lines {
            caption.push_str(&line);
        }
        caption.push('\n');
        count += 1;
    }
    caption.push_str("_Sumber: Goal.com_");
    caption.trim().to_string()
}

#[plugin(
    name = "livescore",
    alias = ["skorbola", "livebola", "skor"],
    category = "info",
    description = "Menampilkan live score pertandingan sepak bola dari Goal.com",
    usage = ".livescore",
    example = ".livescore",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn livescore(ctx: Ctx) -> Reply {
    let res = match ctx.http.get(URL) {
        Ok(r) => r,
        Err(e) => return failed(&e.message),
    };
    let live_scores = match extract(&res.text()) {
        Ok(v) => v,
        Err(msg) => return failed(msg),
    };
    let Some(list) = live_scores.as_array() else {
        return failed("Data liveScores kosong");
    };
    if list.is_empty() {
        return Reply::text(NO_MATCH).with_reaction("❌");
    }
    Reply::text(render(list)).with_reaction("✅")
}

fn failed(message: &str) -> Reply {
    Reply::text(format!(
        "❌ *GAGAL MENGAMBIL SKOR*\n\n\
         Maaf, sistem gagal mengambil data live score saat ini. Error: _{message}_"
    ))
    .with_reaction("❌")
}

#[cfg(test)]
mod tests {
    use super::*;


    fn run() -> Reply {
        let ctx = Ctx {
            command: "livescore".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("livescore").unwrap().handler)(ctx)
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

    fn group(name: &str, matches: Value) -> Value {
        serde_json::json!({"competition": {"name": name}, "matches": matches})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("livescore").unwrap();
        assert_eq!(p.meta.name, "livescore");
        assert_eq!(p.meta.alias, ["skorbola", "livebola", "skor"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(
            p.meta.description,
            "Menampilkan live score pertandingan sepak bola dari Goal.com"
        );
        assert_eq!(p.meta.usage, ".livescore");
        assert_eq!(p.meta.example, ".livescore");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["skorbola", "livebola", "skor"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "livescore");
        }
    }

    #[test]
    fn a_dead_http_client_reports_the_error_branch() {
        let r = run();
        assert!(
            text_of(&r).starts_with("❌ *GAGAL MENGAMBIL SKOR*"),
            "{}",
            text_of(&r)
        );
        assert!(
            text_of(&r).ends_with("Error: _HTTP tidak tersedia di konteks ini_"),
            "{}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r), "❌");
    }

    #[test]
    fn an_html_page_without_the_next_data_block_is_an_error() {
        assert_eq!(
            extract("<html><body></body></html>"),
            Err("Gagal menemukan data skor")
        );
        assert_eq!(
            extract(&format!("{MARKER}{{\"a\":1}}")),
            Err("Gagal menemukan data skor"),
            "tanpa `</script>` penutup"
        );
    }

    #[test]
    fn malformed_json_inside_the_block_is_an_error() {
        assert_eq!(
            extract(&format!("{MARKER}{{bukan json}}</script>")),
            Err("Gagal menemukan data skor")
        );
    }

    #[test]
    fn a_page_without_live_scores_is_an_error() {
        let html = r#"{"props":{"pageProps":{"content":{}}}}"#;
        assert_eq!(
            extract(&format!("{MARKER}{html}</script>")),
            Err("Data liveScores kosong")
        );
    }

    #[test]
    fn the_block_is_cut_at_the_first_closing_script_tag() {
        let html = r#"{"props":{"pageProps":{"content":{"liveScores":[{"competition":{"name":"X"},"matches":[]}]}}}}"#;
        let value = extract(&format!("{MARKER}{html}</script><script>lain</script>")).unwrap();
        assert_eq!(value[0]["competition"]["name"], serde_json::json!("X"));
    }

    #[test]
    fn the_status_map_covers_every_js_key_and_passes_unknowns_through() {
        for (wire, indo) in [
            ("FIXTURE", "Belum Mulai"),
            ("LIVE", "Berlangsung"),
            ("FINISHED", "Selesai"),
            ("POSTPONED", "Ditunda"),
            ("CANCELLED", "Dibatalkan"),
            ("HALF_TIME", "Turun Minum"),
            ("MENUNGGU", "MENUNGGU"),
        ] {
            assert_eq!(status_indo(Some(&serde_json::json!(wire))), indo, "{wire}");
        }
        assert_eq!(
            status_indo(None),
            "undefined",
            "sama seperti ${{undefined}}"
        );
        assert_eq!(status_indo(Some(&Value::Null)), "null");
    }

    #[test]
    fn an_object_score_walks_the_chain_current_normal_time_score() {
        let m = serde_json::json!({"score": {"teamA": {"normalTime": "1-0"}, "teamB": {"score": 2}}});
        assert_eq!(score_side(&m, "teamA"), "1-0");
        assert_eq!(score_side(&m, "teamB"), "2");
        let m = serde_json::json!({"score": {"teamA": {"current": 3, "normalTime": "1-0", "score": 9}}});
        assert_eq!(score_side(&m, "teamA"), "3", "yang pertama menang");
    }

    #[test]
    fn a_scalar_or_missing_score_falls_back_to_itself_then_to_a_dash() {
        assert_eq!(score_side(&serde_json::json!({"score": {"teamA": 0}}), "teamA"), "0");
        assert_eq!(
            score_side(&serde_json::json!({"score": {"teamA": "menang"}}), "teamA"),
            "menang"
        );
        assert_eq!(score_side(&serde_json::json!({"score": {"teamA": null}}), "teamA"), "-");
        assert_eq!(score_side(&serde_json::json!({"score": {}}), "teamA"), "-");
        assert_eq!(score_side(&serde_json::json!({}), "teamA"), "-");
        let empty: Value = serde_json::json!({"score": {"teamA": {}}});
        assert_eq!(
            score_side(&empty, "teamA"),
            "-",
            "objek tanpa satu pun kunci"
        );
    }

    #[test]
    fn the_period_prefers_name_then_short_then_long() {
        let m = serde_json::json!({"period": {"name": "Babak Kedua", "short": "B2", "long": "Second Half"}});
        assert_eq!(period(&m), "Babak Kedua");
        let m = serde_json::json!({"period": {"short": "B2"}});
        assert_eq!(period(&m), "B2");
        let m = serde_json::json!({"period": "HT"});
        assert_eq!(period(&m), "HT");
        assert_eq!(period(&serde_json::json!({"period": ""})), "", "falsy → string kosong");
        assert_eq!(period(&serde_json::json!({})), "");
        assert_eq!(period(&serde_json::json!({"period": {}})), "-");
    }

    #[test]
    fn a_match_renders_status_period_and_red_cards() {
        let g = group(
            "Liga Inggris",
            serde_json::json!([{
                "status": "LIVE",
                "period": {"name": "Babak Kedua"},
                "teamA": {"name": "Arsenal"},
                "teamB": {"name": "Chelsea"},
                "score": {"teamA": {"current": 2}, "teamB": {"current": 1}},
                "redCards": {"teamA": 1, "teamB": 0},
            }]),
        );
        let out = render(&[g]);
        assert!(out.starts_with("⚽ *LIVE SCORE BOLA HARI INI* ⚽"), "{out}");
        assert!(out.contains("🏆 *LIGA INGGRIS*"), "{out}");
        assert!(
            out.contains("▪️ Arsenal 🟥1 *[ 2 - 1 ]* Chelsea\n   └ ⏳ _Status: Berlangsung_ | ⏱️ _Babak Kedua_"),
            "{out}"
        );
        assert!(out.ends_with("_Sumber: Goal.com_"), "{out}");
    }

    #[test]
    fn a_match_without_a_period_drops_the_period_segment() {
        let g = group(
            "Liga Italia",
            serde_json::json!([{"status": "FINISHED", "teamA": {"name": "A"}, "teamB": {"name": "B"}}]),
        );
        let out = render(&[g]);
        assert!(
            out.contains("   └ ⏳ _Status: Selesai_\n"),
            "tanpa `| ⏱️`: {out:?}"
        );
    }

    #[test]
    fn a_competition_without_matches_is_skipped_entirely() {
        let list = vec![
            group("Kosong", serde_json::json!([])),
            group("LigaItalia", serde_json::json!([])),
            group(
                "Isi",
                serde_json::json!([{"status": "LIVE", "teamA": {"name": "A"}, "teamB": {"name": "B"}}]),
            ),
        ];
        let out = render(&list);
        assert!(
            !out.contains("KOSONG"),
            "kompetisi kosong tidak boleh muncul: {out}"
        );
        assert!(!out.contains("LIGA ITALIA"), "{out}");
        assert!(out.contains("🏆 *ISI*"), "{out}");
    }

    #[test]
    fn at_most_fifteen_competitions_with_matches_are_rendered() {
        let list: Vec<Value> = (0..20)
            .map(|i| {
                group(
                    &format!("Liga{i}"),
                    serde_json::json!([{"status": "LIVE", "teamA": {"name": "A"}, "teamB": {"name": "B"}}]),
                )
            })
            .collect();
        let out = render(&list);
        assert_eq!(out.matches("🏆 *").count(), 15, "{out}");
        assert!(out.contains("🏆 *LIGA14*"), "indeks ke-15: {out}");
        assert!(!out.contains("LIGA15"), "indeks ke-16 terpotong: {out}");
    }

    #[test]
    fn a_competition_without_a_name_uses_the_dash_fallback() {
        let g = serde_json::json!({"matches": [{"status": "LIVE", "teamA": {"name": "A"}, "teamB": {"name": "B"}}]});
        let teks = render(&[g]);
        assert!(teks.contains("🏆 *-*"), "{teks}");
    }
}
