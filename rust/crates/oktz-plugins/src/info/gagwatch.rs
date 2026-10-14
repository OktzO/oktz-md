//! `plugins/info/gagwatch.js` — stok GAG2 Watch dari `my.izuka-api.xyz`.
//!
//! `ponytail:` reaksi hilang sebagian. JS bereaksi `🕕` di awal lalu `✅`,
//! `❌`, atau `☢`; `Reply::Reaction` hanya bisa membawa satu emoji, jadi
//! hanya reaksi terakhir yang ikut.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_falsy, js_truthy_value};

pub(crate) const URL: &str = "https://my.izuka-api.xyz/api/tools/gag2-watch?watchItems=Seed";

/// `data.status`/`data.data`/`data.data.stock` kosong → cabang `❌`
/// (`gagwatch.js:23-26`).
pub(crate) const NO_DATA: &str = "Maaf, data GAG2 tidak ditemukan atau sistem sedang bermasalah.";

/// Cabang `catch` (`gagwatch.js:60-63`).
pub(crate) const FAILED: &str = "Terjadi kesalahan saat mengambil data GAG. Coba lagi nanti.";

/// `${value}` di JS — `undefined` untuk field yang hilang.
fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `arr || '-'` (`gagwatch.js:33,34`) — fallback kalau nilai falsy.
fn or_dash(v: Option<&Value>) -> String {
    if v.is_some_and(js_truthy_value) {
        interp(v)
    } else {
        "-".to_string()
    }
}

/// `*TITLE:*\n` + `- {name}: {quantity}\n` per item, lalu baris kosong.
/// `gagwatch.js:47-60`. `Err` kalau `arr` bukan array — di JS `.forEach`
/// melempar TypeError yang ditangkap `catch`.
fn item_list(arr: Option<&Value>, title: &str) -> Result<String, String> {
    let items = arr
        .and_then(Value::as_array)
        .ok_or_else(|| FAILED.to_string())?;
    let mut txt = format!("*{title}*\n");
    for item in items {
        txt.push_str(&format!(
            "- {}: {}\n",
            interp(item.get("name")),
            interp(item.get("quantity"))
        ));
    }
    txt.push('\n');
    Ok(txt)
}

/// Teks untuk objek `stock` yang sudah lolos cek `status`/`stock`.
pub(crate) fn render(stock: &Value) -> Result<String, String> {
    let mut txt = String::from("🌱 *GAG2 STOCK MONITOR*\n\n");
    txt.push_str(&format!("*STATUS:* {}\n", or_dash(stock.get("message"))));
    txt.push_str(&format!(
        "*RESTOCK IN:* {}\n\n",
        or_dash(stock.get("restockInLabel"))
    ));

    // `if (weather && weather.active)` — `weather.active` pada nilai non-objek
    // selalu `undefined`, jadi tidak pernah salah masuk ke blok ini.
    if let Some(weather) = stock.get("weather").filter(|w| js_truthy_value(w))
        && weather.get("active").is_some_and(js_truthy_value)
    {
        // `weather.type.toUpperCase()` melempar kalau `type` hilang.
        let kind = weather
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| FAILED.to_string())?
            .to_uppercase();
        txt.push_str(&format!("⛅ *WEATHER:* {kind}\n"));
        if let Some(effects) = weather.get("effects").and_then(Value::as_array)
            && !effects.is_empty()
        {
            txt.push_str(&format!("_Efek:_ {}\n", interp(effects.first())));
        }
        txt.push('\n');
    }

    txt.push_str(&item_list(stock.get("seeds"), "SEEDS")?);
    txt.push_str(&item_list(stock.get("gear"), "GEAR")?);
    let crates = item_list(stock.get("crates"), "CRATES")?;
    // Blok terakhir JS tidak menambah baris kosong sebelum `reply(txt)`.
    txt.push_str(crates.trim_end_matches('\n'));
    Ok(txt)
}

#[plugin(
    name = "gag2watch",
    alias = ["gag2-watch"],
    category = "info",
    description = "Cek informasi stok GAG2 Watch",
    usage = ".gag2-watch",
    example = "",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn gagwatch(ctx: Ctx) -> Reply {
    let res = match ctx.http.get(URL) {
        Ok(r) => r,
        Err(_) => return Reply::text(FAILED).with_reaction("☢"),
    };
    let data = res.json().unwrap_or(Value::Null);
    if js_falsy(data.get("status"))
        || js_falsy(data.get("data"))
        || js_falsy(data.get("data").and_then(|d| d.get("stock")))
    {
        return Reply::text(NO_DATA).with_reaction("❌");
    }
    let stock = data
        .get("data")
        .and_then(|d| d.get("stock"))
        .expect("sudah dijaga js_falsy");
    match render(stock) {
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
            command: "gag2-watch".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("gag2watch").unwrap().handler)(ctx)
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

    fn stock() -> Value {
        json!({
            "message": "Update selesai",
            "restockInLabel": "2j 15m",
            "seeds": [{"name": "Carrot", "quantity": 3}],
            "gear": [{"name": "Watering Can", "quantity": 1}],
            "crates": [{"name": "Common", "quantity": 8}],
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gag2watch").unwrap();
        assert_eq!(p.meta.name, "gag2watch");
        assert_eq!(p.meta.alias, ["gag2-watch"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Cek informasi stok GAG2 Watch");
        assert_eq!(p.meta.usage, ".gag2-watch");
        assert_eq!(p.meta.example, "", "tidak ada `example` di pluginConfig JS");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn the_usage_alias_resolves_here() {
        assert_eq!(
            oktz_plugin_api::find("gag2-watch").unwrap().meta.name,
            "gag2watch"
        );
    }

    #[test]
    fn a_dead_http_client_lands_on_the_catch_branch() {
        let r = run();
        assert_eq!(text_of(&r), FAILED);
        assert_eq!(reaction_of(&r), "☢");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_header_uses_a_dash_when_the_value_is_falsy() {
        let s =
            json!({"message": "", "restockInLabel": null, "seeds": [], "gear": [], "crates": []});
        assert_eq!(
            render(&s).unwrap(),
            "🌱 *GAG2 STOCK MONITOR*\n\n\
             *STATUS:* -\n*RESTOCK IN:* -\n\n\
             *SEEDS:*\n\n*GEAR:*\n\n*CRATES:*"
        );
    }

    #[test]
    fn all_three_lists_render_in_order() {
        let out = render(&stock()).unwrap();
        let seeds = out.find("*SEEDS:*").expect("SEEDS");
        let gear = out.find("*GEAR:*").expect("GEAR");
        let crates = out.find("*CRATES:*").expect("CRATES");
        assert!(seeds < gear && gear < crates, "urutan salah: {out}");
        assert!(out.contains("- Carrot: 3"), "{out}");
        assert!(out.contains("- Watering Can: 1"), "{out}");
        assert!(out.contains("- Common: 8"), "{out}");
        assert!(
            out.ends_with("*CRATES:*\n- Common: 8"),
            "blok terakhir tidak boleh menambah baris kosong: {out:?}"
        );
    }

    #[test]
    fn an_active_weather_block_prints_the_uppercased_type_and_first_effect() {
        let s = json!({
            "message": "ok", "restockInLabel": "1m",
            "weather": {"active": true, "type": "rain", "effects": ["Wet", "Slow"]},
            "seeds": [], "gear": [], "crates": [],
        });
        assert!(
            render(&s).unwrap().contains("⛅ *WEATHER:* RAIN"),
            "{}",
            render(&s).unwrap()
        );
        assert!(
            render(&s).unwrap().contains("_Efek:_ Wet"),
            "hanya efek pertama"
        );
        assert!(
            !render(&s).unwrap().contains("Slow"),
            "efek kedua tidak ikut"
        );
    }

    #[test]
    fn an_inactive_weather_block_is_skipped_entirely() {
        let s = json!({
            "message": "ok", "restockInLabel": "1m",
            "weather": {"active": false, "type": "rain", "effects": ["Wet"]},
            "seeds": [], "gear": [], "crates": [],
        });
        assert!(
            !render(&s).unwrap().contains("WEATHER"),
            "{}",
            render(&s).unwrap()
        );
    }

    #[test]
    fn a_missing_list_is_an_error_not_a_silent_empty_section() {
        // `stock.seeds.forEach` melempar TypeError di JS → cabang catch.
        let mut s = stock();
        s.as_object_mut().unwrap().remove("gear");
        assert_eq!(render(&s).unwrap_err(), FAILED);
    }

    #[test]
    fn an_active_weather_without_a_type_is_an_error_too() {
        let s = json!({
            "message": "ok", "restockInLabel": "1m",
            "weather": {"active": true, "effects": []},
            "seeds": [], "gear": [], "crates": [],
        });
        assert_eq!(render(&s).unwrap_err(), FAILED);
    }
}
