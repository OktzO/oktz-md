//! `plugins/owner/srt.js` — Sistem Shuffle Reply Thumb (SRT).
//! `pub mod srt;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `global.srtSession` (state antarm pesan di memori global) dan `setTimeout`
//!   TTL 10 menit tidak tersedia (`sessions` ada tetapi per-plugin/keyed,
//!   bukan global per-chat dengan TTL otomatis).
//! - `crypto` MD5 + `sharp` resize tersedia, namun `onigis`
//!   (`generateWAMessageFromContent`, `generateWAMessage`, album message,
//!   `sock.waUploadToServer`, `sock.relayMessage`) dan `jidNormalizedUser`
//!   tidak tersedia pada `Reply`/`Sock`.
//! Akibatnya SRT tidak dapat di-port apa adanya.
