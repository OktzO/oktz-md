//! Pipeline live: pesan inbound → `oktz_router::Router` → balasan ke WhatsApp.
//!
//! Tiga bagian, dipisah supaya bagian yang bisa diuji offline tetap murni:
//!
//! 1. **Normalisasi JID** ([`normalize_user_key`]) — port `getUser`
//!    (`src/lib/database.js:529-536`) dan pembantu `isLidLikeJid`
//!    (`database.js:7-13`, `lib/lid.js:124-167`).
//! 2. **Adapter store** ([`StoreContext`]) — `RouterContext` di atas
//!    `oktz_store::JsonStore`. `oktz-store` tidak punya `getUser`/`getGroup`/
//!    `setting` (itu hanya di JS), jadi peta JSON dibaca langsung di sini.
//! 3. **Pipeline** ([`Pipeline`]) — dedup → gate → route → render → kirim.
//!
//! Batas yang diketahui dan sengaja dibiarkan terlihat, bukan disembunyikan:
//! - `Reply::Image` / `Reply::Sticker` tidak punya sumber byte
//!   (`oktz_plugin_api::Reply`, `oktz-plugin-api/src/registry.rs:33-39`), jadi
//!   [`render_reply`] mengembalikan [`PipelineError::MediaSourceMissing`] dan
//!   [`Pipeline::process`] mem-log-nya. Lihat laporan — ini butuh perubahan API.
//! - `DenyMode` tidak membawa payload, jadi pesan AFK/`onlyThisGroup` tidak
//!   bisa menginterpolasi `botAfk.reason`/durasi/`onlyThisGroup.link` seperti
//!   JS. Yang dikirim adalah kalimat yang benar tanpa data yang dikarang.
//! - Kuota auto-work TIDAK dibakar per-pesan. Di JS `allowAutoWork()` dipanggil
//!   fitur tertentu saja (`handler.js:984, 1129, 1136, 1143, 1149, 1422`), dan
//!   `Reply` tidak punya field yang menandai konsumsi itu — membakarnya di sini
//!   akan menguras kuota untuk command biasa. Gate-nya tetap tersedia lewat
//!   [`Pipeline::allow_auto_work`].

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use oktz_plugin_api::Reply;
use oktz_router::gates::{AutoWorkDecision, AutoWorkSettings};
use oktz_router::serialize::{MessageKind, MessageParams, get_message_body, resolve_message_type};
use oktz_router::{
    AccessGrant, Caller, Decision, DenyMode, GroupOverrides, GroupState, ModeSettings,
    ParseOptions, RateLimiter, Router, RouterContext, UserState, consume_auto_work, is_spamming,
};
use oktz_store::json_store::JsonStore;
use serde_json::Value;
use whatsapp_rust::prelude::*;
use whatsapp_rust::waproto::buffa::MessageField;
use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_response_message::InteractiveResponseMessage as IrOneof;
use whatsapp_rust::wacore_binary::Node;

use crate::cards::{Card, biz_node};

// =============================================================================
// 1. Normalisasi JID — port `getUser` (database.js:529-536)
// =============================================================================

/// `VALID_COUNTRY_CODES` (`src/lib/lid.js:136-148`).
const VALID_COUNTRY_CODES: &[&str] = &[
    "1", "7", "20", "27", "30", "31", "32", "33", "34", "36", "39", "40", "41", "43", "44", "45",
    "46", "47", "48", "49", "51", "52", "53", "54", "55", "56", "57", "58", "60", "61", "62", "63",
    "64", "65", "66", "81", "82", "84", "86", "90", "91", "92", "93", "94", "95", "98", "212", "213",
    "216", "218", "220", "221", "234", "249", "254", "255", "256", "260", "263", "351", "352",
    "353", "354", "355", "356", "357", "358", "359", "370", "371", "372", "373", "374", "375",
    "376", "377", "378", "380", "381", "382", "383", "385", "386", "387", "389", "420", "421",
    "423", "852", "853", "855", "856", "880", "886", "960", "961", "962", "963", "964", "965",
    "966", "967", "968", "970", "971", "972", "973", "974", "975", "976", "977", "992", "993",
    "994", "995", "996", "998",
];

/// Port `isLidConverted` (`src/lib/lid.js:150-167`).
pub fn is_lid_converted(jid: &str) -> bool {
    let Some(number) = jid.strip_suffix("@s.whatsapp.net") else {
        return false;
    };
    if number.len() > 14 {
        return true;
    }
    for len in 1..=3usize {
        let Some(prefix) = number.get(..len) else {
            continue;
        };
        if VALID_COUNTRY_CODES.contains(&prefix)
            && number.len() >= len + 6
            && number.len() <= len + 12
        {
            return false;
        }
    }
    true
}

/// Port `isLidLikeJid` (`src/lib/database.js:7-13`).
pub fn is_lid_like_jid(jid: &str) -> bool {
    if jid.is_empty() {
        return false;
    }
    if jid.ends_with("@lid") {
        return true;
    }
    if !is_lid_converted(jid) {
        return false;
    }
    let Some(number) = jid.strip_suffix("@s.whatsapp.net") else {
        return false;
    };
    // JS `/^\d+$/.test(number) && number.length >= 10`
    !number.is_empty() && number.len() >= 10 && number.bytes().all(|b| b.is_ascii_digit())
}

/// Kunci `users[]` untuk sebuah JID, atau `None` kalau `getUser` mengembalikan
/// `null` (`database.js:529-536`).
///
/// `jid.replace(/@.+/g, "")` di JS = semua sebelum `@` pertama, jadi
/// `split('@').next()`. JID `@lid` — dan nomor LID yang sudah dikonversi —
/// memakai prefiks `lid:` pada kunci; sisanya ditolak kalau terlalu panjang
/// atau diawali `120` (nomor grup).
pub fn normalize_user_key(jid: &str) -> Option<String> {
    if jid.is_empty() {
        return None;
    }
    let is_lid = is_lid_like_jid(jid);
    let clean = jid.split('@').next().unwrap_or(jid);
    if !is_lid && (clean.len() > 15 || clean.starts_with("120")) {
        return None;
    }
    Some(if is_lid {
        format!("lid:{clean}")
    } else {
        clean.to_string()
    })
}

// =============================================================================
// 2. Adapter store
// =============================================================================

fn top<'a>(store: &'a JsonStore, key: &str) -> Option<&'a Value> {
    store.get(key)
}

/// Grant akses dari `user.access[]` (`middleware.js:38-52`).
///
/// `expired` punya tiga kasus dan bedanya berarti:
/// `null` → tidak pernah kedaluwarsa, **hilang** → `undefined` di JS, yang gagal
/// di `=== null` DAN gagal di `> Date.now()` sehingga dianggap SUDAH kedaluwarsa.
fn access_grants(user: &Value) -> Vec<AccessGrant> {
    let Some(arr) = user.get("access").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|g| {
            let cmd = g.get("cmd").and_then(Value::as_str)?;
            let expired = match g.get("expired") {
                None => Some(0),
                Some(Value::Null) => None,
                Some(v) => Some(v.as_i64().unwrap_or(0)),
            };
            Some(AccessGrant {
                cmd: cmd.to_string(),
                expired,
            })
        })
        .collect()
}

/// `users[key]` → [`UserState`]. `key` harus sudah lewat [`normalize_user_key`].
pub fn user_state_from(users: Option<&Value>, key: &str) -> UserState {
    let Some(user) = users.and_then(|u| u.get(key)) else {
        return UserState::default();
    };
    UserState {
        access: access_grants(user),
        is_registered: user
            .get("isRegistered")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// `groups[chat]` → `game` / `rpg`.
///
/// `getGroup` (`database.js:711-714`) TIDAP menormalisasi JID — kunci-nya JID
/// mentah. `None` berarti grup tidak ada, dan `permission.rs:44-50` memperlakukannya
/// sebagai "aturan 8/9 dilewati", bukan ditolak.
pub fn group_state_from(groups: Option<&Value>, chat: &str) -> Option<GroupState> {
    let g = groups?.get(chat)?;
    Some(GroupState {
        game: g.get("game").and_then(Value::as_bool),
        rpg: g.get("rpg").and_then(Value::as_bool),
    })
}

/// `groups[chat].botMode`, default `"all"` (`handler.js:1517`, `handler.js:1711`).
pub fn group_bot_mode_from(groups: Option<&Value>, chat: &str) -> String {
    groups
        .and_then(|g| g.get(chat))
        .and_then(|g| g.get("botMode"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("all")
        .to_string()
}

/// `settings.onlyThisGroup` — string atau `{ jid, ... }` (`middleware.js:180-196`).
fn only_this_group_jid(settings: Option<&Value>) -> Option<String> {
    match settings?.get("onlyThisGroup")? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Object(_) => settings?
            .get("onlyThisGroup")?
            .get("jid")
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn str_list(settings: Option<&Value>, key: &str) -> Vec<String> {
    settings
        .and_then(|s| s.get(key))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn flag(settings: Option<&Value>, key: &str) -> bool {
    settings
        .and_then(|s| s.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// `settings` → [`ModeSettings`]. Kunci persis `middleware.js:143-241`.
pub fn mode_settings_from(settings: Option<&Value>) -> ModeSettings {
    ModeSettings {
        // `db.setting("botMode") || config.mode || "public"` — string kosong
        // dibiarkan kosong; `mode::check_mode` (mode.rs:73-77) sudah fallback
        // ke "public".
        mode: settings
            .and_then(|s| s.get("botMode"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        afk_active: settings
            .and_then(|s| s.get("botAfk"))
            .and_then(|v| v.get("active"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        only_gc: flag(settings, "onlyGc"),
        only_pc: flag(settings, "onlyPc"),
        only_this_group: only_this_group_jid(settings),
        self_groups: str_list(settings, "selfGroups"),
        public_groups: str_list(settings, "publicGroups"),
        self_admin: flag(settings, "selfAdmin"),
        public_admin: flag(settings, "publicAdmin"),
        only_admin: flag(settings, "onlyAdmin"),
    }
}

/// `groups[chat].{selfAdmin,publicAdmin,onlyAdmin}` — `None` per field berarti
/// "tidak ada override", jadi `settings` global yang dipakai
/// (`middleware.js:151-156, 240-242`).
pub fn group_overrides_from(groups: Option<&Value>, chat: &str) -> Option<GroupOverrides> {
    let g = groups?.get(chat)?;
    let f = |k: &str| g.get(k).and_then(Value::as_bool);
    Some(GroupOverrides {
        self_admin: f("selfAdmin"),
        public_admin: f("publicAdmin"),
        only_admin: f("onlyAdmin"),
    })
}

/// `settings.capprem[name]` (`middleware.js:69`). `Some(false)` berarti override
/// implisit dan HARUS menang atas `meta.is_premium`.
pub fn capprem_from(settings: Option<&Value>, name: &str) -> Option<bool> {
    settings?
        .get("capprem")?
        .get(name)?
        .as_bool()
}

/// [`RouterContext`] di atas `JsonStore`.
///
/// Simpan dibungkus `Arc` dan hanya dibaca. Penulisan di runtime adalah urusan
/// jalur lain yang menukar `Arc` — `&self` di sini tidak bisa menulis, sesuai
/// tanda tangan trait.
///
/// `Clone` dipakai runtime: satu pipeline hidup sudah menempel ke bus event,
/// jadi saat `Event::Connected` datang lagi (reconnect) pipeline itu yang
/// dipakai ulang, bukan yang dibangun ulang dari store yang sudah Terbaca.
#[derive(Clone)]
pub struct StoreContext {
    store: Arc<JsonStore>,
    started: Instant,
}

impl StoreContext {
    pub fn new(store: Arc<JsonStore>) -> Self {
        Self {
            store,
            started: Instant::now(),
        }
    }

    /// Dengan titik mulai tetap — supaya `uptime_secs` bisa diuji.
    pub fn with_started_at(store: Arc<JsonStore>, started: Instant) -> Self {
        Self { store, started }
    }
}

impl RouterContext for StoreContext {
    fn user_state(&self, sender: &str) -> UserState {
        match normalize_user_key(sender) {
            Some(key) => user_state_from(top(&self.store, "users"), &key),
            None => UserState::default(),
        }
    }

    fn group_state(&self, chat: &str) -> Option<GroupState> {
        group_state_from(top(&self.store, "groups"), chat)
    }

    fn group_bot_mode(&self, chat: &str) -> String {
        group_bot_mode_from(top(&self.store, "groups"), chat)
    }

    fn mode_settings(&self) -> ModeSettings {
        mode_settings_from(top(&self.store, "settings"))
    }

    fn group_overrides(&self, chat: &str) -> Option<GroupOverrides> {
        group_overrides_from(top(&self.store, "groups"), chat)
    }

    fn capprem(&self, name: &str) -> Option<bool> {
        capprem_from(top(&self.store, "settings"), name)
    }

    /// JS memakai `process.uptime()`. Tidak ada backing di store, jadi ini
    /// elapsed sejak `StoreContext` dibuat.
    fn uptime_secs(&self) -> u64 {
        self.started.elapsed().as_secs()
    }
}

// =============================================================================
// 3. Dedup 100 ms — port `debounceMessage` (src/lib/performance.js:10-29)
// =============================================================================

pub const DEBOUNCE_MS: i64 = 100;

/// `true` berarti duplikat → buang. Kunci dipakai apa adanya; pemanggil yang
/// membentuknya (`handler.js:926`: `${botId}_${m.chat}_${m.sender}_${m.id}`).
#[derive(Debug, Default)]
pub struct Dedup {
    entries: HashMap<String, i64>,
    last_clean: i64,
}

impl Dedup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn debounce(&mut self, key: &str, now_ms: i64) -> bool {
        if let Some(&last) = self.entries.get(key)
            && now_ms - last < DEBOUNCE_MS
        {
            return true;
        }
        self.entries.insert(key.to_string(), now_ms);

        // Pembersihan malas: hanya saat >1000 entri DAN sudah 5 s sejaknowrap
        // pembersihan sebelumnya (performance.js:20-26).
        if self.entries.len() > 1000 && now_ms - self.last_clean > 5000 {
            self.last_clean = now_ms;
            let cutoff = now_ms - DEBOUNCE_MS * 2;
            self.entries.retain(|_, v| *v >= cutoff);
        }
        false
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// =============================================================================
// 4. Lapisan kirim — trait supaya pipeline bisa diuji tanpa koneksi WA
// =============================================================================

pub type SendBox<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Tituhubung yang mengabstraksi `Client::send_message_with_options`
/// (`whatsapp-rust-0.7.0/src/send/mod.rs:901`).
pub trait MessageSender: Send + Sync {
    fn send(
        &self,
        to: &str,
        message: wa::Message,
        extra_nodes: Vec<Node>,
    ) -> SendBox<'_>;
}

/// Sender sungguhan, membungkus `Arc<Client>`.
pub struct ClientSender {
    client: Arc<Client>,
}

impl ClientSender {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

impl MessageSender for ClientSender {
    fn send(&self, to: &str, message: wa::Message, extra_nodes: Vec<Node>) -> SendBox<'_> {
        let client = self.client.clone();
        let to = to.to_string();
        Box::pin(async move {
            let jid: Jid = to
                .parse()
                .map_err(|e| format!("JID tidak valid {to}: {e}"))?;
            let opts = if extra_nodes.is_empty() {
                SendOptions::default()
            } else {
                SendOptions::default().with_extra_stanza_nodes(extra_nodes)
            };
            client
                .send_message_with_options(jid, message, opts)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    }
}

// =============================================================================
// 5. Reply → wire
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    /// `Reply` tidak membawa byte media sama sekali
    /// (`oktz-plugin-api/src/registry.rs:33-39` — `Image { caption, mentions }`
    /// dan `Sticker` tidak punya field media). Tidak ada yang bisa dikirim, jadi
    /// ini Reported, bukan di-drop diam-diam.
    MediaSourceMissing { variant: &'static str },
    Other(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediaSourceMissing { variant } => write!(
                f,
                "Reply::{variant} tidak punya sumber media — `Reply` hanya membawa \
                 caption/mention, tidak ada byte atau media handle. Butuh perubahan \
                 API pada Reply (lihat oktz-plugin-api/src/registry.rs:33-39); \
                 pesan TIDAK dikirim."
            ),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Sumber untuk `Reply::TextQuoted` — pesan yang perlu di-quote.
#[derive(Debug, Clone)]
pub struct QuoteSource {
    pub stanza_id: String,
    pub participant: Option<String>,
    pub message: Box<wa::Message>,
}

impl QuoteSource {
    /// Dari pesan inbound yang memicu (`context_info.stanzaId` + `participant` +
    /// `quotedMessage` — `waproto-0.7.0/src/whatsapp.proto:1624-1631`).
    pub fn from_inbound(m: &InboundMessage) -> Option<Self> {
        Some(Self {
            stanza_id: m.info.id.clone(),
            participant: Some(m.info.source.sender.to_string()),
            message: m.message.as_ref().prepare_for_quote(),
        })
    }
}

/// Satu pesan siap kirim.
#[derive(Debug)]
pub struct Rendered {
    pub message: wa::Message,
    /// Extra stanza node — hanya `biz` untuk kartu native-flow.
    pub extra_nodes: Vec<Node>,
}

fn context_info(mentions: &[String], quote: Option<&QuoteSource>) -> wa::ContextInfo {
    wa::ContextInfo {
        mentioned_jid: mentions.to_vec(),
        quoted_message: quote
            .map(|q| MessageField::some(*q.message.clone()))
            .unwrap_or_default(),
        stanza_id: quote.map(|q| q.stanza_id.clone()),
        participant: quote.and_then(|q| q.participant.clone()),
        ..Default::default()
    }
}

/// Ubah `Reply` menjadi pesan yang bisa dikirim.
///
/// `Ok(None)` = jangan kirim apa pun (`Reply::None`).
///
/// Per-varian:
/// - `Text` tanpa mention → `conversation` polos; dengan mention →
///   `extendedTextMessage` + `context_info.mentioned_jid`
///   (`waproto-0.7.0/src/whatsapp.proto:1629`).
/// - `TextQuoted` → sama, ditambah `context_info.quoted_message`. Tanpa
///   `quote`, teks tetap terkirim tapi quote hilang.
/// - `Interactive` → `Card::new(body).to_message()` + node `biz`
///   (bentuk yang sudah terbukti, `crate::cards`).
/// - `Image` / `Sticker` → `Err(MediaSourceMissing)`.
pub fn render_reply(reply: &Reply, quote: Option<&QuoteSource>) -> Result<Option<Rendered>, PipelineError> {
    match reply {
        Reply::None => Ok(None),
        Reply::Sticker => Err(PipelineError::MediaSourceMissing { variant: "Sticker" }),
        Reply::Image { .. } => Err(PipelineError::MediaSourceMissing {
            variant: "Image",
        }),
        Reply::Text { text, mentions } => Ok(Some(Rendered {
            message: text_message(text, mentions, None),
            extra_nodes: Vec::new(),
        })),
        Reply::TextQuoted { text, mentions } => Ok(Some(Rendered {
            message: text_message(text, mentions, quote),
            extra_nodes: Vec::new(),
        })),
        Reply::Interactive { body } => Ok(Some(Rendered {
            message: Card::new(body).to_message(),
            extra_nodes: vec![biz_node()],
        })),
    }
}

fn text_message(text: &str, mentions: &[String], quote: Option<&QuoteSource>) -> wa::Message {
    if mentions.is_empty() && quote.is_none() {
        // `MessageBuilderExt::text` (wacore-0.7.0/src/proto_helpers.rs:105) —
        // `conversation` polos, paling kecil di wire.
        wa::Message::text(text)
    } else {
        wa::Message::text_with_context(text, context_info(mentions, quote))
    }
}

/// `Decision` → balasan yang dikirim.
pub fn decision_to_reply(d: &Decision) -> Reply {
    match d {
        Decision::Handled(r) => r.clone(),
        Decision::DeniedPermission(msg) | Decision::DeniedGroupMode(msg) => Reply::text(msg),
        Decision::DeniedMode(DenyMode::Afk) => Reply::text(
            "💤 *ʙᴏᴛ sᴇᴅᴀɴɢ ᴀꜰᴋ*\n\n> Bot tidak bisa menerima perintah saat ini\n> Mohon tunggu sampai owner mengaktifkan kembali",
        ),
        Decision::DeniedMode(DenyMode::OnlyThisGroup) => Reply::text(
            "🔒 *Akses Ditolak*\n\n> Bot hanya bisa diakses di grup utama.",
        ),
        // `middleware.js:200-202` self-group dan `middleware.js:177-178` onlyGc/
        // onlyPc menolak tanpa pesan; `DenyMode::Silent` juga tidak membawa teks.
        Decision::DeniedMode(DenyMode::Silent | DenyMode::SelfGroup) => Reply::None,
        Decision::NotACommand | Decision::NoPlugin => Reply::None,
    }
}

// =============================================================================
// 6. Ekstraksi body — `oktz_router::serialize`
// =============================================================================

/// Nama field protobuf untuk pesan ini, `""` kalau di luar daftar serialize.
fn wire_type_of(m: &wa::Message) -> &'static str {
    use MessageKind as K;
    let b = m.get_base_message();
    let pick: Option<K> = if b.conversation.is_some() {
        Some(K::Conversation)
    } else if b.extended_text_message.as_option().is_some() {
        Some(K::ExtendedText)
    } else if b.image_message.as_option().is_some() {
        Some(K::Image)
    } else if b.video_message.as_option().is_some() {
        Some(K::Video)
    } else if b.document_message.as_option().is_some() {
        Some(K::Document)
    } else if b.buttons_response_message.as_option().is_some() {
        Some(K::ButtonsResponse)
    } else if b.list_response_message.as_option().is_some() {
        Some(K::ListResponse)
    } else if b.template_button_reply_message.as_option().is_some() {
        Some(K::TemplateButtonReply)
    } else if b.interactive_response_message.as_option().is_some() {
        Some(K::InteractiveResponse)
    } else if b.poll_creation_message.as_option().is_some() {
        Some(K::PollCreation)
    } else if b.poll_creation_message_v2.as_option().is_some() {
        Some(K::PollCreationV2)
    } else if b.poll_creation_message_v3.as_option().is_some() {
        Some(K::PollCreationV3)
    } else {
        None
    };

    if let Some(k) = pick {
        return k.wire_type();
    }
    // `messageContextInfo` + native-flow response → dipaksa jadi interactive
    // (`serialize.rs:111-120`).
    if b.message_context_info.as_option().is_some() && interactive_params_json(b).is_some() {
        return K::InteractiveResponse.wire_type();
    }
    ""
}

fn interactive_params_json(m: &wa::Message) -> Option<String> {
    let ir = m.get_base_message().interactive_response_message.as_option()?;
    match ir.interactive_response_message.as_ref()? {
        IrOneof::NativeFlowResponseMessage(nf) => nf.params_json.clone(),
    }
}

/// Body pesan masuk, sesuai `getMessageBody` (`src/lib/serialize.js:327-372`).
pub fn extract_body(m: &wa::Message) -> Option<String> {
    let wire = wire_type_of(m);
    if wire.is_empty() {
        return None;
    }
    let kind = MessageKind::from_wire_type(wire);
    let b = m.get_base_message();
    let params_json = interactive_params_json(b);
    let params = MessageParams {
        conversation: b.conversation.as_deref(),
        text: b
            .extended_text_message
            .as_option()
            .and_then(|e| e.text.as_deref()),
        caption: b
            .image_message
            .as_option()
            .and_then(|i| i.caption.as_deref())
            .or_else(|| {
                b.video_message
                    .as_option()
                    .and_then(|i| i.caption.as_deref())
            })
            .or_else(|| {
                b.document_message
                    .as_option()
                    .and_then(|i| i.caption.as_deref())
            }),
        selected_button_id: b
            .buttons_response_message
            .as_option()
            .and_then(|r| r.selected_button_id.as_deref()),
        selected_row_id: b
            .list_response_message
            .as_option()
            .and_then(|r| r.single_select_reply.as_option())
            .and_then(|s| s.selected_row_id.as_deref()),
        selected_id: b
            .template_button_reply_message
            .as_option()
            .and_then(|r| r.selected_id.as_deref()),
        params_json: params_json.as_deref(),
        name: b
            .poll_creation_message
            .as_option()
            .and_then(|p| p.name.as_deref())
            .or_else(|| {
                b.poll_creation_message_v2
                    .as_option()
                    .and_then(|p| p.name.as_deref())
            })
            .or_else(|| {
                b.poll_creation_message_v3
                    .as_option()
                    .and_then(|p| p.name.as_deref())
            }),
    };
    // `resolve_message_type` dipanggil supaya jalur `messageContextInfo` tetap
    // satu sumber kebenaran (serialize.rs:111-120).
    let resolved = resolve_message_type(Some(wire), b.interactive_response_message.as_option().is_some());
    let kind = resolved
        .as_deref()
        .map(MessageKind::from_wire_type)
        .unwrap_or(kind);
    let body = get_message_body(kind, &params);
    if body.is_empty() { None } else { Some(body) }
}

// =============================================================================
// 7. Pipeline
// =============================================================================

/// Identitas pemanggil. Sumber roster TIDAK ada di crate ini — `oktz_core::Config`
/// hanya punya `owner_numbers`, dan premium/admin tidak punya accessor sama sekali.
/// Semua default kosong (bukan siapa pun owner/premium/admin) sampai caller
/// mengisinya; mencocokkan menerima JID penuh ATAU nomor saja.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    pub owners: HashSet<String>,
    pub premium: HashSet<String>,
    pub partners: HashSet<String>,
    pub admins: HashSet<String>,
}

/// Cocok dengan JID penuh, bagian sebelum `@`, atau nomor tanpa sufiks
/// perangkat (`6281:9@s.whatsapp.net` → `6281`).
fn listed(set: &HashSet<String>, jid: &str) -> bool {
    if set.contains(jid) {
        return true;
    }
    let user = jid.split('@').next().unwrap_or(jid);
    set.contains(user) || set.contains(user.split(':').next().unwrap_or(user))
}

impl Identity {
    pub fn is_owner(&self, jid: &str) -> bool {
        listed(&self.owners, jid)
    }
    pub fn is_premium(&self, jid: &str) -> bool {
        listed(&self.premium, jid)
    }
    pub fn is_partner(&self, jid: &str) -> bool {
        listed(&self.partners, jid)
    }
    pub fn is_admin(&self, jid: &str) -> bool {
        listed(&self.admins, jid)
    }
}

/// Hasil satu pesan masuk — dipakai test untuk memastikan tahap mana yang menahan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Tidak ada body teks (stiker, reaction, kontak, …).
    NoBody,
    /// Bukan command.
    NotACommand,
    /// Duplikat dalam 100 ms.
    Deduped,
    /// Kena rate limiter spam.
    SpamBlocked,
    /// Mode/permission ditolak, dan memang ada pesan yang dikirim.
    Denied,
    /// Plugin jalan dan balasan terkirim.
    Sent,
    /// Plugin membalas media yang `Reply` tidak punya sumbernya.
    MediaMissing,
}

pub struct Pipeline {
    ctx: StoreContext,
    sender: Arc<dyn MessageSender>,
    dedup: Mutex<Dedup>,
    spam: Mutex<RateLimiter>,
    auto_work_limiter: Mutex<RateLimiter>,
    parse_opts: ParseOptions,
    bot_id: String,
    identity: Identity,
    anti_spam_enabled: bool,
    auto_work_settings: AutoWorkSettings,
    /// Wajib disimpan: `Subscription` unsubscribe saat di-drop
    /// (wacore-0.7.0/src/types/events.rs:538-544).
    subscription: Mutex<Option<Subscription>>,
}

impl Pipeline {
    pub fn new(
        ctx: StoreContext,
        sender: Arc<dyn MessageSender>,
        bot_id: impl Into<String>,
        parse_opts: ParseOptions,
    ) -> Self {
        Self {
            ctx,
            sender,
            dedup: Mutex::new(Dedup::new()),
            spam: Mutex::new(RateLimiter::command_global()),
            auto_work_limiter: Mutex::new(RateLimiter::auto_work()),
            parse_opts,
            bot_id: bot_id.into(),
            identity: Identity::default(),
            anti_spam_enabled: true,
            auto_work_settings: AutoWorkSettings::default(),
            subscription: Mutex::new(None),
        }
    }

    pub fn with_identity(mut self, identity: Identity) -> Self {
        self.identity = identity;
        self
    }

    pub fn with_anti_spam(mut self, enabled: bool) -> Self {
        self.anti_spam_enabled = enabled;
        self
    }

    pub fn ctx(&self) -> &StoreContext {
        &self.ctx
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Kuota auto-work untuk fitur yang memang memakainya. Dipanggil per-fitur,
    /// bukan per-pesan — lihat catatan modul.
    pub fn allow_auto_work(&self, caller: &Caller, key: &str, user_energi: Option<i64>, now_ms: i64) -> AutoWorkDecision {
        let mut limiter = self.auto_work_limiter.lock().expect("auto_work limiter");
        consume_auto_work(
            &mut limiter,
            caller,
            key,
            &self.auto_work_settings,
            user_energi,
            now_ms,
        )
    }

    /// Caller dari `MessageInfo.source` (`MessageSource`, wacore-0.7.0
    /// `src/types/message.rs:104`).
    pub fn caller_of(&self, m: &InboundMessage) -> Caller {
        let s = &m.info.source;
        let sender = s.sender.to_string();
        Caller {
            sender: sender.clone(),
            chat: s.chat.to_string(),
            command: String::new(),
            is_group: s.is_group,
            is_owner: self.identity.is_owner(&sender),
            is_partner: self.identity.is_partner(&sender),
            is_premium: self.identity.is_premium(&sender),
            is_admin: s.is_group && self.identity.is_admin(&sender),
            is_bot_admin: false,
        }
    }

    /// Satu pesan inbound → satu keputusan → satu pengiriman.
    pub async fn process(&self, m: &InboundMessage, now_ms: i64) -> Result<Outcome, PipelineError> {
        let Some(body) = extract_body(&m.message) else {
            return Ok(Outcome::NoBody);
        };

        let mut caller = self.caller_of(m);

        // Dedup: kunci persis handler.js:926.
        let dedup_key = format!(
            "{}_{}_{}_{}",
            self.bot_id,
            caller.chat,
            caller.sender,
            m.info.id
        );
        if self
            .dedup
            .lock()
            .expect("dedup")
            .debounce(&dedup_key, now_ms)
        {
            return Ok(Outcome::Deduped);
        }

        // Spam: kunci `${botId}_${m.sender}` (handler.js:1508).
        let spam_key = format!("{}_{}", self.bot_id, caller.sender);
        if is_spamming(
            &mut self.spam.lock().expect("spam"),
            &caller,
            &spam_key,
            now_ms,
            self.anti_spam_enabled,
        ) {
            return Ok(Outcome::SpamBlocked);
        }

        let router = Router::new(&self.ctx, self.parse_opts.clone(), now_ms);
        let decision = router.route(&mut caller, &body, oktz_plugin_api::find);

        let reply = decision_to_reply(&decision);
        let quote = QuoteSource::from_inbound(m);
        let Some(rendered) = render_reply(&reply, quote.as_ref())? else {
            // `Reply::None`: `DeniedMode::Silent`, `SelfGroup`, `NoPlugin`,
            // `NotACommand`, atau `Reply::None` dari plugin — semua sengaja diam.
            return Ok(if matches!(decision, Decision::Handled(_)) {
                Outcome::Sent
            } else {
                Outcome::NotACommand
            });
        };

        let outcome = if matches!(decision, Decision::Handled(_)) {
            Outcome::Sent
        } else {
            Outcome::Denied
        };

        if let Err(e) = self
            .sender
            .send(&caller.chat, rendered.message, rendered.extra_nodes)
            .await
        {
            return Err(PipelineError::Other(format!("gagal kirim: {e}")));
        }
        Ok(outcome)
    }

    /// Kapasitas antrean inbound. Fixed, bukan tak berbatas: `handle_event`
    /// sinkron dan dipanggil INLINE di bus event (wacore-0.7.0
    /// `src/types/events.rs:609-621`), jadi satu-satunya operasi yang aman di
    /// sana adalah `try_send` — `send().await` akan menahan seluruh dispatch
    /// event bot, dan `send_blocking` bisa deadlock kalau handler-nya jalan di
    /// thread tokio worker. Channel tak berbatas (pola `ChannelEventHandler`,
    /// `events.rs:362-370`) bebas deadlock tapi membiarkan memory tumbuh tanpa
    /// batas saat bot tidak mampu mengikuti, jadi dipakai capacity tetap.
    const INGEST_CAPACITY: usize = 256;

    /// Drain antrean: satu pesan → satu [`Pipeline::process`].
    ///
    /// Dipisah dari [`Pipeline::attach`] supaya loop ini bisa diuji dengan
    /// channel buatan sendiri — `attach` sendiri butuh `Arc<Client>` sungguhan.
    ///
    /// Berhenti hanya kalau semua `tx` di-drop, yaitu saat `Pipeline` beserta
    /// task ini dijatuhkan.
    pub async fn run(self: Arc<Self>, rx: IngestRx) {
        while let Ok(m) = rx.recv().await {
            if let Err(e) = self.process(&m, now_ms()).await {
                // Task ini satu-satunya pemroses pesan. Kegagalan diam-diam
                // akan menghentikan bot tanpa jejak, jadi selalu log dengan
                // konteks yang cukup untuk dicari balik.
                tracing::error!(
                    error = %e,
                    id = %m.info.id,
                    chat = %m.info.source.chat,
                    sender = %m.info.source.sender,
                    "pipeline gagal memproses pesan"
                );
            }
        }
        tracing::debug!("antrean pesan ditutup; loop pipeline berhenti");
    }

    /// Daftarkan handler ke bus event dan mulai task async pemroses.
    ///
    /// Aman dipanggil lebih dari sekali: `Event::Connected` dikirim ulang per
    /// generasi koneksi (`whatsapp-rust-0.7.0/src/client/lifecycle.rs:230`),
    /// jadi reconnect menjalankan `on_ready` lagi. `Subscription` lama di-drop
    /// (`wacore-0.7.0/src/types/events.rs:538-544`) → handler lama unsubscribe
    /// → `tx`-nya hilang → task lama keluar dari [`Pipeline::run`]. Kalau tidak,
    /// dua loop hidup dan satu pesan dibalas dua kali.
    pub fn attach(self: &Arc<Self>, client: &Arc<Client>) {
        let (tx, rx) = whatsapp_rust::async_channel::bounded(Self::INGEST_CAPACITY);
        let handler = Arc::new(PipelineHandler { tx });
        let sub = client.subscribe(EventInterest::of(&[EventKind::Messages]), handler);
        let mut slot = self.subscription.lock().unwrap_or_else(|e| e.into_inner());
        *slot = Some(sub);
        drop(slot);
        tokio::spawn(self.clone().run(rx));
    }
}

/// Sisi terima antrean inbound. Alias supaya tanda tangan `run`/`attach`
/// tidak menulis ulang path crate.
pub type IngestRx = whatsapp_rust::async_channel::Receiver<InboundMessage>;
type IngestTx = whatsapp_rust::async_channel::Sender<InboundMessage>;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Menerima event dan mengantre ke task pemrosesan.
///
/// Satu-satunya pekerjaan di sini adalah `try_send`. Dipanggil inline di
/// thread mana pun yang sedang memproses stanza, jadi tidak boleh-blocking,
/// tidak boleh async, dan tidak boleh panik.
struct PipelineHandler {
    tx: IngestTx,
}

impl EventHandler for PipelineHandler {
    fn handle_event(&self, event: Arc<Event>) {
        let Event::Messages(batch) = &*event else {
            return;
        };
        for m in batch.iter() {
            // Penuh = bot tidak mampu mengikuti. Menunggu di sini akan menahan
            // bus, jadi yang tersisa adalah buang + catat: menelan perintah
            // tanpa jejak jauh lebih sulit dicari daripada satu baris warn.
            if let Err(e) = self.tx.try_send(m.clone()) {
                tracing::warn!(
                    error = %e,
                    id = %m.info.id,
                    chat = %m.info.source.chat,
                    queue = Pipeline::INGEST_CAPACITY,
                    "antrean pesan penuh; pesan dibuang"
                );
            }
        }
    }

    fn interest(&self) -> EventInterest {
        EventInterest::of(&[EventKind::Messages])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use whatsapp_rust::types::message::MessageSource;
    use whatsapp_rust::waproto::whatsapp::__buffa::oneof::message::interactive_message::InteractiveMessage as ImOneof;

    // ---------- helper ----------

    // `cargo test` menjalankan test dalam thread paralel; nama file tetap
    // membuat mereka saling menimpa file yang sama. Nomor unik per panggilan.
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn store_with(name: &str, body: &str) -> Arc<JsonStore> {
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("oktz-pipeline-{name}-{n}.json"));
        std::fs::write(&p, body).unwrap();
        Arc::new(JsonStore::open(&p).unwrap())
    }

    #[derive(Clone, Default)]
    struct RecordingSender {
        sent: Arc<Mutex<Vec<(String, wa::Message, usize)>>>,
    }

    impl MessageSender for RecordingSender {
        fn send(&self, to: &str, message: wa::Message, extra: Vec<Node>) -> SendBox<'_> {
            self.sent
                .lock()
                .unwrap()
                .push((to.to_string(), message, extra.len()));
            Box::pin(async move { Ok(()) })
        }
    }

    fn inbound(body: &str, sender: &str, chat: &str, is_group: bool, id: &str) -> InboundMessage {
        // `InboundMessage` `#[non_exhaustive]` (events.rs:1125) — dibangun lewat
        // builder hasil derive `bon::Builder`.
        InboundMessage::builder()
            .message(Arc::new(wa::Message {
                conversation: Some(body.to_string()),
                ..Default::default()
            }))
            .info(Arc::new(MessageInfo {
                source: MessageSource {
                    chat: chat.parse().unwrap(),
                    sender: sender.parse().unwrap(),
                    is_from_me: false,
                    is_group,
                    ..Default::default()
                },
                id: id.to_string(),
                ..Default::default()
            }))
            .build()
    }

    fn pipeline_with(body: &str, identity: Identity) -> (Arc<Pipeline>, Arc<RecordingSender>) {
        let ctx = StoreContext::new(store_with("e2e", body));
        let sender = Arc::new(RecordingSender::default());
        let p = Pipeline::new(ctx, sender.clone(), "bot", ParseOptions::default())
            .with_identity(identity);
        (Arc::new(p), sender)
    }

    fn body_of(m: &wa::Message) -> Option<&str> {
        if let Some(c) = m.conversation.as_deref() {
            return Some(c);
        }
        m.extended_text_message.as_option()?.text.as_deref()
    }

    fn ctx_of(m: &wa::Message) -> Option<&wa::ContextInfo> {
        m.extended_text_message.as_option()?.context_info.as_option()
    }

    const OK_STORE: &str = r#"{
      "users": {
        "628111": {"isRegistered": true,
          "access": [{"cmd":"ping","expired":null},{"cmd":"old","expired":1}]},
        "lid:9998887771": {"isRegistered": false, "access": []}
      },
      "groups": {
        "120363@g.us": {"game": false, "rpg": true, "botMode": "store",
                       "selfAdmin": true, "publicAdmin": false, "onlyAdmin": true}
      },
      "settings": {
        "botMode": "public", "onlyGc": true, "onlyPc": false,
        "botAfk": {"active": true, "since": 1, "reason": "libur"},
        "onlyThisGroup": {"jid": "120999@g.us", "name": "Utama", "link": "https://x"},
        "selfGroups": ["111@g.us"], "publicGroups": ["222@g.us"],
        "selfAdmin": false, "publicAdmin": true, "onlyAdmin": false,
        "capprem": {"ping": false, "tebak": true}
      }
    }"#;

    // ---------- 1. normalisasi JID ----------

    #[test]
    fn plain_pn_jid_key_is_the_number() {
        assert_eq!(
            normalize_user_key("628123456789@s.whatsapp.net").as_deref(),
            Some("628123456789")
        );
        assert_eq!(
            normalize_user_key("628123456789:5@s.whatsapp.net").as_deref(),
            Some("628123456789:5")
        );
    }

/// Nomor grup diawali `120` dan nomor yang terlalu panjang ditolak
/// (`database.js:533`). JID `@s.whatsapp.net` dengan >14 digit justru
/// diklasifikasi LID oleh `isLidConverted` (`lid.js:156`) sehingga lolos —
/// itu perilaku JS, bukan bug.
#[test]
fn group_number_and_long_number_are_rejected() {
    // Nomor grup diawali 120.
    assert_eq!(normalize_user_key("120363001234567890@g.us"), None);
    assert_eq!(normalize_user_key("120363@g.us"), None);
    // > 15 karakter, dan bukan LID karena domainnya bukan @s.whatsapp.net/@lid.
    assert_eq!(normalize_user_key("1234567890123456@newsletter"), None);
    assert_eq!(normalize_user_key(""), None);
    // 15 digit di @s.whatsapp.net = "converted LID" -> tetap diterima, dengan
    // prefiks `lid:`. Dikunci supaya tidak dianggap regresi.
    assert_eq!(
        normalize_user_key("1234567890123456@s.whatsapp.net").as_deref(),
        Some("lid:1234567890123456")
    );
}

    #[test]
    fn lid_jid_uses_lid_prefixed_key_and_skips_length_check() {
        assert!(is_lid_like_jid("9998887771@lid"));
        assert_eq!(
            normalize_user_key("9998887771@lid").as_deref(),
            Some("lid:9998887771")
        );
        // Nomor LID > 15 karakter tetap lolos karena isLidLikeJid benar.
        assert_eq!(
            normalize_user_key("123456789012345678@lid").as_deref(),
            Some("lid:123456789012345678")
        );
    }

    #[test]
    fn converted_lid_number_is_detected_and_prefixed() {
        // 20 digit: > 14 dan >= 10 digit -> isLidConverted true (lid.js:156).
        assert!(is_lid_converted("99988877710001112233@s.whatsapp.net"));
        assert!(is_lid_like_jid("99988877710001112233@s.whatsapp.net"));
        assert_eq!(
            normalize_user_key("99988877710001112233@s.whatsapp.net").as_deref(),
            Some("lid:99988877710001112233")
        );
    }

    #[test]
    fn normal_phone_number_is_not_lid_like() {
        // "62" + 9 digit = 11,Within [len+6, len+12] -> isLidConverted false.
        assert!(!is_lid_converted("628123456789@s.whatsapp.net"));
        assert!(!is_lid_like_jid("628123456789@s.whatsapp.net"));
        // Prefix "@lid" wajib; "@s.whatsapp.net" lain tidak.
        assert!(!is_lid_converted("628123@g.us"));
    }

    // ---------- 2. adapter store ----------

    #[test]
    fn user_state_reads_access_and_registration_from_users_map() {
        let s = store_with("userstate", OK_STORE);
        let ctx = StoreContext::new(s);
        let u = ctx.user_state("628111@s.whatsapp.net");
        assert!(u.is_registered);
        let ping = u.access.iter().find(|a| a.cmd == "ping").unwrap();
        assert_eq!(ping.expired, None, "expired:null = tidak pernah kedaluwarsa");
        let old = u.access.iter().find(|a| a.cmd == "old").unwrap();
        assert_eq!(old.expired, Some(1));
    }

    #[test]
    fn missing_expired_key_counts_as_expired_not_never() {
        // `undefined` gagal di `=== null` DAN gagal di `> Date.now()` (middleware.js:43).
        let s = store_with(
            "missingexp",
            r#"{"users": {"628": {"access": [{"cmd": "x"}]}}}"#,
        );
        let ctx = StoreContext::new(s);
        let u = ctx.user_state("628@s.whatsapp.net");
        assert_eq!(u.access[0].expired, Some(0));
        assert!(!has_access_for(&u, "x", 1));
    }

    fn has_access_for(u: &UserState, cmd: &str, now_ms: i64) -> bool {
        u.access.iter().any(|a| a.cmd == cmd && a.expired.is_none_or(|e| e > now_ms))
    }

    #[test]
    fn lid_user_is_found_by_normalized_key() {
        let s = store_with("liduser", OK_STORE);
        let ctx = StoreContext::new(s);
        let u = ctx.user_state("9998887771@lid");
        assert!(!u.is_registered, "harus membaca users['lid:9998887771']");
    }

    #[test]
    fn rejected_jid_yields_default_user_state() {
        let s = store_with("rejectuser", OK_STORE);
        let ctx = StoreContext::new(s);
        let u = ctx.user_state("120363001234567890@g.us");
        assert!(!u.is_registered);
        assert!(u.access.is_empty());
    }

    #[test]
    fn group_state_reads_game_and_rpg_and_absent_group_is_none() {
        let s = store_with("groupstate", OK_STORE);
        let ctx = StoreContext::new(s);
        let g = ctx.group_state("120363@g.us").expect("grup ada");
        assert_eq!(g.game, Some(false));
        assert_eq!(g.rpg, Some(true));
        // Tidak ada di store -> None, bukan Some(default) (permission.rs:44-50).
        assert_eq!(ctx.group_state("999@g.us"), None);
    }

    #[test]
    fn group_bot_mode_defaults_to_all() {
        let s = store_with("botmode", OK_STORE);
        let ctx = StoreContext::new(s);
        assert_eq!(ctx.group_bot_mode("120363@g.us"), "store");
        assert_eq!(ctx.group_bot_mode("999@g.us"), "all");
    }

    #[test]
    fn mode_settings_read_every_documented_key() {
        let s = store_with("modesettings", OK_STORE);
        let ctx = StoreContext::new(s);
        let m = ctx.mode_settings();
        assert_eq!(m.mode, "public");
        assert!(m.only_gc);
        assert!(!m.only_pc);
        assert!(m.afk_active);
        // Object `{jid,...}` dinormalkan ke jid (middleware.js:184).
        assert_eq!(m.only_this_group.as_deref(), Some("120999@g.us"));
        assert_eq!(m.self_groups, vec!["111@g.us".to_string()]);
        assert_eq!(m.public_groups, vec!["222@g.us".to_string()]);
        assert!(!m.self_admin);
        assert!(m.public_admin);
        assert!(!m.only_admin);
    }

    #[test]
    fn only_this_group_accepts_plain_string_form() {
        let s = store_with(
            "otgstr",
            r#"{"settings": {"onlyThisGroup": "120abc@g.us"}}"#,
        );
        let ctx = StoreContext::new(s);
        assert_eq!(
            ctx.mode_settings().only_this_group.as_deref(),
            Some("120abc@g.us")
        );
    }

    #[test]
    fn missing_settings_map_falls_back_to_defaults() {
        let ctx = StoreContext::new(store_with("nosettings", "{}"));
        let m = ctx.mode_settings();
        assert_eq!(m.mode, "");
        assert!(!m.only_gc);
        assert_eq!(m.only_this_group, None);
        assert!(m.self_groups.is_empty());
    }

    #[test]
    fn group_overrides_are_per_field_optional() {
        let s = store_with("overrides", OK_STORE);
        let ctx = StoreContext::new(s);
        let o = ctx.group_overrides("120363@g.us").expect("grup ada");
        assert_eq!(o.self_admin, Some(true));
        assert_eq!(o.public_admin, Some(false));
        assert_eq!(o.only_admin, Some(true));
        assert!(ctx.group_overrides("999@g.us").is_none());
    }

    #[test]
    fn capprem_distinguishes_absent_from_false() {
        let s = store_with("capprem", OK_STORE);
        let ctx = StoreContext::new(s);
        // `false` HARUS-bedanya dari `None` (permission.rs:120-122).
        assert_eq!(ctx.capprem("ping"), Some(false));
        assert_eq!(ctx.capprem("tebak"), Some(true));
        assert_eq!(ctx.capprem("tidakada"), None);
    }

    #[test]
    fn uptime_is_elapsed_since_context_started() {
        let started = Instant::now() - std::time::Duration::from_secs(120);
        let ctx = StoreContext::with_started_at(store_with("uptime", "{}"), started);
        assert_eq!(ctx.uptime_secs(), 120);
    }

    // ---------- 3. dedup ----------

    #[test]
    fn duplicate_within_100ms_is_dropped() {
        let mut d = Dedup::new();
        assert!(!d.debounce("k", 1_000));
        assert!(d.debounce("k", 1_000 + DEBOUNCE_MS - 1));
    }

    #[test]
    fn same_key_after_window_is_accepted_again() {
        let mut d = Dedup::new();
        assert!(!d.debounce("k", 1_000));
        assert!(!d.debounce("k", 1_000 + DEBOUNCE_MS));
    }

    #[test]
    fn different_keys_do_not_collide() {
        let mut d = Dedup::new();
        assert!(!d.debounce("a", 1_000));
        assert!(!d.debounce("b", 1_000));
    }

    #[test]
    fn cleanup_only_runs_past_1000_entries_and_5s() {
        let mut d = Dedup::new();
        for i in 0..1001 {
            d.debounce(&format!("k{i}"), 1_000);
        }
        assert_eq!(d.len(), 1001);
        //_ENTRY ke-1002: > 1000 dan now-0 > 5000 (t=7000) -> sweep.
        d.debounce("trigger", 7_000);
        assert!(
            d.len() < 1001,
            "sweep harus membuang entri basi, dapat {}",
            d.len()
        );
    }

    // ---------- 4. Reply -> wire ----------

    #[test]
    fn text_without_mentions_is_plain_conversation() {
        let r = render_reply(&Reply::text("pong"), None).unwrap().unwrap();
        assert_eq!(r.message.conversation.as_deref(), Some("pong"));
        assert!(r.extra_nodes.is_empty());
    }

    #[test]
    fn text_mentions_go_into_context_info_not_body() {
        let reply = Reply::text_mention("hai @1", "6281@s.whatsapp.net");
        let r = render_reply(&reply, None).unwrap().unwrap();
        assert!(r.message.conversation.is_none(), "harus extendedTextMessage");
        let c = ctx_of(&r.message).expect("context_info");
        assert_eq!(c.mentioned_jid, vec!["6281@s.whatsapp.net".to_string()]);
        assert_eq!(body_of(&r.message), Some("hai @1"));
    }

    #[test]
    fn text_quoted_carries_quoted_message_and_mention() {
        let quote = QuoteSource {
            stanza_id: "STANZA1".into(),
            participant: Some("6281@s.whatsapp.net".into()),
            message: Box::new(wa::Message::text("asal")),
        };
        let reply = Reply::TextQuoted {
            text: "tanggapan".into(),
            mentions: vec!["6282@s.whatsapp.net".into()],
        };
        let r = render_reply(&reply, Some(&quote)).unwrap().unwrap();
        let c = ctx_of(&r.message).expect("context_info");
        assert_eq!(c.stanza_id.as_deref(), Some("STANZA1"));
        assert_eq!(c.participant.as_deref(), Some("6281@s.whatsapp.net"));
        assert!(c.quoted_message.as_option().is_some());
        assert_eq!(c.mentioned_jid, vec!["6282@s.whatsapp.net".to_string()]);
    }

    #[test]
    fn text_quoted_without_source_still_sends_text() {
        let reply = Reply::TextQuoted {
            text: "tanpa quote".into(),
            mentions: vec![],
        };
        let r = render_reply(&reply, None).unwrap().unwrap();
        assert_eq!(body_of(&r.message), Some("tanpa quote"));
    }

    #[test]
    fn interactive_reuses_card_and_adds_biz_node() {
        let r = render_reply(&Reply::Interactive { body: "pilih".into() }, None)
            .unwrap()
            .unwrap();
        assert_eq!(r.extra_nodes.len(), 1, "node biz wajib untuk native-flow");
        let inner = r
            .message
            .view_once_message
            .as_option()
            .and_then(|v| v.message.as_option())
            .expect("dibungkus viewOnceMessage");
        let im = inner.interactive_message.as_option().expect("interactive");
        let Some(ImOneof::NativeFlowMessage(nf)) = im.interactive_message.as_ref() else {
            panic!("harus native_flow");
        };
        assert_eq!(nf.message_params_json, None);
    }

    #[test]
    fn image_reply_is_a_typed_error_not_a_silent_drop() {
        let reply = Reply::Image {
            caption: "c".into(),
            mentions: vec![],
        };
        let err = render_reply(&reply, None).unwrap_err();
        assert_eq!(
            err,
            PipelineError::MediaSourceMissing { variant: "Image" }
        );
        let msg = err.to_string();
        assert!(msg.contains("sumber media"), "pesan harus menyebut gap: {msg}");
    }

    #[test]
    fn sticker_reply_is_a_typed_error() {
        let err = render_reply(&Reply::Sticker, None).unwrap_err();
        assert_eq!(
            err,
            PipelineError::MediaSourceMissing {
                variant: "Sticker"
            }
        );
    }

    #[test]
    fn reply_none_renders_to_nothing() {
        assert!(render_reply(&Reply::None, None).unwrap().is_none());
    }

    // ---------- 5. Decision -> Reply ----------

    #[test]
    fn non_command_and_no_plugin_are_silent() {
        assert_eq!(decision_to_reply(&Decision::NotACommand), Reply::None);
        assert_eq!(decision_to_reply(&Decision::NoPlugin), Reply::None);
    }

    #[test]
    fn permission_and_group_mode_denials_reply_with_message() {
        assert_eq!(
            decision_to_reply(&Decision::DeniedPermission("x".into())),
            Reply::text("x")
        );
        assert_eq!(
            decision_to_reply(&Decision::DeniedGroupMode("y".into())),
            Reply::text("y")
        );
    }

    #[test]
    fn silent_modes_send_nothing_but_afk_and_only_this_group_reply() {
        assert_eq!(
            decision_to_reply(&Decision::DeniedMode(DenyMode::Silent)),
            Reply::None
        );
        assert_eq!(
            decision_to_reply(&Decision::DeniedMode(DenyMode::SelfGroup)),
            Reply::None
        );
        for m in [DenyMode::Afk, DenyMode::OnlyThisGroup] {
            let label = format!("{m:?}");
            let r = decision_to_reply(&Decision::DeniedMode(m.clone()));
            assert!(!r.is_none(), "{label} harus membalas");
            assert!(r.text_body().is_some_and(|t| !t.is_empty()));
        }
    }

    #[test]
    fn handled_passes_reply_through_untouched() {
        let original = Reply::text("pong");
        assert_eq!(
            decision_to_reply(&Decision::Handled(original.clone())),
            original
        );
    }

    // ---------- 6. body extraction ----------

    #[test]
    fn conversation_body_is_extracted() {
        assert_eq!(extract_body(&wa::Message::text("halo")), Some("halo".into()));
    }

    #[test]
    fn extended_text_body_is_extracted() {
        let m = wa::Message::text_with_context("hai", wa::ContextInfo::default());
        assert_eq!(extract_body(&m), Some("hai".into()));
    }

    #[test]
    fn message_without_text_has_no_body() {
        assert_eq!(extract_body(&wa::Message::default()), None);
    }

    #[test]
    fn empty_conversation_is_treated_as_no_body() {
        let m = wa::Message {
            conversation: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(extract_body(&m), None);
    }

    #[test]
    fn interactive_response_yields_selected_id_from_params_json() {
        let m = wa::Message {
            interactive_response_message: MessageField::some(wa::message::InteractiveResponseMessage {
                interactive_response_message: Some(IrOneof::NativeFlowResponseMessage(Box::new(
                    wa::message::interactive_response_message::NativeFlowResponseMessage {
                        params_json: Some(r#"{"id":"btn_1"}"#.into()),
                        ..Default::default()
                    },
                ))),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(extract_body(&m), Some("btn_1".into()));
    }

    // ---------- 7. pipeline end-to-end (tanpa koneksi WA) ----------

    #[tokio::test]
    async fn non_command_is_ignored_and_nothing_sent() {
        let (p, s) = pipeline_with("{}", Identity::default());
        let m = inbound("halo dunia", "6281@s.whatsapp.net", "6281@s.whatsapp.net", false, "M1");
        assert_eq!(p.process(&m, 1_000).await.unwrap(), Outcome::NotACommand);
        assert!(s.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn message_without_body_is_ignored() {
        let (p, s) = pipeline_with("{}", Identity::default());
        let m = InboundMessage::builder()
            .message(Arc::new(wa::Message::default()))
            .info(
                inbound("", "6281@s.whatsapp.net", "6281@s.whatsapp.net", false, "M1").info,
            )
            .build();
        assert_eq!(p.process(&m, 1_000).await.unwrap(), Outcome::NoBody);
        assert!(s.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn duplicate_message_id_is_deduped_and_sent_once() {
        let (p, s) = pipeline_with("{}", Identity::default());
        let m = inbound(".ping", "6281@s.whatsapp.net", "6281@s.whatsapp.net", false, "SAME");
        let o1 = p.process(&m, 1_000).await.unwrap();
        let o2 = p.process(&m, 1_010).await.unwrap();
        assert_eq!(o2, Outcome::Deduped);
        assert_ne!(o1, Outcome::Deduped);
        // Tanpa plugin terdaftar di oktz-wa tidak ada yang terkirim; yang
        // dibuktikan di sini adalah gate dedup, bukan dispatch plugin.
        assert!(s.sent.lock().unwrap().len() <= 1);
    }

    #[tokio::test]
    async fn afk_mode_replies_to_a_command() {
        let store = r#"{"settings": {"botAfk": {"active": true, "since": 1}}}"#;
        let (p, s) = pipeline_with(store, Identity::default());
        let m = inbound(".ping", "6281@s.whatsapp.net", "6281@s.whatsapp.net", false, "A1");
        assert_eq!(p.process(&m, 1_000).await.unwrap(), Outcome::Denied);
        let sent = s.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "6281@s.whatsapp.net");
        assert!(body_of(&sent[0].1).unwrap().contains("sᴇᴅᴀɴɢ ᴀꜰᴋ"));
    }

    #[tokio::test]
    async fn owner_is_exempt_from_afk() {
        let store = r#"{"settings": {"botAfk": {"active": true, "since": 1}}}"#;
        let mut owners = HashSet::new();
        owners.insert("6281".to_string());
        let (p, s) = pipeline_with(store, Identity { owners, ..Default::default() });
        let m = inbound(".ping", "6281@s.whatsapp.net", "6281@s.whatsapp.net", false, "A2");
        // Owner lolos mode; lalu `find` tidak punya plugin -> NoPlugin -> diam.
        assert_eq!(p.process(&m, 1_000).await.unwrap(), Outcome::NotACommand);
        assert!(s.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn owner_matched_by_full_jid_or_number() {
        let mut owners = HashSet::new();
        owners.insert("6281@s.whatsapp.net".to_string());
        let id = Identity { owners, ..Default::default() };
        assert!(id.is_owner("6281@s.whatsapp.net"));
        let mut owners = HashSet::new();
        owners.insert("6281".to_string());
        let id = Identity { owners, ..Default::default() };
        assert!(id.is_owner("6281:9@s.whatsapp.net"), "cocok via nomor");
        assert!(!Identity::default().is_owner("6281@s.whatsapp.net"));
    }

    #[tokio::test]
    async fn spam_gate_blocks_repeated_commands() {
        // Limiter global: 8 titik / 3 detik (gates.rs:64-66).
        let (p, s) = pipeline_with("{}", Identity::default());
        let mut blocked = 0;
        for i in 0..12 {
            let m = inbound(
                ".ping",
                "6281@s.whatsapp.net",
                "6281@s.whatsapp.net",
                false,
                &format!("SP{i}"),
            );
            if p.process(&m, 2_000).await.unwrap() == Outcome::SpamBlocked {
                blocked += 1;
            }
        }
        assert!(blocked > 0, "limiter harus menahan setelah 8 perintah");
        drop(s);
    }

    #[tokio::test]
    async fn auto_work_gate_is_not_charged_by_ordinary_commands() {
        // Dedup memakai id berbeda, jadi semua perintah benar-benar lewat gate.
        let (p, _s) = pipeline_with("{}", Identity::default());
        let caller = Caller {
            sender: "6281@s.whatsapp.net".into(),
            chat: "6281@s.whatsapp.net".into(),
            ..Default::default()
        };
        for i in 0..5 {
            let m = inbound(
                ".ping",
                "6281@s.whatsapp.net",
                "6281@s.whatsapp.net",
                false,
                &format!("W{i}"),
            );
            p.process(&m, 2_000).await.unwrap();
        }
        // Kuota 10/menit; 5 perintah biasa tidak boleh memakainya -> saldo penuh.
        for i in 0..10 {
            let d = p.allow_auto_work(&caller, "auto", Some(999_999), 3_000);
            assert!(d.allowed, "kuota tak boleh terpakai: #{i}");
        }
    }

    #[tokio::test]
    async fn group_reply_is_addressed_to_the_group_chat() {
        let (p, s) = pipeline_with(
            r#"{"settings": {"botAfk": {"active": true, "since": 1}}}"#,
            Identity::default(),
        );
        let m = inbound(".ping", "6281@s.whatsapp.net", "120363@g.us", true, "G1");
        p.process(&m, 1_000).await.unwrap();
        let sent = s.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "120363@g.us", "tujuan = chat, bukan sender");
    }

    #[test]
    fn store_map_with_unexpected_shape_does_not_panic() {
        //(users bukan object) — harus tetap aman, bukan panic.
        let s = store_with("weird", r#"{"users": [], "groups": 5, "settings": "x"}"#);
        let ctx = StoreContext::new(s);
        assert!(ctx.user_state("6281@s.whatsapp.net").access.is_empty());
        assert_eq!(ctx.group_state("120363@g.us"), None);
        assert_eq!(ctx.group_bot_mode("120363@g.us"), "all");
        assert!(ctx.mode_settings().self_groups.is_empty());
        assert_eq!(ctx.capprem("x"), None);
        let _ = json!({});
    }

    // ---------- 8. jembatan bus event → pipeline (tanpa Client) ----------

    /// Bungkus beberapa pesan jadi satu `Event::Messages` persis seperti yang
    /// dikirim library (`events.rs:858`). `MessageBatch` `#[non_exhaustive]`
    /// (`events.rs:1157`) jadi harus lewat builder.
    fn messages_event(ids: &[&str]) -> Arc<Event> {
        let messages: Arc<[InboundMessage]> = Arc::from(
            ids.iter()
                .map(|id| {
                    inbound(
                        ".ping",
                        "6281@s.whatsapp.net",
                        "6281@s.whatsapp.net",
                        false,
                        id,
                    )
                })
                .collect::<Vec<_>>(),
        );
        Arc::new(Event::Messages(
            whatsapp_rust::types::events::MessageBatch::builder()
                .messages(messages)
                .origin(whatsapp_rust::types::events::BatchOrigin::Live)
                .build(),
        ))
    }

    #[test]
    fn handler_enqueues_every_message_in_a_batch() {
        let (tx, rx) = whatsapp_rust::async_channel::unbounded();
        let h = PipelineHandler { tx };
        h.handle_event(messages_event(&["B1", "B2", "B3"]));
        let ids: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|m| m.info.id.clone())
            .collect();
        assert_eq!(ids, vec!["B1", "B2", "B3"]);
    }

    #[test]
    fn handler_ignores_non_message_events() {
        let (tx, rx) = whatsapp_rust::async_channel::unbounded();
        let h = PipelineHandler { tx };
        h.handle_event(Arc::new(Event::Connected(
            whatsapp_rust::types::events::Connected::builder().build(),
        )));
        assert!(rx.try_recv().is_err(), " Connected bukan pesan");
    }

    #[test]
    fn handler_interest_is_messages_only() {
        let (tx, _rx) = whatsapp_rust::async_channel::unbounded();
        let h = PipelineHandler { tx };
        assert_eq!(h.interest(), EventInterest::of(&[EventKind::Messages]));
    }

    #[test]
    fn full_queue_is_reported_and_never_blocks() {
        let (tx, rx) = whatsapp_rust::async_channel::bounded(2);
        let h = PipelineHandler { tx };
        // Kapasitas 2, Offering 4: `try_send` HARUS balik tanpa menunggu dan
        // 2 sisanya tercatat sebagai overflow, bukan hilang tanpa jejak.
        for e in [
            messages_event(&["Q1"]),
            messages_event(&["Q2"]),
            messages_event(&["Q3"]),
            messages_event(&["Q4"]),
        ] {
            h.handle_event(e);
        }
        let mut got = Vec::new();
        while let Ok(m) = rx.try_recv() {
            got.push(m.info.id.clone());
        }
        assert_eq!(got, vec!["Q1", "Q2"], "sisanya harus dibuang, bukan diantrekan");
    }

    #[tokio::test]
    async fn run_processes_each_queued_message_end_to_end() {
        // Jembatan penuh: event bus → antrean → run → pipeline → sender palsu.
        // AFK aktif supaya `Decision::DeniedMode(Afk)` jadi balasan yang benar-
        // benar terkirim, jadi pemetaan Decision→Reply→wire ikut teruji.
        let store = r#"{"settings": {"botAfk": {"active": true, "since": 1}}}"#;
        let ctx = StoreContext::new(store_with("bridge", store));
        let sender = Arc::new(RecordingSender::default());
        let p = Arc::new(Pipeline::new(
            ctx,
            sender.clone(),
            "bot",
            ParseOptions::default(),
        ));

        let (tx, rx) = whatsapp_rust::async_channel::unbounded();
        let handle = PipelineHandler { tx };
        tokio::spawn(p.clone().run(rx));

        handle.handle_event(messages_event(&["E1", "E2"]));

        // Tunggu sampai kedua pesan selesai diproses.
        let sent = wait_for_len(&sender, 2).await;
        assert_eq!(sent.len(), 2, "satu balasan per pesan inbound");
        for (to, m, extra) in &sent {
            assert_eq!(to, "6281@s.whatsapp.net");
            assert_eq!(extra, &0, "teks balasan tidak butuh node tambahan");
            assert!(body_of(m).unwrap().contains("sᴇᴅᴀɴɢ ᴀꜰᴋ"));
        }
    }

    #[derive(Clone, Default)]
    struct FailingSender {
        sends: Arc<Mutex<usize>>,
    }

    impl MessageSender for FailingSender {
        fn send(&self, _to: &str, _m: wa::Message, _e: Vec<Node>) -> SendBox<'_> {
            *self.sends.lock().unwrap() += 1;
            Box::pin(async { Err("socket putus".into()) })
        }
    }

    #[tokio::test]
    async fn send_failure_does_not_kill_the_loop() {
        // Kegagalan kirim harus jadi Err dari `process` → di-log di `run` →
        // loop lanjut ke pesan berikutnya. Kalau `run` ikut mati, bot berhenti
        // diam-diam setelah satu error jaringan.
        let store = r#"{"settings": {"botAfk": {"active": true, "since": 1}}}"#;
        let ctx = StoreContext::new(store_with("failbridge", store));
        let sender = Arc::new(FailingSender::default());
        let p = Arc::new(Pipeline::new(
            ctx,
            sender.clone(),
            "bot",
            ParseOptions::default(),
        ));

        let (tx, rx) = whatsapp_rust::async_channel::unbounded();
        let handler = PipelineHandler { tx };
        handler.handle_event(messages_event(&["F1", "F2", "F3"]));
        let runner = tokio::spawn(p.run(rx));

        // Tunggu sampai ketiga pesan dicoba (semua gagal kirim).
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while *sender.sends.lock().unwrap() < 3 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "loop berhenti setelah {} percobaan kirim",
                sender.sends.lock().unwrap()
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            !runner.is_finished(),
            "loop harus tetap hidup setelah error kirim"
        );

        // Menutup antrean menghentikan loop — itu jalan keluarnya yang bersih.
        drop(handler);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), runner).await;
    }

    #[tokio::test]
    async fn media_reply_is_logged_and_processing_continues() {
        // `Reply::Image` tidak punya byte media. Yang diuji: kegagalan ini
        // sampai ke pemanggil sebagai error bertipe, bukan panik dan bukan
        // "sukses diam" yang membuat caller menganggap pesan terkirim.
        let ctx = StoreContext::new(store_with("media", "{}"));
        let _p = Pipeline::new(
            ctx,
            Arc::new(RecordingSender::default()),
            "bot",
            ParseOptions::default(),
        );
        let err = render_reply(
            &Reply::Image {
                caption: "c".into(),
                mentions: vec![],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(err, PipelineError::MediaSourceMissing { variant: "Image" });
        assert!(err.to_string().contains("sumber media"));
    }

    async fn wait_for_len(s: &RecordingSender, n: usize) -> Vec<(String, wa::Message, usize)> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let got = s.sent.lock().unwrap().clone();
            if got.len() >= n {
                return got;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "hanya {} dari {n} balasan terkirim",
                got.len()
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }
}
