//! `plugins/search/lyrics.js` (`lirik`) — cari lirik lewat
//! `api.nexray.eu.cc`.
//!
//! `ponytail:` `fetchLyrics` menelan semua error axios dan mengembalikan `null`,
//! jadi kegagalan jaringan berakhir di cabang "tidak ditemukan", bukan di
//! `catch` handler. Cabang `catch` handler karena itu tidak bisa dicapai dari
//! `Ctx` mana pun — sama seperti di JS — dan tetap ditulis supaya tidak hilang
//! tanpa jejak.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;
use crate::tools::phisingataubukan::encode_uri_component;

/// `lyrics.js:5` — endpoint lirik.
pub(crate) fn api_url(judul: &str) -> String {
    format!(
        "https://api.nexray.eu.cc/search/lyrics?q={}",
        encode_uri_component(judul)
    )
}

/// `lyrics.js:36-39` — kartu saat judul kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "Hai kak! ✨ Lupa masukin judul lagunya ya? 😅\n\n\
         Coba deh ketik perintahnya begini: *{prefix}lirik sempurna andra and the backbone* 🎶\n\n\
         Yuk, masukin judulnya biar kita bisa nyanyi bareng! 🎤🔥"
    )
}

/// `lyrics.js:49` — lirik tidak ketemu.
pub(crate) fn not_found(query: &str) -> String {
    format!("Waduh, maaf banget kak 🥺 lirik lagu *{query}* nggak ketemu nih di database. Coba pakai kata kunci atau judul yang lebih spesifik ya! 💔")
}

/// `lyrics.js:76` — pesan server tidak merespons.
pub(crate) const SERVER_DOWN: &str = "Yah, server liriknya lagi ngambek nih kak 😭 Coba lagi nanti ya! 🛠️✨";

/// Ambil nilai di jalur titik — padanan `a?.b?.c`.
fn dig<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = v;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
///
/// Sama dengan `search/apkmod.rs` dan `search/android1.rs`; kandidat
/// `common.rs`.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `!data || !data.lyrics || !data.lyrics.plain_lyrics` + teks jawaban
/// (`lyrics.js:47-61`). `None` berarti cabang "tidak ditemukan".
pub(crate) fn lyrics_text(data: &Value, query: &str) -> Option<String> {
    if !js_truthy(dig(data, "lyrics")) {
        return None;
    }
    let plain = dig(data, "lyrics.plain_lyrics")?;
    if !js_truthy(Some(plain)) {
        return None;
    }

    let title = match data.get("title") {
        Some(v) if js_truthy(Some(v)) => interp(Some(v)),
        _ => query.to_string(),
    };
    let artist = match data
        .get("artist")
        .filter(|v| js_truthy(Some(*v)))
        .or_else(|| dig(data, "lyrics.artist_name").filter(|v| js_truthy(Some(*v))))
    {
        Some(v) => interp(Some(v)),
        None => "Tidak diketahui".to_string(),
    };

    Some(format!(
        "Ketemu nih liriknya! 🎉\n\n\
         🎵 *Judul:* {title}\n\
         🎤 *Artis:* {artist}\n\n\
         Ini dia lirik lengkapnya buat kamu:\n\n\
         {}\n\n\
         Selamat bernyanyi ria, kak! 🎧💖",
        interp(Some(plain))
    ))
}

/// `data.thumbnail && data.thumbnail !== '-'` (`lyrics.js:63`).
pub(crate) fn thumbnail_of(data: &Value) -> Option<String> {
    let thumb = data.get("thumbnail").and_then(Value::as_str)?;
    if thumb.is_empty() || thumb == "-" {
        return None;
    }
    Some(thumb.to_string())
}

#[plugin(
    name = "lirik",
    alias = ["lyric", "lyrics", "liriklagu"],
    category = "search",
    description = "Cari lirik lagu",
    usage = ".lirik <query>",
    example = ".lirik sempurna",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn lirik(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `fetchLyrics` mengembalikan `null` untuk status falsy, `result` kosong,
    // dan error apa pun — jadi satu cabang "tidak ditemukan" untuk semuanya.
    let Ok(resp) = ctx.http.get(&api_url(query)) else {
        return Reply::text(not_found(query)).with_reaction("❌");
    };
    let root = resp.json().unwrap_or(Value::Null);
    // `fetchLyrics` (`lyrics.js:6-9`): hanya `result` yang dipakai, dan hanya
    // kalau `status` truthy. Tanpa `status`, hasilnya `null` seperti gagal.
    let data = match root.get("result") {
        Some(v) if js_truthy(root.get("status")) && js_truthy(Some(v)) => v.clone(),
        _ => return Reply::text(not_found(query)).with_reaction("❌"),
    };

    let Some(text) = lyrics_text(&data, query) else {
        return Reply::text(not_found(query)).with_reaction("❌");
    };

    match thumbnail_of(&data) {
        Some(url) => Reply::image(MediaSource::url(url), text).with_reaction("✅"),
        None => Reply::text(text).with_reaction("✅"),
    }
}

/// Cabang `catch` handler (`lyrics.js:74-77`).
///
/// `ponytail:` tidak bisa dicapai: satu-satunya pemanggilan jaringan dibungkus
/// `try {} catch { return null }` di `fetchLyrics`, dan setelah itu tidak ada
/// lagi yang bisa melempar. Ditulis supaya teks aslinya tidak hilang.
#[allow(dead_code)]
pub(crate) fn server_down() -> Reply {
    Reply::text(SERVER_DOWN).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = self.body.clone().ok_or_else(|| oktz_plugin_api::HttpError {
                message: "offline".into(),
            })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
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
            command: "lirik".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("lirik").unwrap().handler)(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    const ADA_LIRIK: &str = r#"{
        "status": true,
        "result": {
            "title": "Sempurna",
            "artist": "Andra and The Backbone",
            "thumbnail": "https://cdn/cover.jpg",
            "lyrics": {"artist_name": "Fallback Artist", "plain_lyrics": "Mungkin kau ingin\nTersenyum"}
        }
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("lirik").unwrap();
        assert_eq!(p.meta.name, "lirik");
        assert_eq!(p.meta.alias, ["lyric", "lyrics", "liriklagu"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari lirik lagu");
        assert_eq!(p.meta.usage, ".lirik <query>");
        assert_eq!(p.meta.example, ".lirik sempurna");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["lyric", "lyrics", "liriklagu"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "lirik", "alias {a}");
        }
    }

    #[test]
    fn a_blank_title_shows_the_friendly_hint_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_found_song_becomes_an_image_with_the_full_caption() {
        let http = Stub::new(ADA_LIRIK);
        let r = run(http.clone(), "sempurna andra");
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url("https://cdn/cover.jpg"))
        );
        let body = body_of(&r);
        assert!(body.starts_with("Ketemu nih liriknya! 🎉\n\n🎵 *Judul:* Sempurna\n"), "{body}");
        assert!(
            body.contains("🎤 *Artis:* Andra and The Backbone\n\n"),
            "{body}"
        );
        assert!(
            body.contains("Ini dia lirik lengkapnya buat kamu:\n\nMungkin kau ingin\nTersenyum\n\n"),
            "{body}"
        );
        assert!(body.ends_with("Selamat bernyanyi ria, kak! 🎧💖"), "{body}");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.nexray.eu.cc/search/lyrics?q=sempurna%20andra"
        );
    }

    #[test]
    fn the_artist_falls_back_to_the_lyrics_object_then_to_a_placeholder() {
        let data = serde_json::json!({"lyrics": {"artist_name": "Dari Lyrics", "plain_lyrics": "x"}});
        assert!(lyrics_text(&data, "q")
            .unwrap()
            .contains("🎤 *Artis:* Dari Lyrics\n"));

        let data = serde_json::json!({"title": "", "lyrics": {"plain_lyrics": "x"}});
        let text = lyrics_text(&data, "judulku").unwrap();
        assert!(
            text.contains("🎵 *Judul:* judulku\n"),
            "`title` kosong pakai query: {text}"
        );
        assert!(text.contains("🎤 *Artis:* Tidak diketahui\n"), "{text}");
    }

    #[test]
    fn a_missing_lyric_block_reports_not_found() {
        for body in [
            r#"{"status": true, "result": {}}"#,
            r#"{"status": true, "result": {"lyrics": {}}}"#,
            r#"{"status": true, "result": {"lyrics": {"plain_lyrics": ""}}}"#,
            r#"{"status": true, "result": {"lyrics": {"plain_lyrics": null}}}"#,
            r#"{"status": true, "result": {"lyrics": "bukan objek"}}"#,
            r#"{"status": false, "result": {"lyrics": {"plain_lyrics": "x"}}}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Stub::new(body), "laguku");
            assert_eq!(body_of(&r), not_found("laguku"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn a_transport_failure_reads_as_not_found_like_the_js_catch() {
        // `fetchLyrics` mengembalikan `null` untuk error axios.
        let r = run(Stub::empty(), "laguku");
        assert_eq!(body_of(&r), not_found("laguku"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn a_placeholder_thumbnail_means_plain_text() {
        let hasil = serde_json::json!({"lyrics": {"plain_lyrics": "x"}, "thumbnail": "-"});
        assert_eq!(thumbnail_of(&hasil), None);
        let root = serde_json::json!({"status": true, "result": hasil});
        let r = run(Stub::new(&root.to_string()), "laguku");
        assert!(!r.has_media());
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(body_of(&r).starts_with("Ketemu nih liriknya! 🎉"), "{}", body_of(&r));
    }

    #[test]
    fn the_unreachable_catch_reply_keeps_its_text() {
        assert_eq!(body_of(&server_down()), SERVER_DOWN);
        assert_eq!(reaction_of(&server_down()), Some("☢"));
    }

    #[test]
    fn the_title_is_encoded_like_encode_uri_component() {
        let http = Stub::new(r#"{"status": true, "result": {}}"#);
        let _ = run(http.clone(), "lirik 100% & co's");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.nexray.eu.cc/search/lyrics?q=lirik%20100%25%20%26%20co's"
        );
    }

    #[test]
    fn a_very_long_title_is_sent_whole() {
        let http = Stub::new(r#"{"status": true, "result": {}}"#);
        let _ = run(http.clone(), &"a".repeat(4_000));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }
}