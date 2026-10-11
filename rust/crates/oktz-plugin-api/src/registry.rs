use std::sync::{Arc, OnceLock};

use crate::meta::PluginMeta;

pub type HandlerFn = Arc<dyn Fn(Ctx) -> Reply + Send + Sync>;

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
    /// Image dengan caption.
    ///
    /// BELUM bisa dikirim: variant ini tidak membawa bytes media maupun
    /// handle-nya. Pipeline mengembalikan `MediaSourceMissing` alih-alih
    /// mengirim apa-apa. Butuh penambahan sumber media (URL / path /
    /// upload handle) sebelum plugin class C bisa dipakai.
    Image {
        caption: String,
        mentions: Vec<String>,
    },
    /// Sticker (WA `stickerMessage`).
    Sticker,
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

    /// `true` kalau plugin tidak mau membalas.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Teks balasan kalau ada (router butuh ini untuk log/permission reply).
    pub fn text_body(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } | Self::TextQuoted { text, .. } => Some(text),
            Self::Image { caption, .. } => Some(caption),
            Self::Interactive { body } => Some(body),
            Self::None | Self::Sticker => None,
        }
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
                caption: "c".into(),
                mentions: vec![]
            }
            .text_body(),
            Some("c")
        );
        // Sticker tidak punya teks — router tidak boleh reply teks kosong.
        assert_eq!(Reply::Sticker.text_body(), None);
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