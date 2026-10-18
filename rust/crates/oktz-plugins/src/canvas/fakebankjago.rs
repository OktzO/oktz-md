//! `plugins/canvas/fakebankjago.js` — gambar tampilan saldo Bank Jago palsu.
//!
//! Gambar dibuat di luar bot: plugin cuma meneruskan nama + saldo ke maker API
//! nexray dan meneruskan byte-nya sebagai `MediaSource::Bytes`.
//!
//! `ponytail:` `Number(saldoClean).toLocaleString("id-ID")` memakai pemisah
//! ribuan titik. Saldo sepanjang puluhan digit meleset: `Number` jadi `1e+20`
//! (dan V8 menulis `100.000.000.000.000.000.000`), sedangkan
//! `thousands_id` memformat digit apa adanya.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use crate::tools::kalkulatormbg::thousands_id;
use crate::tools::phisingataubukan::encode_uri_component;

const MAKER_URL: &str = "https://api.nexray.eu.cc/maker/fakebank-jago";

/// `(nama, saldo)` hasil pemecahan input (`fakebankjago.js:20-33`).
///
/// Pemisah `|` didahulukan daripada `,` — sama seperti `if/else if` di JS.
/// Saat tidak ada pemisah, keduanya kosong dan plugin jatuh ke kartu bantuan.
fn split_input(raw: &str) -> (String, String) {
    let raw = raw.trim();
    let sep = if raw.contains('|') {
        '|'
    } else if raw.contains(',') {
        ','
    } else {
        return (String::new(), String::new());
    };
    let parts: Vec<&str> = raw.split(sep).map(str::trim).collect();
    (
        parts.first().copied().unwrap_or_default().to_string(),
        parts.get(1).copied().unwrap_or_default().to_string(),
    )
}

/// `saldo.replace(/[^0-9]/g, "")`.
fn digits_only(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}

/// `fakebankjago.js:38-48`.
fn help(prefix: &str) -> String {
    format!(
        "⚠️ *PEMBUATAN FAKE BANK JAGO*\n\n\
         Fitur ini digunakan untuk membuat gambar tampilan akun Bank Jago kustom dengan nama dan nominal saldo yang ditentukan.\n\n\
         *PENGGUNAAN:*\n\
         - *{prefix}fakebankjago <nama>|<saldo>*\n\
         - *{prefix}fakebankjago <nama>,<saldo>*\n\n\
         *CONTOH:*\n\
         - *{prefix}fakebankjago Furina|4342342*\n\
         - *{prefix}fakebankjago Zann,5000000*\n\n\
         _Gunakan pemisah tanda vertikal | atau koma antara nama dan saldo._"
    )
}

#[plugin(
    name = "fakebankjago",
    alias = ["fakebank-jago", "fakejago", "bankjago"],
    category = "canvas",
    description = "Membuat gambar tampilan rekening atau saldo Fake Bank Jago",
    usage = ".fakebankjago <nama>|<saldo>",
    example = ".fakebankjago Furina|4342342",
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
pub fn fakebankjago(ctx: Ctx) -> Reply {
    let (nama, saldo) = split_input(&ctx.text);
    let clean = digits_only(&saldo);
    if nama.is_empty() || clean.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = format!(
        "{MAKER_URL}?nama={}&saldo={}",
        encode_uri_component(&nama),
        encode_uri_component(&clean)
    );
    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let caption = format!(
        "✅ *BERHASIL*\n\nGambar *Fake Bank Jago* atas nama *{nama}* dengan saldo *Rp {}* berhasil dibuat.",
        thousands_id(clean.parse::<f64>().unwrap_or(0.0))
    );
    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), caption).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn png() -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(vec![0x89, 0x50, 0x4E, 0x47]),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                resp: None,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn respond(&self) -> Result<HttpResponse, HttpError> {
            let Some(bytes) = &self.resp else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(bytes.clone()),
                    mimetype: Some("image/png".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.into());
            self.respond()
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fakebankjago".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("fakebankjago").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    fn emoji(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("fakebankjago").unwrap();
        assert_eq!(p.meta.name, "fakebankjago");
        assert_eq!(p.meta.alias, ["fakebank-jago", "fakejago", "bankjago"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(
            p.meta.description,
            "Membuat gambar tampilan rekening atau saldo Fake Bank Jago"
        );
        assert_eq!(p.meta.usage, ".fakebankjago <nama>|<saldo>");
        assert_eq!(p.meta.example, ".fakebankjago Furina|4342342");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["fakebankjago", "fakebank-jago", "fakejago", "bankjago"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("fakebankjago"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn a_pipe_separated_input_becomes_an_image_with_the_formatted_saldo() {
        let http = Stub::png();
        let r = run(http.clone(), "Furina|4342342");
        let Some(MediaSource::Bytes { data, .. }) = r.media_source() else {
            panic!("gambar maker harus jadi byte, bukan URL");
        };
        assert_eq!(data.len(), 4);
        assert_eq!(
            body(&r),
            "✅ *BERHASIL*\n\nGambar *Fake Bank Jago* atas nama *Furina* dengan saldo *Rp 4.342.342* berhasil dibuat."
        );
        assert_eq!(emoji(&r), "✅");

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(
            seen[0],
            format!("{MAKER_URL}?nama=Furina&saldo=4342342")
        );
    }

    #[test]
    fn a_comma_also_separates_and_the_pipe_wins_when_both_appear() {
        let http = Stub::png();
        run(http.clone(), "Zann,5000000");
        assert!(http.seen.lock().unwrap()[0].ends_with("nama=Zann&saldo=5000000"));

        // `Furina,434|2` -> pemisah `|`, jadi nama = "Furina,434", saldo = "2".
        let http = Stub::png();
        run(http.clone(), "Furina,434|2");
        assert!(
            http.seen.lock().unwrap()[0].ends_with("nama=Furina%2C434&saldo=2"),
            "{}",
            http.seen.lock().unwrap()[0]
        );
    }

    #[test]
    fn non_digit_saldo_is_stripped_before_the_request() {
        let http = Stub::png();
        let r = run(http.clone(), "Furina|Rp 4.342.342");
        assert!(http.seen.lock().unwrap()[0].ends_with("nama=Furina&saldo=4342342"));
        assert!(body(&r).contains("*Rp 4.342.342*"));
    }

    #[test]
    fn the_name_is_percent_encoded() {
        let http = Stub::png();
        run(http.clone(), "Furina Hoshino&test|10");
        assert!(
            http.seen.lock().unwrap()[0].ends_with("nama=Furina%20Hoshino%26test&saldo=10"),
            "{}",
            http.seen.lock().unwrap()[0]
        );
    }

    #[test]
    fn missing_name_or_saldo_shows_the_help_card_without_a_request() {
        let http = Stub::png();
        for text in [
            "",
            "   ",
            "4342342",
            "|4342342",
            "Furina|",
            "Furina|abc",
            "Furina|,,,",
            "Furina",
        ] {
            let r = run(http.clone(), text);
            assert!(!r.has_media(), "{text:?} tidak boleh kirim gambar");
            assert!(
                body(&r).starts_with("⚠️ *PEMBUATAN FAKE BANK JAGO*"),
                "{text:?}"
            );
            assert!(body(&r).ends_with("antara nama dan saldo._"), "{text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_failed_request_reports_the_error_template() {
        let r = run(Stub::offline(), "Furina|10");
        assert_eq!(body(&r), crate::common::error_template(".", "fakebankjago"));
        assert_eq!(emoji(&r), "☢");
    }

    #[test]
    fn an_empty_image_body_is_still_forwarded_as_bytes() {
        // JS tidak mengecek panjang buffer; `Buffer.from([])` tetap terkirim.
        let http = Arc::new(Stub {
            resp: Some(Vec::new()),
            seen: Mutex::new(Vec::new()),
        });
        let r = run(http.clone(), "Furina|10");
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::bytes(Vec::new()))
        );
        
        assert_eq!(r.media_source(), Some(&MediaSource::bytes(Vec::new())));
    }

    #[test]
    fn the_pipe_branch_takes_precedence_and_empty_parts_become_empty_strings() {
        assert_eq!(split_input("A|B"), ("A".into(), "B".into()));
        assert_eq!(split_input(" A | B "), ("A".into(), "B".into()));
        assert_eq!(split_input("|"), ("".into(), "".into()));
        assert_eq!(split_input("A,B,C"), ("A".into(), "B".into()));
        assert_eq!(split_input("bebas"), ("".into(), "".into()));
        assert_eq!(digits_only("1a2b3"), "123");
        assert_eq!(digits_only(""), "");
    }
}