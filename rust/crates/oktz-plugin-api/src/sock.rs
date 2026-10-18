//! Padanan `sock` Baileys yang dipakai plugin JS.
//!
//! Plugin JS memanggil `sock.*` untuk hal-hal yang bukan sekadar mengirim
//! balasan: cek nomor punya akun WhatsApp, ambil foto profil, baca metadata
//! grup, naik/turunkan admin, keluar dari grup, dan set foto profil bot.
//! Di Rust, `Reply` sudah menutup jalur kirim pesan, tapi sisanya belum ada —
//! dan itu yang memblokir puluhan plugin.
//!
//! Kontrak di sini SENGAJA memakai `serde_json::Value` untuk bentuk yang
//! kompleks (metadata grup), supaya port bisa mengikuti bentuk JS apa adanya
//! tanpa menerjemahkan dulu. Yang mengembalikan `Value` harus mempertahankan
//! nama kunci yang sama dengan Baileys; plugin yang membaca
//! `metadata.participants[i].admin` tidak perlu diubah.
//!
//! Implementasi nyatanya ada di crate `oktz-wa`, memakai klien WhatsApp;
//! di sini hanya kontrak supaya plugin bisa diuji dengan stub, sama seperti
//! [`HttpGet`](crate::HttpGet).

use std::fmt;

use serde_json::Value;

/// Kegagalan pemanggilan `sock.*`.
///
/// JS membedakan "request gagal" (lempar) dari "hasil kosong" (`null`).
/// Yang kedua bukan error: `profilePictureUrl` mengembalikan `Err` hanya
/// kalau WhatsApp menolak, sedangkan foto profil tidak ada dikembalikan
/// sebagai `Ok(None)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SockError {
    /// Pesan yang bisa ditampilkan; sudah meniru `err.message` JS.
    pub pesan: String,
}

impl SockError {
    pub fn baru(pesan: impl Into<String>) -> Self {
        Self {
            pesan: pesan.into(),
        }
    }
}

impl fmt::Display for SockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pesan)
    }
}

impl std::error::Error for SockError {}

/// Satu entri hasil `sock.onWhatsApp` (Baileys: `{ jid, exists }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaWhatsapp {
    pub jid: String,
    pub ada: bool,
}

/// Perubahan keanggotaan grup, memetakan `sock.groupParticipantsUpdate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AksiPeserta {
    Tambah,
    Keluar,
    Naikkan, // promote
    Turunkan, // demote
}

impl AksiPeserta {
    /// Kata yang dipakai Baileys (`"add"`, `"remove"`, `"promote"`, `"demote"`).
    pub fn kata(&self) -> &'static str {
        match self {
            Self::Tambah => "add",
            Self::Keluar => "remove",
            Self::Naikkan => "promote",
            Self::Turunkan => "demote",
        }
    }
}

/// Padanan objek `sock`. Sinkron, sama seperti [`HttpGet`](crate::HttpGet):
/// implementasi yang memakai klien asinkron bertanggung jawab menunggu di
/// dalam (`tokio::task::block_in_place` + `Handle::block_on`).
pub trait Sock: Send + Sync + std::fmt::Debug {
    /// `sock.onWhatsApp(...nomor)` — bentuk `[(nomor, ada)]` mengikuti
    /// Baileys, termasuk entri `jid` yang sudah dinormalkan.
    fn ada_whatsapp(&self, nomor: &[String]) -> Result<Vec<AdaWhatsapp>, SockError>;

    /// `sock.profilePictureUrl(jid, "image" | "preview")`.
    /// `Ok(None)` = nomor/grup tidak punya foto (JS: `undefined`).
    fn foto_profil(&self, jid: &str, tipe: &str) -> Result<Option<String>, SockError>;

    /// `sock.groupMetadata(jid)` — `Value` dengan kunci Baileys
    /// (`subject`, `participants`, `id`, ...).
    fn metadata_grup(&self, jid: &str) -> Result<Value, SockError>;

    /// `sock.groupFetchAllParticipating()` — peta `jid -> metadata`.
    fn semua_grup(&self) -> Result<Value, SockError>;

    /// `sock.groupParticipantsUpdate(jid, [...], aksi)`.
    fn ubah_peserta(
        &self,
        jid: &str,
        peserta: &[String],
        aksi: AksiPeserta,
    ) -> Result<(), SockError>;

    /// `sock.updateProfilePicture(jid, isi)`.
    fn set_foto_profil(&self, jid: &str, isi: &[u8]) -> Result<(), SockError>;

    /// `sock.groupLeave(jid)`.
    fn keluar_grup(&self, jid: &str) -> Result<(), SockError>;

    /// `sock.groupInviteCode(jid)`.
    fn kode_undangan(&self, jid: &str) -> Result<String, SockError>;

    /// `sock.groupGetInviteInfo(kode)`.
    fn info_undangan(&self, kode: &str) -> Result<Value, SockError>;

    /// `sock.groupCreate(subject, peserta)` — hasilnya metadata grup baru.
    fn buat_grup(&self, subjek: &str, peserta: &[String]) -> Result<Value, SockError>;

    /// `sock.groupAcceptInvite(kode)`.
    fn terima_undangan(&self, kode: &str) -> Result<String, SockError>;

    /// `sock.readMessages([...])` — array `{ remoteJid, id, participant }`.
    fn tandai_dibaca(&self, pesan: &[Value]) -> Result<(), SockError>;

    /// `sock.sendPresenceUpdate(jenis, jid)`.
    fn presence(&self, jenis: &str, jid: &str) -> Result<(), SockError>;

    /// `sock.fetchStatus(jid)` — `{ status, setAt }`.
    fn status_teks(&self, jid: &str) -> Result<Value, SockError>;

    /// `sock.getName(jid)` — nama kontak dari kontak tersimpan.
    fn nama_kontak(&self, jid: &str) -> Result<String, SockError>;

    /// `sock.getBusinessProfile(jid)` — `Value` atau `None` kalau bukan
    /// akun bisnis.
    fn profil_bisnis(&self, jid: &str) -> Result<Option<Value>, SockError>;
}

/// `Sock` yang selalu gagal — dipakai kalau bot berjalan tanpa klien
/// WhatsApp (test, atau jalur `--offline`).
#[derive(Debug)]
pub struct SockMati;

impl Sock for SockMati {
    fn ada_whatsapp(&self, _nomor: &[String]) -> Result<Vec<AdaWhatsapp>, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn foto_profil(&self, _jid: &str, _tipe: &str) -> Result<Option<String>, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn metadata_grup(&self, _jid: &str) -> Result<Value, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn semua_grup(&self) -> Result<Value, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn ubah_peserta(
        &self,
        _jid: &str,
        _peserta: &[String],
        _aksi: AksiPeserta,
    ) -> Result<(), SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn set_foto_profil(&self, _jid: &str, _isi: &[u8]) -> Result<(), SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn keluar_grup(&self, _jid: &str) -> Result<(), SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn kode_undangan(&self, _jid: &str) -> Result<String, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn info_undangan(&self, _kode: &str) -> Result<Value, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn buat_grup(&self, _subjek: &str, _peserta: &[String]) -> Result<Value, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn terima_undangan(&self, _kode: &str) -> Result<String, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn tandai_dibaca(&self, _pesan: &[Value]) -> Result<(), SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn presence(&self, _jenis: &str, _jid: &str) -> Result<(), SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn status_teks(&self, _jid: &str) -> Result<Value, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn nama_kontak(&self, _jid: &str) -> Result<String, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }

    fn profil_bisnis(&self, _jid: &str) -> Result<Option<Value>, SockError> {
        Err(SockError::baru("sock tidak tersedia"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aksi_peserta_memakai_kata_baileys() {
        // Berkas plugin JS menulis "add"/"remove"/"promote"/"demote";
        // kalau kata ini berubah, port-nya ikut salah.
        assert_eq!(AksiPeserta::Tambah.kata(), "add");
        assert_eq!(AksiPeserta::Keluar.kata(), "remove");
        assert_eq!(AksiPeserta::Naikkan.kata(), "promote");
        assert_eq!(AksiPeserta::Turunkan.kata(), "demote");
    }

    #[test]
    fn sock_mati_selalu_gagal_tanpa_panic() {
        let s = SockMati;
        assert!(s.ada_whatsapp(&["628111@s.whatsapp.net".into()]).is_err());
        assert!(s.foto_profil("628111@s.whatsapp.net", "image").is_err());
        assert!(s.metadata_grup("628111@g.us").is_err());
        assert!(s.semua_grup().is_err());
        assert!(s.ubah_peserta("628111@g.us", &[], AksiPeserta::Naikkan).is_err());
        assert!(s.set_foto_profil("628111@s.whatsapp.net", b"x").is_err());
        assert!(s.keluar_grup("628111@g.us").is_err());
        assert!(s.kode_undangan("628111@g.us").is_err());
        assert!(s.info_undangan("abc").is_err());
        assert!(s.buat_grup("x", &[]).is_err());
        assert!(s.terima_undangan("abc").is_err());
        assert!(s.tandai_dibaca(&[]).is_err());
        assert!(s.presence("available", "628111@s.whatsapp.net").is_err());
        assert!(s.status_teks("628111@s.whatsapp.net").is_err());
        assert!(s.nama_kontak("628111@s.whatsapp.net").is_err());
        assert!(s.profil_bisnis("628111@s.whatsapp.net").is_err());
    }
}
