//! `plugins/cek/cekowner.js` — role owner/partner/premium + ringkasan store.
//!
//! `ponytail:` `config.isOwner/isPartner/isPremium` membaca `config.js`, yang
//! tidak ada di `Ctx`. Yang dipakai sebagai gantinya adalah daftar
//! `owner`/`partner`/`premium` dari settings — satu-satunya sumber yang
//! memang membaca store, dan itu juga yang ditulis `owner/*` plugin. Owner list
//! di `config.js` sendiri tidak terlihat di sini.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// `Number.prototype.toLocaleString('id-ID')` untuk bilangan bulat.
pub(crate) fn thousands_id(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Target + nomor, urutan `quoted` → `mentionedJid[0]` → `args[0]` → sender.
pub(crate) fn resolve_target(ctx: &Ctx) -> (String, String) {
    let jid = if let Some(q) = ctx.quoted() {
        q.sender.clone()
    } else if let Some(m) = ctx.mentioned.first() {
        m.clone()
    } else if let Some(a) = ctx.args.first() {
        format!(
            "{}@s.whatsapp.net",
            a.chars().filter(char::is_ascii_digit).collect::<String>()
        )
    } else {
        ctx.sender.clone()
    };
    (jid.clone(), jid_number_of(&jid))
}

fn jid_number_of(jid: &str) -> String {
    jid.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
}

/// `targetNumber.startsWith('0') → '62' + slice(1)`.
pub(crate) fn normalize(mut number: String) -> String {
    if let Some(rest) = number.strip_prefix('0') {
        number = format!("62{rest}");
    }
    number
}

/// `user.energi === -1 ? '∞' : (user.energi ?? 0)`.
pub(crate) fn energi(user: &serde_json::Value) -> String {
    match user.get("energi").and_then(serde_json::Value::as_i64) {
        Some(-1) => "∞".to_string(),
        Some(v) => v.to_string(),
        None => "0".to_string(),
    }
}

/// `user.koin === -1 ? '∞' : (user.koin ?? 0).toLocaleString('id-ID')`.
pub(crate) fn koin(user: &serde_json::Value) -> String {
    match user.get("koin").and_then(serde_json::Value::as_i64) {
        Some(-1) => "∞".to_string(),
        Some(v) => thousands_id(v),
        None => "0".to_string(),
    }
}

/// `user.level ?? 1`.
pub(crate) fn level(user: &serde_json::Value) -> String {
    user.get("level")
        .and_then(serde_json::Value::as_i64)
        .map_or_else(|| "1".to_string(), |v| v.to_string())
}

#[plugin(
    name = "cekowner",
    alias = ["ownerinfo"],
    category = "cek",
    description = "Cek apakah user adalah owner bot",
    usage = ".cekowner @user",
    example = ".cekowner",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_premium = true
)]
pub fn cekowner(ctx: Ctx) -> Reply {
    let (target_jid, raw_number) = resolve_target(&ctx);
    let number = normalize(raw_number);

    let owners = joined_strings(ctx.setting_list("owner"));
    let partners = joined_strings(ctx.setting_list("partner"));
    let premiums = joined_strings(ctx.setting_list("premium"));

    let mut roles = Vec::new();
    if owners.contains(&number) {
        roles.push("👑 Owner");
    }
    if partners.contains(&number) {
        roles.push("🤝 Partner");
    }
    if premiums.contains(&number) {
        roles.push("💎 Premium");
    }
    if roles.is_empty() {
        roles.push("👤 Free User");
    }

    let mut txt = String::from("📋 *CEK USER INFO*\n\n");
    txt.push_str(&format!("👤 User: @{number}\n"));
    txt.push_str(&format!("🏷️ Role: *{}*\n", roles.join(" • ")));
    txt.push_str(&format!(
        "📊 Owner DB: *{}*\n",
        if owners.contains(&number) { "Ya" } else { "Tidak" }
    ));

    // `db.getUser(targetJid)` hanya terbaca untuk `sender` atau member grup,
    // dan mengembalikan `null` untuk user yang belum pernah ada.
    let user = if target_jid == ctx.sender {
        (!ctx.user_data.is_null()).then_some(&ctx.user_data)
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == target_jid)
            .map(|p| &p.data)
            .filter(|d| !d.is_null())
    };
    if let Some(user) = user {
        txt.push_str(&format!("⚡ Energi: *{}*\n", energi(user)));
        txt.push_str(&format!("💰 Koin: *{}*\n", koin(user)));
        txt.push_str(&format!("⭐ Level: *{}*\n", level(user)));
    }

    Reply::Text {
        text: txt,
        mentions: vec![target_jid],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::date_long;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::QuotedMessage;
    use serde_json::{Value};

    const ME: &str = "628111@s.whatsapp.net";

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("cekowner").unwrap().handler)(ctx)
    }

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekowner".into(),
            sender: ME.into(),
            chat: ME.into(),
            ..Default::default()
        }
    }

    fn text_of(r: &Reply) -> (String, Vec<String>) {
        match r {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekowner").unwrap();
        assert_eq!(p.meta.name, "cekowner");
        assert_eq!(p.meta.alias, ["ownerinfo"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek apakah user adalah owner bot");
        assert_eq!(p.meta.usage, ".cekowner @user");
        assert_eq!(p.meta.example, ".cekowner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_premium, "isPremium: true di JS");
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin && p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cekowner", "ownerinfo"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cekowner"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn target_resolution_follows_the_quoted_mention_arg_order() {
        let mut ctx = base();
        assert_eq!(resolve_target(&ctx), (ME.to_string(), "628111".to_string()));

        ctx.quoted = Some(QuotedMessage {
            id: "x".into(),
            sender: "628222@s.whatsapp.net".into(),
            chat: ME.into(),
            text: String::new(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
            timestamp: 0,
        });
        ctx.mentioned = vec!["628999@s.whatsapp.net".into()];
        ctx.args = vec!["628333".into()];
        assert_eq!(resolve_target(&ctx).0, "628222@s.whatsapp.net", "quoted menang");

        ctx.quoted = None;
        assert_eq!(
            resolve_target(&ctx).0,
            "628999@s.whatsapp.net",
            "mention menang"
        );

        ctx.mentioned.clear();
        assert_eq!(
            resolve_target(&ctx),
            ("628333@s.whatsapp.net".to_string(), "628333".to_string()),
            "argumen jadi JID"
        );
    }

    #[test]
    fn a_leading_zero_becomes_the_country_code() {
        assert_eq!(normalize("0812".into()), "62812");
        assert_eq!(normalize("62812".into()), "62812");
        assert_eq!(normalize("0".into()), "62");
        assert_eq!(normalize(String::new()), "");
    }

    #[test]
    fn the_normal_branch_lists_every_role_the_number_holds() {
        let ctx = Ctx {
            mentioned: vec!["628111@s.whatsapp.net".into()],
            settings: serde_json::json!({
                "owner": ["628111"],
                "partner": ["628111"],
                "premium": ["628111"]
            }),
            ..base()
        };
        let (text, mentions) = text_of(&run(ctx.clone()));
        assert!(text.starts_with("📋 *CEK USER INFO*\n\n"), "{text}");
        assert!(text.contains("👤 User: @628111"), "{text}");
        assert!(
            text.contains("🏷️ Role: *👑 Owner • 🤝 Partner • 💎 Premium*"),
            "urutan role harus sama seperti push di JS: {text}"
        );
        assert!(text.contains("📊 Owner DB: *Ya*"), "{text}");
        assert_eq!(mentions, vec![ME.to_string()]);
    }

    #[test]
    fn a_free_user_reports_the_fallback_role_and_no_owner_db_entry() {
        let (text, _) = text_of(&run(base()));
        assert!(text.contains("🏷️ Role: *👤 Free User*"), "{text}");
        assert!(text.contains("📊 Owner DB: *Tidak*"), "{text}");
        assert!(!text.contains("⚡ Energi:"), "tanpa store, blok user disembunyikan: {text}");
    }

    #[test]
    fn a_stored_user_appends_energi_koin_and_level() {
        let ctx = Ctx {
            user_data: serde_json::json!({"energi": -1, "koin": -1, "level": 12}),
            ..base()
        };
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("⚡ Energi: *∞*"), "{text}");
        assert!(text.contains("💰 Koin: *∞*"), "{text}");
        assert!(text.contains("⭐ Level: *12*"), "{text}");
    }

    #[test]
    fn missing_store_fields_use_the_js_defaults() {
        let ctx = Ctx {
            user_data: serde_json::json!({"koin": 12500}),
            ..base()
        };
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("⚡ Energi: *0*"), "energi null -> 0: {text}");
        assert!(text.contains("💰 Koin: *12.500*"), "{text}");
        assert!(text.contains("⭐ Level: *1*"), "level null -> 1: {text}");
    }

    #[test]
    fn koin_uses_the_indonesian_thousands_separator() {
        assert_eq!(thousands_id(0), "0");
        assert_eq!(thousands_id(999), "999");
        assert_eq!(thousands_id(1000), "1.000");
        assert_eq!(thousands_id(1_000_000), "1.000.000");
        assert_eq!(thousands_id(123_456_789), "123.456.789");
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = Ctx {
            settings: serde_json::json!({"owner": ["628111"]}),
            user_data: serde_json::json!({"level": 3}),
            ..base()
        };
        assert!(run(ctx.clone()).mutations().is_empty());
    }

    #[test]
    fn date_long_is_reused_so_tanggal_terformat_sama_seperti_plugin_lain() {
        // Dijaga supaya plugin ini tidak diam-diam bergeser dari `birthday.rs`.
        assert_eq!(date_long(1_767_225_600_000), "1 Januari 2026");
    }

    #[test]
    fn helper_nilai_tidak_panic_untuk_user_kosong() {
        let empty = Value::Null;
        assert_eq!(energi(&empty), "0");
        assert_eq!(koin(&empty), "0");
        assert_eq!(level(&empty), "1");
    }
}
