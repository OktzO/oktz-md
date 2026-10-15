//! `plugins/group/anticustom.js` — pasang/pantau proteksi AntiCustom per grup.
//!
//! Yang dipindah: seluruh `handler` — status, guide, `on`/`off`, `metode`,
//! `list`, `del`, `cancel`, dan penanda wizard yang sedang berjalan.
//!
//! Yang tidak dipindah:
//! - `replyHandler` (`:307`) adalah entry point kedua (dijalankan untuk pesan
//!   biasa yang me-quote prompt), dan `PluginEntry` cuma punya satu `handler`.
//! - `sendPrompt` (`:127`) menyimpan `sent.key.id` ke `session.promptId`;
//!   `Reply` tidak pernah mengembalikan id pesan yang terkirim, jadi
//!   `promptId` selalu `null`. Tidak ada yang membacanya tanpa `replyHandler`.
//! - `refreshSessionTimeout`/`clearSession` memakai `setTimeout` 10 menit;
//!   `Ctx` tidak punya penjadwal, jadi sesi tidak kedaluwarsa sendiri.
//! - `parsePatternAnswer` + `analyzeCustomPattern` hanya dipakai
//!   `replyHandler`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::{Value, json};

/// Awalan key session — `global.anticustomSessions` punya peta sendiri di JS,
/// sedangkan `Ctx::sessions` satu map global untuk semua plugin.
const SESSION_PREFIX: &str = "anticustomSessions";

fn session_key(ctx: &Ctx) -> String {
    format!("{SESSION_PREFIX}:{}:{}", ctx.chat, ctx.sender)
}

/// `normalizeRules` (`anticustom.js:29`).
fn rules(group_data: &Value) -> Vec<Value> {
    group_data
        .get("anticustomRules")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter(|r| crate::common::js_truthy_value(r) && crate::common::js_truthy(r.get("pattern")))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// `normalizeAction(action, fallback)` (`anticustom.js:64`).
///
/// `String(action || fallback)` berarti argumen kosong/null ikut `fallback`;
/// `delete`/`hapus` diseragamkan jadi `remove`.
fn normalize_action(action: Option<&str>, fallback: &str) -> String {
    let value = action.filter(|a| !a.is_empty()).unwrap_or(fallback).to_lowercase();
    match value.as_str() {
        "kick" => "kick".to_string(),
        "remove" => "remove".to_string(),
        "delete" | "hapus" => "remove".to_string(),
        _ => fallback.to_string(),
    }
}

/// `formatRule(rule, index)` (`anticustom.js:35`).
fn format_rule(rule: &Value, index: usize) -> String {
    let title = ["groupName", "name"]
        .iter()
        .find_map(|k| rule.get(*k).filter(|v| crate::common::js_truthy_value(v)))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let rule_type = match rule.get("type").and_then(Value::as_str) {
        Some("regex") => "regex",
        _ => "contains",
    };
    let action = match rule.get("action") {
        Some(v) if crate::common::js_truthy_value(v) => v.as_str().unwrap_or(""),
        _ => "remove",
    };
    let pattern = rule.get("pattern").and_then(Value::as_str).unwrap_or("");
    format!(
        "{}. *{title}*\n> pattern: `{pattern}`\n> type: *{rule_type}*\n> action: *{action}*",
        index + 1
    )
}

/// `buildGuideMessage(m, status, mode, rules)` (`anticustom.js:182`).
fn guide(prefix: &str, status: &str, mode: &str, total: usize) -> String {
    format!(
        "🛡️ *ᴀɴᴛɪᴄᴜsᴛᴏᴍ*\n\n\
         > Status: *{status}*\n\
         > Mode default: *{mode}*\n\
         > Total rule: *{total}*\n\n\
         Kalau mau nambah AntiCustom lagi:\n\
         > `{prefix}anticustom add`\n\n\
         Kalau mau atur status:\n\
         > `{prefix}anticustom on`\n\
         > `{prefix}anticustom off`\n\n\
         Kalau mau lihat atau hapus rule:\n\
         > `{prefix}anticustom list`\n\
         > `{prefix}anticustom del <judul>`\n\n\
         Kalau mau ubah mode default:\n\
         > `{prefix}anticustom metode kick`\n\
         > `{prefix}anticustom metode remove`",
        status = status.to_uppercase(),
        mode = normalize_action(Some(mode), "remove").to_uppercase(),
    )
}

/// `startWizard` (`anticustom.js:132`) — bagian yang belum punya sesi.
fn wizard_prompt(first_setup: bool) -> String {
    let intro = if first_setup {
        "🛡️ *Selamat datang di setup AntiCustom*\n\n\
         Aku akan bantu bikin AntiCustom per sesi, langkah demi langkah.\n\n\
         *Alurnya:*\n\
         1. Tentukan judul rule\n\
         2. Isi kata atau pattern yang ingin dideteksi\n\
         3. Pilih action saat terdeteksi\n\
         4. Konfirmasi detail akhir\n\n"
    } else {
        "🛡️ *Yuk tambah rule AntiCustom baru*\n\n"
    };
    format!(
        "{intro}\
         *Pertanyaan 1/4*\n\
         Judulnya apa?\n\n\
         > Reply pesan ini dengan judul rule yang kamu mau\n\
         > Contoh: `Anti Kata Kotor`"
    )
}

fn start_wizard(ctx: &Ctx, mode: &str, first_setup: bool) -> Reply {
    let key = session_key(ctx);
    if ctx.session_get(&key).is_some() {
        return Reply::text(format!(
            "⚠️ Kamu masih punya sesi AntiCustom yang belum selesai.\n\n\
             > Balas pertanyaan terakhir bot untuk lanjut\n\
             > Atau batalkan dengan `{}anticustom cancel`",
            ctx.prefix
        ));
    }

    ctx.session_set(
        &key,
        json!({
            "chat": ctx.chat,
            "sender": ctx.sender,
            "step": "title",
            "title": "",
            "type": "contains",
            "patterns": [],
            "action": normalize_action(Some(mode), "remove"),
            // `sent.key.id` tidak bisa dilihat dari `Reply`, dan hanya
            // `replyHandler` yang membacanya.
            "promptId": Value::Null,
            "startedAt": crate::common::now_ms(),
            "timeout": Value::Null,
        }),
    );

    Reply::text(wizard_prompt(first_setup))
}

#[plugin(
    name = "anticustom",
    alias = ["antiaddcustom", "customanti"],
    category = "group",
    description = "Bikin AntiCustom lewat sesi tanya jawab per langkah",
    usage = ".anticustom <on/off/list/add/del/metode/cancel>",
    example = ".anticustom",
    cooldown = 3,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn anticustom(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let list = rules(&ctx.group_data);
    let mode = match ctx.group_field("anticustomMode") {
        Some(v) if crate::common::js_truthy_value(v) => v.as_str().unwrap_or("").to_string(),
        _ => "remove".to_string(),
    };
    let status = match ctx.group_field("anticustom") {
        Some(v) if crate::common::js_truthy_value(v) => v.as_str().unwrap_or("").to_string(),
        _ => "off".to_string(),
    };

    let Some(sub) = sub else {
        if list.is_empty() {
            return start_wizard(&ctx, &mode, true);
        }
        return Reply::text(guide(&ctx.prefix, &status, &mode, list.len()));
    };

    match sub.as_str() {
        "add" | "baru" | "new" | "buat" => start_wizard(&ctx, &mode, false),
        "cancel" | "batal" => {
            if ctx.session_get(&session_key(&ctx)).is_none() {
                return Reply::text("⚠️ Tidak ada sesi AntiCustom yang sedang berjalan.");
            }
            ctx.session_take(&session_key(&ctx));
            Reply::text("✅ Sesi AntiCustom dibatalkan.")
        }
        "on" => set_group(&ctx, json!({ "anticustom": "on" }), "✅ *AntiCustom diaktifkan*"),
        "off" => set_group(&ctx, json!({ "anticustom": "off" }), "❌ *AntiCustom dinonaktifkan*"),
        "metode" => {
            let action = normalize_action(ctx.args.get(1).map(String::as_str), "");
            if action.is_empty() {
                return Reply::text(
                    "❌ Gunakan: `.anticustom metode kick` atau `.anticustom metode remove`",
                );
            }
            let text = format!("✅ *Mode default AntiCustom sekarang {}*", action.to_uppercase());
            set_group(&ctx, json!({ "anticustom": "on", "anticustomMode": action }), text)
        }
        "list" => {
            if list.is_empty() {
                return Reply::text("📋 Belum ada rule AntiCustom di grup ini.");
            }
            let body: Vec<String> = list.iter().enumerate().map(|(i, r)| format_rule(r, i)).collect();
            Reply::text(format!(
                "📋 *ʟɪsᴛ ᴀɴᴛɪᴄᴜsᴛᴏᴍ*\n\n{}",
                body.join("\n\n")
            ))
        }
        "del" | "delete" | "remove" => {
            let name = ctx.args[1..].join(" ").trim().to_lowercase();
            if name.is_empty() {
                return Reply::text("❌ Format: `.anticustom del <judul>`");
            }
            let kept: Vec<Value> = list.iter().filter(|r| !rule_matches(r, &name)).cloned().collect();
            if kept.len() == list.len() {
                return Reply::text(format!(
                    "❌ Rule dengan judul `{name}` tidak ditemukan."
                ));
            }
            set_group(
                &ctx,
                json!({ "anticustomRules": kept }),
                format!("✅ Rule dengan judul `{name}` berhasil dihapus."),
            )
        }
        _ => Reply::text(
            "❌ Subcommand tidak valid. Gunakan: on, off, list, add, del, metode, cancel",
        ),
    }
}

/// `rules.filter(...)` di cabang `del`: rule hilang kalau nama atau groupName
/// sama persis dengan judul yang diketik, atau nama mulai dengan `"{judul} #"`
/// (suffix yang ditambahkan wizard saat satu rule punya banyak pattern).
fn rule_matches(rule: &Value, name: &str) -> bool {
    let rule_name = rule.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase();
    let group_name = rule.get("groupName").and_then(Value::as_str).unwrap_or("").to_lowercase();
    rule_name == name || group_name == name || rule_name.starts_with(&format!("{name} #"))
}

fn set_group(ctx: &Ctx, data: Value, text: impl Into<String>) -> Reply {
    Reply::text(text).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const CHAT: &str = "120363@g.us";
    const ME: &str = "628111@s.whatsapp.net";

    fn rule(name: &str, pattern: &str) -> Value {
        json!({ "name": name, "groupName": name, "pattern": pattern, "type": "contains", "action": "remove" })
    }

    fn run(args: &[&str], group: Value, sessions: Option<oktz_plugin_api::SessionStore>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "anticustom".into(),
            sender: ME.into(),
            chat: CHAT.into(),
            is_group: true,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            group_data: group,
            sessions: sessions.unwrap_or_else(oktz_plugin_api::new_session_store),
            ..Default::default()
        };
        (find("anticustom").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn written(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, CHAT);
                data.clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("anticustom").unwrap();
        assert_eq!(p.meta.name, "anticustom");
        assert_eq!(p.meta.alias, ["antiaddcustom", "customanti"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Bikin AntiCustom lewat sesi tanya jawab per langkah");
        assert_eq!(p.meta.usage, ".anticustom <on/off/list/add/del/metode/cancel>");
        assert_eq!(p.meta.example, ".anticustom");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["antiaddcustom", "customanti"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("anticustom"), "{a}");
        }
    }

    #[test]
    fn without_rule_it_starts_the_first_setup_wizard_and_stores_a_session() {
        let sessions = oktz_plugin_api::new_session_store();
        let r = run(&[], json!({}), Some(sessions.clone()));
        let t = text_of(&r);
        assert!(t.starts_with("🛡️ *Selamat datang di setup AntiCustom*"), "{t}");
        assert!(t.contains("*Alurnya:*\n1. Tentukan judul rule"), "{t}");
        assert!(t.contains("*Pertanyaan 1/4*"), "{t}");
        assert!(t.ends_with("> Contoh: `Anti Kata Kotor`"), "{t}");
        let key = format!("{SESSION_PREFIX}:{CHAT}:{ME}");
        let s = sessions.lock().unwrap().get(&key).cloned().expect("sesi tersimpan");
        assert_eq!(s["step"], json!("title"));
        assert_eq!(s["type"], json!("contains"));
        assert_eq!(s["action"], json!("remove"));
        assert_eq!(s["patterns"], json!([]));
    }

    #[test]
    fn an_existing_session_blocks_a_new_wizard() {
        let sessions = oktz_plugin_api::new_session_store();
        sessions.lock().unwrap().insert(
            format!("{SESSION_PREFIX}:{CHAT}:{ME}"),
            json!({ "step": "title" }),
        );
        for sub in [vec![], vec!["add"], vec!["baru"], vec!["new"], vec!["buat"]] {
            let r = run(&sub, json!({}), Some(sessions.clone()));
            let t = text_of(&r);
            assert!(
                t.starts_with("⚠️ Kamu masih punya sesi AntiCustom yang belum selesai."),
                "input {sub:?}: {t}"
            );
            assert!(t.contains("> Atau batalkan dengan `.anticustom cancel`"), "{t}");
            assert!(r.mutations().is_empty(), "{sub:?} tidak boleh menulis apa pun");
        }
    }

    #[test]
    fn a_non_first_wizard_uses_the_short_intro() {
        let r = run(&["add"], json!({}), None);
        let t = text_of(&r);
        assert!(t.starts_with("🛡️ *Yuk tambah rule AntiCustom baru*"), "{t}");
        assert!(!t.contains("Selamat datang"), "{t}");
        assert!(t.contains("*Pertanyaan 1/4*"), "{t}");
    }

    #[test]
    fn with_rules_the_default_action_prints_the_guide() {
        let r = run(&[], json!({ "anticustomRules": [rule("Kata Kotor", "anjing")] }), None);
        let t = text_of(&r);
        assert!(t.starts_with("🛡️ *ᴀɴᴛɪᴄᴜsᴛᴏᴍ*"), "{t}");
        assert!(t.contains("> Status: *OFF*"), "{t}");
        assert!(t.contains("> Mode default: *REMOVE*"), "{t}");
        assert!(t.contains("> Total rule: *1*"), "{t}");
        assert!(t.ends_with("> `.anticustom metode remove`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_guide_uppercases_status_and_normalizes_the_mode() {
        let g = json!({ "anticustom": "on", "anticustomMode": "kick", "anticustomRules": [rule("a", "b")] });
        let t = text_of(&run(&[], g, None));
        assert!(t.contains("> Status: *ON*"), "{t}");
        assert!(t.contains("> Mode default: *KICK*"), "{t}");

        // `delete` diseragamkan jadi `remove` oleh `normalizeAction`.
        let g = json!({ "anticustomMode": "delete", "anticustomRules": [rule("a", "b")] });
        let t = text_of(&run(&[], g, None));
        assert!(t.contains("> Mode default: *REMOVE*"), "{t}");
    }

    #[test]
    fn rules_without_a_pattern_are_ignored() {
        let g = json!({ "anticustomRules": [rule("ok", "x"), { "name": "kosong" }, json!(null), json!(0)] });
        let t = text_of(&run(&[], g, None));
        assert!(t.contains("> Total rule: *1*"), "{t}");
    }

    #[test]
    fn on_and_off_write_only_the_status_field() {
        assert_eq!(written(&run(&["on"], json!({}), None)), json!({ "anticustom": "on" }));
        assert_eq!(text_of(&run(&["on"], json!({}), None)), "✅ *AntiCustom diaktifkan*");
        assert_eq!(written(&run(&["off"], json!({}), None)), json!({ "anticustom": "off" }));
        assert_eq!(text_of(&run(&["off"], json!({}), None)), "❌ *AntiCustom dinonaktifkan*");
    }

    #[test]
    fn metode_maps_the_aliases_and_turns_the_plugin_on() {
        for (arg, stored, shown) in [
            ("kick", "kick", "KICK"),
            ("remove", "remove", "REMOVE"),
            ("delete", "remove", "REMOVE"),
            ("hapus", "remove", "REMOVE"),
        ] {
            let r = run(&["metode", arg], json!({}), None);
            assert_eq!(written(&r), json!({ "anticustom": "on", "anticustomMode": stored }), "{arg}");
            assert_eq!(text_of(&r), format!("✅ *Mode default AntiCustom sekarang {shown}*"), "{arg}");
        }
    }

    #[test]
    fn metode_without_a_valid_action_explains_the_syntax() {
        for args in [vec!["metode"], vec!["metode", "hajar"]] {
            let r = run(&args, json!({}), None);
            assert_eq!(
                text_of(&r),
                "❌ Gunakan: `.anticustom metode kick` atau `.anticustom metode remove`",
                "{args:?}"
            );
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn list_prints_numbered_rules_joined_by_a_blank_line() {
        let g = json!({ "anticustomRules": [
            { "name": "Jahat", "pattern": "tolol", "type": "regex", "action": "kick" },
            { "name": "Kotor", "pattern": "anjing" },
        ] });
        let t = text_of(&run(&["list"], g, None));
        assert!(t.starts_with("📋 *ʟɪsᴛ ᴀɴᴛɪᴄᴜsᴛᴏᴍ*\n\n1."), "{t}");
        assert!(t.contains("1. *Jahat*\n> pattern: `tolol`\n> type: *regex*\n> action: *kick*"), "{t}");
        assert!(
            t.contains("\n\n2. *Kotor*\n> pattern: `anjing`\n> type: *contains*\n> action: *remove*"),
            "{t}"
        );
    }

    #[test]
    fn list_uses_group_name_first_and_falls_back_to_a_dash() {
        let g = json!({ "anticustomRules": [
            { "groupName": "Grup", "name": "lain", "pattern": "x" },
            { "pattern": "y" },
        ] });
        let t = text_of(&run(&["list"], g, None));
        assert!(t.contains("1. *Grup*"), "{t}");
        assert!(t.contains("2. *-*"), "{t}");
    }

    #[test]
    fn list_without_rules_says_so() {
        let r = run(&["list"], json!({}), None);
        assert_eq!(text_of(&r), "📋 Belum ada rule AntiCustom di grup ini.");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn del_removes_the_matching_rule_and_writes_the_rest() {
        let g = json!({ "anticustomRules": [rule("Jahat", "a"), rule("Kotor", "b")] });
        let r = run(&["del", "Jahat"], g, None);
        assert_eq!(written(&r), json!({ "anticustomRules": [rule("Kotor", "b")] }));
        assert_eq!(text_of(&r), "✅ Rule dengan judul `jahat` berhasil dihapus.");
    }

    #[test]
    fn del_also_catches_the_numbered_variants_of_one_title() {
        // Wizard menamai rule `Judul #1`, `Judul #2` saat satu judul punya
        // banyak pattern.
        let g = json!({ "anticustomRules": [
            { "name": "Kata #1", "groupName": "Kata", "pattern": "a" },
            { "name": "Kata #2", "groupName": "Kata", "pattern": "b" },
            { "name": "Lain", "pattern": "c" },
        ] });
        let r = run(&["del", "kata"], g, None);
        assert_eq!(
            written(&r),
            json!({ "anticustomRules": [{ "name": "Lain", "pattern": "c" }] })
        );
    }

    #[test]
    fn del_needs_a_title_and_reports_an_unknown_one() {
        let r = run(&["del"], json!({ "anticustomRules": [rule("a", "b")] }), None);
        assert_eq!(text_of(&r), "❌ Format: `.anticustom del <judul>`");
        assert!(r.mutations().is_empty());

        let r = run(&["del", "TidakAda"], json!({ "anticustomRules": [rule("a", "b")] }), None);
        assert_eq!(text_of(&r), "❌ Rule dengan judul `tidakada` tidak ditemukan.");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn del_and_delete_and_remove_share_the_same_branch() {
        let g = json!({ "anticustomRules": [rule("Jahat", "a"), rule("Kotor", "b")] });
        for sub in ["del", "delete", "remove"] {
            let r = run(&[sub, "Kotor"], g.clone(), None);
            assert_eq!(written(&r), json!({ "anticustomRules": [rule("Jahat", "a")] }), "{sub}");
        }
    }

    #[test]
    fn cancel_clears_the_session_and_refuses_when_there_is_none() {
        let sessions = oktz_plugin_api::new_session_store();
        let r = run(&["cancel"], json!({}), Some(sessions.clone()));
        assert_eq!(text_of(&r), "⚠️ Tidak ada sesi AntiCustom yang sedang berjalan.");

        let key = format!("{SESSION_PREFIX}:{CHAT}:{ME}");
        for sub in ["cancel", "batal"] {
            // `clearSession` menghapus sesinya, jadi tiap sub perlu sesi baru.
            sessions
                .lock()
                .unwrap()
                .insert(key.clone(), json!({ "step": "title" }));
            let r = run(&[sub], json!({}), Some(sessions.clone()));
            assert_eq!(text_of(&r), "✅ Sesi AntiCustom dibatalkan.", "{sub}");
            assert!(!sessions.lock().unwrap().contains_key(&key), "{sub} harus menghapus sesi");
        }
    }

    #[test]
    fn a_session_of_another_user_does_not_block_the_wizard() {
        let sessions = oktz_plugin_api::new_session_store();
        sessions
            .lock()
            .unwrap()
            .insert(format!("{SESSION_PREFIX}:{CHAT}:628999@s.whatsapp.net"), json!({}));
        let r = run(&["add"], json!({}), Some(sessions));
        assert!(text_of(&r).starts_with("🛡️ *Yuk tambah rule AntiCustom baru*"));
    }

    #[test]
    fn an_unknown_subcommand_lists_the_valid_ones() {
        let r = run(&["ngapain"], json!({}), None);
        assert_eq!(
            text_of(&r),
            "❌ Subcommand tidak valid. Gunakan: on, off, list, add, del, metode, cancel"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_guide_keeps_the_prefix_inside_the_markdown() {
        let g = json!({ "anticustomRules": [rule("a", "b")] });
        let ctx = Ctx {
            prefix: "!".into(),
            command: "anticustom".into(),
            chat: CHAT.into(),
            is_group: true,
            group_data: g,
            ..Default::default()
        };
        let t = text_of(&(find("anticustom").unwrap().handler)(ctx));
        assert!(t.contains("> `!anticustom add`"), "{t}");
        assert!(t.contains("> `!anticustom del <judul>`"), "{t}");
    }
}