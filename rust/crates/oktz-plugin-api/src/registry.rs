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
    /// Tidak membalas (JS: handler yang hanya副作用 samping atau `return`).
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

    /// `true` kalau plugin tidak mau membalas.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// `true` kalau reply ini butuh upload media sebelum bisa dikirim.
    pub fn has_media(&self) -> bool {
        matches!(self, Self::Image { .. } | Self::Sticker { .. })
    }

    /// Sumber media yang di-carry, kalau ada. `None` = tidak ada sumber yang
    /// bisa dikirim — pemanggil WAJIB memperlakukan itu sebagai error.
    pub fn media_source(&self) -> Option<&MediaSource> {
        match self {
            Self::Image { media, .. } | Self::Sticker { media } => media.as_ref(),
            _ => None,
        }
    }

    /// Teks balasan kalau ada (router butuh ini untuk log/permission reply).
    pub fn text_body(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } | Self::TextQuoted { text, .. } => Some(text),
            Self::Image { caption, .. } => Some(caption),
            Self::Interactive { body } => Some(body),
            Self::None | Self::Sticker { .. } => None,
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