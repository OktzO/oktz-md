//! `plugins/user/buyfitur.js` — beli fitur premium dengan koin (3000/fitur).
//!
//! `ponytail:` `config.isPremium(m.sender)` TIDAK di-port: itu memanggil
//! `config.js`, bukan store, dan `Ctx` hanya membawa `is_owner` (kepemilikan
//! bot). Jadi hanya pemeriksaan `user.isPremium` yang tersisa — user premium
//! yang secondhand diabaikan, dan angka koinnya tidak terpakai.
//!
//! `db.updateKoin(jid, -3000)` tidak ada di `Mutation`; `SetUser` menulis
//! objek partial dengan operator `??` di JS, jadi `koin` yang dikirim di sini
//! harus berupa ANGKA LENGAP, bukan `{"$dec": 3000}` — kalau tidak, field itu
//! tidak akan pernah berkurang.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const PRICE: i64 = 3000;

/// `PREMIUM_FEATURES` — `(id, name, desc)`.
const FEATURES: [(&str, &str, &str); 5] = [
    ("sticker", "Sticker Unlimited", "Unlimited sticker commands"),
    ("downloader", "Downloader Pro", "Download tanpa limit"),
    ("ai", "AI Access", "Akses fitur AI premium"),
    ("tools", "Advanced Tools", "Tools eksklusif"),
    ("game", "Game Bonus", "2x rewards game"),
];

/// `num.toString().replace(/\B(?=(\d{3})+(?!\d))/g, '.')` — pemisah ribuan
/// titik. Bentuknya sama dengan `beli.rs` `rupiah`, tapi tanpa awalan `Rp`.
fn dots(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

const ALREADY_PREMIUM: &str = "✨ *ᴘʀᴇᴍɪᴜᴍ ᴅsᴇʀ*\n\n\
     > Kamu sudah premium!\n\
     > Semua fitur sudah ter-unlock!";

fn menu(koin: i64, unlocked: &[String]) -> String {
    let mut text = String::from("╭━━━━━━━━━━━━━━━━━╮\n┃  🛒 *ʙᴜʏ ꜰɪᴛᴜʀ*\n╰━━━━━━━━━━━━━━━━━╯\n\n");
    text.push_str(&format!(
        "> Harga: *{}* bal/fitur\n> Koin: *{}*\n\n",
        dots(PRICE),
        dots(koin)
    ));
    text.push_str("╭┈┈⬡「 📋 *ꜰɪᴛᴜʀ* 」\n");
    for (id, name, desc) in FEATURES {
        let on = unlocked.iter().any(|u| u == id);
        text.push_str(&format!(
            "┃ {} *{name}*\n┃    _{desc}_\n┃    ID: `{id}`\n┃\n",
            if on { "✅" } else { "🔒" }
        ));
    }
    text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n> Gunakan: `.buyfitur <id>`\n> Atau jadi *Premium* unlock semua!");
    text
}

fn not_found(feature_name: &str) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n> Fitur `{feature_name}` tidak ditemukan\n\
         > Ketik `.buyfitur` untuk lihat daftar"
    )
}

const ALREADY_UNLOCKED: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Fitur `{}` sudah ter-unlock!";

fn not_enough(koin: i64) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n\
         > Koin tidak cukup!\n\
         > Butuh: *{}*\n\
         > Kamu punya: *{}*",
        dots(PRICE),
        dots(koin)
    )
}

fn unlocked_msg(name: &str, desc: &str, left: i64) -> String {
    format!(
        "✅ *ꜰɪᴛᴜʀ ᴅɪ-ᴜɴʟᴏᴄᴋ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 🎁 ꜰɪᴛᴜʀ: *{name}*\n\
         ┃ 💵 ʜᴀʀɢᴀ: *-{}* bal\n\
         ┃ 💰 sɪsᴀ: *{}*\n\
         ╰┈┈⬡\n\n\
         > _{desc}_\n\n\
         > 💡 Tip: Jadi *Premium* untuk unlock SEMUA!",
        dots(PRICE),
        dots(left)
    )
}

#[plugin(
    name = "buyfitur",
    alias = ["belifitur", "purchasefeature", "buyfeature"],
    category = "user",
    description = "Beli fitur premium (1 fitur = 3000 koin)",
    usage = ".buyfitur [nama_fitur]",
    example = ".buyfitur",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn buyfitur(ctx: Ctx) -> Reply {
    let is_premium = crate::common::js_truthy(ctx.user_field("isPremium"));
    if is_premium {
        return Reply::text(ALREADY_PREMIUM);
    }

    let koin = ctx
        .user_field("koin")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let mut unlocked = crate::common::string_list(ctx.user_field("unlockedFeatures"));

    // `m.args[0]?.toLowerCase()` — bukan `ctx.text`, supaya kata kedua tidak
    // ikut jadi id fitur.
    let feature_name = ctx.args.first().map(|a| a.to_lowercase());

    let Some(feature_name) = feature_name else {
        return Reply::text(menu(koin, &unlocked));
    };

    let Some((id, name, desc)) = FEATURES.iter().find(|(fid, ..)| *fid == feature_name) else {
        return Reply::text(not_found(&feature_name));
    };

    if unlocked.iter().any(|u| u == id) {
        return Reply::text(ALREADY_UNLOCKED.replace("{}", name));
    }
    if koin < PRICE {
        return Reply::text(not_enough(koin));
    }

    unlocked.push((*id).to_string());
    Reply::text(unlocked_msg(name, desc, koin - PRICE))
        .with_reaction("✅")
        .mutating(vec![
            Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({"koin": koin - PRICE}),
            },
            Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: serde_json::json!({"unlockedFeatures": unlocked}),
            },
        ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buyfitur".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("buyfitur").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn user_writes(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 2, "koin + unlockedFeatures");
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, data } => {
                    assert_eq!(jid, "628111@s.whatsapp.net");
                    data.clone()
                }
                other => panic!("harus SetUser, dapat {other:?}"),
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("buyfitur").unwrap();
        assert_eq!(p.meta.name, "buyfitur");
        assert_eq!(
            p.meta.alias,
            ["belifitur", "purchasefeature", "buyfeature"]
        );
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Beli fitur premium (1 fitur = 3000 koin)");
        assert_eq!(p.meta.usage, ".buyfitur [nama_fitur]");
        assert_eq!(p.meta.example, ".buyfitur");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn all_three_aliases_resolve_back_to_this_plugin() {
        for a in ["belifitur", "purchasefeature", "buyfeature"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("buyfitur")
            );
        }
    }

    #[test]
    fn a_premium_user_is_turned_away_even_when_naming_a_feature() {
        for args in [vec![""], vec!["sticker"]] {
            let r = run(&args, serde_json::json!({"isPremium": true, "koin": 99999}));
            assert_eq!(text_of(&r), ALREADY_PREMIUM, "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn without_an_argument_the_menu_lists_every_feature_locked() {
        let r = run(&[], serde_json::json!({"koin": 5000}));
        let t = text_of(&r);
        assert!(t.starts_with("╭━━━━━━━━━━━━━━━━━╮"), "{t}");
        assert!(t.contains("> Harga: *3.000* bal/fitur"), "{t}");
        assert!(t.contains("> Koin: *5.000*"), "{t}");
        for (id, name, desc) in FEATURES {
            assert!(t.contains(&format!("┃ 🔒 *{name}*")), "{id}: {t}");
            assert!(t.contains(&format!("┃    _{desc}_")), "{id}: {t}");
            assert!(t.contains(&format!("┃    ID: `{id}`")), "{id}: {t}");
        }
        assert!(t.contains("> Gunakan: `.buyfitur <id>`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unlocked_feature_shows_a_checkmark() {
        let r = run(&[], serde_json::json!({"koin": 0, "unlockedFeatures": ["ai", "game"]}));
        let t = text_of(&r);
        assert!(t.contains("┃ ✅ *AI Access*"), "{t}");
        assert!(t.contains("┃ ✅ *Game Bonus*"), "{t}");
        assert!(t.contains("┃ 🔒 *Sticker Unlimited*"), "{t}");
    }

    #[test]
    fn a_missing_koin_field_reads_as_zero() {
        let r = run(&[], serde_json::json!({}));
        assert!(text_of(&r).contains("> Koin: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unknown_feature_id_is_reported_and_nothing_is_charged() {
        // Balasan JS memakai `featureName` yang sudah `toLowerCase()`, jadi
        // `STICKERX` dibalas `stickerx`.
        for (input, echoed) in [
            ("gakada", "gakada"),
            ("sticker2", "sticker2"),
            ("STICKERX", "stickerx"),
        ] {
            let r = run(&[input], serde_json::json!({"koin": 99999}));
            let t = text_of(&r);
            assert!(t.starts_with("❌ *ɢᴀɢᴀʟ*"), "input {input}: {t}");
            assert!(t.contains(&format!("> Fitur `{echoed}` tidak ditemukan")), "{t}");
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn the_feature_id_is_matched_case_insensitively() {
        let r = run(&["TOOLS"], serde_json::json!({"koin": 3000}));
        let t = text_of(&r);
        assert!(t.contains("┃ 🎁 ꜰɪᴛᴜʀ: *Advanced Tools*"), "{t}");
        assert_eq!(user_writes(&r)[1]["unlockedFeatures"], serde_json::json!(["tools"]));
    }

    #[test]
    fn an_already_unlocked_feature_is_refused() {
        let r = run(&["ai"], serde_json::json!({"koin": 99999, "unlockedFeatures": ["ai"]}));
        assert_eq!(text_of(&r), ALREADY_UNLOCKED.replace("{}", "AI Access"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn insufficient_koin_reports_both_the_price_and_the_balance() {
        for koin in [0, 2999] {
            let r = run(&["sticker"], serde_json::json!({"koin": koin}));
            let t = text_of(&r);
            assert!(t.starts_with("❌ *ɢᴀɢᴀʟ*"), "{t}");
            assert!(t.contains("> Butuh: *3.000*"), "{t}");
            assert!(t.contains(&format!("> Kamu punya: *{}*", dots(koin))), "{t}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn the_exact_price_is_enough() {
        let r = run(&["sticker"], serde_json::json!({"koin": 3000}));
        let t = text_of(&r);
        assert!(t.starts_with("✅ *ꜰɪᴛᴜʀ ᴅɪ-ᴜɴʟᴏᴄᴋ*"), "{t}");
        assert!(t.contains("┃ 🎁 ꜰɪᴛᴜʀ: *Sticker Unlimited*"), "{t}");
        assert!(t.contains("┃ 💵 ʜᴀʀɢᴀ: *-3.000* bal"), "{t}");
        assert!(t.contains("┃ 💰 sɪsᴀ: *0*"), "{t}");
        assert!(t.contains("> _Unlimited sticker commands_"), "{t}");
    }

    #[test]
    fn a_purchase_charges_koin_and_appends_the_feature_id() {
        let r = run(&["game"], serde_json::json!({"koin": 12500, "unlockedFeatures": ["ai"]}));
        assert!(text_of(&r).contains("┃ 💰 sɪsᴀ: *9.500*"), "{}", text_of(&r));
        let w = user_writes(&r);
        assert_eq!(
            w[0]["koin"],
            serde_json::json!(9500),
            "koin harus ANGKA LENGAP — SetUser menimpa, bukan menjumlahkan"
        );
        assert_eq!(w[1]["unlockedFeatures"], serde_json::json!(["ai", "game"]));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_thousand_separator_matches_the_js_regex() {
        assert_eq!(dots(0), "0");
        assert_eq!(dots(7), "7");
        assert_eq!(dots(999), "999");
        assert_eq!(dots(1000), "1.000");
        assert_eq!(dots(3000), "3.000");
        assert_eq!(dots(123456789), "123.456.789");
    }

    #[test]
    fn only_the_first_argument_is_read_as_the_feature_id() {
        // `m.args[0]?.toLowerCase()` — `lagi` diabaikan, `ai` yang dibeli.
        let r = run(&["ai", "lagi"], serde_json::json!({"koin": 99999}));
        let t = text_of(&r);
        assert!(t.contains("┃ 🎁 ꜰɪᴛᴜʀ: *AI Access*"), "{t}");
        assert_eq!(user_writes(&r)[1]["unlockedFeatures"], serde_json::json!(["ai"]));
    }
}
