//! `plugins/canvas/musiccard.js` — kartu musik dari gambar + judul/artis.
//!
//! `ponytail:` `ImageUploadService` (pixhost.to) dan
//! `api.nexray.eu.cc/canvas/musiccard` untuk menghasilkan gambar kartu tidak
//! tersedia di `Ctx`/`Reply`; handler untuk gambar+teks lengkap diakhiri
//! `error_template` + `☢` (pola `swm.rs`). Unduh gambar (tersedia via
//! `Ctx::download_media`) dan pemisahan judul/artis di-port agar bisa diuji.
//! Saat unggah + API kartu tersedia, tambahkan `pub mod musiccard;` di
//! `src/canvas/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `m.pushName` tidak tersedia di `Ctx`; fallback nama artis seperti JS
/// (`parts[1].trim() || m.pushName` → `"Unknown Artist"`).
#[cfg(test)]
const NAMA_ARTIS_DEFAULT: &str = "Unknown Artist";

/// Keterangan judul belum diisi (`musiccard.js:60-62`).
pub(crate) const JUDUL_BELUM_DIISI: &str = "❌ *Judul lagu dan artis belum diisi!*\n\nFormat penulisan yang benar adalah: `.musiccard <judul>|<nama>`\nPisahkan judul dan nama artis dengan simbol pita ( | ).";

/// Keterangan gambar tidak terdeteksi (`musiccard.js:56-58`).
pub(crate) const GAMBAR_TIDAK_TERDETEKSI: &str = "❌ *Gambar tidak terdeteksi!* Pastikan kamu mengirim gambar dengan benar.";

/// Unduh gambar: quoted dulu, baru trigger (`musiccard.js:32-36` / `46-50`).
pub(crate) fn unduh_media(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some() {
        ctx.download_quoted_media().ok()
    } else if ctx.is_media() {
        ctx.download_media().ok()
    } else {
        None
    }
}

/// Pisahkan `judul|nama` (`musiccard.js:67-71`: `text.split("|")`, ambil
/// `parts[0]` dan `parts[1]` — pipa ketiga dst. dibuang). Tanpa `|`: nama =
/// `NAMA_ARTIS_DEFAULT` (karena `m.pushName` tidak ada di `Ctx`).
#[cfg(test)]
pub(crate) fn pisah_judul_nama(text: &str) -> (String, String) {
    let mut bagian = text.split('|');
    let judul = bagian.next().unwrap_or_default().trim().to_string();
    let nama = bagian.next().unwrap_or_default().trim();
    let nama = if nama.is_empty() {
        NAMA_ARTIS_DEFAULT.to_string()
    } else {
        nama.to_string()
    };
    (judul, nama)
}

#[plugin(
    name = "musiccard",
    alias = ["mcard", "spotifycard"],
    category = "canvas",
    description = "Membuat kartu musik (music card) keren dari gambar yang dikirim.",
    usage = ".musiccard <judul>|<nama artis>",
    example = ".musiccard Rewrite The Stars|James Arthur",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn musiccard(ctx: Ctx) -> Reply {
    let Some(_buffer) = unduh_media(&ctx) else {
        return Reply::text(GAMBAR_TIDAK_TERDETEKSI);
    };

    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(JUDUL_BELUM_DIISI);
    }

    // `ImageUploadService` (pixhost.to) dan `api.nexray.eu.cc/canvas/musiccard`
    // tidak tersedia — diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{IncomingKind, IncomingMedia, QuotedKind, QuotedMessage};
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct StubFetch {
        ok: bool,
        bytes: Vec<u8>,
    }

    impl oktz_plugin_api::MediaFetch for StubFetch {
        fn download(&self, _: oktz_plugin_api::MediaOrigin) -> Result<Vec<u8>, oktz_plugin_api::MediaError> {
            if self.ok {
                Ok(self.bytes.clone())
            } else {
                Err(oktz_plugin_api::MediaError {
                    message: "gagal".into(),
                })
            }
        }
    }

    fn run(has_media: bool, quoted_media: bool, text: &str) -> Reply {
        let media = if has_media {
            Some(IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            })
        } else {
            None
        };
        let quoted = if quoted_media {
            Some(QuotedMessage {
                id: "Q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: String::new(),
                kind: QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            })
        } else {
            None
        };
        let ctx = Ctx {
            prefix: ".".into(),
            command: "musiccard".into(),
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            media,
            quoted,
            media_fetch: Arc::new(StubFetch {
                ok: true,
                bytes: b"IMG".to_vec(),
            }),
            ..Ctx::default()
        };
        musiccard(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("musiccard").unwrap();
        assert_eq!(p.meta.name, "musiccard");
        assert_eq!(p.meta.alias, ["mcard", "spotifycard"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn pisah_judul_nama_mirip_js() {
        assert_eq!(
            pisah_judul_nama("Rewrite The Stars|James Arthur"),
            ("Rewrite The Stars".to_string(), "James Arthur".to_string())
        );
        assert_eq!(
            pisah_judul_nama("Perfect|"),
            ("Perfect".to_string(), NAMA_ARTIS_DEFAULT.to_string())
        );
        assert_eq!(
            pisah_judul_nama("HanyaJudul"),
            ("HanyaJudul".to_string(), NAMA_ARTIS_DEFAULT.to_string())
        );
        assert_eq!(
            pisah_judul_nama("A|B|C"),
            ("A".to_string(), "B".to_string()),
            "musiccard.js:68 — split(\"|\") lalu ambil parts[0] dan parts[1]"
        );
    }

    #[test]
    fn tanpa_gambar_tetek_dan_tanpa_teks() {
        let r = run(false, false, "");
        assert_eq!(teks(&r), GAMBAR_TIDAK_TERDETEKSI);
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn ada_gambar_tanpa_teks_judul_belum_diisi() {
        let r = run(true, false, "");
        assert_eq!(teks(&r), JUDUL_BELUM_DIISI);
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn quoted_gambar_diprioritaskan_unduh() {
        let r = run(false, true, "Judul|Artis");
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "musiccard")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn trigger_gambar_dipakai_unduh() {
        let r = run(true, false, "Judul|Artis");
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "musiccard")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }
}
