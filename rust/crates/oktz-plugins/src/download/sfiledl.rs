//! `plugins/download/sfiledl.js` — unduh file dari Sfile.mobi.
//!
//! `ponytail:` lapisan kapabilitas (`src/capabilities/sfile.js`) belum di-port:
//! scraper lokal `src/scraper/sfiledl.js` mem-parsing HTML dengan `cheerio`, dan
//! aggregator `neoxr` butuh `config.APIkey.neoxr` yang tidak ada di `Ctx`.
//! `resolve_sfile` selalu gagal, jadi yang menyala adalah cabang `catch` biasa.
//!
//! Dua pesan lain di `catch` — "API key belum diset" (`explainFailure` +
//! `missingEnvNameOf`) dan "gagal memnormalisasi" (`adaNormalisasiGagal`) —
//! keduanya membaca `error.tried` dari resolver. Tidak ada `tried` di port ini,
//! jadi keduanya tidak terjangkau; teksnya tetap ditulis sebagai fungsi murni
//! supaya tidak hilang dan supaya bisa dipakai ulang saat kapabilitas menyusul.
//!
//! `ponytail:` `Reply` tidak punya variant document, jadi hasil `sendMedia`
//! tidak ikut terkirim.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{error_template, host_dari};

/// `sfiledl.js:49-52` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}sfiledl <url_sfile>`\n\n\
         > Contoh: `{prefix}sfiledl https://sfile.mobi/xxxxx`"
    )
}

/// `sfiledl.js:56` — host bukan Sfile.
pub(crate) const BUKAN_HOST: &str = "❌ URL harus dari sfile.mobi atau sfile.co!";

/// `sfiledl.js:105` — `adaNormalisasiGagal(error)` benar.
#[allow(dead_code)] // cabang tidak terjangkau tanpa `error.tried`
pub(crate) const NORMALISASI_GAGAL: &str =
    "❌ Gagal mendapatkan link download. File mungkin tidak tersedia.";

/// `explainFailure(new MissingApiKeyError("aggregator", envName), …)`
/// (`src/lib/stalker-fallback.js:29-33`). `command` sudah dirangkai jadi
/// `` `${prefix}sfiledl` `` oleh pemanggil.
#[allow(dead_code)] // cabang tidak terjangkau tanpa `error.tried`
pub(crate) fn api_key_belum_diset(command: &str, env_name: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `{env_name}` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

/// `hostSfile` (`sfiledl.js:32-42`) tanpa crate URL: protokol harus http(s),
/// userinfo dan port dibuang, host berhenti di `/?#`, lalu dibandingkan per label
/// (`sfile.mobi`, `*.sfile.mobi`, `sfile.co`, `*.sfile.co`).
///
/// Guard per label, bukan `includes`: `sfile.mobi.evil.example` dan
/// `notsfile.mobi` tetap memuat teks "sfile" tapi bukan Sfile.
pub(crate) fn host_sfile(url: &str) -> bool {
    let Some(host) = host_dari(url) else {
        return false;
    };
    host == "sfile.mobi"
        || host.ends_with(".sfile.mobi")
        || host == "sfile.co"
        || host.ends_with(".sfile.co")
}

/// `resolver.resolve("sfile", { url })` — lapisan kapabilitas belum di-port.
/// `Err` = melempar, persis seperti scraper yang gagal.
fn resolve_sfile(_url: &str) -> Result<SfileData, ()> {
    Err(())
}

/// Bentuk `data` yang plugin baca (`src/capabilities/sfile.js:84-96`).
#[allow(dead_code)] // hanya dipakai `resolve_sfile`
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SfileData {
    pub url: String,
    pub filename: String,
    pub mime: String,
}

#[plugin(
    name = "sfiledl",
    alias = ["sfile", "sfiledownload"],
    category = "download",
    description = "Download file dari Sfile.mobi",
    usage = ".sfiledl <url>",
    example = ".sfiledl https://sfile.mobi/xxx",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn sfiledl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();

    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !host_sfile(url) {
        return Reply::text(BUKAN_HOST);
    }

    match resolve_sfile(url) {
        // `sfiledl.js:73-83` — `sendMedia` document; `Reply` tidak punya variant
        // document, jadi URL filenya yang terkirim sebagai teks.
        Ok(data) => Reply::text(data.url).with_reaction("✅"),
        // `sfiledl.js:85-109` — tanpa `error.tried`, `missingEnvNameOf` dan
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
            command: "sfiledl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("sfiledl").unwrap().handler)(ctx)
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
        let p = find("sfiledl").unwrap();
        assert_eq!(p.meta.name, "sfiledl");
        assert_eq!(p.meta.alias, ["sfile", "sfiledownload"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download file dari Sfile.mobi");
        assert_eq!(p.meta.usage, ".sfiledl <url>");
        assert_eq!(p.meta.example, ".sfiledl https://sfile.mobi/xxx");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["sfile", "sfiledownload"] {
            assert_eq!(find(a).unwrap().meta.name, "sfiledl", "alias {a}");
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
            command: "sfiledl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("sfiledl").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), help("!"));
    }

    #[test]
    fn only_sfile_hosts_pass_the_guard() {
        for good in [
            "https://sfile.mobi/xxxxx",
            "http://sfile.mobi/x",
            "https://www.sfile.mobi/x",
            "https://download1.sfile.co/x",
            "https://sfile.co/x",
            "https://SFILE.MOBI/x",
            "  https://sfile.mobi/x  ",
            "https://user:pw@sfile.mobi:8443/x?y=1#z",
        ] {
            assert!(host_sfile(good), "harus lolos: {good}");
        }
        for bad in [
            // Per label, bukan `includes`.
            "https://sfile.mobi.evil.example/x",
            "https://notsfile.mobi/x",
            "https://xsfile.co/x",
            "https://example.com/?u=sfile.mobi",
            "sfile.mobi/x",
            "ftp://sfile.mobi/x",
            "javascript:alert(1)",
            "",
        ] {
            assert!(!host_sfile(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn a_foreign_host_is_rejected_without_reacting() {
        for text in [
            "https://example.com/x",
            "https://sfile.mobi.evil.example/x",
            "halo dunia",
        ] {
            let r = run(text);
            assert_eq!(text_of(&r), BUKAN_HOST, "input {text}");
            assert_eq!(reaksi(&r), None, "input {text}");
        }
    }

    #[test]
    fn the_resolve_failure_lands_on_the_generic_catch_branch() {
        // Tidak ada `error.tried`, jadi `missingEnvNameOf` dan
        // `adaNormalisasiGagal` keduanya false.
        let r = run("https://sfile.mobi/xxxxx");
        assert_eq!(text_of(&r), error_template(".", "sfiledl"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn the_two_unreachable_catch_messages_are_still_byte_exact() {
        assert_eq!(
            NORMALISASI_GAGAL,
            "❌ Gagal mendapatkan link download. File mungkin tidak tersedia."
        );
        assert_eq!(
            api_key_belum_diset(".sfiledl", "APIKEY_NEOXR"),
            "❌ *API Key belum diset!*\n\n\
             > Command `.sfiledl` butuh `APIKEY_NEOXR` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://sfile.mobi/x").mutations().is_empty());
    }
}