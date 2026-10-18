//! `plugins/search/playsoundcloud.js` — cari dan download lagu SoundCloud.
//!
//! Pakai `oktz_scraper::soundcloud::search_sound_cloud` dan
//! `oktz_scraper::soundclouddl::scdl` (bukan menyalin logika scraper).
//! Reaksi ganda JS (`🕕` lalu `✅`) disatukan jadi satu reaksi akhir `✅`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use oktz_scraper::{soundclouddl::scdl, soundcloud::search_sound_cloud};

/// Kartu pakai (`playsoundcloud.js:22-28`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎶 *PLAY SOUNDCLOUD* 🎶\n\n\
         Halo kak! Pengen dengerin lagu dari SoundCloud? Aku bisa cariin sekalian downloadin format MP3-nya buat kamu!\n\n\
         *Cara Pakai:*\n\
         👉 `{prefix}playsc <judul lagu>`\n\n\
         *Contoh:*\n\
         `{prefix}playsc Only We Know`"
    )
}

/// `❌ Yah kak, lagunya nggak ketemu! Coba cari dengan judul lain ya. 😭` (`playsoundcloud.js:36`).
pub(crate) const LAGU_TIDAK_KETEMU: &str = "❌ Yah kak, lagunya nggak ketemu! Coba cari dengan judul lain ya. 😭";

/// `contentTxt` dari downloadInfo (`playsoundcloud.js:41-46`).
pub(crate) fn info_lagu(download_info: &serde_json::Value) -> String {
    format!(
        "🎵 *Judul :* {}\n👤 *Uploader :* {}\n⏱️ *Durasi :* {}\n👁️ *Views :* {}\n❤️ *Likes :* {}\n📦 *Ukuran :* {}",
        download_info.get("title").and_then(|v| v.as_str()).unwrap_or("-"),
        download_info.get("uploader").and_then(|v| v.as_str()).unwrap_or("-"),
        download_info.get("duration").and_then(|v| v.as_str()).unwrap_or("-"),
        download_info.get("views").and_then(|v| v.as_str()).unwrap_or("-"),
        download_info.get("likes").and_then(|v| v.as_str()).unwrap_or("-"),
        download_info.get("size").and_then(|v| v.as_str()).unwrap_or("-")
    )
}

/// `txt` keseluruhan (`playsoundcloud.js:48-51`).
pub(crate) fn teks_berhasil(content_txt: &str) -> String {
    format!(
        "🎉 *BERHASIL DOWNLOAD LAGU!* 🎉\n\n{content_txt}\n\n\
         _Audio MP3 sedang dikirim, ditunggu ya kak!_ 🎶"
    )
}

#[plugin(
    name = "playsoundcloud",
    alias = ["playsc"],
    category = "search",
    description = "Cari dan download lagu dari SoundCloud",
    usage = ".playsc judul",
    example = ".playsc Only We Know",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn playsoundcloud(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    let query = ctx.args.join(" ");

    let Ok(search) = search_sound_cloud(&ctx.http, &query) else {
        return Reply::text("❌ Gagal mencari di SoundCloud");
    };

    let results = search
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if results.is_empty() {
        return Reply::text(LAGU_TIDAK_KETEMU);
    }

    let track = &results[0];
    let track_url = track
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let Ok(download_info) = scdl(&ctx.http, track_url) else {
        return Reply::text("❌ Gagal mengambil unduhan SoundCloud");
    };

    let content_txt = info_lagu(&download_info);
    let txt = teks_berhasil(&content_txt);

    let thumbnail = download_info
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .or_else(|| track.get("artwork").and_then(|v| v.as_str()));

    let title = download_info
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let download_url = download_info
        .get("download_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut replies: Vec<Reply> = Vec::new();
    if let Some(thumb) = thumbnail {
        replies.push(Reply::image(MediaSource::url(thumb), txt.clone()));
    }
    replies.push(Reply::audio(MediaSource::url(download_url.as_str())));

    Reply::sequence(replies).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// Stub yang menjawab GET (pencarian) dan POST form (unduhan) berurutan.
    #[derive(Debug, Default)]
    struct Stub {
        respons: Mutex<Vec<String>>,
        index: Mutex<usize>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn baru(respons: &[&str]) -> Arc<Stub> {
            Arc::new(Stub {
                respons: Mutex::new(respons.iter().map(|s| s.to_string()).collect()),
                index: Mutex::new(0),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.ambil(url)
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, url: &str, _: &str) -> Result<HttpResponse, HttpError> {
            self.ambil(url)
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    impl Stub {
        fn ambil(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let (isi, _) = {
                let mut i = self.index.lock().unwrap();
                let respons = self.respons.lock().unwrap();
                let isi = respons[*i].clone();
                *i = (*i + 1).min(respons.len().saturating_sub(1));
                (isi, ())
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(isi.into_bytes()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
    }

    fn run(stub: Arc<Stub>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "playsc".into(),
            text: args.join(" "),
            args: args.iter().map(|s| s.to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            http: stub,
            ..Ctx::default()
        };
        playsoundcloud(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    const SEARCH_OK: &str = r#"{"success":true,"query":"jazz","total":2,"results":[
        {"id":1,"title":"Lagu Satu","url":"https://s/1","artwork":"https://a/1.jpg"},
        {"id":2,"title":"Lagu Dua","url":"https://s/2"}
    ]}"#;

    const SC_DOWNLOAD_OK: &str = r#"{"title":"Lagu Satu","uploader":"Artis","duration":"3:35","views":"1.234","likes":"56","size":"3.50 MB","thumbnail":"https://t/1.jpg","download_url":"https://dl/1.mp3"}"#;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("playsoundcloud").unwrap();
        assert_eq!(p.meta.name, "playsoundcloud");
        assert_eq!(p.meta.alias, ["playsc"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn tanpa_args_tampilkan_pakai() {
        let stub = Stub::baru(&[SEARCH_OK]);
        let r = run(stub, &[]);
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn sukses_kirim_gambar_dan_audio_dengan_reaksi_centang() {
        let stub = Stub::baru(&[SEARCH_OK, SC_DOWNLOAD_OK]);
        let stub_clone = Arc::clone(&stub);
        let r = run(stub, &["Only", "We", "Know"]);
        assert_eq!(reaksi(&r), Some("✅"));
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        match isi {
            Reply::Sequence(replies) => {
                assert_eq!(replies.len(), 2);
                match &replies[0] {
                    Reply::Image { media, caption, .. } => {
                        assert_eq!(
                            media,
                            &Some(MediaSource::Url {
                                url: "https://t/1.jpg".into(),
                                mimetype: None,
                            })
                        );
                        assert!(caption.contains("*Judul :* Lagu Satu"));
                        assert!(caption.contains("_Audio MP3 sedang dikirim, ditunggu ya kak!_"));
                    }
                    other => panic!("harus Image, dapat {other:?}"),
                }
                match &replies[1] {
                    Reply::Audio { media, .. } => {
                        assert_eq!(
                            media,
                            &Some(MediaSource::Url {
                                url: "https://dl/1.mp3".into(),
                                mimetype: None,
                            })
                        );
                    }
                    other => panic!("harus Audio, dapat {other:?}"),
                }
            }
            other => panic!("harus Sequence, dapat {other:?}"),
        }
        let seen = stub_clone.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
        assert!(seen[0].contains("/search"));
        assert!(seen[1].contains("/soundcloud-downloader/soundcloud-downloader.php"));
    }

    #[test]
    fn tanpa_thumbnail_hanya_audio() {
        let search = r#"{"success":true,"results":[{"id":1,"title":"X","url":"https://s/1"}]}"#;
        let download = r#"{"title":"X","uploader":"A","duration":"-","views":"-","likes":"-","size":"-","download_url":"https://dl/1.mp3"}"#;
        let stub = Stub::baru(&[search, download]);
        let r = run(stub, &["X"]);
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        match isi {
            Reply::Sequence(replies) => assert_eq!(replies.len(), 1),
            other => panic!("harus Sequence, dapat {other:?}"),
        }
    }

    #[test]
    fn tidak_ada_hasil_lagu_tidak_ketemu() {
        let stub = Stub::baru(&[r#"{"success":true,"results":[]}"#]);
        let r = run(stub, &["abcdefgh"]);
        assert_eq!(teks(&r), LAGU_TIDAK_KETEMU);
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn info_lagu_bentuk_js() {
        let di = serde_json::json!({
            "title": "Judul Lagu",
            "uploader": "Artis",
            "duration": "3:35",
            "views": "1.234",
            "likes": "56",
            "size": "3.50 MB"
        });
        let out = info_lagu(&di);
        assert!(out.contains("🎵 *Judul :* Judul Lagu"));
        assert!(out.contains("👤 *Uploader :* Artis"));
        assert!(out.contains("📦 *Ukuran :* 3.50 MB"));
    }

    #[test]
    fn teks_berhasil_bentuk_js() {
        let out = teks_berhasil("🎵 *Judul :* X");
        assert!(out.starts_with("🎉 *BERHASIL DOWNLOAD LAGU!* 🎉\n\n🎵 *Judul :* X"));
        assert!(out.contains("_Audio MP3 sedang dikirim, ditunggu ya kak!_ 🎶"));
    }

    #[test]
    fn fallback_artwork_kalau_tidak_ada_thumbnail() {
        // download_info tanpa thumbnail, track punya artwork.
        let search = r#"{"success":true,"results":[{"id":1,"title":"X","url":"https://s/1","artwork":"https://a/art.jpg"}]}"#;
        let download = r#"{"title":"X","uploader":"A","duration":"-","views":"-","likes":"-","size":"-","download_url":"https://dl/1.mp3"}"#;
        let stub = Stub::baru(&[search, download]);
        let r = run(stub, &["X"]);
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        match isi {
            Reply::Sequence(replies) => {
                match &replies[0] {
                    Reply::Image { media, .. } => {
                        assert_eq!(
                            media,
                            &Some(MediaSource::Url {
                                url: "https://a/art.jpg".into(),
                                mimetype: None,
                            })
                        );
                    }
                    other => panic!("harus Image, dapat {other:?}"),
                }
            }
            other => panic!("harus Sequence, dapat {other:?}"),
        }
    }
}
