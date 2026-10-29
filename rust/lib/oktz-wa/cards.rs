//! Pengiriman kartu interaktif WhatsApp (native flow).
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
//! Bentuk legacy (`ListMessage`, `interactiveButtons` telanjang) relay-nya
//! sukses tapi WA membuangnya diam-diam — tidak ada yang muncul di chat.
//! Jangan dipakai.
//!
//! Model kartyanya ada di `oktz-plugin-api::kartu` (supaya plugin bisa
//! menyusunnya); file ini hanya menerjemahkan model itu ke proto WA.

use oktz_plugin_api::kartu::{Kartu, Tombol};
use serde_json::Value;
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;
use whatsapp_rust::{NodeBuilder, wacore_binary::Node};

/// Header kartu dengan media gambar yang sudah ter-upload.
///
/// `header_image` = `imageMessage` hasil upload; `None` = header tanpa
/// gambar (judul tetap tampil). JS: `interactiveMessage.header.imageMessage`.
fn header_proto(
    kartu: &Kartu,
    header_image: Option<wa::message::ImageMessage>,
) -> wa::message::interactive_message::Header {
    use wa::message::interactive_message::header::Media;

    let (title, subtitle) = match &kartu.header {
        Some(h) => (h.title.clone(), h.subtitle.clone()),
        None => (None, None),
    };
    let media = header_image.map(|img| Media::ImageMessage(Box::new(img)));
    wa::message::interactive_message::Header {
        title,
        subtitle,
        // WA hanya menampilkan sampul kalau flag ini true; JS selalu
        // mengirimnya saat header ada media (`menu.js:598`).
        has_media_attachment: Some(media.is_some()),
        media,
        ..Default::default()
    }
}

/// Kartu native-flow → proto WA.
///
/// `header_image` = `imageMessage` yang sudah di-upload; `None` = tanpa
/// gambar header. `context_info` diisi pipeline (mention + quote).
pub fn kartu_to_message(
    kartu: &Kartu,
    header_image: Option<wa::message::ImageMessage>,
    context_info: Option<wa::ContextInfo>,
) -> wa::Message {
    use wa::message::interactive_message::{
        Body, Footer, native_flow_message::NativeFlowButton,
    };

    let buttons: Vec<NativeFlowButton> = kartu
        .buttons
        .iter()
        .map(|b: &Tombol| NativeFlowButton {
            name: Some(b.aksi.as_str().to_string()),
            button_params_json: Some(b.params.json()),
        })
        .collect();

    let mut interactive = wa::message::InteractiveMessage {
        header: MessageField::some(header_proto(kartu, header_image)),
        body: MessageField::some(Body {
            text: Some(kartu.body.clone()),
        }),
        interactive_message: Some(ImOneof::NativeFlowMessage(Box::new(
            wa::message::interactive_message::NativeFlowMessage {
                buttons,
                message_params_json: kartu.params.as_ref().map(|p| p.json()),
                ..Default::default()
            },
        ))),
        ..Default::default()
    };
    // Footer kosong tidak dikirim — `sendInteractiveCard` (socket.js:365)
    // hanya menyertakan footer kalau isinya ada.
    if let Some(f) = &kartu.footer {
        interactive.footer = MessageField::some(Footer {
            text: Some(f.clone()),
            ..Default::default()
        });
    }
    if let Some(c) = context_info {
        interactive.context_info = MessageField::some(c);
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

/// Kartu teks tanpa tombol (`oktz --kind card`). Kartu dengan tombol
/// dibangun lewat [`kartu_to_message`].
pub fn build_card(body: &str, footer: Option<&str>) -> wa::Message {
    let mut kartu = Kartu::baru(body);
    if let Some(f) = footer {
        kartu = kartu.footer(f);
    }
    kartu_to_message(&kartu, None, None)
}

/// Kartu `single_select` tanpa section — hanya smoke test jalur encode
/// `single_select` (`oktz --kind list`).
pub fn build_list() -> wa::Message {
    let kartu = Kartu::baru("").tombol(oktz_plugin_api::kartu::Tombol::pilih(
        &oktz_plugin_api::kartu::Daftar::baru(""),
    ));
    kartu_to_message(&kartu, None, None)
}

/// Extra stanza node wajib supaya WA merender kartu native-flow.
///
/// Bentuknya meniru `onigis` (`lib/WABinary/generic-utils.js:139`
/// `getAdditionalNode`): `biz` dengan `engagement` + `interactive/native_flow`.
/// Tanpa node ini kartu tidak muncul di chat.
pub fn biz_node() -> Node {
    let engagement = NodeBuilder::new("engagement")
        .attr("customer_service_state", "open")
        .attr("conversation_state", "open")
        .build();
    let native_flow = NodeBuilder::new("native_flow")
        .attr("v", "9")
        .attr("name", "mixed")
        .build();
    let interactive = NodeBuilder::new("interactive")
        .attr("type", "native_flow")
        .attr("v", "1")
        .children([native_flow])
        .build();
    // `privacy_mode_ts` = sekarang - 77_980_457 detik (nilai onigis).
    let privacy_mode_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 - 77_980_457)
        .unwrap_or_default();
    NodeBuilder::new("biz")
        .attr("actual_actors", "2")
        .attr("host_storage", "2")
        .attr("privacy_mode_ts", privacy_mode_ts.to_string())
        .children([engagement, interactive])
        .build()
}

/// `messageParamsJson` — `bottom_sheet`, `limited_time_offer`, dan
/// `tap_target_configuration` (JS `menu.js:755`).
pub fn params_bottom_sheet(
    list_title: impl Into<String>,
    button_title: impl Into<String>,
) -> Value {
    serde_json::json!({
        "bottom_sheet": {
            "in_thread_buttons_limit": 2,
            "divider_indices": [1, 2, 3, 4, 5, 999],
            "list_title": list_title.into(),
            "button_title": button_title.into(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::kartu::{Baris, Bagian, Daftar};
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
    fn kartu_menjadi_view_once_dengan_device_list_metadata() {
        let msg = build_card("body", Some("footer"));
        let view = msg.view_once_message.as_option().unwrap();
        assert!(view.message.as_option().is_some());
        let mci = inner(&msg)
            .message_context_info
            .as_option()
            .expect("wajib ada messageContextInfo");
        assert_eq!(mci.device_list_metadata_version, Some(2));
        assert!(mci.device_list_metadata.as_option().is_some());
    }

    #[test]
    fn tombol_dari_model_masuk_sebagai_native_flow_button() {
        let kartu = Kartu::baru("pilih satu")
            .footer("footer uji")
            .tombol(Tombol::balas_cepat("Ya", "cmd_ya"))
            .tombol(Tombol::salin("Salin", "id", "ABC123"))
            .tombol(Tombol::buka_url("Buka", "https://example.com"));
        let msg = kartu_to_message(&kartu, None, None);

        let flow = native_flow(&msg);
        assert_eq!(flow.buttons.len(), 3);
        assert_eq!(flow.buttons[0].name.as_deref(), Some("quick_reply"));
        assert_eq!(flow.buttons[1].name.as_deref(), Some("cta_copy"));
        assert_eq!(flow.buttons[2].name.as_deref(), Some("cta_url"));
        assert!(flow.buttons[0]
            .button_params_json
            .as_deref()
            .unwrap()
            .contains("cmd_ya"));
        assert!(flow.buttons[2]
            .button_params_json
            .as_deref()
            .unwrap()
            .contains("merchant_url"));
    }

    #[test]
    fn single_select_membawa_sections_dan_rows() {
        let daftar = Daftar::baru("🍃 List Menu")
            .bagian(
                Bagian::baru("pilihan")
                    .baris(Baris::baru("🏠 main", ".menucat main").description("12 command")),
            )
            .icon("DEFAULT");
        let kartu = Kartu::baru("body").tombol(Tombol::pilih(&daftar));
        let msg = kartu_to_message(&kartu, None, None);
        let params = native_flow(&msg).buttons[0]
            .button_params_json
            .as_deref()
            .unwrap()
            .to_string();
        let json: Value = serde_json::from_str(&params).unwrap();
        assert_eq!(json["title"], "🍃 List Menu");
        assert_eq!(json["icon"], "DEFAULT");
        assert_eq!(json["sections"][0]["rows"][0]["id"], ".menucat main");
        assert_eq!(json["sections"][0]["rows"][0]["description"], "12 command");
    }

    #[test]
    fn message_params_json_dikirim_apa_adanya() {
        let kartu = Kartu::baru("body").params(json!({"bottom_sheet": {"list_title": "x"}}));
        let msg = kartu_to_message(&kartu, None, None);
        let params = native_flow(&msg).message_params_json.as_deref().unwrap();
        assert!(params.contains("bottom_sheet"), "{params}");
    }

    #[test]
    fn mention_dari_kartu_masuk_context_info() {
        let kartu = Kartu::baru("body").mention("628@s.whatsapp.net");
        let ci = wa::ContextInfo {
            mentioned_jid: vec!["628@s.whatsapp.net".to_string()],
            ..Default::default()
        };
        let msg = kartu_to_message(&kartu, None, Some(ci));
        let ctx = interactive(&msg).context_info.as_option().unwrap();
        assert_eq!(ctx.mentioned_jid, vec!["628@s.whatsapp.net".to_string()]);
    }

    #[test]
    fn header_tanpa_media_tidak_menandai_lampiran() {
        let kartu = Kartu::baru("body").header(oktz_plugin_api::kartu::KartuHeader::baru().subjudul("Bot"));
        let msg = kartu_to_message(&kartu, None, None);
        let header = interactive(&msg).header.as_option().unwrap();
        assert_eq!(header.subtitle.as_deref(), Some("Bot"));
        assert_eq!(header.has_media_attachment, Some(false));
        assert!(header.media.is_none());
    }

    #[test]
    fn header_dengan_gambar_menandai_lampiran() {
        let img = wa::message::ImageMessage {
            url: Some("https://cdn.example/img.jpg".into()),
            mimetype: Some("image/jpeg".into()),
            ..Default::default()
        };
        let kartu = Kartu::baru("body").header(
            oktz_plugin_api::kartu::KartuHeader::baru()
                .judul("")
                .subjudul("Bot")
                .media(oktz_plugin_api::MediaSource::Path {
                    path: "assets/image/foto.jpeg".into(),
                    mimetype: None,
                }),
        );
        let msg = kartu_to_message(&kartu, Some(img), None);
        let header = interactive(&msg).header.as_option().unwrap();
        assert_eq!(header.has_media_attachment, Some(true));
        assert!(header.media.is_some());
    }

    #[test]
    fn biz_node_mirip_onigis() {
        let node = biz_node();
        assert_eq!(node.tag, "biz");
        let children = match node.content.as_ref().expect("biz punya anak") {
            NodeContent::Nodes(children) => children,
            lain => panic!("anak biz harus node, bukan {lain:?}"),
        };
        let tags: Vec<&str> = children.iter().map(|n| n.tag.as_ref()).collect();
        assert_eq!(tags, vec!["engagement", "interactive"]);
    }
}
