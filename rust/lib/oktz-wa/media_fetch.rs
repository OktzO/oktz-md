//! `m.download()` — unduh media pesan dari CDN WhatsApp.
//!
//! Padanan `downloadContentFromMessage` di `serialize.js:1669` (`m.download`)
//! dan `:543` (`m.quoted.download`). Di Rust jalurnya `Client::download`
//! (`whatsapp-rust-0.7.0/src/download.rs:531`) yang menerima proto media
//! mentah sebagai `Downloadable`.
//!
//! Tiga hal yang perlu diketahui pembaca modul ini:
//!
//! - Handler plugin sinkron, sedangkan `Client::download` async. Unduhan
//!   terjadi DI DALAM handler (JS juga `await` di situ: bufernya dipakai untuk
//!   resize/tulis sebelum balasan dikirim), jadi jalurnya memblokir thread
//!   worker lewat `block_in_place` — pola yang sama dengan `UreqHttp` yang
//!   memblokir selama request (`oktz-plugin-api/src/ureq_http.rs:1-7`).
//!   Runtime produksi multi-thread (`oktz-bin/src/main.rs:80`), jadi
//!   `block_in_place` sah; test tidak pernah masuk sini karena pengirimnya
//!   stub — `Ctx` memakai `NoMediaFetch` dan gagal dengan alasan jelas.
//! - JS mengulang unduhan lewat `sock.updateMediaMessage` kalau CDN menjawab
//!   400/404/410/428/429/500/870 (`serialize.js:1684-1707`). Itu butuh
//!   re-enkripsi sisi klien yang belum ada di Rust: kegagalan CDN langsung
//!   jadi `Err`, dan bedanya dicatat di `docs/rust-rewrite/BLOCKED.md`.
//! - `m.download(filename)` yang menulis ke `storage/temp` TIDAK di-port
//!   (butuh `fs`); pemanggil menerima byte-nya dan yang berurusan dengan
//!   berkas lokal tetap terblokir seperti sebelumnya.

use std::sync::Arc;

use oktz_plugin_api::{
    IncomingKind, IncomingMedia, MediaError, MediaFetch, MediaOrigin,
};
use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore::download::{Downloadable, MediaType};

use crate::pipeline::extract_body;

/// Field CDN satu media, disalin dari proto supaya hidup lebih lama dari
/// `InboundMessage` (handler plugin dipanggil setelah `m` tidak lagi dipinjam).
#[derive(Debug, Clone)]
pub struct MediaData {
    /// `m.isImage`/`m.mimetype`/dll yang dibaca plugin.
    pub info: IncomingMedia,
    direct_path: Option<String>,
    static_url: Option<String>,
    media_key: Vec<u8>,
    file_enc_sha256: Vec<u8>,
    file_sha256: Vec<u8>,
    file_length: u64,
    media_type: MediaType,
}

impl Downloadable for MediaData {
    fn direct_path(&self) -> Option<&str> {
        self.direct_path.as_deref()
    }

    fn media_key(&self) -> Option<&[u8]> {
        // `Some(vec![])` dan `None` sama-sama berarti "tidak ada kunci":
        // mengembalikan slice kosong akan bikin `is_encrypted()` bohong.
        (!self.media_key.is_empty()).then_some(self.media_key.as_slice())
    }

    fn file_enc_sha256(&self) -> Option<&[u8]> {
        (!self.file_enc_sha256.is_empty()).then_some(self.file_enc_sha256.as_slice())
    }

    fn file_sha256(&self) -> Option<&[u8]> {
        (!self.file_sha256.is_empty()).then_some(self.file_sha256.as_slice())
    }

    fn file_length(&self) -> Option<u64> {
        Some(self.file_length)
    }

    fn app_info(&self) -> MediaType {
        self.media_type
    }

    /// `staticUrl` — ada di `imageMessage`/`videoMessage` kiriman saluran;
    /// kalau ada, unduhan melewati konstruksi host (`wacore/src/download.rs:132`).
    fn static_url(&self) -> Option<&str> {
        self.static_url.as_deref()
    }
}

impl MediaData {
    /// Baca media dari proto pesan; `None` kalau pesannya bukan media.
    ///
    /// Urutannya mengikuti `getContentType`: image → video → audio → sticker →
    /// document. Satu pesan hanya boleh punya satu media (WA tidak pernah
    /// mengirim dua), jadi urutan ini tidak pernah "kalah cepat" satu sama
    /// lain seperti di JS.
    pub fn of(m: &wa::Message) -> Option<Self> {
        let b = m.get_base_message();
        let caption = extract_body(m).unwrap_or_default();

        if let Some(i) = b.image_message.as_option() {
            return Some(Self {
                info: IncomingMedia {
                    kind: IncomingKind::Image,
                    mimetype: i.mimetype.clone(),
                    file_name: None,
                    caption,
                },
                direct_path: i.direct_path.clone(),
                static_url: i.static_url.clone(),
                media_key: i.media_key.clone().unwrap_or_default(),
                file_enc_sha256: i.file_enc_sha256.clone().unwrap_or_default(),
                file_sha256: i.file_sha256.clone().unwrap_or_default(),
                file_length: i.file_length.unwrap_or_default(),
                media_type: MediaType::Image,
            });
        }
        if let Some(v) = b.video_message.as_option() {
            return Some(Self {
                info: IncomingMedia {
                    kind: IncomingKind::Video,
                    mimetype: v.mimetype.clone(),
                    file_name: None,
                    caption,
                },
                direct_path: v.direct_path.clone(),
                static_url: v.static_url.clone(),
                media_key: v.media_key.clone().unwrap_or_default(),
                file_enc_sha256: v.file_enc_sha256.clone().unwrap_or_default(),
                file_sha256: v.file_sha256.clone().unwrap_or_default(),
                file_length: v.file_length.unwrap_or_default(),
                media_type: MediaType::Video,
            });
        }
        if let Some(a) = b.audio_message.as_option() {
            return Some(Self {
                info: IncomingMedia {
                    kind: IncomingKind::Audio,
                    mimetype: a.mimetype.clone(),
                    file_name: None,
                    caption,
                },
                direct_path: a.direct_path.clone(),
                static_url: None,
                media_key: a.media_key.clone().unwrap_or_default(),
                file_enc_sha256: a.file_enc_sha256.clone().unwrap_or_default(),
                file_sha256: a.file_sha256.clone().unwrap_or_default(),
                file_length: a.file_length.unwrap_or_default(),
                media_type: MediaType::Audio,
            });
        }
        if let Some(s) = b.sticker_message.as_option() {
            return Some(Self {
                info: IncomingMedia {
                    kind: IncomingKind::Sticker,
                    mimetype: s.mimetype.clone(),
                    file_name: None,
                    caption,
                },
                direct_path: s.direct_path.clone(),
                static_url: None,
                media_key: s.media_key.clone().unwrap_or_default(),
                file_enc_sha256: s.file_enc_sha256.clone().unwrap_or_default(),
                file_sha256: s.file_sha256.clone().unwrap_or_default(),
                file_length: s.file_length.unwrap_or_default(),
                media_type: MediaType::Sticker,
            });
        }
        if let Some(d) = b.document_message.as_option() {
            return Some(Self {
                info: IncomingMedia {
                    kind: IncomingKind::Document,
                    mimetype: d.mimetype.clone(),
                    file_name: d.file_name.clone(),
                    caption,
                },
                direct_path: d.direct_path.clone(),
                static_url: None,
                media_key: d.media_key.clone().unwrap_or_default(),
                file_enc_sha256: d.file_enc_sha256.clone().unwrap_or_default(),
                file_sha256: d.file_sha256.clone().unwrap_or_default(),
                file_length: d.file_length.unwrap_or_default(),
                media_type: MediaType::Document,
            });
        }
        None
    }
}

/// `m.download()` + `m.quoted.download()` untuk satu pesan masuk.
pub struct WaMediaFetch {
    client: Arc<Client>,
    trigger: Option<MediaData>,
    quoted: Option<MediaData>,
}

impl std::fmt::Debug for WaMediaFetch {
    /// `Client` tidak `Debug`; yang berguna saat gagal cuma ada-tidaknya media.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaMediaFetch")
            .field("ada_media_pemicu", &self.trigger.is_some())
            .field("ada_media_kutipan", &self.quoted.is_some())
            .finish()
    }
}

impl MediaFetch for WaMediaFetch {
    fn download(&self, origin: MediaOrigin) -> Result<Vec<u8>, MediaError> {
        let data = match origin {
            MediaOrigin::Trigger => self.trigger.as_ref(),
            MediaOrigin::Quoted => self.quoted.as_ref(),
        };
        let Some(data) = data else {
            return Err(MediaError {
                message: format!("tidak ada media di {}", origin.label()),
            });
        };
        let client = self.client.clone();
        // Lihat catatan modul: satu-satunya tempat menunggu I/O dari handler
        // sinkron, dan runtime produksi memang multi-thread.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.download(data))
        })
        .map_err(|e| MediaError {
            message: format!("unduh media {} gagal: {e}", origin.label()),
        })
    }
}

/// Media pesan yang di-reply, kalau ada (`contextInfo.quotedMessage`).
fn quoted_media_of(m: &wa::Message) -> Option<MediaData> {
    let b = m.get_base_message();
    let ci = [
        b.extended_text_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
        b.image_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
        b.video_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
        b.document_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
        b.audio_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
        b.sticker_message
            .as_option()
            .and_then(|v| v.context_info.as_option()),
    ]
    .into_iter()
    .flatten()
    .next()?;
    MediaData::of(ci.quoted_message.as_option()?)
}

/// Susun isi `Ctx::media` dan `Ctx::media_fetch` untuk satu pesan masuk.
///
/// `client` = `None` untuk pengirim stub (test): `Ctx::media` tetap dibaca
/// plugin, tapi unduhannya menolak dengan alasan jelas lewat `NoMediaFetch`.
pub fn ctx_media(
    client: Option<Arc<Client>>,
    m: &InboundMessage,
) -> (Option<IncomingMedia>, Option<Arc<dyn MediaFetch>>) {
    let trigger = MediaData::of(&m.message);
    let quoted = quoted_media_of(&m.message);
    let info = trigger.as_ref().map(|d| d.info.clone());
    let fetch: Option<Arc<dyn MediaFetch>> = match client {
        Some(client) if trigger.is_some() || quoted.is_some() => Some(Arc::new(WaMediaFetch {
            client,
            trigger,
            quoted,
        })),
        _ => None,
    };
    (info, fetch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use whatsapp_rust::waproto::buffa::MessageField;
    use whatsapp_rust::waproto::whatsapp as wa;

    fn gambar() -> wa::Message {
        wa::Message {
            image_message: MessageField::some(wa::message::ImageMessage {
                direct_path: Some("/v/t62/gambar".to_string()),
                media_key: Some(vec![1, 2, 3]),
                file_enc_sha256: Some(vec![4, 5]),
                file_sha256: Some(vec![6, 7]),
                file_length: Some(1234),
                mimetype: Some("image/jpeg".to_string()),
                caption: Some("kucing lucu".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn media_gambar_membawa_field_cdn_dan_info_js() {
        let d = MediaData::of(&gambar()).expect("harus media");
        assert_eq!(d.info.kind, IncomingKind::Image);
        assert_eq!(d.info.mimetype.as_deref(), Some("image/jpeg"));
        assert_eq!(d.info.file_name, None);
        assert_eq!(d.info.caption, "kucing lucu");
        assert_eq!(d.direct_path(), Some("/v/t62/gambar"));
        assert_eq!(d.media_key(), Some([1u8, 2, 3].as_slice()));
        assert_eq!(d.file_length(), Some(1234));
        assert_eq!(d.app_info(), MediaType::Image);
        // `m.type` — nilai wire yang dipakai plugin JS.
        assert_eq!(d.info.kind.wire(), "imageMessage");
    }

    #[test]
    fn pesan_teks_bukan_media() {
        let m = wa::Message {
            conversation: Some("halo".to_string()),
            ..Default::default()
        };
        assert!(MediaData::of(&m).is_none());
    }

    #[test]
    fn dokumen_membawa_nama_berkas() {
        let m = wa::Message {
            document_message: MessageField::some(wa::message::DocumentMessage {
                direct_path: Some("/v/dok".to_string()),
                media_key: Some(vec![9]),
                file_name: Some("laporan.pdf".to_string()),
                file_length: Some(42),
                ..Default::default()
            }),
            ..Default::default()
        };
        let d = MediaData::of(&m).expect("harus media");
        assert_eq!(d.info.kind, IncomingKind::Document);
        assert_eq!(d.info.file_name.as_deref(), Some("laporan.pdf"));
        assert_eq!(d.info.caption, "");
        assert_eq!(d.app_info(), MediaType::Document);
    }

    #[test]
    fn media_tanpa_kunci_dianggap_tidak_terenkripsi() {
        // Kiriman saluran: `mediaKey` tidak ada dan `staticUrl` yang dipakai.
        let d = MediaData {
            info: IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            },
            direct_path: None,
            static_url: Some("https://mmg.whatsapp.net/x".to_string()),
            media_key: Vec::new(),
            file_enc_sha256: Vec::new(),
            file_sha256: Vec::new(),
            file_length: 10,
            media_type: MediaType::Image,
        };
        assert!(!d.is_encrypted());
        assert_eq!(d.static_url(), Some("https://mmg.whatsapp.net/x"));
        assert_eq!(d.file_enc_sha256(), None);
    }

    #[test]
    fn kutipan_media_dibaca_dari_context_info() {
        let pesan = wa::Message {
            extended_text_message: MessageField::some(wa::message::ExtendedTextMessage {
                text: Some(".ganti-large".to_string()),
                context_info: MessageField::some(wa::ContextInfo {
                    quoted_message: MessageField::some(gambar()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let q = quoted_media_of(&pesan).expect("kutipan harus terbaca");
        assert_eq!(q.info.kind, IncomingKind::Image);
        assert_eq!(q.direct_path(), Some("/v/t62/gambar"));
        // Pesan pemicunya sendiri teks: `Ctx::media` harus kosong.
        assert!(MediaData::of(&pesan).is_none());
    }
}
