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
    /// Card/button list interaktif (WA native flow).
    Interactive { body: String },
    /// Reaksi emoji pada pesan yang memicu (JS `m.react`).
    ///
    /// 502 plugin JS memakainya, jadi absen di enum berarti setiap plugin itu
    /// kehilangan konfirmasi. Reaksi tidak selalu mengirim teks balasan,
    /// jadi ini berdiri sendiri, bukan tempelan pada `Text`.
    Reaction { emoji: String, reply: Box<Reply> },
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
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mentions: Vec::new(),
        }
    }

    pub fn text_mention(text: impl Into<String>, mention: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mentions: vec![mention.into()],
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
    pub fn is_none(&self) -> bool {
        matches!(self.without_mutations(), Self::None)
    }

    /// `true` kalau reply ini butuh upload media sebelum bisa dikirim.
    pub fn has_media(&self) -> bool {
        matches!(
            self.without_mutations(),
            Self::Image { .. } | Self::Sticker { .. }
        )
    }

    /// Sumber media yang di-carry, kalau ada. `None` = tidak ada sumber yang
    /// bisa dikirim — pemanggil WAJIB memperlakukan itu sebagai error.
    pub fn media_source(&self) -> Option<&MediaSource> {
        match self.without_mutations() {
            Self::Image { media, .. } | Self::Sticker { media } => media.as_ref(),
            _ => None,
        }
    }

    /// Teks balasan kalau ada (router butuh ini untuk log/permission reply).
    pub fn text_body(&self) -> Option<&str> {
        match self.without_mutations() {
            Self::Text { text, .. } | Self::TextQuoted { text, .. } => Some(text),
            Self::Image { caption, .. } => Some(caption),
            Self::Interactive { body } => Some(body),
            Self::Reaction { reply, .. } => reply.text_body(),
            // `Sticker` tidak punya teks — router tidak boleh reply teks kosong.
            // Cabang `Mutate` mustahil (`without_mutations` sudah turun),
            // tapi tetap ditulis supaya menambah variant baru tidak diam-diam
            // jadi teks kosong yang terkirim ke user.
            Self::None | Self::Sticker { .. } | Self::Mutate { .. } => None,
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

/// Konteks yang diterima handler.
///
/// Cerminan subset yang dipakai plugin class A. Field JS lain (`sock`, `db`,
/// `config`) sengaja tidak ada di sini: plugin yang butuh I/O、网络 atau media
/// belong ke class B/C/D dan akan dapat API sendiri nanti. Jangan diam-diam
/// menambah field — kalau port plugin butuh,itu keputusan sadar.
#[derive(Debug, Clone, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

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

    fn participant(jid: &str, admin: bool, owner: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: owner,
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
}