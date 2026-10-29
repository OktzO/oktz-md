//! `plugins/info/gag.js` — stok Grow a Garden dari `api.nexray.eu.cc`.
//!
//! `ponytail:` reaksi hilang sebagian. JS bereaksi `🕕` di awal lalu `✅`,
//! `❌`, atau `☢`; `Reply::Reaction` hanya bisa membawa satu emoji, jadi
//! hanya reaksi terakhir yang ikut. Reaksi `🕕` dibuang.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_falsy;

pub(crate) const URL: &str = "https://api.nexray.eu.cc/information/growagarden";

/// `data.status`/`data.result` kosong → cabang `❌` (`gag.js:26-29`).
pub(crate) const NO_DATA: &str = "⚠️ Gagal mengambil informasi Grow a Garden saat ini.";

/// Cabang `catch` (`gag.js:65-67`).
pub(crate) const FAILED: &str = "😔 Terjadi kesalahan saat mengambil data GAG.";

/// `${value}` di JS: field yang hilang jadi `"undefined"`, `null` jadi
/// `"null"`. `Value::to_string` sudah benar untuk angka dan string.
fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `formatStock(arr, title)` (`gag.js:35-40`): `"!arr || arr.length === 0"`
/// jadi string kosong, sehingga bagian kosong hilang total dari caption.
pub(crate) fn format_stock(arr: Option<&Value>, title: &str) -> String {
    let Some(items) = arr.and_then(Value::as_array).filter(|a| !a.is_empty()) else {
        return String::new();
    };
    let mut txt = format!("*{title}*\n");
    for item in items {
        txt.push_str(&format!(
            "- {}: {}\n",
            interp(item.get("name")),
            interp(item.get("value"))
        ));
    }
    txt.push('\n');
    txt
}

/// Caption penuh untuk objek `result` yang sudah lolos cek `status`.
pub(crate) fn render(result: &Value) -> String {
    let mut caption = String::from("🌱 *GROW A GARDEN INFO* 🌱\n\n");
    for (key, title) in [
        ("gearStock", "⚙️ Gear Stock"),
        ("eggStock", "🥚 Egg Stock"),
        ("eventStock", "🎟️ Event Stock"),
        ("cosmeticsStock", "👕 Cosmetics Stock"),
        ("seedsStock", "🌾 Seeds Stock"),
        ("merchantsStock", "🏪 Merchants Stock"),
    ] {
        caption.push_str(&format_stock(result.get(key), title));
    }

    if let Some(seen) = result
        .get("lastSeen")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
    {
        caption.push_str("*👀 Last Seen*\n");
        for item in seen.iter().take(5) {
            caption.push_str(&format!(
                "- {}: {}\n",
                interp(item.get("name")),
                interp(item.get("seen"))
            ));
        }
    }
    caption.trim().to_string()
}

#[plugin(
    name = "gag",
    alias = ["growagarden", "gaginfo"],
    category = "info",
    description = "Menampilkan informasi stok Grow a Garden",
    usage = ".gag",
    example = ".gag",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn gag(ctx: Ctx) -> Reply {
    let res = match ctx.http.get(URL) {
        Ok(r) => r,
        Err(_) => return Reply::text(FAILED).with_reaction("☢"),
    };
    // Body yang bukan JSON membuat `res.data.status` undefined di JS, jadi
    // jatuh ke cabang yang sama dengan payload rusak.
    let data = res.json().unwrap_or(Value::Null);
    if js_falsy(data.get("status")) || js_falsy(data.get("result")) {
        return Reply::text(NO_DATA).with_reaction("❌");
    }
    let result = data.get("result").expect("sudah dijaga js_falsy");
    Reply::text(render(result)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;


    fn run() -> Reply {
        let ctx = Ctx {
            command: "gag".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("gag").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("gag").unwrap();
        assert_eq!(p.meta.name, "gag");
        assert_eq!(p.meta.alias, ["growagarden", "gaginfo"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(
            p.meta.description,
            "Menampilkan informasi stok Grow a Garden"
        );
        assert_eq!(p.meta.usage, ".gag");
        assert_eq!(p.meta.example, ".gag");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["growagarden", "gaginfo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "gag");
        }
    }

    #[test]
    fn a_dead_http_client_lands_on_the_catch_branch() {
        // `Ctx::default()` memakai `NoHttp`, jadi tidak ada jaringan di test.
        let r = run();
        assert_eq!(text_of(&r), FAILED);
        assert_eq!(reaction_of(&r), "☢");
        assert!(r.mutations().is_empty(), "tidak ada state yang ditulis");
    }

    #[test]
    fn an_empty_or_missing_section_disappears_entirely() {
        assert_eq!(format_stock(None, "⚙️ Gear Stock"), "");
        assert_eq!(format_stock(Some(&serde_json::json!([])), "⚙️ Gear Stock"), "");
        assert_eq!(format_stock(Some(&serde_json::json!({})), "⚙️ Gear Stock"), "");
        assert_eq!(format_stock(Some(&serde_json::json!(null)), "⚙️ Gear Stock"), "");
        assert_eq!(format_stock(Some(&serde_json::json!(5)), "⚙️ Gear Stock"), "");
    }

    #[test]
    fn a_section_renders_one_line_per_item_with_a_blank_line_after() {
        let arr = serde_json::json!([{"name": "watering_can", "value": 3}, {"name": "hoe", "value": 0}]);
        assert_eq!(
            format_stock(Some(&arr), "⚙️ Gear Stock"),
            "*⚙️ Gear Stock*\n- watering_can: 3\n- hoe: 0\n\n"
        );
    }

    #[test]
    fn every_section_of_the_js_payload_appears_in_the_caption() {
        let result = serde_json::json!({
            "gearStock": [{"name": "g1", "value": 1}],
            "eggStock": [{"name": "e1", "value": 2}],
            "eventStock": [{"name": "v1", "value": 3}],
            "cosmeticsStock": [{"name": "c1", "value": 4}],
            "seedsStock": [{"name": "s1", "value": 5}],
            "merchantsStock": [{"name": "m1", "value": 6}],
            "lastSeen": [{"name": "l1", "seen": "now"}],
        });
        let out = render(&result);
        for title in [
            "⚙️ Gear Stock",
            "🥚 Egg Stock",
            "🎟️ Event Stock",
            "👕 Cosmetics Stock",
            "🌾 Seeds Stock",
            "🏪 Merchants Stock",
            "👀 Last Seen",
        ] {
            assert!(out.contains(&format!("*{title}*")), "{title} hilang: {out}");
        }
        for line in [
            "- g1: 1",
            "- e1: 2",
            "- v1: 3",
            "- c1: 4",
            "- s1: 5",
            "- m1: 6",
            "- l1: now",
        ] {
            assert!(out.contains(line), "{line} hilang: {out}");
        }
        assert!(out.starts_with("🌱 *GROW A GARDEN INFO* 🌱"), "{out}");
    }

    #[test]
    fn the_last_seen_list_is_capped_at_five_entries() {
        let seen: Vec<_> = (0..9)
            .map(|i| serde_json::json!({"name": format!("s{i}"), "seen": "now"}))
            .collect();
        let out = render(&serde_json::json!({"lastSeen": seen}));
        for i in 0..5 {
            assert!(out.contains(&format!("- s{i}: now")), "s{i} hilang: {out}");
        }
        for i in 5..9 {
            assert!(
                !out.contains(&format!("- s{i}: now")),
                "s{i} tidak boleh ikut: {out}"
            );
        }
    }

    #[test]
    fn a_payload_without_last_seen_skips_the_block_entirely() {
        let out = render(&serde_json::json!({"gearStock": [{"name": "g", "value": 1}]}));
        assert!(!out.contains("Last Seen"), "{out}");
        assert_eq!(
            out, "🌱 *GROW A GARDEN INFO* 🌱\n\n*⚙️ Gear Stock*\n- g: 1",
            "trim() harus mengikis blank line terakhir"
        );
    }

    #[test]
    fn a_missing_field_is_interpolated_the_way_javascript_does_it() {
        assert_eq!(interp(None), "undefined");
        assert_eq!(interp(Some(&Value::Null)), "null");
        assert_eq!(interp(Some(&serde_json::json!("x"))), "x");
        assert_eq!(interp(Some(&serde_json::json!(7))), "7");
    }

    #[test]
    fn a_result_object_with_no_sections_still_produces_the_header() {
        assert_eq!(render(&serde_json::json!({})), "🌱 *GROW A GARDEN INFO* 🌱");
    }
}
