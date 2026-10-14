//! `plugins/owner/addlimitall.js` — tambah energi ke semua member grup.
//!
//! Nama plugin di JS adalah `addenergiall`, bukan nama file — dicerminkan di
//! `#[plugin(name = ...)]` supaya router tetap menemukan command yang sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::parse_int;

/// `config.energi.default` (`config.js:114`).
const DEFAULT_ENERGI: i64 = 99_999;

fn bad_input(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> Masukkan jumlah limit yang ingin ditambahkan.\n\n`Contoh: {p}addlimitall 50`",
        p = prefix
    )
}

pub(crate) const NO_MEMBER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada member di grup ini";

/// Energi member = `max(0, lama + amount)`, dengan `-1` (unlimited) tidak
/// pernah disentuh — `db.updateEnergi` mengembalikan `-1` sebelum mengubah
/// apa pun (`database.js:630`).
fn next_energy(current: i64, amount: i64) -> i64 {
    if current == -1 {
        -1
    } else {
        (current + amount).max(0)
    }
}

#[plugin(
    name = "addenergiall",
    alias = ["addenergianall", "bonusenergiall"],
    category = "owner",
    description = "Menambahkan limit/energi ke semua member grup",
    usage = ".addenergiall <jumlah>",
    example = ".addenergiall 50",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addlimitall(ctx: Ctx) -> Reply {
    // `parseInt(m.args[0])` — tanpa argumen `undefined` jadi `NaN`.
    let amount = ctx
        .args
        .first()
        .and_then(|a| parse_int(a))
        .unwrap_or(0);
    if amount <= 0 {
        return Reply::text(bad_input(&ctx.prefix));
    }

    if ctx.roster.is_empty() {
        return Reply::text(NO_MEMBER);
    }

    let mut writes = Vec::new();
    let mut success = 0usize;
    for p in &ctx.roster.participants {
        let digits: String = p.jid.chars().filter(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let jid = format!("{digits}@s.whatsapp.net");
        let current = p
            .data
            .get("energi")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(DEFAULT_ENERGI);
        writes.push(Mutation::SetUser {
            jid,
            data: json!({ "energi": next_energy(current, amount) }),
        });
        success += 1;
    }

    Reply::text(format!(
        "✅ Berhasil menambahkan limit ke semua member ( Total *{success}* Member ) di grup *{}*",
        // `ponytail:` `gb?.subject` diambil dari metadata grup yang tidak ada
        // di `Ctx` (lihat `group/intro.rs`), jadi nama grup dari
        // `group_data.subject` — atau teks cadangan kalau tidak ada.
        ctx.group_field("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Grup")
    ))
    .with_reaction("🕕")
    .with_reaction("⚡")
    .mutating(writes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, registry::GroupRoster};
    use serde_json::Value;

    const CHAT: &str = "120363@g.us";

    fn member(jid: &str, energi: Value) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: false,
            is_owner: false,
            data: json!({ "energi": energi }),
        }
    }

    fn run(text: &str, roster: GroupRoster) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addenergiall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: CHAT.into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            roster,
            ..Default::default()
        };
        (oktz_plugin_api::find("addenergiall").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn writes(r: &Reply) -> Vec<(String, i64)> {
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, data } => {
                    (jid.clone(), data["energi"].as_i64().expect("energi angka"))
                }
                other => panic!("harus SetUser, dapat {other:?}"),
            })
            .collect()
    }

    #[test]
    fn registers_under_the_js_plugin_name_not_the_file_name() {
        let p = oktz_plugin_api::find("addenergiall").unwrap();
        assert_eq!(p.meta.name, "addenergiall");
        assert_eq!(p.meta.alias, ["addenergianall", "bonusenergiall"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Menambahkan limit/energi ke semua member grup"
        );
        assert_eq!(p.meta.usage, ".addenergiall <jumlah>");
        assert_eq!(p.meta.example, ".addenergiall 50");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["addenergianall", "bonusenergiall"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addenergiall");
        }
    }

    #[test]
    fn a_missing_or_non_positive_amount_is_rejected_before_any_write() {
        let roster = GroupRoster {
            participants: vec![member("1@s.whatsapp.net", json!(10))],
            is_bot_admin: false,
        };
        for msg in ["", "0", "-5", "abc"] {
            let r = run(msg, roster);
            assert!(text_of(&r).starts_with("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn an_empty_roster_is_refused_before_any_write() {
        let r = run("50", GroupRoster::default());
        assert_eq!(text_of(&r), NO_MEMBER);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_member_gets_one_write_and_the_count_matches() {
        let roster = GroupRoster {
            participants: vec![
                member("628111@s.whatsapp.net", json!(10)),
                member("628222@s.whatsapp.net", json!(0)),
            ],
            is_bot_admin: false,
        };
        let r = run("50", roster);
        let w = writes(&r);
        assert_eq!(
            w,
            [
                ("628111@s.whatsapp.net".to_string(), 60),
                ("628222@s.whatsapp.net".to_string(), 50)
            ]
        );
        assert!(text_of(&r).contains("Total *2* Member"), "{}", text_of(&r));
    }

    #[test]
    fn a_participant_without_digits_is_skipped_and_not_counted() {
        let roster = GroupRoster {
            participants: vec![
                member("@lid", json!(5)),
                member("628111@s.whatsapp.net", json!(10)),
            ],
            is_bot_admin: false,
        };
        let r = run("50", roster);
        assert_eq!(writes(&r).len(), 1);
        assert!(text_of(&r).contains("Total *1* Member"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_member_keeps_minus_one() {
        let roster = GroupRoster {
            participants: vec![member("628111@s.whatsapp.net", json!(-1))],
            is_bot_admin: false,
        };
        assert_eq!(writes(&run("50", roster)), [("628111@s.whatsapp.net".to_string(), -1)]);
    }

    #[test]
    fn the_energy_floor_is_zero() {
        let roster = GroupRoster {
            participants: vec![member("628111@s.whatsapp.net", json!(-20))],
            is_bot_admin: false,
        };
        // `Math.max(0, -20 + 50)` = 30 — Amount di plugin ini selalu positif,
        // jadi lantai 0 hanya terlihat kalau row-nya sudah negatif.
        assert_eq!(writes(&run("50", roster))[0].1, 30);
    }

    #[test]
    fn a_missing_stored_energy_uses_the_config_default() {
        let roster = GroupRoster {
            participants: vec![Participant {
                jid: "628111@s.whatsapp.net".into(),
                is_admin: false,
                is_owner: false,
                data: json!({}),
            }],
            is_bot_admin: false,
        };
        assert_eq!(writes(&run("1", roster))[0].1, DEFAULT_ENERGI + 1);
    }

    #[test]
    fn the_final_reaction_is_the_one_from_the_js_order() {
        let roster = GroupRoster {
            participants: vec![member("628111@s.whatsapp.net", json!(10))],
            is_bot_admin: false,
        };
        match run("50", roster).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "⚡"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}