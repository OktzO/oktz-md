//! Bukti bahwa router tersambung ke registry plugin sungguhan.
//!
//! Unit test `oktz-wa` memakai `TestPlugin` buatan sendiri, jadi ia membuktikan
//! rantai (event → antrean → run → route → kirim) tapi TIDAK membuktikan plugin
//! yang benar yang dipanggil. `oktz-bin` satu-satunya crate yang melihat
//! `oktz-plugins` dan `oktz-wa` sekaligus, jadi ia bisa menjalankan `Pipeline`
//! asli dengan registry asli.
//!
//! Yang dibuktikan: `.benefitpartner` memanggil
//! `oktz-plugins/src/info/benefitpartner.rs`. Balasannya memuat `BOT_NAME`
//! yang di-hardcode di file itu, dan teks itu tidak ada di crate lain.
//!
//! Batasnya jelas: `MessageSender` masih tiruan, jadi soket WA belum
//! tersentuh. Yang terbukti everything sebelum `client.send_message`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use oktz_store::json_store::JsonStore;
use oktz_wa::{
    InboundMessage, Jid, Message, MessageInfo, MessageSender, MessageSource, Node, Outcome, Pipeline,
    RosterSource, SendBox, SendBoxRoster, StoreContext,
};
use oktz_router::command::ParseOptions;

/// Tanpa rujukan ini `oktz_plugins` tidak ter-link ke binari test dan
/// `find()` mengembalikan `None` untuk semua command — bukan karena pluginnya
/// hilang, tapi karena tidak ada yang mendaftarkannya. Sama seperti jebakan yang
/// dijaga `plugin_link.rs` untuk binari `oktz`.
const _: fn() = || {
    let _ = oktz_plugins::info::benefitpartner::benefitpartner;
};

const GROUP: &str = "120363@g.us";
const MEMBER: &str = "628111@s.whatsapp.net";
const BOT: &str = "628999@s.whatsapp.net";

/// Roster statis: bot admin, satu member biasa. Tanpa ini gate grup tidak
/// tahu bot punya hak hapus, jadi `DeleteAndReply` turun jadi peringatan
/// "bukan admin" dan pesan TIDAK terhapus.
struct Roster;

impl RosterSource for Roster {
    fn roster(&self, _chat: &str) -> SendBoxRoster<'_> {
        Box::pin(async {
            Ok(oktz_plugin_api::registry::GroupRoster {
                participants: vec![
                    oktz_plugin_api::registry::Participant {
                        jid: BOT.to_string(),
                        is_admin: true,
                        ..Default::default()
                    },
                    oktz_plugin_api::registry::Participant {
                        jid: MEMBER.to_string(),
                        ..Default::default()
                    },
                ],
                is_bot_admin: true,
            })
        })
    }
}

static SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct Recorder {
    sent: Mutex<Vec<(String, String)>>,
    deleted: Mutex<Vec<(String, String, String)>>,
}

impl Recorder {
    fn texts(&self) -> Vec<String> {
        self.sent.lock().unwrap().iter().map(|(_, b)| b.clone()).collect()
    }
}

impl MessageSender for Recorder {
    fn send(
        &self,
        to: &str,
        message: Message,
        _extra_nodes: Vec<Node>,
    ) -> SendBox<'_> {
        let body = message
            .conversation
            .clone()
            .or_else(|| {
                message
                    .extended_text_message
                    .as_option()
                    .and_then(|e| e.text.clone())
            })
            .unwrap_or_default();
        self.sent.lock().unwrap().push((to.to_string(), body));
        Box::pin(async { Ok(()) })
    }

    fn delete_message(&self, chat: &str, id: &str, sender: &str) -> SendBox<'_> {
        self.deleted
            .lock()
            .unwrap()
            .push((chat.to_string(), id.to_string(), sender.to_string()));
        Box::pin(async { Ok(()) })
    }
}

fn store_with(body: &str) -> Arc<JsonStore> {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oktz-live-route-{n}.json"));
    std::fs::write(&p, body).unwrap();
    Arc::new(JsonStore::open(&p).unwrap())
}

fn inbound(body: &str, id: &str) -> InboundMessage {
    InboundMessage::builder()
        .message(Arc::new(Message {
            conversation: Some(body.to_string()),
            ..Default::default()
        }))
        .info(Arc::new(MessageInfo {
            source: MessageSource {
                chat: GROUP.parse::<Jid>().unwrap(),
                sender: MEMBER.parse::<Jid>().unwrap(),
                is_from_me: false,
                is_group: true,
                ..Default::default()
            },
            id: id.to_string(),
            ..Default::default()
        }))
        .build()
}

#[tokio::test]
async fn a_real_inbound_command_reaches_the_real_plugin_registry() {
    let rec = Arc::new(Recorder::default());
    let p = Pipeline::new(
        StoreContext::new(store_with(r#"{"settings":{},"users":{}}"#)),
        rec.clone(),
        BOT,
        ParseOptions::default(),
    );

    let out = p.process(&inbound(".benefitpartner", "R1"), 1_000).await.unwrap();
    assert_eq!(out, Outcome::Sent, "command nyata harus menjalankan plugin");

    let texts = rec.texts();
    assert_eq!(texts.len(), 1, "tepat satu balasan: {texts:?}");
    assert!(
        texts[0].starts_with("🤝 *BENEFIT PARTNER*"),
        "balasan bukan dari plugin benefitpartner: {:?}",
        texts[0]
    );
    assert!(
        texts[0].contains("Oktz Bot"),
        "`BOT_NAME` di-hardcode di info/benefitpartner.rs tidak muncul: {:?}",
        texts[0]
    );
    assert_eq!(
        rec.sent.lock().unwrap()[0].0, GROUP,
        "balasan harus dikirim ke chat asal"
    );
}

#[tokio::test]
async fn an_unknown_command_is_not_routed_anywhere() {
    let rec = Arc::new(Recorder::default());
    let p = Pipeline::new(
        StoreContext::new(store_with(r#"{"settings":{},"users":{}}"#)),
        rec.clone(),
        BOT,
        ParseOptions::default(),
    );

    let out = p
        .process(&inbound(".zzzztidakadaregistry", "R2"), 1_000)
        .await
        .unwrap();
    assert_eq!(out, Outcome::NotACommand);
    assert!(rec.texts().is_empty(), "tidak boleh ada balasan: {:?}", rec.texts());
}

#[tokio::test]
async fn group_protection_stops_the_message_before_any_plugin_runs() {
    let rec = Arc::new(Recorder::default());
    let p = Pipeline::new(
        StoreContext::new(store_with(
            r#"{"settings":{},"users":{},"groups":{"120363@g.us":{"antilink":"on"}}}"#,
        )),
        rec.clone(),
        BOT,
        ParseOptions::default(),
    )
    .with_roster_source(Arc::new(Roster));

    let m = inbound(".benefitpartner https://contoh.com/x", "R3");
    let out = p.process(&m, 1_000).await.unwrap();
    assert_eq!(out, Outcome::Sent, "gate membalas peringatan sendiri");

    let texts = rec.texts();
    assert_eq!(texts.len(), 1, "tepat satu balasan: {texts:?}");
    assert!(
        texts[0].contains("Antilink"),
        "balasan harus peringatan antilink: {:?}",
        texts[0]
    );
    assert!(
        !texts[0].contains("Oktz Bot"),
        "plugin harus TIDAK jalan: {:?}",
        texts[0]
    );
    assert_eq!(
        *rec.deleted.lock().unwrap(),
        vec![(GROUP.to_string(), "R3".to_string(), MEMBER.to_string())],
        "pesan link harus dihapus untuk semua orang"
    );
}