//! `plugins/tools/carbon.js` — gambar kode ala Carbon.
//!
//! `ponytail:` pengiriman tidak bisa di-port. `buildCarbonUrl` +
//! `THEMES`/`FONTS` ikut hilang bersamanya: satu-satunya pemanggilnya adalah
//! `@microlink/mql`, yaitu layanan screenshot berbasis browser yang tidak
//! ada di `HttpGet` (yang cuma bisa GET/POST biasa). Reaksi 🕕 dan kartu
//! usage — dua hal yang murni teks — tetap dipertahankan.
//!
//! Teks sumber diambil `m.text || m.quoted?.text` (`quoted` menang kalau
//! `m.text` kosong).

use oktz_plugin_api::{Ctx, Reply, plugin};

fn help(prefix: &str) -> String {
    format!(
        "🖥️ *CARBON CODE*\n\n\
         Fitur ini mengubah teks kode program kamu menjadi gambar cantik ala Carbon\n\n\
         *Cara pakai:*\n\
         > `{prefix}carbon <kode>`\n\
         > Atau kamu bisa reply pesan yang berisi kode\n\n\
         *Contoh:*\n\
         > `{prefix}carbon console.log(\"Halo\")`"
    )
}

fn unavailable() -> &'static str {
    "⚠️ *Carbon tidak tersedia di port Rust*\n\n\
     > Butuh layanan screenshot `@microlink/mql` (browser headless) yang tidak\n\
     > ada di `Ctx`, dan `sock.sendMedia` untuk mengirim hasilnya."
}

/// `m.text || m.quoted?.text` — string kosong di JS berarti falsy, jadi quoted
/// baru dibaca kalau `m.text` kosong; quoted kosong pun tetap falsy.
fn source_code(ctx: &Ctx) -> &str {
    if ctx.text.is_empty() {
        ctx.quoted().map(|q| q.text.as_str()).unwrap_or("")
    } else {
        ctx.text.as_str()
    }
}

#[plugin(
    name = "carbon",
    alias = ["carbonify", "carboncode"],
    category = "tools",
    description = "Membuat gambar kode dengan tampilan carbon style",
    usage = ".carbon <kode>",
    example = ".carbon console.log(\"Hello World\")",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn carbon(ctx: Ctx) -> Reply {
    if source_code(&ctx).is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    Reply::text(unavailable()).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, QuotedKind, QuotedMessage, Reply as R};

    fn quoted(text: &str) -> QuotedMessage {
        QuotedMessage {
            id: "q1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn run(text: &str, quoted: Option<QuotedMessage>) -> R {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "carbon".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("carbon").unwrap().handler)(ctx)
    }

    fn body(r: &R) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("carbon").unwrap();
        assert_eq!(p.meta.name, "carbon");
        assert_eq!(p.meta.alias, ["carbonify", "carboncode"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(
            p.meta.description,
            "Membuat gambar kode dengan tampilan carbon style"
        );
        assert_eq!(p.meta.usage, ".carbon <kode>");
        assert_eq!(p.meta.example, ".carbon console.log(\"Hello World\")");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        for a in ["carbonify", "carboncode"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("carbon")
            );
        }
    }

    #[test]
    fn without_code_anywhere_it_shows_the_help_card() {
        assert_eq!(body(&run("", None)), help("."));
        assert_eq!(body(&run("", Some(quoted("")))), help("."), "quoted kosong tetap falsy");
        assert!(body(&run("", None)).contains("> `.carbon <kode>`"));
        assert!(body(&run("", None)).contains("reply pesan yang berisi kode"));
    }

    #[test]
    fn the_command_text_wins_over_a_quoted_body() {
        // `m.text || m.quoted?.text` — teks yang tidak kosong menang.
        assert_eq!(source_code(&run_ctx("halo", Some(quoted("dari quoted")))), "halo");
        assert_eq!(source_code(&run_ctx("", Some(quoted("dari quoted")))), "dari quoted");
    }

    fn run_ctx(text: &str, quoted: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "carbon".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            quoted,
            ..Default::default()
        }
    }

    #[test]
    fn code_present_reacts_then_reports_the_missing_service() {
        let r = run("console.log(1)", None);
        match r.without_mutations() {
            R::Reaction { emoji, .. } => assert_eq!(emoji, "🕕"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(body(&r).contains("@microlink/mql"), "{}", body(&r));
        assert!(!r.has_media(), "tidak ada gambar yang benar-benar terkirim");
    }
}
