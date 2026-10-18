pub mod meta;
pub mod registry;
mod ureq_http;

pub use meta::PluginMeta;
pub use ureq_http::UreqHttp;
pub use oktz_plugin_macros::{answer_handler, plugin};

// `sock` JS (cek nomor, foto profil, operasi grup). Kontraknya di sini,
// implementasinya di crate `oktz-wa`.
pub mod sock;
pub use sock::{AksiPeserta, AdaWhatsapp, Sock, SockError, SockMati};

// Padanan `fs` dan ffmpeg Node untuk plugin yang menulis berkas atau
// memotong media.
pub mod berkas;
pub mod ffmpeg;
pub mod gambar;

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
