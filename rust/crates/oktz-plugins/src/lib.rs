//! Plugin yang sudah ter-port — 1:1 dari `plugins/*.js` versi JS yang terbukti jalan.
//!
//! Cuma class A (logika murni, tanpa HTTP/media) sampai sekarang. Kalau plugin
//! butuh I/O atau WA fitur khusus, jangan dipaksa ke sini — class B/C/D butuh
//! API sendiri (lihat `docs/rust-rewrite/PLUGIN-CLASSIFICATION.md`).
//!
//! Peta:
//! - `cek`  — 10 plugin `cek*` dengan pola 4-tier seragam (90/70/50/30).
//! - `cek2` — 6 plugin `cek*` yang tidak seragam: ambang dan offset RNG beda.
//! - `umum` — 14 plugin class A lain (fun/tools), termasuk `tanya_plugin!`.

pub mod cek;
pub mod cek2;
pub mod umum;
