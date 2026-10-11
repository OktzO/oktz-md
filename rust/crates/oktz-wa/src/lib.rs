//! Lapisan WhatsApp produksi. Kode di sini dipromosikan dari spike yang sudah
//! terverifikasi end-to-end (impor 514/514 sesi, decrypt pesan nyata, tombol
//! WA ter-render). Spike di `rust-spike/` adalah throwaway; ini yang dipakai bot.

pub mod bot;
pub mod cards;
pub mod creds;
pub mod load;
pub mod rss;
pub mod session;

pub use bot::{build_device, BotHandle, connect_device};
pub use cards::{build_card, build_list, biz_node};
pub use creds::{inspect_creds, DeviceMapping};
pub use load::{load_into_store, LoadReport};
pub use session::{import_session_record, ImportStats};