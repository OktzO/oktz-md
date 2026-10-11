//! Lapisan WhatsApp produksi. Kode di sini dipromosikan dari spike yang sudah
//! terverifikasi end-to-end (impor 514/514 sesi, decrypt pesan nyata, tombol
//! WA ter-render). Spike di `rust-spike/` adalah throwaway; ini yang dipakai bot.

pub mod bot;
pub mod cards;
pub mod creds;
pub mod load;
pub mod rss;
pub mod session;

pub use bot::{build_device, connect_device};
pub use cards::{biz_node, build_card, build_list};
pub use creds::{DeviceMapping, inspect_creds};
pub use load::{LoadReport, load_into_store};
pub use session::{ImportStats, import_session_record};
pub use whatsapp_rust::bot::BotHandle;
