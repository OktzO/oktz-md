//! `plugins/rpg/beg.js` — mengemis: 4 dari 7 hasil memberi uang, satu
//! merampok balik.
//!
//! Cabang rugi (`money < 0`) TIDAK memanggil `addExpWithLevelCheck`, jadi exp
//! sama sekali tak tersentuh — hanya `koin` yang dijepit di 0. Cabang nihil
//! (`money === 0`) memanggil `db.save()` tanpa perubahan apa pun.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::heal::{apply_exp, or_default, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

/// `(success, money, exp, msg)` — urutan object literal = bobot undian.
const RESPONSES: [(bool, i64, i64, &str); 7] = [
    (true, 500, 10, "Seorang dermawan memberikanmu uang!"),
    (true, 1000, 20, "Kamu dapat tips dari orang baik!"),
    (true, 2000, 50, "WOW! Ada sultan yang kasihan!"),
    (false, 0, 0, "Tidak ada yang peduli..."),
    (false, 0, 0, "Orang-orang mengabaikanmu..."),
    (true, 100, 5, "Dapat receh dari kantong orang!"),
    (false, -500, 0, "Kamu malah dirampok pengemis lain!"),
];

fn pick() -> (bool, i64, i64, &'static str) {
    RESPONSES[crate::common::random_index(RESPONSES.len())]
}

#[plugin(
    name = "beg",
    alias = ["ngemis"],
    category = "rpg",
    description = "Mengemis untuk mendapatkan uang receh",
    usage = ".beg",
    example = ".beg",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn beg(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let (_success, money, exp, msg) = pick();
    let koin = crate::common::int_field(&ctx, "koin");

    let text = if money > 0 {
        let mut t = format!(
            "🙏 *ɴɢᴇᴍɪs sᴜᴋsᴇs*\n\n> {msg}\n> 💰 Dapat: *+Rp {}*",
            thousands_id(money as f64)
        );
        if exp > 0 {
            t.push_str(&format!("\n> 🚄 Exp: *+{exp}*"));
        }
        t
    } else if money < 0 {
        format!(
            "😭 *ɴɢᴇᴍɪs ɢᴀɢᴀʟ*\n\n> {msg}\n> 💸 Lost: *Rp {}*",
            thousands_id(money.unsigned_abs() as f64)
        )
    } else {
        format!("😢 *ɴɢᴇᴍɪs ɢᴀɢᴀʟ*\n\n> {msg}")
    };

    let data = if money > 0 {
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        serde_json::json!({ "koin": koin + money, "exp": apply_exp(&mut rpg, old_exp, exp), "rpg": rpg })
    } else if money < 0 {
        serde_json::json!({ "koin": (koin + money).max(0) })
    } else {
        // `db.save()` tanpa perubahan — tak ada yang perlu ditulis.
        return Reply::text(text);
    };

    Reply::text(text).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "beg".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("beg").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
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
        let p = oktz_plugin_api::find("beg").unwrap();
        assert_eq!(p.meta.name, "beg");
        assert_eq!(p.meta.alias, ["ngemis"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Mengemis untuk mendapatkan uang receh"
        );
        assert_eq!(p.meta.usage, ".beg");
        assert_eq!(p.meta.example, ".beg");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `ngemis` juga nama `plugins/rpg/ngemis.js`; nama didahulukan
        // (`plugins.js:539`), jadi alias `ngemis` bukan milik beg.
        assert_eq!(oktz_plugin_api::find("ngemis").unwrap().meta.name, "ngemis");
    }

    #[test]
    fn the_outcome_table_keeps_every_js_row() {
        assert_eq!(RESPONSES.len(), 7);
        assert_eq!(
            RESPONSES[0],
            (true, 500, 10, "Seorang dermawan memberikanmu uang!")
        );
        assert_eq!(
            RESPONSES[5],
            (true, 100, 5, "Dapat receh dari kantong orang!")
        );
        assert_eq!(RESPONSES[6].1, -500);
    }

    #[test]
    fn every_pull_lands_in_one_of_the_three_branches() {
        let (mut win, mut robbed, mut nothing) = (0, 0, 0);
        for _ in 0..900 {
            let r = run(serde_json::json!({ "koin": 1_000, "exp": 0 }));
            let t = text_of(&r);
            assert!(
                t.starts_with("🙏 *ɴɢᴇᴍɪs sᴜᴋsᴇs*")
                    || t.starts_with("😭 *ɴɢᴇᴍɪs ɢᴀɢᴀʟ*")
                    || t.starts_with("😢 *ɴɢᴇᴍɪs ɢᴀɢᴀʟ*"),
                "cabang di luar tabel: {t}"
            );
            if t.starts_with("🙏") {
                win += 1;
                assert!(t.contains("> 🚄 Exp: *+"), "exp selalu > 0 di cabang menang: {t}");
                assert_eq!(r.mutations().len(), 1);
                assert!(t.contains("> 💰 Dapat: *+Rp "), "{t}");
            } else if t.contains("💸 Lost:") {
                robbed += 1;
                assert_eq!(write_of(&r)["koin"], serde_json::json!(500), "1.000 - 500");
                assert!(
                    write_of(&r).get("exp").is_none(),
                    "cabang rugi tidak memanggil addExpWithLevelCheck"
                );
            } else {
                nothing += 1;
                assert!(
                    r.mutations().is_empty(),
                    "tak ada yang dapat -> tak ada yang ditulis"
                );
            }
        }
        assert!(win > 0 && robbed > 0 && nothing > 0, "win={win} robbed={robbed} nothing={nothing}");
    }

    #[test]
    fn a_win_pays_exactly_one_of_the_three_paying_rows() {
        // (koin, exp) dari tabel: (500, 10), (1000, 20), (2000, 50), (100, 5).
        let allowed = [(100, 5), (500, 10), (1000, 20), (2000, 50)];
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..900 {
            let r = run(serde_json::json!({ "koin": 0, "exp": 0 }));
            if !text_of(&r).starts_with("🙏") {
                continue;
            }
            let data = write_of(&r);
            let pair = (data["koin"].as_i64().unwrap(), data["exp"].as_i64().unwrap());
            assert!(allowed.contains(&pair), "pasangan (koin, exp) di luar tabel: {pair:?}");
            seen.insert(pair);
        }
        assert!(seen.len() >= 3, "hanya terlihat {seen:?}");
    }

    #[test]
    fn a_robbery_never_drives_the_purse_below_zero() {
        for _ in 0..900 {
            let r = run(serde_json::json!({ "koin": 100 }));
            if text_of(&r).contains("💸 Lost:") {
                assert_eq!(write_of(&r)["koin"], serde_json::json!(0), "Math.max(0, 100 - 500)");
            }
        }
    }

    #[test]
    fn a_missing_user_row_never_panics() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "koin": null })] {
            let r = run(user.clone());
            assert!(!text_of(&r).is_empty(), "input {user}");
        }
    }
}