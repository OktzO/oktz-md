//! `plugins/download/tiktokdl2.js` — download video/slide TikTok tanpa watermark.
//! `pub mod tiktokdl2;` perlu ditambah manual ke `src/download/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `cheerio` (parse HTML dari savett.cc) dan pemroongan CSRF/cookie
//!   multi-langkah tidak tersedia di kontrak API (parser mini per berkas
//!   belum disediakan).
//! - Scraping `savett.cc` (token CSRF, cookie, formatselect, slides,
//!   album via `onigis`) sangat tergantung struktur HTML pihak ketiga.
//! Akibatnya downloader TikTok tidak dapat di-port apa adanya.
