//! `plugins/rpg/garden.js` — beli bibit, tanam, panen. Waktu tumbuh memakai
//! `Date.now()` absolut per petak.
//!
//! Kursor: `qty = Math.max(1, parseInt(args[2]) || 1)` — jumlah beli minimal
//! satu dan `NaN`/`0` menjadi 1; EFEK panenya cuma tambah `crop.exp` SEKALI
//! per petak, bukan dikali qty.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{apply_exp, inventory_map, or_default, rpg_map};

/// `(key, name, growTime_ms, exp, sellPrice, seedPrice)`.
type Crop = (
    &'static str,
    &'static str,
    i64,
    i64,
    i64,
    i64,
);

const CROPS: [Crop; 8] = [
    ("carrot", "🥕 Wortel", 300_000, 50, 30, 10),
    ("tomato", "🍅 Tomat", 600_000, 80, 50, 20),
    ("corn", "🌽 Jagung", 900_000, 120, 80, 35),
    ("potato", "🥔 Kentang", 1_200_000, 150, 100, 45),
    ("herb", "🌿 Herba", 1_500_000, 180, 120, 50),
    ("strawberry", "🍓 Stroberi", 1_800_000, 200, 150, 60),
    ("watermelon", "🍉 Semangka", 3_600_000, 350, 300, 100),
    ("pumpkin", "🎃 Labu", 7_200_000, 500, 500, 150),
];

const MAX_PLOTS: usize = 3;

fn crop(key: &str) -> Option<&'static Crop> {
    CROPS.iter().find(|(k, ..)| *k == key)
}

/// `formatTime` (`garden.js:31`) — hanya menit dan detik.
fn format_time(ms: i64) -> String {
    let ms = ms.max(0);
    let minutes = ms / 60_000;
    let seconds = (ms % 60_000) / 1000;
    if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// `Math.max(1, parseInt(args[2]) || 1)`.
fn buy_qty(raw: Option<&String>) -> i64 {
    raw.map_or(0, |a| crate::common::parse_int(a).unwrap_or(0)).max(1)
}

fn seed_key(crop: &str) -> String {
    format!("{crop}seed")
}

/// Petak = `{ crop, plantedAt }`.
fn plots(rpg: &Map<String, Value>) -> Vec<Value> {
    rpg.get("garden")
        .and_then(|g| g.get("plots"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn max_plots(rpg: &Map<String, Value>) -> usize {
    rpg.get("garden")
        .and_then(|g| g.get("maxPlots"))
        .and_then(Value::as_u64)
        .unwrap_or(MAX_PLOTS as u64) as usize
}

#[plugin(
    name = "garden",
    alias = ["kebun", "farm", "tanam"],
    category = "rpg",
    description = "Berkebun dan panen tanaman",
    usage = ".garden <plant/harvest/status>",
    example = ".garden plant carrot",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn garden(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut inv = inventory_map(&ctx);

    let action = crate::common::first_keyword(&ctx);
    let crop_name = ctx.args.get(1).map(|a| a.to_lowercase());
    let now = crate::common::now_ms();

    let known = matches!(
        action.as_deref(),
        Some("plant" | "harvest" | "status" | "buy")
    );
    if !known {
        let mut txt = format!(
            "Halo Juragan Kebun! 👨‍🌾🌻\nPusat informasi kebun pribadimu nih.\n\n\
             *Menu Kebun:*\n\
             • `{}garden status` - Cek kondisi kebun\n\
             • `{}garden buy <tanaman> <jumlah>` - Beli bibit\n\
             • `{}garden plant <tanaman>` - Tanam bibit di tanah kosong\n\
             • `{}garden harvest` - Panen semua yang udah matang\n\n\
             *Daftar Bibit Tersedia:*\n",
            ctx.prefix, ctx.prefix, ctx.prefix, ctx.prefix
        );
        for (key, name, grow, _, sell, seed) in CROPS {
            txt.push_str(&format!(
                "\n*{name}*\n⏳ Waktu Tumbuh: {}\n💰 Harga Jual: Rp {sell} | 🌱 Harga Bibit: Rp {seed}\n👉 Beli: `.garden buy {key}`\n",
                format_time(grow)
            ));
        }
        return Reply::text(txt);
    }

    match action.as_deref().unwrap_or_default() {
        "status" => {
            let all = plots(&rpg);
            let mut txt = format!(
                "Cek Lahan Kebun... 🚜🌱\n\n*Kapasitas Tanah:* {} dari {} terisi.\n\n",
                all.len(),
                max_plots(&rpg)
            );
            if all.is_empty() {
                txt.push_str(&format!(
                    "Waduh, kebun kamu masih gersang kak! 🏜️\nBuruan beli bibit terus `{}garden plant <nama>` biar hijau lagi!",
                    ctx.prefix
                ));
            } else {
                txt.push_str("*Daftar Lahan:*\n");
                for (i, plot) in all.iter().enumerate() {
                    let key = plot.get("crop").and_then(Value::as_str).unwrap_or_default();
                    let planted = plot.get("plantedAt").and_then(Value::as_i64).unwrap_or(0);
                    let info = crop(key);
                    let name = info.map_or("???", |(_, n, ..)| n);
                    let left = info.map_or(0, |(_, _, grow, ..)| (grow - (now - planted)).max(0));
                    let status = if left <= 0 {
                        "✨ SIAP PANEN! ✨".to_string()
                    } else {
                        format!("Tumbuh dalam ⏳ {}", format_time(left))
                    };
                    txt.push_str(&format!("\n📍 Plot {}: *{name}*\n└ Status: {status}\n", i + 1));
                }
            }
            Reply::text(txt)
        }

        "buy" => {
            let Some(crop_name) = crop_name else {
                return Reply::text(format!(
                    "Hayo, mau beli bibit apaan? Namanya belum ditulis kak! 😂\nContoh: `{}garden buy carrot 5`",
                    ctx.prefix
                ));
            };
            let Some(&(_, name, _, _, _, seed_price)) = crop(&crop_name) else {
                return Reply::text(format!(
                    "Bibit itu nggak dijual di toko tani kita kak! ❌\nCek listnya lagi pake `{}garden`",
                    ctx.prefix
                ));
            };

            let qty = buy_qty(ctx.args.get(2));
            let cost = seed_price * qty;
            let koin = crate::common::int_field(&ctx, "koin");
            if koin < cost {
                return Reply::text(format!(
                    "Eits, uang kamu kurang kak! 😭\n\
                     Total belanjanya Rp {}, tapi koin kamu sisa Rp {}.",
                    crate::tools::kalkulatormbg::thousands_id(cost as f64),
                    crate::tools::kalkulatormbg::thousands_id(koin as f64)
                ));
            }

            let key = seed_key(&crop_name);
            let have = or_default(inv.get(&key).and_then(Value::as_i64), 0);
            inv.insert(key, serde_json::json!(have + qty));

            Reply::text(format!(
                "Makasih udah belanja di Toko Tani! 🛒🌱\n\n\
                 Kamu beli *{qty}x Bibit {name}*\n\
                 Total Bayar: *Rp {}*\n\n\
                 Jangan lupa ditanam ya pake `{}garden plant {crop_name}`!",
                crate::tools::kalkulatormbg::thousands_id(cost as f64),
                ctx.prefix
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "koin": koin - cost, "inventory": inv }),
            })
        }

        "plant" => {
            let Some(crop_name) = crop_name else {
                return Reply::text(format!(
                    "Tanah udah siap, tapi bibit apa yang mau ditanam nih? 🌱\nContoh: `{}garden plant carrot`",
                    ctx.prefix
                ));
            };
            let Some(&(_, name, grow, ..)) = crop(&crop_name) else {
                return Reply::text("Tanaman itu nggak ada di buku panduan tani kak! ❌");
            };

            let mut all = plots(&rpg);
            if all.len() >= max_plots(&rpg) {
                return Reply::text(
                    "Waduh, lahannya udah penuh semua! 🚜💨\n\
                     Kamu harus panen dulu atau *upgrade* kebunmu!",
                );
            }

            let key = seed_key(&crop_name);
            let have = or_default(inv.get(&key).and_then(Value::as_i64), 0);
            if have < 1 {
                return Reply::text(format!(
                    "Kamu nggak punya bibit *{name}* nih kak! 😭\n\
                     Beli dulu gih di `{}garden buy {crop_name}`",
                    ctx.prefix
                ));
            }

            let left = have - 1;
            if left <= 0 {
                inv.remove(&key);
            } else {
                inv.insert(key.clone(), serde_json::json!(left));
            }
            all.push(serde_json::json!({ "crop": crop_name, "plantedAt": now }));
            rpg.insert("plots".into(), Value::Null);
            let mut garden_obj = rpg
                .get("garden")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            garden_obj.insert("plots".into(), Value::Array(all));
            garden_obj.entry("maxPlots").or_insert(serde_json::json!(MAX_PLOTS));
            rpg.insert("garden".into(), Value::Object(garden_obj));

            Reply::text(format!(
                "Sipp! Bibit *{name}* udah ditanam di tanah! 🌱💦\n\
                 Jangan lupa disiram (eh, otomatis sih), tinggal tunggu *{}* buat panen ya!",
                format_time(grow)
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "inventory": inv, "rpg": rpg }),
            })
        }

        _ => {
            let all = plots(&rpg);
            let ready: Vec<&Value> = all
                .iter()
                .filter(|p| {
                    let key = p.get("crop").and_then(Value::as_str).unwrap_or_default();
                    let planted = p.get("plantedAt").and_then(Value::as_i64).unwrap_or(0);
                    crop(key).is_some_and(|(_, _, grow, ..)| now - planted >= *grow)
                })
                .collect();

            if ready.is_empty() {
                return Reply::text(format!(
                    "Hadeh, belum ada yang mateng kak! Sabar dikit napa 😂\n\
                     Cek waktunya pake `{}garden status`",
                    ctx.prefix
                ));
            }

            let mut total_exp = 0;
            let mut lines: Vec<String> = Vec::new();
            for plot in &ready {
                let key = plot.get("crop").and_then(Value::as_str).unwrap_or_default();
                let Some(&(_, name, _, exp, ..)) = crop(key) else {
                    continue;
                };
                let qty = crate::common::roll(3) as i64 + 2;
                let have = or_default(inv.get(key).and_then(Value::as_i64), 0);
                inv.insert(key.to_string(), serde_json::json!(have + qty));
                total_exp += exp;
                lines.push(format!("• {name} x{qty}"));
            }

            let pending: Vec<Value> = all
                .into_iter()
                .filter(|p| {
                    let key = p.get("crop").and_then(Value::as_str).unwrap_or_default();
                    let planted = p.get("plantedAt").and_then(Value::as_i64).unwrap_or(0);
                    crop(key).is_none_or(|(_, _, grow, ..)| now - planted < *grow)
                })
                .collect();
            let mut garden_obj = rpg
                .get("garden")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            garden_obj.insert("plots".into(), Value::Array(pending));
            garden_obj.entry("maxPlots").or_insert(serde_json::json!(MAX_PLOTS));
            rpg.insert("garden".into(), Value::Object(garden_obj));

            let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
            let new_exp = apply_exp(&mut rpg, old_exp, total_exp);

            Reply::text(format!(
                "HOREE! PANEN TIBA! 🚜🌾✨\n\n\
                 Hasil kerja kerasmu terbayarkan. Ini yang kamu dapet:\n\
                 {}\n\n\
                 📈 Bonus EXP Tani: *+{total_exp}*\n\n\
                 Buruan tanam lagi biar makin tajir! 💸",
                lines.join("\n")
            ))
            .with_reaction("✅")
            .with_mutation(Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
            })
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "garden".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("garden").unwrap().handler)(ctx)
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

    fn planted(crop: &str, planted_at: i64) -> Value {
        serde_json::json!({ "rpg": { "garden": { "plots": [{ "crop": crop, "plantedAt": planted_at }], "maxPlots": 3 } } })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("garden").unwrap();
        assert_eq!(p.meta.name, "garden");
        assert_eq!(p.meta.alias, ["kebun", "farm", "tanam"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Berkebun dan panen tanaman");
        assert_eq!(p.meta.usage, ".garden <plant/harvest/status>");
        assert_eq!(p.meta.example, ".garden plant carrot");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kebun", "farm", "tanam"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "garden");
        }
    }

    #[test]
    fn the_seed_format_counts_minutes_not_hours() {
        assert_eq!(format_time(0), "0s");
        assert_eq!(format_time(30_000), "30s");
        assert_eq!(format_time(300_000), "5m 0s");
        assert_eq!(format_time(7_200_000), "120m 0s", "tidak ada satuan jam");
        assert_eq!(format_time(-1), "0s");
    }

    #[test]
    fn an_unknown_action_shows_the_menu_with_every_seed() {
        for args in [vec![], vec!["water"], vec!["PLANT!"]] {
            let r = run(serde_json::json!({}), &args);
            let t = text_of(&r);
            assert!(t.starts_with("Halo Juragan Kebun!"), "args {args:?}: {t}");
            for (key, name, grow, _, sell, seed) in CROPS {
                assert!(t.contains(&format!("*{name}*")), "{key} hilang");
                assert!(t.contains(&format!("⏳ Waktu Tumbuh: {}", format_time(grow))), "{key}");
                assert!(t.contains(&format!("💰 Harga Jual: Rp {sell} | 🌱 Harga Bibit: Rp {seed}")), "{key}");
            }
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn buying_needs_a_seed_name_that_exists() {
        let r = run(serde_json::json!({ "koin": 9_999 }), &["buy"]);
        assert!(text_of(&r).starts_with("Hayo, mau beli bibit apaan?"));
        assert!(text_of(&r).contains("`.garden buy carrot 5`"));

        let r = run(serde_json::json!({ "koin": 9_999 }), &["buy", "nasi"]);
        assert!(text_of(&r).starts_with("Bibit itu nggak dijual di toko tani kita"));
    }

    /// `Math.max(1, parseInt(args[2]) || 1)` — 0, negatif, dan bukan-angka
    /// semuanya jadi satu.
    #[test]
    fn the_buy_quantity_never_drops_below_one() {
        assert_eq!(buy_qty(None), 1);
        assert_eq!(buy_qty(Some(&"0".into())), 1);
        assert_eq!(buy_qty(Some(&"-4".into())), 1);
        assert_eq!(buy_qty(Some(&"abc".into())), 1);
        assert_eq!(buy_qty(Some(&"7".into())), 7);
        assert_eq!(buy_qty(Some(&"3bushel".into())), 3, "parseInt memotong");
    }

    #[test]
    fn buying_costs_the_seed_price_per_unit_and_stacks_the_seeds() {
        // Wortel 10 per bibit, beli 4 = 40.
        let r = run(
            serde_json::json!({ "koin": 1_000, "inventory": { "carrotseed": 2 } }),
            &["buy", "carrot", "4"],
        );
        let t = text_of(&r);
        assert!(t.contains("Kamu beli *4x Bibit 🥕 Wortel*"), "{t}");
        assert!(t.contains("Total Bayar: *Rp 40*"), "{t}");
        let data = write_of(&r);
        assert_eq!(data["koin"], serde_json::json!(960));
        assert_eq!(data["inventory"]["carrotseed"], serde_json::json!(6));
    }

    #[test]
    fn a_too_small_purse_blocks_the_purchase() {
        let r = run(serde_json::json!({ "koin": 39 }), &["buy", "carrot", "4"]);
        let t = text_of(&r);
        assert!(t.starts_with("Eits, uang kamu kurang kak!"), "{t}");
        assert!(t.contains("Total belanjanya Rp 40"), "{t}");
        assert!(t.contains("koin kamu sisa Rp 39"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn planting_needs_a_seed_and_a_free_plot() {
        let r = run(serde_json::json!({}), &["plant", "carrot"]);
        let t = text_of(&r);
        assert!(t.contains("Kamu nggak punya bibit *🥕 Wortel*"), "{t}");
        assert!(t.contains("`.garden buy carrot`"), "{t}");
        assert!(r.mutations().is_empty());

        let r = run(serde_json::json!({}), &["plant"]);
        assert!(text_of(&r).starts_with("Tanah udah siap, tapi bibit apa"));

        let r = run(
            serde_json::json!({ "inventory": { "carrotseed": 1 } }),
            &["plant", "nasi"],
        );
        assert!(text_of(&r).starts_with("Tanaman itu nggak ada di buku panduan tani"));
    }

    #[test]
    fn a_full_garden_refuses_another_plot() {
        let now = crate::common::now_ms();
        let plots = vec![
            serde_json::json!({ "crop": "carrot", "plantedAt": now }),
            serde_json::json!({ "crop": "tomato", "plantedAt": now }),
            serde_json::json!({ "crop": "corn", "plantedAt": now }),
        ];
        let r = run(
            serde_json::json!({ "inventory": { "carrotseed": 1 },
                    "rpg": { "garden": { "plots": plots, "maxPlots": 3 } } }),
            &["plant", "carrot"],
        );
        assert!(text_of(&r).starts_with("Waduh, lahannya udah penuh semua!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn planting_consumes_one_seed_and_records_the_moment() {
        let r = run(
            serde_json::json!({ "inventory": { "carrotseed": 2 } }),
            &["plant", "carrot"],
        );
        let t = text_of(&r);
        assert!(t.starts_with("Sipp! Bibit *🥕 Wortel* udah ditanam"), "{t}");
        assert!(t.contains("tunggu *5m 0s*"), "{t}");
        let data = write_of(&r);
        assert_eq!(data["inventory"]["carrotseed"], serde_json::json!(1));
        let plots = data["rpg"]["garden"]["plots"].as_array().unwrap();
        assert_eq!(plots.len(), 1);
        assert_eq!(plots[0]["crop"], serde_json::json!("carrot"));
        assert!(plots[0]["plantedAt"].as_i64().unwrap() > 0);
        assert_eq!(data["rpg"]["garden"]["maxPlots"], serde_json::json!(3));
    }

    #[test]
    fn the_last_seed_leaves_the_bag_entirely() {
        let r = run(serde_json::json!({ "inventory": { "carrotseed": 1 } }), &["plant", "carrot"]);
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert!(!inv.contains_key("carrotseed"), "1 - 1 = 0 -> dihapus");
    }

    #[test]
    fn the_status_report_lists_every_plot_with_its_remaining_time() {
        let now = crate::common::now_ms();
        let r = run(
            planted("carrot", now - 400_000),
            &["status"],
        );
        let t = text_of(&r);
        assert!(t.starts_with("Cek Lahan Kebun... 🚜🌱"), "{t}");
        assert!(t.contains("*Kapasitas Tanah:* 1 dari 3 terisi."), "{t}");
        assert!(t.contains("📍 Plot 1: *🥕 Wortel*"), "{t}");
        assert!(t.contains("✨ SIAP PANEN! ✨"), "300.000 - 400.000 <= 0: {t}");
    }

    #[test]
    fn a_ready_plot_reads_as_harvestable() {
        let now = crate::common::now_ms();
        let t_reply = run(planted("carrot", now - 400_000), &["status"]);
        let t = text_of(&t_reply);
        assert!(t.contains("✨ SIAP PANEN! ✨"), "{t}");
    }

    #[test]
    fn an_empty_garden_says_it_is_bare() {
        let r = run(serde_json::json!({ "rpg": { "garden": { "plots": [], "maxPlots": 3 } } }), &["status"]);
        let t = text_of(&r);
        assert!(t.contains("*Kapasitas Tanah:* 0 dari 3 terisi."), "{t}");
        assert!(t.contains("kebun kamu masih gersang kak!"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn harvesting_nothing_ready_is_refused() {
        let r = run(planted("pumpkin", crate::common::now_ms()), &["harvest"]);
        let t = text_of(&r);
        assert!(t.starts_with("Hadeh, belum ada yang mateng kak!"), "{t}");
        assert!(t.contains("`.garden status`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_harvest_yields_two_to_four_and_clears_the_plot() {
        let now = crate::common::now_ms();
        let mut harvested = false;
        for _ in 0..50 {
            let r = run(
                serde_json::json!({ "rpg": { "garden": {
                    "plots": [{ "crop": "carrot", "plantedAt": now - 400_000 },
                              { "crop": "corn", "plantedAt": now }],
                    "maxPlots": 3 } } }),
                &["harvest"],
            );
            let t = text_of(&r);
            if !t.starts_with("HOREE! PANEN TIBA!") {
                continue;
            }
            harvested = true;
            let data = write_of(&r);
            assert!(t.contains("• 🥕 Wortel x"), "{t}");
            let qty = data["inventory"]["carrot"].as_i64().unwrap();
            assert!((2..=4).contains(&qty), "qty {qty}");
            // Wortel exp 50, satu kali per petak — bukan dikali qty.
            assert_eq!(data["exp"], serde_json::json!(50));
            assert!(t.contains("📈 Bonus EXP Tani: *+50*"), "{t}");
            let plots = data["rpg"]["garden"]["plots"].as_array().unwrap();
            assert_eq!(plots.len(), 1, "hanya petak matang yang dicabut");
            assert_eq!(plots[0]["crop"], serde_json::json!("corn"));
        }
        assert!(harvested, "dalam 50 percobaan harus ada petak matang");
    }

    #[test]
    fn an_harvested_stack_grows_rather_than_resets() {
        let now = crate::common::now_ms();
        let mut seen = false;
        for _ in 0..50 {
            let r = run(
                serde_json::json!({ "inventory": { "carrot": 10 },
                        "rpg": { "garden": { "plots": [{ "crop": "carrot", "plantedAt": now - 400_000 }],
                                            "maxPlots": 3 } } }),
                &["harvest"],
            );
            if !text_of(&r).starts_with("HOREE!") {
                continue;
            }
            seen = true;
            let qty = write_of(&r)["inventory"]["carrot"].as_i64().unwrap();
            assert!((12..=14).contains(&qty), "10 + 2..4 = {qty}");
        }
        assert!(seen);
    }

    #[test]
    fn the_crop_table_keeps_every_js_row() {
        assert_eq!(CROPS.len(), 8);
        assert_eq!(CROPS[0], ("carrot", "🥕 Wortel", 300_000, 50, 30, 10));
        assert_eq!(CROPS[7], ("pumpkin", "🎃 Labu", 7_200_000, 500, 500, 150));
        assert!(crop("tidak-ada").is_none());
        assert_eq!(seed_key("carrot"), "carrotseed");
    }
}