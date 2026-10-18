//! `plugins/clan/clanleaderboard.js` — ranking clan berdasarkan skor
//! `wins*100 + exp + level*500`, Turun 10 teratas.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::clan::clancreate::{clans, emblem, field_num, field_str, members};

/// `getRankTitle(level)` — di `clanleaderboard.js` emblem saja, tanpa nama tier
/// seperti di `claninfo.js`. Dua fungsi itu berbeda dan tidak boleh disatukan.
pub(crate) fn rank_emoji(level: i64) -> &'static str {
    match level {
        l if l >= 50 => "👑",
        l if l >= 30 => "💎",
        l if l >= 20 => "🏆",
        l if l >= 10 => "🥇",
        l if l >= 5 => "🥈",
        _ => "🥉",
    }
}

/// `(wins * 100) + exp + (level * 500)` — `|| 1` untuk level, jadi clan tanpa
/// level dihitung sebagai level 1.
pub(crate) fn score(clan: &Value) -> f64 {
    let level = field_num(clan, "level");
    let level = if level == 0.0 { 1.0 } else { level };
    field_num(clan, "wins") * 100.0 + field_num(clan, "exp") + level * 500.0
}

fn level_of(clan: &Value) -> i64 {
    let l = field_num(clan, "level") as i64;
    if l == 0 { 1 } else { l }
}

/// `((wins / total) * 100).toFixed(0)` — TANPA desimal, berbeda dari
/// `claninfo` yang `toFixed(1)`.
fn win_rate0(wins: i64, losses: i64) -> String {
    let total = wins + losses;
    if total > 0 {
        format!("{}", (wins as f64 / total as f64 * 100.0).round() as i64)
    } else {
        "—".to_string()
    }
}

/// `medals[i] || `${i + 1}.`` — tiga medali, sisanya nomor.
fn medal(index: usize) -> String {
    match index {
        0 => "🥇".to_string(),
        1 => "🥈".to_string(),
        2 => "🥉".to_string(),
        i => format!("{}.", i + 1),
    }
}

/// Urutan `Array.prototype.sort` JS: V8 stabilize, jadi comparator yang
/// mengembalikan 0 tidak mengubah urutan asli. `slice::sort_by` di Rust tidak
/// dijamin stable, jadi indeks asli ikut disimpan dan dipakai sebagai
/// tie-break — urutan clan dengan skor kembar jadi sama dengan JS.
pub(crate) fn ranked<'a>(all: &[(&'a String, &'a Value)]) -> Vec<(&'a String, &'a Value)> {
    let mut idx: Vec<usize> = (0..all.len()).collect();
    idx.sort_by(|a, b| {
        let sa = score(all[*a].1);
        let sb = score(all[*b].1);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    idx.into_iter().map(|i| all[i]).collect()
}

pub(crate) fn empty() -> &'static str {
    "🏰 Belum ada clan terdaftar\n\nBuat: *.clancreate <nama>*"
}

pub(crate) fn board(rows: &[String], total: usize) -> String {
    let mut txt = String::from("🏰 *CLAN LEADERBOARD*\n\n");
    for line in rows {
        txt.push_str(line);
    }
    txt.push_str(&format!("Total *{total}* clan terdaftar"));
    txt
}

pub(crate) fn row(index: usize, clan: &Value) -> String {
    let wins = field_num(clan, "wins") as i64;
    let losses = field_num(clan, "losses") as i64;
    let level = level_of(clan);
    format!(
        "{} {} *{}*\n   {} Lv.{} · {}W/{}L ({}%) · 👥 {}\n\n",
        medal(index),
        emblem(clan),
        field_str(clan, "name"),
        rank_emoji(level),
        level,
        wins,
        losses,
        win_rate0(wins, losses),
        members(clan).len(),
    )
}

#[plugin(
    name = "clanleaderboard",
    alias = ["clanlb", "topclan", "guildrank"],
    category = "clan",
    description = "Lihat ranking clan",
    usage = ".clanleaderboard",
    example = ".clanleaderboard",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clanleaderboard(ctx: Ctx) -> Reply {
    let all = clans(&ctx);
    if all.is_empty() {
        return Reply::text(empty());
    }

    let entries: Vec<(&String, &Value)> = all.iter().collect();
    let sorted = ranked(&entries);
    let rows: Vec<String> = sorted
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (_, c))| row(i, c))
        .collect();

    Reply::text(board(&rows, entries.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(clans: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clanleaderboard".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            args: Vec::new(),
            settings: serde_json::json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("clanleaderboard").unwrap().handler)(ctx)
    }

    fn clan(name: &str, wins: i64, losses: i64, exp: i64, level: i64, count: usize) -> Value {
        serde_json::json!({
            "id": name,
            "name": name,
            "emblem": "🐺",
            "wins": wins,
            "losses": losses,
            "exp": exp,
            "level": level,
            "members": vec!["628111@s.whatsapp.net"; count],
        })
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clanleaderboard").unwrap();
        assert_eq!(p.meta.name, "clanleaderboard");
        assert_eq!(p.meta.alias, ["clanlb", "topclan", "guildrank"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Lihat ranking clan");
        assert_eq!(p.meta.usage, ".clanleaderboard");
        assert_eq!(p.meta.example, ".clanleaderboard");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["clanlb", "topclan", "guildrank"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "clanleaderboard");
        }
    }

    #[test]
    fn an_empty_registry_shows_the_creation_hint_and_writes_nothing() {
        let r = run(serde_json::json!({}));
        assert_eq!(text_of(&r), empty());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_score_ordering_and_every_tier_boundary_hold() {
        // wins=1 (100) vs exp=99 → wins menang meski level jauh lebih tinggi.
        let a = clan("A", 1, 0, 99, 1, 1);
        let b = clan("B", 0, 0, 100, 1, 1);
        assert!(score(&a) > score(&b));
        // level menambah 500 per tingkat.
        let hi = clan("A", 0, 0, 0, 2, 1);
        let lo = clan("B", 0, 0, 400, 1, 1);
        assert!(score(&hi) > score(&lo), "level 2 = 1000 vs exp 400");
    }

    #[test]
    fn a_missing_level_counts_as_one() {
        let c = serde_json::json!({});
        assert_eq!(score(&c), 500.0);
        assert_eq!(level_of(&c), 1);
    }

    #[test]
    fn the_board_lists_at_most_ten_and_reports_the_true_total() {
        let mut all = serde_json::Map::new();
        for i in 0..12 {
            all.insert(
                format!("C{i:02}"),
                clan(&format!("C{i:02}"), i, 0, 0, 1, i as usize + 1),
            );
        }
        let r = run(Value::Object(all));
        let t = text_of(&r);
        assert!(t.starts_with("🏰 *CLAN LEADERBOARD*\n\n"), "{t}");
        assert!(t.ends_with("Total *12* clan terdaftar"), "{t}");
        assert_eq!(t.matches("· 👥").count(), 10, "hanya 10 baris: {t}");
        // Skor = wins*100 + 500, jadi C11 (1600) sampai C02 (700) masuk;
        // C01 (600) dan C00 (500) terpotong.
        assert!(t.contains("*C11*"), "{t}");
        assert!(t.contains("*C02*"), "{t}");
        assert!(!t.contains("*C01*"), "C01 di luar 10 teratas: {t}");
        assert!(!t.contains("*C00*"), "C00 di luar 10 teratas: {t}");
    }

    #[test]
    fn medals_cover_the_first_three_then_fall_back_to_numbers() {
        for (i, expected) in [(0, "🥇"), (1, "🥈"), (2, "🥉"), (3, "4."), (9, "10.")] {
            assert_eq!(medal(i), expected, "baris {i}");
        }
    }

    #[test]
    fn win_rate_rounds_to_zero_decimals_and_dashes_when_empty() {
        let line = row(0, &clan("A", 7, 3, 0, 5, 2));
        assert!(line.contains("🥈 Lv.5 · 7W/3L (70%) · 👥 2"), "{line}");

        // `totalGames === 0` → `—`, bukan `NaN%`.
        let line = row(3, &clan("B", 0, 0, 0, 1, 1));
        assert!(line.contains("(—%)"), "tanpa war: {line}");
        assert!(line.starts_with("4. "), "medali habis → nomor: {line}");
    }

    #[test]
    fn rank_emojis_use_inclusive_thresholds() {
        assert_eq!(rank_emoji(50), "👑");
        assert_eq!(rank_emoji(49), "💎");
        assert_eq!(rank_emoji(30), "💎");
        assert_eq!(rank_emoji(29), "🏆");
        assert_eq!(rank_emoji(20), "🏆");
        assert_eq!(rank_emoji(19), "🥇");
        assert_eq!(rank_emoji(10), "🥇");
        assert_eq!(rank_emoji(9), "🥈");
        assert_eq!(rank_emoji(5), "🥈");
        assert_eq!(rank_emoji(4), "🥉");
    }
}
