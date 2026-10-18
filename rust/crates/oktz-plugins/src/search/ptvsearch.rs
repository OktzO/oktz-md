//! `plugins/search/ptvsearch.js` — cari video TikTok via scraper `tiktoksearch`.
//!
//! `ponytail:` JS mengirim `video: { url }` dengan `mimetype: video/mp4` dan
//! `ptv: true`, tanpa caption. `Reply::video` tidak punya bidang `ptv`, jadi
//! dikirim sebagai video biasa. Satu reaksi akhir (`✅`/`❌`/`☢`) dipasang.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::{error_template, random_index};
use oktz_plugin_api::HttpError;

fn help(prefix: &str) -> String {
    format!(
        "╭┈┈⬡「 🎵 *ᴛɪᴋᴛᴏᴋ sᴇᴀʀᴄʜ* 」\n┃\n┃ ㊗ ᴜsᴀɢᴇ: `{prefix}ptvsearch <query>`\n┃\n╰┈┈⬡\n\n\
         > `Contoh: {prefix}ptvsearch anime`"
    )
}

fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Tidak ditemukan video untuk: {query}")
}

#[plugin(
    name = "ptvsearch",
    alias = ["ptvs"],
    category = "search",
    description = "Cari video TikTok",
    usage = ".ptvsearch <query>",
    example = ".ptvsearch jj epep",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ptvsearch(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim().to_string();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let videos = match oktz_scraper::tiktoksearch::tiktok_search_video(&*ctx.http, &query) {
        Ok(v) => v,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
        }
    };

    let arr = match videos.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => {
            return Reply::text(tidak_ditemukan(&query)).with_reaction("❌");
        }
    };

    let item = arr[random_index(arr.len())];
    let link = item
        .get("link")
        .and_then(Value::as_str)
        .unwrap_or("");

    if link.is_empty() {
        return Reply::text(tidak_ditemukan(&query)).with_reaction("❌");
    }

    Reply::video(MediaSource::url(link.to_string()), "").with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ptvsearch".into(),
            http,
            ..Ctx::default()
        };
        ptvsearch(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_query_menghasilkan_kartu_pakai() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn hasil_kosong_menghasilkan_tidak_ditemukan() {
        let r = run(Stub::body(serde_json::json!({ "status": true, "result": [] })), "anime");
        assert_eq!(text_of(&r), tidak_ditemukan("anime"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn item_tanpa_link_dibuang_dan_hasilkan_tidak_ditemukan() {
        let r = run(
            Stub::body(serde_json::json!({ "status": true, "result": [{ "title": "Tanpa link" }] })),
            "anime",
        );
        assert_eq!(text_of(&r), tidak_ditemukan("anime"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn status_falsy_melempar_dan_menghasilkan_error_template() {
        let r = run(Stub::body(serde_json::json!({ "status": false, "message": "batas tercapai" })), "anime");
        assert_eq!(text_of(&r), crate::common::error_template(".", "ptvsearch"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn jalur_berhasil_mengirim_video_dengan_link_random() {
        // Dua item dengan link; urutan stabil di test karena random_index
        // dipanggil sekali. Kita cakup: link yang keluar harus salah satu dari
        // dua yang diketahui.
        let body = serde_json::json!({
            "status": true,
            "result": [
                { "title": "A", "link": "https://tiktok.com/@a/video/1" },
                { "title": "B", "link": "https://tiktok.com/@b/video/2" },
            ]
        });
        let http = Stub::body(body);
        let r = run(http.clone(), "anime");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Video { media, .. } => {
                let url = match media {
                    Some(oktz_plugin_api::MediaSource::Url { url, .. }) => url.as_str(),
                    _ => "",
                };
                assert!(
                    url == "https://tiktok.com/@a/video/1" || url == "https://tiktok.com/@b/video/2",
                    "link tidak dikenal: {url}"
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
        let seen = http.seen.lock().unwrap();
        assert_eq!(
            seen.as_slice(),
            &["https://api.nexray.eu.cc/search/tiktok?q=anime"]
        );
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run(Stub::offline(), "anime");
        assert_eq!(text_of(&r), crate::common::error_template(".", "ptvsearch"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
