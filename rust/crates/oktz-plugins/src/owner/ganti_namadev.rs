//! `plugins/owner/ganti-namadev.js` — ganti nama developer di config.js.
//!
//! `ponytail:` objek config live (`config.bot.developer = newName`) tidak
//! dapat dimutasi di `Ctx` — hanya berkas `config.js` yang ditulis, sama
//! seperti JS yang menimpa berkas lalu memuat ulang.
//! `fs.readFileSync`/`writeFileSync` → `ctx.berkas().baca_teks()` /
//! `tulis_teks()` jalur `"config.js"` (relatif ke akar aset = CWD).
//! Regex tanpa crate `regex` (tidak ada di Cargo.toml): cari `developer:`,
//! lewati spasi, ambil antiko cutip, ganti isinya.
//! `te(...)` → `crate::common::error_template(&ctx.prefix, &ctx.command)`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

/// Jalur berkas konfigurasi (`ganti-namadev.js:28`).
const CONFIG_PATH: &str = "config.js";

/// Kunci yang diganti (`ganti-namadev.js:31`).
const KUNCI: &str = "developer:";

/// Bantuan saat nama kosong (`ganti-namadev.js:23-25`).
pub(crate) fn bantuan(prefix: &str, developer_saat_ini: &str) -> String {
    format!(
        "👨‍💻 *ɢᴀɴᴛɪ ɴᴀᴍᴀ ᴅᴇᴠᴇʟᴏᴘᴇʀ*\n\n\
         > Nama saat ini: *{developer_saat_ini}*\n\n\
         *Penggunaan:*\n\
         `{prefix}ganti-namadev <nama baru>`"
    )
}

/// Sukses (`ganti-namadev.js:40`).
pub(crate) fn sukses(nama_baru: &str) -> String {
    format!(
        "✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Nama developer diganti ke: *{nama_baru}*"
    )
}

/// Ganti nilai di dalam `developer: '...'` / `developer: "..."` (pertama
/// saja, mirip `String.replace` tanpa flag `g` di JS).
fn ganti_developer(content: &str, nama_baru: &str) -> String {
    let prefix = KUNCI;
    let Some(idx) = content.find(prefix) else {
        return content.to_string();
    };
    let setelah_prefix = &content[idx + prefix.len()..];
    let spasi_akhir = setelah_prefix
        .find(|c: char| !c.is_whitespace())
        .unwrap_or(0);
    let isi = &setelah_prefix[spasi_akhir..];
    let quote = match isi.chars().next() {
        Some('\'') | Some('"') => isi.chars().next().unwrap(),
        _ => return content.to_string(),
    };
    let Some(akhir_rel) = isi[1..].find(quote) else {
        return content.to_string();
    };
    let akhir = 1 + akhir_rel;
    let sebelum = &content[..idx + prefix.len() + spasi_akhir];
    let sesudah = &content[idx + prefix.len() + spasi_akhir + 1 + akhir..];
    format!("{sebelum}{quote}{nama_baru}{quote}{sesudah}")
}

#[plugin(
    name = "ganti-namadev",
    alias = ["setnamadev", "setnamedev", "gantideveloper"],
    category = "owner",
    description = "Ganti nama developer di config.js",
    usage = ".ganti-namadev <nama baru>",
    example = ".ganti-namadev Lucky Archz",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_namadev(ctx: Ctx) -> Reply {
    let nama_baru = ctx.args.join(" ");

    if nama_baru.is_empty() {
        let sekarang = ctx
            .config()
            .bot
            .developer
            .as_str()
            .to_string();
        let sekarang = if sekarang.is_empty() {
            "-".to_string()
        } else {
            sekarang
        };
        return Reply::text(bantuan(&ctx.prefix, &sekarang));
    }

    let konten = match ctx.berkas().baca_teks(CONFIG_PATH) {
        Ok(k) => k,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
        }
    };

    let baru = ganti_developer(&konten, &nama_baru);

    if ctx.berkas().tulis_teks(CONFIG_PATH, &baru).is_err() {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
    }

    Reply::text(sukses(&nama_baru))
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::BerkasPalsu;

    const ME: &str = "628111@s.whatsapp.net";

    fn berkas_dengan(konten: &str) -> std::sync::Arc<BerkasPalsu> {
        let b = BerkasPalsu::baru();
        b.tulis_teks(CONFIG_PATH, konten).unwrap();
        std::sync::Arc::new(b)
    }

    fn run(berkas: &std::sync::Arc<BerkasPalsu>, args: &[&str]) -> Reply {
        let text = args.join(" ");
        let ctx = Ctx {
            text,
            args: args.iter().map(|s| s.to_string()).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-namadev".into(),
            berkas: berkas.clone(),
            config: std::sync::Arc::new(
                oktz_plugin_api::config::Runtime::default(),
            ),
            ..Ctx::default()
        };
        ganti_namadev(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn nama_kosong_menghasilkan_bantuan() {
        let b = berkas_dengan("bot: { developer: 'Zann' }");
        let r = run(&b, &[]);
        assert_eq!(text_of(&r), bantuan(".", "Zann"));
    }

    #[test]
    fn nama_kosong_dengan_developer_kosong_pakai_tanda_hubung() {
        let b = berkas_dengan("bot: {}");
        let r = run(&b, &[]);
        assert_eq!(text_of(&r), bantuan(".", "-"));
    }

    #[test]
    fn ganti_nama_single_quote() {
        let b = berkas_dengan("developer: 'Zann'");
        let r = run(&b, &["Lucky", "Archz"]);
        assert_eq!(text_of(&r), sukses("Lucky Archz"));
        assert_eq!(
            b.baca_teks(CONFIG_PATH).unwrap(),
            "developer: 'Lucky Archz'"
        );
    }

    #[test]
    fn ganti_nama_double_quote() {
        let b = berkas_dengan("developer: \"Zann\"");
        let r = run(&b, &["Budi"]);
        assert_eq!(text_of(&r), sukses("Budi"));
        assert_eq!(b.baca_teks(CONFIG_PATH).unwrap(), r#"developer: "Budi""#);
    }

    #[test]
    fn spasi_di_sekitar_colon_dipertahankan() {
        let b = berkas_dengan("developer  :  'Zann'");
        let r = run(&b, &["Budi"]);
        assert_eq!(text_of(&r), sukses("Budi"));
        assert_eq!(b.baca_teks(CONFIG_PATH).unwrap(), "developer  :  'Budi'");
    }

    #[test]
    fn kunci_tidak_ditemukan_berkas_tidak_berubah() {
        let b = berkas_dengan("bot: { name: 'Oktz' }");
        let r = run(&b, &["Budi"]);
        assert_eq!(text_of(&r), sukses("Budi"));
        assert_eq!(b.baca_teks(CONFIG_PATH).unwrap(), "bot: { name: 'Oktz' }");
    }

    #[test]
    fn berkas_tidak_ada_menggunakan_error_template() {
        let b = BerkasPalsu::baru();
        let r = run(&b, &["Budi"]);
        assert_eq!(
            text_of(&r),
            crate::common::error_template(".", "ganti-namadev")
        );
    }

    #[test]
    fn alias_semua_menunjuk_ke_ganti_namadev() {
        for alias in ["setnamadev", "setnamedev", "gantideveloper"] {
            assert_eq!(
                oktz_plugin_api::find(alias).unwrap().meta.name,
                "ganti-namadev"
            );
        }
    }
}
