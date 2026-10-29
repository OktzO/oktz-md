//! Builder wire untuk balasan rich — `oktz-plugin-api::rich` → proto WA.
//!
//! Empat builder, empat kontrak yang HARUS tetap terpisah. JS punya jalan
//! berbeda untuk masing-masing, dan yang berbeda itu bukan nama field tapi
//! nilai yang dilihat user (asal forward, JID bot, ada/tidaknya proof):
//!
//! | builder | context score | bot JID | outer |
//! |---|---|---|---|
//! | V2 table (`generateTableContentV2`) | 2 | `259786046210223@bot` | `messageContextInfo` + `unifiedResponse` |
//! | list V1 (`generateListContent`) | 1 | `867051314767696@bot` | — |
//! | AIRich (`builder.js`) | 1 | `0@bot` | `deviceListMetadata` + `botMetadata` disclaimer |
//! | prepared rich (`prepareRichResponseMessage`) | 1 | `867051314767696@bot` | proof + `botResponseId` |
//!
//! # `unifiedResponse.data` = byte JSON UTF-8 MENTAH
//!
//! Bukan base64 ASCII. JS `generateTableContentV2` menulis
//! `Buffer.from(JSON.stringify(unifiedData)).toString("base64")`
//! (`rich-messages.js:1030`) — itu byte teks base64, bukan byte JSON-nya.
//! Proto field-nya `optional bytes data = 1`, dan yang harus dibandingkan
//! klien adalah byte JSON-nya. Test `rich_v2_raw_utf8_unified_bytes_are_not_base64`
//! mengunci itu.
//!
//! `ponytail:` response ID & secret = waktu + counter, bukan RNG. JS pakai
//! `randomUUID`/`randomBytes`. Upgrade path: tambah `rand` kalau task lain
//! sudah membutuhkannya, lalu ganti `unique_id`/`secret_32` — dua fungsi,
//! kontrak yang dikunci tetap "string unik / 32 byte".

use oktz_plugin_api::rich::{AiSubMessage, PreparedRich, RichAi, RichList, RichV2};
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::ai_rich_response_table_metadata::AIRichResponseTableRow;
use whatsapp_rust::waproto::whatsapp::bot_signature_verification_use_case_proof::BotSignatureUseCase;
use whatsapp_rust::waproto::whatsapp::{
    AIRichResponseMessage, AIRichResponseMessageType, AIRichResponseSubMessage,
    AIRichResponseSubMessageType, AIRichResponseTableMetadata,
    AIRichResponseUnifiedResponse, BotMetadata, BotMessageSharingInfo, BotMetricsEntryPoint,
    BotSignatureVerificationMetadata, BotSignatureVerificationUseCaseProof, BotSourcesMetadata,
    ContextInfo, DeviceListMetadata, ForwardedAIBotMessageInfo, MessageContextInfo,
};

use crate::pipeline::QuoteSource;

/// Bot JID tiap jalur. Konstanta bernama supaya salah salin-tempel (angka
/// yang kedip di string literal) tidak diam-diam terjadi.
mod bot {
    /// V2 (`rich-messages.js:1036`).
    pub const V2: &str = "259786046210223@bot";
    /// `buildRichContextInfo` (`rich-messages.js:421`) — V1 table & list,
    /// dan prepared rich (`:1625`).
    pub const RICH: &str = "867051314767696@bot";
    /// `builder.js:1470` (`AIRich.build` dengan `forwarded = true`).
    pub const AI: &str = "0@bot";
}

/// Field proto opsional: `None` = field TIDAK ditulis sama sekali.
///
/// `unwrap_or_default()` di mana-mana menyamarkan niat; satu helper membuat
/// "tidak ada field" eksplisit dan bisa dibaca sekali lihat.
fn fld<T: Default>(v: Option<T>) -> MessageField<T> {
    match v {
        Some(x) => MessageField::some(x),
        None => MessageField::default(),
    }
}

/// `messageSecret` JS = `randomBytes(32)`. Tanpa `rand` (tidak boleh
/// ditambah di task ini) 32 byte-nya dari waktu + counter — BUKAN
/// kriptografis, dan itu tidak mengubah kontrak yang dikunci test: panjang
/// dan tipenya.
fn secret_32() -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    for _ in 0..32 {
        // xorshift64: deterministic dari seed, beda per pemanggilan.
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        out.push(seed as u8);
    }
    out
}

/// ID unik berbasis waktu (padanan `randomUUID` JS tanpa dependensi baru).
///
/// `as_nanos` sudah membedakan pemanggilan dalam satu proses; counter
/// atomik menutup kasio nano-detik yang sama persis.
fn unique_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{nanos:016x}-{n:08x}")
}

/// Submessage teks (`messageType: 2`).
fn sub_text(text: &str) -> AIRichResponseSubMessage {
    AIRichResponseSubMessage {
        message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TEXT),
        message_text: Some(text.to_string()),
        ..Default::default()
    }
}

/// Submessage tabel (`messageType: 4`).
fn sub_table(
    title: &str,
    header: &[String],
    baris: &[Vec<String>],
    header_heading: bool,
) -> AIRichResponseSubMessage {
    AIRichResponseSubMessage {
        message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TABLE),
        table_metadata: MessageField::some(AIRichResponseTableMetadata {
            title: Some(title.to_string()),
            rows: table_rows(header, baris, header_heading),
        }),
        ..Default::default()
    }
}

/// `tableMetadata.rows`. Baris pertama (harga) pakai `is_heading: Some(true)`,
/// baris isi `None` — bukan `Some(false)` (lihat `oktz_plugin_api::tabel`).
///
/// `header_heading = false` untuk LIST V1: JS `generateListContent` mengirim
/// `items.map(...)` TANPA baris heading sama sekali (`rich-messages.js:640`).
fn table_rows(
    header: &[String],
    baris: &[Vec<String>],
    header_heading: bool,
) -> Vec<AIRichResponseTableRow> {
    let mut rows = Vec::with_capacity(baris.len() + 1);
    if header_heading {
        rows.push(AIRichResponseTableRow {
            items: header.to_vec(),
            is_heading: Some(true),
        });
    }
    for b in baris {
        rows.push(AIRichResponseTableRow {
            items: b.clone(),
            is_heading: None,
        });
    }
    rows
}

/// Bungkus jadi `botForwardedMessage` + `messageContextInfo` outer.
///
/// Cuma V2 dan AIRich/prepared yang mengisi outer `messageContextInfo`;
/// jalur lain meninggalkannya kosong (`None`).
fn wrap(rich: AIRichResponseMessage, outer: Option<MessageContextInfo>) -> wa::Message {
    wa::Message {
        bot_forwarded_message: MessageField::some(wa::message::FutureProofMessage {
            message: MessageField::some(wa::Message {
                rich_response_message: MessageField::some(rich),
                ..Default::default()
            }),
        }),
        message_context_info: fld(outer),
        ..Default::default()
    }
}

/// Byte JSON UTF-8 dari `unifiedResponse` — mentah, bukan base64.
fn unified_bytes(response_id: &str, sections: Vec<serde_json::Value>) -> Vec<u8> {
    let unified = serde_json::json!({
        "response_id": response_id,
        "sections": sections,
    });
    serde_json::to_vec(&unified).unwrap_or_default()
}

/// `richResponseMessage` bersejarah untuk builder yang memakai
/// `unifiedResponse` (V2, AIRich, prepared rich).
fn rich_with_unified(
    submessages: Vec<AIRichResponseSubMessage>,
    response_id: &str,
    sections: Vec<serde_json::Value>,
    ci: ContextInfo,
) -> AIRichResponseMessage {
    AIRichResponseMessage {
        message_type: Some(AIRichResponseMessageType::AI_RICH_RESPONSE_TYPE_STANDARD),
        submessages,
        unified_response: fld(Some(AIRichResponseUnifiedResponse {
            data: Some(unified_bytes(response_id, sections)),
        })),
        context_info: MessageField::some(ci),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// contextInfo inner
// ---------------------------------------------------------------------------

/// Isu bersama semua context rich: ter-forward dari `META_AI` + kutipan
/// pemicu kalau ada (`buildRichContextInfo`, `rich-messages.js:417-431`).
///
/// Dipanggil semua builder, sehingga kutipan tidak bisa "lupa" terpasang di
/// salah satu jalur. Score/bot JID ditambah pemanggil masing-masing.
fn base_context(quoted: Option<&QuoteSource>) -> ContextInfo {
    crate::tabel::tabel_context_info(&[], quoted)
}

/// V2 (`:1033-1042`): score 2, `259786046210223@bot`,
/// `botMessageSharingInfo` (`botEntryPointOrigin: 1`, `forwardScore: 2`).
fn v2_context(quoted: Option<&QuoteSource>) -> ContextInfo {
    let mut ci = base_context(quoted);
    ci.forwarding_score = Some(2);
    ci.forwarded_ai_bot_message_info = MessageField::some(ForwardedAIBotMessageInfo {
        bot_jid: Some(bot::V2.to_string()),
        ..Default::default()
    });
    ci.bot_message_sharing_info = fld(Some(BotMessageSharingInfo {
        // `botEntryPointOrigin: 1` = FAVICON (enum `BotMetricsEntryPoint`).
        bot_entry_point_origin: Some(BotMetricsEntryPoint::FAVICON),
        forward_score: Some(2),
    }));
    ci
}

/// AIRich (`builder.js:1523-1527`): score 1, `0@bot`, `META_AI`.
///
/// `0@bot` itu hasil `forwarded = true` di `AIRich.build`, BUKAN angka
/// kosong yang boleh "diganti" bot JID lain. Score-nya 1, bukan 2 seperti V2.
fn ai_context(quoted: Option<&QuoteSource>) -> ContextInfo {
    let mut ci = base_context(quoted);
    ci.forwarding_score = Some(1);
    ci.forwarded_ai_bot_message_info = MessageField::some(ForwardedAIBotMessageInfo {
        bot_jid: Some(bot::AI.to_string()),
        ..Default::default()
    });
    ci
}

// ---------------------------------------------------------------------------
// sections
// ---------------------------------------------------------------------------

/// `GenAIMarkdownTextUXPrimitive` section — pola yang sama di semua builder
/// (`rich-messages.js:521-531`).
fn text_section(text: &str) -> serde_json::Value {
    serde_json::json!({
        "view_model": {
            "primitive": {
                "text": text,
                "__typename": "GenAIMarkdownTextUXPrimitive",
            },
            "__typename": "GenAISingleLayoutViewModel",
        }
    })
}

/// `toTableMetadataV2` (`:944-977`) → `unified_rows`: huruf kecil semua,
/// `is_header` (bukan `isHeading`), sel di-padding `""` sampai panjang
/// terpanjang.
///
/// Padding itu PERLU: JS `Math.max(header.length, ...rows)` +
/// `Array(maxLen - n).fill("")`. Kalau baris lebih pendek dari header dan
/// tidak dipadding, `GenATableUXPrimitive` menaruh sel kosong di tempat lain
/// dan tabel yang tampil bergeser kolom.
fn v2_unified_rows(header: &[String], baris: &[Vec<String>]) -> Vec<serde_json::Value> {
    let max = header
        .len()
        .max(baris.iter().map(Vec::len).max().unwrap_or(0));
    let pad = |cells: &[String]| -> Vec<serde_json::Value> {
        let mut out: Vec<serde_json::Value> = cells
            .iter()
            .map(|c| serde_json::Value::String(c.clone()))
            .collect();
        while out.len() < max {
            out.push(serde_json::Value::String(String::new()));
        }
        out
    };
    let mut rows = vec![serde_json::json!({
        "is_header": true,
        "cells": pad(header),
    })];
    for b in baris {
        rows.push(serde_json::json!({
            "is_header": false,
            "cells": pad(b),
        }));
    }
    rows
}

/// `sections` V2 (`:981-1024`): heading → teks → tabel → footer.
///
/// Urutan mengikuti urutan blok `if` di JS. Section heading memakai
/// `headerText || title` (`:982-983`).
fn v2_sections(input: &RichV2) -> Vec<serde_json::Value> {
    let mut sections = Vec::new();
    let heading = input
        .heading
        .as_deref()
        .filter(|h| !h.is_empty())
        .or_else(|| (!input.judul.is_empty()).then(|| input.judul.as_str()));
    if let Some(h) = heading {
        sections.push(text_section(h));
    }
    if let Some(t) = &input.teks {
        sections.push(text_section(t));
    }
    sections.push(serde_json::json!({
        "view_model": {
            "primitive": {
                "rows": v2_unified_rows(&input.header, &input.baris),
                "__typename": "GenATableUXPrimitive",
            },
            "__typename": "GenAISingleLayoutViewModel",
        }
    }));
    if let Some(f) = &input.footer {
        sections.push(text_section(f));
    }
    sections
}

// ---------------------------------------------------------------------------
// 1. V2 table — `generateTableContentV2` (`rich-messages.js:978`)
// ---------------------------------------------------------------------------

/// Submessage V2 = KOSONG. JS mengirim `submessages: []` dan seluruh isi
/// lewat `unifiedResponse` (`:1063`); menaruh `tableMetadata` DI SINI membuat
/// pesan dobel di beberapa klien.
pub fn rich_v2_to_message(input: &RichV2, quote: Option<&QuoteSource>) -> wa::Message {
    let response_id = unique_id();
    let outer = MessageContextInfo {
        device_list_metadata_version: Some(2),
        message_secret: Some(secret_32()),
        // `threadId: []` (`:1050`) — repeated kosong, proto tidak menuliskannya.
        ..Default::default()
    };
    wrap(
        rich_with_unified(Vec::new(), &response_id, v2_sections(input), v2_context(quote)),
        Some(outer),
    )
}

// ---------------------------------------------------------------------------
// 2. list V1 — `generateListContent` (`rich-messages.js:638`)
// ---------------------------------------------------------------------------

/// Tanpa `unifiedResponse`, tanpa baris heading. Context level V1
/// (`buildRichContextInfo`).
pub fn rich_list_to_message(input: &RichList, quote: Option<&QuoteSource>) -> wa::Message {
    let mut submessages: Vec<AIRichResponseSubMessage> = Vec::new();
    if let Some(h) = &input.heading {
        submessages.push(sub_text(h));
    }
    submessages.push(sub_table(&input.judul, &[], &input.items, false));
    if let Some(f) = &input.footer {
        submessages.push(sub_text(f));
    }
    wrap(
        AIRichResponseMessage {
            message_type: Some(AIRichResponseMessageType::AI_RICH_RESPONSE_TYPE_STANDARD),
            submessages,
            context_info: MessageField::some(base_context(quote)),
            ..Default::default()
        },
        None,
    )
}

// ---------------------------------------------------------------------------
// 3. AIRich — `builder.js` `AIRich.build` (`:1504`)
// ---------------------------------------------------------------------------

/// Submessage AIRich: teks / tabel.
///
/// JS `prepareRichResponseMessage` menerima `richResponse` mentah dan
/// menerjemahkan tiga bentuk (`text`, `code`, `table`); sisanya dikirim apa
/// adanya. Yang dimodelkan di sini cuma dua bentuk yang benar-benar dipakai
/// plugin repo — `code` butuh tokenizer yang utuh (`rich-messages.js:842`),
/// dan itu bukan milik task ini.
///
/// `ponytail:` submessage tanpa text/tabel tidak dirender. JS mengirim
/// bentuk mentahnya; memetakan bentuk yang belum ada pemakainya berarti
/// menebak field. Upgrade path: tambah varian di `AiSubMessage`.
fn ai_submessages(input: &[AiSubMessage]) -> Vec<AIRichResponseSubMessage> {
    let mut out = Vec::with_capacity(input.len());
    for s in input {
        if let Some(t) = &s.text {
            out.push(sub_text(t));
        } else if let Some((title, rows, heading)) = &s.tabel {
            let rows: Vec<AIRichResponseTableRow> = rows
                .iter()
                .enumerate()
                .map(|(i, r)| AIRichResponseTableRow {
                    items: r.clone(),
                    is_heading: (*heading && i == 0).then_some(true),
                })
                .collect();
            out.push(AIRichResponseSubMessage {
                message_type: Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TABLE),
                table_metadata: MessageField::some(AIRichResponseTableMetadata {
                    title: Some(title.clone()),
                    rows,
                }),
                ..Default::default()
            });
        }
    }
    out
}

/// Outer AIRich: `deviceListMetadata: {}`, version 2, `botMetadata` dengan
/// `messageDisclaimerText` — TANPA `verificationMetadata`
/// (`builder.js:1504-1514`).
///
/// `richResponseSourcesMetadata: { sources: [] }` ikut ditulis: JS selalu
/// menaruh objek itu walau kosong.
fn ai_outer(disclaimer: Option<&str>) -> MessageContextInfo {
    MessageContextInfo {
        device_list_metadata: fld(Some(DeviceListMetadata::default())),
        device_list_metadata_version: Some(2),
        bot_metadata: fld(Some(BotMetadata {
            message_disclaimer_text: disclaimer.map(str::to_string),
            rich_response_sources_metadata: fld(Some(BotSourcesMetadata::default())),
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// `sections` AIRich mentah dari plugin — JS `AIRich.build`
/// (`builder.js:1521`) menaruhnya langsung jadi byte tanpa transllasi.
fn ai_sections(input: &[serde_json::Value]) -> Vec<serde_json::Value> {
    input.to_vec()
}

pub fn rich_ai_to_message(input: &RichAi) -> wa::Message {
    let response_id = unique_id();
    wrap(
        rich_with_unified(
            ai_submessages(&input.submessages),
            &response_id,
            ai_sections(&input.sections),
            ai_context(None),
        ),
        Some(ai_outer(input.disclaimer.as_deref())),
    )
}

// ---------------------------------------------------------------------------
// 4. prepared rich — `prepareRichResponseMessage` (`rich-messages.js:1502`)
// ---------------------------------------------------------------------------

/// Outer prepared rich: `botMetadata` berisi `botResponseId =
/// unified.response_id` (`:1634`), `messageDisclaimerText` kalau ada
/// (`:1631-1633`), dan `verificationMetadata` dengan satu proof.
///
/// Proof-nya `wrapToBotForwardedMessage` (`:1480-1501`): satu
/// `BotSignatureVerificationUseCaseProof` berisi dua sertifikat + satu
/// signature. Byte sertifikat/signature di JS ACak (`randomBytes(685)`,
/// `randomBytes(892)`, `getRandomValues(64)`) dan tidak ditiru — yang dikunci
/// test hanyalah ADA-nya proof, panjang bytenya, dan `botResponseId`.
fn prepared_outer(response_id: &str, disclaimer: Option<&str>) -> MessageContextInfo {
    let proof = BotSignatureVerificationUseCaseProof {
        version: Some(1),
        use_case: Some(BotSignatureUseCase::WA_BOT_MSG),
        signature: Some(secret_32()),
        certificate_chain: vec![secret_32(), secret_32()],
        ..Default::default()
    };
    MessageContextInfo {
        bot_metadata: fld(Some(BotMetadata {
            message_disclaimer_text: disclaimer.map(str::to_string),
            bot_response_id: Some(response_id.to_string()),
            verification_metadata: fld(Some(BotSignatureVerificationMetadata {
                proofs: vec![proof],
            })),
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// Context dalam prepared rich (`:1622-1627`): score 1, bot JID
/// `867051314767696@bot`, `forwardOrigin: 4`. TANPA `stanzaId`/`participant`/
/// `quotedMessage` — JS-nya objek tetap, tidak ada cabang kutipan.
pub fn prepared_rich_to_message(input: &PreparedRich) -> wa::Message {
    let response_id = unique_id();
    let mut ci = crate::tabel::tabel_context_info(&[], None);
    ci.forwarding_score = Some(1);
    ci.forwarded_ai_bot_message_info = MessageField::some(ForwardedAIBotMessageInfo {
        bot_jid: Some(bot::RICH.to_string()),
        ..Default::default()
    });
    wrap(
        rich_with_unified(
            ai_submessages(&input.submessages),
            &response_id,
            ai_sections(&input.sections),
            ci,
        ),
        Some(prepared_outer(&response_id, input.disclaimer.as_deref())),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn quote() -> QuoteSource {
        QuoteSource {
            stanza_id: "STANZA1".into(),
            participant: Some("6281@s.whatsapp.net".into()),
            message: Box::new(wa::Message::text("pesan asal")),
        }
    }

    fn rich_response(msg: &wa::Message) -> &AIRichResponseMessage {
        msg.bot_forwarded_message
            .as_option()
            .expect("wajib botForwardedMessage")
            .message
            .as_option()
            .expect("wajib ada message di dalamnya")
            .rich_response_message
            .as_option()
            .expect("wajib richResponseMessage")
    }

    fn unified_data(msg: &wa::Message) -> Vec<u8> {
        rich_response(msg)
            .unified_response
            .as_option()
            .expect("wajib unifiedResponse")
            .data
            .clone()
            .expect("data wajib byte JSON, bukan None")
    }

    fn unified_json(msg: &wa::Message) -> serde_json::Value {
        serde_json::from_slice(&unified_data(msg)).expect("data wajib JSON UTF-8")
    }

    fn inner_context(msg: &wa::Message) -> &ContextInfo {
        rich_response(msg)
            .context_info
            .as_option()
            .expect("wajib contextInfo di richResponseMessage")
    }

    fn bot_jid(msg: &wa::Message) -> Option<String> {
        inner_context(msg)
            .forwarded_ai_bot_message_info
            .as_option()
            .and_then(|i| i.bot_jid.clone())
    }

    fn outer(msg: &wa::Message) -> &MessageContextInfo {
        msg.message_context_info
            .as_option()
            .expect("wajib messageContextInfo")
    }

    fn bot_metadata(msg: &wa::Message) -> &BotMetadata {
        outer(msg).bot_metadata.as_option().expect("wajib botMetadata")
    }

    // ---------- V2 ----------

    #[test]
    fn rich_v2_preserves_parsed_rows_and_sections() {
        // `:981-1024`: heading (headerText || title), teks opsional, tabel,
        // footer opsional. Semuanya di `unifiedResponse`, sama sekali TIDAK
        // di submessages.
        let input = RichV2::baru("Judul")
            .header(["Metric", "Value"])
            .baris(["Status", "Online"])
            .baris(["Cores"])
            .heading("Judul Bagian")
            .teks("Catatan")
            .footer("Penutup");
        let msg = rich_v2_to_message(&input, None);
        let rich = rich_response(&msg);

        assert!(
            rich.submessages.is_empty(),
            "V2 tidak boleh menaruh tableMetadata di submessages"
        );

        let parsed = unified_json(&msg);
        let sections = parsed["sections"].as_array().expect("sections array");
        assert_eq!(sections.len(), 4, "heading + teks + tabel + footer");
        assert_eq!(sections[0]["view_model"]["primitive"]["text"], "Judul Bagian");
        assert_eq!(
            sections[0]["view_model"]["primitive"]["__typename"],
            "GenAIMarkdownTextUXPrimitive"
        );
        assert_eq!(sections[1]["view_model"]["primitive"]["text"], "Catatan");
        assert_eq!(sections[3]["view_model"]["primitive"]["text"], "Penutup");

        let table = &sections[2]["view_model"]["primitive"];
        assert_eq!(table["__typename"], "GenATableUXPrimitive");
        let rows = table["rows"].as_array().expect("rows array");
        assert_eq!(rows.len(), 3, "header + 2 baris");
        assert_eq!(rows[0]["is_header"], json!(true));
        assert_eq!(rows[0]["cells"], json!(["Metric", "Value"]));
        assert_eq!(rows[1]["is_header"], json!(false));
        assert_eq!(rows[1]["cells"], json!(["Status", "Online"]));
        assert_eq!(
            rows[2]["cells"],
            json!(["Cores", ""]),
            "baris pendek wajib dipadding string kosong sampai panjang header"
        );
    }

    #[test]
    fn rich_v2_heading_section_falls_back_to_the_table_title() {
        // `:982-983`: `if (headerText || title)` — heading kosong berarti
        // section heading memakai `title`.
        let tanpa = rich_v2_to_message(&RichV2::baru("Judul"), None);
        assert_eq!(
            unified_json(&tanpa)["sections"][0]["view_model"]["primitive"]["text"],
            "Judul"
        );
        // Judul juga kosong: section heading tidak ada sama sekali.
        let kosong = rich_v2_to_message(&RichV2::baru(""), None);
        assert_eq!(unified_json(&kosong)["sections"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn rich_v2_raw_utf8_unified_bytes_are_not_base64() {
        let input = RichV2::baru("Judul").header(["A"]).baris(["b"]);
        let msg = rich_v2_to_message(&input, None);
        let data = unified_data(&msg);
        let parsed = unified_json(&msg);

        // Byte == `serde_json::to_vec(&expected_json)`: JSON UTF-8 mentah.
        let expected = json!({
            "response_id": parsed["response_id"].clone(),
            "sections": v2_sections(&input),
        });
        assert_eq!(data, serde_json::to_vec(&expected).unwrap());

        // DAN bukan base64 ASCII: base64 dari `{"response_id"...}` dimulai
        // `eyJ` — byte pertama payload JSON adalah `{`.
        assert_eq!(&data[..14], b"{\"response_id\"");
    }

    #[test]
    fn rich_v2_outer_carries_device_metadata_and_thirty_two_byte_secret() {
        let msg = rich_v2_to_message(&RichV2::baru("J"), None);
        let outer = outer(&msg);
        assert_eq!(outer.device_list_metadata_version, Some(2));
        assert_eq!(
            outer.message_secret.as_deref().map(<[u8]>::len),
            Some(32),
            "`messageSecret: randomBytes(32)` (`:1059`)"
        );
    }

    #[test]
    fn rich_v2_context_score_is_two_and_bot_jid_is_meta() {
        let msg = rich_v2_to_message(&RichV2::baru("J"), None);
        let ci = inner_context(&msg);
        assert_eq!(ci.forwarding_score, Some(2), "V2 pakai score 2");
        assert_eq!(ci.is_forwarded, Some(true));
        assert_eq!(ci.forward_origin, Some(wa::context_info::ForwardOrigin::META_AI));
        assert_eq!(bot_jid(&msg).as_deref(), Some("259786046210223@bot"));
    }

    #[test]
    fn rich_v2_context_carries_bot_message_sharing_info() {
        let msg = rich_v2_to_message(&RichV2::baru("J"), None);
        let sharing = inner_context(&msg)
            .bot_message_sharing_info
            .as_option()
            .expect("V2 wajib punya botMessageSharingInfo");
        assert_eq!(sharing.forward_score, Some(2));
    }

    #[test]
    fn rich_v2_keeps_the_quote_from_the_triggering_message() {
        let msg = rich_v2_to_message(&RichV2::baru("J"), Some(&quote()));
        let ci = inner_context(&msg);
        assert_eq!(ci.stanza_id.as_deref(), Some("STANZA1"));
        assert_eq!(ci.participant.as_deref(), Some("6281@s.whatsapp.net"));
        assert!(ci.quoted_message.as_option().is_some());
    }

    // ---------- list V1 ----------

    #[test]
    fn rich_list_body_rows_omit_is_heading_field() {
        let input = RichList::baru("Daftar").item(["a", "b"]).item(["c"]);
        let msg = rich_list_to_message(&input, None);
        let rich = rich_response(&msg);

        assert!(
            rich.unified_response.as_option().is_none(),
            "list V1 tidak punya unifiedResponse"
        );
        let table = rich
            .submessages
            .iter()
            .find_map(|s| s.table_metadata.as_option())
            .expect("wajib ada tableMetadata");
        assert_eq!(table.rows.len(), 2);
        for row in &table.rows {
            assert_eq!(row.is_heading, None, "list V1 tidak punya baris heading");
        }
    }

    // ---------- AIRich ----------

    #[test]
    fn ai_rich_context_score_and_bot_jid_are_distinct() {
        // AIRich: score 1 + `0@bot`. Bukan V2 (score 2, `259786046210223@bot`)
        // dan bukan prepared/V1 (score 1, `867051314767696@bot`).
        let ai = rich_ai_to_message(&RichAi::default());
        let ci = inner_context(&ai);
        assert_eq!(ci.forwarding_score, Some(1), "AIRich pakai score 1");
        assert_eq!(ci.forward_origin, Some(wa::context_info::ForwardOrigin::META_AI));
        assert_eq!(bot_jid(&ai).as_deref(), Some("0@bot"));
    }

    #[test]
    fn ai_rich_keeps_bot_metadata_without_verification_proof() {
        let msg = rich_ai_to_message(&RichAi::default());
        let meta = bot_metadata(&msg);
        assert!(
            meta.verification_metadata.as_option().is_none(),
            "AIRich tidak punya proof (itu khusus prepared rich)"
        );
        assert!(meta.bot_response_id.is_none(), "botResponseId khusus prepared rich");
        // `deviceListMetadata: {}` (`builder.js:1506`) — objek kosong, bukan None.
        assert!(outer(&msg).device_list_metadata.as_option().is_some());
        assert_eq!(outer(&msg).device_list_metadata_version, Some(2));
    }

    #[test]
    fn ai_rich_disclaimer_lands_in_bot_metadata() {
        let msg = rich_ai_to_message(&RichAi {
            disclaimer: Some("Syarat pakai".into()),
            ..Default::default()
        });
        assert_eq!(
            bot_metadata(&msg).message_disclaimer_text.as_deref(),
            Some("Syarat pakai")
        );
    }

    #[test]
    fn ai_rich_sections_are_sent_verbatim() {
        // JS `AIRich.build` menaruh `sections` mentah jadi byte.
        let sections = vec![json!({
            "view_model": {
                "primitive": { "text": "hai", "__typename": "GenAIMarkdownTextUXPrimitive" },
                "__typename": "GenAISingleLayoutViewModel",
            }
        })];
        let msg = rich_ai_to_message(&RichAi {
            sections: sections.clone(),
            ..Default::default()
        });
        assert_eq!(unified_json(&msg)["sections"], json!(sections));
    }

    // ---------- prepared rich ----------

    #[test]
    fn prepared_rich_uses_separate_proof_and_bot_jid() {
        // Prepared: score 1 + `867051314767696@bot` — bot JID SAMA dengan V1,
        // tapi BEDA dari AIRich (`0@bot`) dan V2 (`259786046210223@bot`), dan
        // dia satu-satunya yang membawa proof + `botResponseId`.
        let prepared = prepared_rich_to_message(&PreparedRich::default());
        let ci = inner_context(&prepared);
        assert_eq!(ci.forwarding_score, Some(1));
        assert_eq!(ci.forward_origin, Some(wa::context_info::ForwardOrigin::META_AI));
        assert_eq!(bot_jid(&prepared).as_deref(), Some("867051314767696@bot"));

        let proofs = bot_metadata(&prepared)
            .verification_metadata
            .as_option()
            .expect("hanya prepared rich yang punya proof")
            .proofs
            .clone();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].certificate_chain.len(), 2);
        assert_eq!(proofs[0].signature.as_deref().map(<[u8]>::len), Some(32));
    }

    #[test]
    fn prepared_rich_bot_response_id_equals_unified_response_id() {
        // `:1634` — `botMetadata.botResponseId = unified.response_id`.
        let msg = prepared_rich_to_message(&PreparedRich::default());
        let parsed = unified_json(&msg);
        let response_id = parsed["response_id"].as_str().unwrap().to_string();
        assert_eq!(
            bot_metadata(&msg).bot_response_id.as_deref(),
            Some(response_id.as_str())
        );
    }

    #[test]
    fn prepared_rich_disclaimer_lands_in_bot_metadata() {
        let msg = prepared_rich_to_message(&PreparedRich {
            disclaimer: Some("Bukan nasihat medis".into()),
            ..Default::default()
        });
        assert_eq!(
            bot_metadata(&msg).message_disclaimer_text.as_deref(),
            Some("Bukan nasihat medis")
        );
    }

    #[test]
    fn prepared_rich_context_has_no_quote_fields() {
        // `:1622-1627`: contextInfo-nya objek tetap, tidak ada cabang kutipan.
        let msg = prepared_rich_to_message(&PreparedRich::default());
        let ci = inner_context(&msg);
        assert!(ci.stanza_id.is_none());
        assert!(ci.participant.is_none());
        assert!(ci.quoted_message.as_option().is_none());
    }

    // ---------- submessages ----------

    #[test]
    fn ai_submessage_table_marks_first_row_heading() {
        let input = RichAi {
            submessages: vec![AiSubMessage {
                text: None,
                tabel: Some(("J".into(), vec![vec!["a".into()], vec!["b".into()]], true)),
            }],
            ..Default::default()
        };
        let msg = rich_ai_to_message(&input);
        let table = rich_response(&msg)
            .submessages
            .iter()
            .find_map(|s| s.table_metadata.as_option())
            .expect("tableMetadata");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].is_heading, Some(true));
        assert_eq!(table.rows[1].is_heading, None);
    }

    #[test]
    fn prepared_rich_submessage_text_is_sent_as_text() {
        let input = PreparedRich {
            submessages: vec![AiSubMessage {
                text: Some("halo".into()),
                tabel: None,
            }],
            ..Default::default()
        };
        let msg = prepared_rich_to_message(&input);
        assert_eq!(
            rich_response(&msg).submessages[0].message_text.as_deref(),
            Some("halo")
        );
        assert_eq!(
            rich_response(&msg).submessages[0].message_type,
            Some(AIRichResponseSubMessageType::AI_RICH_RESPONSE_TEXT)
        );
    }

// Pagar parity byte-level: sections V2 Rust == sections V2 JS.
//
// Oracle: `node /tmp/opencode/oracle.mjs` (fungsi `toTableMetadataV2` +
// blok `sections` disalin dari `rich-messages.js:981-1013`). Byte di bawah
// disalin dari keluaran oracle itu.
#[test]
fn v2_sections_byte_for_byte_match_the_js_oracle() {
    let oracle = r#"{"response_id":"RID","sections":[{"view_model":{"primitive":{"text":"Judul","__typename":"GenAIMarkdownTextUXPrimitive"},"__typename":"GenAISingleLayoutViewModel"}},{"view_model":{"primitive":{"rows":[{"is_header":true,"cells":["Metric","Value"]},{"is_header":false,"cells":["Status","Online"]},{"is_header":false,"cells":["Cores",""]}],"__typename":"GenATableUXPrimitive"},"__typename":"GenAISingleLayoutViewModel"}}]}"#;
    // Sumber: `table = ["Judul", "Metric|Value", "Status|Online", "Cores"]`
    // — pemisah kolom `|`, pemisah baris `;;`, keduanya dari `splitCols`/
    // `splitRows` (`:949-958`).
    let input = RichV2::baru("Judul")
        .header(["Metric", "Value"])
        .baris(["Status", "Online"])
        .baris(["Cores"]);
    let got = serde_json::to_vec(&serde_json::json!({
        "response_id": "RID",
        "sections": v2_sections(&input),
    }))
    .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&got),
        oracle,
        "byte sections V2 wajib sama dengan JS (termasuk padding sel dan urutan key)"
    );
}

    // ---------- pagar regresi ----------

    /// Pagar: tabel V1 (model lama) tetap pakai builder lamanya. Regresi yang
    /// mengalihkan `Reply::Tabel` ke jalur rich akan memperbaiki test ini
    /// karena gagal.
    #[test]
    fn tabel_v1_is_untouched_by_the_rich_builders() {
        use oktz_plugin_api::tabel::Tabel;
        let tabel = Tabel::baru("⚡ System Performance")
            .header(["Metric", "Value"])
            .baris(["Status", "Online"]);
        let msg = crate::tabel::tabel_to_message(&tabel, None);
        let rich = rich_response(&msg);
        assert_eq!(rich.submessages.len(), 1, "tanpa header_teks/footer = 1 submessage");
        assert!(
            rich.unified_response.as_option().is_none(),
            "tabel V1 tidak punya unifiedResponse"
        );
        let table = rich.submessages[0].table_metadata.as_option().unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].is_heading, Some(true));
        assert_eq!(table.rows[1].is_heading, None);
    }

    #[test]
    fn each_builder_produces_a_distinct_wire_message() {
        // Bukti separasi empat jalur: byte-nya tidak boleh sama walau isi
        // modelnya sama bentuknya.
        let input = RichAi::default();
        let ai = rich_ai_to_message(&input);
        let prepared = prepared_rich_to_message(&PreparedRich::default());
        let v2 = rich_v2_to_message(&RichV2::baru("J"), None);
        let list = rich_list_to_message(&RichList::baru("J"), None);
        assert_ne!(bot_jid(&ai), bot_jid(&v2));
        assert_ne!(bot_jid(&ai), bot_jid(&prepared));
        assert_ne!(
            ai.message_context_info.as_option().is_some(),
            list.message_context_info.as_option().is_some(),
            "list V1 tidak punya messageContextInfo, AIRich punya"
        );
    }
}
