//! Port `getMessageType` + `getMessageBody` dari `src/lib/serialize.js:309-372`.
//!
//! Layer ini SENGaja tidak tahu apa-apa soal protobuf: caller adaptasi
//! `wacore::proto` ke [`MessageKind`] + [`MessageParams`], lalu dapat
//! `String`. Itu yang bikin bisa diuji tanpa koneksi WA.
//!
//! Bentuk `params` dibuat longgar: field yang tidak ada = `None`, dan `None`
//!means `""` — sama dengan `if (!messageContent) return ""` di JS.
//!
//! Cabang `interactiveResponseMessage` melakukan `JSON.parse` BERTAHAP dan
//! seluruhnya dibungkus satu `try`. Kalau `response_json` rusak, `catch` JS
//! menelan error dan mengembalikan `""` — bukan jatuh ke `selectedRowId`
//! tingkat atas. Presedensi itu di Reproduksi persis di `js_truthy_text` +
//! `interactive_body`; salah urutan = tombol yang diklik beda.

use serde_json::Value;

/// Tipe pesan yang punya arti buat router. `Other` = tipe yang tidak ada di
/// switch JS (dan `getMessageBody` mengembalikan `""` untuknya).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    Conversation,
    ExtendedText,
    Image,
    Video,
    Document,
    ButtonsResponse,
    ListResponse,
    TemplateButtonReply,
    InteractiveResponse,
    PollCreation,
    PollCreationV2,
    PollCreationV3,
    /// Tipe di luar daftar di atas → body `""`.
    Other,
}

impl MessageKind {
    /// Pemetaan nama field protobuf → enum. Nama persis dengan switch JS.
    pub fn from_wire_type(wire: &str) -> Self {
        match wire {
            "conversation" => Self::Conversation,
            "extendedTextMessage" => Self::ExtendedText,
            "imageMessage" => Self::Image,
            "videoMessage" => Self::Video,
            "documentMessage" => Self::Document,
            "buttonsResponseMessage" => Self::ButtonsResponse,
            "listResponseMessage" => Self::ListResponse,
            "templateButtonReplyMessage" => Self::TemplateButtonReply,
            "interactiveResponseMessage" => Self::InteractiveResponse,
            "pollCreationMessage" => Self::PollCreation,
            "pollCreationMessageV2" => Self::PollCreationV2,
            "pollCreationMessageV3" => Self::PollCreationV3,
            _ => Self::Other,
        }
    }

    /// Nama field protobuf, kebalikan dari [`Self::from_wire_type`].
    pub fn wire_type(self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::ExtendedText => "extendedTextMessage",
            Self::Image => "imageMessage",
            Self::Video => "videoMessage",
            Self::Document => "documentMessage",
            Self::ButtonsResponse => "buttonsResponseMessage",
            Self::ListResponse => "listResponseMessage",
            Self::TemplateButtonReply => "templateButtonReplyMessage",
            Self::InteractiveResponse => "interactiveResponseMessage",
            Self::PollCreation => "pollCreationMessage",
            Self::PollCreationV2 => "pollCreationMessageV2",
            Self::PollCreationV3 => "pollCreationMessageV3",
            Self::Other => "",
        }
    }
}

/// Isi pesan yang sudah dilepas dari protobuf. Hanya field yang dibaca
/// `getMessageBody`; sisanya tidak perlu di-copy ke sini.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageParams<'a> {
    /// `message.conversation`
    pub conversation: Option<&'a str>,
    /// `extendedTextMessage.text`
    pub text: Option<&'a str>,
    /// `imageMessage|videoMessage|documentMessage.caption`
    pub caption: Option<&'a str>,
    /// `buttonsResponseMessage.selectedButtonId`
    pub selected_button_id: Option<&'a str>,
    /// `listResponseMessage.singleSelectReply.selectedRowId`
    pub selected_row_id: Option<&'a str>,
    /// `templateButtonReplyMessage.selectedId`
    pub selected_id: Option<&'a str>,
    /// `interactiveResponseMessage.nativeFlowResponseMessage.paramsJson`
    pub params_json: Option<&'a str>,
    /// `pollCreationMessage*.name`
    pub name: Option<&'a str>,
}

/// `or_empty` — JS `x || ""` pada field string.
fn or_empty(v: Option<&str>) -> String {
    v.unwrap_or_default().to_string()
}

/// Port `getMessageType` (`serialize.js:309-319`).
///
/// `content_type` = hasil `getContentType()` dari onigis (memerlukan
/// protobuf, jadi meneruskan lewat caller). `messageContextInfo` +
/// ada `interactiveResponseMessage` → dipaksa jadi interactive, karena
/// onigis menandai pesan native-flow itu sebagai `messageContextInfo`.
pub fn resolve_message_type(
    content_type: Option<&str>,
    has_interactive_response: bool,
) -> Option<String> {
    let ct = content_type?;
    if ct == "messageContextInfo" && has_interactive_response {
        return Some("interactiveResponseMessage".to_string());
    }
    Some(ct.to_string())
}

/// Port `getMessageBody` (`serialize.js:327-372`).
pub fn get_message_body(kind: MessageKind, params: &MessageParams<'_>) -> String {
    match kind {
        MessageKind::Conversation => or_empty(params.conversation),
        MessageKind::ExtendedText => or_empty(params.text),
        MessageKind::Image | MessageKind::Video | MessageKind::Document => {
            or_empty(params.caption)
        }
        MessageKind::ButtonsResponse => or_empty(params.selected_button_id),
        MessageKind::ListResponse => or_empty(params.selected_row_id),
        MessageKind::TemplateButtonReply => or_empty(params.selected_id),
        MessageKind::InteractiveResponse => interactive_body(params),
        MessageKind::PollCreation | MessageKind::PollCreationV2 | MessageKind::PollCreationV3 => {
            or_empty(params.name)
        }
        MessageKind::Other => String::new(),
    }
}

/// Cabang `interactiveResponseMessage` (`serialize.js:348-364`).
///
/// Presedensi, PERSIS urutan JS:
/// 1. `parsed.id`
/// 2. `parsed.response_json` → parse → `nested.id`
/// 3. `parsed.response_json` → parse → `nested.selectedRowId`
/// 4. `parsed.selectedRowId`
/// 5. `parsed.selected_row_id`
/// 6. `""`
///
/// Satu-satunya `try` di JS membungkus langkah 2–3, jadi `JSON.parse` yang
/// gagal di `response_json` mengembalikan `""` untuk SELURUH fungsi — bukan
/// lanjut ke langkah 4. Itu yang bikin `None` di sini, bukan `else`.
fn interactive_body(params: &MessageParams<'_>) -> String {
    // `messageContent.nativeFlowResponseMessage?.paramsJson || "{}"`
    let raw = params.params_json.filter(|s| !s.is_empty()).unwrap_or("{}");
    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return String::new(); // catch → ""
    };

    // 1
    if let Some(id) = js_truthy_text(parsed.get("id")) {
        return id;
    }

    // 2 + 3 — `if (parsed.response_json)` dulu (JS truthiness), baru parse.
    if let Some(nested_raw) = js_truthy_text(parsed.get("response_json")) {
        let Ok(nested) = serde_json::from_str::<Value>(&nested_raw) else {
            return String::new(); // parse gagal → catch → "", bukan lanjut ke 4
        };
        if let Some(id) = js_truthy_text(nested.get("id")) {
            return id;
        }
        if let Some(row) = js_truthy_text(nested.get("selectedRowId")) {
            return row;
        }
    }

    // 4
    if let Some(row) = js_truthy_text(parsed.get("selectedRowId")) {
        return row;
    }
    // 5
    if let Some(row) = js_truthy_text(parsed.get("selected_row_id")) {
        return row;
    }
    // 6
    String::new()
}

/// Setara `if (value)` versi JS atas `serde_json::Value`, dikembalikan sebagai
/// teks.
///
/// JS tidak cares tipe di sini — `parsed.id` yang berupa angka tetap
/// dikembalikan sebagai body (dan nanti bikin `body.startsWith` crash).
/// Versi ini: string dipakai apa adanya, angka/boolean_truthy jadi teks
/// JSON-nya, array/object dianggap bukan-id (menghasilkan `""` alih-alih
/// crash). Semua perbedaan itu soal "kalau tidak valid" — urutan presetnya
/// tetap sama.
///
/// `ponytail:` kalau nanti terlihat payload native-flow dengan `id` numerik
/// sungguhan, kembalikan `serde_json::to_string(&value)` di sini.
fn js_truthy_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        Value::Bool(b) => b.then(|| "true".to_string()),
        Value::Number(n) => n
            .as_f64()
            .filter(|f| *f != 0.0)
            .map(|_| n.to_string()),
        Value::String(s) => (!s.is_empty()).then(|| s.clone()),
        // `[]` di JS truthy tapi bukan teks — tidak ada id yang bisa dikirim.
        Value::Array(_) | Value::Object(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: params dengan `params_json` saja.
    fn interactive(json: &str) -> MessageParams<'_> {
        MessageParams {
            params_json: Some(json),
            ..Default::default()
        }
    }

    // ---- MessageKind mapping ------------------------------------------

    #[test]
    fn wire_type_roundtrips_for_every_kind() {
        for kind in [
            MessageKind::Conversation,
            MessageKind::ExtendedText,
            MessageKind::Image,
            MessageKind::Video,
            MessageKind::Document,
            MessageKind::ButtonsResponse,
            MessageKind::ListResponse,
            MessageKind::TemplateButtonReply,
            MessageKind::InteractiveResponse,
            MessageKind::PollCreation,
            MessageKind::PollCreationV2,
            MessageKind::PollCreationV3,
        ] {
            assert_eq!(MessageKind::from_wire_type(kind.wire_type()), kind);
        }
    }

    #[test]
    fn unknown_wire_type_is_other() {
        assert_eq!(MessageKind::from_wire_type("reactionMessage"), MessageKind::Other);
        assert_eq!(MessageKind::from_wire_type(""), MessageKind::Other);
        assert_eq!(MessageKind::Other.wire_type(), "");
    }

    // ---- getMessageType ------------------------------------------------

    #[test]
    fn context_info_with_interactive_is_promoted_to_interactive() {
        assert_eq!(
            resolve_message_type(Some("messageContextInfo"), true).as_deref(),
            Some("interactiveResponseMessage")
        );
    }

    #[test]
    fn context_info_without_interactive_stays_context_info() {
        assert_eq!(
            resolve_message_type(Some("messageContextInfo"), false).as_deref(),
            Some("messageContextInfo")
        );
    }

    #[test]
    fn other_content_types_pass_through() {
        assert_eq!(
            resolve_message_type(Some("conversation"), true).as_deref(),
            Some("conversation")
        );
        assert_eq!(resolve_message_type(None, true), None);
    }

    // ---- conversation / extendedText -----------------------------------

    #[test]
    fn conversation_returns_conversation() {
        let p = MessageParams {
            conversation: Some("halo"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::Conversation, &p), "halo");
    }

    #[test]
    fn conversation_missing_is_empty() {
        assert_eq!(get_message_body(MessageKind::Conversation, &MessageParams::default()), "");
    }

    #[test]
    fn extended_text_returns_text() {
        let p = MessageParams {
            text: Some("buka"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::ExtendedText, &p), "buka");
        assert_eq!(get_message_body(MessageKind::ExtendedText, &MessageParams::default()), "");
    }

    // ---- caption: image / video / document -----------------------------

    #[test]
    fn media_kinds_return_caption() {
        let p = MessageParams {
            caption: Some("foto"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::Image, &p), "foto");
        assert_eq!(get_message_body(MessageKind::Video, &p), "foto");
        assert_eq!(get_message_body(MessageKind::Document, &p), "foto");
    }

    #[test]
    fn media_kinds_without_caption_are_empty() {
        for kind in [MessageKind::Image, MessageKind::Video, MessageKind::Document] {
            assert_eq!(get_message_body(kind, &MessageParams::default()), "");
        }
    }

    // ---- button responses ----------------------------------------------

    #[test]
    fn buttons_response_returns_selected_button_id() {
        let p = MessageParams {
            selected_button_id: Some("btn-1"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::ButtonsResponse, &p), "btn-1");
        assert_eq!(
            get_message_body(MessageKind::ButtonsResponse, &MessageParams::default()),
            ""
        );
    }

    #[test]
    fn list_response_returns_selected_row_id() {
        let p = MessageParams {
            selected_row_id: Some("row-3"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::ListResponse, &p), "row-3");
        assert_eq!(
            get_message_body(MessageKind::ListResponse, &MessageParams::default()),
            ""
        );
    }

    #[test]
    fn template_button_reply_returns_selected_id() {
        let p = MessageParams {
            selected_id: Some("tpl-9"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::TemplateButtonReply, &p), "tpl-9");
        assert_eq!(
            get_message_body(MessageKind::TemplateButtonReply, &MessageParams::default()),
            ""
        );
    }

    // ---- poll ----------------------------------------------------------

    #[test]
    fn all_poll_versions_return_name() {
        let p = MessageParams {
            name: Some("Makan siang?"),
            ..Default::default()
        };
        for kind in [
            MessageKind::PollCreation,
            MessageKind::PollCreationV2,
            MessageKind::PollCreationV3,
        ] {
            assert_eq!(get_message_body(kind, &p), "Makan siang?");
            assert_eq!(get_message_body(kind, &MessageParams::default()), "");
        }
    }

    // ---- unknown -------------------------------------------------------

    #[test]
    fn other_kind_is_always_empty_even_with_fields_present() {
        let p = MessageParams {
            conversation: Some("x"),
            text: Some("y"),
            caption: Some("z"),
            ..Default::default()
        };
        assert_eq!(get_message_body(MessageKind::Other, &p), "");
    }

    // ---- interactive: happy paths --------------------------------------

    #[test]
    fn interactive_top_level_id_wins() {
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(r#"{"id":"top"}"#)),
            "top"
        );
    }

    #[test]
    fn interactive_nested_id_used_when_no_top_level_id() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"response_json":"{\"id\":\"nested\"}"}"#)
            ),
            "nested"
        );
    }

    #[test]
    fn interactive_nested_selected_row_id_used_when_nested_has_no_id() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"response_json":"{\"selectedRowId\":\"nrow\"}"}"#)
            ),
            "nrow"
        );
    }

    #[test]
    fn interactive_top_level_selected_row_id_used_when_no_nested_match() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"response_json":"{\"other\":1}","selectedRowId":"row"}"#)
            ),
            "row"
        );
    }

    #[test]
    fn interactive_snake_case_selected_row_id_is_last_resort() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"selected_row_id":"snake"}"#)
            ),
            "snake"
        );
    }

    // ---- interactive: PRECEDENCE — the load-bearing part ---------------

    #[test]
    fn precedence_top_id_beats_everything() {
        let json = r#"{
            "id":"top",
            "response_json":"{\"id\":\"nested\",\"selectedRowId\":\"nrow\"}",
            "selectedRowId":"row",
            "selected_row_id":"snake"
        }"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "top"
        );
    }

    #[test]
    fn precedence_nested_id_beats_nested_row_and_outer_fields() {
        let json = r#"{
            "response_json":"{\"id\":\"nested\",\"selectedRowId\":\"nrow\"}",
            "selectedRowId":"row",
            "selected_row_id":"snake"
        }"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "nested"
        );
    }

    #[test]
    fn precedence_nested_row_beats_outer_row_fields() {
        let json = r#"{
            "response_json":"{\"selectedRowId\":\"nrow\"}",
            "selectedRowId":"row",
            "selected_row_id":"snake"
        }"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "nrow"
        );
    }

    #[test]
    fn precedence_outer_row_beats_snake_case() {
        let json = r#"{"response_json":"{}","selectedRowId":"row","selected_row_id":"snake"}"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "row"
        );
    }

    #[test]
    fn precedence_empty_top_id_is_falsy_so_next_rule_wins() {
        // `if (parsed.id)` — string kosong itu falsy di JS.
        let json = r#"{"id":"","selectedRowId":"row"}"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "row"
        );
    }

    #[test]
    fn precedence_empty_nested_id_is_falsy_so_nested_row_wins() {
        let json = r#"{"response_json":"{\"id\":\"\",\"selectedRowId\":\"nrow\"}"}"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "nrow"
        );
    }

    #[test]
    fn precedence_empty_response_json_is_falsy_so_outer_row_wins() {
        let json = r#"{"response_json":"","selectedRowId":"row"}"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            "row"
        );
    }

    #[test]
    fn precedence_null_and_zero_values_are_falsy() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"id":null,"selectedRowId":"row"}"#)
            ),
            "row"
        );
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"id":0,"selected_row_id":"snake"}"#)
            ),
            "snake"
        );
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"response_json":null,"selectedRowId":"row"}"#)
            ),
            "row"
        );
    }

    // ---- interactive: parse-failure paths ------------------------------

    #[test]
    fn parse_failure_top_level_returns_empty() {
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive("{bukan json")),
            ""
        );
    }

    #[test]
    fn parse_failure_nested_returns_empty_without_falling_through() {
        // Ini yang paling mudah salah: `catch` JS membungkus blok
        // response_json, jadi `selectedRowId` di luar TIDAK dipakai.
        let json = r#"{"response_json":"{rusak","selectedRowId":"row","selected_row_id":"snake"}"#;
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(json)),
            ""
        );
    }

    #[test]
    fn missing_params_json_behaves_as_empty_object() {
        let p = MessageParams::default();
        assert_eq!(get_message_body(MessageKind::InteractiveResponse, &p), "");
    }

    #[test]
    fn empty_params_json_behaves_as_empty_object() {
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive("")),
            ""
        );
    }

    #[test]
    fn params_json_without_any_known_key_returns_empty() {
        assert_eq!(
            get_message_body(
                MessageKind::InteractiveResponse,
                &interactive(r#"{"unrelated":"x","response_json":"{\"a\":1}"}"#)
            ),
            ""
        );
    }

    #[test]
    fn non_object_params_json_returns_empty() {
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive("123")),
            ""
        );
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive("null")),
            ""
        );
    }

    // ---- js_truthy_text unit -------------------------------------------

    #[test]
    fn js_truthy_text_handles_every_json_type() {
        assert_eq!(js_truthy_text(Some(&Value::Null)), None);
        assert_eq!(js_truthy_text(Some(&Value::Bool(false))), None);
        assert_eq!(js_truthy_text(Some(&Value::Bool(true))).as_deref(), Some("true"));
        assert_eq!(js_truthy_text(Some(&serde_json::json!(0))), None);
        assert_eq!(js_truthy_text(Some(&serde_json::json!(7))).as_deref(), Some("7"));
        assert_eq!(js_truthy_text(Some(&Value::String(String::new()))), None);
        assert_eq!(js_truthy_text(Some(&Value::String("x".into()))).as_deref(), Some("x"));
        assert_eq!(js_truthy_text(Some(&serde_json::json!([]))), None);
        assert_eq!(js_truthy_text(Some(&serde_json::json!({}))), None);
        assert_eq!(js_truthy_text(None), None);
    }

    #[test]
    fn numeric_id_is_stringified_instead_of_crashing() {
        assert_eq!(
            get_message_body(MessageKind::InteractiveResponse, &interactive(r#"{"id":42}"#)),
            "42"
        );
    }
}
