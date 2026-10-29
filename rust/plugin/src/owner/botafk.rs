//! `plugins/owner/botafk.js` — toggle `setting('botAfk')`.
//!
//! Plugin ini adalah TOGGLE tanpa argumen: kalau bot sedang AFK, satu
//! perintah apa pun akan mengembalikannya online dan melaporkan durasinya.
//! Durasi dihitung dari `since` yang disimpan, jadi store adalah sumber kebenaran
//! tunggal — tidak ada state in-memory terpisah seperti di JS.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{format_duration, now_ms};

/// `currentAfk && currentAfk.active` — `active` WAJIB true; objek yang ada
/// tapi `active: false` berarti sudah offline.
pub(crate) fn is_afk(ctx: &Ctx) -> bool {
    ctx.setting("botAfk")
        .and_then(|v| v.get("active"))
        .is_some_and(crate::common::js_truthy_value)
}

/// `formatNow('HH:mm:ss')` — `common::time_hm` hanya memberi HH:mm, jadi detik
/// dihitung ulang dari epoch yang sama.
pub(crate) fn now_hms(now: i64) -> String {
    let secs = now.div_euclid(1000).rem_euclid(86_400);
    format!("{h:02}:{m:02}:{s:02}", h = secs / 3600, m = (secs % 3600) / 60, s = secs % 60)
}

pub(crate) fn offline(duration: &str, reason: &str) -> String {
    format!(
        "✅ *ʙᴏᴛ ᴋᴇᴍʘᴀʟɪ ᴏɴʟɪɴᴇ*\n\n\
         ╭┈┈⬡「 📊 *sᴛᴀᴛɪsᴛɪᴋ ᴀꜰᴋ* 」\n\
         ┃ ⏱️ ᴅᴜʀᴀsɪ: `{duration}`\n\
         ┃ 📝 ᴀʟᴀsᴀɴ: `{reason}`\n\
         ╰┈┈⬡\n\n\
         > Bot siap menerima command!"
    )
}

pub(crate) fn active(prefix: &str, reason: &str, now: i64) -> String {
    format!(
        "💤 *ʙᴏᴛ ᴀꜰᴋ ᴀᴋᴛɪꜰ*\n\n\
         ╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n\
         ┃ 📝 ᴀʟᴀsᴀɴ: `{reason}`\n\
         ┃ ⏰ sᴇᴊᴀᴋ: `{stamp}`\n\
         ╰┈┈⬡\n\n\
         ╭┈┈⬡「 🔒 *ᴀᴋsᴇs* 」\n\
         ┃ ✅ Owner bot\n\
         ┃ ✅ Bot sendiri (fromMe)\n\
         ┃ ❌ Semua user lain\n\
         ╰┈┈⬡\n\n\
         > User lain akan dapat pesan AFK\n\
         > Ketik `{prefix}botafk` untuk kembali online",
        stamp = now_hms(now)
    )
}

#[plugin(
    name = "botafk",
    alias = ["afkbot", "afkmode"],
    category = "owner",
    description = "Mode AFK untuk bot - bot tidak merespon command, hanya reply pesan AFK",
    usage = ".botafk <alasan>",
    example = ".botafk Lagi istirahat",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn botafk(ctx: Ctx) -> Reply {
    let now = now_ms();

    if is_afk(&ctx) {
        // `Date.now() - currentAfk.since` — `since` yang tidak ada berarti
        // NaN di JS; `unwrap_or(now)` supaya tidak jadi durasi gila.
        let since = ctx
            .setting("botAfk")
            .and_then(|v| v.get("since"))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(now);
        let reason = ctx
            .setting("botAfk")
            .and_then(|v| v.get("reason"))
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("-");
        return Reply::text(offline(&format_duration(now - since), reason))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "botAfk".into(),
                value: serde_json::json!({ "active": false }),
            });
    }

    // `m.args.join(' ') || 'AFK'` — string kosong (atau whitespace-only yang
    // sudah jadi satu token) jatuh ke default.
    let reason = ctx.args.join(" ");
    let reason = if reason.is_empty() { "AFK".to_string() } else { reason };

    Reply::text(active(&ctx.prefix, &reason, now))
        .with_reaction("💤")
        .with_mutation(Mutation::SetSetting {
            key: "botAfk".into(),
            value: serde_json::json!({
                "active": true,
                "reason": reason,
                "since": now,
            }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    const SINCE: i64 = 1_700_000_000_000;

    fn run(settings: Value, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "botafk".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("botafk").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "botAfk");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("botafk").unwrap();
        assert_eq!(p.meta.name, "botafk");
        assert_eq!(p.meta.alias, ["afkbot", "afkmode"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mode AFK untuk bot - bot tidak merespon command, hanya reply pesan AFK"
        );
        assert_eq!(p.meta.usage, ".botafk <alasan>");
        assert_eq!(p.meta.example, ".botafk Lagi istirahat");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["afkbot", "afkmode"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "botafk");
        }
    }

    #[test]
    fn without_a_reason_the_bot_goes_afk_with_the_default_label() {
        let r = run(serde_json::json!({}), "");
        let data = write_of(&r);
        assert_eq!(data["active"], serde_json::json!(true));
        assert_eq!(data["reason"], serde_json::json!("AFK"), "`|| 'AFK'`");
        assert!(data["since"].as_i64().unwrap() > 0, "since harus diisi");
        assert!(text_of(&r).starts_with("💤 *ʙᴏᴛ ᴀꜰᴋ ᴀᴋᴛɪꜰ*"));
    }

    #[test]
    fn a_reason_is_joined_from_all_arguments() {
        let r = run(serde_json::json!({}), "lagi istirahat sebentar");
        assert_eq!(write_of(&r)["reason"], serde_json::json!("lagi istirahat sebentar"));
        assert!(text_of(&r).contains("┃ 📝 ᴀʟᴀsᴀɴ: `lagi istirahat sebentar`"));
    }

    #[test]
    fn a_second_invocation_brings_the_bot_back_online() {
        let r = run(serde_json::json!({"botAfk": {"active": true, "reason": "istirahat", "since": SINCE}}), "");
        let data = write_of(&r);
        assert_eq!(data["active"], serde_json::json!(false));
        assert_eq!(
            data.as_object().unwrap().len(),
            1,
            "cabang off hanya menulis `active: false`"
        );
        assert!(text_of(&r).starts_with("✅ *ʙᴏᴛ ᴋᴇᴍʘᴀʟɪ ᴏɴʟɪɴᴇ*"));
        assert!(text_of(&r).contains("┃ 📝 ᴀʟᴀsᴀɴ: `istirahat`"));
    }

    #[test]
    fn the_offline_reply_reports_the_stored_duration() {
        // Durasi dihitung dari `since`, jadi reply berubah seiring waktu; yang
        // dikunci adalah bentuknya dan sumber angkanya.
        let old = now_ms() - 90 * 60 * 1000;
        let r = run(
            serde_json::json!({"botAfk": {"active": true, "reason": "x", "since": old}}),
            "",
        );
        assert!(
            text_of(&r).contains("1 jam 30 menit") || text_of(&r).contains("1 jam 3"),
            "durasi harus dihitung dari since: {}",
            text_of(&r)
        );
    }

    #[test]
    fn a_stored_afk_without_a_reason_reports_a_dash() {
        let r = run(serde_json::json!({"botAfk": {"active": true, "since": SINCE}}), "");
        assert!(
            text_of(&r).contains("┃ 📝 ᴀʟᴀsᴀɴ: `-`"),
            "`currentAfk.reason || '-'`"
        );
    }

#[test]
    fn an_empty_reason_string_also_falls_back_to_the_dash() {
        let r = run(
            serde_json::json!({"botAfk": {"active": true, "reason": "", "since": SINCE}}),
            "",
        );
        let dash = format!("`{dash}`", dash = "-");
        assert!(
            text_of(&r).contains(&dash),
            "alasan kosong harus jadi tanda hubung: {}",
            text_of(&r)
        );
    }

    #[test]
    fn a_record_with_active_false_goes_back_into_afk_not_offline() {
        let r = run(serde_json::json!({"botAfk": {"active": false, "since": SINCE}}), "");
        assert_eq!(
            write_of(&r)["active"],
            serde_json::json!(true),
            "`currentAfk && currentAfk.active` exige active === true"
        );
    }

    #[test]
    fn the_two_branches_react_differently() {
        match run(serde_json::json!({}), "").without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "💤"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        match run(serde_json::json!({"botAfk": {"active": true, "since": SINCE}}), "").without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_active_reply_names_the_live_prefix_and_shows_a_time() {
        let r = run(serde_json::json!({}), "x");
        let t = text_of(&r);
        assert!(t.contains("Ketik `.botafk` untuk kembali online"), "{t}");
        let stamp_line = t
            .lines()
            .find(|l| l.contains("sᴇᴊᴀᴋ"))
            .expect("baris waktu");
        let stamp = stamp_line.split('`').nth(1).unwrap_or("");
        assert_eq!(stamp.split(':').count(), 3, "HH:mm:ss: {stamp}");
    }

    #[test]
    fn each_branch_is_paired_with_exactly_one_write() {
        assert_eq!(run(serde_json::json!({}), "").mutations().len(), 1);
        assert_eq!(
            run(serde_json::json!({"botAfk": {"active": true, "since": SINCE}}), "")
                .mutations()
                .len(),
            1
        );
    }

    #[test]
    fn the_time_helper_formats_hours_minutes_seconds() {
        let h = now_hms(12 * 3600 * 1000 + 34 * 60 * 1000 + 56 * 1000);
        assert_eq!(h, "12:34:56");
        // Nol di depan, bukan spasi.
        assert_eq!(now_hms(9 * 3600 * 1000 + 5 * 60 * 1000 + 7 * 1000), "09:05:07");
    }
}