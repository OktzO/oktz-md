pub mod meta;
pub mod registry;
mod ureq_http;

pub use meta::PluginMeta;
pub use ureq_http::UreqHttp;
pub use oktz_plugin_macros::{answer_handler, plugin};

// Model kartu interaktif (native flow). Ada di plugin-api, bukan di
// `oktz-wa`, supaya plugin bisa menyusun kartu tanpa dependensi ke crate
// yang memegang protokol WA.
pub mod kartu;
pub use kartu::{AksiTombol, Baris, Bagian, Daftar, Kartu, KartuHeader, Tombol};

// Tabel AI (WA `richResponseMessage`) — padanan `sock.sendTable` JS.
pub mod tabel;
pub use tabel::Tabel;

// Balasan rich: V2, list, AI rich, prepared rich. Empat wire contract yang
// BERBEDA di JS, jadi empat model yang berdiri sendiri di sini.
pub mod rich;
pub use rich::{AiSubMessage, PreparedRich, RichAi, RichList, RichV2};

// `sock` JS (cek nomor, foto profil, operasi grup). Kontraknya di sini,
// implementasinya di crate `oktz-wa`.
pub mod sock;
pub use sock::{AksiPeserta, AdaWhatsapp, Sock, SockError, SockMati};

// Padanan `fs` dan ffmpeg Node untuk plugin yang menulis berkas atau
// memotong media.
pub mod berkas;
pub mod ffmpeg;
pub mod gambar;
pub mod kanvas;
pub use kanvas::Kanvas;
pub mod sticker;
pub use sticker::{
    OpsiS, StickerOpsi, STICKER_WEBP_VF, gambar_ke_stiker_webp, opsi_s, tempel_metadata_stiker,
    video_ke_stiker_webp,
};

// Plugin butuh menyusun `Runtime` sendiri di test (untuk menguji jalur
// config terisi), jadi tipe yang dibaca `Ctx::config()` harus bisa diakses
// tanpa menambah dependensi baru per crate plugin.
pub use oktz_core::config;
pub use registry::{
    AnswerHandlerFn, Ctx, HandlerFn, HttpBody, HttpCall, HttpError, HttpGet, HttpMethod,
    HttpResponse, IncomingKind, IncomingMedia, MediaError, MediaFetch, MediaOrigin, MediaSource,
    MultipartPart, Mutation, NoHttp, NoMediaFetch, Participant, PluginEntry, QuotedKind,
    QuotedMessage, Reply, SessionStore, all_answer_handlers, all_plugins, count, find,
    new_session_store, run_answer_handlers,
};
