//! `plugins/group/hapusabsen.js` — hapus sesi absen milik grup ini.
//!
//! Sesi tidak ada di `db`, tapi di peta proses `global.absensi` yang sama
//! dengan [`crate::group::mulaiabsen`] — jadi keduanya baca/tulis kunci
//! session `absensi`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::group::mulaiabsen::ABSENSI;

pub(crate) const NO_SESSION: &str = "❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ᴀʙsᴇɴ*\n\n> Tidak ada sesi absen di grup ini!";

/// Ambil lalu hapus entri `chat` — `delete global.absensi[chatId]` dalam satu
/// lock. `None` kalau memang tidak ada sesi.
pub(crate) fn take_session(ctx: &Ctx) -> Option<Value> {
    let mut taken = None;
    ctx.session_update(ABSENSI, |v| {
        taken = v.as_object_mut().and_then(|m| m.remove(&ctx.chat));
    });
    taken
}

#[plugin(
    name = "hapusabsen",
    alias = ["deleteabsen", "tutupabsen", "closeabsen", "resetabsen"],
    category = "group",
    description = "Hapus/tutup sesi absen (admin only)",
    usage = ".hapusabsen",
    example = ".hapusabsen",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn hapusabsen(ctx: Ctx) -> Reply {
    let Some(session) = take_session(&ctx) else {
        return Reply::text(NO_SESSION);
    };

    let total = session
        .get("peserta")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let keterangan = session
        .get("keterangan")
        .and_then(Value::as_str)
        .unwrap_or("undefined");

    Reply::text(format!(
        "✅ *ABSEN DITUTUP!*\n\n\
         Penyebab?\n\
         📝 {keterangan}\n\
         👥 Total hadir: {total}\n\n\
         Sesi absen telah dihapus."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::group::mulaiabsen::{ABSENSI, DEFAULT_PESAN};
    use oktz_plugin_api::{SessionStore, new_session_store};
    use serde_json::json;

    const CHAT: &str = "120363@g.us";

    fn run(sessions: SessionStore) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hapusabsen".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: CHAT.into(),
            is_group: true,
            sessions,
            ..Default::default()
        };
        (oktz_plugin_api::find("hapusabsen").unwrap().handler)(ctx)
    }

    fn with_session(entry: serde_json::Value) -> SessionStore {
        let s = new_session_store();
        s.lock()
            .unwrap()
            .insert(ABSENSI.to_string(), json!({ CHAT: entry }));
        s
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn session_of(s: &SessionStore) -> Option<Value> {
        s.lock().unwrap().get(ABSENSI).and_then(|v| v.get(CHAT).cloned())
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("hapusabsen").unwrap();
        assert_eq!(p.meta.name, "hapusabsen");
        assert_eq!(
            p.meta.alias,
            ["deleteabsen", "tutupabsen", "closeabsen", "resetabsen"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Hapus/tutup sesi absen (admin only)");
        assert_eq!(p.meta.usage, ".hapusabsen");
        assert_eq!(p.meta.example, ".hapusabsen");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["deleteabsen", "tutupabsen", "closeabsen", "resetabsen"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "hapusabsen");
        }
    }

    #[test]
    fn without_a_session_the_plugin_says_so() {
        let s = new_session_store();
        assert_eq!(text_of(&run(s.clone())), NO_SESSION);

        let s = new_session_store();
        s.lock().unwrap().insert(ABSENSI.to_string(), json!({}));
        let r = run(s.clone());
        assert_eq!(text_of(&r), NO_SESSION);
        assert_eq!(
            s.lock().unwrap().get(ABSENSI).unwrap().as_object().unwrap().len(),
            0,
            "peta harus tetap ada, cuma tanpa chat ini"
        );
    }

    #[test]
    fn a_closed_session_reports_keterangan_and_head_count_then_disappears() {
        let s = with_session(json!({
            "keterangan": "Rapat Mingguan",
            "peserta": ["628111@s.whatsapp.net", "628222@s.whatsapp.net"]
        }));
        let r = run(s.clone());
        assert_eq!(
            text_of(&r),
            "✅ *ABSEN DITUTUP!*\n\n\
             Penyebab?\n\
             📝 Rapat Mingguan\n\
             👥 Total hadir: 2\n\n\
             Sesi absen telah dihapus."
        );
        assert!(session_of(&s).is_none(), "sesi harus benar-benar dihapus");
    }

    #[test]
    fn an_empty_session_still_closes_with_zero_attendees() {
        let s = with_session(json!({"keterangan": DEFAULT_PESAN, "peserta": []}));
        let r = run(s.clone());
        assert!(text_of(&r).contains("👥 Total hadir: 0"), "{}", text_of(&r));
        assert!(session_of(&s).is_none());
    }

    #[test]
    fn only_this_chat_is_cleared() {
        let s = new_session_store();
        s.lock().unwrap().insert(
            ABSENSI.to_string(),
            json!({
                CHAT: {"keterangan": "a", "peserta": []},
                "120364@g.us": {"keterangan": "b", "peserta": []}
            }),
        );
        run(s.clone());
        assert!(session_of(&s).is_none());
        assert!(
            s.lock().unwrap().get(ABSENSI).unwrap().get("120364@g.us").is_some(),
            "sesi grup lain tidak boleh ikut terhapus"
        );
    }

    #[test]
    fn a_session_without_keterangan_prints_the_js_undefined() {
        let s = with_session(json!({"peserta": []}));
        let r = run(s);
        assert!(text_of(&r).contains("📝 undefined"), "{}", text_of(&r));
    }
}