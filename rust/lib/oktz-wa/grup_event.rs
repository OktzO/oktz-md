//! Event `group-participants.update` -> model murni + gerbang welcome/goodbye.
//!
//! Port dari JS produksi:
//! - `src/connection.js:1199-1328` handler `group-participants.update`
//!   (satu event = satu aksi + daftar peserta; `p.phoneNumber || p.id`
//!   di `connection.js:1218`).
//! - `src/handler.js:1981-2088` `groupHandler`: default grup
//!   (`handler.js:1997-2005`), pemilihan JID peserta (`handler.js:2048-2078`),
//!   pelaku dari `update.author` (`handler.js:2117`).
//! - `src/lib/lid.js:208-211` `extractNumber`, `lid.js:223-258`
//!   `resolveLidFromParticipants`, `lid.js:267-338` `resolveAnyLidToJid`.
//! - `src/lib/time.js:8,47-73` `formatPattern` (TZ Asia/Jakarta, locale id-ID).
//! - `plugins/group/welcome.js:127-128`, `plugins/group/goodbye.js:129-130`
//!   gerbang `welcome`/`goodbye` + `welcomeType`/`goodbyeType`.
//!
//! MURNI: tanpa I/O, tanpa async, tanpa cache global. Pengiriman pesan dan
//! pembacaan store dilakukan orkestrator (`pipeline.rs`).

use chrono::{Datelike, Local, Timelike, Weekday};
use serde_json::{Value, json};
use whatsapp_rust::types::events::Event;
use whatsapp_rust::wacore::stanza::groups::{GroupNotificationAction, GroupParticipantInfo};

/// Aksi dari `<notification type="w:gp2">`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AksiGrup {
    Add,
    Remove,
    Promote,
    Demote,
    Modify,
    Lain,
}

/// Satu peserta di dalam event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PesertaGrup {
    pub jid: String,
    /// `phone_number` dari wire (JID, bukan digit saja).
    pub nomor: Option<String>,
    /// Attr `lid` terpisah dari `jid`.
    pub lid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGrup {
    /// group_jid.
    pub id: String,
    pub aksi: AksiGrup,
    pub peserta: Vec<PesertaGrup>,
    /// `participant` (admin yang memicu).
    pub pelaku: Option<String>,
    pub mode_lid: bool,
}

impl UpdateGrup {
    /// `None` kalau event bukan `Event::GroupUpdate`.
    ///
    /// Satu `GroupUpdate` wacore = satu aksi, sama seperti event
    /// `group-participants.update` Baileys (`src/connection.js:1199`).
    /// Varian aksi di luar add/remove/promote/demote/modify jadi
    /// [`AksiGrup::Lain`] (JS hanya menangani lima itu, `handler.js:2082-2116`).
    pub fn dari_event(ev: &Event) -> Option<Self> {
        let Event::GroupUpdate(u) = ev else {
            return None;
        };
        let (aksi, peserta) = match &u.action {
            GroupNotificationAction::Add { participants, .. } => {
                (AksiGrup::Add, peserta_dari(participants))
            }
            GroupNotificationAction::Remove { participants, .. } => {
                (AksiGrup::Remove, peserta_dari(participants))
            }
            GroupNotificationAction::Promote { participants } => {
                (AksiGrup::Promote, peserta_dari(participants))
            }
            GroupNotificationAction::Demote { participants } => {
                (AksiGrup::Demote, peserta_dari(participants))
            }
            GroupNotificationAction::Modify { participants } => {
                (AksiGrup::Modify, peserta_dari(participants))
            }
            _ => (AksiGrup::Lain, Vec::new()),
        };
        Some(Self {
            id: u.group_jid.to_string(),
            aksi,
            peserta,
            pelaku: u.participant.as_ref().map(|j| j.to_string()),
            mode_lid: u.is_lid_addressing_mode,
        })
    }
}

fn peserta_dari(daftar: &[GroupParticipantInfo]) -> Vec<PesertaGrup> {
    daftar
        .iter()
        .map(|p| PesertaGrup {
            jid: p.jid.to_string(),
            nomor: p.phone_number.as_ref().map(|j| j.to_string()),
            lid: p.lid.as_ref().map(|j| j.to_string()),
        })
        .collect()
}

/// Nomor dari JID: buang `@...` dan `:device`.
///
/// `extractNumber` (`src/lib/lid.js:208-211`) membuang `@...`; bagian
/// `:device` dibuang seperti `connection.js:1221` (`split("@")[0].split(":")[0]`).
pub fn nomor_jid(jid: &str) -> String {
    let tanpa_server = jid.split('@').next().unwrap_or("");
    tanpa_server.split(':').next().unwrap_or("").to_string()
}

/// `resolveAnyLidToJid` (`src/lib/lid.js:267-338`) untuk satu peserta event.
///
/// `peserta_metadata` = array `groupMeta.participants` bentuk Baileys/JSON
/// (kunci `id`, `lid`, `jid`, `phoneNumber`), sumber tabel `lid -> pn`
/// (`lid.js:75-108`, `lid.js:223-258`).
///
/// Urutan: nomor telepon wire dulu (`connection.js:1218`), lalu JID non-LID
/// apa adanya (`handler.js:2057`), lalu terjemahkan LID lewat metadata,
/// terakhir input apa adanya.
///
/// ponytail: tanpa cache LID global (`lid.js:5,270-283`) dan tanpa
/// `isLidConverted` (`lid.js:150-168`, tabel kode negara) — modul murni tidak
/// punya state itu; tambahkan kalau JID `@s.whatsapp.net` berisi nomor LID
/// terbukti lolos ke sapaan.
pub fn jid_peserta(p: &PesertaGrup, peserta_metadata: &Value) -> String {
    if let Some(nomor) = p.nomor.as_deref()
        && !nomor.is_empty()
        && !nomor.ends_with("@lid")
    {
        return nomor.to_string();
    }
    if !p.jid.ends_with("@lid") {
        return p.jid.clone();
    }

    // Pencocokan persis `resolveLidFromParticipants` (`lid.js:227-255`):
    // `pLid === lidFormat || pLid === jid || pLidNumber === lidNumber`.
    let lid_num = nomor_jid(&p.jid);
    let lid_format = format!("{lid_num}@lid");
    if let Some(daftar) = peserta_metadata.as_array() {
        for m in daftar {
            let (m_lid, m_jid) = lid_pn_metadata(m);
            if (m_lid == lid_format || m_lid == p.jid || m_lid.replace("@lid", "") == lid_num)
                && !m_jid.is_empty()
                && !m_jid.ends_with("@lid")
            {
                return m_jid;
            }
        }
    }

    // JS jatuh ke `lidToJid(jid)` (`lid.js:257`); spec modul ini: input apa
    // adanya. Orkestrator yang memutuskan JID final untuk mention/kirim.
    p.jid.clone()
}

/// Pasangan `(lid, jid)` dari satu entri metadata, mengikuti cabang
/// `cacheParticipantLids`/`resolveLidFromParticipants` (`src/lib/lid.js:76-92`,
/// `lid.js:230-246`).
fn lid_pn_metadata(m: &Value) -> (String, String) {
    let teks = |kunci: &str| {
        m.get(kunci)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let id = teks("id");
    let jid = teks("jid");
    let lid = teks("lid");
    let pn = teks("phoneNumber");
    let id_atau_jid = if id.is_empty() { jid.clone() } else { id.clone() };

    if lid.ends_with("@lid") {
        (lid, id_atau_jid)
    } else if !pn.is_empty() {
        (id, pn)
    } else if id.ends_with("@lid") {
        (id, jid)
    } else {
        (lid, id_atau_jid)
    }
}

/// Default grup baru, persis `db.setGroup(groupJid, {...})`
/// (`src/handler.js:1999-2003`): `welcome`/`goodbye`/`leave` =
/// `config.welcome?.defaultEnabled ?? true` (goodbye juga untuk `leave`).
///
/// `config.js:186-187` memasang `defaultEnabled: false` untuk dua-duanya, jadi
/// nilai produksi bot JS yang ditiru adalah `false`. Modul ini murni (tanpa
/// config), jadi angkanya di-hardcode; kalau `config.json` Rust nanti punya
/// blok `welcome`/`goodbye`, ganti sumbernya ke situ.
pub fn default_grup() -> Value {
    json!({ "welcome": false, "goodbye": false, "leave": false })
}

/// `groupData?.welcome !== true` -> tidak dikirim
/// (`plugins/group/welcome.js:127`).
pub fn boleh_welcome(grup: &Value) -> bool {
    grup.get("welcome").and_then(Value::as_bool) == Some(true)
}

/// `groupData?.goodbye !== true && groupData?.leave !== true` -> tidak
/// dikirim (`plugins/group/goodbye.js:129`); jadi cukup salah satu `true`.
pub fn boleh_goodbye(grup: &Value) -> bool {
    grup.get("goodbye").and_then(Value::as_bool) == Some(true)
        || grup.get("leave").and_then(Value::as_bool) == Some(true)
}

/// `db.setting("welcomeType") || 1` (`plugins/group/welcome.js:128`).
/// Nilai falsy (0/null/tipe salah) jadi 1.
pub fn tipe_welcome(setting: &Value) -> i64 {
    tipe_setting(setting, "welcomeType")
}

/// `db.setting("goodbyeType") || 1` (`plugins/group/goodbye.js:130`).
pub fn tipe_goodbye(setting: &Value) -> i64 {
    tipe_setting(setting, "goodbyeType")
}

/// `nilai_setting` menerima peta settings utuh (`pipeline.rs`
/// `settings_entry()`, sumber `db.setting` di `src/lib/database.js:753`)
/// maupun nilai mentah satu kunci.
fn tipe_setting(setting: &Value, kunci: &str) -> i64 {
    setting
        .get(kunci)
        .unwrap_or(setting)
        .as_i64()
        .filter(|t| *t != 0)
        .unwrap_or(1)
}

/// Waktu lokal host.
///
/// JS memformat `new Date()` lewat `formatPattern` dengan TZ `Asia/Jakarta`
/// (`src/lib/time.js:8,69-73`); di Rust dipakai `Local::now()`, jadi TZ host
/// harus `Asia/Jakarta` supaya identik.
pub struct WaktuLokal {
    /// Pola `DD/MM/YYYY` (`time.js:58-60`).
    pub tanggal: String,
    /// Pola `HH:mm` dua digit (`time.js:53-54,62`).
    pub jam: String,
    /// Pola `dddd`, nama hari locale `id-ID` (`time.js:16-19,59`).
    pub hari: String,
    pub jam_angka: u32,
}

pub fn waktu_lokal() -> WaktuLokal {
    let kini = Local::now();
    WaktuLokal {
        tanggal: format!(
            "{:02}/{:02}/{:04}",
            kini.day(),
            kini.month(),
            kini.year()
        ),
        jam: format!("{:02}:{:02}", kini.hour(), kini.minute()),
        hari: nama_hari(kini.weekday()).to_string(),
        jam_angka: kini.hour(),
    }
}

/// Nama hari `Intl.DateTimeFormat('id-ID', { weekday: 'long' })`
/// (`src/lib/time.js:16-19`): Minggu..Sabtu.
fn nama_hari(hari: Weekday) -> &'static str {
    match hari.num_days_from_sunday() {
        0 => "Minggu",
        1 => "Senin",
        2 => "Selasa",
        3 => "Rabu",
        4 => "Kamis",
        5 => "Jumat",
        _ => "Sabtu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use whatsapp_rust::types::events::{Connected, GroupUpdate};

    fn peserta(jid: &str, nomor: Option<&str>, lid: Option<&str>) -> PesertaGrup {
        PesertaGrup {
            jid: jid.to_string(),
            nomor: nomor.map(str::to_string),
            lid: lid.map(str::to_string),
        }
    }

    fn info(jid: &str, nomor: Option<&str>, lid: Option<&str>) -> GroupParticipantInfo {
        GroupParticipantInfo {
            jid: jid.parse().unwrap(),
            phone_number: nomor.map(|n| n.parse().unwrap()),
            display_name: None,
            r#type: None,
            lid: lid.map(|l| l.parse().unwrap()),
            username: None,
            join_time: None,
            group_history_sent_state: None,
        }
    }

    #[test]
    fn nomor_jid_buang_server_dan_device() {
        assert_eq!(nomor_jid("6281234567890@s.whatsapp.net"), "6281234567890");
        assert_eq!(nomor_jid("6281234567890:5@s.whatsapp.net"), "6281234567890");
        assert_eq!(nomor_jid("123456789@lid"), "123456789");
        assert_eq!(nomor_jid("120363000000000001@g.us"), "120363000000000001");
        assert_eq!(nomor_jid(""), "");
    }

    #[test]
    fn jid_peserta_pakai_nomor_wire_apa_adanya() {
        let p = peserta("111@lid", Some("628111@s.whatsapp.net"), None);
        assert_eq!(jid_peserta(&p, &json!([])), "628111@s.whatsapp.net");
        // JID non-LID lewat apa adanya, metadata tidak dibaca.
        let p = peserta("628222@s.whatsapp.net", None, None);
        assert_eq!(jid_peserta(&p, &Value::Null), "628222@s.whatsapp.net");
    }

    #[test]
    fn jid_peserta_terjemahkan_lid_lewat_metadata() {
        // Bentuk groupMetadata.participants: { id: PN, lid: LID }.
        let meta = json!([
            { "id": "628333@s.whatsapp.net", "lid": "333@lid", "admin": "admin" },
            { "id": "628444@s.whatsapp.net", "lid": "444@lid", "admin": null }
        ]);
        let p = peserta("444@lid", None, None);
        assert_eq!(jid_peserta(&p, &meta), "628444@s.whatsapp.net");

        // Bentuk event GroupHandler: { id: LID, phoneNumber: PN }.
        let meta = json!([{ "id": "555@lid", "phoneNumber": "628555@s.whatsapp.net" }]);
        let p = peserta("555@lid", None, None);
        assert_eq!(jid_peserta(&p, &meta), "628555@s.whatsapp.net");
    }

    #[test]
    fn jid_peserta_tidak_ketemu_kembalikan_input() {
        let meta = json!([{ "id": "628666@s.whatsapp.net", "lid": "666@lid" }]);
        let p = peserta("999@lid", None, None);
        assert_eq!(jid_peserta(&p, &meta), "999@lid");
        assert_eq!(jid_peserta(&p, &Value::Null), "999@lid");
        // Entri cocok tapi pJid-nya LID -> tetap input asal.
        let meta = json!([{ "id": "777@lid", "lid": "999@lid" }]);
        assert_eq!(jid_peserta(&p, &meta), "999@lid");
    }

    #[test]
    fn tipe_welcome_falsy_dan_tipe_salah_jadi_satu() {
        assert_eq!(tipe_welcome(&json!(null)), 1);
        assert_eq!(tipe_welcome(&json!(0)), 1);
        assert_eq!(tipe_welcome(&json!(2)), 2);
        assert_eq!(tipe_welcome(&json!({"welcomeType": null})), 1);
        assert_eq!(tipe_welcome(&json!({"welcomeType": 0})), 1);
        assert_eq!(tipe_welcome(&json!({"welcomeType": 3})), 3);
        assert_eq!(tipe_welcome(&json!("2")), 1);
        assert_eq!(tipe_goodbye(&json!({"goodbyeType": 4})), 4);
        assert_eq!(tipe_goodbye(&json!({})), 1);
    }

    #[test]
    fn default_grup_tiga_kunci_false_mengikuti_config_js() {
        let g = default_grup();
        assert_eq!(g.get("welcome"), Some(&json!(false)));
        assert_eq!(g.get("goodbye"), Some(&json!(false)));
        assert_eq!(g.get("leave"), Some(&json!(false)));
    }

    #[test]
    fn gerbang_welcome_hanya_true_ketat() {
        assert!(boleh_welcome(&json!({"welcome": true})));
        assert!(!boleh_welcome(&json!({"welcome": "on"})));
        assert!(!boleh_welcome(&json!({"welcome": false})));
        assert!(!boleh_welcome(&json!(null)));
        assert!(!boleh_welcome(&json!({})));
    }

    #[test]
    fn gerbang_goodbye_benar_kalau_goodbye_atau_leave_true() {
        assert!(boleh_goodbye(&json!({"goodbye": true})));
        assert!(boleh_goodbye(&json!({"goodbye": true, "leave": false})));
        assert!(boleh_goodbye(&json!({"goodbye": false, "leave": true})));
        assert!(!boleh_goodbye(&json!({"goodbye": "on"})));
        assert!(!boleh_goodbye(&json!({"goodbye": false, "leave": false})));
        assert!(!boleh_goodbye(&json!(null)));
    }

    #[test]
    fn waktu_lokal_pola_dd_mm_yyyy_dan_hh_mm() {
        let w = waktu_lokal();
        assert_eq!(w.tanggal.len(), 10, "tanggal: {}", w.tanggal);
        let t = w.tanggal.as_bytes();
        assert_eq!(t[2], b'/');
        assert_eq!(t[5], b'/');
        for i in [0, 1, 3, 4, 6, 7, 8, 9] {
            assert!(t[i].is_ascii_digit(), "tanggal: {}", w.tanggal);
        }
        assert_eq!(w.jam.len(), 5, "jam: {}", w.jam);
        let j = w.jam.as_bytes();
        assert_eq!(j[2], b':');
        assert!(j[0].is_ascii_digit() && j[1].is_ascii_digit());
        assert!(j[3].is_ascii_digit() && j[4].is_ascii_digit());
        assert!(w.jam_angka <= 23, "jam_angka: {}", w.jam_angka);
        assert!(
            ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"]
                .contains(&w.hari.as_str()),
            "hari: {}",
            w.hari
        );
    }

    #[test]
    fn dari_event_add_membawa_peserta_dan_pelaku() {
        let ev = Event::GroupUpdate(
            GroupUpdate::builder()
                .group_jid("120363000000000001@g.us".parse().unwrap())
                .timestamp(DateTime::<Utc>::UNIX_EPOCH)
                .is_lid_addressing_mode(true)
                .maybe_participant(Some("6289999999999@s.whatsapp.net".parse().unwrap()))
                .action(GroupNotificationAction::Add {
                    participants: vec![info(
                        "111@lid",
                        Some("628111@s.whatsapp.net"),
                        Some("111@lid"),
                    )],
                    reason: None,
                })
                .build(),
        );
        let u = UpdateGrup::dari_event(&ev).expect("event grup");
        assert_eq!(u.id, "120363000000000001@g.us");
        assert_eq!(u.aksi, AksiGrup::Add);
        assert!(u.mode_lid);
        assert_eq!(u.pelaku.as_deref(), Some("6289999999999@s.whatsapp.net"));
        assert_eq!(
            u.peserta,
            vec![peserta(
                "111@lid",
                Some("628111@s.whatsapp.net"),
                Some("111@lid")
            )]
        );
    }

    #[test]
    fn dari_event_aksi_lain_tanpa_peserta() {
        let ev = Event::GroupUpdate(
            GroupUpdate::builder()
                .group_jid("120363000000000001@g.us".parse().unwrap())
                .timestamp(DateTime::<Utc>::UNIX_EPOCH)
                .is_lid_addressing_mode(false)
                .action(GroupNotificationAction::Unlocked)
                .build(),
        );
        let u = UpdateGrup::dari_event(&ev).expect("event grup");
        assert_eq!(u.aksi, AksiGrup::Lain);
        assert!(u.peserta.is_empty());
        assert_eq!(u.pelaku, None);
    }

    #[test]
    fn dari_event_bukan_grup_none() {
        let ev = Event::Connected(Connected::builder().build());
        assert!(UpdateGrup::dari_event(&ev).is_none());
    }
}
