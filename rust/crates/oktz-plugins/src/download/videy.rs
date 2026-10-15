//! `plugins/download/videy.js` — unduh video dari videy.co.
//!
//! `ponytail:` lapisan kapabilitas (`src/capabilities/videy.js`) belum di-port:
//! backend lokal mem-probe CDN videy lewat `src/scraper/videy.js`, dan
//! aggregator `neoxr` butuh `config.APIkey.neoxr` yang tidak ada di `Ctx`.
//! `resolve_videy` selalu gagal, jadi yang menyala adalah cabang `catch` biasa.
//!
//! Dua pesan lain di `catch` — "API key belum diset" (`explainFailure` +
//! `missingEnvNameOf`) dan "gagal memnormalisasi" (`adaNormalisasiGagal`) —
//! keduanya membaca `error.tried` dari resolver. Tidak ada `tried` di port ini,
//! jadi keduanya tidak terjangkau; teksnya tetap ditulis sebagai fungsi murni
//! supaya tidak hilang dan supaya bisa dipakai ulang saat kapabilitas menyusul.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` tidak
//! ikut terkirim.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{error_template, host_dari};

/// `videy.js:48-51` — kartu pakai. Baris contoh dibungkus satu pasang backtick,
/// sama seperti di JS.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎬 *ᴠɪᴅᴇʏ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n\
         > Masukkan URL videy.co\n\n\
         `Contoh: {prefix}videy https://videy.co/v?id=7ZH1ZRIF`"
    )
}

/// `videy.js:55` — host bukan Videy.
pub(crate) const BUKAN_HOST: &str = "❌ URL tidak valid. Gunakan link dari videy.co";

/// `videy.js:101` — `adaNormalisasiGagal(error)` benar.
#[allow(dead_code)] // cabang tidak terjangkau tanpa `error.tried`
pub(crate) const NORMALISASI_GAGAL: &str =
    "❌ Gagal mengambil video. Link tidak valid atau sudah expired.";

/// `explainFailure(new MissingApiKeyError("aggregator", envName), …)`
/// (`src/lib/stalker-fallback.js:29-33`). `command` sudah dirangkai jadi
/// `` `${prefix}videy` `` oleh pemanggil.
#[allow(dead_code)] // cabang tidak terjangkau tanpa `error.tried`
pub(crate) fn api_key_belum_diset(command: &str, env_name: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `{env_name}` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

/// `hostVidey` (`videy.js:31-41`) tanpa crate URL: protokol harus http(s),
/// userinfo dan port dibuang, host berhenti di `/?#`, lalu dibandingkan per
/// label. Sub-domain ikut diterima karena halaman videy dilayani dari
/// `www.videy.co`.
pub(crate) fn host_videy(url: &str) -> bool {
    let Some(host) = host_dari(url) else {
        return false;
    };
    host == "videy.co" || host.ends_with(".videy.co")
}

/// `resolver.resolve("videy", { url })` — lapisan kapabilitas belum di-port.
/// `Err` = melempar, persis seperti scraper yang gagal.
fn resolve_videy(_url: &str) -> Result<String, ()> {
    Err(())
}

#[plugin(
    name = "videy",
    alias = ["vdl", "videydownload", "videydl"],
    category = "download",
    description = "Download video dari videy.co",
    usage = ".videy <url>",
    example = ".videy https://videy.co/v?id=7ZH1ZRIF",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn videy(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();

    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !host_videy(url) {
        return Reply::text(BUKAN_HOST);
    }

    match resolve_videy(url) {
        // `videy.js:72-80` — `sendMedia` video; `Reply` tidak punya variant
        // video, jadi URL-nya yang terkirim sebagai teks.
        Ok(url_video) => Reply::text(url_video).with_reaction("✅"),
        // `videy.js:82-105` — tanpa `error.tried`, `missingEnvNameOf` dan
        // `adaNormalisasiGagal` sama-sama `false`, jadi template error yang
        // menyala. `m.pushName` tidak ada di `Ctx`, jadi `User` yang dipakai.
        Err(()) => Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "videy".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("videy").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("videy").unwrap();
        assert_eq!(p.meta.name, "videy");
        assert_eq!(p.meta.alias, ["vdl", "videydownload", "videydl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video dari videy.co");
        assert_eq!(p.meta.usage, ".videy <url>");
        assert_eq!(p.meta.example, ".videy https://videy.co/v?id=7ZH1ZRIF");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["vdl", "videydownload", "videydl"] {
            assert_eq!(find(a).unwrap().meta.name, "videy", "alias {a}");
        }
    }

    #[test]
    fn a_missing_link_shows_the_card_without_reacting() {
        for text in ["", "   ", "\t"] {
            let r = run(text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaksi(&r), None, "input {text:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "videy".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("videy").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), help("!"));
    }

    #[test]
    fn only_videy_hosts_pass_the_guard() {
        for good in [
            "https://videy.co/v?id=7ZH1ZRIF",
            "http://videy.co/v?id=x",
            "https://www.videy.co/v?id=x",
            "https://sub.videy.co/v?id=x",
            "https://VIDEY.CO/v?id=x",
            "  https://videy.co/v?id=x  ",
            "https://user:pw@videy.co:8443/v?x=1#z",
        ] {
            assert!(host_videy(good), "harus lolos: {good}");
        }
        for bad in [
            // Per label, bukan `includes`.
            "https://videy.co.evil.example/x",
            "https://notvidey.co/x",
            "https://xvidey.co/x",
            "https://example.com/?u=videy.co",
            "videy.co/v?id=x",
            "ftp://videy.co/x",
            "javascript:alert(1)",
            "",
        ] {
            assert!(!host_videy(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn a_foreign_host_is_rejected_without_reacting() {
        for text in [
            "https://example.com/x",
            "https://videy.co.evil.example/x",
            "halo dunia",
        ] {
            let r = run(text);
            assert_eq!(text_of(&r), BUKAN_HOST, "input {text}");
            assert_eq!(reaksi(&r), None, "input {text}");
        }
    }

    #[test]
    fn the_resolve_failure_lands_on_the_generic_catch_branch() {
        let r = run("https://videy.co/v?id=7ZH1ZRIF");
        assert_eq!(text_of(&r), error_template(".", "videy"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn the_two_unreachable_catch_messages_are_still_byte_exact() {
        assert_eq!(
            NORMALISASI_GAGAL,
            "❌ Gagal mengambil video. Link tidak valid atau sudah expired."
        );
        assert_eq!(
            api_key_belum_diset(".videy", "APIKEY_NEOXR"),
            "❌ *API Key belum diset!*\n\n\
             > Command `.videy` butuh `APIKEY_NEOXR` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://videy.co/v?id=x").mutations().is_empty());
    }
}