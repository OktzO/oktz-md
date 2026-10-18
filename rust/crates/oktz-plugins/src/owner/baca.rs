//! `plugins/owner/baca.js` — tandai pesan sebagai sudah dibaca.
//!
//! `//! BLOKIRAN:` JS memanggil `sock.readMessages([m.key])`, tetapi `Ctx`
//! tidak exposes `key` pesan pemicu (hanya `chat`, `sender`, dll.), sehingga
//! `tandai_dibaca` tidak dapat dipanggil dengan key yang benar. Implementasi
//! di bawah memanggil `tandai_dibaca(&[])` sebagai pendekat, lalu reaksi ✅
//! dan balasan — teks balasan sama persis dengan JS.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "baca",
    alias = ["read", "markread"],
    category = "owner",
    description = "Tandai pesan sebagai sudah dibaca",
    usage = ".baca",
    example = ".baca",
    cooldown = 3,
    energi = 0,
    is_owner = true,
    is_group = false,
    is_private = false,
    is_enabled = true
)]
pub fn baca(ctx: Ctx) -> Reply {
    let _ = ctx.sock().tandai_dibaca(&[]);
    Reply::text("📖 *Pesan ditandai sudah dibaca*").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::sock::{AdaWhatsapp, Sock, SockError};

    #[derive(Debug, Default)]
    struct StubSock {
        dipanggil: std::sync::Mutex<Option<Vec<serde_json::Value>>>,
    }

    impl Sock for StubSock {
        fn ada_whatsapp(&self, _: &[String]) -> Result<Vec<AdaWhatsapp>, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn foto_profil(&self, _: &str, _: &str) -> Result<Option<String>, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn metadata_grup(&self, _: &str) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn semua_grup(&self) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn ubah_peserta(&self, _: &str, _: &[String], _: oktz_plugin_api::sock::AksiPeserta) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn set_foto_profil(&self, _: &str, _: &[u8]) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn keluar_grup(&self, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn kode_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn info_undangan(&self, _: &str) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn buat_grup(&self, _: &str, _: &[String]) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn terima_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn tandai_dibaca(&self, pesan: &[serde_json::Value]) -> Result<(), SockError> {
            *self.dipanggil.lock().unwrap() = Some(pesan.to_vec());
            Ok(())
        }
        fn presence(&self, _: &str, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn status_teks(&self, _: &str) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn nama_kontak(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
        fn profil_bisnis(&self, _: &str) -> Result<Option<serde_json::Value>, SockError> {
            Err(SockError::baru("tidak dipakai"))
        }
    }

    fn run() -> (Reply, StubSock) {
        let sock = StubSock::default();
        let sock_clone = sock.clone();
        let ctx = Ctx {
            command: "baca".into(),
            sock: std::sync::Arc::new(sock_clone),
            ..Ctx::default()
        };
        let r = baca(ctx);
        (r, sock)
    }

    #[test]
    fn tandai_dibaca_dipanggil_dan_balasan_sama_dengan_js() {
        let (r, sock) = run();
        assert_eq!(
            r.without_mutations().text_body(),
            Some("📖 *Pesan ditandai sudah dibaca*")
        );
        assert_eq!(sock.dipanggil.lock().unwrap().as_deref(), Some(&[]));
    }

    #[test]
    fn reaksi_checkmark() {
        let (r, _sock) = run();
        match r.without_mutations() {
            Reply::Reaction { emoji, reply } => {
                assert_eq!(emoji, "✅");
                assert_eq!(reply.text_body(), Some("📖 *Pesan ditandai sudah dibaca*"));
            }
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}
