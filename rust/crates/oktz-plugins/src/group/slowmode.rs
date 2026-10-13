//! `plugins/group/slowmode.js` — `groups[chat].slowmode = { enabled, delay, mode }`.
//!
//! Yang diport hanya perintahnya; `checkSlowmode()` diekspor ke `handler.js`
//! untuk dipakai sebagai filter pesan dan tidak punya padanan di `Ctx`.
//!
//! `ponytail:` `Mutation::SetGroup` merge, jadi `slowmode: { enabled: false }`
//! menimpa objek lama secara utuh — persis seperti `{...groupData, slowmode}`
//! di JS, bukan patch di dalam objek.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy, parse_int};

/// `PRESETS` (`slowmode.js:17-23`) — urutan object literal, dipakai untuk
/// daftar di layar status dan untuk mencari label delay.
pub(crate) const PRESETS: [(&str, i64); 5] = [
    ("santai", 10),
    ("normal", 30),
    ("ketat", 60),
    ("superketat", 120),
    ("max", 300),
];

/// `MODES` (`slowmode.js:25-28`).
const MODES: [(&str, &str); 2] = [
    ("all", "Semua pesan + command dihapus"),
    ("onlycommand", "Command di-silent, chat biasa tetap jalan"),
];

pub(crate) fn preset_of(name: &str) -> Option<i64> {
    PRESETS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// `Object.entries(PRESETS).find(([, v]) => v === delay)?.[0]`.
fn preset_label(delay: i64) -> Option<&'static str> {
    PRESETS.iter().find(|(_, v)| *v == delay).map(|(n, _)| *n)
}

fn mode_desc(mode: &str) -> &'static str {
    MODES.iter().find(|(n, _)| *n == mode).map(|(_, d)| *d).unwrap_or("")
}

pub(crate) const BAD_SUBCMD: &str = "❌ Gunakan *.slowmode on 30* atau *.slowmode onlycommand 30*";

pub(crate) const BAD_DELAY: &str = "❌ Delay harus antara 5–600 detik";

pub(crate) fn status(ctx: &Ctx) -> String {
    let sm = ctx.group_field("slowmode");
    let enabled = js_truthy(sm.and_then(|s| s.get("enabled")));
    let delay = match sm.and_then(|s| s.get("delay")) {
        Some(v) if js_truthy(Some(v)) => match v.as_str() {
            Some(s) => s.to_string(),
            None => v.to_string(),
        },
        _ => "30".to_string(),
    };
    let mode = match sm.and_then(|s| s.get("mode")) {
        Some(Value::String(m)) if !m.is_empty() => m.clone(),
        _ => "all".to_string(),
    };
    let preset_list: Vec<String> = PRESETS
        .iter()
        .map(|(name, sec)| format!("  *.slowmode {name}* — {sec}s"))
        .collect();

    format!(
        "🐢 *SLOWMODE*\n\n\
         Status: {}\n\
         Mode: *{mode}*\n\n\
         *Penggunaan:*\n\
         *.slowmode on 30* — semua pesan + command\n\
         *.slowmode onlycommand 30* — command only\n\
         *.slowmode off* — nonaktifkan\n\n\
         *Preset:*\n{}\n\n\
         *Mode:*\n\
         \x20\x20*all* — hapus semua pesan saat delay\n\
         \x20\x20*onlycommand* — silent command, chat bebas\n\n\
         _Admin & owner tidak terpengaruh_",
        if enabled {
            format!("✅ ON ({delay}s)")
        } else {
            "❌ OFF".to_string()
        },
        preset_list.join("\n"),
    )
}

#[plugin(
    name = "slowmode",
    alias = ["slow", "setslowmode"],
    category = "group",
    description = "Slowmode grup — batasi kecepatan pesan member",
    usage = ".slowmode <on/off/onlycommand> [detik]",
    example = ".slowmode on 30",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn slowmode(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let sub = sub.as_deref();

    if sub.is_none() || sub == Some("status") {
        return Reply::text(status(&ctx));
    }

    if sub == Some("off") {
        return Reply::text("✅ Slowmode *dinonaktifkan*").with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "slowmode": { "enabled": false } }),
        });
    }

    let mut mode = "all";
    let mut delay: Option<i64> = None;
    let delay_arg = ctx.args.get(1).map(String::as_str);

    if sub == Some("onlycommand") || sub == Some("oc") {
        mode = "onlycommand";
    } else if sub == Some("on") || sub == Some("set") {
        // mode tetap "all", delay dari argumen kedua.
    } else if let Some(preset) = sub.and_then(preset_of) {
        delay = Some(preset);
        mode = match delay_arg.map(str::to_lowercase) {
            Some(m) if m == "onlycommand" || m == "oc" => "onlycommand",
            _ => "all",
        };
    } else {
        match parse_int(sub.unwrap_or_default()) {
            Some(n) => delay = Some(n),
            None => return Reply::text(BAD_SUBCMD),
        }
    }

    // `if (!delay)` — `undefined` dan `0` dua-duanya minta delayArg. Di
    // dalamnya `parseInt(x) || 30`, jadi `0` dari argumen pun jatuh ke 30.
    if delay.is_none_or(|d| d == 0) {
        delay = delay_arg
            .and_then(preset_of)
            .or_else(|| delay_arg.and_then(parse_int))
            .filter(|d| *d != 0)
            .or(Some(30));
    }
    let delay = delay.unwrap_or(30);

    if !(5..=600).contains(&delay) {
        return Reply::text(BAD_DELAY);
    }

    let label = preset_label(delay).map(|p| format!(" ({p})")).unwrap_or_default();
    Reply::text(format!(
        "✅ Slowmode *aktif*\n\n\
         Delay: *{delay} detik*{label}\n\
         Mode: *{mode}*\n\
         {}\n\n\
         _Admin & owner tidak terpengaruh_",
        mode_desc(mode)
    ))
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({
            "slowmode": { "enabled": true, "delay": delay, "mode": mode }
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(group: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "slowmode".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("slowmode").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn slowmode_write(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data["slowmode"].clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("slowmode").unwrap();
        assert_eq!(p.meta.name, "slowmode");
        assert_eq!(p.meta.alias, ["slow", "setslowmode"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Slowmode grup — batasi kecepatan pesan member");
        assert_eq!(p.meta.usage, ".slowmode <on/off/onlycommand> [detik]");
        assert_eq!(p.meta.example, ".slowmode on 30");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["slow", "setslowmode"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "slowmode");
        }
    }

    #[test]
    fn the_status_branch_lists_every_preset_in_js_order() {
        let r = run(json!({}), &[]);
        let t = text_of(&r);
        assert!(t.starts_with("🐢 *SLOWMODE*\n\nStatus: ❌ OFF\nMode: *all*"), "{t}");
        for (name, sec) in PRESETS {
            assert!(t.contains(&format!("  *.slowmode {name}* — {sec}s\n")), "preset {name} hilang: {t}");
        }
        let order: Vec<usize> = PRESETS
            .iter()
            .map(|(n, _)| t.find(&format!("*.slowmode {n}*")).expect("ada"))
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "urutan preset: {order:?}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_status_keyword_and_missing_keyword_share_one_branch() {
        for args in [vec![], vec!["status"]] {
            let r = run(json!({}), &args);
            assert!(text_of(&r).starts_with("🐢 *SLOWMODE*"), "args {args:?}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_stored_configuration_is_reported_verbatim() {
        let r = run(json!({"slowmode": {"enabled": true, "delay": 45, "mode": "onlycommand"}}), &[]);
        let t = text_of(&r);
        assert!(t.contains("Status: ✅ ON (45s)"), "{t}");
        assert!(t.contains("Mode: *onlycommand*"), "{t}");
    }

    #[test]
    fn a_missing_delay_or_mode_falls_back_to_thirty_and_all() {
        let r = run(json!({"slowmode": {"enabled": true}}), &[]);
        assert!(text_of(&r).contains("Status: ✅ ON (30s)"), "{}", text_of(&r));
        assert!(text_of(&r).contains("Mode: *all*"), "{}", text_of(&r));
    }

    #[test]
    fn off_disables_without_touching_delay_or_mode() {
        let r = run(json!({"slowmode": {"enabled": true, "delay": 45}}), &["off"]);
        assert_eq!(text_of(&r), "✅ Slowmode *dinonaktifkan*");
        assert_eq!(slowmode_write(&r), json!({"enabled": false}));
    }

    #[test]
    fn on_takes_the_delay_from_the_second_argument() {
        let r = run(json!({}), &["on", "30"]);
        assert_eq!(slowmode_write(&r), json!({"enabled": true, "delay": 30, "mode": "all"}));
        assert!(text_of(&r).contains("Delay: *30 detik* (normal)"), "{}", text_of(&r));
        assert!(text_of(&r).contains("Semua pesan + command dihapus"), "{}", text_of(&r));
    }

    #[test]
    fn on_without_a_delay_falls_back_to_thirty() {
        let r = run(json!({}), &["on"]);
        assert_eq!(slowmode_write(&r)["delay"], json!(30));
    }

    #[test]
    fn onlycommand_and_its_shortcut_both_switch_the_mode() {
        for sub in ["onlycommand", "oc"] {
            let r = run(json!({}), &[sub, "45"]);
            assert_eq!(
                slowmode_write(&r),
                json!({"enabled": true, "delay": 45, "mode": "onlycommand"}),
                "sub {sub}"
            );
            assert!(text_of(&r).contains("Command di-silent, chat biasa tetap jalan"));
        }
    }

    #[test]
    fn a_bare_number_is_accepted_as_the_delay() {
        let r = run(json!({}), &["90"]);
        assert_eq!(slowmode_write(&r), json!({"enabled": true, "delay": 90, "mode": "all"}));
        assert!(
            text_of(&r).contains("Delay: *90 detik*\n"),
            "delay tanpa preset tidak boleh dapat label: {}",
            text_of(&r)
        );
    }

    #[test]
    fn a_preset_subcommand_can_also_switch_the_mode() {
        let r = run(json!({}), &["ketat", "onlycommand"]);
        assert_eq!(slowmode_write(&r), json!({"enabled": true, "delay": 60, "mode": "onlycommand"}));
        assert!(text_of(&r).contains("Delay: *60 detik* (ketat)"), "{}", text_of(&r));

        let r = run(json!({}), &["santai", "oc"]);
        assert_eq!(slowmode_write(&r), json!({"enabled": true, "delay": 10, "mode": "onlycommand"}));

        let r = run(json!({}), &["santai", "ngapain"]);
        assert_eq!(slowmode_write(&r)["mode"], json!("all"), "bukan mode yang dikenal -> all");
    }

    #[test]
    fn the_delay_argument_of_on_also_accepts_a_preset_name() {
        let r = run(json!({}), &["on", "max"]);
        assert_eq!(slowmode_write(&r)["delay"], json!(300));
        assert!(text_of(&r).contains("(max)"));
    }

    #[test]
    fn delays_outside_five_to_six_hundred_are_refused() {
        for d in ["4", "601", "-1", "1000000"] {
            let r = run(json!({}), &["on", d]);
            assert_eq!(text_of(&r), BAD_DELAY, "delay {d}");
            assert!(r.mutations().is_empty(), "delay {d}");
        }
        for d in [5, 600] {
            let r = run(json!({}), &["on", &d.to_string()]);
            assert_eq!(slowmode_write(&r)["delay"], json!(d), "batas {d} harus boleh");
        }
    }

    #[test]
    fn a_zero_delay_is_falsy_in_js_so_it_falls_back_to_thirty() {
        // `parseInt('0') || 30` — bukan error.
        for args in [&["on", "0"][..], &["0"][..]] {
            let r = run(json!({}), args);
            assert_eq!(slowmode_write(&r)["delay"], json!(30), "args {args:?}");
        }
    }

    #[test]
    fn a_non_numeric_subcommand_is_refused() {
        for sub in ["nyanyi", "onx"] {
            let r = run(json!({}), &[sub]);
            assert_eq!(text_of(&r), BAD_SUBCMD, "sub {sub}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn set_is_an_alias_for_on_inside_the_plugin_body() {
        // `set` bukan alias yang terdaftar, tapi tubuh JS memperlakukannya
        // sama dengan `on`.
        let r = run(json!({}), &["set", "20"]);
        assert_eq!(slowmode_write(&r), json!({"enabled": true, "delay": 20, "mode": "all"}));
    }
}