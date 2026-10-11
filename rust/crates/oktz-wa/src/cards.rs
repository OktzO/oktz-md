use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::{NodeBuilder, wacore_binary::Node};

pub fn build_card(body: &str, footer: Option<&str>) -> wa::Message {
    use wa::message::interactive_message::{
        Body, Footer, Header, native_flow_message::NativeFlowButton,
    };
    use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;

    let buttons = vec![
        NativeFlowButton {
            name: Some("quick_reply".into()),
            button_params_json: Some(r#"{"display_text":"OKE","id":"test_oke"}"#.into()),
        },
        NativeFlowButton {
            name: Some("quick_reply".into()),
            button_params_json: Some(r#"{"display_text":"GAGAW","id":"test_gagaw"}"#.into()),
        },
    ];

    let inner = wa::Message {
        message_context_info: MessageField::some(wa::MessageContextInfo {
            device_list_metadata: MessageField::some(wa::DeviceListMetadata::default()),
            device_list_metadata_version: Some(2),
            ..Default::default()
        }),
        interactive_message: MessageField::some(wa::message::InteractiveMessage {
            header: MessageField::some(Header {
                has_media_attachment: Some(false),
                ..Default::default()
            }),
            body: MessageField::some(Body {
                text: Some(body.to_string()),
            }),
            footer: MessageField::some(Footer {
                text: Some(footer.unwrap_or("").to_string()),
                ..Default::default()
            }),
            interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
                wa::message::interactive_message::NativeFlowMessage {
                    buttons: buttons.clone(),
                    ..Default::default()
                },
            ))),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Cocok dengan Onigis WAProto.proto:2095 — viewOnceMessage bertipe
    // FutureProofMessage { optional Message message = 1 }, bukan ViewOnceMessage.
    wa::Message {
        view_once_message: MessageField::some(wa::message::FutureProofMessage {
            message: MessageField::some(inner),
        }),
        ..Default::default()
    }
}

// List native-flow: tombol single_select dengan sections/rows. Pola ini yang
// dipakai plugins/jpm/jpm.js:302 dan terbukti jalan di produksi; field
// ListMessage (proto) adalah bentuk legacy yang WA tidak render.
pub fn build_list() -> wa::Message {
    use wa::message::interactive_message::{
        Body, Footer, Header, native_flow_message::NativeFlowButton,
    };
    use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;

    let params = serde_json::json!({
        "title": "Pilih Mode",
        "sections": [{
            "title": "MODE",
            "rows": [
                {"title": "JPM Basic", "description": "Kirim ke semua grup tanpa tag", "id": "test_basic"},
                {"title": "JPM Hidetag", "description": "Tag tersembunyi", "id": "test_hidetag"}
            ]
        }]
    })
    .to_string();

    let buttons = vec![NativeFlowButton {
        name: Some("single_select".into()),
        button_params_json: Some(params),
    }];

    let inner = wa::Message {
        message_context_info: MessageField::some(wa::MessageContextInfo {
            device_list_metadata: MessageField::some(wa::DeviceListMetadata::default()),
            device_list_metadata_version: Some(2),
            ..Default::default()
        }),
        interactive_message: MessageField::some(wa::message::InteractiveMessage {
            header: MessageField::some(Header {
                has_media_attachment: Some(false),
                ..Default::default()
            }),
            body: MessageField::some(Body {
                text: Some("List dari produksi Rust".into()),
            }),
            footer: MessageField::some(Footer {
                text: Some("footer list".into()),
                ..Default::default()
            }),
            interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
                wa::message::interactive_message::NativeFlowMessage {
                    buttons: buttons.clone(),
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
