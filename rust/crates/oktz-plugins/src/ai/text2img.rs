//! `plugins/ai/text2img.js` — buat gambar dari teks lewat Firefly.
//!
//! `ponytail:` jalur HTTP setelah guard API key TIDAK ditulis di sini.
//! `config.APIkey.firefly` (`config.js:332`) = `process.env.APIKEY_FIREFLY || ""`,
//! dan `Ctx` tidak membawa config, jadi kuncinya kosong — persis seperti
//! `info::jadwalbola` untuk `APIKEY_NEOXR`. Tanpa key itu `explainFailure`
//! selalu mengembalikan kartu "API Key belum diset", jadi `try` di
//! `text2img.js:35-50` tidak pernah terjangkau. Menuliskannya berarti menambah
//! kode mati yang diam-diam menyimpang dari perilaku yang benar-benar terjadi.
//! Tambahkan jalur itu di bawah guard begitu router menyediakannya lewat `Ctx`.
//!
//! Catatan klasifikasi: `firefly.maiku.my.id` sudah mati (Cloudflare HTTP 530,
//! Argo Tunnel origin down) — lihat `docs/rust-rewrite/PLUGIN-CLASSIFICATION.md`
//! kelas E.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `explainFailure(new MissingApiKeyError('firefly', 'APIKEY_FIREFLY'), cmd)`
/// (`src/lib/stalker-fallback.js:29-34`).
fn missing_key(command: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `APIKEY_FIREFLY` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

#[plugin(
    name = "text2img",
    alias = [],
    category = "ai",
    description = "Buat gambar dari teks",
    usage = ".text2img <teks>",
    example = ".text2img Buat gambar dari teks",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn text2img(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(format!(
            "📿 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: {}text2img Buat gambar dari teks`",
            ctx.prefix
        ));
    }

    // `text2img.js:29-33` — reaksi `🕕` terjadi sebelum cek key, jadi ikut.
    Reply::text(missing_key(&format!("{}text2img", ctx.prefix))).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "text2img".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("text2img").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("text2img").unwrap();
        assert_eq!(p.meta.name, "text2img");
        assert!(p.meta.alias.is_empty(), "alias di JS kosong");
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Buat gambar dari teks");
        assert_eq!(p.meta.usage, ".text2img <teks>");
        assert_eq!(p.meta.example, ".text2img Buat gambar dari teks");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn no_alias_means_the_command_itself_is_the_only_key() {
        assert_eq!(
            oktz_plugin_api::find("text2img").map(|p| p.meta.name.as_str()),
            Some("text2img")
        );
        assert_eq!(
            oktz_plugin_api::find("t2i").map(|p| p.meta.name.as_str()),
            None
        );
    }

    #[test]
    fn an_empty_prompt_shows_the_usage_card_without_a_reaction() {
        // `text2img.js:22-27` return sebelum `m.react('🕕')` di `:29`.
        for args in ["", "   "] {
            let r = run(args.clone());
            assert!(body(&r).contains("📿 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*"), "args={args:?}");
            assert!(
                body(&r).contains("*.text2img Buat gambar dari teks*"),
                "args={args:?}"
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_prompt_without_an_api_key_answers_with_the_key_card() {
        let r = run("kucing");
        assert_eq!(
            body(&r),
            "❌ *API Key belum diset!*\n\n\
             > Command `.text2img` butuh `APIKEY_FIREFLY` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "🕕"
        ));
    }

    #[test]
    fn the_key_card_names_the_prefix_that_was_used() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "text2img".into(),
            text: "kucing".into(),
            args: vec!["kucing".into()],
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("text2img").unwrap().handler)(ctx);
        assert!(body(&r).contains("Command `!text2img`"), "{}", body(&r));
    }

    #[test]
    fn a_very_long_prompt_still_reaches_the_key_guard() {
        let r = run(&"a".repeat(20_000));
        assert!(
            body(&r).contains("API Key belum diset"),
            "{}",
            &body(&r)[..80]
        );
    }
}
