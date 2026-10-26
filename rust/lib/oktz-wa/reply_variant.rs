//! `replyVariant` — bentuk balasan yang dilihat user, dibaca dari
//! `db.setting("replyVariant")` (`serialize.js:976-984`).
//!
//! Satu setting, sebelas bentuk pesan yang BERBEDA. Yang berbeda bukan nama
//! field tapi isi yang user lihat: dokumen `package.json` dengan kutipan
//! kontak palsu (varian 2), video GIF dengan caption (3), link preview (4),
//! teks dengan order palsu (5), order card `WXX`/`CONTOL` (9), tagihan (10),
//! favicon MMS (11). Meratakannya berarti mengirim bubble yang berbeda dari
//! produksi, jadi tiap varian punya builder sendiri.
//!
//! # Matriks varian
//!
//! | varian | wire | kutipan | thumbnail | `forwardingScore` |
//! |---|---|---|---|---|
//! | 1 `Basic` | teks biasa | pemicu | — | — |
//! | 2 `DocumentContact` | dokumen + kontak palsu | kontak palsu | srt/foto2 | — |
//! | 3 `GifVideo` | video `gifPlayback` | pemicu | — | — |
//! | 4 `LinkPreview` | preview link | pemicu | srt/foto | 9 |
//! | 5 `FakeOrderQuote` | teks + order palsu | order palsu | srt/foto | — |
//! | 6 `TriggerDocument` | dokumen | pemicu | srt/foto2 | — |
//! | 7 `LocationCard` | viewOnce + kartu lokasi | — | srt/foto | 9 |
//! | 8 `SignupCard` | viewOnce + tombol signup | order palsu | srt/foto | 999 |
//! | 9 `Order` | `orderMessage` | pemicu (pesan asli) | srt/foto | — |
//! | 10 `Payment` | `requestPaymentMessage` | pemicu (di catatan) | — | — |
//! | 11 `FaviconPreview` | preview + favicon MMS | kontak palsu | srt/foto | 999 |
//!
//! # Dua aturan yang dijaga ketat
//!
//! 1. **Hanya ANGKA 2..11** punya builder sendiri. String, `0`, `1`, pecahan,
//!    dan di luar rentang jatuh ke [`ReplyVariant::Basic`] — persis rangkaian
//!    `===` angka di JS, di mana `"9"` dan `9.5` tidak pernah masuk cabang.
//! 2. **Hanya balasan berbentuk teks** yang ikut varian. Kartu, tabel, dan
//!    media punya wire contract sendiri (`render_reply`), dan `serialize.js`
//!    hanya punya `text` untuk jalur varian.
//!
//! `ponytail:` `sharp` tidak ada di crate ini (`image` bukan dependency
//! `oktz-wa`), jadi thumbnail dikirim sebagai byte aset apa adanya — sama
//! seperti cabang `catch` JS yang menjatuhkan `jpegThumbnail` ke buffer
//! mentah (`socket.js:1234`). Upgrade path: tambahkan dependency image lalu
//! resize di fungsi `thumbnail`; kontrak yang dikunci test tetap "byte dari
//! aset yang benar".

use std::path::{Path, PathBuf};

use oktz_core::config::Runtime;
use whatsapp_rust::prelude::wa;
use whatsapp_rust::wacore::proto_helpers::MessageBuilderExt;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;
use whatsapp_rust::waproto::whatsapp::context_info::ForwardedNewsletterMessageInfo;
use whatsapp_rust::waproto::whatsapp::message::extended_text_message::PreviewType;
use whatsapp_rust::waproto::whatsapp::message::interactive_message::header::Media as HeaderMedia;
use whatsapp_rust::waproto::whatsapp::message::interactive_message::native_flow_message::NativeFlowButton;
use whatsapp_rust::waproto::whatsapp::message::interactive_message::{Body, Header};
use whatsapp_rust::waproto::whatsapp::message::order_message::{OrderStatus, OrderSurface};

use crate::pipeline::{MediaKind, PendingMedia, QuoteSource, Rendered, ReplyTarget, context_info};

/// `document: package.json` (`serialize.js:1015` dan `:1119`) — dibaca dari
/// direktori kerja, sama seperti `join(process.cwd(), "package.json")`.
const PAKET_JSON: &str = "package.json";

/// `mimetype: "image/png"` untuk dokumen varian 2/6 — palsu tapi persis, itu
/// yang ditulis JS (`:1017`, `:1121`).
const MIME_DOKUMEN: &str = "image/png";

/// `fileLength: 99999999999999` (`:1019`, `:1123`) — nilai palsu yang harus
/// terkirim APA ADANYA: user melihat angka itu di judul dokumen.
const FILE_LENGTH_DOKUMEN: u64 = 99_999_999_999_999;

/// `item1.TEL;waid=13135550002` — vCard kontak palsu varian 2 (`:1005`).
const VCARD_KONTAK_PALSU: &str = "BEGIN:VCARD\nVERSION:3.0\nN:XL;ttname,;;;\nFN:ttname\nitem1.TEL;waid=13135550002:+1 (313) 555-0002\nitem1.X-ABLabel:Ponsel\nEND:VCARD";

/// `🪸 ${config.bot?.name}` (`:1004`) — U+1FAB8, coral.
const EMAIL_BATU_KARANG: &str = "\u{1fab8}";

/// `address: "Bot Wa Multi Device"` (`:1152`).
const ALAMAT_BOT: &str = "Bot Wa Multi Device";

/// `forwardingScore: 9` — varian 4 dan 7.
const SKOR_LINK: u32 = 9;

/// `forwardingScore: 999` — varian 8 dan 11.
const SKOR_KARTU: u32 = 999;

/// Cadangan path aset kalau `Runtime::assets` kosong (test, config tanpa
/// `assets`). Nilainya sama dengan `config.js:44-66`.
const FOTO: &str = "./assets/image/foto.jpeg";
const FOTO2: &str = "./assets/image/foto2.jpeg";
const MP4: &str = "./assets/video/mp4.mp4";

/// `db.setting("replyVariant")` → varian. HANYA angka bulat 2..11.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyVariant {
    /// 1 (dan 0, string, pecahan, di luar rentang) — balasan teks biasa.
    Basic,
    /// 2 — dokumen `package.json` dengan kutipan kontak palsu.
    DocumentContact,
    /// 3 — video `gifPlayback`.
    GifVideo,
    /// 4 — link preview (`sendPreview`).
    LinkPreview,
    /// 5 — teks dengan kutipan order palsu.
    FakeOrderQuote,
    /// 6 — dokumen dengan kutipan pemicu.
    TriggerDocument,
    /// 7 — kartu lokasi dalam `viewOnceMessage`.
    LocationCard,
    /// 8 — kartu signup dalam `viewOnceMessage`.
    SignupCard,
    /// 9 — `orderMessage` (jalan yang aktif di deployment).
    Order,
    /// 10 — `requestPaymentMessage`.
    Payment,
    /// 11 — preview link dengan `faviconMMSMetadata`.
    FaviconPreview,
}

/// `db.setting("replyVariant") || ... || 1` (`serialize.js:976-984`) — dibaca
/// dari setting yang SUDAH diambil, bukan dari store langsung, supaya jalur
/// ini murni dan bisa diuji tanpa disk.
///
/// `serde_json` menyimpan angka bulat JSON sebagai integer, tapi `2.0` mendarat
/// sebagai `f64` — dan `as_f64` menerima keduanya. Itu memang yang dibutuhkan:
/// `2.0 === 2` di JS (`:995` dan seterusnya rangkaian `===`), jadi 2.0 tetap
/// masuk cabang varian 2. Pecahan (`2.5`, `9.5`) dan string `"9"` TIDAK.
pub fn variant_from(settings: &serde_json::Value) -> ReplyVariant {
    let angka = settings
        .get("replyVariant")
        .and_then(serde_json::Value::as_f64)
        .filter(|f| f.fract() == 0.0 && f.is_finite())
        .map(|f| f as i64);
    match angka {
        Some(2) => ReplyVariant::DocumentContact,
        Some(3) => ReplyVariant::GifVideo,
        Some(4) => ReplyVariant::LinkPreview,
        Some(5) => ReplyVariant::FakeOrderQuote,
        Some(6) => ReplyVariant::TriggerDocument,
        Some(7) => ReplyVariant::LocationCard,
        Some(8) => ReplyVariant::SignupCard,
        Some(9) => ReplyVariant::Order,
        Some(10) => ReplyVariant::Payment,
        Some(11) => ReplyVariant::FaviconPreview,
        _ => ReplyVariant::Basic,
    }
}

/// `true` kalau varian ini punya builder sendiri (`Basic` tidak).
pub fn needs_custom_render(variant: ReplyVariant) -> bool {
    !matches!(variant, ReplyVariant::Basic)
}

/// Semua data yang dibutuhkan builder varian: teks balasan, mention, pemicu
/// (`m` di JS), config, dan `srtImage`.
pub struct VariantInput<'a> {
    pub text: &'a str,
    pub mentions: &'a [String],
    pub target: &'a ReplyTarget,
    pub config: &'a Runtime,
    pub srt_enabled: bool,
    pub srt_image: Option<&'a [u8]>,
}

/// Hasil render: pesan wire siap kirim, atau media yang harus di-upload dulu.
///
/// Varian 2/3/6/4/11 butuh upload (`prepareWAMessageMedia`/`sendMedia` di JS),
/// jadi tidak bisa jadi `wa::Message` di sini — byte CDN-nya baru ada setelah
/// `Client::upload`. Pemanggil (= pipeline) yang memutuskan jalur kirim.
#[derive(Debug)]
pub enum VariantRender {
    Wire(wa::Message),
    Media(PendingMedia),
}

/// Render varian. `None` hanya untuk [`ReplyVariant::Basic`] — pemanggil
/// sudah memastikan varian itu tidak pernah masuk sini.
pub fn render(variant: ReplyVariant, input: &VariantInput<'_>) -> Option<VariantRender> {
    let media = |p: PendingMedia| Some(VariantRender::Media(p));
    match variant {
        ReplyVariant::Basic => None,
        ReplyVariant::DocumentContact => media(dokumen(input, Some(kontak_palsu(input)))),
        ReplyVariant::GifVideo => media(video(input)),
        ReplyVariant::LinkPreview => media(link_preview(input)),
        ReplyVariant::FakeOrderQuote => Some(VariantRender::Wire(order_palsu_teks(input))),
        ReplyVariant::TriggerDocument => media(dokumen(input, input.target.quote.clone())),
        ReplyVariant::LocationCard => Some(VariantRender::Wire(kartu_lokasi(input))),
        ReplyVariant::SignupCard => Some(VariantRender::Wire(kartu_signup(input))),
        ReplyVariant::Order => Some(VariantRender::Wire(order(input))),
        ReplyVariant::Payment => Some(VariantRender::Wire(pembayaran(input))),
        ReplyVariant::FaviconPreview => media(favicon(input)),
    }
}

// ---------------------------------------------------------------------------
// pembantu bersama
// ---------------------------------------------------------------------------

/// `getAssetBuffer(nama)` — `fs.readFileSync(config.assets[nama])`
/// (`asset-manager.js:36-59`). File hilang = byte kosong, bukan panic: JS
/// juga mengembalikan `null` dan pemanggilnya sudah menanganinya.
fn asset(config: &Runtime, nama: &str, bawaan: &str) -> Vec<u8> {
    let path = config.assets.get(nama).unwrap_or(bawaan);
    std::fs::read(path).unwrap_or_default()
}

/// `srtImage || getAssetBuffer(nama)` (`serialize.js:1051`, `:1079`, ...).
///
/// `srtImage` hanya diisi kalau `srtEnabled` (`:969-974`); flag tanpa gambar
/// menghasilkan `None`, sama seperti `getCachedSrtImage()` yang bisa `null`.
fn thumbnail(input: &VariantInput<'_>, nama: &str, bawaan: &str) -> Vec<u8> {
    if input.srt_enabled
        && let Some(b) = input.srt_image
    {
        return b.to_vec();
    }
    asset(input.config, nama, bawaan)
}

/// `config.bot?.name || "Bot"`.
fn nama_bot(config: &Runtime) -> String {
    if config.bot.name.is_empty() {
        "Bot".into()
    } else {
        config.bot.name.clone()
    }
}

/// `m?.sender` — bisa kosong di test/kode non-WA, dan JS menulis `undefined`
/// untuk string kosong (field-nya hilang, bukan string kosong).
fn jid_atau_kosong(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// `options?.mentions || [m?.sender] || []` (`serialize.js:987`).
///
/// Daftarnya dihitung dari `Reply` yang sedang dikirim, bukan di sini:
/// `TextNoMention` = `mentions: []` eksplisit di JS — dan array kosong adalah
/// TRUTHY di JS, jadi `[] || [m.sender]` = `[]`. Kalau pemanggil tidak tahu
/// varian `Reply`-nya, daftar ini akan salah untuk `TextNoMention`; lihat
/// `mentions_of_reply` di `pipeline.rs`, yang tahu.
fn mentions(input: &VariantInput<'_>) -> Vec<String> {
    input.mentions.to_vec()
}

/// `contextInfo` kutipan persis seperti yang ditulis `options.quoted` di
/// pipeline (`context_info`).
fn context_kutipan(input: &VariantInput<'_>) -> wa::ContextInfo {
    context_info(&mentions(input), input.target.quote.as_ref())
}

/// `m?.sender.split('@')[0]` (`:1298`) — nomor tanpa domain.
fn nomor(sender: &str) -> &str {
    sender.split('@').next().unwrap_or("")
}

// ---------------------------------------------------------------------------
// varian 2 & 6 — dokumen `package.json`
// ---------------------------------------------------------------------------

/// Kontak palsu varian 2 (`serialize.js:997-1009`): `participant` palsu,
/// `remoteJid: status@broadcast`, dan `vcard` dengan nomor `13135550002`.
///
/// `sendEphemeral: true` di JS tidak punya padanan proto di `ContactMessage`
/// (`whatsapp.proto:3105-3110`), jadi memang tidak ada di wire.
fn kontak_palsu(input: &VariantInput<'_>) -> QuoteSource {
    QuoteSource {
        stanza_id: String::new(),
        participant: Some("0@s.whatsapp.net".into()),
        message: Box::new(wa::Message {
            contact_message: MessageField::some(wa::message::ContactMessage {
                display_name: Some(format!(
                    "{EMAIL_BATU_KARANG} {}",
                    nama_bot(input.config)
                )),
                vcard: Some(VCARD_KONTAK_PALSU.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
    }
}

/// Varian 2 dan 6 memakai isi dokumen yang SAMA; bedanya hanya kutipan.
///
/// `jpegThumbnail` = `safeThumbnail(srtImage || getAssetBuffer("foto2"), 300)`
/// (`:1022-1024`, `:1126-1128`) — byte aset apa adanya, lihat catatan modul.
fn dokumen(input: &VariantInput<'_>, quote: Option<QuoteSource>) -> PendingMedia {
    PendingMedia {
        kind: MediaKind::Document,
        source: oktz_plugin_api::MediaSource::bytes(std::fs::read(PAKET_JSON).unwrap_or_default())
            .with_mimetype(MIME_DOKUMEN),
        caption: input.text.to_string(),
        mentions: mentions(input),
        quote,
        gif_playback: false,
        ptt: false,
        file_name: Some(nama_bot(input.config)),
        jpeg_thumbnail: Some(thumbnail(input, "foto2", FOTO2)),
        file_length: Some(FILE_LENGTH_DOKUMEN),
        preview: None,
    }
}

// ---------------------------------------------------------------------------
// varian 3 — video GIF
// ---------------------------------------------------------------------------

/// `getAssetBuffer("mp4")` + `gifPlayback: true`, memetik pemicu
/// (`serialize.js:1033-1049`).
///
/// `ponytail:` `jpegThumbnail` TIDAK dikirim — JS varian 3 juga tidak
/// menuliskannya; yang membuat thumbnail video (kalau ada) sisi
/// WA/Baileys saat merakit `videoMessage`. Rust tidak menambah dependency
/// ffmpeg/image untuk menirunya, jadi bubble GIF ini tanpa thumbnail sampai
/// perilaku aslinya dicek terhadap server nyata.
fn video(input: &VariantInput<'_>) -> PendingMedia {
    PendingMedia {
        kind: MediaKind::Video,
        source: oktz_plugin_api::MediaSource::path(MP4)
            .with_mimetype("video/mp4"),
        caption: input.text.to_string(),
        mentions: mentions(input),
        quote: input.target.quote.clone(),
        gif_playback: true,
        ptt: false,
        file_name: None,
        jpeg_thumbnail: None,
        file_length: None,
        preview: None,
    }
}

// ---------------------------------------------------------------------------
// varian 4 — link preview
// ---------------------------------------------------------------------------

/// `sendPreview` (`serialize.js:1050-1077`): teks berawalan website,
/// `previewType: 0`, forward score 9, dan metadata saluran.
///
/// `serverMessageId` = `Math.floor(Math.random() * 1000000)` (`:1073`) —
/// tanpa dependency RNG, diturunkan dari jam sistem.
fn link_preview(input: &VariantInput<'_>) -> PendingMedia {
    let website = if input.config.info.website.is_empty() {
        "https://github.com"
    } else {
        input.config.info.website.as_str()
    };
    let mut ci = context_kutipan(input);
    ci.is_forwarded = Some(true);
    ci.forwarding_score = Some(SKOR_LINK);
    ci.forwarded_newsletter_message_info =
        MessageField::some(newsletter_info(input, nama_saluran_atau_bot(input.config), true));

    let ext = wa::message::ExtendedTextMessage {
        text: Some(format!("{website}\n\n{}", input.text)),
        matched_text: Some(website.to_string()),
        // `canonicalUrl` hanya ada di `ConsumerApplication.ExtendedTextMessage`,
        // BUKAN di `Message.ExtendedTextMessage` yang dipakai oneof pesan
        // (`waproto-0.7.0/src/whatsapp.proto:3209-3241`) — `matchedText` yang
        // membawa URL yang sama di wire, seperti `sendPreview` JS
        // (`socket.js:1207-1211`).
        title: Some(nama_bot(input.config)),
        description: Some(format!(
            "Pengembang: {} | Versi: {}",
            input.config.bot.developer, input.config.bot.version
        )),
        preview_type: Some(PreviewType::NONE),
        jpeg_thumbnail: Some(thumbnail(input, "foto", FOTO)),
        context_info: MessageField::some(ci),
        ..Default::default()
    };

    let bytes = ext.jpeg_thumbnail.clone().unwrap_or_default();
    PendingMedia {
        kind: MediaKind::Image,
        source: oktz_plugin_api::MediaSource::bytes(bytes.clone()),
        caption: String::new(),
        mentions: Vec::new(),
        quote: None,
        gif_playback: false,
        ptt: false,
        file_name: None,
        jpeg_thumbnail: None,
        file_length: None,
        preview: Some(crate::pipeline::PendingPreview {
            message: wa::Message {
                extended_text_message: MessageField::some(ext),
                ..Default::default()
            },
            bytes,
            favicon: false,
        }),
    }
}

// ---------------------------------------------------------------------------
// varian 5 — teks dengan order palsu
// ---------------------------------------------------------------------------

/// `serialize.js:1078-1113` — balasannya teks BIASA, tapi kutipannya
/// `orderMessage` palsu berisi nilai statis.
fn order_palsu_teks(input: &VariantInput<'_>) -> wa::Message {
    let order = wa::message::OrderMessage {
        order_id: Some("123456".into()),
        // `sharp(thumbnailBuf).resize(300, 300)` — byte aset apa adanya.
        thumbnail: Some(thumbnail(input, "foto", FOTO)),
        item_count: Some(999),
        status: Some(OrderStatus::Inquiry),
        surface: Some(OrderSurface::Catalog),
        message: Some(nama_bot(input.config)),
        order_title: Some("System Notification".into()),
        seller_jid: Some("0@s.whatsapp.net".into()),
        token: Some("ARU1+".into()),
        // JS menulis string "1000000"; proto-nya int64.
        total_amount1000: Some(1_000_000),
        total_currency_code: Some("IDR".into()),
        ..Default::default()
    };
    let quote = QuoteSource {
        stanza_id: String::new(),
        participant: Some("0@s.whatsapp.net".into()),
        message: Box::new(wa::Message {
            order_message: MessageField::some(order),
            ..Default::default()
        }),
    };
    crate::pipeline::text_message(input.text, &mentions(input), Some(&quote))
}

// ---------------------------------------------------------------------------
// varian 7 — kartu lokasi
// ---------------------------------------------------------------------------

/// `generateWAMessageFromContent` + `relayMessage` (`serialize.js:1137-1174`).
///
/// `messageContextInfo: {}` di sini KOSONG — tanpa `deviceListMetadata` dan
/// tanpa `messageSecret` seperti kartu native-flow (`cards.rs`), dan itu
/// perbedaan yang dikunci test.
///
/// `jpegThumbnail` = `sharp(thumbnailBuf).resize(300, 300).toBuffer()` (`:1152`)
/// — byte aset apa adanya, lihat catatan modul.
fn kartu_lokasi(input: &VariantInput<'_>) -> wa::Message {
    let mut ci = context_kutipan(input);
    ci.is_forwarded = Some(true);
    ci.forwarding_score = Some(SKOR_LINK);

    // `locationMessage: { degreesLatitude, degreesLongitude, name, address }`
    // di JS (`:1146-1153`) adalah `Message.LocationMessage` — field latitude
    // ada langsung di sana, BUKAN `ConsumerApplication.Location`
    // (`waproto-0.7.0/src/whatsapp.proto:3663-3676` vs `:1518-1521`).
    let lokasi = wa::message::LocationMessage {
        degrees_latitude: Some(0.0),
        degrees_longitude: Some(0.0),
        name: Some(nama_bot(input.config)),
        address: Some(ALAMAT_BOT.to_string()),
        jpeg_thumbnail: Some(thumbnail(input, "foto", FOTO)),
        ..Default::default()
    };
    let inner = wa::Message {
        message_context_info: MessageField::some(wa::MessageContextInfo::default()),
        interactive_message: MessageField::some(wa::message::InteractiveMessage {
            header: MessageField::some(Header {
                has_media_attachment: Some(true),
                media: Some(HeaderMedia::LocationMessage(Box::new(lokasi))),
                ..Default::default()
            }),
            body: MessageField::some(Body {
                text: Some(input.text.to_string()),
            }),
            context_info: MessageField::some(ci),
            interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
                wa::message::interactive_message::NativeFlowMessage {
                    buttons: Vec::new(),
                    ..Default::default()
                },
            ))),
            ..Default::default()
        }),
        ..Default::default()
    };
    wa::Message {
        view_once_message: MessageField::some(wa::message::FutureProofMessage {
            message: MessageField::some(inner),
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// varian 8 — kartu signup
// ---------------------------------------------------------------------------

/// `serialize.js:1175-1223` — tombol `inapp_signup`, dan `quotedMessage` yang
/// isinya `m?.body` (teks MENTAH pemicu, bukan teks balasan).
fn kartu_signup(input: &VariantInput<'_>) -> wa::Message {
    let order = wa::message::OrderMessage {
        order_id: Some("8999999999999".into()),
        thumbnail: Some(thumbnail(input, "foto", FOTO)),
        item_count: Some(999),
        status: Some(OrderStatus::Inquiry),
        surface: Some(OrderSurface::Catalog),
        message: Some(input.target.body.clone()),
        order_title: Some("blablabla".into()),
        seller_jid: Some("0@s.whatsapp.net".into()),
        total_amount1000: Some(0),
        total_currency_code: Some("IDR".into()),
        ..Default::default()
    };
    let ci = wa::ContextInfo {
        mentioned_jid: mentions(input),
        participant: jid_atau_kosong(&input.target.sender),
        quoted_message: MessageField::some(wa::Message {
            order_message: MessageField::some(order),
            ..Default::default()
        }),
        remote_jid: jid_atau_kosong(&input.target.chat),
        forwarding_score: Some(SKOR_KARTU),
        is_forwarded: Some(true),
        ..Default::default()
    };
    let inner = wa::Message {
        interactive_message: MessageField::some(wa::message::InteractiveMessage {
            header: MessageField::some(Header {
                title: Some(nama_bot(input.config)),
                ..Default::default()
            }),
            body: MessageField::some(Body {
                text: Some(input.text.to_string()),
            }),
            context_info: MessageField::some(ci),
            interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
                wa::message::interactive_message::NativeFlowMessage {
                    buttons: vec![NativeFlowButton {
                        name: Some("inapp_signup".into()),
                        button_params_json: Some("{}".into()),
                    }],
                    ..Default::default()
                },
            ))),
            ..Default::default()
        }),
        ..Default::default()
    };
    wa::Message {
        view_once_message: MessageField::some(wa::message::FutureProofMessage {
            message: MessageField::some(inner),
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// varian 9 — orderMessage
// ---------------------------------------------------------------------------

/// `serialize.js:1224-1252` — `relayMessage` langsung, tanpa `viewOnceMessage`
/// dan tanpa node `biz`.
///
/// `participant` dan `quotedMessage` ditulis LANGSUNG di sini, terpisah dari
/// blok `contextInfo` di atas fungsi: `participant` = `m?.sender` (BUKAN
/// peserta kutipan), dan `quotedMessage` jatuh ke `m?.message` — pesan asli
/// apa adanya, bukan teksnya saja. Baru kalau `m.message` kosong dipakai
/// `{conversation: m?.text || ""}`.
fn order(input: &VariantInput<'_>) -> wa::Message {
    let mut ci = context_info(&mentions(input), None);
    ci.participant = jid_atau_kosong(&input.target.sender);
    ci.quoted_message = MessageField::some(match &input.target.trigger_message {
        Some(m) => (**m).clone(),
        None => wa::Message::text(&input.target.text),
    });

    wa::Message {
        order_message: MessageField::some(wa::message::OrderMessage {
            order_id: Some("WXX".into()),
            thumbnail: Some(thumbnail(input, "foto", FOTO)),
            item_count: Some(1),
            status: Some(OrderStatus::Inquiry),
            surface: Some(OrderSurface::Catalog),
            message: Some(input.text.to_string()),
            order_title: Some("CONTOL".into()),
            token: Some("SDsdafma".into()),
            total_amount1000: Some(0),
            total_currency_code: Some("IDR".into()),
            message_version: Some(1),
            context_info: MessageField::some(ci),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Tanda tangan lama yang dipakai `main.rs --kind order9`: varian 9 tanpa
/// pemicu, thumbnail dari pipeline pemanggil.
pub fn render_variant_9(
    text: &str,
    target: &ReplyTarget,
    thumbnail: Option<&[u8]>,
) -> Rendered {
    let mentions = mentions_untuk_target(target);
    let input = VariantInput {
        text,
        mentions: &mentions,
        target,
        config: &Runtime::default(),
        srt_enabled: thumbnail.is_some(),
        srt_image: thumbnail,
    };
    let msg = render(ReplyVariant::Order, &input).and_then(|r| match r {
        VariantRender::Wire(m) => Some(m),
        VariantRender::Media(_) => None,
    });
    Rendered::wire(
        msg.unwrap_or_else(|| wa::Message::text(text)),
        Vec::new(),
    )
}

/// `[m?.sender]` untuk pemanggil tanpa daftar mention eksplisit.
fn mentions_untuk_target(target: &ReplyTarget) -> Vec<String> {
    jid_atau_kosong(&target.sender)
        .map(|s| vec![s])
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// varian 10 — requestPaymentMessage
// ---------------------------------------------------------------------------

/// `serialize.js:1253-1276` — `noteMessage.extendedTextMessage`, dan kuote
/// yang sama seperti varian 9 (`m?.message`, lalu `{conversation}`).
fn pembayaran(input: &VariantInput<'_>) -> wa::Message {
    let mut ci = context_info(&mentions(input), None);
    ci.participant = jid_atau_kosong(&input.target.sender);
    ci.quoted_message = MessageField::some(match &input.target.trigger_message {
        Some(m) => (**m).clone(),
        None => wa::Message::text(&input.target.text),
    });
    let note = wa::message::ExtendedTextMessage {
        text: Some(input.text.to_string()),
        context_info: MessageField::some(ci),
        ..Default::default()
    };
    wa::Message {
        request_payment_message: MessageField::some(wa::message::RequestPaymentMessage {
            currency_code_iso4217: Some("IDR".into()),
            amount1000: Some(75_000_000),
            // `m?.sender || "0@s.whatsapp.net"`.
            request_from: Some(jid_atau_kosong(&input.target.sender).unwrap_or_else(|| "0@s.whatsapp.net".into())),
            note_message: MessageField::some(wa::Message {
                extended_text_message: MessageField::some(note),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// varian 11 — favicon preview
// ---------------------------------------------------------------------------

/// `serialize.js:1277-1343` — preview link dengan `faviconMMSMetadata` yang
/// diambil dari upload `thumbnail-link`, plus kutipan kontak palsu.
///
/// Byte favicon mengikuti `getCachedSrtImage()` sendiri (bukan `srtImage`
/// bersama) — `:1278-1288`.
fn favicon(input: &VariantInput<'_>) -> PendingMedia {
    let config = input.config;
    let bot = nama_bot(config);

    let mut ci = context_kutipan(input);
    ci.is_forwarded = Some(true);
    ci.forwarding_score = Some(SKOR_KARTU);
    ci.forwarded_newsletter_message_info =
        MessageField::some(newsletter_info(input, config.saluran.name.clone(), false));
    let kontak = wa::Message {
        contact_message: MessageField::some(wa::message::ContactMessage {
            display_name: Some(bot.clone()),
            vcard: Some(format!(
                "BEGIN:VCARD\nVERSION:3.0\nFN:{bot}\nTEL;type=CELL;type=VOICE;waid={nomor}:+{nomor}\nEND:VCARD",
                nomor = nomor(&input.target.sender)
            )),
            ..Default::default()
        }),
        ..Default::default()
    };
    ci.participant = Some("0@s.whatsapp.net".into());
    ci.quoted_message = MessageField::some(kontak);
    // Kutipan kontak palsu tidak punya `key.id` — `quoted.key` JS varian 11
    // (`serialize.js:1329-1333`) hanya `fromMe`/`participant`/`remoteJid`, dan
    // `undefined` hilang saat serialize. `context_kutipan` membawa stanzaId
    // pemicu nyata, jadi harus dikosongkan di sini: kalau tidak, quote palsu
    // menunjuk stanza pesan asli.
    ci.stanza_id = None;

    let ext = wa::message::ExtendedTextMessage {
        text: Some(format!("{}\n{}", config.info.website, input.text)),
        matched_text: Some(config.info.website.clone()),
        title: Some(bot),
        // `Hello ${m.pushName || "User"}`.
        description: Some(format!(
            "Hello {}",
            if input.target.push_name.is_empty() {
                "User"
            } else {
                input.target.push_name.as_str()
            }
        )),
        preview_type: Some(PreviewType::VIDEO),
        jpeg_thumbnail: Some(thumbnail(input, "foto", FOTO)),
        context_info: MessageField::some(ci),
        ..Default::default()
    };
    let bytes = ext.jpeg_thumbnail.clone().unwrap_or_default();
    PendingMedia {
        kind: MediaKind::Image,
        source: oktz_plugin_api::MediaSource::bytes(bytes.clone()),
        caption: String::new(),
        mentions: Vec::new(),
        quote: None,
        gif_playback: false,
        ptt: false,
        file_name: None,
        jpeg_thumbnail: None,
        file_length: None,
        preview: Some(crate::pipeline::PendingPreview {
            message: wa::Message {
                extended_text_message: MessageField::some(ext),
                ..Default::default()
            },
            bytes,
            favicon: true,
        }),
    }
}

/// `forwardedNewsletterMessageInfo` — varian 4 (`:1070-1074`) dan 11
/// (`:1321-1324`), dengan ISI yang beda: hanya varian 4 menulis
/// `serverMessageId` = `Math.floor(Math.random() * 1000000)`. Varian 11 cuma
/// `newsletterJid`/`newsletterName`; menambah `serverMessageId` di sana
/// berarti mengirim field yang tidak pernah ada di JS.
///
/// `nama` beda per varian: varian 4 memakai
/// `config.saluran?.name || config.bot?.name || "Bot"`, varian 11 memakai
/// `config.saluran?.name` apa adanya.
fn newsletter_info(
    input: &VariantInput<'_>,
    nama: String,
    server_message_id: bool,
) -> ForwardedNewsletterMessageInfo {
    ForwardedNewsletterMessageInfo {
        newsletter_jid: jid_atau_kosong(&input.config.saluran.id),
        newsletter_name: jid_atau_kosong(&nama),
        server_message_id: server_message_id.then(|| pseudo_random_index(1_000_000) as i32),
        ..Default::default()
    }
}

/// `config.saluran?.name || config.bot?.name || "Bot"` (`:1072`).
fn nama_saluran_atau_bot(config: &Runtime) -> String {
    if !config.saluran.name.is_empty() {
        config.saluran.name.clone()
    } else {
        nama_bot(config)
    }
}

// ---------------------------------------------------------------------------
// srt image
// ---------------------------------------------------------------------------

/// `getCachedSrtImage()` (`serialize.js:68-82`): satu file acak dari
/// `assets/image/shuffle`, di-cache 30 menit per file.
///
/// `read_dir` mengembalikan urutan filesystem, sedangkan `readdirSync` di Node
/// juga begitu — urutan keduanya tidak dijamin sama, tapi file dipilih acak
/// setelah itu, jadi tidak ada yang bisa dibandingkan byte-per-byte.
fn srt_dir(root: &Path) -> PathBuf {
    root.join("assets").join("image").join("shuffle")
}

/// Thumbnail srt: `srtEnabled && getCachedSrtImage() || getAssetBuffer("foto")`
/// (`serialize.js:976-980,1232`).
///
/// Dipakai pipeline untuk mengisi [`VariantInput::srt_image`]; nama asetnya
/// menunjuk `assets/image/*.jpeg` sesuai `config.assets`.
pub fn thumbnail_bytes(root: &Path, srt_enabled: bool) -> Option<Vec<u8>> {
    let dir = srt_dir(root);
    if srt_enabled
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| {
                        matches!(
                            e.to_ascii_lowercase().as_str(),
                            "webp" | "jpg" | "jpeg" | "png"
                        )
                    })
            })
            .collect();
        if !files.is_empty() {
            files.sort();
            let idx = pseudo_random_index(files.len());
            if let Ok(b) = std::fs::read(&files[idx]) {
                return Some(b);
            }
        }
    }
    std::fs::read(root.join("assets").join("image").join("foto.jpeg")).ok()
}

/// `Math.floor(Math.random() * n)` tanpa dependency RNG.
///
/// Jam sistem sudah cukup: yang perlu tercermin hanya "acak", dan ini bukan
/// sumber keacakan yang dipakai untuk keamanan.
fn pseudo_random_index(len: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0);
    ((nanos as u128 * len as u128) >> 32) as usize % len.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{MediaKind, QuoteSource, ReplyTarget};
    use oktz_plugin_api::registry::MediaSource;
    use serde_json::json;
    use whatsapp_rust::wacore::proto_helpers::MessageBuilderExt;

    /// Pemicu uji. `quote.participant` SENGAJA berbeda dari `sender` supaya
    /// test bisa mengunci varian 9/10 memakai `m?.sender`, bukan peserta
    /// kutipan.
    fn target() -> ReplyTarget {
        ReplyTarget {
            quote: Some(QuoteSource {
                stanza_id: "STANZA7".into(),
                participant: Some("6281@s.whatsapp.net".into()),
                message: Box::new(wa::Message::text("pesan asal")),
            }),
            sender: "6289@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            push_name: "Budi".into(),
            body: "body mentah pemicu".into(),
            text: "teks terparsing".into(),
            trigger_message: Some(Box::new(wa::Message::text("pesan asal"))),
            // Tanpa sequence: belum ada kiriman bot sebelumnya di balasan ini.
            previous_sent: None,
            bot_participant: None,
        }
    }

    /// Nilai `config.js` produksi tanpa menyentuh disk.
    fn config() -> Runtime {
        Runtime::from_config()
    }

    /// `getCachedSrtImage()` palsu: yang diuji pemakaian `srtImage`, bukan
    /// pemilihan file acak (itu sudah diuji lewat `thumbnail_bytes`).
    fn srt() -> &'static [u8] {
        b"srt-image"
    }

    fn input<'a>(text: &'a str, t: &'a ReplyTarget, cfg: &'a Runtime) -> VariantInput<'a> {
        VariantInput {
            text,
            // `mentions` diisi pemanggil; test memakai `[m.sender]` seperti
            // default JS (`serialize.js:987`).
            mentions: std::slice::from_ref(&t.sender),
            target: t,
            config: cfg,
            srt_enabled: true,
            srt_image: Some(srt()),
        }
    }

    fn wire(variant: ReplyVariant, i: &VariantInput<'_>) -> wa::Message {
        match render(variant, i) {
            Some(VariantRender::Wire(m)) => m,
            lain => panic!("varian {variant:?} harus jadi pesan wire, dapat {lain:?}"),
        }
    }

    fn media(variant: ReplyVariant, i: &VariantInput<'_>) -> PendingMedia {
        match render(variant, i) {
            Some(VariantRender::Media(p)) => p,
            lain => panic!("varian {variant:?} butuh upload dulu, dapat {lain:?}"),
        }
    }

    fn body_of(m: &wa::Message) -> Option<&str> {
        m.conversation
            .as_deref()
            .or_else(|| m.extended_text_message.as_option()?.text.as_deref())
    }

    fn ctx_of(m: &wa::Message) -> Option<&wa::ContextInfo> {
        m.extended_text_message.as_option()?.context_info.as_option()
    }

    #[test]
    fn variant_missing_or_zero_or_string_selects_basic() {
        // `serialize.js:995-1277` adalah rangkaian `===` angka: nilai non-angka
        // tidak pernah masuk cabang varian, dan `0`/`1` tidak punya builder.
        for nilai in [
            json!({}),
            json!({"replyVariant": 0}),
            json!({"replyVariant": 1}),
            json!({"replyVariant": "9"}),
            json!({"replyVariant": ""}),
            json!({"replyVariant": null}),
            json!({"replyVariant": 2.5}),
            json!({"replyVariant": 12}),
            json!({"replyVariant": -3}),
        ] {
            assert_eq!(variant_from(&nilai), ReplyVariant::Basic, "{nilai}");
        }
        assert!(!needs_custom_render(ReplyVariant::Basic));
        let (t, c) = (target(), config());
        assert!(render(ReplyVariant::Basic, &input("hai", &t, &c)).is_none());
    }

    #[test]
    fn variant_only_numbers_2_through_11_dispatch_custom() {
        let harapan = [
            (2, ReplyVariant::DocumentContact),
            (3, ReplyVariant::GifVideo),
            (4, ReplyVariant::LinkPreview),
            (5, ReplyVariant::FakeOrderQuote),
            (6, ReplyVariant::TriggerDocument),
            (7, ReplyVariant::LocationCard),
            (8, ReplyVariant::SignupCard),
            (9, ReplyVariant::Order),
            (10, ReplyVariant::Payment),
            (11, ReplyVariant::FaviconPreview),
        ];
        for (angka, varian) in harapan {
            let dipilih = variant_from(&json!({ "replyVariant": angka }));
            assert_eq!(dipilih, varian, "angka {angka}");
            assert!(needs_custom_render(dipilih), "varian {angka} butuh builder");
        }
        // `2.0 === 2` di JS — angka bulat bentuk float tetap varian 2.
        assert_eq!(
            variant_from(&json!({ "replyVariant": 2.0 })),
            ReplyVariant::DocumentContact
        );
    }

    #[test]
    fn variant2_document_contact_quote_is_exact_fake_contact() {
        let (t, c) = (target(), config());
        let p = media(ReplyVariant::DocumentContact, &input("hai", &t, &c));
        assert_eq!(p.kind, MediaKind::Document);
        assert_eq!(p.caption, "hai");
        assert_eq!(p.file_name.as_deref(), Some("Oktz Bot"));
        assert_eq!(p.file_length, Some(99_999_999_999_999));
        assert_eq!(p.source.mimetype(), Some("image/png"));
        assert_eq!(p.jpeg_thumbnail.as_deref(), Some(&b"srt-image"[..]));
        assert!(matches!(p.source, MediaSource::Bytes { .. }));
        assert_eq!(p.mentions, vec!["6289@s.whatsapp.net".to_string()]);
        assert!(p.preview.is_none(), "varian 2 tidak upload thumbnail-link");

        let quote = p.quote.as_ref().expect("varian 2 mengutip kontak palsu");
        let kontak = quote
            .message
            .contact_message
            .as_option()
            .expect("kutipan varian 2 = contactMessage");
        assert_eq!(kontak.display_name.as_deref(), Some("🪸 Oktz Bot"));
        let vcard = kontak.vcard.as_deref().expect("vcard wajib terkirim");
        assert!(vcard.starts_with("BEGIN:VCARD\nVERSION:3.0\n"), "{vcard}");
        assert!(
            vcard.contains("item1.TEL;waid=13135550002:+1 (313) 555-0002"),
            "{vcard}"
        );
        assert!(vcard.contains("FN:ttname"), "{vcard}");
        assert_eq!(quote.participant.as_deref(), Some("0@s.whatsapp.net"));
    }

    #[test]
    fn variant3_gif_video_carries_caption_and_trigger_quote() {
        let (t, c) = (target(), config());
        let p = media(ReplyVariant::GifVideo, &input("hai", &t, &c));
        assert_eq!(p.kind, MediaKind::Video);
        assert!(p.gif_playback, "`gifPlayback: true` wajib ikut");
        assert_eq!(p.caption, "hai");
        assert!(
            matches!(&p.source, MediaSource::Path { path, .. } if path.ends_with("mp4.mp4")),
            "{:?}",
            p.source
        );
        assert!(p.jpeg_thumbnail.is_none(), "varian 3 tidak pakai foto2");
        let quote = p.quote.as_ref().expect("varian 3 mengutip pemicu");
        assert_eq!(quote.stanza_id, "STANZA7");
        assert_eq!(quote.message.conversation.as_deref(), Some("pesan asal"));
    }

    #[test]
    fn variant4_link_preview_forwards_score_nine() {
        let (t, c) = (target(), config());
        let p = media(ReplyVariant::LinkPreview, &input("hai", &t, &c));
        let preview = p.preview.as_ref().expect("varian 4 upload thumbnail-link");
        assert!(!preview.favicon, "varian 4 mengisi preview link, bukan favicon");
        assert_eq!(preview.bytes, b"srt-image".to_vec());

        let ext = preview
            .message
            .extended_text_message
            .as_option()
            .expect("extendedTextMessage");
        assert_eq!(ext.text.as_deref(), Some("https://oktzz.xyz\n\nhai"));
        assert_eq!(ext.matched_text.as_deref(), Some("https://oktzz.xyz"));
        assert_eq!(ext.title.as_deref(), Some("Oktz Bot"));
        assert_eq!(
            ext.description.as_deref(),
            Some("Pengembang: Zann | Versi: 3.3.1")
        );
        assert_eq!(ext.preview_type, Some(PreviewType::NONE), "`previewType: 0`");
        assert_eq!(ext.jpeg_thumbnail.as_deref(), Some(&b"srt-image"[..]));

        let ci = ext.context_info.as_option().expect("contextInfo");
        assert_eq!(ci.forwarding_score, Some(9));
        assert_eq!(ci.is_forwarded, Some(true));
        assert_eq!(ci.mentioned_jid, vec!["6289@s.whatsapp.net".to_string()]);
        let info = ci
            .forwarded_newsletter_message_info
            .as_option()
            .expect("forwardedNewsletterMessageInfo");
        assert_eq!(
            info.newsletter_jid.as_deref(),
            Some("120363411617217441@newsletter")
        );
        assert_eq!(info.newsletter_name.as_deref(), Some("Join saluran resmi Oktz"));
        assert!(info.server_message_id.is_some(), "`Math.floor(Math.random()*1e6)`");
        // `quoted: m` — kutipan pemicu seperti yang dibaca pipeline.
        assert_eq!(ci.participant.as_deref(), Some("6281@s.whatsapp.net"));
        assert_eq!(ci.stanza_id.as_deref(), Some("STANZA7"));
        assert!(ci.quoted_message.as_option().is_some());
    }

    #[test]
    fn variant5_fake_order_uses_static_wire_values() {
        let (t, c) = (target(), config());
        let msg = wire(ReplyVariant::FakeOrderQuote, &input("hai", &t, &c));
        assert_eq!(body_of(&msg), Some("hai"), "varian 5 tetap kirim teks");
        let ci = ctx_of(&msg).expect("contextInfo");
        let order = ci
            .quoted_message
            .as_option()
            .expect("kutipan varian 5 = orderMessage palsu")
            .order_message
            .as_option()
            .expect("orderMessage");
        assert_eq!(order.order_id.as_deref(), Some("123456"));
        assert_eq!(order.item_count, Some(999));
        assert_eq!(order.status, Some(OrderStatus::INQUIRY));
        assert_eq!(order.surface, Some(OrderSurface::CATALOG));
        assert_eq!(order.message.as_deref(), Some("Oktz Bot"));
        assert_eq!(order.order_title.as_deref(), Some("System Notification"));
        assert_eq!(order.seller_jid.as_deref(), Some("0@s.whatsapp.net"));
        assert_eq!(order.token.as_deref(), Some("ARU1+"));
        // JS menulis string "1000000"; proto `int64`.
        assert_eq!(order.total_amount1000, Some(1_000_000));
        assert_eq!(order.total_currency_code.as_deref(), Some("IDR"));
        assert_eq!(order.thumbnail.as_deref(), Some(&b"srt-image"[..]));
        assert_eq!(ci.participant.as_deref(), Some("0@s.whatsapp.net"));
        assert!(ci.stanza_id.is_none(), "order palsu tidak punya stanzaId");
    }

    #[test]
    fn variant6_document_uses_trigger_quote() {
        let (t, c) = (target(), config());
        let p = media(ReplyVariant::TriggerDocument, &input("hai", &t, &c));
        // Isi dokumen identik dengan varian 2 (`serialize.js:1114-1136`)...
        assert_eq!(p.kind, MediaKind::Document);
        assert_eq!(p.caption, "hai");
        assert_eq!(p.file_name.as_deref(), Some("Oktz Bot"));
        assert_eq!(p.file_length, Some(99_999_999_999_999));
        assert_eq!(p.source.mimetype(), Some("image/png"));
        assert_eq!(p.jpeg_thumbnail.as_deref(), Some(&b"srt-image"[..]));
        // ...yang beda hanya kutipannya: pemicu, bukan kontak palsu.
        let quote = p.quote.as_ref().expect("varian 6 mengutip pemicu");
        assert_eq!(quote.stanza_id, "STANZA7");
        assert_eq!(quote.message.conversation.as_deref(), Some("pesan asal"));
        assert!(quote.message.contact_message.as_option().is_none());
    }

    #[test]
    fn variant7_location_card_has_empty_outer_context() {
        let (t, c) = (target(), config());
        let msg = wire(ReplyVariant::LocationCard, &input("hai", &t, &c));
        let inner = msg
            .view_once_message
            .as_option()
            .expect("viewOnceMessage")
            .message
            .as_option()
            .expect("viewOnceMessage.message");
        // `messageContextInfo: {}` — ADA tapi KOSONG: tanpa deviceListMetadata
        // dan tanpa messageSecret seperti kartu native-flow.
        let outer = inner
            .message_context_info
            .as_option()
            .expect("messageContextInfo wajib ada");
        assert!(outer.device_list_metadata.as_option().is_none(), "{outer:?}");
        assert!(outer.device_list_metadata_version.is_none());
        assert!(outer.message_secret.is_none());
        assert!(msg.message_context_info.as_option().is_none());

        let im = inner
            .interactive_message
            .as_option()
            .expect("interactiveMessage");
        assert_eq!(
            im.body.as_option().and_then(|b| b.text.as_deref()),
            Some("hai")
        );
        let header = im.header.as_option().expect("header");
        assert_eq!(header.has_media_attachment, Some(true));
        assert!(header.title.is_none(), "varian 7 tidak menulis header.title");
        match &header.media {
            Some(HeaderMedia::LocationMessage(loc)) => {
                // `Message.LocationMessage` — latitude/name langsung di sana.
                assert_eq!(loc.degrees_latitude, Some(0.0));
                assert_eq!(loc.degrees_longitude, Some(0.0));
                assert_eq!(loc.name.as_deref(), Some("Oktz Bot"));
                assert_eq!(loc.address.as_deref(), Some("Bot Wa Multi Device"));
                // `jpegThumbnail: sharp(thumbnailBuf).resize(300,300).toBuffer()`
                // (`serialize.js:1152`) — byte aset apa adanya.
                assert_eq!(loc.jpeg_thumbnail.as_deref(), Some(&b"srt-image"[..]));
            }
            lain => panic!("header media varian 7 harus locationMessage, dapat {lain:?}"),
        }
        let ci = im.context_info.as_option().expect("contextInfo");
        assert_eq!(ci.forwarding_score, Some(9));
        assert_eq!(ci.is_forwarded, Some(true));
        assert_eq!(ci.mentioned_jid, vec!["6289@s.whatsapp.net".to_string()]);
        match im.interactive_message.as_ref().expect("oneof native flow") {
            ImOneof::NativeFlowMessage(nf) => assert!(nf.buttons.is_empty()),
            lain => panic!("varian 7 harus NativeFlowMessage, dapat {lain:?}"),
        }
    }

    #[test]
    fn variant8_signup_card_uses_raw_body_and_sender_participant() {
        let (t, c) = (target(), config());
        let msg = wire(ReplyVariant::SignupCard, &input("balasan", &t, &c));
        let inner = msg
            .view_once_message
            .as_option()
            .expect("viewOnceMessage")
            .message
            .as_option()
            .expect("viewOnceMessage.message");
        assert!(
            inner.message_context_info.as_option().is_none(),
            "varian 8 tidak menulis messageContextInfo"
        );
        let im = inner
            .interactive_message
            .as_option()
            .expect("interactiveMessage");
        assert_eq!(
            im.header.as_option().and_then(|h| h.title.as_deref()),
            Some("Oktz Bot")
        );
        assert_eq!(
            im.body.as_option().and_then(|b| b.text.as_deref()),
            Some("balasan")
        );
        match im.interactive_message.as_ref().expect("oneof native flow") {
            ImOneof::NativeFlowMessage(nf) => {
                assert_eq!(nf.buttons.len(), 1);
                assert_eq!(nf.buttons[0].name.as_deref(), Some("inapp_signup"));
                assert_eq!(nf.buttons[0].button_params_json.as_deref(), Some("{}"));
            }
            lain => panic!("varian 8 harus NativeFlowMessage, dapat {lain:?}"),
        }
        let ci = im.context_info.as_option().expect("contextInfo");
        assert_eq!(ci.participant.as_deref(), Some("6289@s.whatsapp.net"));
        assert_eq!(ci.remote_jid.as_deref(), Some("120363@g.us"), "`remoteJid: m.chat`");
        assert_eq!(ci.forwarding_score, Some(999));
        assert_eq!(ci.is_forwarded, Some(true));
        assert!(ci.stanza_id.is_none(), "varian 8 tidak menulis stanzaId");
        let order = ci
            .quoted_message
            .as_option()
            .expect("quotedMessage")
            .order_message
            .as_option()
            .expect("orderMessage");
        assert_eq!(order.order_id.as_deref(), Some("8999999999999"));
        assert_eq!(order.order_title.as_deref(), Some("blablabla"));
        assert_eq!(
            order.message.as_deref(),
            Some("body mentah pemicu"),
            "`message: m?.body` — teks MENTAH, bukan teks balasan"
        );
        assert_eq!(order.item_count, Some(999));
        assert_eq!(order.seller_jid.as_deref(), Some("0@s.whatsapp.net"));
        assert_eq!(order.total_amount1000, Some(0));
        assert_eq!(order.total_currency_code.as_deref(), Some("IDR"));
        assert_eq!(order.thumbnail.as_deref(), Some(&b"srt-image"[..]));
    }

    #[test]
    fn variant9_order_participant_is_trigger_sender() {
        let (t, c) = (target(), config());
        let msg = wire(ReplyVariant::Order, &input("hai", &t, &c));
        assert!(
            msg.view_once_message.as_option().is_none(),
            "varian 9 relay langsung, tanpa viewOnceMessage"
        );
        let order = msg.order_message.as_option().expect("orderMessage");
        assert_eq!(order.order_id.as_deref(), Some("WXX"));
        assert_eq!(order.item_count, Some(1));
        assert_eq!(order.message.as_deref(), Some("hai"));
        assert_eq!(order.order_title.as_deref(), Some("CONTOL"));
        assert_eq!(order.token.as_deref(), Some("SDsdafma"));
        assert_eq!(order.total_amount1000, Some(0));
        assert_eq!(order.total_currency_code.as_deref(), Some("IDR"));
        assert_eq!(order.message_version, Some(1));
        assert_eq!(order.thumbnail.as_deref(), Some(&b"srt-image"[..]));
        let ci = order.context_info.as_option().expect("contextInfo");
        assert_eq!(
            ci.participant.as_deref(),
            Some("6289@s.whatsapp.net"),
            "`participant: m?.sender`, bukan peserta kutipan (6281)"
        );
        assert_eq!(ci.mentioned_jid, vec!["6289@s.whatsapp.net".to_string()]);
        assert_eq!(
            ci.quoted_message.as_option().expect("quotedMessage"),
            &wa::Message::text("pesan asal"),
            "`quotedMessage: m?.message` — pesan asli apa adanya"
        );
        assert!(ci.stanza_id.is_none(), "varian 9 tidak menulis stanzaId");

        // Tanpa pemicu: cadangan `{conversation: m?.text || ""}`.
        let tanpa = ReplyTarget {
            trigger_message: None,
            ..target()
        };
        let msg = wire(ReplyVariant::Order, &input("hai", &tanpa, &c));
        let ci = msg
            .order_message
            .as_option()
            .unwrap()
            .context_info
            .as_option()
            .unwrap();
        assert_eq!(
            ci.quoted_message.as_option().unwrap().conversation.as_deref(),
            Some("teks terparsing")
        );
    }

    #[test]
    fn variant10_payment_note_uses_trigger_sender() {
        let (t, c) = (target(), config());
        let msg = wire(ReplyVariant::Payment, &input("hai", &t, &c));
        let pay = msg
            .request_payment_message
            .as_option()
            .expect("requestPaymentMessage");
        assert_eq!(pay.currency_code_iso4217.as_deref(), Some("IDR"));
        assert_eq!(pay.amount1000, Some(75_000_000));
        assert_eq!(pay.request_from.as_deref(), Some("6289@s.whatsapp.net"));
        let ext = pay
            .note_message
            .as_option()
            .expect("noteMessage")
            .extended_text_message
            .as_option()
            .expect("extendedTextMessage");
        assert_eq!(ext.text.as_deref(), Some("hai"));
        let ci = ext.context_info.as_option().expect("contextInfo");
        assert_eq!(ci.participant.as_deref(), Some("6289@s.whatsapp.net"));
        assert_eq!(ci.mentioned_jid, vec!["6289@s.whatsapp.net".to_string()]);
        assert_eq!(
            ci.quoted_message.as_option().unwrap().conversation.as_deref(),
            Some("pesan asal")
        );

        // `m?.sender || "0@s.whatsapp.net"` dan `{conversation: ""}`.
        let kosong = ReplyTarget {
            sender: String::new(),
            trigger_message: None,
            text: String::new(),
            quote: None,
            ..target()
        };
        let msg = wire(ReplyVariant::Payment, &input("hai", &kosong, &c));
        let pay = msg
            .request_payment_message
            .as_option()
            .expect("requestPaymentMessage");
        assert_eq!(pay.request_from.as_deref(), Some("0@s.whatsapp.net"));
        let ci = pay
            .note_message
            .as_option()
            .unwrap()
            .extended_text_message
            .as_option()
            .unwrap()
            .context_info
            .as_option()
            .unwrap();
        assert!(ci.participant.is_none(), "sender kosong = tanpa participant");
        assert_eq!(
            ci.quoted_message.as_option().unwrap().conversation.as_deref(),
            Some("")
        );
    }

    #[test]
    fn variant11_favicon_metadata_and_push_name_description() {
        let (t, c) = (target(), config());
        let p = media(ReplyVariant::FaviconPreview, &input("hai", &t, &c));
        let preview = p.preview.as_ref().expect("varian 11 upload favicon dulu");
        assert!(preview.favicon, "varian 11 mengisi faviconMMSMetadata");
        assert_eq!(preview.bytes, b"srt-image".to_vec());

        let ext = preview
            .message
            .extended_text_message
            .as_option()
            .expect("extendedTextMessage");
        assert_eq!(ext.text.as_deref(), Some("https://oktzz.xyz\nhai"));
        assert_eq!(ext.matched_text.as_deref(), Some("https://oktzz.xyz"));
        assert_eq!(ext.title.as_deref(), Some("Oktz Bot"));
        assert_eq!(ext.description.as_deref(), Some("Hello Budi"));
        assert_eq!(ext.preview_type, Some(PreviewType::VIDEO), "`previewType: 1`");
        assert_eq!(ext.jpeg_thumbnail.as_deref(), Some(&b"srt-image"[..]));
        assert!(
            ext.favicon_mms_metadata.as_option().is_none(),
            "favicon baru ada SETELAH upload"
        );
        let ci = ext.context_info.as_option().expect("contextInfo");
        assert_eq!(ci.forwarding_score, Some(999));
        assert_eq!(ci.is_forwarded, Some(true));
        let info = ci
            .forwarded_newsletter_message_info
            .as_option()
            .expect("forwardedNewsletterMessageInfo");
        assert_eq!(
            info.newsletter_jid.as_deref(),
            Some("120363411617217441@newsletter")
        );
        assert_eq!(info.newsletter_name.as_deref(), Some("Join saluran resmi Oktz"));
        assert!(
            info.server_message_id.is_none(),
            "varian 11 cuma newsletterJid/newsletterName (`serialize.js:1321-1324`)"
        );
        assert!(
            ci.stanza_id.is_none(),
            "kutipan kontak palsu varian 11 tidak punya key.id (`serialize.js:1329-1333`)"
        );
        assert_eq!(ci.participant.as_deref(), Some("0@s.whatsapp.net"));
        let kontak = ci
            .quoted_message
            .as_option()
            .expect("quotedMessage")
            .contact_message
            .as_option()
            .expect("contactMessage");
        assert_eq!(kontak.display_name.as_deref(), Some("Oktz Bot"));
        let vcard = kontak.vcard.as_deref().expect("vcard");
        assert!(vcard.contains("FN:Oktz Bot"), "{vcard}");
        assert!(vcard.contains("waid=6289:+6289"), "{vcard}");

        // `m.pushName` kosong → "User" (`m.pushName || "User"`).
        let tanpa_nama = ReplyTarget {
            push_name: String::new(),
            ..target()
        };
        let p = media(ReplyVariant::FaviconPreview, &input("hai", &tanpa_nama, &c));
        let ext = p
            .preview
            .as_ref()
            .unwrap()
            .message
            .extended_text_message
            .as_option()
            .unwrap();
        assert_eq!(ext.description.as_deref(), Some("Hello User"));
    }

    #[test]
    fn render_variant_nine_still_serves_the_old_order9_caller() {
        // `main.rs --kind order9` memakai tanda tangan ini; varian 9 juga
        // jalur yang aktif di deployment.
        let t = ReplyTarget::default();
        let r = render_variant_9("hai", &t, Some(b"foto"));
        let order = r.message.order_message.as_option().expect("orderMessage");
        assert_eq!(order.order_id.as_deref(), Some("WXX"));
        assert_eq!(order.message.as_deref(), Some("hai"));
        assert_eq!(order.thumbnail.as_deref(), Some(&b"foto"[..]));
        assert!(r.extra_nodes.is_empty(), "relayMessage tanpa node biz");
    }

    #[test]
    fn thumbnail_falls_back_when_nothing_is_on_disk() {
        // Direktori kerja yang tidak punya assets/ -> `None`, bukan panic.
        assert_eq!(thumbnail_bytes(Path::new("/tmp/tidak-ada-oktz"), true), None);
    }

    #[test]
    fn pseudo_random_index_stays_in_range() {
        for len in 1..32usize {
            for _ in 0..8 {
                assert!(pseudo_random_index(len) < len);
            }
        }
    }
}