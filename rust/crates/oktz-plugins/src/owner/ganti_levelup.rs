//! `plugins/owner/ganti-levelup.js` — ganti gambar levelup.jpg.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `m.download()` / `m.quoted.download()` (media intake yang
//! tidak ada di `Ctx`), `updateAssetUrl` dari `src/lib/uploader.js` (upload
//! ke host eksternal + menulis ulang `config.js` lewat `fs`, termasuk
//! `sharp` resize ke JPEG), dan `m.reply` aliran (`⏳ Sedang mengupload...`
//! sebelum hasil). Satu `#[plugin]` juga hanya menangani satu nama command,
//! sementara JS mendaftarkan `ganti-levelup.jpg` dengan alias
//! `gantilevelup`/`setlevelup`.
//!
//! `ponytail:` kalau `download_media` dan uploader tersedia di `Ctx` nanti,
//! port ulang dari `plugins/owner/ganti-levelup.js` (guard isImage, unduh,
//! upload, tampilkan URL baru).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ganti-levelup.jpg",
    alias = ["gantilevelup", "setlevelup"],
    category = "owner",
    description = "Ganti gambar levelup.jpg (DIBLOKIR: butuh download media + uploader + tulis config)",
    usage = ".ganti-levelup.jpg (reply/kirim gambar)",
    example = ".ganti-levelup.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_levelup(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ganti-levelup.jpg").unwrap();
        assert_eq!(p.meta.name, "ganti-levelup.jpg");
        assert_eq!(p.meta.alias, ["gantilevelup", "setlevelup"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn nama_command_menyimpan_titik_pertipe_js() {
        // `name` JS literal `ganti-levelup.jpg`, bukan disamarkan ke
        // `ganti_levelup` — titik penting untuk manifest plugin.
        assert_eq!(
            oktz_plugin_api::find("ganti-levelup.jpg").unwrap().meta.name,
            "ganti-levelup.jpg"
        );
    }
}
