//! Bukti bahwa `.menu` benar-benar terkirim sebagai KARTU native-flow.
//!
//! `tests/live_route.rs` membuktikan command mencapai plugin sungguhan.
//! Test ini melanjutkan dari situ: balasan `.menu` harus sampai ke WA
//! sebagai `interactiveMessage` (dibungkus `viewOnceMessage`) plus extra
//! stanza node `biz` — bukan teks biasa. Tanpa node `biz`, WA diam-diam
//! membuang kartu (pola `onigis/lib/WABinary/generic-utils.js:139`).
//!
//! Batas: `MessageSender` di sini tiruan, jadi soket WA belum tersentuh.
//! Yang terbukti: pipeline merakit kartu native-flow yang benar dan
//! mengirimnya ke chat asal.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use oktz_router::command::ParseOptions;
use oktz_store::json_store::JsonStore;
use oktz_wa::{
    ImOneof, InboundMessage, Jid, Message, MessageInfo, MessageSender, MessageSource, Node, Outcome,
    Pipeline, SendBox, StoreContext,
};

// Tanpa rujukan ini `oktz_plugins` tidak ter-link ke binari test dan
// `find("menu")` mengembalikan `None` — bukan karena pluginnya hilang, tapi
// karena tidak ada yang mendaftarkannya.
const _: fn() = || {
    let _ = oktz_plugins::main::menu::menu;
};

const CHAT: &str = "628111@s.whatsapp.net";
const BOT: &str = "628999@s.whatsapp.net";

static SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct Recorder {
    terkirim: Mutex<Vec<(String, Message, Vec<Node>)>>,
}

impl MessageSender for Recorder {
    fn send(&self, to: &str, message: Message, extra_nodes: Vec<Node>) -> SendBox<'_> {
        self.terkirim
            .lock()
            .unwrap()
            .push((to.to_string(), message, extra_nodes));
        Box::pin(async { Ok(()) })
    }
}

fn store_kosong() -> Arc<JsonStore> {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oktz-menu-card-{n}.json"));
    std::fs::write(&p, r#"{"settings":{},"users":{}}"#).unwrap();
    Arc::new(JsonStore::open(&p).unwrap())
}

/// Config seperti produksi: nama bot terisi dan nomor owner ada — tanpa
/// nomor owner, tombol "Owner Kami" jadi `quick_reply` (bukan `cta_url`).
fn config_produksi() -> oktz_plugin_api::config::Runtime {
    oktz_plugin_api::config::Runtime {
        bot: oktz_plugin_api::config::BotInfo {
            name: "Oktz Bot".into(),
            version: "3.3.1".into(),
            developer: "Zann".into(),
            website: String::new(),
        },
        owner: oktz_plugin_api::config::Owner {
            name: "Oktz".into(),
            number: vec!["6285608953677".into()],
        },
        ..Default::default()
    }
}

fn inbound(body: &str) -> InboundMessage {
    InboundMessage::builder()
        .message(Arc::new(Message {
            conversation: Some(body.to_string()),
            ..Default::default()
        }))
        .info(Arc::new(MessageInfo {
            source: MessageSource {
                chat: CHAT.parse::<Jid>().unwrap(),
                sender: CHAT.parse::<Jid>().unwrap(),
                is_from_me: false,
                is_group: false,
                ..Default::default()
            },
            id: "M1".to_string(),
            ..Default::default()
        }))
        .build()
}

#[tokio::test]
async fn menu_dikirim_sebagai_kartu_native_flow_dengan_node_biz() {
    let rec = Arc::new(Recorder::default());
    let p = Pipeline::new(
        StoreContext::new(store_kosong()),
        rec.clone(),
        BOT,
        ParseOptions::default(),
    )
    .with_config(config_produksi());

    let out = p.process(&inbound(".menu"), 1_000).await.unwrap();
    assert_eq!(out, Outcome::Sent, ".menu harus dibalas");

    let terkirim = rec.terkirim.lock().unwrap().clone();
    assert_eq!(terkirim.len(), 1, "tepat satu balasan");
    let (chat, pesan, node) = &terkirim[0];
    assert_eq!(chat, CHAT, "balasan harus dikirim ke chat asal");

    // Node `biz` wajib; tanpa itu WA membuang kartu.
    let tag_biz = node.iter().any(|n| n.tag == "biz");
    assert!(tag_biz, "node biz wajib ada: {node:?}");

    // Kartu dibungkus `viewOnceMessage` (tipe `FutureProofMessage`).
    let view = pesan.view_once_message.as_option().expect("viewOnceMessage");
    let inner = view.message.as_option().expect("isi viewOnceMessage");
    let im = inner.interactive_message.as_option().expect("interactiveMessage");
    let body = im.body.as_option().and_then(|b| b.text.clone()).unwrap_or_default();
    assert!(body.contains("Hello Brother"), "body kartu: {body}");
    assert!(body.contains("BOT INFORMATION"), "body kartu: {body}");

    let Some(ImOneof::NativeFlowMessage(native)) = im.interactive_message.as_ref() else {
        panic!("harus native flow");
    };
    let aksi: Vec<&str> = native
        .buttons
        .iter()
        .map(|b| b.name.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(aksi, vec!["cta_url", "quick_reply"], "tombol kartu: {aksi:?}");

    // `deviceListMetadata` + versi 2 = syarat WA merender kartu.
    let mci = inner
        .message_context_info
        .as_option()
        .expect("messageContextInfo");
    assert_eq!(mci.device_list_metadata_version, Some(2));
    assert!(mci.device_list_metadata.as_option().is_some());
}
