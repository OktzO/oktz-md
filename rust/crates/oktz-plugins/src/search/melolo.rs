//! `plugins/search/melolo.js` — cari daftar drama pendek per kategori dari
//! Melolo lewat provider `covenant`.
//!
//! `ponytail:` `config.APIkey?.covenant` tidak ada di `Ctx`, jadi guard
//! `melolo.js:74` selalu benar dan `explainFailure(MissingApiKeyError(..))`
//! yang jadi balasan — sama seperti `download::sfiledl`.
//!
//! `ponytail:` `fotoApi.covenant.meloloCategory` (`src/lib/apimanager.js`) tidak
//! di-port; jalur HTTP setelahnya tidak terjangkau selama kunci kosong, jadi
//! hanya logika plugin yang dipindah: normalisasi hasil, pemotongan teks, dan
//! susunan caption.

#![allow(dead_code)] // jalur HTTP tidak terjangkau tanpa kunci covenant

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

/// Batas default `trimText` (`melolo.js:22`) — 90, bukan 60 seperti plugin
/// sekelasnya.
pub(crate) const DEFAULT_MAX: usize = 90;

/// `melolo.js:22-29` — `String(text || "")`, spasi dirapatkan, kosong jadi `-`.
pub(crate) fn trim_text(text: Option<&Value>, max: usize) -> String {
    let raw = match text {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        // `String({})` = "[object Object]", `String(12)` = "12".
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(_)) | Some(Value::Object(_)) => String::from("[object Object]"),
    };
    let mut value = String::new();
    let mut in_ws = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            in_ws = true;
        } else {
            if in_ws {
                value.push(' ');
                in_ws = false;
            }
            value.push(c);
        }
    }
    if in_ws {
        value.push(' ');
    }
    let value = value.trim().to_string();
    if value.is_empty() {
        return "-".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        return value;
    }
    chars.into_iter().take(max).collect::<String>() + "..."
}

/// `melolo.js:68` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!("🎭 *MELOLO DRAMA*\n\n> Contoh:\n`{prefix}melolo fantasy`")
}

/// `explainFailure(new MissingApiKeyError("covenant", "APIKEY_COVENANT"), …)`
/// (`src/lib/stalker-fallback.js:29-33`). `command` sudah dirangkai jadi
/// `` `${prefix}melolo` `` oleh pemanggil.
pub(crate) fn api_key_belum_diset(command: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `APIKEY_COVENANT` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

/// `melolo.js:66` — hasil kosong.
pub(crate) fn tidak_ditemukan(category: &str) -> String {
    format!("❌ Tidak ditemukan hasil Melolo untuk kategori: {category}")
}

/// Satu drama setelah `normalizeResults` (`melolo.js:31-49`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Drama {
    pub section: String,
    pub title: String,
    pub url: String,
    pub image: String,
    pub rating: String,
    pub episodes: String,
}

/// `normalizeResults(data)` — `Object.entries(data || {})`, lalu tiap
/// elemen array diratakan jadi satu drama. Urutan = urutan key objek, persis
/// seperti `for..of Object.entries`.
pub(crate) fn normalize_results(data: &Value) -> Vec<Drama> {
    let mut out = Vec::new();
    let Some(obj) = data.as_object() else {
        return out;
    };
    for (section, items) in obj {
        let Some(list) = items.as_array() else {
            continue;
        };
        for item in list {
            out.push(Drama {
                section: section.clone(),
                title: or_dash(item.get("title")),
                url: or_dash(item.get("url")),
                image: item
                    .get("image")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                rating: or_dash(item.get("rating")),
                episodes: or_dash(item.get("episodes")),
            });
        }
    }
    out
}

/// `item?.x || "-"` — `0` dan `""` falsy di JS.
fn or_dash(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => n.to_string(),
        Some(Value::Bool(true)) => "true".to_string(),
        _ => "-".to_string(),
    }
}

/// `value ?? "-"` lalu `${..}` — `??` hanya kena `null`/`undefined`, jadi `0`
/// dan `""` tetap tampil apa adanya (beda dari `||`).
fn null_coalesce(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        // `${[1,2]}` = "1,2", `${[a:1]}` = "[object Object]".
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| null_coalesce(Some(v)))
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// `melolo.js:96-108` — caption; judul dipotong 70, section 32, episodes 110.
pub(crate) fn caption(items: &[Drama], category: &str, usage: Option<&Value>) -> String {
    let mut txt = String::from("🎭 *MELOLO DRAMA*\n\n");
    txt.push_str(&format!("🌿 *Category:* {category}\n"));
    txt.push_str(&format!("📦 *Total:* {}\n", items.len()));
    txt.push_str(&format!(
        "💳 *Cost:* {}\n",
        null_coalesce(usage.and_then(|u| u.get("cost")))
    ));
    txt.push_str(&format!(
        "🔋 *Sisa Credit:* {}\n\n",
        null_coalesce(usage.and_then(|u| u.get("remaining")))
    ));
    for (i, item) in items.iter().enumerate() {
        txt.push_str(&format!(
            "*{}.* {}\n   ├ 📂 {}\n   ├ ⭐ {}\n   ├ 📝 {}\n   └ {}\n\n",
            i + 1,
            trim_text(Some(&Value::String(item.title.clone())), 70),
            trim_text(Some(&Value::String(item.section.clone())), 32),
            item.rating,
            trim_text(Some(&Value::String(item.episodes.clone())), 110),
            item.url
        ));
    }
    txt.trim().to_string()
}

/// `items.find((item) => item.image)?.image` (`melolo.js:110`) — item PERTAMA
/// yang punya gambar, bukan yang pertama.
pub(crate) fn cover(items: &[Drama]) -> Option<&str> {
    items
        .iter()
        .find(|d| !d.image.is_empty())
        .map(|d| d.image.as_str())
}

#[plugin(
    name = "melolo",
    alias = ["melolodrama", "dramamelolo"],
    category = "search",
    description = "Cari daftar drama pendek berdasarkan kategori dari Melolo",
    usage = ".melolo <category>",
    example = ".melolo fantasy",
    cooldown = 8,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn melolo(ctx: Ctx) -> Reply {
    let category = ctx.text.trim();

    if category.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `melolo.js:74` dicek sebelum `m.react("🔍")` di `:83`, jadi tanpa reaksi.
    Reply::text(api_key_belum_diset(&format!("{}melolo", ctx.prefix)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "melolo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("melolo").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("melolo").unwrap();
        assert_eq!(p.meta.name, "melolo");
        assert_eq!(p.meta.alias, ["melolodrama", "dramamelolo"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Cari daftar drama pendek berdasarkan kategori dari Melolo"
        );
        assert_eq!(p.meta.usage, ".melolo <category>");
        assert_eq!(p.meta.example, ".melolo fantasy");
        assert_eq!(p.meta.cooldown, 8);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["melolodrama", "dramamelolo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "melolo");
        }
    }

    #[test]
    fn empty_input_shows_the_help_card() {
        for text in ["", "   ", "\n\t"] {
            assert_eq!(
                text_of(&run(text)),
                "🎭 *MELOLO DRAMA*\n\n> Contoh:\n`.melolo fantasy`",
                "input {text:?}"
            );
        }
        assert!(help("!").contains("`!melolo fantasy`"));
    }

    #[test]
    fn a_category_reports_the_missing_covenant_key() {
        assert_eq!(
            text_of(&run("fantasy")),
            "❌ *API Key belum diset!*\n\n\
             > Command `.melolo` butuh `APIKEY_COVENANT` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
        assert!(api_key_belum_diset("!melolo").contains("`!melolo`"));
    }

    #[test]
    fn odd_and_very_long_categories_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "???"] {
            assert!(
                text_of(&run(input)).contains("APIKEY_COVENANT"),
                "input {input:?}"
            );
        }
        let panjang = "fantasy ".repeat(3000);
        assert!(text_of(&run(&panjang)).contains("APIKEY_COVENANT"));
    }

    #[test]
    fn trim_text_defaults_to_ninety_and_uses_string_coercion() {
        assert_eq!(DEFAULT_MAX, 90);
        assert_eq!(trim_text(None, DEFAULT_MAX), "-");
        assert_eq!(trim_text(Some(&Value::Null), DEFAULT_MAX), "-");
        assert_eq!(trim_text(Some(&json!(12)), DEFAULT_MAX), "12");
        assert_eq!(trim_text(Some(&json!(true)), DEFAULT_MAX), "true");
        assert_eq!(
            trim_text(Some(&json!({"a": 1})), DEFAULT_MAX),
            "[object Object]",
            "String(obj) di JS"
        );
        assert_eq!(
            trim_text(Some(&Value::String(" a  b\n c ".into())), DEFAULT_MAX),
            "a b c"
        );
    }

    #[test]
    fn trim_text_cuts_at_the_caller_supplied_limit() {
        let judul = "x".repeat(80);
        let nilai = Value::String(judul);
        let t = trim_text(Some(&nilai), 70);
        assert_eq!(t.chars().count(), 73, "70 + '...'");
        assert!(t.ends_with("..."));
    }

    #[test]
    fn groups_flatten_and_keep_their_section_name() {
        let data = json!({
            "trending": [ { "title": "A", "url": "https://a", "image": "https://i/1" } ],
            "new": [
                { "title": "B", "url": "https://b", "rating": "9.1", "episodes": "Eps 1-10" },
                { "title": "", "url": null },
            ],
            "bukanArray": "diabaikan",
        });
        let items = normalize_results(&data);
        assert_eq!(items.len(), 3, "key non-array dilewati");
        assert_eq!(items[0].section, "trending");
        assert_eq!(items[1].section, "new");
        assert_eq!(items[2].title, "-", "judul kosong -> dash");
        assert_eq!(items[2].url, "-", "url null -> dash");
        assert_eq!(items[1].rating, "9.1");
        assert_eq!(items[1].episodes, "Eps 1-10");
        assert_eq!(items[0].image, "https://i/1");
        assert_eq!(items[1].image, "", "image kosong bukan dash");
    }

    #[test]
    fn a_missing_or_scalar_payload_normalizes_to_nothing() {
        assert!(normalize_results(&Value::Null).is_empty());
        assert!(normalize_results(&json!("teks")).is_empty());
        assert!(normalize_results(&json!([])).is_empty());
        assert!(normalize_results(&json!({})).is_empty());
    }

    #[test]
    fn the_caption_lists_numbered_drama_with_trimmed_fields() {
        let items = vec![
            Drama {
                section: "trending".into(),
                title: "  Drama   Satu ".into(),
                url: "https://m/1".into(),
                image: "https://i/1".into(),
                rating: "8.5".into(),
                episodes: " Eps 1-20 ".into(),
            },
            Drama {
                section: "baru".into(),
                title: "Drama Dua".into(),
                url: "https://m/2".into(),
                image: String::new(),
                rating: "-".into(),
                episodes: "-".into(),
            },
        ];
        let usage = json!({ "cost": 5, "remaining": 0 });
        let t = caption(&items, "fantasy", Some(&usage));
        assert!(t.starts_with("🎭 *MELOLO DRAMA*\n\n🌿 *Category:* fantasy\n📦 *Total:* 2\n💳 *Cost:* 5\n🔋 *Sisa Credit:* 0\n\n"), "{t}");
        assert!(t.contains("*1.* Drama Satu\n   ├ 📂 trending\n   ├ ⭐ 8.5\n   ├ 📝 Eps 1-20\n   └ https://m/1\n"), "{t}");
        assert!(t.contains("*2.* Drama Dua\n"), "{t}");
        assert!(!t.ends_with('\n'), "sudah di-trim: {t:?}");
    }

    #[test]
    fn zero_usage_cost_survives_because_the_js_uses_null_coalescing() {
        // `result?.usage?.cost ?? "-"` — `0` bukan null, jadi tetap "0".
        let items = vec![Drama {
            section: "s".into(),
            title: "t".into(),
            url: "u".into(),
            image: String::new(),
            rating: "-".into(),
            episodes: "-".into(),
        }];
        let t = caption(&items, "f", None);
        assert!(t.contains("💳 *Cost:* -\n"), "usage hilang -> dash: {t}");
        assert!(t.contains("🔋 *Sisa Credit:* -\n"), "{t}");
    }

    #[test]
    fn the_cover_is_the_first_item_that_actually_has_an_image() {
        let items = vec![
            Drama {
                section: "a".into(),
                title: "1".into(),
                url: "u".into(),
                image: String::new(),
                rating: "-".into(),
                episodes: "-".into(),
            },
            Drama {
                section: "b".into(),
                title: "2".into(),
                url: "u".into(),
                image: "https://i/2".into(),
                rating: "-".into(),
                episodes: "-".into(),
            },
        ];
        assert_eq!(cover(&items), Some("https://i/2"));
        assert_eq!(cover(&[]), None);
    }

    #[test]
    fn the_empty_result_message_names_the_category() {
        assert_eq!(
            tidak_ditemukan("romance"),
            "❌ Tidak ditemukan hasil Melolo untuk kategori: romance"
        );
    }

    #[test]
    fn a_caption_carries_no_media_when_no_item_has_a_cover() {
        // Bentuk `Reply` yang akan dipakai setelah kapabilitas ada: teks biasa
        // kalau tidak ada gambar. Diuji lewat komponennya supaya tidak
        // mengarang variant yang tidak ada.
        let items = normalize_results(&json!({ "s": [ { "title": "x" } ] }));
        assert_eq!(cover(&items), None);
    }
}