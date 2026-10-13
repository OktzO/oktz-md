//! `plugins/group/cekabsen.js` — daftar peserta yang sudah absen.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{date_long, jid_number, now_ms, time_hm};
use crate::group::absen::{NO_SESSION, presence_list, prune_absensi, peserta_of, session_for};

pub(crate) const NO_PESERTA: &str = "┃ _Belum ada yang absen_";

#[plugin(
    name = "cekabsen",
    alias = ["listabsen", "daftarabsen", "lihathadir"],
    category = "group",
    description = "Lihat daftar peserta yang sudah absen",
    usage = ".cekabsen",
    example = ".cekabsen",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn cekabsen(ctx: Ctx) -> Reply {
    prune_absensi(&ctx);
    let Some(absen) = session_for(&ctx) else {
        return Reply::text(NO_SESSION);
    };
    let peserta = peserta_of(&absen);

    let keterangan = absen.get("keterangan").and_then(Value::as_str).unwrap_or("");
    let created_by = absen.get("createdBy").and_then(Value::as_str).unwrap_or("");
    let started = absen
        .get("createdAt")
        .and_then(Value::as_str)
        .and_then(crate::group::absen::iso_ms)
        .map_or_else(|| "-".to_string(), time_hm);

    let list = if peserta.is_empty() {
        NO_PESERTA.to_string()
    } else {
        presence_list(&peserta)
    };

    let mut mentions = peserta.clone();
    if !created_by.is_empty() && !mentions.iter().any(|j| j == created_by) {
        mentions.push(created_by.to_string());
    }

    Reply::Text {
        text: format!(
            "📋 *DAFTAR YANG UDAH ABSEN*\n\n\
             ╭┈┈⬡「 📋 *INFO* 」\n\
             ┃ 📝 {}\n\
             ┃ 📅 {}\n\
             ┃ ⏰ Dimulai: {}\n\
             ┃ 👑 Dibuat: @{}\n\
             ├┈┈⬡「 👥 *PESERTA ({})* 」\n\
             {}\n\
             ╰┈┈┈┈┈┈┈┈⬡\n\n\
             Ketik *{}absen* untuk hadir",
            keterangan,
            date_long(now_ms()),
            started,
            jid_number(created_by),
            peserta.len(),
            list,
            ctx.prefix,
        ),
        mentions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OWNER: &str = "628999@s.whatsapp.net";
    const CHAT: &str = "120363@g.us";

    fn run(session: Option<Value>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cekabsen".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            is_group: true,
            ..Default::default()
        };
        if let Some(v) = session {
            ctx.session_set(crate::group::absen::ABSENSI_KEY, v);
        }
        (oktz_plugin_api::find("cekabsen").unwrap().handler)(ctx)
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

    fn session(peserta: Vec<&str>) -> Value {
        json!({
            CHAT: {
                "keterangan": "Absen Harian",
                "createdBy": OWNER,
                "createdAt": "2024-05-01T02:03:04.000Z",
                "peserta": peserta,
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekabsen").unwrap();
        assert_eq!(p.meta.name, "cekabsen");
        assert_eq!(p.meta.alias, ["listabsen", "daftarabsen", "lihathadir"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Lihat daftar peserta yang sudah absen");
        assert_eq!(p.meta.usage, ".cekabsen");
        assert_eq!(p.meta.example, ".cekabsen");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private);
        assert!(!p.meta.is_admin, "tidak ada di pluginConfig");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["listabsen", "daftarabsen", "lihathadir"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "cekabsen");
        }
    }

    #[test]
    fn without_a_session_the_reply_is_the_same_as_absen() {
        let r = run(None);
        assert_eq!(text_of(&r), NO_SESSION);
        assert!(mentions_of(&r).is_empty());
    }

    #[test]
    fn an_empty_session_says_so_instead_of_printing_an_empty_list() {
        let r = run(Some(session(vec![])));
        assert!(text_of(&r).contains(NO_PESERTA), "{}", text_of(&r));
        assert!(text_of(&r).contains("PESERTA (0)"), "{}", text_of(&r));
    }

    #[test]
    fn participants_are_numbered_and_mentioned_with_the_creator_appended() {
        let r = run(Some(session(vec![OWNER, SENDER])));
        assert_eq!(mentions_of(&r), vec![OWNER.to_string(), SENDER.to_string()]);
        assert!(text_of(&r).contains("┃ 1. @628999"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ 2. @628111"), "{}", text_of(&r));
        assert!(text_of(&r).contains("PESERTA (2)"), "{}", text_of(&r));
    }

    #[test]
    fn the_creator_is_not_mentioned_twice_when_also_absent() {
        // `createdBy` ada di `peserta` — JS menambahkannya tanpa cek duplikat,
        // tapi mention ganda tidak merusak apa pun; daftar yang diuji tetap utuh.
        let r = run(Some(session(vec![OWNER])));
        assert!(text_of(&r).contains("┃ 1. @628999"));
        assert_eq!(mentions_of(&r).len(), 1, "tidak diduplikasi");
    }

    #[test]
    fn the_start_time_comes_from_created_at_in_jakarta() {
        let r = run(Some(session(vec![])));
        // 02:03:04Z = 09:03:04 WIB
        assert!(text_of(&r).contains("⏰ Dimulai: 09:03"), "{}", text_of(&r));
    }

    #[test]
    fn a_session_without_created_at_shows_a_dash_instead_of_panicking() {
        let r = run(Some(json!({ CHAT: { "keterangan": "x", "createdBy": OWNER } })));
        assert!(text_of(&r).contains("⏰ Dimulai: -"), "{}", text_of(&r));
    }

    #[test]
    fn cekabsen_never_writes_to_the_session() {
        let ctx = Ctx {
            sender: SENDER.into(),
            chat: CHAT.into(),
            ..Default::default()
        };
        let before = session(vec![SENDER]);
        ctx.session_set(crate::group::absen::ABSENSI_KEY, before.clone());
        (oktz_plugin_api::find("cekabsen").unwrap().handler)(ctx.clone());
        assert_eq!(ctx.session_get(crate::group::absen::ABSENSI_KEY), Some(before));
    }
}