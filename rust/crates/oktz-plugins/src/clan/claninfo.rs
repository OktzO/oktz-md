//! `plugins/clan/claninfo.js` — tampilkan detail satu clan (level, exp bar, war stats).

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::clan::clancreate::{clans, emblem, field_i64, field_str_or, find_clan, members};
use crate::common::jid_number;

/// `expBar(exp, nextLevel)` — target `level * 10000`, isi dibulatkan ke 0..=10
/// sel dari 10.
///
/// `progress` TIDAK dijaga untuk `target == 0`: di JS `exp / 0` jadi `Infinity`
/// (atau `NaN` kalau `exp` juga 0) lalu `Math.min(…, 1)` tetap menghasilkan 1,
/// dan `f64::min` Rust mengembalikan operand kedua saat satuannya NaN — jadi
/// kedua kasus jatuh ke bar penuh, sama seperti JS.
pub(crate) fn exp_bar(exp: f64, next_level: i64) -> String {
    let progress = (exp / (next_level as f64 * 10_000.0)).min(1.0);
    let filled = (progress * 10.0).round().clamp(0.0, 10.0) as usize;
    format!(
        "{}{} {}%",
        "█".repeat(filled),
        "░".repeat(10 - filled),
        (progress * 100.0).round() as i64
    )
}

/// `getRankTitle(level)` — ambang inklusif, urut dari tertinggi.
pub(crate) fn rank_title(level: i64) -> &'static str {
    match level {
        l if l >= 50 => "👑 Legendary",
        l if l >= 30 => "💎 Diamond",
        l if l >= 20 => "🏆 Platinum",
        l if l >= 10 => "🥇 Gold",
        l if l >= 5 => "🥈 Silver",
        _ => "🥉 Bronze",
    }
}

/// `((wins / totalGames) * 100).toFixed(1)` — satu desimal, atau `—` kalau
/// belum pernah war. `toFixed` JS setengah ke atas; `f64::round`_set halfway
/// ke atas juga, jadi keduanya cocok.
pub(crate) fn win_rate(wins: i64, losses: i64) -> String {
    let total = wins + losses;
    if total > 0 {
        format!("{:.1}", wins as f64 / total as f64 * 100.0)
    } else {
        "—".to_string()
    }
}

pub(crate) fn no_clan() -> &'static str {
    "❌ Kamu belum punya clan\n\nBuat: *.clancreate <nama>*\nGabung: *.clanjoin <id>*"
}

pub(crate) fn not_found() -> &'static str {
    "❌ Clan tidak ditemukan"
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn detail(
    clan: &serde_json::Value,
    rank: &str,
    level: i64,
    bar: &str,
    wins: i64,
    losses: i64,
    wr: &str,
    created: &str,
) -> String {
    let leader = field_str_or(clan, "leader", "");
    let description = field_str_or(clan, "description", "Belum ada deskripsi");
    let is_open = crate::common::js_truthy(clan.get("isOpen"));
    let id = field_str_or(clan, "id", "");
    format!(
        "{emblem} *{name}*\n\
         {rank} · Level {level}\n\n\
         EXP  {bar}\n\n\
         ┌ 👑 Leader · @{leader_num}\n\
         ├ 👥 Members · {count}/{max}\n\
         ├ 🔓 Status · {status}\n\
         └ 📅 Dibuat · {created}\n\n\
         ⚔️ *War Stats*\n\
         {wins}W · {losses}L · {wr}% WR\n\n\
         _{description}_\n\n\
         ID: `{id}`",
        emblem = emblem(clan),
        name = field_str_or(clan, "name", ""),
        leader_num = jid_number(&leader),
        count = members(clan).len(),
        max = crate::clan::clancreate::MAX_MEMBERS,
        status = if is_open { "Open" } else { "Closed" },
    )
}

#[plugin(
    name = "claninfo",
    alias = ["infoclan", "myclan", "guildinfo"],
    category = "clan",
    description = "Lihat info clan",
    usage = ".claninfo [clan_id]",
    example = ".claninfo",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn claninfo(ctx: Ctx) -> Reply {
    let arg = ctx.text.trim().to_string();
    let own_clan = ctx
        .user_field("clanId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let clan_id = if arg.is_empty() { own_clan } else { arg };
    if clan_id.is_empty() {
        return Reply::text(no_clan());
    }

    let all = clans(&ctx);
    let Some(clan) = find_clan(&all, &clan_id) else {
        return Reply::text(not_found());
    };

    let wins = field_i64(clan, "wins");
    let losses = field_i64(clan, "losses");
    let level = {
        let l = field_i64(clan, "level");
        if l == 0 { 1 } else { l }
    };
    let wr = win_rate(wins, losses);
    let bar = exp_bar(field_i64(clan, "exp") as f64, level);
    let created = locale_date_id(parse_created(&field_str_or(clan, "createdAt", "")));

    let text = detail(
        clan,
        rank_title(level),
        level,
        &bar,
        wins,
        losses,
        &wr,
        &created,
    );
    let leader = field_str_or(clan, "leader", "");
    Reply::Text {
        text,
        mentions: if leader.is_empty() {
            Vec::new()
        } else {
            vec![leader]
        },
    }
}

/// `new Date(clan.createdAt).toLocaleDateString('id-ID')` — format default
/// `id-ID` adalah `d/M/yyyy` TANPA padding nol (`2/1/2026`, bukan
/// `02/01/2026`), jadi `date_dmy` dari `common` tidak bisa dipakai di sini.
fn locale_date_id(ms: i64) -> String {
    let (y, m, d, ..) = crate::common::jakarta_parts(ms);
    format!("{d}/{m}/{y}")
}

/// `new Date(clan.createdAt)` → epoch ms. String ISO milik JS
/// (`2026-01-02T03:04:05.000Z`) dibaca lewat offset Jakarta supaya angkanya sama
/// dengan `Date.parse` pada waktu yang sama.
fn parse_created(raw: &str) -> i64 {
    let b = raw.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return 0;
    }
    let num = |r: std::ops::Range<usize>| -> i64 {
        std::str::from_utf8(&b[r]).unwrap_or("0").parse().unwrap_or(0)
    };
    let (y, mo, d) = (num(0..4), num(5..7), num(8..10));
    let (h, mi, s) = (num(11..13), num(14..16), num(17..19));
    crate::common::days_from_civil(y, mo as u32, d as u32) * 86_400_000
        + (h * 3_600 + mi * 60 + s) * 1000
        - crate::common::JAKARTA_OFFSET_SECS * 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value, text: &str, clans: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "claninfo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            settings: json!({ crate::clan::clancreate::CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("claninfo").unwrap().handler)(ctx)
    }

    fn clan(extra: Value) -> Value {
        let mut c = json!({
            "id": "DRG",
            "name": "Dragon",
            "emblem": "🐉",
            "leader": "628999@s.whatsapp.net",
            "members": ["628999@s.whatsapp.net", "628888@s.whatsapp.net"],
            "exp": 0,
            "level": 1,
            "wins": 0,
            "losses": 0,
            "createdAt": "2026-01-02T00:00:00.000Z",
            "description": "Belum ada deskripsi",
            "isOpen": true,
        });
        if let (Some(a), Some(b)) = (c.as_object_mut(), extra.as_object()) {
            for (k, v) in b {
                a.insert(k.clone(), v.clone());
            }
        }
        Value::Object(c.as_object().unwrap().clone())
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("claninfo").unwrap();
        assert_eq!(p.meta.name, "claninfo");
        assert_eq!(p.meta.alias, ["infoclan", "myclan", "guildinfo"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Lihat info clan");
        assert_eq!(p.meta.usage, ".claninfo [clan_id]");
        assert_eq!(p.meta.example, ".claninfo");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["infoclan", "myclan", "guildinfo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "claninfo");
        }
    }

    #[test]
    fn without_an_id_or_a_clan_id_the_user_is_told_how_to_get_one() {
        let r = run(json!({}), "", json!({}));
        assert_eq!(text_of(&r), no_clan());
        assert!(r.mutations().is_empty(), "baca tidak boleh menulis");
    }

    #[test]
    fn an_unknown_id_is_reported_as_not_found() {
        let r = run(json!({}), "Zzz", json!({}));
        assert_eq!(text_of(&r), not_found());
    }

    #[test]
    fn the_lookup_falls_back_from_id_to_name() {
        for arg in ["DRG", "Dragon", "dragon"] {
            let r = run(json!({}), arg, json!({ "DRG": clan(json!({})) }));
            assert!(text_of(&r).contains("Dragon"), "arg {arg}: {}", text_of(&r));
        }
    }

    #[test]
    fn a_clan_id_argument_overrides_the_senders_own_clan() {
        let r = run(
            json!({ "clanId": "DRG" }),
            "Naga",
            json!({ "DRG": clan(json!({})), "NG": clan(json!({"id":"NG","name":"Naga"})) }),
        );
        assert!(text_of(&r).contains("Naga"), "{}", text_of(&r));
    }

    #[test]
    fn a_full_clan_renders_leader_members_status_and_created_date() {
        let r = run(json!({}), "DRG", json!({ "DRG": clan(json!({})) }));
        let t = text_of(&r);
        assert!(t.starts_with("🐉 *Dragon*"), "{t}");
        assert!(t.contains("🥉 Bronze · Level 1"), "{t}");
        assert!(t.contains("EXP  ░░░░░░░░░░ 0%"), "{t}");
        assert!(t.contains("👑 Leader · @628999"), "{t}");
        assert!(t.contains("👥 Members · 2/50"), "{t}");
        assert!(t.contains("🔓 Status · Open"), "{t}");
        assert!(t.contains("└ 📅 Dibuat · 2/1/2026"), "{t}");
        assert!(t.contains("0W · 0L · —% WR"), "{t}");
        assert!(t.contains("_Belum ada deskripsi_"), "{t}");
        assert!(t.ends_with("ID: `DRG`"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_closed_clan_with_a_description_shows_both() {
        let r = run(
            json!({}),
            "DRG",
            json!({ "DRG": clan(json!({"isOpen": false, "description": "Elite", "emblem": null})) }),
        );
        let t = text_of(&r);
        assert!(t.starts_with("🏰 *Dragon*"), "emblem null → default: {t}");
        assert!(t.contains("🔓 Status · Closed"), "{t}");
        assert!(t.contains("_Elite_"), "{t}");
    }

    #[test]
    fn war_stats_and_exp_bar_scale_with_progress() {
        // Target = level * 10000, jadi level 2 dengan 10.000 EXP = 50%.
        let r = run(
            json!({}),
            "DRG",
            json!({ "DRG": clan(json!({"wins": 7, "losses": 3, "level": 2, "exp": 10_000})) }),
        );
        let t = text_of(&r);
        assert!(t.contains("🥉 Bronze · Level 2"), "level 2 belum Silver: {t}");
        assert!(t.contains("EXP  █████░░░░░ 50%"), "{t}");
        assert!(t.contains("7W · 3L · 70.0% WR"), "{t}");

        // Level 30 → Diamond, dan 300.000 EXP = tepat satu target.
        let dia = run(
            json!({}),
            "DRG",
            json!({ "DRG": clan(json!({"level": 30, "exp": 300_000})) }),
        );
        let t = text_of(&dia);
        assert!(t.contains("💎 Diamond · Level 30"), "{t}");
        assert!(t.contains("EXP  ██████████ 100%"), "{t}");

        // Level 1 dengan 10.000 EXP = bar penuh.
        let full = run(
            json!({}),
            "DRG",
            json!({ "DRG": clan(json!({"level": 1, "exp": 10_000})) }),
        );
        assert!(text_of(&full).contains("EXP  ██████████ 100%"), "{}", text_of(&full));
    }

    #[test]
    fn exp_beyond_the_target_clamps_at_a_full_bar() {
        assert!(exp_bar(99_999.0, 1).starts_with("██████████"), "100%");
        // Level 0 → target 0, JS `exp / 0` = Infinity lalu `Math.min(…, 1)` = 1.
        assert_eq!(exp_bar(0.0, 0), "██████████ 100%");
        assert_eq!(exp_bar(0.0, 1), "░░░░░░░░░░ 0%");
    }

    #[test]
    fn rank_titles_use_inclusive_thresholds() {
        for (level, expected) in [
            (50, "👑 Legendary"),
            (49, "💎 Diamond"),
            (30, "💎 Diamond"),
            (29, "🏆 Platinum"),
            (20, "🏆 Platinum"),
            (19, "🥇 Gold"),
            (10, "🥇 Gold"),
            (9, "🥈 Silver"),
            (5, "🥈 Silver"),
            (4, "🥉 Bronze"),
            (1, "🥉 Bronze"),
        ] {
            assert_eq!(rank_title(level), expected, "level {level}");
        }
    }

    #[test]
    fn win_rate_shows_a_dash_when_there_is_no_war_yet() {
        assert_eq!(win_rate(0, 0), "—");
        assert_eq!(win_rate(1, 0), "100.0");
        assert_eq!(win_rate(1, 2), "33.3");
    }

    #[test]
    fn a_missing_level_reads_as_one_and_a_missing_emblem_as_the_castle() {
        let c = clan(json!({ "level": null, "exp": null }));
        let level = {
            let l = field_i64(&c, "level");
            if l == 0 { 1 } else { l }
        };
        assert_eq!(level, 1);
        assert_eq!(emblem(&c), "🐉");
        assert_eq!(emblem(&json!({})), "🏰");
    }
}
