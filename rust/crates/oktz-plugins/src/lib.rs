#![allow(special_module_name)]

// `plugins/main/` adalah kategori plugin JS, bukan binary crate kita —
// makanya `pub mod main;` di bawah bukan salah. Lint `special_module_name`
// hanya bisa ditekan di level crate: `#[allow]` pada item-nya sendiri
// diabaikan rustc.
pub mod ai;
pub mod anime;
pub mod canvas;
pub mod cek;
pub mod clan;
pub mod common;
pub mod download;
pub mod ephoto;
pub mod fun;
pub mod game;
pub mod group;
pub mod info;
pub mod main;
pub mod nsfw;
pub mod owner;
pub mod panel;
pub mod primbon;
pub mod random;
pub mod religi;
pub mod stalker;
pub mod rpg;
pub mod search;
pub mod sticker;
pub mod store;
pub mod tools;
pub mod user;
pub mod utility;
