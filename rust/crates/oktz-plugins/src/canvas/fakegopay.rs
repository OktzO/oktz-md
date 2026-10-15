//! `plugins/canvas/fakegopay.js` — gambar saldo GoPay palsu dari API
//! `kyzznekoo`.
//!
//! `ponytail:` axios di JS mengirim dua header (`User-Agent` dan
//! `Content-Type`). `HttpGet` tidak punya setter header per-request, jadi dua
//! header itu tidak ikut. Dicatat supaya tidak hilang saat mekanismenya
//! ditambahkan.
//!
//! `ponytail:` `console.error("[FakeGopay Error]", e)` di cabang `catch` tidak
//! punya padanan — `Reply` tidak membawa log.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::canvas::fakedana::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// Cabang `if (!text)` (`fakegopay.js:21`).
const FORMAT_SALAH: &str = "Format salah!\n\n> Contoh: .fakegopay 100000|500|20000|Januari";

/// Cabang argumen kurang (`fakegopay.js:23`).
const ARGUMEN_KURANG: &str = "Pastikan semua argumen diisi dengan benar, dipisah dengan tanda |.";

/// Caption sukses (`fakegopay.js:35`).
const CAPTION: &str = "✅ Berhasil membuat fake gopay";

/// Empat argumen hasil `text.split("|").map(v => v.trim())` (`fakegopay.js:22`).
///
/// Destructuring tanpa default: field yang tidak ada `undefined` dan lolos
/// pemeriksaan `!saldo`, sama seperti di JS. Batasnya hanya empat — sisa
/// `|` diabaikan persis seperti di JS.
pub(crate) fn split_args(text: &str) -> [String; 4] {
    let mut out = [const { String::new() }; 4];
    for (slot, part) in out.iter_mut().zip(text.split('|')) {
        *slot = part.trim().to_string();
    }
    out
}

/// URL API dengan keempat argumen ter-encode (`fakegopay.js:27`).
pub(crate) fn api_url(args: &[String; 4]) -> String {
    format!(
        "https://kyzznekoo.zone.id/api/canvas/fakegopay?saldo={}&coin={}&terpakai={}&bulan={}",
        encode_uri_component(&args[0]),
        encode_uri_component(&args[1]),
        encode_uri_component(&args[2]),
        encode_uri_component(&args[3]),
    )
}

#[plugin(
    name = "fakegopay",
    alias = ["fakegopay"],
    category = "canvas",
    description = "Membuat canvas fake gopay",
    usage = ".fakegopay <saldo>|<coin>|<terpakai>|<bulan>",
    example = ".fakegopay 100000|500|20000|Januari",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fakegopay(ctx: Ctx) -> Reply {
    if ctx.text.is_empty() {
        return Reply::text(FORMAT_SALAH);
    }

    let args = split_args(&ctx.text);
    if args.iter().any(String::is_empty) {
        return Reply::text(ARGUMEN_KURANG);
    }

    // `await m.react("🕕")` lalu `await m.react("✅")` / `"❌"` — `Reply` hanya
    // menyimpan satu reaksi, jadi yang dipegang adalah reaksi terakhir.
    let Ok(res) = ctx.http.get(&api_url(&args)) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("❌");
    };

    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), CAPTION).with_reaction("✅")
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
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(vec![0x89, 0x50]),
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
        })
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fakegopay".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("fakegopay").unwrap().handler)(ctx)
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
        let meta = oktz_plugin_api::find("fakegopay").unwrap().meta.clone();
        assert_eq!(meta.name, "fakegopay");
        assert_eq!(meta.alias, ["fakegopay"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(meta.description, "Membuat canvas fake gopay");
        assert_eq!(meta.usage, ".fakegopay <saldo>|<coin>|<terpakai>|<bulan>");
        assert_eq!(meta.example, ".fakegopay 100000|500|20000|Januari");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 2);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
    }

    #[test]
    fn four_arguments_produce_the_image() {
        let s = stub();
        let r = run(s.clone(), "100000|500|20000|Januari");
        assert!(r.has_media());
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://kyzznekoo.zone.id/api/canvas/fakegopay?saldo=100000&coin=500&terpakai=20000&bulan=Januari"
        );
        assert_eq!(text_of(&r), CAPTION);
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn empty_text_short_circuits_before_the_guard() {
        let s = stub();
        let r = run(s.clone(), "");
        assert_eq!(text_of(&r), FORMAT_SALAH);
        assert!(!r.has_media());
        assert!(s.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn every_missing_or_blank_argument_is_rejected() {
        for input in [
            "100000",
            "100000|500",
            "100000|500|20000",
            "100000|500|20000|",
            "100000||20000|Januari",
            "|500|20000|",
            " |  |  |  ",
        ] {
            let s = stub();
            let r = run(s.clone(), input);
            assert_eq!(text_of(&r), ARGUMEN_KURANG, "input {input:?}");
            assert!(!r.has_media(), "input {input:?}");
            assert!(
                s.seen.lock().unwrap().is_empty(),
                "input {input:?} tidak boleh sampai ke jaringan"
            );
        }
    }

    #[test]
    fn arguments_are_trimmed_and_percent_encoded() {
        assert_eq!(
            split_args("  100000 | 500 |20000| Januari  "),
            ["100000", "500", "20000", "Januari"]
        );
        let args = [
            "1 & 2".to_string(),
            "a/b".to_string(),
            "50%".to_string(),
            "Januari 2026".to_string(),
        ];
        assert_eq!(
            api_url(&args),
            "https://kyzznekoo.zone.id/api/canvas/fakegopay?saldo=1%20%26%202&coin=a%2Fb&terpakai=50%25&bulan=Januari%202026"
        );
    }

    #[test]
    fn extra_segments_beyond_the_fourth_are_ignored() {
        // Destructuring JS hanya mengambil empat elemen pertama, jadi segmen
        // kelima tidak membuat guard gagal — request tetap terkirim.
        let args = split_args("1|2|3|4|5|6");
        assert_eq!(args, ["1", "2", "3", "4"]);
        let s = stub();
        let r = run(s.clone(), "1|2|3|4|5|6");
        assert!(r.has_media());
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://kyzznekoo.zone.id/api/canvas/fakegopay?saldo=1&coin=2&terpakai=3&bulan=4"
        );
    }

    #[test]
    fn unicode_and_very_long_arguments_do_not_panic() {
        let panjang = "a".repeat(5000);
        let r = run(stub(), &format!("{panjang}|b|c|d"));
        assert!(r.has_media());
        // CJK ditulis sebagai escape supaya file ini tetap tanpa huruf
        // non-Latin; byte UTF-8-nya tetap yang diuji.
        let s = stub();
        let r = run(s.clone(), "\u{65E5}\u{672C}\u{8A9E}|emoji|\u{FFFD}|Desember");
        assert!(r.has_media());
        let seen = s.seen.lock().unwrap()[0].clone();
        assert!(seen.contains("%E6%97%A5%E6%9C%AC%E8%AA%9E"), "{seen}");
    }

    #[test]
    fn http_failure_falls_back_to_the_error_template_with_a_red_cross() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fakegopay".into(),
            text: "1|2|3|4".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("fakegopay").unwrap().handler)(ctx);
        assert!(!r.has_media());
        assert_eq!(text_of(&r), error_template(".", "fakegopay"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }
}
