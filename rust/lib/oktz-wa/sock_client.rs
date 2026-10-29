//! Implementasi nyata [`Sock`] di atas klien WhatsApp.
//!
//! Trait [`Sock`] sinkron, klien asinkron; jembatannya `block_in_place` +
//! `Handle::block_on` karena itu hanya sah di runtime multi-thread, runtime
//! current-thread dikembalikan sebagai `SockError`, bukan panic.
//!
//! Bentuk hasil mengikuti Baileys karena plugin JS membaca kunci itu apa adanya.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

use oktz_plugin_api::{AdaWhatsapp, AksiPeserta, Sock, SockError};
use serde_json::{Value, json};
use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore_binary::JidExt;
use whatsapp_rust::{
    ChatStateType, GroupCreateOptions, GroupMetadata, GroupParticipant, GroupParticipantOptions,
    ParticipantType,
};

/// `Sock` produksi: meneruskan panggilan sinkron plugin ke klien WhatsApp.
pub struct SockKlien {
    client: Arc<Client>,
}

impl std::fmt::Debug for SockKlien {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SockKlien")
    }
}

impl SockKlien {
    pub fn baru(client: Arc<Client>) -> Arc<dyn Sock> {
        Arc::new(Self { client })
    }

    /// Tunggu future dari konteks sinkron.
    fn tunggu<F: Future>(&self, fut: F) -> Result<F::Output, SockError> {
        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            SockError::baru("sock tidak tersedia: dipanggil tanpa runtime tokio")
        })?;
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            return Err(SockError::baru(
                "sock tidak tersedia: runtime tokio current-thread tidak mendukung block_in_place",
            ));
        }
        Ok(tokio::task::block_in_place(|| handle.block_on(fut)))
    }

    /// Jalankan future `Result<T, E>`; error WhatsApp jadi pesan `SockError`.
    fn jalan<F, T, E>(&self, fut: F, konteks: &str) -> Result<T, SockError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        match self.tunggu(fut)? {
            Ok(v) => Ok(v),
            Err(e) => Err(SockError::baru(format!("{konteks}: {e}"))),
        }
    }
}

/// JID sudah berbentuk teks; biarkan parser yang memutuskan valid atau tidak.
fn ke_jid(teks: &str, konteks: &str) -> Result<Jid, SockError> {
    teks.trim()
        .parse::<Jid>()
        .map_err(|e| SockError::baru(format!("{konteks}: jid tidak valid {teks:?}: {e}")))
}

/// Nomor polos (`628xx`, `+62 8xx`) jadi JID PN; teks ber-JID dibiarkan.
fn jid_nomor(nomor: &str, konteks: &str) -> Result<Jid, SockError> {
    if nomor.contains('@') {
        return ke_jid(nomor, konteks);
    }
    let digit: String = nomor.chars().filter(char::is_ascii_digit).collect();
    if digit.is_empty() {
        return Err(SockError::baru(format!(
            "{konteks}: nomor tidak punya digit: {nomor:?}"
        )));
    }
    format!("{digit}@s.whatsapp.net")
        .parse::<Jid>()
        .map_err(|e| SockError::baru(format!("{konteks}: jid tidak valid {nomor:?}: {e}")))
}

/// `admin` Baileys: `"superadmin"`, `"admin"`, atau `null`.
fn admin_kata(tipe: ParticipantType) -> Value {
    match tipe {
        ParticipantType::SuperAdmin => json!("superadmin"),
        ParticipantType::Admin => json!("admin"),
        ParticipantType::Member => Value::Null,
    }
}

fn jid_teks(j: &Option<Jid>) -> Value {
    match j {
        Some(j) => json!(j.to_string()),
        None => Value::Null,
    }
}

fn peserta_json(p: &GroupParticipant) -> Value {
    json!({
        "id": p.jid.to_string(),
        "admin": admin_kata(p.participant_type),
        "lid": jid_teks(&p.lid),
        "phoneNumber": jid_teks(&p.phone_number),
    })
}

/// Metadata grup dalam bentuk Baileys. Waktu dikali 1000 karena Baileys
/// memakai milidetik, klien mengembalikan detik.
pub fn metadata_json(m: &GroupMetadata) -> Value {
    let peserta: Vec<Value> = m.participants.iter().map(peserta_json).collect();
    json!({
        "id": m.id.to_string(),
        "subject": m.subject,
        "subjectOwner": jid_teks(&m.subject_owner),
        "subjectTime": m.subject_time.map(|s| s.saturating_mul(1000)),
        "creation": m.creation_time.map(|s| s.saturating_mul(1000)),
        "owner": jid_teks(&m.creator),
        "desc": m.description,
        "descId": m.description_id,
        "descOwner": jid_teks(&m.description_owner),
        "descTime": m.description_time.map(|s| s.saturating_mul(1000)),
        "restrict": m.is_locked,
        "announce": m.is_announcement,
        "size": m.size.unwrap_or(peserta.len() as u32),
        "isCommunity": m.is_parent_group,
        "isCommunityAnnounce": m.is_default_sub_group,
        "linkedParent": jid_teks(&m.parent_group_jid),
        "participants": peserta,
    })
}

/// Kelompokkan entri `readMessages` per (chat, pengirim): satu balasan hanya
/// bisa menyebut satu chat + satu pengirim.
pub fn kelompokkan_dibaca(pesan: &[Value]) -> Vec<(String, Option<String>, Vec<String>)> {
    let mut peta: BTreeMap<(String, Option<String>), Vec<String>> = BTreeMap::new();
    for entri in pesan {
        let Some(chat) = entri.get("remoteJid").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = entri.get("id").and_then(Value::as_str) else {
            continue;
        };
        let pengirim = entri
            .get("participant")
            .and_then(Value::as_str)
            .map(str::to_string);
        peta.entry((chat.to_string(), pengirim))
            .or_default()
            .push(id.to_string());
    }
    peta.into_iter()
        .map(|((chat, pengirim), id)| (chat, pengirim, id))
        .collect()
}

impl Sock for SockKlien {
    fn ada_whatsapp(&self, nomor: &[String]) -> Result<Vec<AdaWhatsapp>, SockError> {
        let jids: Vec<Jid> = nomor
            .iter()
            .map(|n| jid_nomor(n, "onWhatsApp"))
            .collect::<Result<_, _>>()?;
        let hasil = self.jalan(self.client.contacts().is_on_whatsapp(&jids), "onWhatsApp")?;

        // Balasan server tidak dijamin urut seperti permintaan.
        let mut peta: BTreeMap<String, bool> = BTreeMap::new();
        for r in &hasil {
            peta.insert(r.jid.user.as_str().to_string(), r.is_registered);
            if let Some(pn) = &r.pn_jid {
                peta.insert(pn.user.as_str().to_string(), r.is_registered);
            }
            if let Some(lid) = &r.lid {
                peta.insert(lid.user.as_str().to_string(), r.is_registered);
            }
        }
        Ok(jids
            .iter()
            .map(|j| AdaWhatsapp {
                jid: j.to_string(),
                ada: peta.get(j.user.as_str()).copied().unwrap_or(false),
            })
            .collect())
    }

    fn foto_profil(&self, jid: &str, tipe: &str) -> Result<Option<String>, SockError> {
        let j = ke_jid(jid, "profilePictureUrl")?;
        // Baileys: "image" (penuh) atau "preview"; selain itu dianggap preview.
        let preview = tipe != "image";
        let pic = self.jalan(self.client.contacts().get_profile_picture(&j, preview), "profilePictureUrl")?;
        Ok(pic.map(|p| p.url))
    }

    fn metadata_grup(&self, jid: &str) -> Result<Value, SockError> {
        let j = ke_jid(jid, "groupMetadata")?;
        let meta = self.jalan(self.client.groups().get_metadata(&j), "groupMetadata")?;
        Ok(metadata_json(&meta))
    }

    fn semua_grup(&self) -> Result<Value, SockError> {
        let peta = self.jalan(self.client.groups().get_participating(), "groupFetchAllParticipating")?;
        let mut out = serde_json::Map::new();
        for (j, meta) in peta {
            out.insert(j.to_string(), metadata_json(&meta));
        }
        Ok(Value::Object(out))
    }

    fn ubah_peserta(
        &self,
        jid: &str,
        peserta: &[String],
        aksi: AksiPeserta,
    ) -> Result<(), SockError> {
        let j = ke_jid(jid, "groupParticipantsUpdate")?;
        let anggota: Vec<Jid> = peserta
            .iter()
            .map(|p| jid_nomor(p, "groupParticipantsUpdate"))
            .collect::<Result<_, _>>()?;
        let groups = self.client.groups();
        match aksi {
            AksiPeserta::Tambah => self.jalan(groups.add_participants(&j, &anggota), "groupParticipantsUpdate")?,
            AksiPeserta::Keluar => {
                self.jalan(groups.remove_participants(&j, &anggota), "groupParticipantsUpdate")?
            }
            AksiPeserta::Naikkan => {
                self.jalan(groups.promote_participants(&j, &anggota), "groupParticipantsUpdate")?
            }
            AksiPeserta::Turunkan => {
                self.jalan(groups.demote_participants(&j, &anggota), "groupParticipantsUpdate")?
            }
        };
        Ok(())
    }

    fn set_foto_profil(&self, jid: &str, isi: &[u8]) -> Result<(), SockError> {
        let j = ke_jid(jid, "updateProfilePicture")?;
        if j.is_group() {
            self.jalan(
                self.client.groups().set_profile_picture(&j, isi.to_vec()),
                "updateProfilePicture",
            )?;
        } else {
            // Klien hanya bisa mengubah foto profil sendiri.
            self.jalan(self.client.profile().set_profile_picture(isi.to_vec()), "updateProfilePicture")?;
        }
        Ok(())
    }

    fn keluar_grup(&self, jid: &str) -> Result<(), SockError> {
        let j = ke_jid(jid, "groupLeave")?;
        self.jalan(self.client.groups().leave(j), "groupLeave")
    }

    fn kode_undangan(&self, jid: &str) -> Result<String, SockError> {
        let j = ke_jid(jid, "groupInviteCode")?;
        self.jalan(self.client.groups().get_invite_link(j, false), "groupInviteCode")
    }

    fn info_undangan(&self, kode: &str) -> Result<Value, SockError> {
        let meta = self.jalan(self.client.groups().get_invite_info(kode), "groupGetInviteInfo")?;
        Ok(metadata_json(&meta))
    }

    fn buat_grup(&self, subjek: &str, peserta: &[String]) -> Result<Value, SockError> {
        let mut opsi = GroupCreateOptions::new(subjek);
        for p in peserta {
            opsi = opsi.with_participant(GroupParticipantOptions::new(jid_nomor(p, "groupCreate")?));
        }
        let hasil = self.jalan(self.client.groups().create_group(opsi), "groupCreate")?;
        Ok(metadata_json(&hasil.metadata))
    }

    fn terima_undangan(&self, kode: &str) -> Result<String, SockError> {
        let hasil = self.jalan(self.client.groups().join_with_invite_code(kode), "groupAcceptInvite")?;
        Ok(hasil.group_jid().to_string())
    }

    fn tandai_dibaca(&self, pesan: &[Value]) -> Result<(), SockError> {
        for (chat, pengirim, ids) in kelompokkan_dibaca(pesan) {
            let c = ke_jid(&chat, "readMessages")?;
            let p = pengirim
                .as_deref()
                .map(|s| ke_jid(s, "readMessages"))
                .transpose()?;
            let teks: Vec<&str> = ids.iter().map(String::as_str).collect();
            self.jalan(
                self.client.mark_as_read(&c, p.as_ref(), &teks),
                "readMessages",
            )?;
        }
        Ok(())
    }

    fn presence(&self, jenis: &str, jid: &str) -> Result<(), SockError> {
        let j = ke_jid(jid, "sendPresenceUpdate")?;
        match jenis {
            "available" => self.jalan(self.client.presence().set_available(), "sendPresenceUpdate"),
            "unavailable" => {
                self.jalan(self.client.presence().set_unavailable(), "sendPresenceUpdate")
            }
            "composing" => {
                self.jalan(self.client.chatstate().send_composing(&j), "sendPresenceUpdate")
            }
            "recording" => {
                self.jalan(self.client.chatstate().send_recording(&j), "sendPresenceUpdate")
            }
            "paused" => self.jalan(
                self.client.chatstate().send(&j, ChatStateType::Paused),
                "sendPresenceUpdate",
            ),
            lain => Err(SockError::baru(format!(
                "sendPresenceUpdate: jenis tidak dikenal: {lain:?}"
            ))),
        }
    }

    fn status_teks(&self, jid: &str) -> Result<Value, SockError> {
        let j = ke_jid(jid, "fetchStatus")?;
        let peta = self.jalan(
            self.client.contacts().get_user_info(std::slice::from_ref(&j)),
            "fetchStatus",
        )?;
        // `setAt` tidak dikirim server lewat usync; tetap ada sebagai null.
        let status = peta
            .values()
            .next()
            .and_then(|u| u.status.clone())
            .map(Value::String)
            .unwrap_or(Value::Null);
        Ok(json!({"status": status, "setAt": Value::Null}))
    }

    fn nama_kontak(&self, _jid: &str) -> Result<String, SockError> {
        // `sock.getName` di Baileys membaca toko kontak lokal. Klien Rust tidak
        // menyimpannya, jadi lebih jujur error daripada mengarang nama.
        Err(SockError::baru(
            "getName: klien Rust tidak menyimpan toko kontak (lihat BLOCKED.md)",
        ))
    }

    fn profil_bisnis(&self, jid: &str) -> Result<Option<Value>, SockError> {
        let j = ke_jid(jid, "getBusinessProfile")?;
        let profil = self.jalan(self.client.get_business_profile(&j), "getBusinessProfile")?;
        match profil {
            Some(p) => serde_json::to_value(&p)
                .map(Some)
                .map_err(|e| SockError::baru(format!("getBusinessProfile: serialize: {e}"))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_kata_mengikuti_baileys() {
        assert_eq!(admin_kata(ParticipantType::SuperAdmin), json!("superadmin"));
        assert_eq!(admin_kata(ParticipantType::Admin), json!("admin"));
        assert_eq!(admin_kata(ParticipantType::Member), Value::Null);
    }

    #[test]
    fn jid_nomor_menerima_format_bebas_dan_jid() {
        assert_eq!(
            jid_nomor("+62 812-345", "t").unwrap().to_string(),
            "62812345@s.whatsapp.net"
        );
        assert_eq!(
            jid_nomor("62812345@s.whatsapp.net", "t").unwrap().to_string(),
            "62812345@s.whatsapp.net"
        );
        assert!(jid_nomor("bukan nomor", "t").is_err());
    }

    #[test]
    fn kelompokkan_dibaca_menyatukan_id_per_chat_dan_pengirim() {
        let pesan = vec![
            json!({"remoteJid": "6281@g.us", "id": "A", "participant": "6282@s.whatsapp.net"}),
            json!({"remoteJid": "6281@g.us", "id": "B", "participant": "6282@s.whatsapp.net"}),
            json!({"remoteJid": "6281@g.us", "id": "C", "participant": "6283@s.whatsapp.net"}),
            json!({"remoteJid": "6282@s.whatsapp.net", "id": "D"}),
            json!({"remoteJid": "6282@s.whatsapp.net"}),
        ];
        let hasil = kelompokkan_dibaca(&pesan);
        assert_eq!(hasil.len(), 3);
        assert!(hasil.contains(&(
            "6281@g.us".to_string(),
            Some("6282@s.whatsapp.net".to_string()),
            vec!["A".to_string(), "B".to_string()]
        )));
        assert!(hasil.contains(&(
            "6281@g.us".to_string(),
            Some("6283@s.whatsapp.net".to_string()),
            vec!["C".to_string()]
        )));
        assert!(hasil.contains(&(
            "6282@s.whatsapp.net".to_string(),
            None,
            vec!["D".to_string()]
        )));
    }
}
