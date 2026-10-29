//! `plugins/group/mulaiabsen.js` — buka sesi absen di `global.absensi[chat]`.
//!
//! `global.absensi` adalah peta proses milik beberapa plugin (juga
//! `plugins/group/absen.js` dan `cekabsen.js`), jadi di sini satu kunci session
//! `absensi` yang isinya peta `chatId -> sesi`. Tidak ada `db.setGroup` di
//! plugin ini — sesi hidup di memori saja, sama seperti di JS.
//!
//! `ponytail:` `pruneAbsensi()` membaca `createdAt` yang di JS berupa string
//! ISO dari `new Date().toISOString()`. `Ctx` tidak punya parser tanggal,
//! jadi string itu diurai dengan fungsi lokal di bawah (bentuknya tetap
//! `Date#toISOString`, persis seperti yang ditulis JS).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value};

use crate::common::{JAKARTA_OFFSET_SECS, days_from_civil, jakarta_parts, jid_number, js_truthy_value, now_ms};

/// Kunci session untuk `global.absensi`.
pub(crate) const ABSENSI: &str = "absensi";

/// `ABSENSI_MAX_CHATS = 500` (`absen.js:15`).
const MAX_CHATS: usize = 500;
/// `ABSENSI_TTL_MS = 24 * 60 * 60 * 1000` (`absen.js:16`).
const TTL_MS: i64 = 24 * 60 * 60 * 1000;

pub(crate) const DEFAULT_PESAN: &str = "Absen Harian";

pub(crate) const STILL_RUNNING: &str = "❌ *ᴍᴀsɪʜ ᴀᴅᴀ ᴀʙsᴇɴ*\n\n\
> Masih ada sesi absen di grup ini!\n\n\
> Ketik *.hapusabsen* untuk menghapus\n\
> atau *.cekabsen* untuk melihat daftar";

/// `new Date().toISOString()` untuk milidetik yang sama — bentuk yang sama
/// persis dengan yang ditulis `mulaiabsen.js`, hanya tanpa milidetik (yang
/// tidak pernah dibaca: satu-satunya konsumennya `pruneAbsensi`).
fn iso_utc(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = jakarta_parts(ms - JAKARTA_OFFSET_SECS * 1000);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

/// `new Date(str).getTime()` untuk bentuk `Date#toISOString`. `None` kalau
/// bentuknya bukan itu — `|| 0` di JS menutupi kasus yang sama.
fn iso_ms(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let mo: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    let (hms, _frac) = time.split_once('.').unwrap_or((time, ""));
    let mut t = hms.split(':');
    let h: i64 = t.next()?.parse().ok()?;
    let mi: i64 = t.next()?.parse().ok()?;
    let sec: i64 = t.next()?.parse().ok()?;
    if t.next().is_some() {
        return None;
    }
    Some(
        days_from_civil(y, mo as u32, d as u32) * 86_400_000
            + (h * 3_600 + mi * 60 + sec) * 1000,
    )
}

/// `pruneAbsensi()` (`absen.js:21-31`) — buang entri yang sudah lebih tua
/// dari TTL, tapi hanya kalau peta sudah melewati batas 500 chat.
pub(crate) fn prune_absensi(ctx: &Ctx) {
    let cutoff = now_ms() - TTL_MS;
    ctx.session_update(ABSENSI, |v| {
        let Some(map) = v.as_object_mut() else {
            return;
        };
        if map.len() <= MAX_CHATS {
            return;
        }
        map.retain(|_, entry| {
            let created = entry
                .get("createdAt")
                .filter(|v| js_truthy_value(v))
                .or_else(|| entry.get("lastUpdate").filter(|v| js_truthy_value(v)))
                .or_else(|| entry.get("lastActive").filter(|v| js_truthy_value(v)));
            let ms = match created {
                Some(Value::Number(n)) => n.as_f64().map(|f| f as i64),
                Some(Value::String(s)) => iso_ms(s),
                _ => None,
            };
            // `(entry && (...)) || 0` — tanpa timestamp berarti 0, dan 0
            // selalu lebih tua dari cutoff, jadi ikut terhapus.
            matches!(ms, Some(ms) if ms != 0 && ms >= cutoff)
        });
    });
}

#[plugin(
    name = "mulaiabsen",
    alias = ["startabsen", "bukaabsen", "openabsen"],
    category = "group",
    description = "Mulai sesi absen di grup (admin only)",
    usage = ".mulaiabsen [keterangan]",
    example = ".mulaiabsen Rapat Mingguan",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn mulaiabsen(ctx: Ctx) -> Reply {
    prune_absensi(&ctx);

    let mut already_open = false;
    ctx.session_update(ABSENSI, |v| {
        if v.get(&ctx.chat).is_some_and(js_truthy_value) {
            already_open = true;
        }
    });
    if already_open {
        return Reply::text(STILL_RUNNING);
    }

    let keterangan = match ctx.text.trim() {
        "" => DEFAULT_PESAN.to_string(),
        t => t.to_string(),
    };

    ctx.session_update(ABSENSI, |v| {
        if !v.is_object() {
            *v = serde_json::json!({});
        }
        if let Some(map) = v.as_object_mut() {
            map.insert(
                ctx.chat.clone(),
                serde_json::json!({
                    "keterangan": keterangan,
                    "createdBy": ctx.sender,
                    "createdAt": iso_utc(now_ms()),
                    "peserta": []
                }),
            );
        }
    });

    Reply::Text {
        text: format!(
            "📋 *ABSEN UDAH JALAN NIHH*\n\n\
             「 📋 *ɪɴғᴏ* 」\n\
             📝 {keterangan}\n\
             👑 Dibuat oleh: @{sender}\n\
             👥 Peserta: 0\n\n\
             Untuk kamu yang mau ikutan absen, silahkan ketik *{prefix}absen*\
             Untuk admin yang mau cek absen, silahkan ketik *{prefix}cekabsen*\
             Untuk admin yang mau hapus absen, silahkan ketik *{prefix}hapusabsen*",
            sender = jid_number(&ctx.sender),
            prefix = ctx.prefix,
        ),
        mentions: vec![ctx.sender.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{SessionStore, new_session_store};

    const CHAT: &str = "120363@g.us";

    fn run(sessions: SessionStore, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mulaiabsen".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: CHAT.into(),
            is_group: true,
            text: text.into(),
            sessions,
            ..Default::default()
        };
        (oktz_plugin_api::find("mulaiabsen").unwrap().handler)(ctx)
    }

    fn fresh() -> SessionStore {
        new_session_store()
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn session_of(s: &SessionStore, chat: &str) -> Option<Value> {
        s.lock().unwrap().get(ABSENSI).and_then(|v| v.get(chat).cloned())
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("mulaiabsen").unwrap();
        assert_eq!(p.meta.name, "mulaiabsen");
        assert_eq!(p.meta.alias, ["startabsen", "bukaabsen", "openabsen"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mulai sesi absen di grup (admin only)");
        assert_eq!(p.meta.usage, ".mulaiabsen [keterangan]");
        assert_eq!(p.meta.example, ".mulaiabsen Rapat Mingguan");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["startabsen", "bukaabsen", "openabsen"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mulaiabsen");
        }
    }

    #[test]
    fn a_new_session_is_stored_under_this_chat() {
        let s = fresh();
        let r = run(s.clone(), "Rapat Mingguan");
        let entry = session_of(&s, CHAT).expect("sesi harus tersimpan");
        assert_eq!(entry["keterangan"], serde_json::json!("Rapat Mingguan"));
        assert_eq!(entry["createdBy"], serde_json::json!("628111@s.whatsapp.net"));
        assert_eq!(entry["peserta"], serde_json::json!([]));
        assert!(entry["createdAt"].as_str().unwrap().ends_with('Z'), "{entry}");
        assert!(r.mutations().is_empty(), "absensi bukan data store");
    }

    #[test]
    fn empty_text_falls_back_to_the_default_keterangan() {
        for text in ["", "   ", "\t"] {
            let s = fresh();
            run(s.clone(), text);
            assert_eq!(
                session_of(&s, CHAT).unwrap()["keterangan"],
                serde_json::json!(DEFAULT_PESAN),
                "input {text:?}"
            );
        }
    }

    #[test]
    fn the_announcement_mentions_the_creator_and_the_prefixed_commands() {
        let s = fresh();
        let r = run(s.clone(), "");
        assert_eq!(
            text_of(&r),
            format!(
                "📋 *ABSEN UDAH JALAN NIHH*\n\n\
                 「 📋 *ɪɴғᴏ* 」\n\
                 📝 {DEFAULT_PESAN}\n\
                 👑 Dibuat oleh: @628111\n\
                 👥 Peserta: 0\n\n\
                 Untuk kamu yang mau ikutan absen, silahkan ketik *.absen*\
                 Untuk admin yang mau cek absen, silahkan ketik *.cekabsen*\
                 Untuk admin yang mau hapus absen, silahkan ketik *.hapusabsen*"
            )
        );
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &["628111@s.whatsapp.net".to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_second_start_on_an_open_session_is_refused_and_keeps_the_old_one() {
        let s = fresh();
        run(s.clone(), "Pertama");
        let r = run(s.clone(), "Kedua");
        assert_eq!(text_of(&r), STILL_RUNNING);
        assert_eq!(
            session_of(&s, CHAT).unwrap()["keterangan"],
            serde_json::json!("Pertama"),
            "sesi yang sudah jalan tidak boleh ditimpa"
        );
    }

    #[test]
    fn other_chats_do_not_block_each_other() {
        let s = fresh();
        run(s.clone(), "");
        let ctx = Ctx {
            prefix: ".".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120364@g.us".into(),
            is_group: true,
            sessions: s.clone(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("mulaiabsen").unwrap().handler)(ctx);
        assert!(!text_of(&r).contains("MASIH ADA"), "{}", text_of(&r));
        assert!(session_of(&s, "120364@g.us").is_some());
    }

    #[test]
    fn prune_only_runs_past_the_chat_cap_and_only_drops_expired_entries() {
        let s = fresh();
        {
            let mut guard = s.lock().unwrap();
            let mut map = serde_json::Map::new();
            for i in 0..=MAX_CHATS {
                // 500 entri baru + 1 entri basi = satu di atas batas.
                let created = if i == 0 {
                    iso_utc(now_ms() - TTL_MS - 1000)
                } else {
                    iso_utc(now_ms())
                };
                map.insert(
                    format!("chat{i}@g.us"),
                    serde_json::json!({"createdAt": created, "peserta": []}),
                );
            }
            guard.insert(ABSENSI.to_string(), Value::Object(map));
        }
        let ctx = Ctx {
            chat: "lain@g.us".into(),
            sessions: s.clone(),
            ..Default::default()
        };
        prune_absensi(&ctx);
        let left = s.lock().unwrap().get(ABSENSI).unwrap().as_object().unwrap().len();
        assert_eq!(left, MAX_CHATS, "satu entri basi harus hilang, sisanya tetap");
        assert!(
            s.lock().unwrap().get(ABSENSI).unwrap().get("chat0@g.us").is_none(),
            "entri paling lama justru yang dibuang"
        );
    }

    #[test]
    fn prune_is_a_no_op_below_the_chat_cap() {
        let s = fresh();
        {
            let mut guard = s.lock().unwrap();
            guard.insert(
                ABSENSI.to_string(),
                serde_json::json!({"chat@g.us": {"createdAt": iso_utc(0), "peserta": []}}),
            );
        }
        let ctx = Ctx {
            sessions: s.clone(),
            ..Default::default()
        };
        prune_absensi(&ctx);
        assert!(
            session_of(&s, "chat@g.us").is_some(),
            "di bawah 500 chat tidak ada yang dibuang"
        );
    }

    #[test]
    fn the_iso_round_trip_survives_the_prune_boundary() {
        let ms = now_ms() - TTL_MS + 60_000;
        assert_eq!(iso_ms(&iso_utc(ms)), Some(ms - ms % 1000));
        assert_eq!(iso_ms("bukan tanggal"), None);
    }
}