//! `plugins/asupan/asupan.js` — random video asupan.
//!
//! `pub mod asupan;` perlu ditambah manual ke `src/asupan/mod.rs` (mod.rs
//! berisi plugin lain, bukan milik saya — lihat PORT-BRIEF §6).
//!
//! `ponytail:` JS membaca 8 berkas JSON lalu mengambil byte video lewat HTTP.
//! Di Rust berkas dibaca lewat `ctx.berkas()` dan byte video lewat
//! `ctx.http.get()`. Reaksi `🕕` / `❌` dari JS diruntuhkan jadi satu reaksi
//! terakhir karena `Reply::with_reaction` hanya boleh satu emoji.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::random_index;

const DIR: &str = "src/tiktok";

const FILES: [&str; 8] = [
    "bocil.json",
    "gheayubi.json",
    "kayes.json",
    "notnot.json",
    "panrika.json",
    "santuy.json",
    "tiktokgirl.json",
    "ukhty.json",
];

const ERR_DATA_EMPTY: &str = "❌ Data asupan tidak tersedia";
const ERR_AMBIL: &str = "❌ *ᴇʀʀᴏʀ*\n\n> Video asupan tidak ditemukan";

/// `loadJsonData` — gabungkan `url` dari semua berkas JSON — `asupan.js:20-36`.
fn muat_url(berkas: &dyn oktz_plugin_api::berkas::Berkas) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for file in FILES {
        let path = format!("{DIR}/{file}");
        let Ok(teks) = berkas.baca_teks(&path) else {
            continue;
        };
        let Ok(data): Result<Value, _> = serde_json::from_str(&teks) else {
            continue;
        };
        let Some(arr) = data.as_array() else {
            continue;
        };
        for item in arr {
            if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                urls.push(url.to_string());
            }
        }
    }
    urls
}

#[plugin(
    name = "asupan",
    alias = ["asupanrandom"],
    category = "asupan",
    description = "Random video asupan",
    usage = ".asupan",
    example = ".asupan",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn asupan(ctx: Ctx) -> Reply {
    let urls = muat_url(ctx.berkas());
    if urls.is_empty() {
        return Reply::text(ERR_DATA_EMPTY);
    }

    let url = &urls[random_index(urls.len())];

    let Ok(res) = ctx.http.get(url) else {
        return Reply::text(ERR_AMBIL);
    };
    let data = res.body.bytes().to_vec();

    Reply::video(MediaSource::bytes(data), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{berkas::BerkasPalsu, HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn bytes(data: impl Into<Vec<u8>>) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(String::from_utf8(data.into()).unwrap()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.as_bytes().to_vec()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!()
        }
    }

    fn berkas_dengan(files: &[(&str, &str)]) -> BerkasPalsu {
        let b = BerkasPalsu::baru();
        for (path, isi) in files {
            let _ = b.tulis_teks(path, isi);
        }
        b
    }

    fn run(berkas: BerkasPalsu, http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "asupan".into(),
            http,
            berkas: Arc::new(berkas),
            ..Ctx::default()
        };
        asupan(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn tanpa_url_mengembalikan_pesan_data_kosong_tanpa_request() {
        let berkas = berkas_dengan(&[]);
        let http = Arc::new(Stub::offline());
        let r = run(berkas, http);
        assert_eq!(body_of(&r), ERR_DATA_EMPTY);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn satu_url_dikirim_sebagai_video() {
        let berkas = berkas_dengan(&[("src/tiktok/ukhty.json", r#"[{"url":"https://cdn/a.mp4"}]"#)]);
        let http = Arc::new(Stub::bytes("VIDEO_BYTES"));
        let r = run(berkas, http);
        match r {
            Reply::Video { media, .. } => {
                assert_eq!(
                    media,
                    Some(MediaSource::Bytes {
                        data: Arc::from("VIDEO_BYTES".as_bytes()),
                        mimetype: None
                    })
                );
            }
            other => panic!("harus Video, dapat {other:?}"),
        }
        assert_eq!(http.seen.lock().unwrap().as_slice(), &["https://cdn/a.mp4"]);
    }

    #[test]
    fn beberapa_berkas_digabung_dan_ambil_satu() {
        let berkas = berkas_dengan(&[
            ("src/tiktok/bocil.json", r#"[{"url":"https://cdn/b.mp4"}]"#),
            ("src/tiktok/santuy.json", "[]"),
            ("src/tiktok/tiktokgirl.json", r#"[{"url":"https://cdn/c.mp4"},{"url":"https://cdn/d.mp4"}]"#),
        ]);
        let http = Arc::new(Stub::bytes("VID"));
        let _ = run(berkas, http);
        let seen = http.seen.lock().unwrap().clone();
        assert!(seen == vec!["https://cdn/b.mp4"] || seen == vec!["https://cdn/c.mp4"] || seen == vec!["https://cdn/d.mp4"]);
    }

    #[test]
    fn json_berkas_salah_diabaikan() {
        let berkas = berkas_dengan(&[("src/tiktok/bocil.json", "bukan json")]);
        let http = Arc::new(Stub::offline());
        let r = run(berkas, http);
        assert_eq!(body_of(&r), ERR_DATA_EMPTY);
    }
}
