//! `plugins/group/warn.js` — peringatan member + kick otomatis di batas.
//!
//! Yang dipindah: seluruh gerbang, pencatatan `warnings` ke store grup, dan
//! teks SP. Pengeluaran di batas warning
//! `sock.groupParticipantsUpdate(m.chat, [id], 'remove')` (`warn.js:110`)
//! tidak ada di API plugin — di titik itu plugin diam (pola `kick.rs`):
//! warning ke-max tetap tercatat persis urutan JS (pencatatan terjadi SEBELUM
//! kick), tapi tidak ada klaim "telah dikeluarkan dari grup", dan penghapusan
//! `warnings[target]` yang di JS terjadi SETELAH kick sukses juga tidak ada.
//!
//! `ponytail:` gerbang "jangan warn bot sendiri" (`warn.js:84-88`)
//! membandingkan target dengan `sock.user?.id`; `Ctx` tidak punya JID bot,
//! jadi gerbang itu dilewati — pola yang sama dengan `kick.rs`/`delete.rs`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::{jid_number, now_ms, parse_int};
use crate::group::kick::find_participant;
use crate::group::listwarn::max_warnings;

/// Teks bantuan (`warn.js:33-48`).
fn help_text(prefix: &str, max: i64) -> String {
    format!(
        "⚠️ *SISTEM WARNING GRUP*\n\n\
         Sistem manajemen pelanggaran untuk member grup.\n\
         Batas Warning: *{max} kali* (Otomatis Kick)\n\n\
         *PENGGUNAAN:*\n\
         • *{prefix}warn @user <alasan>* — Memberi warning\n\
         • *{prefix}warn max <angka>* — Mengubah batas maksimal warning\n\
         • *{prefix}listwarn* — Melihat daftar member bermasalah\n\
         • *{prefix}resetwarn @user* — Menghapus semua warning member\n\n\
         *PENJELASAN ALUR PENGGUNAAN:*\n\
         1. Saat member melakukan pelanggaran pertama, beri mereka SP1: *{prefix}warn @user Spam pesan*\n\
         2. Bot akan mencatat \"Spam pesan\" sebagai warning ke-1 mereka.\n\
         3. Jika melanggar lagi, beri peringatan kedua dengan alasan baru: *{prefix}warn @user Berkata kasar*\n\
         4. Jika total peringatan member mencapai batas maksimal (saat ini *{max}*), bot akan otomatis MENGELUARKAN (Kick) member tersebut.\n\
         5. Riwayat pelanggaran bisa dilihat lengkap dengan mengetik *{prefix}listwarn @user*."
    )
}

/// Cara pakai saat target tidak ketemu (`warn.js:68-72`).
fn cara_pakai(prefix: &str) -> String {
    format!(
        "⚠️ *CARA PAKAI*\n\n\
         > Reply pesan user + `{prefix}warn alasan`\n\
         > Atau: `{prefix}warn @user alasan`"
    )
}

/// `m.text.replace(/@\d+/g, '')` — buang semua `@` yang diikuti digit.
fn buang_mention_angka(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '@' && chars.peek().is_some_and(|n| n.is_ascii_digit()) {
            while chars.peek().is_some_and(|n| n.is_ascii_digit()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `replace(/^\s*warn\s*/i, '')` — buang whitespace awal, kata `warn` (huruf
/// bebas), lalu whitespace sesudahnya. `warn` dicocokkan sebagai awalan
/// (tanpa batas kata), persis regex JS: `warning` ikut terbuang jadi `ing`.
fn buang_awalan_warn(s: &str) -> String {
    let t = s.trim_start();
    let mut it = t.chars();
    let head: String = it.by_ref().take(4).collect();
    if head.eq_ignore_ascii_case("warn") {
        it.collect::<String>().trim_start().to_string()
    } else {
        t.to_string()
    }
}

/// Alasan pada jalur mention (`warn.js:90`) — tanpa kutipan.
fn alasan_dari_teks(text: &str) -> String {
    let r = buang_awalan_warn(&buang_mention_angka(text)).trim().to_string();
    if r.is_empty() {
        "Tidak ada alasan".to_string()
    } else {
        r
    }
}

/// Alasan pada jalur kutipan (`warn.js:90`) — hanya trim, mention tidak dibuang.
fn alasan_dari_kutipan(text: &str) -> String {
    let r = text.trim();
    if r.is_empty() {
        "Tidak ada alasan".to_string()
    } else {
        r.to_string()
    }
}

/// Pencatatan warning (`warn.js:93-101`): satu entri `{reason, by, time}`
/// ditambahkan, lalu `db.setGroup(m.chat, { ...groupData, warnings })`.
/// Peta `warnings` dikirim utuh seperti spread JS; field grup lain tidak
/// disentuh karena mutasi Rust merge parsial.
fn catat_warning(ctx: &Ctx, target: &str, reason: &str) -> Mutation {
    let mut warnings = ctx
        .group_field("warnings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut list = warnings
        .get(target)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    list.push(serde_json::json!({ "reason": reason, "by": ctx.sender, "time": now_ms() }));
    warnings.insert(target.to_string(), Value::Array(list));
    Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "warnings": Value::Object(warnings) }),
    }
}

#[plugin(
    name = "warn",
    alias = ["warning", "peringatan"],
    category = "group",
    description = "Memberi peringatan kepada member",
    usage = ".warn @user <alasan>",
    example = ".warn @user spam",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn warn(ctx: Ctx) -> Reply {
    let max = max_warnings(&ctx);

    // `!args[0] && !m.quoted && (!m.mentionedJid || length === 0)` (`warn.js:32`).
    if ctx.args.is_empty() && ctx.quoted().is_none() && ctx.mentioned.is_empty() {
        return Reply::text(help_text(&ctx.prefix, max));
    }

    // Cabang `warn max <angka>` (`warn.js:50-58`).
    if ctx.args.first().map(|a| a.to_lowercase()).as_deref() == Some("max") {
        let new_max = ctx.args.get(1).and_then(|a| parse_int(a));
        if !new_max.is_some_and(|n| (1..=20).contains(&n)) {
            return Reply::text(format!(
                "❌ *GAGAL*\n\nBatas referensi warning harus berupa angka 1-20.\nContoh: *{}warn max 5*",
                ctx.prefix
            ));
        }
        let n = new_max.unwrap_or_default();
        return Reply::text(format!(
            "✅ *BATAS WARNING DIUBAH*\n\nMaksimal warning grup ini telah diupdate menjadi *{n} kali*."
        ))
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "maxWarnings": n }),
        });
    }

    // `m.quoted ? m.quoted.sender : m.mentionedJid[0]` (`warn.js:61-65`).
    let target = match ctx.quoted() {
        Some(q) => q.sender.clone(),
        None => match ctx.mentioned.first() {
            Some(jid) => jid.clone(),
            None => return Reply::text(cara_pakai(&ctx.prefix)),
        },
    };

    // Gerbang admin target (`warn.js:75-82`): `participant?.admin` truthy =
    // admin atau superadmin. Roster kosong = metadata tidak terbaca; JS
    // menelan errornya lewat catch kosong dan melanjutkan.
    if find_participant(&ctx.roster.participants, &target)
        .is_some_and(|p| p.is_admin)
    {
        return Reply::text("❌ Tidak bisa memberikan warning kepada admin grup.");
    }

    // Gerbang bot sendiri (`warn.js:84-88`) dilewati — lihat `ponytail:` di
    // header modul.

    let reason = if ctx.quoted().is_some() {
        alasan_dari_kutipan(&ctx.text)
    } else {
        alasan_dari_teks(&ctx.text)
    };

    let mutation = catat_warning(&ctx, &target, &reason);
    let count = ctx
        .group_field("warnings")
        .and_then(|w| w.get(&target))
        .and_then(Value::as_array)
        .map_or(0, |a| a.len() as i64)
        + 1;

    if count >= max {
        // Kick otomatis tidak di-port — plugin diam, warning tetap tercatat
        // (urutan JS: pencatatan sebelum kick). Lihat header modul.
        return Reply::None.with_mutation(mutation);
    }

    let num = jid_number(&target);
    Reply::Text {
        text: format!(
            "⚠️ *PERINGATAN DIBERIKAN*\n\n\
             @{num} telah menerima Surat Peringatan (SP{count})!\n\n\
             *Rincian:*\n\
             > Warning ke: *{count}/{max}*\n\
             > Alasan: *{reason}*\n\n\
             _{} warning lagi = KICK OTOMATIS_",
            max - count
        ),
        mentions: vec![target],
    }
    .with_mutation(mutation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use oktz_plugin_api::registry::GroupRoster;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn participant(jid: &str, admin: bool) -> oktz_plugin_api::Participant {
        oktz_plugin_api::Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: false,
            data: Value::Null,
        }
    }

    fn quoted_from(sender: &str) -> QuotedMessage {
        QuotedMessage {
            id: "M1".into(),
            sender: sender.into(),
            chat: "120363@g.us".into(),
            text: "pesan yang di-reply".into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn ctx_base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "warn".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants: vec![participant("628111@s.whatsapp.net", false)],
                is_bot_admin: true,
            },
            ..Default::default()
        }
    }

    fn run(ctx: Ctx) -> Reply {
        (find("warn").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("warn").unwrap();
        assert_eq!(p.meta.name, "warn");
        assert_eq!(p.meta.alias, ["warning", "peringatan"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Memberi peringatan kepada member");
        assert_eq!(p.meta.usage, ".warn @user <alasan>");
        assert_eq!(p.meta.example, ".warn @user spam");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin && !p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.usage.is_empty() && !p.meta.example.is_empty());
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["warning", "peringatan"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("warn"), "{a}");
        }
    }

    #[test]
    fn no_args_no_quote_no_mention_prints_the_help() {
        let r = run(ctx_base());
        let t = text_of(&r);
        assert!(t.starts_with("⚠️ *SISTEM WARNING GRUP*"), "{t}");
        assert!(t.contains("Batas Warning: *3 kali* (Otomatis Kick)"), "{t}");
        assert!(t.contains("*.warn max <angka>*"), "prefix ikut: {t}");
        assert!(r.mutations().is_empty(), "bantuan tidak menulis store");
    }

    #[test]
    fn the_help_reflects_a_custom_max_from_the_group() {
        let ctx = Ctx {
            group_data: serde_json::json!({ "maxWarnings": 7 }),
            ..ctx_base()
        };
        assert!(text_of(&run(ctx)).contains("Batas Warning: *7 kali*"));
    }

    #[test]
    fn warn_max_rejects_out_of_range_and_nan() {
        for args in [
            vec!["max".to_string(), "21".to_string()],
            vec!["max".to_string(), "0".to_string()],
            vec!["max".to_string(), "abc".to_string()],
            vec!["max".to_string()],
        ] {
            let ctx = Ctx {
                args,
                ..ctx_base()
            };
            let r = run(ctx);
            assert!(
                text_of(&r).contains("harus berupa angka 1-20"),
                "{}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn warn_max_accepts_the_boundaries_one_and_twenty() {
        for (raw, n) in [("1", 1), ("20", 20), ("5abc", 5)] {
            let ctx = Ctx {
                args: vec!["max".into(), raw.into()],
                ..ctx_base()
            };
            let r = run(ctx);
            assert!(
                text_of(&r).contains(&format!("menjadi *{n} kali*")),
                "raw {raw}: {}",
                text_of(&r)
            );
            assert_eq!(
                r.mutations()[0],
                Mutation::SetGroup {
                    jid: "120363@g.us".into(),
                    data: serde_json::json!({ "maxWarnings": n }),
                }
            );
        }
    }

    #[test]
    fn an_argument_without_target_asks_for_one() {
        let ctx = Ctx {
            args: vec!["saja".into()],
            ..ctx_base()
        };
        let t = text_of(&run(ctx));
        assert!(t.starts_with("⚠️ *CARA PAKAI*"), "{t}");
        assert!(t.contains("`.warn @user alasan`"), "prefix ikut: {t}");
    }

    #[test]
    fn a_quoted_sender_wins_over_a_mention() {
        let ctx = Ctx {
            args: vec!["spam".into()],
            text: "spam".into(),
            quoted: Some(quoted_from("628999@s.whatsapp.net")),
            mentioned: vec!["628777@s.whatsapp.net".into()],
            ..ctx_base()
        };
        let r = run(ctx);
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        assert!(
            map.get("warnings").unwrap().get("628999@s.whatsapp.net").is_some(),
            "kunci warnings = pengirim kutipan: {map:?}"
        );
    }

    #[test]
    fn a_mention_target_receives_the_sp_reply_and_a_record() {
        let ctx = Ctx {
            args: vec!["@628999".into(), "spam".into()],
            text: "@628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            ..ctx_base()
        };
        let r = run(ctx);
        let t = text_of(&r);
        assert!(t.starts_with("⚠️ *PERINGATAN DIBERIKAN*"), "{t}");
        assert!(t.contains("@628999 telah menerima Surat Peringatan (SP1)!"), "{t}");
        assert!(t.contains("> Warning ke: *1/3*"), "{t}");
        assert!(t.contains("> Alasan: *spam*"), "{t}");
        assert!(t.contains("_2 warning lagi = KICK OTOMATIS_"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        let warnings = map.get("warnings").unwrap().as_object().unwrap();
        let list = warnings.get("628999@s.whatsapp.net").unwrap().as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["reason"], "spam");
        assert_eq!(list[0]["by"], "628111@s.whatsapp.net");
        assert!(list[0]["time"].as_i64().unwrap_or(0) > 0, "time = Date.now()");
    }

    #[test]
    fn existing_warnings_are_appended_and_other_entries_survive() {
        let ctx = Ctx {
            args: vec!["@628999".into(), "lagi".into()],
            text: "@628999 lagi".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            group_data: serde_json::json!({
                "maxWarnings": 3,
                "warnings": {
                    "628999@s.whatsapp.net": [{"reason": "pertama", "by": "628111@s.whatsapp.net", "time": 1}],
                    "628000@s.whatsapp.net": [{"reason": "milik orang lain", "time": 2}]
                }
            }),
            ..ctx_base()
        };
        let r = run(ctx);
        assert!(text_of(&r).contains("(SP2)!"));
        assert!(text_of(&r).contains("> Warning ke: *2/3*"));
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        let warnings = map.get("warnings").unwrap().as_object().unwrap();
        assert_eq!(
            warnings.get("628000@s.whatsapp.net").unwrap().as_array().unwrap().len(),
            1,
            "entri member lain tidak boleh hilang"
        );
        assert_eq!(
            warnings.get("628999@s.whatsapp.net").unwrap().as_array().unwrap().len(),
            2
        );
    }

    #[test]
    fn an_admin_target_is_refused_without_recording() {
        let ctx = Ctx {
            args: vec!["@628999".into(), "spam".into()],
            text: "@628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            roster: GroupRoster {
                participants: vec![
                    participant("628111@s.whatsapp.net", false),
                    participant("628999@s.whatsapp.net", true),
                ],
                is_bot_admin: true,
            },
            ..ctx_base()
        };
        let r = run(ctx);
        assert_eq!(text_of(&r), "❌ Tidak bisa memberikan warning kepada admin grup.");
        assert!(r.mutations().is_empty(), "tidak ada yang dicatat");
    }

    #[test]
    fn an_empty_roster_skips_the_admin_gate_like_the_js_empty_catch() {
        let ctx = Ctx {
            args: vec!["@628999".into(), "spam".into()],
            text: "@628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            roster: GroupRoster::default(),
            ..ctx_base()
        };
        assert!(text_of(&run(ctx)).contains("SP1"));
    }

    #[test]
    fn the_reason_strips_mentions_then_a_leading_warn_word() {
        // `.warn warn @628999 spam` → teks setelah command = "warn @628999 spam".
        let ctx = Ctx {
            args: vec!["warn".into(), "@628999".into(), "spam".into()],
            text: "warn @628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            ..ctx_base()
        };
        let r = run(ctx);
        assert!(text_of(&r).contains("> Alasan: *spam*"), "{}", text_of(&r));
    }

    #[test]
    fn an_empty_reason_falls_back_to_the_js_default() {
        let ctx = Ctx {
            args: vec!["@628999".into()],
            text: "@628999".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            ..ctx_base()
        };
        assert!(text_of(&run(ctx)).contains("> Alasan: *Tidak ada alasan*"));
    }

    #[test]
    fn a_quoted_reason_keeps_the_raw_text_including_mentions() {
        // Jalur kutipan hanya `m.text.trim()` — token mention TIDAK dibuang.
        let ctx = Ctx {
            args: vec!["kasar".into()],
            text: "@628999 kasar".into(),
            quoted: Some(quoted_from("628999@s.whatsapp.net")),
            ..ctx_base()
        };
        assert!(
            text_of(&run(ctx)).contains("> Alasan: *@628999 kasar*"),
            "{}",
            text_of(&run(Ctx {
                args: vec!["kasar".into()],
                text: "@628999 kasar".into(),
                quoted: Some(quoted_from("628999@s.whatsapp.net")),
                ..ctx_base()
            }))
        );
    }

    #[test]
    fn reaching_the_max_stays_silent_but_still_records_like_the_js_order() {
        // `warn.js` mencatat warning DULU, baru kick. Kick tidak di-port, jadi
        // hasilnya: tercatat + tanpa balasan (pola kick.rs).
        let ctx = Ctx {
            args: vec!["@628999".into(), "spam".into()],
            text: "@628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            group_data: serde_json::json!({ "maxWarnings": 1 }),
            ..ctx_base()
        };
        let r = run(ctx);
        assert!(r.is_none(), "tidak ada klaim kick: {:?}", r.text_body());
        assert_eq!(r.mutations().len(), 1, "warning ke-max tetap tercatat");
    }

    #[test]
    fn a_non_array_warning_entry_is_treated_as_empty() {
        let ctx = Ctx {
            args: vec!["@628999".into(), "spam".into()],
            text: "@628999 spam".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            group_data: serde_json::json!({ "warnings": { "628999@s.whatsapp.net": "bukan-array" } }),
            ..ctx_base()
        };
        assert!(text_of(&run(ctx)).contains("(SP1)!"));
    }
}
