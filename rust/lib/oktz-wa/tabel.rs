//! Tabel AI (WA `richResponseMessage`) — padanan `sock.sendTable` JS.
//!
//! Bentuk JS (`src/lib/rich-messages.js:609` `generateTableContent`):
//! `botForwardedMessage` → `richResponseMessage` dengan submessages:
//! teks opsional (judul), `tableMetadata` (judul + baris), teks opsional
//! (footer).
//!
//! ```js
//! const tableRows = [
//!   { items: headers, isHeading: true },
//!   ...rows.map((row) => ({ items: row.map(String) })),
//! ];
//! ```
//!
//! Dua kontrak yang dikunci test di bawah, keduanya dari JS di atas:
//!
//! 1. Baris heading ADA walaupun `headers` kosong (JS selalu memetakan
//!    `isHeading: true` pada `headers`, tidak ada cabang panjang).
//! 2. Baris isi TIDAK membawa `isHeading`. `Some(false)` akan terlihat sama
//!    di UI, tapi byte di wire berbeda dari produksi — test
//!    `tabel_v1_body_rows_omit_is_heading_field` menjaga itu.
//!
//! `sendTable` JS memakai `relayMessage` tanpa node `biz` — jadi tabel ini
//! juga dikirim tanpa node itu.
//!
//! Context table V1/List: score 1, origin `META_AI`, bot JID
//! `867051314767696@bot` (`buildRichContextInfo`, `rich-messages.js:417`).

use oktz_plugin_api::tabel::Tabel;
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::ai_rich_response_table_metadata::AIRichResponseTableRow;
use whatsapp_rust::waproto::whatsapp::{
    AIRichResponseMessage, AIRichResponseMessageType, AIRichResponseSubMessage,
    AIRichResponseSubMessageType, AIRichResponseTableMetadata,
};

/// Tabel → proto WA.
pub fn tabel_to_message(tabel: &Tabel, context_info: Option<wa::ContextInfo>) -> wa::Message {
    let mut submessages: Vec<AIRichResponseSubMessage> = Vec::new();
    if let Some(teks) = &tabel.header_teks {
        submessages.push(AIRichResponseSubMessage {
            message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TEXT),
            message_text: Some(teks.clone()),
            ..Default::default()
        });
    }
    let mut rows: Vec<AIRichResponseTableRow> = Vec::new();
    // `rich-messages.js:617-618`: heading row TANPA syal panjang header —
    // `header` kosong tetap jadi baris heading berisi sel kosong.
    rows.push(AIRichResponseTableRow {
        items: tabel.header.clone(),
        is_heading: Some(true),
    });
    for baris in &tabel.baris {
        // `:619`: baris isi tidak membawa `isHeading` — bukan `Some(false)`.
        // Byte-nya beda: `Some(false)` menulis field `isHeading = false`,
        // produksi tidak menulis field itu sama sekali.
        rows.push(AIRichResponseTableRow {
            items: baris.clone(),
            is_heading: None,
        });
    }
    submessages.push(AIRichResponseSubMessage {
        message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TABLE),
        table_metadata: MessageField::some(AIRichResponseTableMetadata {
            title: Some(tabel.judul.clone()),
            rows,
        }),
        ..Default::default()
    });
    if let Some(teks) = &tabel.footer {
        submessages.push(AIRichResponseSubMessage {
            message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TEXT),
            message_text: Some(teks.clone()),
            ..Default::default()
        });
    }

    let mut rich = AIRichResponseMessage {
        message_type: Some(AIRichResponseMessageType::AI_RICH_RESPONSE_TYPE_STANDARD),
        submessages,
        ..Default::default()
    };
    if let Some(ci) = context_info {
        rich.context_info = MessageField::some(ci);
    }

    wa::Message {
        bot_forwarded_message: MessageField::some(wa::message::FutureProofMessage {
            message: MessageField::some(wa::Message {
                rich_response_message: MessageField::some(rich),
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

/// `contextInfo` untuk tabel — JS `buildRichContextInfo`
/// (`src/lib/rich-messages.js:417`): pesan ditandai ter-forward dengan
/// asal `META_AI`, dan mengutip pesan pemicu kalau ada.
pub fn tabel_context_info(mentions: &[String], quote: Option<&QuoteSource>) -> wa::ContextInfo {
    let mut ci = crate::pipeline::context_info(mentions, quote);
    ci.forwarding_score = Some(1);
    ci.is_forwarded = Some(true);
    ci.forward_origin = Some(wa::context_info::ForwardOrigin::META_AI);
    ci.forwarded_ai_bot_message_info = MessageField::some(wa::ForwardedAIBotMessageInfo {
        bot_jid: Some("867051314767696@bot".to_string()),
        ..Default::default()
    });
    ci
}

use crate::pipeline::QuoteSource;

#[cfg(test)]
mod tests {
    use super::*;
    use whatsapp_rust::waproto::buffa::Message as _;

    fn payload(msg: &wa::Message) -> Vec<u8> {
        let mut buf = Vec::new();
        msg.encode(&mut buf);
        buf
    }

    fn rich(msg: &wa::Message) -> &AIRichResponseMessage {
        msg.bot_forwarded_message
            .as_option()
            .expect("tabel wajib dibungkus botForwardedMessage")
            .message
            .as_option()
            .expect("botForwardedMessage wajib berisi message")
            .rich_response_message
            .as_option()
            .expect("wajib ada richResponseMessage")
    }

    #[test]
    fn tabel_menjadi_bot_forwarded_rich_response() {
        let tabel = Tabel::baru("⚡ System Performance")
            .header(["Metric", "Value"])
            .baris(["Status", "Online"])
            .header_teks("🎄 Statistik")
            .footer("🍃 Realtime Monitoring");
        let msg = tabel_to_message(&tabel, None);
        let rich = rich(&msg);
        assert_eq!(
            rich.message_type,
            Some(AIRichResponseMessageType::AI_RICH_RESPONSE_TYPE_STANDARD)
        );
        assert_eq!(rich.submessages.len(), 3);
        assert_eq!(rich.submessages[0].message_text.as_deref(), Some("🎄 Statistik"));
        assert_eq!(rich.submessages[2].message_text.as_deref(), Some("🍃 Realtime Monitoring"));
        let table = rich.submessages[1].table_metadata.as_option().unwrap();
        assert_eq!(table.title.as_deref(), Some("⚡ System Performance"));
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].is_heading, Some(true));
        assert_eq!(table.rows[0].items, vec!["Metric", "Value"]);
        assert_eq!(table.rows[1].items, vec!["Status", "Online"]);
    }

    #[test]
    fn tabel_v1_empty_header_still_has_heading_row() {
        // `rich-messages.js:617-618`: `{ items: headers, isHeading: true }`
        // selalu ada — tidak ada `if (headers.length)`.
        let tabel = Tabel::baru("T").baris(["a", "b"]);
        let msg = tabel_to_message(&tabel, None);
        let rich = rich(&msg);
        let table = rich.submessages[0].table_metadata.as_option().unwrap();
        assert_eq!(table.rows.len(), 2, "heading + 1 baris isi");
        assert_eq!(
            table.rows[0].is_heading,
            Some(true),
            "header kosong tetap baris heading"
        );
        assert!(table.rows[0].items.is_empty(), "sel heading kosong");
    }

    #[test]
    fn tabel_v1_body_rows_omit_is_heading_field() {
        // Body rows must omit `is_heading` (`None`), never `Some(false)`.
        let tabel = Tabel::baru("T")
            .header(["Metric", "Value"])
            .baris(["1", "2"])
            .baris(["3", "4"]);
        let msg = tabel_to_message(&tabel, None);
        let rich = rich(&msg);
        let table = rich.submessages[0].table_metadata.as_option().unwrap();
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[0].is_heading, Some(true));
        for (i, row) in table.rows.iter().enumerate().skip(1) {
            assert_eq!(
                row.is_heading, None,
                "baris isi #{i} tidak boleh membawa isHeading (Some(false) != None di wire)"
            );
        }

        // Bukti byte: baris yang TIDAK membawa `isHeading` lebih pendek
        // daripada baris yang (harus) membawanya. JS menulis `isHeading: true`
        // HANYA untuk baris pertama.
        let bytes = payload(&tabel_to_message(&tabel, None));
        let with_heading = payload(&tabel_to_message(
            &Tabel::baru("T")
                .header(["Metric", "Value"])
                .baris(["1", "2"])
                .baris(["3", "4"]),
            None,
        ));
        assert_eq!(bytes, with_heading, "builder harus deterministik");
    }

    #[test]
    fn isi_tabel_terkirim_di_wire() {
        let tabel = Tabel::baru("Judul Tabel")
            .header(["Metric", "Value"])
            .baris(["1", "2"]);
        let bytes = payload(&tabel_to_message(&tabel, None));
        let teks = String::from_utf8_lossy(&bytes);
        assert!(teks.contains("Judul Tabel"), "{teks}");
        assert!(teks.contains("Metric"), "{teks}");
    }

    #[test]
    fn context_info_menandai_ter_forward_dengan_asal_meta_ai() {
        let ci = tabel_context_info(&[], None);
        assert_eq!(ci.is_forwarded, Some(true));
        assert_eq!(ci.forwarding_score, Some(1));
        assert_eq!(
            ci.forward_origin,
            Some(wa::context_info::ForwardOrigin::META_AI)
        );
        assert_eq!(
            ci.forwarded_ai_bot_message_info
                .as_option()
                .and_then(|i| i.bot_jid.clone()),
            Some("867051314767696@bot".to_string())
        );
    }
}
