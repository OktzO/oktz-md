//! Lapisan WhatsApp produksi. Kode di sini dipromosikan dari spike yang sudah
//! terverifikasi end-to-end (impor 514/514 sesi, decrypt pesan nyata, tombol
//! WA ter-render). Spike di `rust-spike/` adalah throwaway; ini yang dipakai bot.

pub mod bot;
pub mod cards;
pub mod creds;
pub mod load;
pub mod pairing;
pub mod pairing_run;
pub mod rss;
pub mod session;

pub use bot::{build_device, connect_device, prepare_store};
pub use cards::{biz_node, build_card, build_list};
pub use creds::{DeviceMapping, inspect_creds};
pub use load::{LoadReport, load_into_store};
pub use session::{ImportStats, import_session_record};
pub use whatsapp_rust::bot::BotHandle;

// Re-export untuk konsumen (oktz-bin) supaya tidak perlu depend ke whatsapp-rust langsung.
pub use whatsapp_rust::wacore::proto_helpers::MessageBuilderExt;
pub use whatsapp_rust::waproto::whatsapp::Message;
pub use whatsapp_rust::{Jid, SendOptions};
