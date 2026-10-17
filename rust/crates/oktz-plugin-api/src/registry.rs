use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::Value;

use crate::meta::PluginMeta;

pub type HandlerFn = Arc<dyn Fn(Ctx) -> Reply + Send + Sync>;

/// Dari mana byte media [`Reply::Image`] / [`Reply::Sticker`] diambil.
///
/// Tiga bentuk, dipilih dari plugin JS yang benar-benar ada di repo — bukan
/// tebakan. Bentuk JS tidak bisa dipakai apa adanya karena whatsapp-rust hanya
/// menerima byte: `Client::upload` butuh `Vec<u8>`
/// (`whatsapp-rust-0.7.0/src/upload.rs:409`), tidak ada "kirim dari URL".
///
/// - `Bytes` — hasil canvas/API yang sudah di-buffer di memori
///   (`plugins/canvas/facepalm.js` `image: buffer`,
///   `plugins/search/bingimage.js` `image: Buffer.from(res.data)`).
/// - `Url` — `image: { url: ... }` dengan URL http(s): avatar, thumbnail, hasil
///   API (`plugins/stalker/githubstalk.js`, `plugins/group/welcome.js`,
///   `plugins/random/anime.js`).
/// - `Path` — file lokal. Di JS path diteruskan sebagai `{ url: outPath }` dan
///   Baileys yang membacanya dari disk (`plugins/canvas/buatquotes.js`,
///   `plugins/canvas/kalender.js`, `plugins/panel/linode.js`
///   `image: 'linode/ubuntu20.04'`). Dipisah dari `Url` karena jalurnya
///   benar-benar berbeda — baca disk vs HTTP GET; menyatukannya berarti menebak
///   yang mana yang dimaksud plugin, dan tebakan yang salah = file lokal
///   terkirim lewat jaringan atau sebaliknya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaSource {
    /// Byte sudah di memori.
    Bytes {
        data: Arc<[u8]>,
        mimetype: Option<String>,
    },
    /// URL http(s), di-fetch saat kirim.
    Url {
        url: String,
        mimetype: Option<String>,
    },
    /// Path file lokal, dibaca saat kirim.
    Path {
        path: String,
        mimetype: Option<String>,
    },
}

impl MediaSource {
    pub fn bytes(data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes {
            data: Arc::from(data.into()),
            mimetype: None,
        }
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self::Url {
            url: url.into(),
            mimetype: None,
        }
    }

    pub fn path(path: impl Into<String>) -> Self {
        Self::Path {
            path: path.into(),
            mimetype: None,
        }
    }

    pub fn with_mimetype(mut self, mimetype: impl Into<String>) -> Self {
        let m = Some(mimetype.into());
        match &mut self {
            Self::Bytes { mimetype, .. }
            | Self::Url { mimetype, .. }
            | Self::Path { mimetype, .. } => *mimetype = m,
        }
        self
    }

    pub fn mimetype(&self) -> Option<&str> {
        match self {
            Self::Bytes { mimetype, .. }
            | Self::Url { mimetype, .. }
            | Self::Path { mimetype, .. } => mimetype.as_deref(),
        }
    }

    /// Ringkasan bentuk sumber untuk pesan error/log. Sengaja TIDAK memuat
    /// URL atau path: URL media sering membawa token bertanda tangan dan path
    /// bisa absolut — keduanya tidak belong di log.
    pub fn label(&self) -> String {
        match self {
            Self::Bytes { data, .. } => format!("bytes({})", data.len()),
            Self::Url { .. } => "url".to_string(),
            Self::Path { .. } => "path".to_string(),
        }
    }
}

/// Satu perubahan state yang diminta plugin.
///
/// Setara salah satu dari pemanggilan tulis di `src/lib/database.js`:
/// `setUser` (`:536`), `setGroup` (`:717`), `setting(key, value)` (`:753`).
///
/// Bentuknya disengaja mengikuti granularity yang benar-benar dipakai plugin,
/// bukan "patch JSON bebas": hanya tiga operasi store yang ada di
/// `database.js`, dan ketiganya punya semantik default sendiri
/// (lihat [`Mutation`] di `oktz-wa`). Tidak ada `set_users`/arbitrary key
/// karena tidak ada padanannya di JS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    /// `db.setUser(jid, data)` (`database.js:536-603`).
    ///
    /// `data` adalah objek partial — field yang tidak ada di `data` memakai
    /// nilai lama, persis operator `??` di JS.
    SetUser { jid: String, data: Value },
    /// `db.setGroup(jid, data)` (`database.js:717-746`).
    SetGroup { jid: String, data: Value },
    /// `db.setting(key, value)` (`database.js:753-758`).
    SetSetting { key: String, value: Value },
}

/// Balasan plugin.
///
/// JS tidak `return` apa pun — plugin memanggil `m.reply(...)` / `sock.sendMessage(...)`
/// sebagai efek samping, lalu `return` `void`. Di sini dibalik jadi nilai balik supaya
/// router yang decide mengirim apa (dan plugin tetap pure + mudah dites).
///
/// Varietas dipilih dari plugin nyata, bukan tebakan: 178 plugin memanggil
/// `sock.sendMessage`, dan sebagian besar mengirim `text` + `mentions`.
/// Media ada karena ~22 plugin mengirim image/sticker/document — variant
/// `Image` dipakai class C (media), `Interactive` untuk card WA.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Reply {
    /// Tidak membalas (JS: handler yang hanya efek samping atau `return`).
    #[default]
    None,
    /// Balasan teks biasa. `mentions` diisijid yang harus di-mention (WA
    /// `contextInfo.mentionedJid`), bukan string — supaya router yang transmit.
    Text {
        text: String,
        mentions: Vec<String>,
    },
    /// Balasan teks + reply ke pesan yang memicu (JS `m.reply`).
    TextQuoted {
        text: String,
        mentions: Vec<String>,
    },
    /// `quoted: false` (`serialize.js:1000`) — balasan TIDAK mengutip pesan
    /// pemicu. Mention tetap mengikuti aturan default.
    NoQuote {
        text: String,
        mentions: Vec<String>,
    },
    /// `mentions: []` (`serialize.js:994`) — mengutip tapi TIDAK menyebut
    /// pengirim. Plugin yang menulis `mentions: []` di JS/No, tapi JS tetap
    /// mengutip; jadi ini bukan "tanpa quote".
    TextNoMention {
        text: String,
    },
    /// Image dengan caption, dari sumber media nyata.
    ///
    /// `media: None` = plugin tidak punya sumber yang bisa dikirim. Itu
    ///Reported sebagai `PipelineError::MediaSourceMissing`, TIDAK diam-diam
    /// dibuang. `Some` = pipeline baca byte-nya, upload ke CDN
    /// (`Client::upload`), lalu kirim `imageMessage`.
    Image {
        media: Option<MediaSource>,
        caption: String,
        mentions: Vec<String>,
    },
    /// Sticker (WA `stickerMessage`), dari sumber media nyata.
    ///
    /// Sama seperti `Image`: `None` = error eksplisit, bukan drop senyap.
    Sticker { media: Option<MediaSource> },
    /// Video (WA `videoMessage`) — `sock.sendMedia(..., { type: 'video' })`
    /// (`src/lib/socket.js:786-793`).
    ///
    /// `gif_playback` = `options.gifPlayback` yang ikut tersebar ke payload
    /// (`socket.js:793` — `...options`): WA memutar video berulang tanpa suara.
    /// Dipakai `asupan/*` dan `cek/cekfemboy` (9 pemanggil di plugins/).
    Video {
        media: Option<MediaSource>,
        caption: String,
        mentions: Vec<String>,
        gif_playback: bool,
    },
    /// Audio (WA `audioMessage`) — `{ type: 'audio' }` (`socket.js:795-805`).
    ///
    /// `ptt` = voice note (`options.ptt || false` di JS). Mimetype bawaannya
    /// `audio/mpeg` kalau `MediaSource` tidak membawa mimetype sendiri —
    /// `socket.js:798` mengganti `application/octet-stream` jadi itu.
    Audio {
        media: Option<MediaSource>,
        ptt: bool,
    },
    /// Dokumen (WA `documentMessage`) — bentuk DEFAULT `sock.sendMedia` tanpa
    /// `type` (`socket.js:805-812`): `fileName` bawaannya `"file"` dan
    /// mimetype bawaannya `application/octet-stream`.
    Document {
        media: Option<MediaSource>,
        file_name: String,
        caption: String,
        mentions: Vec<String>,
    },
    /// Card/button list interaktif (WA native flow).
    Interactive { body: String },
    /// Reaksi emoji pada pesan yang memicu (JS `m.react`).
    ///
    /// 502 plugin JS memakainya, jadi absen di enum berarti setiap plugin itu
    /// kehilangan konfirmasi. Reaksi tidak selalu mengirim teks balasan,
    /// jadi ini berdiri sendiri, bukan tempelan pada `Text`.
    Reaction { emoji: String, reply: Box<Reply> },
    /// Beberapa pesan berurutan dari satu pemanggilan handler.
    ///
    /// Plugin JS yang memanggil `m.reply`/`sock.sendMedia` beberapa kali tidak
    /// punya padanan sebelum ini — satu `Reply` hanya jadi satu pesan. Vektor
    /// kosong berarti "tidak balas", sama seperti `None`.
    ///
    /// Pipeline mengirim berurutan dan berhenti di error pertama, jadi pesan
    /// setelahnya tidak terkirim — meniru `await` berurutan di JS.
    Sequence(Vec<Reply>),
    /// Balasan PLUS mutasi store yang harus ditulis pipeline.
    ///
    /// Dipisah dari variant lain supaya 87 plugin lama yang `-> Reply` tidak
    /// perlu berubah sama sekali, dan supaya "balas tapi tidak menyimpan"
    /// tetap mungkin: `Mutate { reply: None }` = simpan tanpa kirim pesan
    /// (mis. `m.react('✅')` di JS yang tidak punya teks balasan).
    ///
    /// Kotak di dalam kotak: `Reply` adalah enum tanpa pointer, jadi
    /// recursive tanpa alokasi tambahan butuh `Box`. `new`/`with_mutation`
    /// meratakan nesting, jadi plugin tidak pernah melihat bentuk dalam.
    Mutate {
        reply: Box<Reply>,
        mutations: Vec<Mutation>,
    },
}

impl Reply {
    /// Konstruktor yang paling sering dipakai — teks polos tanpa mention.
    ///
    /// Mention dan kutipan TIDAK ditentukan di sini: `serialize.js` menambah
    /// keduanya di sisi pengiriman (`mentionedJid: options?.mentions ||
    /// [m.sender] || []` dan `quotedMsg = options.quoted !== false ? msg :
    /// undefined`), jadi `render_reply` yang memutuskan.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mentions: Vec::new(),
        }
    }

    /// Beberapa pesan berurutan. Vektor kosong diratakan jadi
    /// [`Self::None`] supaya `is_none` benar tanpa penanganan khusus.
    pub fn sequence(list: Vec<Reply>) -> Self {
        if list.iter().all(Reply::is_none) {
            return Self::None;
        }
        Self::Sequence(list)
    }

    /// Bentuk `m.reply` berulang: vektor teks polos, mention mengikuti default.
    pub fn sequence_text(list: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::sequence(list.into_iter().map(|t| Reply::text(t.into())).collect())
    }

    pub fn text_mention(text: impl Into<String>, mention: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mentions: vec![mention.into()],
        }
    }

    /// `quoted: false` — plugin ini memang tidak mau mengutip pemicunya.
    pub fn text_quoted(text: impl Into<String>) -> Self {
        Self::NoQuote {
            text: text.into(),
            mentions: Vec::new(),
        }
    }

    /// `mentions: []` — mengutip pemicu tapi tidak menyebut siapa pun.
    pub fn text_no_mention(text: impl Into<String>) -> Self {
        Self::TextNoMention {
            text: text.into(),
        }
    }

    /// Image siap kirim. Caption kosong berarti tanpa caption — WA menerima
    /// `imageMessage` tanpa `caption`, tapi `Some("")` tidak.
    pub fn image(source: MediaSource, caption: impl Into<String>) -> Self {
        Self::Image {
            media: Some(source),
            caption: caption.into(),
            mentions: Vec::new(),
        }
    }

    pub fn sticker(source: MediaSource) -> Self {
        Self::Sticker {
            media: Some(source),
        }
    }

    /// Video biasa — `sock.sendMedia(..., { type: 'video' })`.
    pub fn video(source: MediaSource, caption: impl Into<String>) -> Self {
        Self::Video {
            media: Some(source),
            caption: caption.into(),
            mentions: Vec::new(),
            gif_playback: false,
        }
    }

    /// Video mode GIF — `{ type: 'video', gifPlayback: true }`.
    pub fn video_gif(source: MediaSource, caption: impl Into<String>) -> Self {
        Self::Video {
            media: Some(source),
            caption: caption.into(),
            mentions: Vec::new(),
            gif_playback: true,
        }
    }

    /// Audio biasa — `{ type: 'audio' }`, bukan voice note.
    pub fn audio(source: MediaSource) -> Self {
        Self::Audio {
            media: Some(source),
            ptt: false,
        }
    }

    /// Voice note — `{ type: 'audio', ptt: true }`.
    pub fn voice_note(source: MediaSource) -> Self {
        Self::Audio {
            media: Some(source),
            ptt: true,
        }
    }

    /// Dokumen dengan nama berkas yang dilihat penerima.
    pub fn document(source: MediaSource, file_name: impl Into<String>) -> Self {
        Self::Document {
            media: Some(source),
            file_name: file_name.into(),
            caption: String::new(),
            mentions: Vec::new(),
        }
    }

    /// Set caption pada balasan media.
    ///
    /// `Image`/`Video` menerima caption sejak konstruktor, tapi `Document`
    /// tidak — JS sering mengirim `document` + `caption` sekaligus
    /// (`socket.js:756-820`), jadi tanpa ini plugin harus membentuk varian
    /// enum secara manual.
    ///
    /// Caption kosong TIDAK dipasang (WA menolak `Some("")`), sama seperti
    /// konstruktor [`Reply::image`].
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        let caption = caption.into();
        if caption.is_empty() {
            return self;
        }
        match &mut self {
            Self::Image { caption: c, .. }
            | Self::Video { caption: c, .. }
            | Self::Document { caption: c, .. } => *c = caption,
            _ => return self,
        }
        self
    }

    /// Bungkus balasan dengan mutasi store, diratakan.
    ///
    /// `Mutate` di dalam `Mutate` digabung ke vektor yang sama, jadi
    /// `with_mutation` boleh dipanggil berkali-kali tanpa membentuk
    /// sarang `Box` yang tidak perlu.
    pub fn mutating(self, mutations: Vec<Mutation>) -> Self {
        if mutations.is_empty() {
            return self;
        }
        match self {
            Self::Mutate {
                reply,
                mutations: mut existing,
            } => {
                existing.extend(mutations);
                Self::Mutate {
                    reply,
                    mutations: existing,
                }
            }
            other => Self::Mutate {
                reply: Box::new(other),
                mutations,
            },
        }
    }

    /// Menambah satu mutasi. Kalau reply ini sudah `Mutate`, digabung —
    /// tidak pernah membentuk `Mutate { reply: Mutate { .. } }`.
    pub fn with_mutation(self, mutation: Mutation) -> Self {
        self.mutating(vec![mutation])
    }

    /// Tambahkan reaksi pada pesan pemicu (JS `m.react(emoji)`).
    ///
    /// Reaksi dibungkus `Mutate` yang sudah ada, jadi tidak menambah field
    /// baru dan tidak mengubah bentuk plugin mana pun.
    pub fn with_reaction(self, emoji: impl Into<String>) -> Self {
        let (reply, mutations) = self.into_parts();
        Self::Mutate {
            reply: Box::new(Self::Reaction {
                emoji: emoji.into(),
                reply: Box::new(reply),
            }),
            mutations,
        }
    }
    /// Pisahkan jadi (balasan, mutasi). Reply biasa selalu mutate kosong.
    pub fn into_parts(self) -> (Self, Vec<Mutation>) {
        match self {
            Self::Mutate { reply, mutations } => (*reply, mutations),
            other => (other, Vec::new()),
        }
    }

    /// Balasan tanpa mutasi — untuk pipeline yang sudah menulis mutasi dan
    /// sekarang hanya perlu merender teksnya.
    pub fn without_mutations(&self) -> &Self {
        let mut cur = self;
        while let Self::Mutate { reply, .. } = cur {
            cur = reply;
        }
        cur
    }

    /// Mutasi yang dibawa reply ini (Kosong kalau bukan `Mutate`).
    pub fn mutations(&self) -> &[Mutation] {
        match self {
            Self::Mutate { mutations, .. } => mutations,
            _ => &[],
        }
    }

    /// `true` kalau plugin tidak mau membalas.
    ///
    /// `Mutate { reply: None }` = simpan tanpa balas, jadi ikut `true`.
    /// `Sequence` kosong juga `true` — tidak ada yang perlu dikirim.
    pub fn is_none(&self) -> bool {
        match self.without_mutations() {
            Self::None => true,
            Self::Sequence(list) => list.iter().all(Self::is_none),
            Self::Text { .. }
            | Self::TextQuoted { .. }
            | Self::NoQuote { .. }
            | Self::TextNoMention { .. }
            | Self::Image { .. }
            | Self::Sticker { .. }
            | Self::Video { .. }
            | Self::Audio { .. }
            | Self::Document { .. }
            | Self::Interactive { .. }
            | Self::Reaction { .. }
            | Self::Mutate { .. } => false,
        }
    }

    /// [`Self::Mutate`] dan [`Self::Reaction`] diratakan: keduanya cuma
    /// pembungkus. `without_mutations` berhenti di `Reaction` karena pipeline
    /// perlu melihat reaksi itu sendiri; untuk mencari isi balasan, turunkan
    /// keduanya. Tanpa ini `Image.with_reaction(..)` terbaca sebagai "tidak
    /// ada media" padahal pipeline tetap meng-upload-nya.
    fn payload(&self) -> &Self {
        let mut cur = self;
        loop {
            cur = match cur {
                Self::Mutate { reply, .. } | Self::Reaction { reply, .. } => reply,
                other => return other,
            };
        }
    }

    /// `true` kalau reply ini butuh upload media sebelum bisa dikirim.
    pub fn has_media(&self) -> bool {
        // `Sequence` jadi `true` kalau satu elemen pun punya media.
        if let Self::Sequence(list) = self.payload() {
            return list.iter().any(Self::has_media);
        }
        matches!(
            self.payload(),
            Self::Image { .. }
                | Self::Sticker { .. }
                | Self::Video { .. }
                | Self::Audio { .. }
                | Self::Document { .. }
        )
    }

    /// Sumber media yang di-carry, kalau ada. `None` = tidak ada sumber yang
    /// bisa dikirim — pemanggil WAJIB memperlakukan itu sebagai error.
    ///
    /// `Sequence` sengaja mengembalikan `None` walau ada isinya: satu sequence
    /// bisa punya beberapa sumber berbeda, jadi pemanggil harus mengiterasi
    /// elemennya dan memanggil fungsi ini per elemen.
    pub fn media_source(&self) -> Option<&MediaSource> {
        match self.payload() {
            Self::Image { media, .. }
            | Self::Sticker { media }
            | Self::Video { media, .. }
            | Self::Audio { media, .. }
            | Self::Document { media, .. } => media.as_ref(),
            _ => None,
        }
    }

    /// Teks balasan kalau ada (router butuh ini untuk log/permission reply).
    pub fn text_body(&self) -> Option<&str> {
        match self.without_mutations() {
            Self::Text { text, .. }
            | Self::TextQuoted { text, .. }
            | Self::NoQuote { text, .. }
            | Self::TextNoMention { text } => Some(text),
            Self::Image { caption, .. } => Some(caption),
            Self::Video { caption, .. } => Some(caption),
            Self::Document { caption, .. } => Some(caption),
            Self::Interactive { body } => Some(body),
            Self::Reaction { reply, .. } => reply.text_body(),
            // `Sticker`/`Audio` tidak punya teks — router tidak boleh reply
            // teks kosong. Cabang `Mutate` mustahil (`without_mutations` sudah
            // turun), tapi tetap ditulis supaya menambah variant baru tidak
            // diam-diam jadi teks kosong yang terkirim ke user.
            // `Sequence` text_body = teks pesan PERTAMA yang punya teks; router
            // cuma butuh satu baris untuk log/permission. `None` kalau
            // tidak ada isinya.
            Self::Sequence(list) => list.iter().find_map(Self::text_body),
            Self::None | Self::Sticker { .. } | Self::Audio { .. } | Self::Mutate { .. } => None,
        }
    }
}

/// Satu anggota grup — cerminan `m.groupMetadata.participants[]` di JS.
///
/// Datar, bukan struct bersarang, karena plugin JS hanya memakai tiga hal dari
/// tiap peserta: `.jid`, `.admin === 'admin'`, `.admin === 'superadmin'`
/// (`plugins/group/listadmin.js:25,32,33`). Bentuk wire-nya persis sama:
/// `admin` string `null`/`"admin"`/`"superadmin"`
/// (`wacore-0.7.0/src/iq/groups.rs:184` — `ParticipantType`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Participant {
    /// JID peserta persis seperti di wire (`…@s.whatsapp.net` atau `…@lid`).
    pub jid: String,
    /// `admin` atau `superadmin`.
    pub is_admin: bool,
    /// Hanya `superadmin` — pemilik grup. JS: `a.admin === 'superadmin'`.
    pub is_owner: bool,
    /// `db.getUser(p.jid)` — di JS tiap iterasi roster memanggilnya sendiri.
    /// `Null` kalau peserta belum pernah ada di store.
    pub data: Value,
}

/// Roster grup untuk satu pesan.
///
/// Diisi pipeline dari metadata grup yang benar-benar ada, TIDAK diambil plugin
/// sendiri: plugin tidak punya `sock`, dan roster yang dibaca plugin di saat
/// lain bisa berbeda dari yang dipakai gate permission untuk pesan yang sama.
///
/// Kosong (`Default`) berarti "tidak diketahui" — bukan grup, metadata tidak
/// bisa dibaca, atau `Client` belum tersambung. Plugin WAJIB memeriksa
/// `.is_empty()`, sama seperti JS yang memeriksa `participants.length === 0`
/// (`plugins/group/tagall.js:31`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupRoster {
    pub participants: Vec<Participant>,
    /// Bot sendiri punya hak admin. JS: `!!botParticipant?.admin`
    /// (`plugins/owner/onlythisgrup.js:43`). `false` berarti "tidak tahu atau
    /// bukan admin" — bukan bukti bahwa bot pasti bukan admin.
    pub is_bot_admin: bool,
}

impl GroupRoster {
    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    /// JID semua anggota, urutan wire apa adanya. Bahan `tagall`
    /// (`plugins/group/tagall.js:29-47`).
    pub fn member_jids(&self) -> Vec<String> {
        self.participants.iter().map(|p| p.jid.clone()).collect()
    }

    /// Admin + superadmin — `participants.filter(p => p.admin)`.
    pub fn admins(&self) -> impl Iterator<Item = &Participant> {
        self.participants.iter().filter(|p| p.is_admin)
    }

    /// Superadmin saja — `admins.find(a => a.admin === 'superadmin')`.
    pub fn owners(&self) -> impl Iterator<Item = &Participant> {
        self.participants.iter().filter(|p| p.is_owner)
    }

    /// `true` kalau JID itu admin (superadmin ikut dihitung).
    pub fn is_admin(&self, jid: &str) -> bool {
        self.participants.iter().any(|p| p.is_admin && p.jid == jid)
    }
}

/// `m.quoted.type`. Baileys menyimpan nama field protobuf mentah dan plugin
/// membandingkan persis string itu, jadi `wire_type()` tidak diterjemahkan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotedKind {
    Text,
    Image,
    Video,
    Audio,
    Sticker,
    Document,
    Contact,
    Location,
    Reaction,
    Poll,
    Other(String),
}

impl QuotedKind {
    pub fn wire_type(&self) -> &str {
        match self {
            Self::Text => "conversation",
            Self::Image => "imageMessage",
            Self::Video => "videoMessage",
            Self::Audio => "audioMessage",
            Self::Sticker => "stickerMessage",
            Self::Document => "documentMessage",
            Self::Contact => "contactMessage",
            Self::Location => "locationMessage",
            Self::Reaction => "reactionMessage",
            Self::Poll => "pollCreationMessage",
            Self::Other(s) => s,
        }
    }

    pub fn is_media(&self) -> bool {
        matches!(
            self,
            Self::Image | Self::Video | Self::Audio | Self::Sticker | Self::Document
        )
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image)
    }
    pub fn is_video(&self) -> bool {
        matches!(self, Self::Video)
    }
    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Audio)
    }
    pub fn is_sticker(&self) -> bool {
        matches!(self, Self::Sticker)
    }
    pub fn is_document(&self) -> bool {
        matches!(self, Self::Document)
    }
}

/// `m.quoted` — pesan yang di-reply pengirim.
///
/// Bacaan saja: method Baileys (`download()`, `delete()`, `react()`) butuh I/O
/// dan tidak ikut di-port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotedMessage {
    pub id: String,
    /// Participant asli di grup, bukan pengirim reply.
    pub sender: String,
    pub chat: String,
    /// Teks atau caption; `""` kalau media tanpa caption.
    pub text: String,
    pub kind: QuotedKind,
    pub mimetype: Option<String>,
    pub file_name: Option<String>,
    pub from_me: bool,
}

impl QuotedMessage {
    pub fn is_media(&self) -> bool {
        self.kind.is_media()
    }
}

/// Konteks yang diterima handler.
///
/// Cerminan subset yang dipakai plugin class A. Field JS lain (`sock`, `db`,
/// `config`) sengaja tidak ada di sini: plugin yang butuh I/O, jaringan, atau media
/// belong ke class B/C/D dan akan dapat API sendiri nanti. Jangan diam-diam
/// menambah field — kalau port plugin butuh,itu keputusan sadar.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// `m.text` — semua teks setelah command word.
    pub text: String,
    /// `m.args` — token split whitespace, tidak pernah mengandung string kosong.
    pub args: Vec<String>,
    /// `m.sender` — jid pengirim.
    pub sender: String,
    /// `m.chat` — jid tujuan (grup atau private).
    pub chat: String,
    /// `m.mentionedJid` — jid yang di-mention di pesan.
    pub mentioned: Vec<String>,
    /// `m.command` — sudah lowercase.
    pub command: String,
    /// `m.prefix` — prefix yang cocok, `""` kalau mode noprefix.
    pub prefix: String,
    /// `m.quoted` — `None` kalau pengirim tidak me-reply.
    pub quoted: Option<QuotedMessage>,
    /// Map globalSessions milik JS. Keyed bebas, bukan per-plugin: beberapa
    /// plugin JS berbagi satu map yang sama.
    pub sessions: SessionStore,
    /// HTTP untuk plugin class B. Tidak pernah `None` — default-nya gagal dengan
    /// alasan jelas, bukan `Option` yang wajib di-unwrap di tiap plugin.
    pub http: Arc<dyn HttpGet>,
    pub is_group: bool,
    pub is_owner: bool,
    pub uptime_secs: u64,
    /// `m.groupMetadata.participants` — roster grup untuk pesan ini.
    /// Kosong kalau bukan grup atau metadata tidak bisa dibaca.
    pub roster: GroupRoster,
    /// Seluruh objek user dari store untuk `sender`, apa adanya.
    /// `Null` kalau user belum pernah ada. Plugin membacanya lewat
    /// [`Ctx::user_field`] / [`Ctx::user_list`], TIDAK lewat I/O sendiri —
    /// crate ini sengaja tidak punya dependensi store.
    pub user_data: Value,
    /// Seluruh objek grup dari store untuk `chat`, apa adanya.
    /// `Null` kalau grup belum pernah ada.
    pub group_data: Value,
    /// Seluruh objek settings global dari store (`settings.json`), apa adanya.
    /// `Null` kalau store belum punya key `settings`.
    ///
    /// Ini padanan `db.setting(key)` (`database.js:753-758`) — plugin yang
    /// butuh `storeLists`/`energi`/`capprem` membacanya lewat
    /// [`Ctx::setting`] dan turunan bertipe, bukan menggali JSON mentah.
    pub settings: Value,
    /// `m.isMedia` dkk untuk pesan PEMICU. `None` = pesan pemicu bukan media
    /// (teks biasa) — bukan berarti gagal dibaca.
    pub media: Option<IncomingMedia>,
    /// `m.download()` / `m.quoted.download()` (`serialize.js:1669`, `:543`).
    /// Selalu terisi; default-nya [`NoMediaFetch`] yang menolak dengan alasan.
    pub media_fetch: Arc<dyn MediaFetch>,
    /// `config.js` yang boleh dibaca plugin (`bot`, `owner`, `APIkey`,
    /// `assets`, `sticker`, `payment`, `donasi`, `saluran`, `info`).
    ///
    /// Default-nya [`oktz_core::config::Runtime::default`] yang SEMUA stringnya
    /// kosong — bukan nilai produksi. Tanpa itu plugin yang salah baca kunci
    /// akan diam-diam memakai API key milik orang lain di test.
    pub config: Arc<oktz_core::config::Runtime>,
}

impl Ctx {
    /// `m.isMedia` — pesan pemicu membawa media.
    pub fn is_media(&self) -> bool {
        self.media.is_some()
    }

    /// `m.type` — `"imageMessage"` dst; `None` kalau pesan pemicu bukan media.
    pub fn media_type(&self) -> Option<&'static str> {
        self.media.as_ref().map(|m| m.kind.wire())
    }

    /// `m.isImage` / `m.isVideo` / `m.isAudio` / `m.isSticker` / `m.isDocument`.
    pub fn is_media_kind(&self, kind: IncomingKind) -> bool {
        self.media.as_ref().is_some_and(|m| m.kind == kind)
    }

    /// `await m.download()` — byte mentah media pesan pemicu.
    pub fn download_media(&self) -> Result<Vec<u8>, MediaError> {
        self.media_fetch.download(MediaOrigin::Trigger)
    }

    /// `await m.quoted.download()` — byte mentah media pesan yang di-reply.
    pub fn download_quoted_media(&self) -> Result<Vec<u8>, MediaError> {
        self.media_fetch.download(MediaOrigin::Quoted)
    }

    /// `config.js` — nilai runtime yang dibaca plugin.
    pub fn config(&self) -> &oktz_core::config::Runtime {
        &self.config
    }
}

impl Ctx {
    /// Target mention pertama, atau sender — polanya yang dipakai plugin `cek*`
    /// (`m.mentionedJid?.[0] || m.sender`).
    pub fn mention_or_sender(&self) -> &str {
        self.mentioned
            .first()
            .map(String::as_str)
            .unwrap_or(&self.sender)
    }

    /// `m.quoted` — pesan yang di-reply, `None` kalau tidak reply.
    pub fn quoted(&self) -> Option<&QuotedMessage> {
        self.quoted.as_ref()
    }

    /// `global.sessions.get(key)` — clone, bukan referensi, supaya plugin
    /// tidak bisa memegang borrow `Ctx` lintas `Mutex`.
    pub fn session_get(&self, key: &str) -> Option<Value> {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    /// `global.sessions.set(key, value)`.
    pub fn session_set(&self, key: &str, value: Value) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), value);
    }

    /// Baca-kemudian-hapus dalam satu lock. `None` kalau key tidak ada.
    pub fn session_take(&self, key: &str) -> Option<Value> {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key)
    }

    /// Mutasi di bawah lock: `session_update(k, |v| …)`. `None` = key baru,
    /// jadi closure menerima `Value::Null`.
    pub fn session_update(&self, key: &str, f: impl FnOnce(&mut Value)) {
        let mut guard = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let entry = guard
            .entry(key.to_string())
            .or_insert(Value::Null);
        f(entry);
    }

    /// Nomor saja dari jid (`628123@s.whatsapp.net` → `628123`), untuk seed
    /// acak plugin `cek*` yang hanya butuh angka.
    pub fn sender_number(&self) -> &str {
        self.sender.split('@').next().unwrap_or(&self.sender)
    }

    /// Field user dari store dengan jalur titik: `ctx.user_field("rpg.stamina")`.
    /// `None` kalau user tidak ada, jalurnya bukan objek, atau bukan field itu.
    pub fn user_field(&self, path: &str) -> Option<&Value> {
        value_at(&self.user_data, path)
    }

    /// Field grup dari store dengan jalur titik: `ctx.group_field("game")`.
    pub fn group_field(&self, path: &str) -> Option<&Value> {
        value_at(&self.group_data, path)
    }

    /// Array dari store milik user, mis. `ctx.user_list("access")`.
    /// Slice kosong kalau bukan array — pemanggil tidak perlu mengecek dua kali.
    pub fn user_list(&self, key: &str) -> &[Value] {
        self.user_field(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
    }

    /// Array dari store milik grup, mis. `ctx.group_list("members")`.
    pub fn group_list(&self, key: &str) -> &[Value] {
        self.group_field(key)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    /// `db.setting(key)` (`database.js:753`) — nilai mentah, `None` kalau
    /// key tidak ada. Plugin yang butuh bentuk mentah pakai ini; yang butuh
    /// bentuk bertipe pakai turunan di bawah.
    pub fn setting(&self, key: &str) -> Option<&Value> {
        self.settings.get(key)
    }

    /// `db.setting(key)` sebagai bool, dengan `default` kalau key tidak ada
    /// atau nilainya bukan bool.
    ///
    /// `default` wajib diberikan, bukan hardcode `false`: `db.setting("energi")`
    /// yang belum pernah di-set berarti "belum decided", dan memutuskan itu
    /// hak caller — `energi_enabled_setting` di pipeline memakai default
    /// yang BERBEDA (`EnergiSettings::enabled`) dari plugin mana pun.
    pub fn setting_bool(&self, key: &str, default: bool) -> bool {
        self.setting(key).and_then(Value::as_bool).unwrap_or(default)
    }

    /// `db.setting(key)` sebagai integer. `None` kalau bukan angka.
    pub fn setting_i64(&self, key: &str) -> Option<i64> {
        self.setting(key).and_then(Value::as_i64)
    }

    /// `db.setting(key) || []` (`database.js`, plugin `store/*.js`).
    /// Slice kosong kalau key tidak ada atau bukan array.
    pub fn setting_list(&self, key: &str) -> &[Value] {
        self.setting(key)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    /// `db.setting(key) || {}` (`capfree.js`, `whatrolethis.js`).
    /// Mengembalikan entry `(key, value)` dengan urutan key apa adanya —
    /// persis urutan iterasi `Object.entries` di JS, yang dipakai plugin
    /// untuk menyusun daftar fitur.
    pub fn setting_entries(&self, key: &str) -> impl Iterator<Item = (&str, &Value)> {
        self.setting(key)
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| (k.as_str(), v)))
            .into_iter()
            .flatten()
    }

    /// `db.setting("capprem")?.[name]` — `middleware.js:65-69`.
    pub fn setting_bool_at(&self, key: &str, name: &str) -> Option<bool> {
        self.setting(key)?.get(name)?.as_bool()
    }
}

/// Ambil `Value` di jalur titik `"a.b.c"`. Segmen kosong dianggap programmer
/// error dan ditolak, bukan dianggap root — diam-diam mengembalikan objek
/// tersimpan utuh karena salah ketik adalah cara data bocor ke balasan.
fn value_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path.split('.') {
        if seg.is_empty() {
            return None;
        }
        cur = cur.get(seg)?;
    }
    Some(cur)
}

pub struct PluginEntry {
    pub meta: PluginMeta,
    pub handler: HandlerFn,
}

#[doc(hidden)]
pub type PluginInit = fn() -> &'static PluginEntry;

#[linkme::distributed_slice]
pub static PLUGINS: [PluginInit];

static CACHE: OnceLock<Vec<&'static PluginEntry>> = OnceLock::new();

pub fn all_plugins() -> &'static [&'static PluginEntry] {
    CACHE.get_or_init(|| PLUGINS.iter().map(|f| f()).collect())
}

pub fn count() -> usize {
    all_plugins().len()
}

pub fn find(command: &str) -> Option<&'static PluginEntry> {
    all_plugins()
        .iter()
        .copied()
        .find(|e| e.meta.matches(command))
}

// ---------------------------------------------------------------------------
// Entry point kedua: `answerHandler(m, sock)` (`src/handler.js:435-473`).
//
// Plugin game memakai jalur ini: jawaban main bukan command, jadi router
// biasa tidak akan pernah memanggilnya. Rantai berhenti di handler pertama
// yang mengembalikan `Some`, jadi `None` berarti "tidak menangani" — padanan
// `return false` di JS.
// ---------------------------------------------------------------------------

pub type AnswerHandlerFn = Arc<dyn Fn(Ctx) -> Option<Reply> + Send + Sync>;

#[doc(hidden)]
pub type AnswerInit = fn() -> &'static [AnswerHandlerFn];

#[linkme::distributed_slice]
pub static ANSWER_HANDLERS: [AnswerInit];

static ANSWER_CACHE: OnceLock<Vec<&'static [AnswerHandlerFn]>> = OnceLock::new();

/// Semua answer handler, urut tautan seperti urutan pemanggilan di JS.
pub fn all_answer_handlers() -> &'static [&'static [AnswerHandlerFn]] {
    ANSWER_CACHE.get_or_init(|| ANSWER_HANDLERS.iter().map(|f| f()).collect())
}

/// Jalankan rantai answer handler; `Some` dari yang pertama menang.
pub fn run_answer_handlers(ctx: Ctx) -> Option<Reply> {
    all_answer_handlers()
        .iter()
        .flat_map(|list| list.iter())
        .find_map(|f| f(ctx.clone()))
}

/// State volatil antar-pesan. Pakai suffix `""` kalau mau menyimpan JSON
/// utuh, bukan cuma [`Mutex`] biasa.
pub type SessionStore = Arc<std::sync::Mutex<HashMap<String, Value>>>;

pub fn new_session_store() -> SessionStore {
    Arc::new(std::sync::Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpBody {
    Bytes {
        data: Arc<[u8]>,
        mimetype: Option<String>,
    },
    Form(String),
    /// `form-data` multipart — `form.append("file", buf, { filename })` +
    /// `form.append("type", 13)`. Bentuk ini muncul di 10 scraper
    /// (`src/scraper/hd.js`, `imglarger`, `imgdrop`, `seaart`, `videy`, …)
    /// yang mengunggah buffer/gambar ke API pihak ketiga.
    Multipart(Vec<MultipartPart>),
}

/// Satu bagian `multipart/form-data`.
///
/// Field `file_name` adalah alasan variant ini ada: `post_multipart` lama
/// menamai part dengan NAMA FIELD (`form.append("file", buf)` jadi part
/// bernama `file`), sedangkan `form-data` JS mengirim `filename=` dari opsi
/// ketiga. Server yang memeriksa nama berkas menolak bentuk lama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartPart {
    /// Nama field (`form.append("file", …)` → `"file"`).
    pub name: String,
    /// `{ filename }` untuk part berkas; `None` = field biasa.
    pub file_name: Option<String>,
    /// `content-type` part; `None` = biarkan implementasi memilih.
    pub mimetype: Option<String>,
    pub body: HttpBody,
}

impl MultipartPart {
    /// `form.append(nama, nilai)` — field teks.
    pub fn field(name: impl Into<String>, value: impl Into<String>) -> Self {
        let value: String = value.into();
        Self {
            name: name.into(),
            file_name: None,
            mimetype: None,
            body: HttpBody::Bytes {
                data: Arc::from(value.into_bytes()),
                mimetype: None,
            },
        }
    }

    /// `form.append(nama, buf, { filename })` — part berkas.
    pub fn file(
        name: impl Into<String>,
        file_name: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            name: name.into(),
            file_name: Some(file_name.into()),
            mimetype: None,
            body: HttpBody::Bytes {
                data: Arc::from(data.into()),
                mimetype: None,
            },
        }
    }

    pub fn with_mimetype(mut self, mimetype: impl Into<String>) -> Self {
        self.mimetype = Some(mimetype.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: HttpBody,
    pub headers: Vec<(String, String)>,
}

impl HttpBody {
    /// Byte datar untuk body. `Multipart` TIDAK punya bentuk datar — yang
    /// dikembalikan slice kosong, dan pemanggil yang butuh multipart harus
    /// menanganinya sendiri (lihat `UreqHttp::call`).
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Bytes { data, .. } => data.as_ref(),
            Self::Form(s) => s.as_bytes(),
            Self::Multipart(_) => &[],
        }
    }
}

impl HttpResponse {
    pub fn json(&self) -> Option<Value> {
        serde_json::from_slice(self.body.bytes()).ok()
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(self.body.bytes()).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    pub message: String,
}

/// Metode HTTP untuk [`HttpCall`].
///
/// `HEAD` disertakan karena scraper `terabox`/`mediafire` memakainya untuk
/// mengecek tautan; `DELETE`/`PATCH` untuk API yang menolak selain POST/GET.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl HttpMethod {
    /// Nama wire, huruf besar seperti `axios` mengirimnya.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
        }
    }
}

/// Permintaan HTTP dengan kendali penuh — padanan objek config `axios({...})`.
///
/// Empat method lama (`get`/`post_json`/`post_form`/`post_multipart`) cuma
/// menjangkau bentuk yang dipakai plugin class B lama. Scraper di
/// `src/scraper/` memakai `axios` dengan `headers: {...}` bebas
/// (`X-CSRF-TOKEN`, `Cookie`, `Referer`, `X-Forwarded-For`), metode `PUT`, dan
/// body mentah; semuanya lewat satu pintu ini.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpCall {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<HttpBody>,
    /// Padanan `validateStatus: () => true` di axios: status 4xx/5xx **tidak**
    /// jadi error, isi bodinya tetap dibaca scraper. Bawaannya `false` — axios
    /// melempar untuk 4xx/5xx, dan itu perilaku yang harus disalin.
    pub terima_semua_status: bool,
}

impl HttpCall {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            terima_semua_status: false,
        }
    }

    /// `validateStatus: () => true` — lihat catatan di `terima_semua_status`.
    pub fn terima_semua_status(mut self) -> Self {
        self.terima_semua_status = true;
        self
    }

    pub fn get(url: impl Into<String>) -> Self {
        Self::new(HttpMethod::Get, url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new(HttpMethod::Post, url)
    }

    pub fn put(url: impl Into<String>) -> Self {
        Self::new(HttpMethod::Put, url)
    }

    /// Tambah header. Nama yang sama dua kali: yang lama dibuang — pemanggil
    /// terakhir menang, sama seperti objek `headers` JS yang tidak bisa punya
    /// kunci kembar.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.headers
            .retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
        self.headers.push((name, value.into()));
        self
    }

    /// Body JSON (`data: {...}` di axios). Content-type sengaja TIDAK dipasang
    /// di sini: implementasi klien yang memutuskan (lihat `UreqHttp::call`),
    /// dan pemanggil yang menulis header eksplisit tetap menang.
    pub fn json(mut self, value: &Value) -> Self {
        let data = serde_json::to_vec(value).unwrap_or_default();
        self.body = Some(HttpBody::Bytes {
            data: Arc::from(data),
            mimetype: Some("application/json".to_string()),
        });
        self
    }

    /// Body mentah dengan tipe isi eksplisit — `data: "url=..."` atau byte
    /// biner yang tidak masuk akal ditafsirkan.
    pub fn raw(mut self, data: impl Into<Vec<u8>>, content_type: &str) -> Self {
        self.body = Some(HttpBody::Bytes {
            data: Arc::from(data.into()),
            mimetype: Some(content_type.to_string()),
        });
        self
    }

    /// Body form-urlencoded (bentuk string yang sudah di-encode pemanggil).
    pub fn form(mut self, body: impl Into<String>) -> Self {
        self.body = Some(HttpBody::Form(body.into()));
        self
    }

    /// Body multipart — `form-data` JS (`form.append(...)`).
    pub fn multipart(mut self, parts: Vec<MultipartPart>) -> Self {
        self.body = Some(HttpBody::Multipart(parts));
        self
    }
}

/// Klien HTTP sinkron untuk plugin.
///
/// `ponytail:` trait, bukan implementasi. Default-nya [`NoHttp`] yang
/// selalu error, dipakai pipeline test/offline. `oktz-wa` yang menyediakan
/// [`UreqHttp`]-nya; pindah ke reqwest async tinggal ganti implementasinya.
pub trait HttpGet: Send + Sync + std::fmt::Debug {
    fn get(&self, url: &str) -> Result<HttpResponse, HttpError>;
    fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError>;
    fn post_form(&self, url: &str, form: &str) -> Result<HttpResponse, HttpError>;
    fn post_multipart(
        &self,
        url: &str,
        parts: &[(&str, HttpBody)],
    ) -> Result<HttpResponse, HttpError>;

    /// Permintaan umum: header kustom, metode apa pun, body mentah.
    ///
    /// Implementasi default merutekan ke empat method lama. Dua alasannya:
    /// stub `HttpGet` di ratusan test plugin lama tetap jalan tanpa disentuh,
    /// dan klien yang tidak bisa lebih (mis. `NoHttp`) tidak perlu menyalin
    /// method ke-5.
    ///
    /// Batas yang jujur: header kustom DIBUANG di jalur default — stub test
    /// yang mencocokkan URL tetap benar, tapi klien nyata WAJIB menimpa
    /// method ini kalau plugin memakai header. `UreqHttp` menimpanya.
    fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
        match (req.method, req.body.as_ref()) {
            (HttpMethod::Get, _) | (HttpMethod::Head, None) => self.get(&req.url),
            (_, Some(HttpBody::Multipart(_))) => Err(HttpError {
                message: "klien HTTP ini tidak mendukung multipart umum".to_string(),
            }),
            (HttpMethod::Post, Some(HttpBody::Form(form))) => self.post_form(&req.url, form),
            (HttpMethod::Post, Some(HttpBody::Bytes { data, .. }))
            | (HttpMethod::Put, Some(HttpBody::Bytes { data, .. })) => {
                match serde_json::from_slice::<Value>(data) {
                    Ok(v) => self.post_json(&req.url, &v),
                    Err(e) => Err(HttpError {
                        message: format!("body bukan JSON: {e}"),
                    }),
                }
            }
            (HttpMethod::Post, None) | (HttpMethod::Put, None) => {
                self.post_json(&req.url, &Value::Null)
            }
            (m, _) => Err(HttpError {
                message: format!("metode {} belum didukung klien HTTP ini", m.as_str()),
            }),
        }
    }
}

/// Default [`Ctx::http`]. Selalu gagal, tidak pernah jaringan.
#[derive(Debug, Default)]
pub struct NoHttp;

fn no_http<T>() -> Result<T, HttpError> {
    Err(HttpError {
        message: "HTTP tidak tersedia di konteks ini".into(),
    })
}

impl HttpGet for NoHttp {
    fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
        no_http()
    }
    fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
        no_http()
    }
    fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
        no_http()
    }
    fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
        no_http()
    }
}

/// Jenis media pada pesan pemicu — nilai `m.type` di JS
/// (`serialize.js:getContentType`). Selalu media: tipe non-media tidak masuk
/// sini sama sekali, dan `None` di [`Ctx::media`] yang mewakilinya.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingKind {
    Image,
    Video,
    Audio,
    Sticker,
    Document,
}

impl IncomingKind {
    /// `m.type` — `"imageMessage"` dst, apa adanya seperti JS.
    pub fn wire(self) -> &'static str {
        match self {
            Self::Image => "imageMessage",
            Self::Video => "videoMessage",
            Self::Audio => "audioMessage",
            Self::Sticker => "stickerMessage",
            Self::Document => "documentMessage",
        }
    }
}

/// Media yang menempel pada pesan pemicu — padanan `m.isMedia`, `m.isImage`,
/// `m.mimetype`, `m.fileName`, `m.body` untuk pesan media.
///
/// Yang TIDAK ada di sini: byte-nya (itu tugas [`MediaFetch`]) dan jalur
/// `m.type.replace("Message", "")` (pemanggil memakai [`IncomingKind`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingMedia {
    pub kind: IncomingKind,
    /// `m.mimetype` — `None` kalau pesan tidak menyebutkannya (sticker JS
    /// sering `image/webp` tanpa field, dan `socket.js:791` yang mengisinya).
    pub mimetype: Option<String>,
    /// `m.fileName` — hanya dokumen yang punya.
    pub file_name: Option<String>,
    /// `getMessageBody(m)` untuk pesan media: caption, `""` kalau tidak ada.
    pub caption: String,
}

/// Asal media yang diunduh — `m.download()` atau `m.quoted.download()`
/// (`serialize.js:1669` dan `:543`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaOrigin {
    /// Pesan yang memicu command (`m.download()`).
    Trigger,
    /// Pesan yang di-reply (`m.quoted.download()`).
    Quoted,
}

impl MediaOrigin {
    /// Nama yang dipakai pesan galat, biar pembaca tahu jalur mana yang gagal.
    pub fn label(self) -> &'static str {
        match self {
            Self::Trigger => "pesan pemicu",
            Self::Quoted => "pesan yang di-reply",
        }
    }
}

/// Kegagalan unduh media. Terpisah dari [`HttpError`] karena jalurnya bukan
/// HTTP biasa: media WA terenkripsi dan datang dari CDN WhatsApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaError {
    pub message: String,
}

/// Unduh isi media pesan — kapabilitas `m.download()`.
///
/// JS mengembalikan `null` kalau pesannya bukan media; di sini pemanggil
/// memeriksa [`Ctx::media`] lebih dulu (itu `m.isMedia`), dan `Err` muncul
/// kalau memang tidak ada media atau CDN menolak.
pub trait MediaFetch: Send + Sync + std::fmt::Debug {
    fn download(&self, origin: MediaOrigin) -> Result<Vec<u8>, MediaError>;
}

/// Default [`Ctx::media_fetch`]: selalu gagal, tidak pernah menyentuh jaringan.
///
/// Dipasang pipeline kalau pengirim pesan bukan `Client` WA sungguhan (test,
/// stub) — sama seperti [`NoHttp`], gagal dengan alasan jelas lebih baik
/// daripada byte palsu.
#[derive(Debug, Default)]
pub struct NoMediaFetch;

impl MediaFetch for NoMediaFetch {
    fn download(&self, origin: MediaOrigin) -> Result<Vec<u8>, MediaError> {
        Err(MediaError {
            message: format!(
                "unduh media {} tidak tersedia di konteks ini",
                origin.label()
            ),
        })
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            text: String::new(),
            args: Vec::new(),
            sender: String::new(),
            chat: String::new(),
            mentioned: Vec::new(),
            command: String::new(),
            prefix: String::new(),
            quoted: None,
            sessions: new_session_store(),
            http: Arc::new(NoHttp),
            is_group: false,
            is_owner: false,
            uptime_secs: 0,
            roster: GroupRoster::default(),
            user_data: Value::Null,
            group_data: Value::Null,
            settings: Value::Null,
            media: None,
            media_fetch: Arc::new(NoMediaFetch),
            config: Arc::new(oktz_core::config::Runtime::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_kosong_supaya_test_tidak_bocor_api_key() {
        let c = Ctx::default();
        assert_eq!(c.config().bot.name, "");
        assert_eq!(c.config().api_keys.firefly, "");
        assert_eq!(c.config().assets.get("foto"), None);
        assert_eq!(c.config().saluran.id, "");
    }

    #[test]
    fn text_reply_has_no_mentions_by_default() {
        let r = Reply::text("halo");
        assert_eq!(r.text_body(), Some("halo"));
        assert!(!r.is_none());
    }

    #[test]
    fn mention_reply_carries_the_jid_not_a_string() {
        let r = Reply::text_mention("hai", "628123@s.whatsapp.net");
        match &r {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628123@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn set_user(jid: &str, koin: i64) -> Mutation {
        Mutation::SetUser {
            jid: jid.into(),
            data: serde_json::json!({ "koin": koin }),
        }
    }

    #[test]
    fn a_mutation_carries_its_payload_and_renders_the_wrapped_reply() {
        let r = Reply::text("tersimpan").with_mutation(set_user("628123@s.whatsapp.net", 5));
        assert_eq!(r.text_body(), Some("tersimpan"), "teks harus dari reply");
        assert_eq!(r.mutations().len(), 1);
        assert_eq!(
            r.mutations()[0],
            set_user("628123@s.whatsapp.net", 5),
            "mutasi harus utuh, tidak berubah oleh pembungkusan"
        );
        assert!(!r.is_none());
    }

    #[test]
    fn mutations_flatten_instead_of_nesting_boxes() {
        let r = Reply::text("x")
            .with_mutation(set_user("a@s.whatsapp.net", 1))
            .with_mutation(Mutation::SetSetting {
                key: "energi".into(),
                value: serde_json::json!(true),
            })
            .with_mutation(Mutation::SetGroup {
                jid: "g".into(),
                data: serde_json::json!({}),
            });
        assert_eq!(r.mutations().len(), 3, "tiga mutasi, bukan tiga sarang");
        let (inner, muts) = r.into_parts();
        assert_eq!(inner.text_body(), Some("x"));
        assert_eq!(muts.len(), 3);
    }

    // `Reply::Mutate { reply: None }` = plugin menyimpan tanpa membalas
    // (padanan `await m.react('✅')` di JS yang tidak punya teks).
    #[test]
    fn storing_without_replying_is_still_a_reply_that_says_nothing() {
        let r = Reply::None.with_mutation(set_user("a@s.whatsapp.net", 1));
        assert!(r.is_none(), "tidak ada pesan yang harus dikirim");
        assert_eq!(r.text_body(), None);
        assert_eq!(r.mutations().len(), 1, "tapi mutasinya tetap ada");
    }

    #[test]
    fn an_empty_mutation_list_does_not_wrap_the_reply() {
        let r = Reply::text("x").mutating(Vec::new());
        assert_eq!(r, Reply::text("x"), "harus identik, bukan Mutate kosong");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn media_accessors_see_through_the_mutation_wrapper() {
        let r = Reply::image(MediaSource::url("https://x/y.png"), "cap")
            .with_mutation(set_user("a@s.whatsapp.net", 1));
        assert!(r.has_media());
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("url"),
            "sumber media tidak boleh hilang di balik Mutate"
        );
        assert_eq!(r.text_body(), Some("cap"));
    }

    #[test]
    fn settings_are_read_by_key_with_typed_defaults() {
        let ctx = Ctx {
            settings: serde_json::json!({
                "energi": true,
                "capprem": {"ping": false, "tebak": true},
                "storeLists": [{"name": "S&K"}, {"name": "Order"}],
                "storeTrxCounter": 7,
            }),
            ..Default::default()
        };
        assert!(ctx.setting_bool("energi", false));
        // Key yang tidak ada dapat default yang diminta caller, bukan `false`
        // diam-diam: pipeline memakai default ENERGI sendiri.
        assert!(ctx.setting_bool("tidakAda", true));
        assert!(!ctx.setting_bool("tidakAda", false));
        assert_eq!(ctx.setting_i64("storeTrxCounter"), Some(7));
        assert_eq!(ctx.setting_list("storeLists").len(), 2);
        assert_eq!(
            ctx.setting_list("storeLists")[0]["name"].as_str(),
            Some("S&K")
        );
        assert_eq!(ctx.setting_bool_at("capprem", "ping"), Some(false));
        assert_eq!(ctx.setting_bool_at("capprem", "tebak"), Some(true));
        assert_eq!(ctx.setting_bool_at("capprem", "nope"), None);
    }

    #[test]
    fn missing_settings_yield_defaults_never_a_panic() {
        // `Ctx::default()` = settings belum pernah ada di store — persis
        // `db.setting("storeLists")` mengembalikan `undefined`.
        let ctx = Ctx::default();
        assert_eq!(ctx.setting("energi"), None);
        assert!(ctx.setting_bool("energi", true));
        assert_eq!(ctx.setting_i64("storeTrxCounter"), None);
        assert!(ctx.setting_list("storeLists").is_empty());
        assert_eq!(ctx.setting_entries("capprem").count(), 0);
        assert_eq!(ctx.setting_bool_at("capprem", "ping"), None);

        // Tipe salah juga tidak boleh panic — plugin JS Mana pun yang
        // menulis string ke key ini akan melihat `undefined`, bukan crash.
        let ctx = Ctx {
            settings: serde_json::json!({"energi": "ya", "storeLists": 5}),
            ..Default::default()
        };
        assert!(!ctx.setting_bool("energi", false), "bukan bool -> default");
        assert!(ctx.setting_list("storeLists").is_empty(), "bukan array -> kosong");
        assert_eq!(ctx.setting_entries("energi").count(), 0, "bukan objek -> kosong");
    }

    #[test]
    fn setting_entries_preserve_key_order_and_dotted_access() {
        let ctx = Ctx {
            settings: serde_json::json!({"capprem": {"z": false, "a": true}}),
            ..Default::default()
        };
        let e: Vec<_> = ctx.setting_entries("capprem").collect();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].0, "z", "urutan key harus sama seperti di JSON");
        assert_eq!(e[1].0, "a");
    }

    #[test]
    fn none_is_default_and_reports_empty_body() {
        let r = Reply::default();
        assert!(r.is_none());
        assert_eq!(r.text_body(), None);
    }

    #[test]
    fn media_variants_expose_a_caption_for_logging() {
        assert_eq!(
            Reply::Image {
                media: None,
                caption: "c".into(),
                mentions: vec![]
            }
            .text_body(),
            Some("c")
        );
        // Sticker tidak punya teks — router tidak boleh reply teks kosong.
        assert_eq!(Reply::Sticker { media: None }.text_body(), None);
    }

    #[test]
    fn media_reply_carries_its_source() {
        let r = Reply::image(MediaSource::url("https://x/y.png"), "halo");
        assert!(r.has_media());
        assert_eq!(r.media_source().map(MediaSource::label).as_deref(), Some("url"));
        assert_eq!(r.media_source().and_then(MediaSource::mimetype), None);

        let with_mime = MediaSource::path("/tmp/a.png").with_mimetype("image/png");
        assert_eq!(with_mime.mimetype(), Some("image/png"));
        assert_eq!(
            Reply::sticker(with_mime).media_source().map(MediaSource::label).as_deref(),
            Some("path")
        );
    }

    #[test]
    fn reply_without_media_reports_no_source() {
        assert!(!Reply::text("x").has_media());
        assert_eq!(Reply::text("x").media_source(), None);
        assert_eq!(Reply::None.media_source(), None);
        // `media: None` tetap variant media — sumbernya yang hilang, dan itu
        // yang harus jadi error di pipeline, bukan `has_media() == false`.
        assert!(Reply::Sticker { media: None }.has_media());
        assert_eq!(Reply::Sticker { media: None }.media_source(), None);
    }

    #[test]
    fn media_source_label_hides_url_and_path() {
        assert_eq!(MediaSource::bytes(vec![1, 2, 3]).label(), "bytes(3)");
        assert_eq!(MediaSource::url("https://h/token123").label(), "url");
        assert_eq!(MediaSource::path("/home/user/a.png").label(), "path");
    }

    #[test]
    fn media_survives_the_reaction_wrapper() {
        let wrapped = Reply::image(MediaSource::bytes(vec![9]), "cap")
            .with_reaction("✅");
        assert!(wrapped.has_media());
        assert_eq!(wrapped.media_source().map(MediaSource::label).as_deref(), Some("bytes(1)"));
        // Reaksi tanpa balasan tetap bukan media.
        assert!(!Reply::None.with_reaction("✅").has_media());
    }

    fn participant(jid: &str, admin: bool, owner: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: owner,
            data: Default::default(),
        }
    }

    #[test]
    fn roster_separates_owner_from_admin() {
        let r = GroupRoster {
            participants: vec![
                participant("1@s.whatsapp.net", true, true),
                participant("2@s.whatsapp.net", true, false),
                participant("3@s.whatsapp.net", false, false),
            ],
            is_bot_admin: true,
        };
        assert_eq!(r.admins().count(), 2, "superadmin ikut terhitung admin");
        assert_eq!(r.owners().count(), 1);
        assert_eq!(r.member_jids().len(), 3);
        assert!(r.is_admin("2@s.whatsapp.net"));
        assert!(r.is_admin("1@s.whatsapp.net"));
        assert!(!r.is_admin("3@s.whatsapp.net"));
        assert!(r.is_bot_admin);
        assert!(!r.is_empty());
    }

    #[test]
    fn default_roster_is_empty_not_a_claim_of_no_members() {
        let r = GroupRoster::default();
        assert!(r.is_empty());
        assert!(!r.is_bot_admin);
        assert_eq!(r.member_jids(), Vec::<String>::new());
    }

    #[test]
    fn stored_state_is_read_by_dotted_path() {
        let ctx = Ctx {
            user_data: serde_json::json!({"rpg": {"stamina": 30, "bank": 0}, "koin": 5}),
            group_data: serde_json::json!({"game": false, "members": ["a", "b"]}),
            ..Default::default()
        };
        assert_eq!(ctx.user_field("rpg.stamina").and_then(|v| v.as_i64()), Some(30));
        assert_eq!(ctx.user_field("rpg.bank").and_then(|v| v.as_i64()), Some(0));
        assert_eq!(ctx.user_field("koin").and_then(|v| v.as_i64()), Some(5));
        assert_eq!(ctx.group_field("game").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(ctx.group_list("members").len(), 2);
        assert_eq!(ctx.user_field("rpg.hp"), None);
        assert_eq!(ctx.user_field(""), None, "segmen kosong harus ditolak");
        assert_eq!(ctx.user_field("rpg."), None);
        assert_eq!(ctx.user_field("rpg.stamina.x"), None);
    }

    #[test]
    fn missing_stored_state_yields_empty_reads_not_a_panic() {
        // `Default` = `Value::Null`, sama seperti user/grup yang belum pernah
        // ada di store.
        let ctx = Ctx::default();
        assert_eq!(ctx.user_field("rpg.stamina"), None);
        assert_eq!(ctx.group_field("game"), None);
        assert!(ctx.user_list("access").is_empty());
        assert!(ctx.group_list("members").is_empty());
        // Tidak ada state, tapi roster tetap bisa dipakai kalau diisi manual.
        let ctx = Ctx {
            roster: GroupRoster {
                participants: vec![participant("1@s.whatsapp.net", false, false)],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        assert_eq!(ctx.roster.member_jids(), vec!["1@s.whatsapp.net".to_string()]);
        assert_eq!(ctx.user_field("koin"), None);
    }

    #[test]
    fn mention_or_sender_falls_back_to_sender() {
        let mut ctx = Ctx {
            sender: "628999@s.whatsapp.net".into(),
            ..Default::default()
        };
        assert_eq!(ctx.mention_or_sender(), "628999@s.whatsapp.net");
        ctx.mentioned = vec!["628111@s.whatsapp.net".into()];
        assert_eq!(ctx.mention_or_sender(), "628111@s.whatsapp.net");
    }

    #[test]
    fn sender_number_strips_the_server() {
        let ctx = Ctx {
            sender: "628123456789:5@s.whatsapp.net".into(),
            ..Default::default()
        };
        assert_eq!(ctx.sender_number(), "628123456789:5");
    }

    /// Klien lama: cuma menimpa empat method asli, seperti ratusan stub test
    /// plugin yang sudah ada. `call` bawaan harus tetap merutekan ke sana.
    #[derive(Debug, Default)]
    struct KlienLama {
        jejak: std::sync::Mutex<Vec<String>>,
    }

    impl KlienLama {
        fn catat(&self, s: String) -> Result<HttpResponse, HttpError> {
            self.jejak.lock().unwrap().push(s);
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from("{}".as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    impl HttpGet for KlienLama {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.catat(format!("get:{url}"))
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.catat(format!("post_json:{url}:{body}"))
        }
        fn post_form(&self, url: &str, form: &str) -> Result<HttpResponse, HttpError> {
            self.catat(format!("post_form:{url}:{form}"))
        }
        fn post_multipart(
            &self,
            url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            self.catat(format!("post_multipart:{url}"))
        }
    }

    #[test]
    fn call_bawaan_merute_ke_method_lama() {
        let klien = KlienLama::default();
        klien.call(&HttpCall::get("https://x/a")).unwrap();
        klien
            .call(&HttpCall::post("https://x/b").form("a=1"))
            .unwrap();
        klien
            .call(&HttpCall::post("https://x/c").json(&serde_json::json!({"k": 1})))
            .unwrap();
        let jejak = klien.jejak.lock().unwrap().clone();
        assert_eq!(
            jejak,
            vec![
                "get:https://x/a",
                "post_form:https://x/b:a=1",
                "post_json:https://x/c:{\"k\":1}",
            ]
        );
    }

    #[test]
    fn call_bawaan_menolak_metode_yang_tidak_bisa_dirutekan() {
        let klien = KlienLama::default();
        let err = klien
            .call(&HttpCall::new(HttpMethod::Delete, "https://x/d"))
            .unwrap_err();
        assert!(err.message.contains("DELETE"), "{}", err.message);
    }

    #[test]
    fn header_dengan_nama_sama_diganti_bukan_ditumpuk() {
        let call = HttpCall::get("https://x")
            .header("Referer", "lama")
            .header("referer", "baru");
        assert_eq!(
            call.headers,
            vec![("referer".to_string(), "baru".to_string())]
        );
    }
    #[test]
    fn sequence_kirim_berurutan_dan_teks_pertama_yang_punya_teks() {
        let r = Reply::sequence_text(["satu", "dua", "tiga"]);
        assert!(!r.is_none());
        assert_eq!(r.text_body(), Some("satu"));
        let Reply::Sequence(list) = &r else {
            panic!("harus Sequence, dapat {r:?}");
        };
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn sequence_kosong_diratakan_jadi_none() {
        assert_eq!(Reply::sequence(Vec::new()), Reply::None);
        assert_eq!(Reply::sequence_text(Vec::<String>::new()), Reply::None);
        // Elemen yang semuanya diam juga dianggap tidak membalas.
        let campur = Reply::sequence(vec![Reply::None, Reply::None]);
        assert_eq!(campur, Reply::None);
    }

    #[test]
    fn sequence_dengan_media_terdeteksi_punya_media() {
        let tanpa = Reply::sequence_text(["a"]);
        assert!(!tanpa.has_media());
        let dengan = Reply::sequence(vec![
            Reply::text("foto dulu"),
            Reply::image(
                MediaSource::Url {
                    url: "https://a/b.jpg".into(),
                    mimetype: None,
                },
                "",
            ),
        ]);
        assert!(dengan.has_media());
        // Sumbernya per elemen, bukan di tingkat sequence.
        assert!(dengan.media_source().is_none());
    }

    #[test]
    fn sequence_membungkus_mutasi_dan_reaksi_seperti_varian_lain() {
        let r = Reply::sequence_text(["a"]).mutating(vec![]);
        assert_eq!(r.text_body(), Some("a"));
        let isi = Reply::sequence(vec![Reply::None]).with_reaction("OK");
        assert_eq!(isi.mutations().len(), 0);
    }
}