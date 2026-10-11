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

pub fn build_list() -> wa::Message {
    let rows = vec![
        wa::message::list_message::Row {
            title: Some("JPM Basic".into()),
            description: Some("Broadcast semua grup".into()),
            row_id: Some("test_basic".into()),
        },
        wa::message::list_message::Row {
            title: Some("JPM Hidetag".into()),
            description: Some("Tag tersembunyi".into()),
            row_id: Some("test_hidetag".into()),
        },
    ];
    let inner = wa::Message {
        message_context_info: MessageField::some(wa::MessageContextInfo {
            device_list_metadata: MessageField::some(wa::DeviceListMetadata::default()),
            device_list_metadata_version: Some(2),
            ..Default::default()
        }),
        list_message: MessageField::some(wa::message::ListMessage {
            title: Some("Pilih Mode".into()),
            description: Some("List message dari Rust".into()),
            button_text: Some("Buka".into()),
            footer_text: Some("footer list".into()),
            list_type: Some(wa::message::list_message::ListType::SINGLE_SELECT),
            sections: vec![wa::message::list_message::Section {
                title: Some("MODE".into()),
                rows,
            }],
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
