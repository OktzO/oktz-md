//! `replyVariant` — bentuk balasan yang dilihat user, dibaca dari
//! `db.setting("replyVariant")`.
//!
//! Setting produksi aktif = **9**, jadi itu jalur yang diimplementasikan di
//! sini. Nilai lain butuh bentuk wire yang berbeda (video, dokumen, preview,
//! `sendPreview`), dan mengarang versi-versi itu lebih berbahaya daripada belum
//! ada: user akan melihat bubble yang tidak sama dengan produksi. Ambang
//! seriousness: cara upgrade ada di [`render_reply_variant`].

use std::path::{Path, PathBuf};

use whatsapp_rust::prelude::wa;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::message::order_message::{OrderStatus, OrderSurface};

use crate::pipeline::{ReplyTarget, Rendered, context_info};

/// Varian yang benar-benar dirender. Anything else jatuh ke teks biasa dengan
/// catatan, bukan diam.
pub const VARIANT_AKTIF: i64 = 9;

/// `serialize.js:1231-1259` — `orderMessage` lewat `sock.relayMessage`.
///
/// Nilai statis di dalamnya (`"WXX"`, `"CONTOL"`, `"SDsdafma"`, `itemCount: 1`,
/// `status: INQUIRY`, `surface: CATALOG`, `totalAmount1000: "0"`,
/// `totalCurrencyCode: "IDR"`, `messageVersion: 1`) disalin apa adanya: itu isi
/// yang user lihat, dan mengeditnya berarti menampilkan produk yang berbeda.
///
/// `contextInfo` bedanya penting dan sering terbalik: JS menulis `participant`
/// dan `quotedMessage` LANGSUNG di sini, terpisah dari blok `contextInfo` di
/// atas fungsi. `quotedMessage` jatuh ke `m.message` — pesan asli apa adanya,
/// bukan teksnya saja; baru kalau `m.message` kosong dipakai `{conversation}`.
///
/// `ponytail:` wacore 0.7 tidak punya `relay_message`, jadi `orderMessage`
/// dikirim lewat jalur `Message` biasa. Bubble yang tampil sama (isi, thumbnail,
/// context); yang berbeda adalah amplop protokolnya. Naikkan ke `relay_message`
/// begitu wacore/album menyediakan API-nya.
pub fn render_variant_9(text: &str, target: &ReplyTarget, thumbnail: Option<&[u8]>) -> Rendered {
    let mentions = if target.sender.is_empty() {
        Vec::new()
    } else {
        vec![target.sender.clone()]
    };
    let mut ci = context_info(&mentions, target.quote.as_ref());
    ci.participant = target.quote.as_ref().and_then(|q| q.participant.clone());

    let order = wa::message::OrderMessage {
        order_id: Some("WXX".into()),
        thumbnail: Some(thumbnail.unwrap_or_default().to_vec()),
        item_count: Some(1),
        status: Some(OrderStatus::Inquiry),
        surface: Some(OrderSurface::Catalog),
        message: Some(text.to_string()),
        order_title: Some("CONTOL".into()),
        token: Some("SDsdafma".into()),
        total_amount1000: Some(0),
        total_currency_code: Some("IDR".into()),
        message_version: Some(1),
        context_info: MessageField::some(ci),
        ..Default::default()
    };

    let msg = wa::Message {
        order_message: MessageField::some(order),
        ..Default::default()
    };
    Rendered::wire(msg, Vec::new())
}

/// `db.setting("replyVariant") || 1` (`serialize.js:983-991`).
///
/// `None` = tidak ada setting, `Some(0)`/`Some("")` = ada tapi falsy. Keduanya
/// berakhir di 1, tapi dibedakan supaya test bisa mengunci bahwa setting nol
/// tidak membuat renderer panik.
pub fn variant_from(settings: Option<&serde_json::Value>) -> i64 {
    let v = settings.and_then(|s| s.get("replyVariant"));
    match v {
        None => 1,
        Some(x) => match x {
            serde_json::Value::Number(n) => n.as_i64().unwrap_or(1),
            serde_json::Value::String(s) => s.trim().parse::<i64>().unwrap_or(1),
            _ => 1,
        },
    }
}

/// `true` kalau varian ini butuh bentuk wire sendiri. Varian lain yang belum
/// diimplementasikan sengaja TIDAK ikut: balasannya tetap teks biasa supaya
/// user dapat jawaban, bukan pesan hilang.
pub fn needs_custom_render(variant: i64) -> bool {
    variant == VARIANT_AKTIF
}

/// `getCachedSrtImage()` (`serialize.js:68-82`): satu file acak dari
/// `assets/image/shuffle`, di-cache 30 menit per file.
///
/// `read_dir` mengembalikan urutan filesystem, sedangkan `readdirSync` di Node
/// juga begitu — urutan keduanya tidak dijamin sama, tapi file dipilih acak
/// setelah itu, jadi tidak ada yang bisa dibandingkan byte-per-byte.
fn srt_dir(root: &Path) -> PathBuf {
    root.join("assets").join("image").join("shuffle")
}

/// Thumbnail varian 9: `srtEnabled && getCachedSrtImage() || getAssetBuffer("foto")`
/// (`serialize.js:976-980,1232`).
///
/// Nama aset ikut `config.assets`, yang menunjuk `assets/image/*.jpeg`.
pub fn thumbnail_bytes(root: &Path, srt_enabled: bool) -> Option<Vec<u8>> {
    let dir = srt_dir(root);
    if srt_enabled && let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "webp" | "jpg" | "jpeg" | "png"))
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
    use crate::pipeline::QuoteSource;
    use whatsapp_rust::wacore::proto_helpers::MessageBuilderExt;
    use serde_json::json;

    fn target() -> ReplyTarget {
        ReplyTarget {
            quote: Some(QuoteSource {
                stanza_id: "STANZA7".into(),
                participant: Some("6281@s.whatsapp.net".into()),
                message: Box::new(wa::Message::text("pesan asal")),
            }),
            sender: "6289@s.whatsapp.net".into(),
        }
    }

    fn order_of(r: &Rendered) -> &wa::message::OrderMessage {
        r.message
            .order_message
            .as_option()
            .expect("orderMessage")
    }

    #[test]
    fn variant_defaults_to_one_when_the_setting_is_missing() {
        assert_eq!(variant_from(None), 1);
        assert_eq!(variant_from(Some(&json!({}))), 1);
        // `db.setting(...) || 1` di JS: nilai falsy jatuh ke 1.
        assert_eq!(variant_from(Some(&json!({ "replyVariant": 0 }))), 0);
        assert_eq!(variant_from(Some(&json!({ "replyVariant": "" }))), 1);
        assert_eq!(variant_from(Some(&json!({ "replyVariant": 9 }))), 9);
        assert_eq!(variant_from(Some(&json!({ "replyVariant": "9" }))), 9);
    }

    #[test]
    fn only_variant_nine_needs_a_custom_render() {
        assert!(needs_custom_render(9));
        for v in [1, 2, 3, 8, 10, 11, 0] {
            assert!(!needs_custom_render(v), "varian {v} belum diimplementasikan");
        }
    }

    #[test]
    fn variant_nine_copies_every_static_field_from_serialize_js() {
        let r = render_variant_9("hai", &target(), Some(b"foto"));
        let o = order_of(&r);
        assert_eq!(o.order_id.as_deref(), Some("WXX"));
        assert_eq!(o.order_title.as_deref(), Some("CONTOL"));
        assert_eq!(o.token.as_deref(), Some("SDsdafma"));
        assert_eq!(o.message.as_deref(), Some("hai"));
        assert_eq!(o.item_count, Some(1));
        assert_eq!(o.status, Some(OrderStatus::INQUIRY));
        assert_eq!(o.surface, Some(OrderSurface::CATALOG));
        assert_eq!(o.total_amount1000, Some(0));
        assert_eq!(o.total_currency_code.as_deref(), Some("IDR"));
        assert_eq!(o.message_version, Some(1));
        assert!(r.extra_nodes.is_empty(), "orderMessage bukan biz node");
    }

    #[test]
    fn thumbnail_goes_through_untouched() {
        // Pemilihan srt-vs-foto ada di `thumbnail_bytes`, bukan di sini, supaya
        // satu tempat saja yang tahu urutan `srtImage || getAssetBuffer("foto")`.
        let r = render_variant_9("hai", &target(), Some(b"srt"));
        assert_eq!(order_of(&r).thumbnail.as_deref(), Some(&b"srt"[..]));
        let r = render_variant_9("hai", &target(), Some(b"foto"));
        assert_eq!(order_of(&r).thumbnail.as_deref(), Some(&b"foto"[..]));
    }

    #[test]
    fn a_missing_thumbnail_sends_empty_bytes_not_none() {
        // JS selalu mengirim Buffer; tidak ada thumbnail berarti byte kosong,
        // bukan field yang dihilangkan.
        let r = render_variant_9("hai", &target(), None);
        assert_eq!(order_of(&r).thumbnail.as_deref(), Some(&[][..]));
    }

    #[test]
    fn context_info_carries_mention_participant_and_the_quoted_message() {
        let r = render_variant_9("hai", &target(), Some(b"f"));
        let ci = order_of(&r).context_info
            .as_option()
            .expect("contextInfo");
        assert_eq!(ci.mentioned_jid, vec!["6289@s.whatsapp.net".to_string()]);
        assert_eq!(ci.participant.as_deref(), Some("6281@s.whatsapp.net"));
        assert!(ci.quoted_message.as_option().is_some());
    }

    #[test]
    fn without_a_quote_the_thumbnail_is_still_sent_and_context_is_bare() {
        let t = ReplyTarget::default();
        let r = render_variant_9("hai", &t, Some(b"foto"));
        let ci = order_of(&r).context_info
            .as_option()
            .expect("contextInfo");
        assert!(ci.quoted_message.as_option().is_none());
        assert!(ci.mentioned_jid.is_empty());
        assert_eq!(order_of(&r).thumbnail.as_deref(), Some(&b"foto"[..]));
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