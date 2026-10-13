//! `plugins/group/absen.js` — tandai kehadiran di sesi absen grup.

use serde_json::{Value, json};
use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{date_long, days_from_civil, jid_number, now_ms};

/// `global.absensi` — satu key session, objek `chatId -> sesi`.
pub(crate) const ABSENSI_KEY: &str = "absensi";

pub(crate) const ABSENSI_MAX_CHATS: usize = 500;
pub(crate) const ABSENSI_TTL_MS: i64 = 24 * 60 * 60 * 1000;

pub(crate) const NO_SESSION: &str = "❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ᴀʙsᴇɴ*\n\n> Belum ada sesi absen di grup ini!\n\n> Admin dapat memulai dengan\n> *.mulaiabsen [keterangan]*";
pub(crate) const ALREADY: &str = "❌ Kamu sudah absen!";

/// `new Date(s).getTime()` untuk keluaran `Date#toISOString()` (UTC, `Z`).
///
/// `ponytail:` milidetik dan offset diabaikan — satu-satunya penulisnya
/// `new Date().toISOString()` (`mulaiabsen.js:41`), jadi bentuknya selalu
/// `YYYY-MM-DDTHH:mm:ss.sssZ`. Naik ke parser penuh kalau ada sumber lain.
pub(crate) fn iso_ms(raw: &str) -> Option<i64> {
    let (date, rest) = raw.split_once('T')?;
    let mut d = date.split('-');
    let y = d.next()?.parse::<i64>().ok()?;
    let m = d.next()?.parse::<u32>().ok()?;
    let day = d.next()?.parse::<u32>().ok()?;
    let clock: String = rest.chars().take(8).collect();
    let mut t = clock.split(':');
    let h = t.next()?.parse::<i64>().ok()?;
    let mi = t.next()?.parse::<i64>().ok()?;
    let s = t.next()?.parse::<i64>().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some((days_from_civil(y, m, day) * 86_400 + h * 3600 + mi * 60 + s) * 1000)
}

/// `createdAt` sebagai milidetik, dengan fallback `lastUpdate`/`lastActive`
/// (`absen.js:22`). String diparse sebagai ISO, angka dipakai apa adanya.
pub(crate) fn entry_created_ms(entry: &Value) -> i64 {
    let raw = ["createdAt", "lastUpdate", "lastActive"]
        .iter()
        .find_map(|k| entry.get(*k));
    match raw {
        Some(Value::Number(n)) => n.as_f64().map_or(0, |f| f as i64),
        Some(Value::String(s)) => iso_ms(s).unwrap_or(0),
        _ => 0,
    }
}

/// `pruneAbsensi()` — buang sesi yang lewat TTL, tapi hanya saat jumlahnya
/// sudah melewati batas; pencadangan tidak jalan pada kondisi normal.
pub(crate) fn prune_absensi(ctx: &Ctx) {
    let cutoff = now_ms() - ABSENSI_TTL_MS;
    ctx.session_update(ABSENSI_KEY, |slot| {
        if !slot.is_object() {
            return;
        }
        let map = slot.as_object_mut().expect("sudah dicek is_object");
        if map.len() <= ABSENSI_MAX_CHATS {
            return;
        }
        map.retain(|_, entry| {
            let created = entry_created_ms(entry);
            created == 0 || created >= cutoff
        });
    });
}

/// Sesi aktif untuk `chat`, atau `None` kalau belum ada / belum berbentuk objek.
pub(crate) fn session_for(ctx: &Ctx) -> Option<Value> {
    let all = ctx.session_get(ABSENSI_KEY)?;
    let entry = all.get(&ctx.chat)?.clone();
    entry.is_object().then_some(entry)
}

/// Peserta sesi sebagai JID, urut seperti `absen.peserta` di JS.
pub(crate) fn peserta_of(entry: &Value) -> Vec<String> {
    entry
        .get("peserta")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

/// Simpan `peserta` terbaru ke `global.absensi[chatId]`, tanpa menyentuh field
/// lain (`mulaiabsen.js` yang menulis `keterangan`/`createdBy`/`createdAt`).
pub(crate) fn store_peserta(ctx: &Ctx, peserta: &[String]) {
    let chat = ctx.chat.clone();
    let list: Vec<Value> = peserta.iter().cloned().map(Value::String).collect();
    ctx.session_update(ABSENSI_KEY, |slot| {
        if !slot.is_object() {
            *slot = json!({});
        }
        let map = slot.as_object_mut().expect("sudah diganti objek");
        map.entry(chat.clone())
            .or_insert_with(|| json!({ "peserta": [] }));
        if let Some(entry) = map.get_mut(&chat).and_then(Value::as_object_mut) {
            entry.insert("peserta".into(), Value::Array(list.clone()));
        }
    });
}

/// `┃ ${i + 1}. @${jid.split('@')[0]}` digabung `\n`.
pub(crate) fn presence_list(peserta: &[String]) -> String {
    peserta
        .iter()
        .enumerate()
        .map(|(i, jid)| format!("┃ {}. @{}", i + 1, jid_number(jid)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[plugin(
    name = "absen",
    alias = ["hadir", "present"],
    category = "group",
    description = "Tandai kehadiran di sesi absen",
    usage = ".absen",
    example = ".absen",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn absen(ctx: Ctx) -> Reply {
    prune_absensi(&ctx);
    let Some(absen) = session_for(&ctx) else {
        return Reply::text(NO_SESSION);
    };
    let mut peserta = peserta_of(&absen);
    if peserta.iter().any(|jid| jid == &ctx.sender) {
        return Reply::text(ALREADY);
    }
    peserta.push(ctx.sender.clone());
    store_peserta(&ctx, &peserta);

    let keterangan = absen.get("keterangan").and_then(Value::as_str).unwrap_or("");
    Reply::Text {
        text: format!(
            "✅ *MANTAP, @{} HADIRR*\n\
             TUJUAN ABSEN: {}\n\
             ╭┈┈⬡「 📋 INFO LAIN 」\n\
             ┃ 📅 {}\n\
             ┃ 👥 Total: {}\n\
             ├┈┈⬡「 📝 *ᴅᴀғᴛᴀʀ ʜᴀᴅɪʀ* 」\n\
             {}\n\
             ╰┈┈┈┈┈┈┈┈⬡\n\n\
             > _Ketik *{}absen* untuk hadir_\n\
             > _Ketik *{}cekabsen* untuk melihat daftar_",
            jid_number(&ctx.sender),
            keterangan,
            date_long(now_ms()),
            peserta.len(),
            presence_list(&peserta),
            ctx.prefix,
            ctx.prefix,
        ),
        mentions: peserta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628222@s.whatsapp.net";
    const CHAT: &str = "120363@g.us";

    fn run(session: Option<Value>, sender: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "absen".into(),
            sender: sender.into(),
            chat: CHAT.into(),
            is_group: true,
            ..Default::default()
        };
        if let Some(v) = session {
            ctx.session_set(ABSENSI_KEY, v);
        }
        (oktz_plugin_api::find("absen").unwrap().handler)(ctx)
    }

    fn open_session(peserta: Vec<&str>) -> Value {
        json!({
            CHAT: {
                "keterangan": "Absen Harian",
                "createdBy": OTHER,
                "createdAt": "2024-05-01T02:03:04.000Z",
                "peserta": peserta,
            }
        })
    }

    fn text_of(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions.clone(),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("absen").unwrap();
        assert_eq!(p.meta.name, "absen");
        assert_eq!(p.meta.alias, ["hadir", "present"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Tandai kehadiran di sesi absen");
        assert_eq!(p.meta.usage, ".absen");
        assert_eq!(p.meta.example, ".absen");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private, "default JS isPrivate=false");
        assert!(!p.meta.is_admin, "tidak ada di pluginConfig");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["hadir", "present"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "absen");
        }
    }

    #[test]
    fn without_a_session_the_reply_points_at_mulaiabsen() {
        let r = run(None, SENDER);
        assert_eq!(text_of(&r), NO_SESSION);
        assert!(mentions_of(&r).is_empty(), "tidak ada yang di-mention");
    }

    #[test]
    fn a_session_for_another_chat_is_not_this_groups_session() {
        let r = run(Some(json!({"120999@g.us": {"keterangan": "x", "peserta": []}})), SENDER);
        assert_eq!(text_of(&r), NO_SESSION);
    }

    #[test]
    fn an_existing_participant_cannot_absen_twice() {
        let r = run(Some(open_session(vec![SENDER])), SENDER);
        assert_eq!(text_of(&r), ALREADY);
    }

    #[test]
    fn presence_appends_the_sender_and_mentions_every_participant() {
        let ctx = Ctx {
            prefix: ".".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            ..Default::default()
        };
        ctx.session_set(ABSENSI_KEY, open_session(vec![OTHER]));
        let r = (oktz_plugin_api::find("absen").unwrap().handler)(ctx);

        assert_eq!(mentions_of(&r), vec![OTHER.to_string(), SENDER.to_string()]);
        assert!(text_of(&r).contains("┃ 1. @628222"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ 2. @628111"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ 👥 Total: 2"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("TUJUAN ABSEN: Absen Harian"),
            "keterangan ikut: {}",
            text_of(&r)
        );
    }

    #[test]
    fn the_session_is_persisted_for_the_next_message() {
        let ctx = Ctx {
            sender: SENDER.into(),
            chat: CHAT.into(),
            ..Default::default()
        };
        ctx.session_set(ABSENSI_KEY, open_session(vec![]));
        (oktz_plugin_api::find("absen").unwrap().handler)(ctx.clone());

        let after = ctx.session_get(ABSENSI_KEY).unwrap();
        assert_eq!(peserta_of(after.get(CHAT).unwrap()), vec![SENDER.to_string()]);
        assert_eq!(
            after[CHAT]["keterangan"], json!("Absen Harian"),
            "field lain milik mulaiabsen tidak boleh tertimpa"
        );
    }

    #[test]
    fn a_second_run_from_the_same_sender_is_refused_after_storing() {
        let ctx = Ctx {
            sender: SENDER.into(),
            chat: CHAT.into(),
            ..Default::default()
        };
        ctx.session_set(ABSENSI_KEY, open_session(vec![]));
        let handler = oktz_plugin_api::find("absen").unwrap().handler.clone();
        handler(ctx.clone());
        let second = handler(ctx.clone());
        assert_eq!(text_of(&second), ALREADY);
        assert_eq!(
            peserta_of(ctx.session_get(ABSENSI_KEY).unwrap().get(CHAT).unwrap()).len(),
            1,
            "tidak boleh jadi dua entri"
        );
    }

    #[test]
    fn iso_ms_reads_the_iso_shape_mulaiabsen_writes() {
        // 2024-05-01T02:03:04Z = 1714528984000
        assert_eq!(iso_ms("2024-05-01T02:03:04.000Z"), Some(1_714_528_984_000));
        assert_eq!(iso_ms("bukan tanggal"), None);
        assert_eq!(iso_ms("2024-13-01T00:00:00.000Z"), None, "bulan 13");
        assert_eq!(iso_ms("2024-01-01T99:00:00.000Z"), None, "jam 99");
    }

    #[test]
    fn entry_created_falls_back_through_last_update_and_last_active() {
        assert_eq!(
            entry_created_ms(&json!({"createdAt": 1_714_528_984_000i64})),
            1_714_528_984_000
        );
        assert_eq!(
            entry_created_ms(&json!({"lastUpdate": "2024-05-01T02:03:04.000Z"})),
            1_714_528_984_000
        );
        assert_eq!(entry_created_ms(&json!({"lastActive": 7})), 7);
        assert_eq!(entry_created_ms(&json!({})), 0, "tidak ada = 0");
        assert_eq!(
            entry_created_ms(&json!({"createdAt": "rusak"})),
            0,
            "string rusak = 0, bukan panic"
        );
    }

    #[test]
    fn pruning_only_runs_past_the_chat_limit() {
        let mut sessions = serde_json::Map::new();
        for i in 0..ABSENSI_MAX_CHATS {
            sessions.insert(format!("g{i}"), json!({ "createdAt": now_ms() }));
        }
        let ctx = Ctx::default();
        ctx.session_set(ABSENSI_KEY, Value::Object(sessions.clone()));
        prune_absensi(&ctx);
        assert_eq!(
            ctx.session_get(ABSENSI_KEY).unwrap().as_object().unwrap().len(),
            ABSENSI_MAX_CHATS,
            "tepat di batas tidak ada yang dibuang"
        );

        let mut sessions = sessions;
        sessions.insert("kejadian".into(), json!({ "createdAt": now_ms() }));
        ctx.session_set(ABSENSI_KEY, Value::Object(sessions));
        prune_absensi(&ctx);
        assert_eq!(
            ctx.session_get(ABSENSI_KEY).unwrap().as_object().unwrap().len(),
            ABSENSI_MAX_CHATS + 1,
            "sesi masih segar tidak boleh dibuang"
        );
    }

    #[test]
    fn a_session_without_a_timestamp_survives_pruning() {
        // `createdAt` 0 = belum pernah ada; `if (created && ...)` di JS
        // menyisakannya, jadi pencadangan tidak boleh ikut membuangnya.
        let mut sessions = serde_json::Map::new();
        for i in 0..ABSENSI_MAX_CHATS {
            sessions.insert(format!("g{i}"), json!({ "createdAt": 0 }));
        }
        sessions.insert("tanpa-waktu".into(), json!({ "peserta": [] }));
        let ctx = Ctx::default();
        ctx.session_set(ABSENSI_KEY, Value::Object(sessions));
        prune_absensi(&ctx);
        let left = ctx.session_get(ABSENSI_KEY).unwrap();
        assert!(left.get("tanpa-waktu").is_some(), "{left}");
    }

    #[test]
    fn pruning_drops_sessions_past_the_ttl_once_over_the_limit() {
        let mut sessions = serde_json::Map::new();
        for i in 0..ABSENSI_MAX_CHATS {
            sessions.insert(format!("g{i}"), json!({ "createdAt": now_ms() }));
        }
        sessions.insert("basi".into(), json!({ "createdAt": now_ms() - ABSENSI_TTL_MS - 1 }));
        let ctx = Ctx::default();
        ctx.session_set(ABSENSI_KEY, Value::Object(sessions));
        prune_absensi(&ctx);

        let left = ctx.session_get(ABSENSI_KEY).unwrap();
        assert!(left.get("basi").is_none(), "sesi lewat TTL dibuang");
        assert_eq!(left.as_object().unwrap().len(), ABSENSI_MAX_CHATS);
    }
}