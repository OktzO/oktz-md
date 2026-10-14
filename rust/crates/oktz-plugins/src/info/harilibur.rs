//! `plugins/info/harilibur.js` — hari libur & nasional mendatang.
//!
//! `ponytail:` reaksi hilang sebagian. JS bereaksi `🕕` di awal lalu `✅`,
//! `❌`, atau `☢`; `Reply::Reaction` hanya bisa membawa satu emoji, jadi
//! hanya reaksi terakhir yang ikut.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_falsy;

pub(crate) const URL: &str = "https://api.nexray.eu.cc/information/hari-libur";

/// `data.status`/`data.result` kosong → cabang `❌` (`harilibur.js:26-29`).
pub(crate) const NO_DATA: &str = "⚠️ Gagal mengambil informasi hari libur saat ini.";

/// Cabang `catch` (`harilibur.js:58-61`).
pub(crate) const FAILED: &str = "😔 Terjadi kesalahan saat mengambil data hari libur.";

/// `${value}` di JS — `undefined` untuk field yang hilang.
fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Satu blok daftar (`harilibur.js:36-49`): judul + maksimal lima baris
/// `- {date}: {event} ({daysUntil} hari lagi)`. `cap: true` menambah baris
/// kosong penutup yang hanya boleh ada setelah blok hari libur.
fn block(items: Option<&Value>, title: &str, cap: bool) -> String {
    let Some(items) = items.and_then(Value::as_array).filter(|a| !a.is_empty()) else {
        return String::new();
    };
    let mut txt = format!("*{title}*\n");
    for item in items.iter().take(5) {
        txt.push_str(&format!(
            "- {}: {} ({} hari lagi)\n",
            interp(item.get("date")),
            interp(item.get("event")),
            interp(item.get("daysUntil"))
        ));
    }
    if cap {
        txt.push('\n');
    }
    txt
}

/// Caption untuk `result` yang sudah lolos cek `status`.
///
/// `Err` kalau `result.mendatang` hilang: di JS itu `TypeError` yang ditangkap
/// `catch`, jadi bukan string kosong.
pub(crate) fn render(result: &Value) -> Result<String, String> {
    // `r.mendatang.hari_libur` melempar TypeError kalau `mendatang` null atau
    // tidak ada — keduanya ditangkap `catch` di JS.
    let mendatang = match result.get("mendatang") {
        Some(Value::Null) | None => return Err(FAILED.to_string()),
        Some(v) => v,
    };
    let mut caption = String::from("📅 *HARI LIBUR & NASIONAL MENDATANG* 📅\n\n");
    caption.push_str(&block(
        mendatang.get("hari_libur"),
        "Hari Libur Mendatang",
        true,
    ));
    caption.push_str(&block(
        mendatang.get("event_nasional"),
        "Hari Nasional Mendatang",
        false,
    ));
    Ok(caption.trim().to_string())
}

#[plugin(
    name = "harilibur",
    alias = ["libur", "harinasional"],
    category = "info",
    description = "Menampilkan informasi hari libur dan hari nasional mendatang",
    usage = ".harilibur",
    example = ".harilibur",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn harilibur(ctx: Ctx) -> Reply {
    let res = match ctx.http.get(URL) {
        Ok(r) => r,
        Err(_) => return Reply::text(FAILED).with_reaction("☢"),
    };
    let data = res.json().unwrap_or(Value::Null);
    if js_falsy(data.get("status")) || js_falsy(data.get("result")) {
        return Reply::text(NO_DATA).with_reaction("❌");
    }
    let result = data.get("result").expect("sudah dijaga js_falsy");
    match render(result) {
        Ok(text) => Reply::text(text).with_reaction("✅"),
        Err(_) => Reply::text(FAILED).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run() -> Reply {
        let ctx = Ctx {
            command: "harilibur".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("harilibur").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("harilibur").unwrap();
        assert_eq!(p.meta.name, "harilibur");
        assert_eq!(p.meta.alias, ["libur", "harinasional"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(
            p.meta.description,
            "Menampilkan informasi hari libur dan hari nasional mendatang"
        );
        assert_eq!(p.meta.usage, ".harilibur");
        assert_eq!(p.meta.example, ".harilibur");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["libur", "harinasional"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "harilibur");
        }
    }

    #[test]
    fn a_dead_http_client_lands_on_the_catch_branch() {
        let r = run();
        assert_eq!(text_of(&r), FAILED);
        assert_eq!(reaction_of(&r), "☢");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn both_lists_render_with_their_own_titles() {
        let result = json!({"mendatang": {
            "hari_libur": [{"date": "2026-01-01", "event": "Tahun Baru", "daysUntil": 3}],
            "event_nasional": [{"date": "2026-06-01", "event": "Pilkada", "daysUntil": 200}],
        }});
        assert_eq!(
            render(&result).unwrap(),
            "📅 *HARI LIBUR & NASIONAL MENDATANG* 📅\n\n\
             *Hari Libur Mendatang*\n\
             - 2026-01-01: Tahun Baru (3 hari lagi)\n\n\
             *Hari Nasional Mendatang*\n\
             - 2026-06-01: Pilkada (200 hari lagi)"
        );
    }

    #[test]
    fn each_list_is_capped_at_five_entries() {
        let items: Vec<_> = (0..8)
            .map(|i| json!({"date": format!("2026-01-0{}", i + 1), "event": format!("E{i}"), "daysUntil": i}))
            .collect();
        let result = json!({"mendatang": {"hari_libur": items.clone()}});
        let out = render(&result).unwrap();
        for i in 0..5 {
            assert!(out.contains(&format!("E{i}")), "E{i} hilang: {out}");
        }
        for i in 5..8 {
            assert!(
                !out.contains(&format!("E{i}")),
                "E{i} tidak boleh ikut: {out}"
            );
        }
    }

    #[test]
    fn an_empty_list_drops_its_title_and_its_trailing_blank_line() {
        let result = json!({"mendatang": {"hari_libur": [], "event_nasional": []}});
        assert_eq!(
            render(&result).unwrap(),
            "📅 *HARI LIBUR & NASIONAL MENDATANG* 📅"
        );
    }

    #[test]
    fn a_missing_mendatang_is_an_error_not_an_empty_caption() {
        // `r.mendatang.hari_libur` melempar TypeError di JS → cabang catch.
        assert!(render(&json!({})).is_err());
        assert_eq!(render(&json!({})).unwrap_err(), FAILED);
    }

    #[test]
    fn a_nasional_only_list_ends_without_a_trailing_blank_line() {
        // `caption += "\n"` hanya ada di blok hari libur, jadi blok nasional
        // sebagai blok terakhir tidak menambah baris kosong.
        let result = json!({"mendatang": {"event_nasional": [
            {"date": "2026-06-01", "event": "Pilkada", "daysUntil": 200}
        ]}});
        assert_eq!(
            render(&result).unwrap(),
            "📅 *HARI LIBUR & NASIONAL MENDATANG* 📅\n\n\
             *Hari Nasional Mendatang*\n\
             - 2026-06-01: Pilkada (200 hari lagi)"
        );
    }
}
