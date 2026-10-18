//! `plugins/game/werewolf.js` — game Werewolf social deduction.
//!
//! Catatan: `pub mod werewolf;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/game/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `global.werewolfGames` state antar-pesan per grup (room, phase, role,
//!   vote, nightAction, timeout `setTimeout`): state volatil per-pesan tidak
//!   diekspos di handler tunggal (`session_*` adalah per-key global, bukan
//!   game state kompleks dengan timeout).
//! - `sock.sendPreview(...)` dengan `jpegThumbnail` + `previewType` +
//!   `contextInfo` newsletter: tidak ada di `Reply`.
//! - PM ke setiap player (`sock.sendPreview(player.id, ...)` untuk role):
//!   pengiriman ke JID lain selain `m.chat` bukan bagian dari `Reply`.
//! - `db.updateKoin` / `db.getUser` / `db.setUser` untuk reward:
//!   `Mutation::SetUser` tersedia, tapi logika update koin/exp per player perlu
//!   koordinasi mutation pipeline.
//! - Night action handler PM (`wwkill`/`wwprotect`/`wwsee`/`wwsorcerer`) sebagai
//!   command terpisah (`wwkill`, `wwprotect`, `wwsee`, `wwsorcerer` di
//!   `game/mod.rs`): bukan bagian dari handler `werewolf` ini.
//! - `config.isOwner?.(m.sender)`: tidak ada di `Ctx` (hanya `is_owner` bool).
