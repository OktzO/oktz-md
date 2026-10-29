//! `plugins/tts/tts.js` — Google Text To Speech.
//!
//! Catatan: ini adalah kategori baru (`tts`) yang belum ada di repo Rust,
//! jadi `tts/mod.rs` dibuat bersama file ini. Baris `pub mod tts;` perlu
//! ditambahkan manual di `rust/crates/oktz-plugins/src/tts/mod.rs`.
//!
//! `fotoApi.nexray.geminiTts` dari `src/lib/apimanager.js` tidak punya
//! padanan di `Ctx`. Handler hanya mendukung teks input; sintesis suara
//! tidak tersedia di konteks Rust.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

#[plugin(
    name = "tts",
    alias = ["say"],
    category = "tts",
    description = "Google Text To Speech",
    usage = ".tts <text>",
    example = ".tts halo semua",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tts(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();

    if text.is_empty() {
        return Reply::text(format!(
            "🎤 *Google TTS*\n\nGunakan:\n{prefix}tts halo dunia",
            prefix = ctx.prefix
        ));
    }

    // JS: `fotoApi.nexray.geminiTts(text)` → `t.result` (URL audio), lalu
    // `sock.sendMessage({ audio: { url: t.result }, mimetype: "audio/mpeg" })`.
    // Tidak ada fungsi TTS di konteks Rust.
    Reply::text(format!(
        "🎤 *Google TTS*\n\nTeks diterima: *{text}*\nSintesis suara belum tersedia di konteks Rust."
    ))
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "tts".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("tts").unwrap();
        assert_eq!(p.meta.name, "tts");
        assert_eq!(p.meta.alias, ["say"]);
        assert_eq!(p.meta.category, "tts");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn alias_say_menunjuk_ke_tts() {
        assert_eq!(oktz_plugin_api::find("say").unwrap().meta.name, "tts");
    }

    #[test]
    fn tanpa_teks_mengembalikan_bantuan() {
        let r = tts(ctx());
        assert!(text_of(&r).contains("🎤 *Google TTS*"));
        assert!(text_of(&r).contains("Gunakan:"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn dengan_teks_menerima_input() {
        let mut c = ctx();
        c.text = "halo semua".into();
        let r = tts(c);
        assert!(text_of(&r).contains("Teks diterima: *halo semua*"));
    }
}
