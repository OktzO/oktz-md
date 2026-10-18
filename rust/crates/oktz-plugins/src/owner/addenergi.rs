//! `plugins/owner/addenergi.js` — tambah energi (bisa `--unlimited`).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `config.energi.default` (`config.js:114`) — nilai yang dipakai `setUser`
/// untuk user yang belum pernah ada.
const DEFAULT_ENERGI: i64 = 99_999;

/// `user.energi === -1`. `db.updateEnergi` juga mengembalikan `-1` tanpa
/// mengubah apa pun kalau user sudah unlimited (`database.js:630`).
const UNLIMITED: i64 = -1;

/// Ribuan bertitik, tanda minus ikut (`-1234` → `-1.234`).
fn thousands(n: i64) -> String {
    if n < 0 {
        format!("-{}", crate::tools::kalkulatormbg::thousands_id(n.unsigned_abs() as f64))
    } else {
        crate::tools::kalkulatormbg::thousands_id(n as f64)
    }
}

/// `formatNumber` (`addenergi.js:17-20`).
fn format_number(n: i64) -> String {
    if n == UNLIMITED {
        "∞ Unlimited".to_string()
    } else {
        thousands(n)
    }
}

/// `args.find(a => !isNaN(a) && !a.includes('@') && !a.startsWith('-'))` —
/// `isNaN` JS itu koersi penuh, jadi token harus benar-benar angka
/// (`12abc` → `NaN`, ditolak; `12.5` → diterima lalu `parseInt` jadi `12`).
fn numeric_arg(ctx: &Ctx) -> Option<&str> {
    ctx.args
        .iter()
        .find(|a| !a.contains('@') && !a.starts_with('-') && a.parse::<f64>().is_ok())
        .map(String::as_str)
}

/// `m.quoted` menang lebih dulu; kalau tidak ada, `mentionedJid[0]`.
fn mentioned_or_quoted(ctx: &Ctx) -> Option<String> {
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    ctx.mentioned.first().cloned()
}

/// `args.find(a => a !== numArg && a.length > 5 && /^\d+$/.test(digits(a)))`.
///
/// Setelah `replace(/[^0-9]/g,'')` sisanya cuma digit, jadi `^\d+$` berarti
/// "tidak kosong" — cukup cek token punya minimal satu digit ASCII.
fn phone_arg(ctx: &Ctx, num_arg: Option<&str>) -> Option<String> {
    ctx.args
        .iter()
        .find(|a| {
            Some(a.as_str()) != num_arg
                && a.chars().count() > 5
                && a.chars().any(|c| c.is_ascii_digit())
        })
        .map(|a| {
            a.chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                + "@s.whatsapp.net"
        })
}

/// Energi tersimpan untuk `jid`.
///
/// `ponytail:` `Ctx` hanya mengisi `user_data` untuk `sender` dan
/// `participants[i].data` untuk roster grup. JID di luar keduanya dianggap
/// user baru — sama dengan `db.getUser(jid) || db.setUser(jid)`, yang
/// memberi `config.energi.default`. Tambahkan `user_entry(jid)` ke `Ctx`
/// kalau router mau membaca state user mana pun.
fn stored_energi(ctx: &Ctx, jid: &str) -> i64 {
    let entry = if jid == ctx.sender {
        Some(&ctx.user_data)
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == jid)
            .map(|p| &p.data)
    };
    entry
        .and_then(|v| v.get("energi"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(DEFAULT_ENERGI)
}

fn help(prefix: &str) -> String {
    format!(
        "⚡ *ᴀᴅᴅ ᴇɴᴇʀɢɪ*\n\n\
         > `.{p}addenergi <jumlah>` - ke diri sendiri\n\
         > `.{p}addenergi <jumlah> @user` - ke user\n\
         > `.{p}addenergi --unlimited` - unlimited\n\n\
         `Contoh: {p}addenergi 100`",
        p = prefix
    )
}

#[plugin(
    name = "addenergi",
    alias = ["tambahenergi", "giveenergi", "addenergy"],
    category = "owner",
    description = "Tambah energi user",
    usage = ".addenergi <jumlah> @user",
    example = ".addenergi 100 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addenergi(ctx: Ctx) -> Reply {
    let lower = ctx.text.to_lowercase();
    let is_unlimited = lower.contains("--unlimited") || lower.contains("--unli");

    let num_arg = numeric_arg(&ctx);
    let amount = num_arg
        .and_then(crate::common::parse_int)
        .unwrap_or(0);

    let target = match mentioned_or_quoted(&ctx) {
        Some(jid) => Some(jid),
        None => phone_arg(&ctx, num_arg),
    };
    // `!targetJid && (amount > 0 || isUnlimited)` — tanpa target DAN tanpa
    // jumlah, plugin jatuh ke help, bukan ke sender.
    let target = match target {
        Some(jid) if !jid.is_empty() => Some(jid),
        _ if amount > 0 || is_unlimited => Some(ctx.sender.clone()),
        _ => None,
    };

    let target = match target {
        Some(jid) if !jid.is_empty() => jid,
        _ => return Reply::text(help(&ctx.prefix)),
    };
    if !is_unlimited && amount <= 0 {
        return Reply::text(help(&ctx.prefix));
    }

    let current = stored_energi(&ctx, &target);
    // `config.isOwner(target) && config.energi.owner === -1`. Cabang premium
    // mati di config ini (`energi.premium = 99999999`), dan `isOwner` hanya
    // bisa dijawab untuk pengirim — `ponytail:` tambahkan `owner_numbers`
    // ke `Ctx` kalau router mau mengecek JID target.
    let effective_unlimited = current == UNLIMITED || (target == ctx.sender && ctx.is_owner);

    if !is_unlimited && effective_unlimited {
        return Reply::Text {
            text: format!(
                "⚡ *INFORMASI*\n@{} sudah memiliki energi *∞ Unlimited*\nTidak perlu menambahkan energi lagi",
                crate::common::jid_number(&target)
            ),
            mentions: vec![target],
        };
    }

    if is_unlimited {
        return Reply::text(format!(
            "✅ Energi @{} sekarang unlimited / tidak terbatas",
            crate::common::jid_number(&target)
        ))
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: target,
            data: serde_json::json!({ "energi": UNLIMITED }),
        });
    }

    let new_energi = if current == UNLIMITED {
        UNLIMITED
    } else {
        (current + amount).max(0)
    };
    Reply::text(format!(
        "✅ Energi *@{}* berhasil di tambahkan sebanyak *{}*!\nSekarang dia mempunyai *{}* energi",
        crate::common::jid_number(&target),
        format_number(amount),
        format_number(new_energi)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: target,
        data: serde_json::json!({ "energi": new_energi }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedMessage;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn run(text: &str, user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addenergi".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("addenergi").unwrap().handler)(ctx)
    }

    fn run_to(text: &str, user: Value, mentioned: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addenergi".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            mentioned: vec![mentioned.into()],
            ..Default::default()
        };
        // `run_quoted` menambahkan quoted; di sini context-nya sudah punya
        // mention, jadi quoted cukup lewat helper terpisah di bawah.
        (oktz_plugin_api::find("addenergi").unwrap().handler)(ctx)
    }

    fn run_quoted(text: &str, user: Value, mentioned: &str) -> Reply {
        let mut ctx = Ctx {
            prefix: ".".into(),
            command: "addenergi".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            mentioned: vec![mentioned.into()],
            ..Default::default()
        };
        ctx.quoted = Some(QuotedMessage {
            id: "x".into(),
            sender: OTHER.into(),
            chat: SENDER.into(),
            text: String::new(),
            kind: oktz_plugin_api::QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        });
        (oktz_plugin_api::find("addenergi").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written_energi(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu SetUser");
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => (
                jid.clone(),
                data["energi"].as_i64().expect("energi harus angka"),
            ),
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addenergi").unwrap();
        assert_eq!(p.meta.name, "addenergi");
        assert_eq!(p.meta.alias, ["tambahenergi", "giveenergi", "addenergy"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Tambah energi user");
        assert_eq!(p.meta.usage, ".addenergi <jumlah> @user");
        assert_eq!(p.meta.example, ".addenergi 100 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambahenergi", "giveenergi", "addenergy"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addenergi");
        }
    }

    #[test]
    fn amount_and_target_both_missing_show_help_and_write_nothing() {
        for msg in ["", "   ", "abc", "@628999"] {
            let r = run(msg, serde_json::json!({}));
            assert!(text_of(&r).starts_with("⚡ *ᴀᴅᴅ ᴇɴᴇʀɢɪ*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn a_target_without_a_positive_amount_still_shows_help() {
        // `!targetJid || (!isUnlimited && amount <= 0)` — Mention tanpa angka
        // tidak bisa jatuh ke "tambah ke diri sendiri".
        for text in ["", "abc", "0"] {
            let r = run_to(text, serde_json::json!({}), OTHER);
            assert!(
                text_of(&r).starts_with("⚡ *ᴀᴅᴅ ᴇɴᴇʀɢɪ*"),
                "input {text:?}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_bare_amount_adds_to_the_sender() {
        let r = run("100", serde_json::json!({ "energi": 40 }));
        assert_eq!(written_energi(&r), (SENDER.to_string(), 140));
        assert!(text_of(&r).contains("berhasil di tambahkan sebanyak *100*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("mempunyai *140* energi"));
    }

    #[test]
    fn a_mention_and_a_quote_both_redirect_the_write() {
        assert_eq!(
            written_energi(&run_to("50", serde_json::json!({}), OTHER)),
            (OTHER.to_string(), 99_999 + 50),
            "target di luar sender+roster: energi lama = default config"
        );
        assert_eq!(
            written_energi(&run_quoted("50", serde_json::json!({}), OTHER)),
            (OTHER.to_string(), 99_999 + 50),
            "m.quoted menang atas mentionedJid"
        );
    }

    #[test]
    fn a_bare_phone_number_is_consumed_as_the_amount_just_like_in_js() {
        // `args.find(a => !isNaN(a) && ...)` — "628999123456" lolos `isNaN`,
        // jadi ia yang jadi numArg, bukan nomor tujuan. Branch phone hanya
        // kena kalau token itu DILEWATI numArg, jadi mustahil untuk angka
        // murni: keep the test honest about what the JS actually does.
        let r = run("628999123456 50", serde_json::json!({ "energi": 0 }));
        let (jid, value) = written_energi(&r);
        assert_eq!(jid, SENDER, "tidak ada target, jadi jatuh ke sender");
        assert_eq!(value, 628_999_123_456, "numArg = 628999123456");
    }

    #[test]
    fn a_phone_number_only_reaches_the_phone_branch_when_it_is_not_the_amount() {
        // Non-numerik sepanjang >5 dengan digit → branch phone. "50" cuma
        // 2 karakter jadi tidak memenuhi `a.length > 5`.
        let ctx = Ctx {
            args: vec!["+62899-123-456".into(), "50".into()],
            ..Default::default()
        };
        assert_eq!(
            phone_arg(&ctx, None),
            Some("62899123456@s.whatsapp.net".to_string())
        );
    }

    #[test]
    fn adding_to_a_user_with_zero_stays_at_or_above_zero() {
        let r = run("500", serde_json::json!({ "energi": 10 }));
        assert_eq!(written_energi(&r), (SENDER.to_string(), 510));
        assert!(text_of(&r).contains("*510* energi"));
    }

    #[test]
    fn the_floor_is_zero_when_the_stored_value_went_negative() {
        // `Math.max(0, (energi ?? 0) + amount)` — row dengan -20 (bukan -1)
        // dijepit ke 0, persis `db.updateEnergi`.
        let r = run("5", serde_json::json!({ "energi": -20 }));
        assert_eq!(written_energi(&r), (SENDER.to_string(), 0));
    }

    #[test]
    fn unlimited_writes_minus_one_for_either_spelling() {
        for flag in ["--unlimited", "--unli", "--UNLIMITED"] {
            let r = run(flag, serde_json::json!({ "energi": 12 }));
            assert_eq!(written_energi(&r), (SENDER.to_string(), UNLIMITED), "flag {flag}");
            assert!(text_of(&r).contains("unlimited / tidak terbatas"), "flag {flag}");
        }
    }

    #[test]
    fn an_unlimited_user_is_never_topped_up() {
        let r = run("100", serde_json::json!({ "energi": -1 }));
        assert!(text_of(&r).starts_with("⚡ *INFORMASI*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty(), "tidak boleh menulis");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[SENDER.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn an_unlimited_target_reached_through_the_roster_is_never_topped_up() {
        // Energi target dibaca dari `participants[i].data` saat target ada di
        // roster grup — jalur yang membuat guard "sudah unlimited" hidup.
        let ctx = Ctx {
            sender: SENDER.into(),
            mentioned: vec![OTHER.into()],
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![oktz_plugin_api::Participant {
                    jid: OTHER.into(),
                    is_admin: false,
                    is_owner: false,
                    data: serde_json::json!({ "energi": -1 }),
                }],
                is_bot_admin: false,
            },
            text: "100".into(),
            args: vec!["100".into()],
            ..Default::default()
        };
        assert_eq!(stored_energi(&ctx, OTHER), UNLIMITED);
        let r = (oktz_plugin_api::find("addenergi").unwrap().handler)(ctx);
        assert!(text_of(&r).starts_with("⚡ *INFORMASI*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[OTHER.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_missing_stored_value_falls_back_to_the_config_default() {
        let ctx = Ctx {
            user_data: serde_json::json!({ "koin": 3 }),
            ..Default::default()
        };
        assert_eq!(stored_energi(&ctx, "628111@s.whatsapp.net"), DEFAULT_ENERGI);
        assert_eq!(DEFAULT_ENERGI, 99_999, "config.energi.default tidak berubah");
    }

    #[test]
    fn thousands_separator_matches_the_js_regex() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1.000");
        assert_eq!(format_number(1_234_567), "1.234.567");
        assert_eq!(format_number(UNLIMITED), "∞ Unlimited");
    }

    #[test]
    fn the_success_reply_reacts_and_carries_the_write_it_claims() {
        let r = run("100", serde_json::json!({ "energi": 0 }));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}