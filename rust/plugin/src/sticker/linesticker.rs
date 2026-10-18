//! `plugins/sticker/linesticker.js` — unduh sticker pack LINE lewat Neoxr.
//!
//! `ponytail:` `sock.sendImageAsSticker` / `sendVideoAsSticker` dengan
//! `packname`/`author` tidak tersedia di `Reply::Sticker`; pengiriman sticker
//! diakhiri `error_template` + `☢` (pola `swm.rs`). Validasi URL, pembuatan
//! endpoint, dan rendering info di-port agar bisa diuji.
//! Saat pengiriman sticker tersedia, tambahkan `pub mod linesticker;` di
//! `src/sticker/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

const API: &str = "https://api.neoxr.eu/api/linesticker";

/// Kartu pakai (`linesticker.js:24-36`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎨 *ʟɪɴᴇ sᴛɪᴄᴋᴇʀ ᴘᴀᴄᴋ*\n\n\
         > Download LINE sticker pack\n\n\
         ╭┈┈⬡「 📋 *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ* 」\n\
         ┃ {prefix}linesticker <url>\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         *ᴄᴀʀᴀ ᴅᴀᴘᴀᴛ ᴜʀʟ:*\n\
         > 1. Buka https://store.line.me\n\
         > 2. Pilih sticker pack\n\
         > 3. Copy URL dari browser\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > {prefix}linesticker https://store.line.me/stickershop/product/9801/en"
    )
}

/// `/^(https?:\/\/store\.line\.me\/)/` (`linesticker.js:23`).
pub(crate) fn is_url_line(url: &str) -> bool {
    url.starts_with("https://store.line.me/") || url.starts_with("http://store.line.me/")
}

/// `apiUrl` (`linesticker.js:48`).
pub(crate) fn api_url(url: &str, apikey: &str) -> String {
    format!(
        "{API}?url={}&apikey={}",
        crate::tools::phisingataubukan::encode_uri_component(url),
        apikey
    )
}

/// `!res.data?.status || !res.data?.data` (`linesticker.js:51`).
pub(crate) fn respons_valid(data: &serde_json::Value) -> bool {
    crate::common::js_truthy(data.get("status")) && data.get("data").is_some()
}

/// `❌ Gagal mengambil sticker dari URL tersebut!` (`linesticker.js:53`).
pub(crate) const GAGAL_AMBIL: &str = "❌ Gagal mengambil sticker dari URL tersebut!";

/// `❌ API Key Neoxr tidak ditemukan di config!` (`linesticker.js:45`).
pub(crate) const KUNCI_TIDAK_ADA: &str = "❌ API Key Neoxr tidak ditemukan di config!";

/// Info pack (`linesticker.js:70-79`).
#[cfg(test)]
pub(crate) fn render_info(title: &str, author: &str, animated: bool, total: usize) -> String {
    format!(
        "🎨 *ʟɪɴᴇ sᴛɪᴄᴋᴇʀ ᴘᴀᴄᴋ*\n\n\
         ╭┈┈⬡「 📦 *ɪɴꜰᴏ* 」\n\
         ┃ 📝 *Title:* {title}\n\
         ┃ 👤 *Author:* {author}\n\
         ┃ 🎬 *Animated:* {}\n\
         ┃ 📊 *Total:* {total}\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         > 🕕 Mengirim sticker...",
        if animated { "Ya" } else { "Tidak" }
    )
}

/// Sukses kirim (`linesticker.js:108-109`).
#[cfg(test)]
pub(crate) fn berhasil_kirim(terkirim: usize, total: usize) -> String {
    format!("✅ Berhasil kirim {terkirim}/{total} sticker")
}

#[plugin(
    name = "linesticker",
    alias = ["linepack", "line"],
    category = "sticker",
    description = "Download sticker pack LINE",
    usage = ".linesticker <url>",
    example = ".linesticker https://store.line.me/stickershop/product/9801/en",
    cooldown = 25,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn linesticker(ctx: Ctx) -> Reply {
    let url = match ctx.args.first() {
        Some(u) => u.trim(),
        None => return Reply::text(pakai(&ctx.prefix)),
    };

    if url.is_empty() || !is_url_line(url) {
        return Reply::text(pakai(&ctx.prefix));
    }

    let apikey = ctx.config().api_keys.neoxr.clone();
    if apikey.is_empty() {
        return Reply::text(KUNCI_TIDAK_ADA).with_reaction("❌");
    }

    let Ok(res) = ctx.http.get(&api_url(url, &apikey)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢");
    };
    let data = res.json().unwrap_or(serde_json::Value::Null);
    if !respons_valid(&data) {
        return Reply::text(GAGAL_AMBIL).with_reaction("❌");
    }

    let pack = data.get("data").unwrap_or(&serde_json::Value::Null);
    let title = pack.get("title").and_then(|v| v.as_str()).unwrap_or("LINE Sticker");
    let author = pack.get("author").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let animated = pack.get("animated").is_some_and(|v| crate::common::js_truthy(Some(v)));
    let sticker_urls = pack
        .get("sticker_url")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // Pengiriman sticker (sendImageAsSticker/sendVideoAsSticker dengan
    // packname+author) tidak tersedia di `Reply::Sticker` — diakhiri error
    // template seperti swm.rs (info pack dicatat untuk port selanjutnya).
    let _ = (title, author, animated, sticker_urls);
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("linesticker").unwrap();
        assert_eq!(p.meta.name, "linesticker");
        assert_eq!(p.meta.alias, ["linepack", "line"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(p.meta.cooldown, 25);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn pakai_dan_validasi_url() {
        assert!(
            pakai(".").contains(".linesticker https://store.line.me/stickershop/product/9801/en")
        );
        assert!(is_url_line("https://store.line.me/stickershop/product/9801/en"));
        assert!(is_url_line("http://store.line.me/x"));
        assert!(!is_url_line("https://google.com"));
        assert!(!is_url_line("store.line.me/x"));
    }

    #[test]
    fn api_url_mengemudikan_dan_masukkan_kunci() {
        assert_eq!(
            api_url("https://store.line.me/x", "KUNCI"),
            "https://api.neoxr.eu/api/linesticker?url=https%3A%2F%2Fstore.line.me%2Fx&apikey=KUNCI"
        );
    }

    #[test]
    fn respons_valid_miripkan_js() {
        assert!(!respons_valid(&serde_json::json!({})));
        assert!(!respons_valid(&serde_json::json!({ "status": true })));
        assert!(!respons_valid(&serde_json::json!({ "status": false, "data": {} })));
        assert!(respons_valid(&serde_json::json!({ "status": true, "data": {} })));
    }

    #[test]
    fn render_info_bentuk_js() {
        let out = render_info("Pack A", "Author B", true, 25);
        assert!(out.contains("📝 *Title:* Pack A"));
        assert!(out.contains("👤 *Author:* Author B"));
        assert!(out.contains("🎬 *Animated:* Ya"));
        assert!(out.contains("📊 *Total:* 25"));
        assert!(out.contains("> 🕕 Mengirim sticker..."));

        let out = render_info("X", "Y", false, 0);
        assert!(out.contains("🎬 *Animated:* Tidak"));
    }

    #[test]
    fn berhasil_kirim_bentuk_js() {
        assert_eq!(berhasil_kirim(3, 10), "✅ Berhasil kirim 3/10 sticker");
    }

    #[test]
    fn tanpa_url_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "linesticker".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = linesticker(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            pakai(".")
        );
    }

    #[test]
    fn kunci_kosong_reaksi_batsu() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "linesticker".into(),
            args: vec!["https://store.line.me/x".into()],
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = linesticker(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), KUNCI_TIDAK_ADA);
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            )
        );
    }

    #[test]
    fn url_bukan_line_tampilkan_pakai() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "linesticker".into(),
            args: vec!["https://google.com".into()],
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = linesticker(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), pakai("."));
    }
}
