//! Tabel AI (WA `richResponseMessage`) — padanan `sock.sendTable` JS.
//!
//! Bentuk JS (`src/lib/rich-messages.js:608` `generateTableContent`):
//! `botForwardedMessage` → `richResponseMessage` dengan submessages:
//! teks opsional (judul), `tableMetadata` (judul + baris), teks opsional
//! (footer). Baris pertama = heading (`isHeading: true`).
//!
//! `sendTable` JS memakai `relayMessage` tanpa node `biz` — jadi tabel ini
//! juga dikirim tanpa node itu.

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
    if !tabel.header.is_empty() {
        rows.push(AIRichResponseTableRow {
            items: tabel.header.clone(),
            is_heading: Some(true),
        });
    }
    for baris in &tabel.baris {
        rows.push(AIRichResponseTableRow {
            items: baris.clone(),
            is_heading: Some(false),
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
    fn baris_pertama_bukan_heading_kalau_tidak_ada_header() {
        let tabel = Tabel::baru("T").baris(["a", "b"]);
        let msg = tabel_to_message(&tabel, None);
        let rich = rich(&msg);
        let table = rich.submessages[0].table_metadata.as_option().unwrap();
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].is_heading, Some(false));
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
