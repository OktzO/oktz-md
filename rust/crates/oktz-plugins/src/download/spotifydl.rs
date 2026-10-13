//! `plugins/download/spotifydl.js` — unduh lagu dari Spotify.
//!
//! `ponytail:` resolver berlapis belum di-port; yang dipakai di sini backend
//! agregator yang sudah jadi cadangan kedua di JS (`nexray` →
//! `/downloader/spotify`, `src/capabilities/spotify.js:60`).
//!
//! `ponytail:` `Reply` tidak punya variant audio, jadi file audio tidak
//! terkirim — hanya teks. Tambahkan `Reply::Audio` di `oktz-plugin-api`
//! kalau pipeline sudah bisa mengunggah audio.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use super::douyindl::query_encode;

/// `nexray` di `src/lib/aggregator.js:22`.
pub(crate) const ENDPOINT: &str = "https://api.nexray.eu.cc/downloader/spotify";

/// `spotifydl.js:18` — `/open\.spotify\.com\/track/i`.
pub(crate) fn is_track_url(text: &str) -> bool {
    text.to_lowercase().contains("open.spotify.com/track")
}

/// `spotifydl.js:19` — link hilang atau bukan track.
pub(crate) const BAD_URL: &str = "❌ *Waduh, link Spotify-nya mana nih atau kurang tepat!*\n\n\
     Kamu harus memasukkan tautan (link) lagu dari Spotify yang valid. \
     Pastikan itu adalah link ke track/lagu ya! \n\n\
     Contoh: `.spdl https://open.spotify.com/track/3RY0NyQQXxuAiyk5eAS4fC`";

/// `spotifydl.js:50` — backend menjawab tapi tidak ada URL unduhan.
pub(crate) const NO_DOWNLOAD: &str =
    "⚠️ *Gagal mengambil lagu!* \n\nServer tidak merespon dengan tautan unduhan yang valid.";

/// `spotifydl.js:53` — kegagalan proses.
pub(crate) const SYS_ERROR: &str = "😔 *Terjadi kesalahan sistem saat memproses tautan Spotify tersebut.* \
     Mohon coba lagi nanti ya!";

/// Hasil `trackUnduh` (`src/capabilities/spotify.js:131`). `Err` = respons
/// tanpa URL unduhan yang bisa dipakai — kelas yang di plugin dipisahkan dari
/// kegagalan proses lewat `adaNormalisasiGagal`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Track {
    pub title: String,
    pub artist: String,
    pub url: String,
}

pub(crate) fn normalize(body: &Value) -> Result<Track, ()> {
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(());
    }
    let isi = body.get("result").filter(|r| r.is_object()).ok_or(())?;
    let raw = isi
        .get("url")
        .or_else(|| isi.get("download_url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !raw.starts_with("http://") && !raw.starts_with("https://") {
        return Err(());
    }
    let text = |k: &str| {
        isi.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    Ok(Track {
        title: text("title"),
        artist: text("artist"),
        url: raw,
    })
}

/// `spotifydl.js:31` — `${artist || "Spotify"} - ${title || "Audio"}.mp3`.
pub(crate) fn filename(track: &Track) -> String {
    let artist = if track.artist.is_empty() {
        "Spotify"
    } else {
        &track.artist
    };
    let title = if track.title.is_empty() {
        "Audio"
    } else {
        &track.title
    };
    format!("{artist} - {title}.mp3")
}

#[plugin(
    name = "spotifydl",
    alias = ["spdl", "spotify-dl", "spotdl"],
    category = "download",
    description = "Unduh lagu favoritmu langsung dari Spotify tanpa ribet!",
    usage = ".spdl <link>",
    example = ".spdl https://open.spotify.com/track/...",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn spotifydl(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() || !is_track_url(text) {
        return Reply::text(BAD_URL);
    }

    let url = format!("{ENDPOINT}?url={}", query_encode(text));

    // Klasifikasi dari `tried` di JS dipisah di sini oleh LAPIS: request yang
    // gagal total (transport/bukan JSON) adalah kegagalan proses, sedangkan
    // respons yang sampai tapi tidak punya URL unduhan adalah "normalisasi
    // gagal" — dua pesan yang berbeda di `spotifydl.js:48-53`.
    let body = ctx.http.get(&url).ok().and_then(|r| r.json());
    match body.as_ref().and_then(|b| normalize(b).ok()) {
        Some(track) => Reply::text(filename(&track)),
        None => {
            if body.is_some() {
                Reply::text(NO_DOWNLOAD).with_reaction("❌")
            } else {
                Reply::text(SYS_ERROR).with_reaction("❌")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .body
                .clone()
                .map_err(|()| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "spdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("spotifydl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn ok_body() -> Value {
        json!({
            "status": true,
            "result": {
                "title": "Indonesia Raya",
                "artist": "Nasional",
                "url": "https://cdn.example/a.mp3",
                "mime": "audio/mpeg"
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("spotifydl").unwrap();
        assert_eq!(p.meta.name, "spotifydl");
        assert_eq!(p.meta.alias, ["spdl", "spotify-dl", "spotdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(
            p.meta.description,
            "Unduh lagu favoritmu langsung dari Spotify tanpa ribet!"
        );
        assert_eq!(p.meta.usage, ".spdl <link>");
        assert_eq!(p.meta.example, ".spdl https://open.spotify.com/track/...");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["spdl", "spotify-dl", "spotdl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "spotifydl");
        }
    }

    #[test]
    fn only_track_urls_are_accepted() {
        assert!(is_track_url("https://open.spotify.com/track/abc"));
        assert!(is_track_url("HTTPS://OPEN.SPOTIFY.COM/TRACK/abc"));
        for bad in [
            "https://open.spotify.com/album/abc",
            "https://open.spotify.com/playlist/abc",
            "https://spotify.com/track/abc",
            "",
        ] {
            assert!(!is_track_url(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn a_missing_or_non_track_link_is_refused_without_a_request() {
        let http = Stub::body(ok_body());
        for text in ["", "   ", "https://open.spotify.com/album/x"] {
            assert_eq!(text_of(&run(http.clone(), text)), BAD_URL, "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_resolved_track_replies_with_the_mp3_filename() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://open.spotify.com/track/3RY0NyQQXxuAiyk5eAS4fC");
        assert_eq!(text_of(&r), "Nasional - Indonesia Raya.mp3");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(
            seen[0].starts_with("https://api.nexray.eu.cc/downloader/spotify?url=https%3A%2F%2F"),
            "{}",
            seen[0]
        );
    }

    #[test]
    fn missing_artist_and_title_fall_back_to_spotify_and_audio() {
        let body = json!({ "status": true, "result": { "url": "https://cdn.example/a.mp3" } });
        let r = run(Stub::body(body), "https://open.spotify.com/track/x");
        assert_eq!(text_of(&r), "Spotify - Audio.mp3");
    }

    #[test]
    fn a_response_without_a_download_url_gets_the_amber_warning() {
        // `adaNormalisasiGagal` (`spotifydl.js:48`): backend menjawab tapi
        // tidak ada yang bisa dikirim — track tidak ada atau terkunci region.
        for body in [
            json!({ "status": false, "msg": "not found" }),
            json!({ "status": true, "result": { "title": "x", "url": "ftp://x" } }),
            json!({ "status": true, "result": {} }),
            json!({ "status": true }),
            json!({}),
        ] {
            let r = run(Stub::body(body.clone()), "https://open.spotify.com/track/x");
            assert_eq!(text_of(&r), NO_DOWNLOAD, "body {body}");
            assert!(
                matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
                "harus bereaksi ❌"
            );
        }
    }

    #[test]
    fn a_transport_failure_reports_the_system_error_instead() {
        // Kegagalan proses harus dilaporkan sebagai kegagalan proses, bukan
        // disamarkan jadi "tidak ada tautan".
        let r = run(Stub::offline(), "https://open.spotify.com/track/x");
        assert_eq!(text_of(&r), SYS_ERROR);
    }

    #[test]
    fn download_url_from_the_other_field_name_is_also_accepted() {
        let body = json!({
            "status": true,
            "result": { "download_url": "https://cdn.example/b.mp3", "title": "T" }
        });
        let r = run(Stub::body(body), "https://open.spotify.com/track/x");
        assert_eq!(text_of(&r), "Spotify - T.mp3");
    }
}