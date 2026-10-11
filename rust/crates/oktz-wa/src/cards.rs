//! Builder kartu interaktif WhatsApp (native flow).
//!
//! Pola yang WA benar-benar render — lihat `src/lib/rich-messages.js:454`
//! (`wrapInteractive`) dan `src/lib/socket.js:359` (`sendInteractiveCard`),
//! keduanya pola yang dipakai plugin yang terbukti jalan di produksi:
//!
//! 1. `interactiveMessage` dibungkus `viewOnceMessage`, dan field itu bertipe
//!    `FutureProofMessage` (WAProto.proto:2095), bukan `ViewOnceMessage`,
//! 2. `messageContextInfo` memuat `deviceListMetadata` +
//!    `deviceListMetadataVersion: 2`,
//! 3. dikirim bersama extra stanza node `biz > interactive > native_flow`,
//!    lihat [`biz_node`].
//!
//! Bentuk legacy (`ListMessage`, `interactiveButtons`) relay-nya sukses tapi WA
//! membuangnya diam-diam — tidak ada yang muncul di chat. Jangan dipakai.

use serde::Serialize;
use serde_json::Value;
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;
use whatsapp_rust::{NodeBuilder, wacore_binary::Node};

/// Nama aksi tombol native-flow. WA membaca `name` sebagai string literal dan
/// nama yang salah membuat tombol hilang tanpa error sama sekali, jadi daftar
/// ini enum. Namanya diambil dari `src/lib/builder.js` dan pemakaian nyata di
/// `plugins/` (quick_reply ~105, single_select ~32, cta_url ~15, cta_copy ~8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    QuickReply,
    SingleSelect,
    CtaUrl,
    CtaCopy,
    CtaCall,
    CtaReminder,
    CtaCancelReminder,
    AddressMessage,
    SendLocation,
}

impl ButtonAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuickReply => "quick_reply",
            Self::SingleSelect => "single_select",
            Self::CtaUrl => "cta_url",
            Self::CtaCopy => "cta_copy",
            Self::CtaCall => "cta_call",
            Self::CtaReminder => "cta_reminder",
            Self::CtaCancelReminder => "cta_cancel_reminder",
            Self::AddressMessage => "address_message",
            Self::SendLocation => "send_location",
        }
    }
}

/// Satu baris di dalam `single_select`. Field yang tidak diisi dilewati dari
/// `buttonParamsJson` — bentuk akhir sudah ditentukan WA, bukan kita.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub title: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Row {
    pub fn new(title: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
            header: None,
            description: None,
        }
    }

    pub fn header(mut self, h: impl Into<String>) -> Self {
        self.header = Some(h.into());
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
}

/// Satu section `single_select` (`builder.js:617` `makeSection`).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Section {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_label: Option<String>,
    pub rows: Vec<Row>,
}

impl Section {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    pub fn highlight_label(mut self, h: impl Into<String>) -> Self {
        self.highlight_label = Some(h.into());
        self
    }

    pub fn row(mut self, r: Row) -> Self {
        self.rows.push(r);
        self
    }

    pub fn rows(mut self, it: impl IntoIterator<Item = Row>) -> Self {
        self.rows.extend(it);
        self
    }
}

/// Isi `buttonParamsJson` untuk `single_select` (`plugins/jpm/jpm.js:304`).
#[derive(Debug, Clone, Serialize)]
pub struct List {
    pub title: String,
    pub sections: Vec<Section>,
}

impl List {
    pub fn new(title: impl Into<String>, sections: impl IntoIterator<Item = Section>) -> Self {
        Self {
            title: title.into(),
            sections: sections.into_iter().collect(),
        }
    }

    /// Kartu list siap kirim: body + footer + satu tombol `single_select`.
    pub fn into_card(self, body: &str, footer: Option<&str>) -> Card {
        let button = Button::select(&self);
        let mut card = Card::new(body);
        if let Some(f) = footer {
            card = card.footer(f);
        }
        card.button(button)
    }
}

/// Satu tombol native-flow. `params` adalah objek yang diserialisasi jadi
/// `buttonParamsJson`; bentuknya sudah ditentukan WA, makanya `Value` mentah
/// dengan constructor yang benar-benar dipakai plugin.
#[derive(Debug, Clone)]
pub struct Button {
    pub action: ButtonAction,
    pub params: Value,
}

impl Button {
    pub fn new(action: ButtonAction, params: Value) -> Self {
        Self { action, params }
    }

    pub fn quick_reply(display_text: impl Into<String>, id: impl Into<String>) -> Self {
        Self::new(
            ButtonAction::QuickReply,
            serde_json::json!({ "display_text": display_text.into(), "id": id.into() }),
        )
    }

    pub fn copy(display_text: impl Into<String>, copy_code: impl Into<String>) -> Self {
        Self::new(
            ButtonAction::CtaCopy,
            serde_json::json!({ "display_text": display_text.into(), "copy_code": copy_code.into() }),
        )
    }

    pub fn url(display_text: impl Into<String>, url: impl Into<String>) -> Self {
        Self::new(
            ButtonAction::CtaUrl,
            serde_json::json!({ "display_text": display_text.into(), "url": url.into() }),
        )
    }

    /// Tombol `single_select` — isi params dari [`List`].
    pub fn select(list: &List) -> Self {
        Self::new(
            ButtonAction::SingleSelect,
            serde_json::json!({ "title": &list.title, "sections": &list.sections }),
        )
    }
}

/// Header kartu. Media header perlu pesan yang sudah ter-upload, jadi
/// [`Card`] hanya mengekspos teks + penanda lampiran.
#[derive(Debug, Clone, Default)]
pub struct Header {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub has_media_attachment: bool,
}

/// Kartu native-flow. Kirim dengan `biz_node()` sebagai extra stanza node,
/// misalnya lewat `SendOptions::with_extra_stanza_nodes`.
#[derive(Debug, Clone, Default)]
pub struct Card {
    pub body: String,
    pub footer: Option<String>,
    pub header: Option<Header>,
    pub buttons: Vec<Button>,
    /// `messageParamsJson` — `bottom_sheet`, `limited_time_offer`, dll.
    pub message_params: Option<Value>,
    /// `contextInfo` — dipakai untuk `mentionedJid` di kartu grup
    /// (`plugins/group/cekidgc.js:189`).
    pub context_info: Option<wa::ContextInfo>,
}

impl Card {
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            ..Default::default()
        }
    }

    pub fn footer(mut self, f: impl Into<String>) -> Self {
        self.footer = Some(f.into());
        self
    }

    pub fn header(mut self, h: Header) -> Self {
        self.header = Some(h);
        self
    }

    pub fn message_params(mut self, v: Value) -> Self {
        self.message_params = Some(v);
        self
    }

    pub fn context_info(mut self, c: wa::ContextInfo) -> Self {
        self.context_info = Some(c);
        self
    }

    pub fn button(mut self, b: Button) -> Self {
        self.buttons.push(b);
        self
    }

    pub fn buttons(mut self, it: impl IntoIterator<Item = Button>) -> Self {
        self.buttons.extend(it);
        self
    }

    pub fn to_message(&self) -> wa::Message {
        use wa::message::interactive_message::{
            Body, Footer, Header as ProtoHeader, native_flow_message::NativeFlowButton,
        };

        let buttons: Vec<NativeFlowButton> = self
            .buttons
            .iter()
            .map(|b| NativeFlowButton {
                name: Some(b.action.as_str().to_string()),
                button_params_json: Some(b.params.to_string()),
            })
            .collect();

        let header = match &self.header {
            Some(h) => ProtoHeader {
                title: h.title.clone(),
                subtitle: h.subtitle.clone(),
                has_media_attachment: Some(h.has_media_attachment),
                ..Default::default()
            },
            None => ProtoHeader {
                has_media_attachment: Some(false),
                ..Default::default()
            },
        };

        let mut interactive = wa::message::InteractiveMessage {
            header: MessageField::some(header),
            body: MessageField::some(Body {
                text: Some(self.body.clone()),
            }),
            interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
                wa::message::interactive_message::NativeFlowMessage {
                    buttons,
                    message_params_json: self.message_params.as_ref().map(|v| v.to_string()),
                    ..Default::default()
                },
            ))),
            ..Default::default()
        };
        // Footer kosong tidak dikirim — `sendInteractiveCard` (socket.js:365)
        // hanya menyertakan footer kalau isinya ada.
        if let Some(f) = &self.footer {
            interactive.footer = MessageField::some(Footer {
                text: Some(f.clone()),
                ..Default::default()
            });
        }
        if let Some(c) = &self.context_info {
            interactive.context_info = MessageField::some(c.clone());
        }

        let inner = wa::Message {
            message_context_info: MessageField::some(wa::MessageContextInfo {
                device_list_metadata: MessageField::some(wa::DeviceListMetadata::default()),
                device_list_metadata_version: Some(2),
                ..Default::default()
            }),
            interactive_message: MessageField::some(interactive),
            ..Default::default()
        };

        // WAProto.proto:2095 — `viewOnceMessage` bertipe FutureProofMessage
        // { optional Message message = 1 }, bukan ViewOnceMessage.
        wa::Message {
            view_once_message: MessageField::some(wa::message::FutureProofMessage {
                message: MessageField::some(inner),
            }),
            ..Default::default()
        }
    }
}

/// Kartu teks tanpa tombol (`oktz-bin --kind card`). Kartu dengan tombol
/// dibangun lewat [`Card`].
pub fn build_card(body: &str, footer: Option<&str>) -> wa::Message {
    Card {
        body: body.to_string(),
        footer: footer.map(str::to_string),
        ..Default::default()
    }
    .to_message()
}

/// Kartu `single_select` tanpa section — hanya smoke test jalur encode
/// `single_select` (`oktz-bin --kind list`). List sebenarnya lewat
/// [`List::into_card`].
pub fn build_list() -> wa::Message {
    List::new("", []).into_card("", None).to_message()
}

/// Extra stanza node wajib supaya WA merender kartu native-flow.
pub fn biz_node() -> Node {
    let native_flow = NodeBuilder::new("native_flow")
        .attr("v", "9")
        .attr("name", "mixed")
        .build();
    let interactive = NodeBuilder::new("interactive")
        .attr("type", "native_flow")
        .attr("v", "1")
        .children([native_flow])
        .build();
    NodeBuilder::new("biz").children([interactive]).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use whatsapp_rust::wacore_binary::NodeContent;
    use whatsapp_rust::waproto::buffa::Message as _;

    /// Encode pesan ke byte protobuf — cara paling jujur mengecek "apa yang
    /// benar-benar terkirim", bukan cuma field yang kebetulan kita sentuh.
    fn payload(msg: &wa::Message) -> Vec<u8> {
        let mut buf = Vec::new();
        msg.encode(&mut buf);
        buf
    }

    fn contains(hay: &[u8], needle: &str) -> bool {
        hay.windows(needle.len()).any(|w| w == needle.as_bytes())
    }

    fn inner(msg: &wa::Message) -> &wa::Message {
        msg.view_once_message
            .as_option()
            .expect("kartu wajib dibungkus viewOnceMessage")
            .message
            .as_option()
            .expect("viewOnceMessage wajib berisi message")
    }

    fn interactive(msg: &wa::Message) -> &wa::message::InteractiveMessage {
        inner(msg)
            .interactive_message
            .as_option()
            .expect("wajib ada interactiveMessage")
    }

    fn native_flow(msg: &wa::Message) -> &wa::message::interactive_message::NativeFlowMessage {
        match interactive(msg)
            .interactive_message
            .as_ref()
            .expect("oneof wajib terisi")
        {
            ImOneof::NativeFlowMessage(nf) => nf,
            _ => panic!("oneof bukan NativeFlowMessage"),
        }
    }

    /// Teks demo yang tadinya dipaku di dalam builder.
    const DEMO: [&str; 8] = [
        "OKE",
        "GAGAW",
        "test_oke",
        "test_gagaw",
        "Pilih Mode",
        "test_basic",
        "test_hidetag",
        "footer list",
    ];

    #[test]
    fn build_card_payload_has_no_hardcoded_demo_text() {
        let bytes = payload(&build_card("body dari pemanggil", Some("footer dari pemanggil")));
        for demo in DEMO {
            assert!(
                !contains(&bytes, demo),
                "build_card masih mengirim teks demo {demo:?}"
            );
        }
        assert!(contains(&bytes, "body dari pemanggil"));
        assert!(contains(&bytes, "footer dari pemanggil"));
    }

    #[test]
    fn build_list_payload_has_no_hardcoded_demo_text() {
        let bytes = payload(&build_list());
        for demo in DEMO {
            assert!(
                !contains(&bytes, demo),
                "build_list masih mengirim teks demo {demo:?}"
            );
        }
    }

    #[test]
    fn card_buttons_come_from_caller() {
        let msg = Card::new("pilih satu")
            .footer("footer uji")
            .button(Button::quick_reply("Ya", "cmd_ya"))
            .button(Button::copy("Salin", "ABC123"))
            .button(Button::url("Buka", "https://example.com"))
            .to_message();

        let flow = native_flow(&msg);
        assert_eq!(flow.buttons.len(), 3);
        assert_eq!(flow.buttons[0].name.as_deref(), Some("quick_reply"));
        assert_eq!(flow.buttons[1].name.as_deref(), Some("cta_copy"));
        assert_eq!(flow.buttons[2].name.as_deref(), Some("cta_url"));

        let params = |i: usize| -> Value {
            serde_json::from_str(flow.buttons[i].button_params_json.as_deref().unwrap()).unwrap()
        };
        assert_eq!(params(0), json!({"display_text": "Ya", "id": "cmd_ya"}));
        assert_eq!(
            params(1),
            json!({"display_text": "Salin", "copy_code": "ABC123"})
        );
        assert_eq!(
            params(2),
            json!({"display_text": "Buka", "url": "https://example.com"})
        );
        assert_eq!(
            interactive(&msg).body.text.as_deref(),
            Some("pilih satu")
        );
        assert_eq!(interactive(&msg).footer.text.as_deref(), Some("footer uji"));
    }

    #[test]
    fn list_sections_come_from_caller() {
        let list = List::new("Menu Utama", [
            Section::new("MODE").highlight_label("baru").row(
                Row::new("Basic", "cmd_basic").description("tanpa tag"),
            ),
            Section::new("PENGATURAN").row(Row::new("Hidetag", "cmd_hidetag").header("H")),
        ]);
        let msg = list.into_card("pilih mode", Some("footer")).to_message();

        let flow = native_flow(&msg);
        assert_eq!(flow.buttons.len(), 1);
        assert_eq!(flow.buttons[0].name.as_deref(), Some("single_select"));
        let params: Value =
            serde_json::from_str(flow.buttons[0].button_params_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            params,
            json!({
                "title": "Menu Utama",
                "sections": [
                    {
                        "title": "MODE",
                        "highlight_label": "baru",
                        "rows": [
                            {"title": "Basic", "description": "tanpa tag", "id": "cmd_basic"}
                        ]
                    },
                    {
                        "title": "PENGATURAN",
                        "rows": [{"title": "Hidetag", "id": "cmd_hidetag", "header": "H"}]
                    }
                ]
            })
        );
        assert_eq!(interactive(&msg).body.text.as_deref(), Some("pilih mode"));
    }

    #[test]
    fn card_keeps_proven_view_once_wrapper_and_context_v2() {
        let msg = Card::new("x").button(Button::quick_reply("Ya", "y")).to_message();

        // WA hanya render kartu nativeFlow di dalam viewOnceMessage +
        // deviceListMetadata v2 (rich-messages.js:468). Bentuk legacy
        // ListMessage/interactiveButtons tidak dirender sama sekali.
        assert!(msg.view_once_message.is_set());
        let inner_msg = inner(&msg);
        assert!(inner_msg.list_message.is_unset(), "ListMessage legacy");
        assert!(inner_msg.interactive_message.is_set());

        let ctx = inner_msg.message_context_info.as_option().unwrap();
        assert!(ctx.device_list_metadata.is_set());
        assert_eq!(ctx.device_list_metadata_version, Some(2));
    }

    #[test]
    fn empty_footer_is_omitted_not_sent_as_empty_string() {
        let msg = Card::new("x").button(Button::quick_reply("Ya", "y")).to_message();
        assert!(
            interactive(&msg).footer.as_option().is_none(),
            "footer kosong tidak boleh dikirim sebagai string kosong"
        );
    }

    #[test]
    fn message_params_reach_native_flow() {
        let msg = Card::new("x")
            .message_params(json!({"bottom_sheet": {"list_title": "Pilih"}}))
            .button(Button::quick_reply("Ya", "y"))
            .to_message();
        let raw = native_flow(&msg).message_params_json.as_deref().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(raw).unwrap(),
            json!({"bottom_sheet": {"list_title": "Pilih"}})
        );
    }

    #[test]
    fn biz_node_is_native_flow_extra() {
        let node = biz_node();
        assert_eq!(node.tag, "biz");
        let Some(NodeContent::Nodes(kids)) = &node.content else {
            panic!("biz harus punya anak node");
        };
        assert_eq!(kids.len(), 1);
        let interactive_node = &kids[0];
        assert_eq!(interactive_node.tag, "interactive");
        assert_eq!(interactive_node.attrs.get("type").unwrap().as_str(), "native_flow");
        assert_eq!(interactive_node.attrs.get("v").unwrap().as_str(), "1");
        let Some(NodeContent::Nodes(flow)) = &interactive_node.content else {
            panic!("interactive harus punya native_flow");
        };
        assert_eq!(flow[0].tag, "native_flow");
        assert_eq!(flow[0].attrs.get("v").unwrap().as_str(), "9");
        assert_eq!(flow[0].attrs.get("name").unwrap().as_str(), "mixed");
    }
}
