//! `plugins/owner/restart.js` — restart bot process (real restart).
//! `pub mod restart;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `child_process.spawn` dan `process.exit` (restart proses nyata) tidak
//!   tersedia di kontrak API plugin (tidak ada kontrol proses).
//! Akibatnya restart bot tidak dapat di-port apa adanya.
