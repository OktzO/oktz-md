// S3: kirim kartu tombol interaktif memakai pola yang terbukti jalan di bot JS
// (src/lib/socket.js:358-420):
//   viewOnceMessage wrapper -> interactiveMessage -> nativeFlowMessage
//   -> messageContextInfo.deviceListMetadata(+version 2)
//   -> additional stanza node: biz > interactive > native_flow
// Tanpa wrapper + additional node, WhatsApp TIDAK me-render kartunya.
use anyhow::Result;
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::NodeBuilder;
use whatsapp_rust::wacore_binary::Node;

use crate::creds_map;
use crate::rss;

fn build_card(body: &str, footer: Option<&str>) -> wa::Message {
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

fn build_list() -> wa::Message {
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

fn biz_node() -> Node {
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

pub async fn run(creds_path: &str, to: &str, hold_secs: u64, kind: &str) -> Result<()> {
    let raw = std::fs::read_to_string(creds_path)?;
    let (_m, c) = creds_map::inspect_creds(&raw)?;
    let device = crate::connect::build_device_pub(&c).await?;

    let db = std::env::temp_dir().join("oktz-spike-buttons.db");
    let _ = std::fs::remove_file(&db);
    let store = SqliteStore::new(db.to_str().unwrap()).await?;
    store.save_device_data_for_device(1, &device).await?;

    let connected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let c1 = connected.clone();
    let target = to.to_string();
    let kind = kind.to_string();

    let bot = Bot::builder()
        .with_backend(store)
        .on_connected(move |client| {
            let c1 = c1.clone();
            let target = target.clone();
            let kind = kind.clone();
            async move {
                println!(
                    "{}",
                    serde_json::json!({"phase":"CONNECTED","rss_kb":rss::current_kb()})
                );
                c1.store(true, std::sync::atomic::Ordering::SeqCst);

                let jid: Jid = match target.parse() {
                    Ok(j) => j,
                    Err(e) => {
                        println!("{}", serde_json::json!({"phase":"ERR","msg":format!("JID: {e}")}));
                        return;
                    }
                };

                let opts = SendOptions::default().with_extra_stanza_nodes(vec![biz_node()]);
                let msg = if kind.as_str() == "list" {
                    build_list()
                } else {
                    build_card("Tombol test dari Rust — apakah muncul?", Some("footer test"))
                };

                match client.send_message_with_options(jid.clone(), msg, opts).await {
                    Ok(s) => println!(
                        "{}",
                        serde_json::json!({
                            "phase":"BUTTONS_SENT","to": jid.to_string(),
                            "message_id": s.message_id, "rss_kb": rss::current_kb()
                        })
                    ),
                    Err(e) => println!(
                        "{}",
                        serde_json::json!({"phase":"ERR","msg": format!("{e}")})
                    ),
                }
            }
        })
        .build()
        .await?;

    let handle = bot.spawn();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    println!(
        "{}",
        serde_json::json!({
            "phase":"done",
            "connected": connected.load(std::sync::atomic::Ordering::SeqCst),
            "rss_kb": rss::current_kb()
        })
    );
    handle.shutdown().await;
    Ok(())
}