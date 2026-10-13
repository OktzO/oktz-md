//! `plugins/owner/addpartner.js` — satu file untuk tiga command
//! (`addpartner` / `delpartner` / `listpartner`), dipilih dari `m.command`.
//!
//! `db.data.partner` adalah store terpisah (`partner.json`), bukan
//! `db.setting`, dan `Mutation` tidak punya operasi untuk menuliskannya —
//! jadi daftar partner TIDAK bisa diubah dari plugin ini (lihat catatan
//! modul `addpremall`). Yang bisa: baca daftar, dan tulis status partner
//! per user lewat `isPartner`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{now_ms, parse_int};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const DEFAULT_DAYS: i64 = 30;

/// `text.match(/(\d{10,15})/)` — digit run sepanjang 10..=15, dimulai dari
/// posisi mana pun di teks gabungan argumen.
fn digits_10_to_15(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if (10..=15).contains(&(i - start)) {
                return Some(chars[start..i].iter().collect());
            }
        } else {
            i += 1;
        }
    }
    None
}

/// `extractNumber` — mention dulu, baru quote, baru nomor di teks argumen.
fn extract_target(ctx: &Ctx) -> Option<String> {
    if let Some(jid) = ctx.mentioned.first() {
        return Some(jid.clone());
    }
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    let text = ctx.args.join(" ");
    digits_10_to_15(text.trim()).map(|d| format!("{d}@s.whatsapp.net"))
}

/// `target.replace(/@.+/g, '')`, lalu `'62' + num.slice(1)` kalau awalan
/// `08` — `slice(1)` membuang SATU karakter, jadi `0812…` → `62812…`.
fn to_number(jid: &str) -> String {
    let num: String = jid.chars().take_while(|c| *c != '@').collect();
    if num.starts_with("08") {
        format!("62{}", &num[1..])
    } else {
        num
    }
}

/// `m.args.find(a => /^\d+$/.test(a) && a.length <= 4)`.
fn days_arg(ctx: &Ctx) -> i64 {
    ctx.args
        .iter()
        .find(|a| !a.is_empty() && a.chars().count() <= 4 && a.chars().all(|c| c.is_ascii_digit()))
        .and_then(|a| parse_int(a))
        .unwrap_or(DEFAULT_DAYS)
}

fn add_help(prefix: &str) -> String {
    format!(
        "🤝 *ᴀᴅᴅ ᴘᴀʀᴛɴᴇʀ*\n\n\
         > Cara pakai:\n\
         > `{p}addpartner @tag [hari]`\n\
         > `{p}addpartner 6281xxx 30`\n\n\
         > Default: 30 hari",
        p = prefix
    )
}

pub(crate) const NO_TARGET: &str = "⚠️ Tag atau reply user yang ingin dihapus dari partner.";
pub(crate) const NOT_PARTNER: &str = "⚠️ User tersebut bukan partner.";
pub(crate) const EMPTY: &str = "🤝 *ᴅᴀꜰᴛᴀʀ ᴘᴀʀᴛɴᴇʀ*\n\n> Belum ada partner.";

/// Daftar partner tersimpan: `settings.partnerList` — bentuk `{id, expired}`
/// yang sama dengan entri `db.data.partner`, dipindah ke `settings` karena
/// store terpisah tidak terjangkau dari plugin (`ponytail:`, lihat modul).
pub(crate) fn partners(ctx: &Ctx) -> Vec<(String, i64)> {
    ctx.setting_list("partnerList")
        .iter()
        .filter_map(|v| {
            let id = v.get("id").and_then(|x| x.as_str())?.to_string();
            let expired = v.get("expired").and_then(|x| x.as_i64()).unwrap_or(0);
            Some((id, expired))
        })
        .collect()
}

#[plugin(
    name = "addpartner",
    alias = ["delpartner", "listpartner"],
    category = "owner",
    description = "Kelola daftar partner bot",
    usage = ".addpartner <nomor/@tag> [hari]\n.delpartner <nomor/@tag>\n.listpartner\n.cekpartner <nomor/@tag>",
    example = ".addpartner 6281234567890 30",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addpartner(ctx: Ctx) -> Reply {
    match ctx.command.as_str() {
        "listpartner" => {
            let list = partners(&ctx);
            if list.is_empty() {
                return Reply::text(EMPTY);
            }
            let now = now_ms();
            let mut txt = String::from("🤝 *DAFTAR PARTNER*\n\n");
            let mut mentions = Vec::new();
            for (i, (id, expired)) in list.iter().enumerate() {
                // `Math.ceil((expired - now) / DAY_MS)` — pembulatan ke atas,
                // jadi 1 ms sebelum kedaluwarsa masih "1d", bukan "0d".
                let remaining = (*expired - now).div_euclid(DAY_MS)
                    + i64::from((*expired - now).rem_euclid(DAY_MS) > 0);
                txt.push_str(&format!(
                    "{}. @{id} — {} ({})\n",
                    i + 1,
                    crate::common::date_long(*expired),
                    if *expired > now {
                        format!("{remaining}d")
                    } else {
                        "Expired".to_string()
                    }
                ));
                mentions.push(format!("{id}@s.whatsapp.net"));
            }
            txt.push_str(&format!("\nTotal: *{}* partner", list.len()));
            Reply::Text {
                text: txt,
                mentions,
            }
        }
        "delpartner" => {
            let target = match extract_target(&ctx) {
                Some(t) => t,
                None => return Reply::text(NO_TARGET),
            };
            let number = to_number(&target);
            let remaining: Vec<serde_json::Value> = ctx
                .setting_list("partnerList")
                .iter()
                .filter(|v| v.get("id").and_then(|x| x.as_str()) != Some(number.as_str()))
                .cloned()
                .collect();
            if remaining.len() == partners(&ctx).len() {
                return Reply::text(NOT_PARTNER);
            }
            Reply::Text {
                text: format!("✅ Berhasil menghapus @{number} dari partner"),
                mentions: vec![target],
            }
            .with_mutation(Mutation::SetSetting {
                key: "partnerList".into(),
                value: serde_json::Value::Array(remaining),
            })
        }
        _ => {
            let target = match extract_target(&ctx) {
                Some(t) => t,
                None => return Reply::text(add_help(&ctx.prefix)),
            };
            let number = to_number(&target);
            let now = now_ms();
            let days = days_arg(&ctx);
            let existing = partners(&ctx);

            // `baseTime = currentExpired > now ? currentExpired : now`
            let base = existing
                .iter()
                .find(|(id, _)| *id == number)
                .map_or(now, |(_, exp)| (*exp).max(now));
            let new_expired = base + days * DAY_MS;

            let mut list: Vec<serde_json::Value> = ctx.setting_list("partnerList").to_vec();
            let renewed = list.iter().any(|v| {
                v.get("id").and_then(|x| x.as_str()) == Some(number.as_str())
            });
            if renewed {
                for v in &mut list {
                    if v.get("id").and_then(|x| x.as_str()) == Some(number.as_str()) {
                        v["expired"] = json!(new_expired);
                    }
                }
            } else {
                list.push(json!({ "id": number, "expired": new_expired, "addedAt": now }));
            }

            Reply::Text {
                text: format!(
                    "✅ Berhasil {} partner @{} selama *{days} hari*\nExpired: *{}*",
                    if renewed { "memperpanjang" } else { "menambahkan" },
                    number,
                    crate::common::date_long(new_expired)
                ),
                mentions: vec![target],
            }
            .with_mutation(Mutation::SetSetting {
                key: "partnerList".into(),
                value: serde_json::Value::Array(list),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(command: &str, text: &str, settings: Value, mentioned: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find(command).unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "partnerList");
                value.as_array().expect("harus array").clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addpartner").unwrap();
        assert_eq!(p.meta.name, "addpartner");
        assert_eq!(p.meta.alias, ["delpartner", "listpartner"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kelola daftar partner bot");
        assert_eq!(
            p.meta.usage,
            ".addpartner <nomor/@tag> [hari]\n.delpartner <nomor/@tag>\n.listpartner\n.cekpartner <nomor/@tag>"
        );
        assert_eq!(p.meta.example, ".addpartner 6281234567890 30");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn all_three_commands_reach_the_same_handler() {
        for c in ["addpartner", "delpartner", "listpartner"] {
            assert_eq!(oktz_plugin_api::find(c).unwrap().meta.name, "addpartner");
        }
    }

    #[test]
    fn adding_without_a_target_shows_help_and_writes_nothing() {
        for text in ["", "abc", "123"] {
            let r = run("addpartner", text, json!({}), None);
            assert!(text_of(&r).starts_with("🤝 *ᴀᴅᴅ ᴘᴀʀᴛɴᴇʀ*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_ten_to_fifteen_digit_number_becomes_a_partner_entry() {
        let r = run("addpartner", "6281234567890 30", json!({}), None);
        let list = written(&r);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], json!("6281234567890"));
        assert!(text_of(&r).contains("✅ Berhasil menambahkan partner @6281234567890"));
        assert!(text_of(&r).contains("selama *30 hari*"));
    }

    #[test]
    fn an_eight_prefixed_number_is_rewritten_to_sixty_two() {
        let r = run("addpartner", "081234567890", json!({}), None);
        assert_eq!(written(&r)[0]["id"], json!("6281234567890"));
    }

    #[test]
    fn a_mention_wins_over_the_number_in_the_text() {
        let r = run("addpartner", "628999000000 5", json!({}), Some("628777@s.whatsapp.net"));
        assert_eq!(written(&r)[0]["id"], json!("628777"));
    }

    #[test]
    fn renewing_an_existing_partner_replaces_the_expiry_in_place() {
        let existing = json!({ "partnerList": [{ "id": "6281234567890", "expired": 4_000_000_000_000i64 }] });
        let r = run("addpartner", "6281234567890 1", existing, None);
        let list = written(&r);
        assert_eq!(list.len(), 1, "tidak boleh menambah entri ganda");
        assert!(text_of(&r).contains("memperpanjang"), "{}", text_of(&r));
    }

    #[test]
    fn deleting_an_unknown_partner_is_refused_and_writes_nothing() {
        let existing = json!({ "partnerList": [{ "id": "6281234567890", "expired": 1 }] });
        let r = run("delpartner", "628999000000", existing, None);
        assert_eq!(text_of(&r), NOT_PARTNER);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn deleting_removes_only_the_named_partner() {
        let existing = json!({ "partnerList": [
            { "id": "6281234567890", "expired": 1 },
            { "id": "628999000000", "expired": 1 },
        ]});
        let r = run("delpartner", "6281234567890", existing, None);
        let list = written(&r);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], json!("628999000000"));
        assert!(text_of(&r).contains("✅ Berhasil menghapus @6281234567890"));
    }

    #[test]
    fn deleting_without_a_target_is_refused() {
        let r = run("delpartner", "", json!({}), None);
        assert_eq!(text_of(&r), NO_TARGET);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn listing_an_empty_store_says_so_and_writes_nothing() {
        let r = run("listpartner", "", json!({}), None);
        assert_eq!(text_of(&r), EMPTY);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn listing_numbers_the_partners_and_mentions_each_one() {
        let existing = json!({ "partnerList": [
            { "id": "6281234567890", "expired": 4_000_000_000_000i64 },
            { "id": "628999000000", "expired": 1 },
        ]});
        let r = run("listpartner", "", existing, None);
        let t = text_of(&r);
        assert!(t.starts_with("🤝 *DAFTAR PARTNER*"), "{t}");
        assert!(t.contains("1. @6281234567890"), "{t}");
        assert!(t.contains("Expired"), "expired di masa lalu: {t}");
        assert!(t.contains("Total: *2* partner"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions.len(), 2, "satu mention per partner");
                assert_eq!(mentions[0], "6281234567890@s.whatsapp.net");
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_malformed_partner_entry_is_skipped_instead_of_panicking() {
        let existing = json!({ "partnerList": ["bukan objek", { "expired": 5 }] });
        let r = run("listpartner", "", existing, None);
        assert_eq!(text_of(&r), EMPTY);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_digit_run_scanner_respects_the_ten_to_fifteen_bound() {
        assert_eq!(digits_10_to_15("6281234567890"), Some("6281234567890".into()));
        assert_eq!(digits_10_to_15("6281234567890123"), None, "16 digit ditolak");
        assert_eq!(digits_10_to_15("628123456"), None, "9 digit ditolak");
        assert_eq!(
            digits_10_to_15("halo 6281234567890"),
            Some("6281234567890".into())
        );
    }
}