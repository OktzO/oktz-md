//! `plugins/sticker/brat.js` — menu variant brat dan generator sticker brat.
//! `pub mod brat;` perlu ditambah manual ke `src/sticker/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `wrapInteractive`/`prepareWAMessageMedia`/`generateWAMessageFromContent`
//!   (lib `onigis`) dan `getAssetBuffer` tidak tersedia di kontrak API.
//! - `sock.waUploadToServer` dan `sock.relayMessage` tidak ada pada `Sock`.
//! - `fetchBratFromAPI`/`generateBrat`/`isBlankImage`/`parseBratArgs`
//!   (`src/lib/brat.js`) tidak ter-port.
//! - `Reply::Interactive { body }` hanya teks, tidak bisa menampung tombol
//!   interaktif dari JS.
//! Akibatnya seluruh fungsi `brat` (menu interaktif + generasi gambar)
//! tidak dapat di-port apa adanya.
