//! `plugins/user/daftar.js` — daftar user melalui sesi reply interaktif 4 langkah
//! (nama → umur → gender → konfirmasi), dengan database dan `sock`.
//!
//! `ponytail:` `getDatabase()`, `db.setUser()`, `db.updateKoin/Exp/Energi()`,
//! `db.save()`, dan `sock.sendMessage`/`m.react` tidak diekspos ke plugin di
//! registry Rust. Tidak ada store, tidak ada sesi, dan tidak ada pengiriman
//! pesan. Seluruh handler di bawah jatuh ke error template — penyimpangan
//! disengaja yang perlu diimplementasikan ulang saat API database/sock tersedia.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

#[plugin(
    name = "daftar",
    alias = ["register", "reg"],
    category = "user",
    description = "Daftar sebagai user bot melalui sesi reply interaktif",
    usage = ".daftar",
    example = ".daftar",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn daftar(ctx: Ctx) -> Reply {
    // `ponytail:` Semua logika daftar (cek isRegistered, sesi registrasi,
    // buat sesi, kirim prompt dengan gambar, m.react) butuh database dan sock
    // yang tidak tersedia di registry Rust.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn selalu_error_tanpa_database_dan_sock() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "daftar".into(),
            ..Ctx::default()
        };
        let r = daftar(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "daftar")
        );
    }
}
