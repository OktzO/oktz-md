//! `plugins/canvas/fakedana.js` — gambar saldo/bukti transaksi DANA palsu dari
//! API `nexray`.
//!
//! `ponytail:` `nominal` dibersihkan jadi digit saja sebelum masuk URL, jadi
//! `encodeURIComponent` di JS tidak pernah mengubah apa pun. Di sini
//! pemanggilan itu dilewati, bukan ditiru.
//!
//! `ponytail:` `te()` memakai `m.pushName`, yang tidak ada di `Ctx` — teks
//! balasan memakai fallback `'User'`, sama seperti `error.js` saat `pushName`
//! kosong.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::tools::kalkulatormbg::thousands_id;

/// `src/lib/error.js` + `config.errorTemplate` (`config.js:153`).
///
/// `pushName || 'User'` jadi `'User'`; `prefix || '.'` dan `command || '?'`
/// tetap dihormati karena keduanya ada di `Ctx`.
pub(crate) fn error_template(prefix: &str, command: &str) -> String {
    let prefix = if prefix.is_empty() { "." } else { prefix };
    let command = if command.is_empty() { "?" } else { command };
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

/// Cabang `if (!nominal)` (`fakedana.js:24-33`).
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *PEMBUATAN FAKE DANA*\n\n\
         Fitur ini digunakan untuk membuat gambar saldo atau bukti transaksi DANA kustom secara otomatis.\n\n\
         *PENGGUNAAN:*\n\
         - *{prefix}fakedana <nominal>*\n\n\
         *CONTOH:*\n\
         - *{prefix}fakedana 500000*\n\
         - *{prefix}fakedana 1000000*\n\n\
         _Pastikan nominal berupa angka tanpa titik atau koma._"
    )
}

/// `(text || m.text || "").trim().replace(/[^0-9]/g, "")` (`fakedana.js:21`).
///
/// `[^0-9]` di JS ASCII, jadi `char::is_ascii_digit` — bukan `is_numeric`,
/// yang menerima angka non-Latin.
pub(crate) fn nominal_of(text: &str) -> String {
    text.trim().chars().filter(char::is_ascii_digit).collect()
}

/// `Number(nominal).toLocaleString("id-ID")` (`fakedana.js:45`).
///
/// `ponytail:` di atas `1e21` V8 beralih ke notasi eksponensial
/// (`1e+30`); `thousands_id` tidak, jadi angka sepanjang itu akan tampil
/// berbeda. Praktis tidak terjadi untuk nominal.
pub(crate) fn nominal_formatted(nominal: &str) -> String {
    thousands_id(nominal.parse::<f64>().unwrap_or(f64::NAN))
}

/// Caption sukses (`fakedana.js:51`).
pub(crate) fn success_caption(nominal: &str) -> String {
    format!(
        "✅ *BERHASIL*\n\nGambar *Fake DANA* dengan nominal *Rp {}* berhasil dibuat.",
        nominal_formatted(nominal)
    )
}

#[plugin(
    name = "fakedana",
    alias = ["danafake", "fakedana"],
    category = "canvas",
    description = "Membuat gambar tampilan saldo atau transaksi Fake DANA",
    usage = ".fakedana <nominal>",
    example = ".fakedana 500000",
    cooldown = 5,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fakedana(ctx: Ctx) -> Reply {
    let nominal = nominal_of(&ctx.text);
    if nominal.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let Ok(res) = ctx.http.get(&format!(
        "https://api.nexray.eu.cc/maker/fakedana?nominal={nominal}"
    )) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    // `await m.react("🕕")` lalu `await m.react("✅")` — dua reaksi pada pesan
    // yang sama. `Reply` hanya menyimpan satu, jadi reaksi terakhir yang
    // dipegang, sama seperti yang dilakukan plugin lain di crate ini.
    Reply::image(
        MediaSource::bytes(res.body.bytes().to_vec()),
        success_caption(&nominal),
    )
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        seen: Mutex<Vec<String>>,
        bytes: Vec<u8>,
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.bytes.clone()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
    }

    fn stub() -> Arc<Stub> {
        Arc::new(Stub {
            seen: Mutex::new(vec![]),
            bytes: vec![0xFF, 0xD8, 0xFF],
        })
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fakedana".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("fakedana").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let meta = oktz_plugin_api::find("fakedana").unwrap().meta.clone();
        assert_eq!(meta.name, "fakedana");
        assert_eq!(meta.alias, ["danafake", "fakedana"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(
            meta.description,
            "Membuat gambar tampilan saldo atau transaksi Fake DANA"
        );
        assert_eq!(meta.usage, ".fakedana <nominal>");
        assert_eq!(meta.example, ".fakedana 500000");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 1);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_to_fakedana() {
        assert_eq!(
            oktz_plugin_api::find("danafake").unwrap().meta.name,
            "fakedana"
        );
        assert_eq!(
            oktz_plugin_api::find("fakedana").unwrap().meta.name,
            "fakedana"
        );
    }

    #[test]
    fn a_normal_nominal_returns_the_image_with_a_dotted_caption() {
        let s = stub();
        let r = run(s.clone(), "500000");
        assert!(r.has_media(), "balasan harus berupa gambar");
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://api.nexray.eu.cc/maker/fakedana?nominal=500000"
        );
        assert_eq!(
            text_of(&r),
            "✅ *BERHASIL*\n\nGambar *Fake DANA* dengan nominal *Rp 500.000* berhasil dibuat."
        );
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("bytes(3)")
        );
    }

    #[test]
    fn thousands_separator_matches_id_id_locale() {
        assert_eq!(nominal_formatted("500000"), "500.000");
        assert_eq!(nominal_formatted("1000000"), "1.000.000");
        assert_eq!(nominal_formatted("999"), "999");
    }

    #[test]
    fn non_digits_are_stripped_before_the_url_is_built() {
        assert_eq!(nominal_of("  Rp 1.250.000  "), "1250000");
        assert_eq!(nominal_of("50rb"), "50");
        // `[^0-9]` ASCII: angka non-Latin dibuang, bukan ikut.
        // Digit Arab-Indic U+0660..U+0669 ditulis sebagai escape supaya file
        // ini tetap tanpa huruf non-Latin.
        assert_eq!(nominal_of("\u{0665}\u{0660}\u{0660}"), "");
        let s = stub();
        let r = run(s.clone(), "12abc34");
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://api.nexray.eu.cc/maker/fakedana?nominal=1234"
        );
        assert_eq!(text_of(&r), success_caption("1234"));
    }

    #[test]
    fn text_without_digits_shows_the_help_and_never_hits_the_network() {
        for input in ["", "   ", "abc", "Rp lima", "-", ".", "💰"] {
            let s = stub();
            let r = run(s.clone(), input);
            assert_eq!(text_of(&r), help("."), "input {input:?}");
            assert!(!r.has_media(), "tidak boleh ada media: {input:?}");
            assert_eq!(
                reaction_of(&r),
                None,
                "cabang ini sebelum `m.react`, jadi tanpa reaksi: {input:?}"
            );
            assert!(s.seen.lock().unwrap().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn a_very_long_digit_run_does_not_panic() {
        let panjang = "9".repeat(400);
        let r = run(stub(), &panjang);
        assert!(r.has_media());
        assert_eq!(text_of(&r), success_caption(&panjang));
    }

    #[test]
    fn http_failure_falls_back_to_the_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fakedana".into(),
            text: "500000".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("fakedana").unwrap().handler)(ctx);
        assert!(!r.has_media());
        assert_eq!(text_of(&r), error_template(".", "fakedana"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn error_template_falls_back_like_the_js_replacements() {
        assert_eq!(
            error_template("", ""),
            "☢ Kayaknya command `.?` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert!(error_template("#", "fakedana").contains("#fakedana"));
    }
}
