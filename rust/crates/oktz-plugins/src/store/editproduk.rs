//! `plugins/store/editproduk.js` — edit produk toko (hanya di private chat).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "✏️ *EDIT PRODUK*\n\n📋 Format: `{prefix}editproduk <nomor> <field> <nilai>`\n\n📌 *Field yang bisa diedit:*\n• *nama* 🏷️ — Nama produk\n• *harga* 💰 — Harga jual (angka)\n• *diskon* 🏷️ — Harga asli/coret (angka, 0 untuk hapus)\n• *stok* 📊 — Jumlah stok atau `unlimited`\n• *tipe* 🔑📦 — `digital` atau `fisik`\n• *deskripsi* 📝 — Deskripsi produk\n• *detail* 🔒 — Info rahasia (dikirim setelah beli)\n• *gambar* 🖼️ — Upload gambar baru (reply gambar)\n• *video* 🎬 — Upload video baru (reply video)\n\n📝 *Contoh:*\n`{prefix}editproduk 1 harga 30000`\n`{prefix}editproduk 1 diskon 40000`\n`{prefix}editproduk 1 tipe fisik`\n`{prefix}editproduk 1 nama Netflix Premium`\n`{prefix}editproduk 1 deskripsi Akun sharing 1 bulan`\n`{prefix}editproduk 1 gambar` (reply gambar 🖼️)\n\n🏷️ _Harga diskon akan ditampilkan sebagai ~~harga asli~~ di katalog_",
        prefix = prefix
    )
}

#[plugin(
    name = "editproduk",
    alias = ["editproduct"],
    category = "store",
    description = "✏️ Edit produk toko (hanya di private chat)",
    usage = ".editproduk <nomor> <field> <nilai>",
    example = ".editproduk 1 harga 30000",
    cooldown = 3,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn editproduk(ctx: Ctx) -> Reply {
    // `editproduk.js` butuh:
    //   1. `getDatabase()` + `db.setting('storeProducts')` / `db.setting(key, v)`
    //      — baca-tulis store produk, tidak ada di `Ctx`.
    //   2. `m.isGroup` / `m.sender` / `m.text` — sebagian ada di `Ctx`, tapi
    //      guard private-chat butuh `m.isGroup` (ada: `ctx.is_group`).
    //   3. `m.quoted.download()` / `m.download()` — unduh media gambar/video,
    //      tersedia via `ctx.media_fetch.download(...)`.
    //   4. `uploadImage(buffer, 'image.jpg')` — unggah ke penyimpanan, butuh
    //      I/O jaringan + filesystem yang belum ada di `Ctx`.
    //
    // Validasi field (nama ≥ 2 char, harga ≥ 1000, diskon > harga jual, stok
    // ≥ 0 atau unlimited, tipe digital/fisik) bisa di-port murni, tapi tanpa
    // akses store hasilnya tidak tersimpan — menyimpang dari JS.
    //
    // `ponytail:` tambahkan akses store (baca `storeProducts`, tulis via
    // `Mutation::SetSetting`) dan unggah media ke `Ctx`, lalu replikasikan
    // seluruh switch field dari JS.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "editproduk".into(),
            is_group: false,
            ..Default::default()
        }
    }

    #[test]
    fn tanpa_argumen_mengembalikan_bantuan() {
        let r = editproduk(ctx_of(""));
        assert_eq!(r.text_body().unwrap(), help("."));
    }

    #[test]
    fn di_grup_masih_bantuan_tanpa_validasi_store() {
        let mut c = ctx_of("1 harga 30000");
        c.is_group = true;
        let r = editproduk(c);
        assert!(r.text_body().unwrap().contains("EDIT PRODUK"));
    }
}
