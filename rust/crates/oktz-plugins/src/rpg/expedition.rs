//! `plugins/rpg/expedition.js` — kirim rombongan ekspedisi, klaim hasil
//! setelah durasi lewat.
//!
//! Kursor: waktu pakai `Date.now()` absolut per implode (`startedAt`), jadi
//! handler ini NON-deterministik terhadap jam — testnya menyuntikkan
//! `startedAt` relatif ke `now_ms()`, bukan meng froze clock. Kapasitas juga
//! naik per 10 level: `min(5, 1 + floor(level / 10))`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::rpg::heal::{apply_exp, inventory_map, math_random, or_default, rpg_map, user_level};

/// `(key, name, duration_ms, rewards, exp, minLevel)`.
type Expedition = (
    &'static str,
    &'static str,
    i64,
    &'static [&'static str],
    i64,
    i64,
);

const EXPEDITIONS: [Expedition; 5] = [
    ("cave", "🏔️ Gua Kelelawar", 3_600_000, &["iron", "gold", "gem"], 200, 5),
    ("forest", "🌲 Hutan Laba-laba", 1_800_000, &["wood", "herb", "mushroom"], 100, 1),
    ("ocean", "🌊 Samudra Kraken", 5_400_000, &["fish", "pearl", "seagem"], 300, 10),
    ("ruins", "🏛️ Reruntuhan Kuno", 10_800_000, &["ancientcoin", "relic", "mysterybox"], 600, 20),
    ("volcano", "🌋 Gunung Naga", 7_200_000, &["lava", "dragonscale", "titancore"], 400, 15),
];

fn area_info(key: &str) -> Option<&'static Expedition> {
    EXPEDITIONS.iter().find(|(k, ..)| *k == key)
}

/// `formatTime` (`expedition.js:28`) — jam > 0 menang atas menit.
fn format_time(ms: i64) -> String {
    let ms = ms.max(0);
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// `Math.min(5, 1 + Math.floor(user.level / 10))`.
fn capacity(level: i64) -> i64 {
    (1 + level.div_euclid(10)).min(5)
}

fn trips(rpg: &Map<String, Value>) -> Vec<Value> {
    rpg.get("expeditions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn started_at(trip: &Value) -> i64 {
    trip.get("startedAt").and_then(Value::as_i64).unwrap_or(0)
}

fn duration(trip: &Value) -> i64 {
    trip.get("duration").and_then(Value::as_i64).unwrap_or(0)
}

/// `rpg.expeditions.filter(e => now - e.startedAt >= e.duration)`.
fn split(trips: &[Value], now: i64) -> (Vec<Value>, Vec<Value>) {
    trips
        .iter()
        .cloned()
        .partition(|t| now - started_at(t) >= duration(t))
}

#[plugin(
    name = "expedition",
    alias = ["ekspedisi", "exp", "explore"],
    category = "rpg",
    description = "Kirim ekspedisi otomatis untuk item",
    usage = ".expedition <start/claim/status>",
    example = ".expedition start forest",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn expedition(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut inv = inventory_map(&ctx);

    let level = user_level(&ctx);
    let max_trips = capacity(level);
    let all = trips(&rpg);
    let action = crate::common::first_keyword(&ctx);
    let area = ctx.args.get(1).map(|a| a.to_lowercase());
    let now = crate::common::now_ms();

    let known = matches!(action.as_deref(), Some("start" | "claim" | "status" | "list"));
    match action.as_deref() {
        _ if !known => {
            Reply::text(format!(
                "🗺️ *MARKAS EKSPEDISI* 🗺️\n\n\
                 Kirim rombongan ekspedisi buat nyari barang-barang langka selagi kamu istirahat kak!\n\n\
                 *Daftar Perintah:*\n\
                 📜 `{}expedition list` (Cek Area)\n\
                 🚀 `{}expedition start <area>` (Mulai Ekspedisi)\n\
                 ⏳ `{}expedition status` (Cek Timer)\n\
                 💰 `{}expedition claim` (Tarik Hasil)\n\n\
                 📊 Kapasitas Ekspedisi Kamu: *{}/{} Rombongan*",
                ctx.prefix, ctx.prefix, ctx.prefix, ctx.prefix,
                all.len(),
                max_trips
            ))
        }

        Some("list") => {
            let mut txt = String::from("📜 *PETA EKSPLORASI DUNIA* 📜\n\n");
            for (key, name, dur, rewards, exp, min) in EXPEDITIONS {
                let open = if level >= min { "🔓" } else { "🔒" };
                txt.push_str(&format!(
                    "📍 {name} {open}\n\
                     \x20  ├ ⏳ Waktu: {}\n\
                     \x20  ├ 🎁 Potensi Loot: {}\n\
                     \x20  ├ 📈 EXP: {exp} (Min Lv. {min})\n\
                     \x20  └ 🚀 Kode Area: `{key}`\n\n",
                    format_time(dur),
                    rewards.join(", ")
                ));
            }
            Reply::text(txt)
        }

        Some("start") => {
            if all.len() as i64 >= max_trips {
                return Reply::text(format!(
                    "Duh kak, kapasitas ekspedisi kamu udah full! ({}/{})\n\
                     Tunggu rombongan yang lain balik dulu ya!",
                    all.len(),
                    max_trips
                ));
            }
            let Some(area) = area else {
                return Reply::text(format!(
                    "Pilih area tujuan ekspedisinya kak!\nContoh: `{}expedition start forest`",
                    ctx.prefix
                ));
            };
            let Some(&(_, name, dur, _, _, min)) = area_info(&area) else {
                return Reply::text(format!("Maaf kak, area *{area}* nggak ada di peta!"));
            };
            if level < min {
                return Reply::text(format!(
                    "Aduh kak, level kamu masih kurang nih. Butuh *Level {min}* buat ekspedisi ke sana!"
                ));
            }

            let mut updated = all;
            updated.push(json!({ "type": area, "startedAt": now, "duration": dur }));
            rpg.insert("expeditions".into(), Value::Array(updated));

            Reply::text(format!(
                "🚀 *EKSPEDISI DIBERANGKATKAN!* 🚀\n\n\
                 Rombongan ekspedisi kamu sudah berangkat menuju tujuan!\n\
                 📍 Tujuan: *{name}*\n\
                 ⏱️ Estimasi Waktu: *{}*\n\n\
                 > Silakan santai dulu kak, nanti ambil hasilnya pakai perintah `{}expedition claim`!",
                format_time(dur),
                ctx.prefix
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: json!({ "rpg": rpg }),
            })
        }

        Some("status") => {
            if all.is_empty() {
                return Reply::text("Belum ada ekspedisi yang jalan nih kak. Kirim sekarang yuk! 🏕️");
            }
            let mut txt = String::from("⏳ *RADAR EKSPEDISI* ⏳\n\n");
            for (i, trip) in all.iter().enumerate() {
                let kind = trip.get("type").and_then(Value::as_str).unwrap_or_default();
                let info = area_info(kind);
                let left = (duration(trip) - (now - started_at(trip))).max(0);
                let status = if left <= 0 {
                    "✅ SELESAI! (Siap Claim)".to_string()
                } else {
                    format!("🕒 Sisa {}", format_time(left))
                };
                txt.push_str(&format!(
                    "🗺️ *Rombongan {}* -> {}\n   └ Status: {status}\n\n",
                    i + 1,
                    info.map_or("??", |(_, n, ..)| *n)
                ));
            }
            Reply::text(txt)
        }

        _ => {
            let (done, pending) = split(&all, now);
            if done.is_empty() {
                return Reply::text(format!(
                    "Belum ada ekspedisi yang selesai kak! Cek dulu pakai `{}expedition status` ya!",
                    ctx.prefix
                ));
            }

            let mut total_exp = 0;
            let mut loot: Vec<String> = Vec::new();
            for trip in &done {
                let kind = trip.get("type").and_then(Value::as_str).unwrap_or_default();
                let Some(&(_, _, _, rewards, exp, _)) = area_info(kind) else {
                    continue;
                };
                total_exp += exp;
                for item in rewards {
                    if math_random() > 0.4 {
                        let qty = crate::common::roll(5) as i64 + 1;
                        let have = or_default(inv.get(*item).and_then(Value::as_i64), 0);
                        inv.insert((*item).to_string(), json!(have + qty));
                        loot.push(format!("{item} x{qty}"));
                    }
                }
            }

            rpg.insert("expeditions".into(), Value::Array(pending));
            let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
            let new_exp = apply_exp(&mut rpg, old_exp, total_exp);

            let mut txt = format!(
                "🎉 *EKSPEDISI SELESAI!* 🎉\n\n\
                 Rombongan kembali dan membawa hasil dari *{} ekspedisi*!\n\n\
                 *🎁 HASIL PENCARIAN:*\n\
                 ✨ EXP: *+{total_exp}*\n",
                done.len()
            );
            if loot.is_empty() {
                txt.push_str("📦 Items: *Aduh sayang sekali, kali ini nggak dapet apa-apa...* 😭\n");
            } else {
                txt.push_str("📦 Items:\n");
                for line in &loot {
                    txt.push_str(&format!("  • {line}\n"));
                }
            }

            Reply::text(txt).with_reaction("✅").with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "expedition".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("expedition").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("expedition").unwrap();
        assert_eq!(p.meta.name, "expedition");
        assert_eq!(p.meta.alias, ["ekspedisi", "exp", "explore"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Kirim ekspedisi otomatis untuk item");
        assert_eq!(p.meta.usage, ".expedition <start/claim/status>");
        assert_eq!(p.meta.example, ".expedition start forest");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["ekspedisi", "explore"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "expedition");
        }
    }

    /// `formatTime` — jam menang atas menit, dan detik yang tak pernah negatif.
    #[test]
    fn the_duration_format_prefers_the_largest_unit() {
        assert_eq!(format_time(0), "0s");
        assert_eq!(format_time(45_000), "45s");
        assert_eq!(format_time(120_000), "2m 0s");
        assert_eq!(format_time(1_800_000), "30m 0s");
        assert_eq!(format_time(3_600_000), "1h 0m");
        assert_eq!(format_time(1_080_000_000), "300h 0m");
        assert_eq!(format_time(-5_000), "0s", "sisa negatif di-clamp ke 0");
    }

    /// `Math.min(5, 1 + Math.floor(level / 10))` — level default 1 → 1.
    #[test]
    fn the_capacity_grows_by_one_per_ten_levels_and_stops_at_five() {
        assert_eq!(capacity(1), 1);
        assert_eq!(capacity(9), 1);
        assert_eq!(capacity(10), 2);
        assert_eq!(capacity(30), 4);
        assert_eq!(capacity(40), 5);
        assert_eq!(capacity(400), 5);
    }

    #[test]
    fn an_unknown_action_shows_the_headquarters_menu() {
        for args in [vec![], vec!["goto"], vec!["STOP"]] {
            let r = run(json!({}), &args);
            let t = text_of(&r);
            assert!(t.starts_with("🗺️ *MARKAS EKSPEDISI*"), "args {args:?}: {t}");
            assert!(t.contains("📊 Kapasitas Ekspedisi Kamu: *0/1 Rombongan*"), "{t}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn the_map_lists_every_area_with_its_level_gate() {
        let r = run(json!({ "level": 1 }), &["list"]);
        let t = text_of(&r);
        assert!(t.starts_with("📜 *PETA EKSPLORASI DUNIA*"), "{t}");
        for (key, name, dur, rewards, exp, min) in EXPEDITIONS {
            let gate = if 1 >= min { "🔓" } else { "🔒" };
            assert!(t.contains(&format!("📍 {name} {gate}")), "{key}: {t}");
            assert!(t.contains(&format!("🎁 Potensi Loot: {}", rewards.join(", "))), "{key}");
            assert!(t.contains(&format!("📈 EXP: {exp} (Min Lv. {min})")), "{key}");
            assert!(t.contains(&format!("⏳ Waktu: {}", format_time(dur))), "{key}");
        }
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_departure_records_the_trip_with_its_own_duration() {
        let r = run(json!({ "level": 1 }), &["start", "forest"]);
        let t = text_of(&r);
        assert!(t.starts_with("🚀 *EKSPEDISI DIBERANGKATKAN!*"), "{t}");
        assert!(t.contains("📍 Tujuan: *🌲 Hutan Laba-laba*"), "{t}");
        let trips = write_of(&r)["rpg"]["expeditions"].as_array().unwrap().clone();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0]["type"], json!("forest"));
        assert_eq!(trips[0]["duration"], json!(1_800_000));
        assert!(trips[0]["startedAt"].as_i64().unwrap() > 0);
    }

    #[test]
    fn starting_needs_an_area_that_exists_and_that_the_level_allows() {
        assert!(text_of(&run(json!({}), &["start"])).starts_with("Pilih area tujuan ekspedisinya"));
        assert!(text_of(&run(json!({}), &["start", "atlantis"])).starts_with("Maaf kak, area *atlantis*"));
        assert!(text_of(&run(json!({ "level": 1 }), &["start", "volcano"]))
            .starts_with("Aduh kak, level kamu masih kurang"));
    }

    #[test]
    fn a_full_party_cannot_depart_again() {
        let trips: Vec<Value> = (0..1)
            .map(|_| json!({ "type": "forest", "startedAt": 0, "duration": 1_800_000 }))
            .collect();
        let r = run(json!({ "rpg": { "expeditions": trips } }), &["start", "forest"]);
        let t = text_of(&r);
        assert!(t.starts_with("Duh kak, kapasitas ekspedisi kamu udah full!"), "{t}");
        assert!(t.contains("(1/1)"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_empty_radar_says_nothing_is_underway() {
        for user in [json!({}), json!({ "rpg": { "expeditions": [] } })] {
            let r = run(user.clone(), &["status"]);
            assert!(text_of(&r).starts_with("Belum ada ekspedisi yang jalan"), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn the_radar_separates_finished_trips_from_the_still_running_ones() {
        let now = crate::common::now_ms();
        let trips = vec![
            json!({ "type": "forest", "startedAt": now - 2_000_000, "duration": 1_800_000 }),
            json!({ "type": "forest", "startedAt": now, "duration": 1_800_000 }),
        ];
        let r = run(json!({ "rpg": { "expeditions": trips } }), &["status"]);
        let t = text_of(&r);
        assert!(t.contains("🗺️ *Rombongan 1* -> 🌲 Hutan Laba-laba"), "{t}");
        assert!(t.contains("✅ SELESAI! (Siap Claim)"), "{t}");
        assert!(t.contains("🗺️ *Rombongan 2* -> 🌲 Hutan Laba-laba"), "{t}");
        // `format_time` sudah dikunci persis di test sendiri. Di sini yang
        // dikunci cuma sisa ~30 menit, karena handler membaca `now_ms()`
        // sendiri beberapa ms setelah test — 1 ms cukup mengubah "30m 0s"
        // jadi "29m 59s".
        let sisa: i64 = t
            .split("Sisa ")
            .nth(1)
            .and_then(|s| s.split('m').next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or_else(|| panic!("sisa waktu tak terbaca: {t}"));
        assert!((29..=30).contains(&sisa), "sisa harus ~30 menit: {t}");
    }

    #[test]
    fn claiming_nothing_finishes_is_refused() {
        let now = crate::common::now_ms();
        let trips = vec![json!({ "type": "forest", "startedAt": now, "duration": 1_800_000 })];
        let r = run(json!({ "rpg": { "expeditions": trips } }), &["claim"]);
        assert!(text_of(&r).starts_with("Belum ada ekspedisi yang selesai"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_finished_claim_pays_exp_drops_items_and_drops_the_trip() {
        let mut with_loot = 0;
        let mut bare = 0;
        for _ in 0..600 {
            let now = crate::common::now_ms();
            let trips = vec![
                json!({ "type": "forest", "startedAt": now - 2_000_000, "duration": 1_800_000 }),
                json!({ "type": "forest", "startedAt": now, "duration": 1_800_000 }),
            ];
            let r = run(json!({ "rpg": { "expeditions": trips }, "exp": 0 }), &["claim"]);
            let t = text_of(&r);
            assert!(t.starts_with("🎉 *EKSPEDISI SELESAI!*"), "{t}");
            assert!(t.contains("dari *1 ekspedisi*"), "{t}");
            assert!(t.contains("✨ EXP: *+100*"), "exp hutan = 100: {t}");
            let data = write_of(&r);
            assert_eq!(data["exp"], json!(100));
            let left = data["rpg"]["expeditions"].as_array().unwrap();
            assert_eq!(left.len(), 1, "hanya ekspedisi selesai yang dicabut");
            let inv = data["inventory"].as_object().cloned().unwrap_or_default();
            if inv.is_empty() {
                bare += 1;
                assert!(t.contains("tidak dalem apa-apa") || t.contains("nggak dapet apa-apa"), "{t}");
            } else {
                with_loot += 1;
                for item in ["wood", "herb", "mushroom"] {
                    if let Some(qty) = inv.get(item) {
                        assert!((1..=5).contains(&qty.as_i64().unwrap()), "{item} {qty}");
                    }
                }
            }
        }
        assert!(with_loot > 0 && bare > 0, "loot={with_loot} kosong={bare}");
    }

    #[test]
    fn the_travel_table_keeps_every_js_row() {
        assert_eq!(EXPEDITIONS.len(), 5);
        let want: Expedition = ("cave", "🏔️ Gua Kelelawar", 3_600_000,
            &["iron", "gold", "gem"], 200, 5);
        assert_eq!(EXPEDITIONS[0], want);
        let want: Expedition = ("volcano", "🌋 Gunung Naga", 7_200_000,
            &["lava", "dragonscale", "titancore"], 400, 15);
        assert_eq!(EXPEDITIONS[4], want);
        assert!(area_info("tidak-ada").is_none());
    }
}