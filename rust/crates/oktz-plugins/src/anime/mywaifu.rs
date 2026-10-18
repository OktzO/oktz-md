//! `plugins/anime/mywaifu.js` — koleksi gambar waifu dari Waifu.im.
//!
//! `ponytail:` `crypto.randomBytes(32)` (messageSecret) dan
//! `sock.waUploadToServer`/`relayMessage` diabaikan — `Reply` tidak bawa
//! contextInfo/messageSecret, dan pipeline yang meng-upload media Url.
//! `m.react('🕕')`/`'✅'`/`'❌'` → satu reaksi terakhir.
//! `te(...)` → `crate::common::error_template(&ctx.prefix, &ctx.command)`.

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// `https://api.waifu.im/images` (`mywaifu.js:48`).
const ENDPOINT: &str = "https://api.waifu.im/images";

/// Jumlah gambar per permintaan (`mywaifu.js:39`).
const PAGE_SIZE: u32 = 10;

/// Halaman (`mywaifu.js:44`).
const PAGE: u32 = 1;

/// Urutan acak (`mywaifu.js:43`).
const ORDER_BY: &str = "Random";

/// Bantuan saat mode bukan `sfw`/`nsfw` (`mywaifu.js:25-33`).
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "🌸 *PENCARI WAIFU* 🌸\n\n\
         Fitur ini akan mencarikan 10 gambar waifu spesial untukmu langsung dari Waifu.im dan menyatukannya dalam satu album rapi!\n\n\
         *CARA PENGGUNAAN:*\n\
         - Ketik `{prefix}mywaifu sfw` untuk gambar waifu yang aman.\n\
         - Ketik `{prefix}mywaifu nsfw` untuk gambar waifu versi dewasa (NSFW).\n\n\
         _Pastikan kamu memilih mode yang sesuai dengan seleramu ya!_"
    )
}

/// Gambar tidak ditemukan (`mywaifu.js:56-58`).
pub(crate) fn tidak_ditemukan(mode: &str) -> String {
    format!(
        "❌ *GAMBAR TIDAK DITEMUKAN*\n\nMaaf, sistem tidak dapat menemukan gambar untuk kategori *{}* saat ini.",
        mode.to_uppercase()
    )
}

/// Gagal memanggil API (`mywaifu.js:106-109`).
pub(crate) const GAGAL_API: &str = "❌ *TERJADI KESALAHAN*\n\nMaaf, terjadi gangguan saat sistem mencoba memanggil API.";

/// Caption sukses (`mywaifu.js:62`).
pub(crate) fn caption(mode: &str, jumlah: usize) -> String {
    format!(
        "🌸 *KOLEKSI WAIFU ({})* 🌸\n\nSistem berhasil mendapatkan *{}* gambar waifu spesial untukmu! Cek album di bawah ini untuk melihat koleksi lengkapnya! ✨",
        mode.to_uppercase(),
        jumlah
    )
}

/// Bangun URL query (`mywaifu.js:41-46`).
pub(crate) fn api_url(is_nsfw: bool) -> String {
    format!(
        "{}?isNsfw={}&orderBy={}&page={}&pageSize={}",
        ENDPOINT,
        encode_uri_component(if is_nsfw { "true" } else { "false" }),
        encode_uri_component(ORDER_BY),
        encode_uri_component(PAGE.to_string()),
        encode_uri_component(PAGE_SIZE.to_string()),
    )
}

#[plugin(
    name = "mywaifu",
    alias = ["waifuim", "waifu", "waifus"],
    category = "anime",
    description = "Mencari sekumpulan gambar waifu (SFW / NSFW) menggunakan API Waifu.im.",
    usage = ".mywaifu sfw\nAtau\n.mywaifu nsfw",
    example = ".mywaifu sfw",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn mywaifu(ctx: Ctx) -> Reply {
    let mode = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    if mode != "sfw" && mode != "nsfw" {
        return Reply::text(bantuan(&ctx.prefix));
    }

    let is_nsfw = mode == "nsfw";

    let Ok(res) = ctx.http.call(&HttpCall::get(&api_url(is_nsfw))) else {
        return Reply::text(GAGAL_API).with_reaction("❌");
    };
    let data = res.json().unwrap_or(serde_json::Value::Null);

    let items = data
        .get("items")
        .and_then(serde_json::Value::as_array)
        .map_or(Vec::new(), |a| a.to_vec());

    if items.is_empty() {
        return Reply::text(tidak_ditemukan(&mode)).with_reaction("❌");
    }

    let urls: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("url").and_then(serde_json::Value::as_str))
        .map(|s| s.to_string())
        .collect();

    if urls.is_empty() {
        return Reply::text(tidak_ditemukan(&mode)).with_reaction("❌");
    }

    let teks_caption = caption(&mode, urls.len());
    let mut balasan: Vec<Reply> = vec![Reply::text(teks_caption)];
    for url in urls {
        balasan.push(Reply::image(MediaSource::url(url), ""));
    }

    Reply::sequence(balasan).with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        panggilan: Mutex<Vec<String>>,
        balasan: Option<String>,
        gagal: bool,
    }

    impl Stub {
        fn body(balasan: &str) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(balasan.to_string()),
                ..Default::default()
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                gagal: true,
                ..Default::default()
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            self.panggilan.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default().into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
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

    fn run(http: Arc<Stub>, args: &[&str]) -> Reply {
        let text = args.join(" ");
        let ctx = Ctx {
            text,
            args: args.iter().map(|s| s.to_string()).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "mywaifu".into(),
            http,
            ..Ctx::default()
        };
        mywaifu(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn mode_bukan_sfw_nsfw_hasilkan_bantuan() {
        let http = Stub::offline();
        let r = run(http.clone(), &["xyz"]);
        assert_eq!(text_of(&r), bantuan("."));
        assert_eq!(reaksi(&r), None);
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn tanpa_args_hasilkan_bantuan() {
        let http = Stub::offline();
        let r = run(http.clone(), &[]);
        assert_eq!(text_of(&r), bantuan("."));
    }

    #[test]
    fn api_url_sfw_dan_nsfw() {
        assert_eq!(
            api_url(false),
            "https://api.waifu.im/images?isNsfw=false&orderBy=Random&page=1&pageSize=10"
        );
        assert_eq!(
            api_url(true),
            "https://api.waifu.im/images?isNsfw=true&orderBy=Random&page=1&pageSize=10"
        );
    }

    #[test]
    fn http_gagal_hasilkan_teks_dan_reaksi_sil() {
        let http = Stub::offline();
        let r = run(http.clone(), &["sfw"]);
        assert_eq!(text_of(&r), GAGAL_API);
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn items_kosong_hasilkan_tidak_ditemukan() {
        let http = Stub::body(r#"{"items":[]}"#);
        let r = run(http.clone(), &["sfw"]);
        assert_eq!(text_of(&r), tidak_ditemukan("sfw"));
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn sukses_kirim_sequence_caption_plus_10_gambar() {
        let mut items = Vec::new();
        for i in 0..10 {
            items.push(serde_json::json!({ "url": format!("https://cdn/waifu-{}.jpg", i) }));
        }
        let http = Stub::body(&serde_json::json!({ "items": items }).to_string());
        let r = run(http.clone(), &["nsfw"]);
        assert_eq!(reaksi(&r), Some("✅"));
        let panggilan = http.panggilan.lock().unwrap().clone();
        assert_eq!(panggilan.len(), 1);
        assert_eq!(panggilan[0], api_url(true));
        match r.without_mutations() {
            Reply::Sequence(balasan) => {
                assert_eq!(balasan.len(), 11, "1 caption + 10 gambar");
                assert!(balasan[0].text_body().is_some(), "elemen pertama adalah caption");
                for (i, b) in balasan.iter().enumerate().skip(1) {
                    match b.media_source() {
                        Some(oktz_plugin_api::MediaSource::Url { url, .. }) => {
                            assert_eq!(url, &format!("https://cdn/waifu-{}.jpg", i - 1));
                        }
                        other => panic!("elemen {i} harus image url, dapat {other:?}"),
                    }
                }
            }
            other => panic!("harus Sequence, dapat {other:?}"),
        }
    }

    #[test]
    fn items_tanpa_url_dihitung_seperti_kosong() {
        let http = Stub::body(r#"{"items":[{}]}"#);
        let r = run(http.clone(), &["sfw"]);
        assert_eq!(text_of(&r), tidak_ditemukan("sfw"));
        assert_eq!(reaksi(&r), Some("❌"));
    }
}
