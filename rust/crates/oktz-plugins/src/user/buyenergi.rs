//! `plugins/user/buyenergi.js` — tukar koin jadi energi (1 energi = 100 koin).
//!
//! `ponytail:` `config.energi.premium` tidak dibaca. Default-nya `99999999`,
//! bukan `-1`, jadi `db.updateEnergi` tidak pernah mengembalikan unlimited ke
//! premium; hanya owner (`config.energi.owner = -1`) yang dapat, dan itu
//! dicerminkan di `unlimited_energi()`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::{int_field, parse_int};
use crate::user::energi::UNLIMITED;
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const PRICE_PER_ENERGI: i64 = 100;

/// `MAX_KOIN` (`database.js:650`) — ceiling `db.updateKoin`.
const MAX_KOIN: i64 = 9_000_000_000_000;

/// `formatNumber` (`buyenergi.js:20-22`) — pemisah ribuan titik. `thousands_id`
/// membuang tanda minus, jadi `-1` (koin tak terbatas) dirakit ulang di sini.
fn format_number(n: i64) -> String {
    if n < 0 {
        format!("-{}", thousands_id(n as f64))
    } else {
        thousands_id(n as f64)
    }
}

/// `config.energi.owner === -1` + `config.isOwner(jid)` di `updateEnergi`
/// (`database.js:632-638`). Premium tidak masuk: default-nya bukan `-1`.
fn unlimited_energi(ctx: &Ctx) -> bool {
    ctx.is_owner
}

/// `m.reply(...)` untuk `amount <= 0` (`buyenergi.js:31-39`).
fn help(ctx: &Ctx, koin: i64) -> String {
    format!(
        "🛒 *ʙᴜʏ ᴇɴᴇʀɢɪ*\n\n\
         ╭┈┈⬡「 💰 *ɪɴꜰᴏ* 」\n\
         ┃ 💵 ʜᴀʀɢᴀ: *{PRICE_PER_ENERGI}* koin/energi\n\
         ┃ 💰 ᴋᴏɪɴ ᴋᴀᴍᴜ: *{}*\n\
         ╰┈┈⬡\n\n\
         > Gunakan: `.buyenergi <jumlah>`\n\n\
         `Contoh: {}buyenergi 10`",
        format_number(koin),
        ctx.prefix
    )
}

/// `m.reply(...)` saat koin kurang (`buyenergi.js:46-52`).
fn insufficient(total: i64, koin: i64) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n\
         > Koin tidak cukup!\n\
         > Butuh: *{}*\n\
         > Kamu punya: *{}*",
        format_number(total),
        format_number(koin)
    )
}

/// `m.reply(...)` untuk `user.energi === -1` (`buyenergi.js:57-63`).
///
/// `ponytail:` teks JS menjanjikan "Koin dikembalikan" tapi tidak ada
/// pengembalian — `db.updateKoin` sudah dijalankan baris sebelumnya. Bug itu
/// ikut dip-port apa adanya; perbaikannya berarti mengubah store, bukan
/// cuma teks, jadi harus lewat keputusan-porting terpisah.
const UNLIMITED_BALASAN: &str = "✅ *ᴘᴇᴍʙᴇʟɪᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n\
     > Tapi kamu sudah punya unlimited energi!\n\
     > Koin dikembalikan.";

/// `m.reply(...)` setelah pembelian (`buyenergi.js:70-80`).
fn success(amount: i64, total: i64, new_energi: i64, new_koin: i64) -> String {
    format!(
        "✅ *ᴘᴇᴍʙᴇʟɪᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ ⚡ ᴇɴᴇʀɢɪ: *+{}*\n\
         ┃ 💵 ʜᴀʀɢᴀ: *-{}* koin\n\
         ╰┈┈⬡\n\n\
         ╭┈┈⬡「 💰 *sᴀʟᴅᴏ* 」\n\
         ┃ ⚡ ᴇɴᴇʀɢɪ: *{}*\n\
         ┃ 💰 ᴋᴏɪɴ: *{}*\n\
         ╰┈┈⬡",
        format_number(amount),
        format_number(total),
        format_number(new_energi),
        format_number(new_koin)
    )
}

#[plugin(
    name = "buyenergi",
    alias = ["belienergi", "purchaseenergi", "buyenergy"],
    category = "user",
    description = "Beli energi dengan koin (1 energi = 100 koin)",
    usage = ".buyenergi <jumlah>",
    example = ".buyenergi 10",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn buyenergi(ctx: Ctx) -> Reply {
    let amount = ctx.args.first().and_then(|a| parse_int(a)).unwrap_or(0);
    let koin = int_field(&ctx, "koin");

    if amount <= 0 {
        return Reply::text(help(&ctx, koin));
    }

    let total = amount.saturating_mul(PRICE_PER_ENERGI);
    if koin < total {
        return Reply::text(insufficient(total, koin));
    }

    let new_koin = (koin - total).clamp(0, MAX_KOIN);
    let mut data = Map::new();
    data.insert("koin".into(), json!(new_koin));
    let write = |data: Map<String, Value>| Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: Value::Object(data),
    };

    if int_field(&ctx, "energi") == UNLIMITED {
        return Reply::text(UNLIMITED_BALASAN)
            .with_reaction("✅")
            .with_mutation(write(data));
    }

    // `db.updateEnergi` mengembalikan -1 tanpa menyentuh store kalau owner;
    // jadi energi tidak ikut ditulis di data.
    let new_energi = if unlimited_energi(&ctx) {
        UNLIMITED
    } else {
        let new = (int_field(&ctx, "energi") + amount).max(0);
        data.insert("energi".into(), json!(new));
        new
    };

    Reply::text(success(amount, total, new_energi, new_koin))
        .with_reaction("✅")
        .with_mutation(write(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], user: Value, is_owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buyenergi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            is_owner,
            ..Default::default()
        };
        (oktz_plugin_api::find("buyenergi").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    fn write_of(r: &Reply) -> Option<Value> {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu SetUser");
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                Some(data.clone())
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    /// `true` kalau reply ini bereaksi `✅` lalu mengirim teks — persis urutan
    /// `m.react('✅')` + `m.reply(...)` di JS.
    fn is_check(r: &Reply) -> bool {
        let mut cur = r.without_mutations();
        let mut seen = false;
        while let Reply::Reaction { emoji, reply } = cur {
            assert_eq!(emoji, "✅", "reaksi sebelum balasan");
            seen = true;
            cur = reply;
        }
        seen && matches!(cur, Reply::Text { .. })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("buyenergi").unwrap();
        assert_eq!(p.meta.name, "buyenergi");
        assert_eq!(
            p.meta.alias,
            ["belienergi", "purchaseenergi", "buyenergy"]
        );
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Beli energi dengan koin (1 energi = 100 koin)");
        assert_eq!(p.meta.usage, ".buyenergi <jumlah>");
        assert_eq!(p.meta.example, ".buyenergi 10");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["buyenergi", "belienergi", "purchaseenergi", "buyenergy"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("buyenergi"),
                "key {key} salah resolve"
            );
        }
    }

    // -- format angka ------------------------------------------------------

    #[test]
    fn format_number_matches_the_js_regex() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1_000), "1.000");
        assert_eq!(format_number(1_000_000), "1.000.000");
        assert_eq!(format_number(-1), "-1", "koin tak terbatas");
    }

    // -- cabang help -------------------------------------------------------

    #[test]
    fn no_argument_shows_the_price_and_the_balance_without_touching_the_store() {
        let r = run(&[], json!({"koin": 12500}), false);
        let out = text_of(&r);
        assert!(out.starts_with("🛒 *ʙᴜʏ ᴇɴᴇʀɢɪ*\n\n╭┈┈⬡「 💰 *ɪɴꜰᴏ* 」\n"));
        assert!(out.contains("┃ 💵 ʜᴀʀɢᴀ: *100* koin/energi\n"));
        assert!(out.contains("┃ 💰 ᴋᴏɪɴ ᴋᴀᴍᴜ: *12.500*\n"));
        assert!(out.ends_with("> Gunakan: `.buyenergi <jumlah>`\n\n`Contoh: .buyenergi 10`"));
        assert!(r.mutations().is_empty(), "harga belum dibeli");
        assert!(!is_check(&r), "reaksi hanya setelah transaksi");
    }

    #[test]
    fn a_missing_user_row_reads_zero_koin() {
        for user in [json!(null), json!({}), json!({"koin": null})] {
            let out = text_of(&run(&[], user.clone(), false));
            assert!(out.contains("┃ 💰 ᴋᴏɪɴ ᴋᴀᴍᴜ: *0*\n"), "{user}");
        }
    }

    #[test]
    fn parse_int_semantics_decide_the_branch() {
        // `parseInt('10abc')` = 10, jadi ini bukan cabang help.
        let bought = run(&["10abc"], json!({"koin": 5_000, "energi": 0}), false);
        assert!(text_of(&bought).contains("┃ ⚡ ᴇɴᴇʀɢɪ: *+10*"));
        // `parseInt('abc')` = NaN → `|| 0` → help.
        assert!(text_of(&run(&["abc"], json!({"koin": 5_000}), false)).starts_with("🛒"));
        assert!(text_of(&run(&["0"], json!({"koin": 5_000}), false)).starts_with("🛒"));
        assert!(text_of(&run(&["-5"], json!({"koin": 5_000}), false)).starts_with("🛒"));
    }

    // -- cabang gagal ------------------------------------------------------

    #[test]
    fn an_underfunded_purchase_lists_both_sides_without_writing() {
        let r = run(&["10"], json!({"koin": 500, "energi": 3}), false);
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n> Koin tidak cukup!\n> Butuh: *1.000*\n> Kamu punya: *500*"
        );
        assert!(r.mutations().is_empty());
        assert!(!is_check(&r));
    }

    #[test]
    fn exactly_enough_koin_is_enough_not_one_more() {
        // 1000 koin = tepat 10 energi; sisa 0 berarti energi tidak negatif.
        let r = run(&["10"], json!({"koin": 1_000, "energi": 0}), false);
        let data = write_of(&r).expect("harus menulis");
        assert_eq!(data["koin"], json!(0));
        assert_eq!(data["energi"], json!(10));
    }

    #[test]
    fn one_koin_short_falls_into_the_gagal_branch() {
        assert!(text_of(&run(&["10"], json!({"koin": 999}), false)).starts_with("❌ *ɢᴀɢᴀʟ*"));
    }

    #[test]
    fn an_unlimited_koin_balance_reads_as_minus_one_and_looks_broke() {
        // `user.koin === -1` selalu `< totalPrice`, jadi plugin menolak meski
        // saldo tak terbatas — sama seperti JS.
        let r = run(&["10"], json!({"koin": -1, "energi": 0}), false);
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n> Koin tidak cukup!\n> Butuh: *1.000*\n> Kamu punya: *-1*"
        );
        assert!(r.mutations().is_empty());
    }

    // -- cabang berhasil ---------------------------------------------------

    #[test]
    fn a_successful_purchase_moves_both_koin_and_energi() {
        let r = run(&["10"], json!({"koin": 9_000, "energi": 12}), false);
        assert_eq!(
            text_of(&r),
            concat!(
                "✅ *ᴘᴇᴍʙᴇʟɪᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n",
                "╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n",
                "┃ ⚡ ᴇɴᴇʀɢɪ: *+10*\n",
                "┃ 💵 ʜᴀʀɢᴀ: *-1.000* koin\n",
                "╰┈┈⬡\n\n",
                "╭┈┈⬡「 💰 *sᴀʟᴅᴏ* 」\n",
                "┃ ⚡ ᴇɴᴇʀɢɪ: *22*\n",
                "┃ 💰 ᴋᴏɪɴ: *8.000*\n",
                "╰┈┈⬡"
            )
        );
        let data = write_of(&r).expect("harus menulis");
        assert_eq!(data["koin"], json!(8_000));
        assert_eq!(data["energi"], json!(22));
        assert!(is_check(&r), "JS bereaksi ✅ setelah updateEnergi");
    }

    #[test]
    fn stored_energi_is_added_to_the_bought_amount() {
        let r = run(&["7"], json!({"koin": 700, "energi": 100_000}), false);
        assert!(text_of(&r).contains("┃ ⚡ ᴇɴᴇʀɢɪ: *100.007*"));
        assert_eq!(write_of(&r).unwrap()["energi"], json!(100_007));
    }

    #[test]
    fn an_unlimited_energi_user_is_refused_but_pays_anyway() {
        // Bug JS yang ikut dip-port: `db.updateKoin` jalan sebelum cek
        // unlimited dan tidak ada pengembalian, meski teks bilang dikembalikan.
        let r = run(&["5"], json!({"koin": 5_000, "energi": -1}), false);
        assert_eq!(
            text_of(&r),
            "✅ *ᴘᴇᴍʙᴇʟɪᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n> Tapi kamu sudah punya unlimited energi!\n> Koin dikembalikan."
        );
        let data = write_of(&r).expect("koin tetap dipotong");
        assert_eq!(data["koin"], json!(4_500));
        assert!(data.get("energi").is_none(), "energi tak terbatas tak diubah");
        assert!(is_check(&r));
    }

    #[test]
    fn an_owner_gets_unlimited_energi_without_the_store_write() {
        // `config.energi.owner = -1` → `db.updateEnergi` return -1, tak setUser.
        let r = run(&["3"], json!({"koin": 300, "energi": 4}), true);
        assert!(text_of(&r).contains("┃ ⚡ ᴇɴᴇʀɢɪ: *-1*\n"), "formatNumber(-1)");
        assert!(text_of(&r).contains("┃ 💰 ᴋᴏɪɴ: *0*\n"));
        let data = write_of(&r).unwrap();
        assert_eq!(data["koin"], json!(0));
        assert!(data.get("energi").is_none());
    }

    #[test]
    fn a_premium_user_is_not_unlimited_by_default() {
        // `config.energi.premium = 99999999`, bukan -1, jadi premium tetap
        // mengikuti saldo energinya sendiri.
        let ctx = Ctx {
            prefix: ".".into(),
            command: "buyenergi".into(),
            sender: "628111@s.whatsapp.net".into(),
            args: vec!["10".into()],
            user_data: json!({"koin": 5_000, "energi": 1, "isPremium": true}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("buyenergi").unwrap().handler)(ctx);
        assert_eq!(write_of(&r).unwrap()["energi"], json!(11));
    }
}