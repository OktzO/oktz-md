//! `plugins/download/tiktokdl2.js` — download video/slide TikTok tanpa watermark.
//!
//! Scraper savett sudah ada di crate `oktz-scraper` sebagai
//! `oktz_scraper::tiktok::ttdown_from_save_tt` (`src/scraper/tiktok.js:87-135`),
//! jadi plugin ini hanya memetakan hasilnya ke balasan.
//!
//! Penyimpangan yang tercatat apa adanya:
//!
//! - **Provider cadangan `ttdown`.** JS cuma memakai savett
//!   (`tiktokdl2.js:96-100,137`); di sini kegagalan savett jatuh ke
//!   `oktz_scraper::tiktok::ttdown` (TikWM → savett → YuuLabs → MusicalDown)
//!   supaya tautan yang ditolak savett tetap ada jalan.
//! - **Album slide tidak dikirim.** JS merakit `albumMessage` + `relayMessage`
//!   per slide (`tiktokdl2.js:170-249`); kontrak `Reply` cuma satu media per
//!   pesan. Cabang foto diperlakukan seperti album kosong: tanpa pesan
//!   `📸 Mengirim N slide`, lanjut ke cabang mp3 (note + audio) atau catch.
//! - **Reaksi awal `⏱️` dibuang** (`tiktokdl2.js:134`), mengikuti konvensi
//!   repo: `Reply` hanya membawa satu reaksi, jadi reaksi penutup yang ikut.
//! - **Reaksi `✅` cabang mp3 hilang** (`tiktokdl2.js:261`). `Reply::Reaction`
//!   di dalam `Sequence` dianggap varian tak didukung pipeline
//!   (`pipeline.rs:3023-3029`), jadi yang terkirim hanya note + audio.
//! - **Statistik selalu `-`.** Caption JS memakai `views`/`likes`/`bookmarks`/
//!   `comments`/`shares`/`duration` dari `#video-info` (`tiktokdl2.js:34-46`);
//!   scraper port tidak mengurai blok itu, jadi nilainya sama seperti
//!   fallback `|| '-'` di JS.
//! - **Penentuan video dari `type`.** JS memakai `downloads.nowm[0]`
//!   (`tiktokdl2.js:146-147`); di sini unduhan pertama yang bukan
//!   watermark/mp3/foto — TikWM menamai HD-nya `nowatermark_hd` dan
//!   MusicalDown mp4-nya `mp4`, keduanya video tanpa watermark.
//!
//! `pub mod tiktokdl2;` perlu ditambah manual ke `src/download/mod.rs` —
//! berkas ini belum ikut dikompilasi tanpa baris itu.

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};
use serde_json::Value;

/// `tiktokdl2.js:131` — regex `/tiktok\.com|vt\.tiktok/i` tidak cocok.
pub(crate) const URL_TIDAK_VALID: &str = "❌ URL tidak valid. Gunakan link TikTok.";

/// `tiktokdl2.js:252` — catatan saat kontennya cuma audio.
pub(crate) const NOTE_MP3: &str =
    "🍀 *NOTE*\n> Konten ini tidak memiliki video/slide, mengirim audio saja...";

/// `tiktokdl2.js:151` — UA unduhan video (Chrome 120, beda dari UA savett).
pub(crate) const UA_VIDEO: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

/// `tiktokdl2.js:152` — `Referer` unduhan video.
pub(crate) const REFERER_VIDEO: &str = "https://www.tiktok.com/";

/// `tiktokdl2.js:123-127` — kartu pakai; `${m.prefix}` muncul dua kali.
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "╭┈┈⬡「 🎵 *ᴛɪᴋᴛᴏᴋ ᴅᴏᴡɴʟᴏᴀᴅ* 」\n\
         ┃ ㊗ ᴜsᴀɢᴇ: `{prefix}tiktok2 <url>`\n\
         ╰┈┈⬡\n\n\
         > Contoh: {prefix}tiktok2 https://vt.tiktok.com/xxx"
    )
}

/// `tiktokdl2.js:139-144` — caption. `views`/`likes`/`comments`/`shares`/
/// `duration` tidak dibawa scraper, jadi selalu `-` seperti `|| '-'` di JS.
/// Baris komentar memakai U+FFFD apa adanya dari JS (byte `ef bf bd`).
pub(crate) fn caption(hasil: &Value) -> String {
    let username = hasil
        .get("author")
        .and_then(|a| a.get("username"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("-");
    format!(
        "✅ *Done kak*\n\n\
         👤 *{username}*\n\
         👁️ Views: - | ❤️ Likes: -\n\
         \u{fffd} Comments: - | 🔗 Shares: -\n\
         ⏱️ Duration: -"
    )
}

/// `result.downloads.nowm[0]` (`tiktokdl2.js:147`) — unduhan video TANPA
/// watermark pertama. `type` kosong/hilang dilewati: tanpa label, tidak
/// diketahui itu video atau bukan.
pub(crate) fn unduhan_video(hasil: &Value) -> Option<&Value> {
    hasil
        .get("downloads")
        .and_then(Value::as_array)?
        .iter()
        .find(|d| match d.get("type").and_then(Value::as_str) {
            Some(t) => !matches!(t, "watermark" | "mp3" | "audio" | "photo"),
            None => false,
        })
}

/// `result.mp3[0]` (`tiktokdl2.js:240,256`) — URL audio pertama.
pub(crate) fn unduhan_mp3(hasil: &Value) -> Option<&str> {
    hasil
        .get("downloads")
        .and_then(Value::as_array)?
        .iter()
        .find(|d| d.get("type").and_then(Value::as_str) == Some("mp3"))
        .and_then(|d| d.get("url"))
        .and_then(Value::as_str)
}

#[plugin(
    name = "tiktok2",
    alias = ["tt2", "tiktokdl2", "ttdown2"],
    category = "download",
    description = "Download video/slide TikTok tanpa watermark",
    usage = ".tiktok2 <url>",
    example = ".tiktok2 https://vt.tiktok.com/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tiktokdl2(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(bantuan(&ctx.prefix));
    }

    // `/tiktok\.com|vt\.tiktok/i` — pencocokan substring, bukan host.
    let kecil = url.to_lowercase();
    if !kecil.contains("tiktok.com") && !kecil.contains("vt.tiktok") {
        return Reply::text(URL_TIDAK_VALID);
    }

    // ponytail: reaksi ⏱️ (`tiktokdl2.js:134`) hilang — `Reply` hanya
    // membawa satu reaksi, jadi hanya reaksi penutup di bawah yang dikirim.

    // `tiktokdl2.js:267-271` — catch membalas template error + reaksi ☢.
    let gagal = || {
        Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
    };

    // ponytail: JS cuma memakai savett; `ttdown` ditambahkan sebagai cadangan
    // (lihat header). Hapus `or_else` kalau ingin persis seperti JS.
    let hasil = match oktz_scraper::tiktok::ttdown_from_save_tt(&*ctx.http, url)
        .or_else(|_| oktz_scraper::tiktok::ttdown(&*ctx.http, url))
    {
        Ok(hasil) => hasil,
        Err(_) => return gagal(),
    };

    if let Some(video) = unduhan_video(&hasil) {
        // `axios.get(nowm[0])` dengan url kosong melempar di JS, jadi masuk
        // catch — bukan lanjut ke cabang mp3.
        let Some(alamat) = video
            .get("url")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
        else {
            return gagal();
        };
        let Ok(res) = ctx.http.call(
            &HttpCall::get(alamat)
                .header("User-Agent", UA_VIDEO)
                .header("Referer", REFERER_VIDEO),
        ) else {
            return gagal();
        };
        // axios melempar untuk 4xx/5xx (`tiktokdl2.js:147-154`).
        if !(200..300).contains(&res.status) {
            return gagal();
        }
        return Reply::video(
            MediaSource::bytes(res.body.bytes().to_vec()).with_mimetype("video/mp4"),
            caption(&hasil),
        )
        .with_reaction("✅");
    }

    if let Some(mp3) = unduhan_mp3(&hasil) {
        // ponytail: album slide butuh `albumMessage` + `relayMessage`
        // (`tiktokdl2.js:196-234`), tidak ada di kontrak `Reply`; cabang foto
        // diperlakukan seperti album kosong sehingga mendarat di note + audio
        // ini. Reaksi ✅ (`tiktokdl2.js:261`) juga tidak bisa menempel di
        // `Sequence` (pipeline melewati varian `Reaction`).
        return Reply::sequence(vec![
            Reply::text(NOTE_MP3),
            Reply::audio(MediaSource::url(mp3).with_mimetype("audio/mpeg")),
        ]);
    }

    // `tiktokdl2.js:265` — `throw new Error('Tidak ada media yang dapat diunduh')`.
    gagal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpMethod, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// Caption yang diharapkan; baris komentar memuat U+FFFD seperti JS.
    const CAPTION_BUDI: &str = "✅ *Done kak*\n\n👤 *Budi*\n👁️ Views: - | ❤️ Likes: -\n\u{fffd} Comments: - | 🔗 Shares: -\n⏱️ Duration: -";
    const CAPTION_TOK: &str = "✅ *Done kak*\n\n👤 *Tok*\n👁️ Views: - | ❤️ Likes: -\n\u{fffd} Comments: - | 🔗 Shares: -\n⏱️ Duration: -";

    /// Balasan TikWM (`src/scraper/tiktok.js:26-66`): `play` → nowatermark.
    const TIKWM_VIDEO: &str = r#"{"code":0,"data":{"title":"Judul","duration":30,"play":"/sd.mp4","music":"/m.mp3","author":{"nickname":"Tok"}}}"#;

    /// `(status, body, headers)` per permintaan.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Halaman token savett: csrf + cookie yang dipotong di `;`.
    fn halaman_token() -> Balasan {
        (
            200,
            r#"<input name="csrf_token" value="TOKENSAYA">"#.to_string(),
            vec![("set-cookie".to_string(), "sess=1; Path=/".to_string())],
        )
    }

    /// Halaman hasil savett berisi satu opsi MP4 tanpa watermark.
    fn halaman_video() -> Balasan {
        (
            200,
            r#"<div id="video-info"><h3>  Budi  </h3></div>
               <select id="formatselect">
                 <option value="{&quot;URL&quot;:[&quot;https://cdn.example/v.mp4&quot;]}">MP4 720p</option>
               </select>"#
                .to_string(),
            Vec::new(),
        )
    }

    /// Halaman hasil savett berisi slide + satu opsi MP3.
    fn halaman_foto_dan_mp3() -> Balasan {
        (
            200,
            r#"<div class="carousel-item" data-data="{&quot;URL&quot;:[&quot;https://cdn.example/1.jpg&quot;]}"></div>
               <select id="formatselect">
                 <option value="{&quot;URL&quot;:[&quot;https://cdn.example/a.mp3&quot;]}">MP3 Audio</option>
               </select>"#
                .to_string(),
            Vec::new(),
        )
    }

    /// Halaman hasil savett berisi slide tanpa opsi MP3.
    fn halaman_foto_saja() -> Balasan {
        (
            200,
            r#"<div class="carousel-item" data-data="{&quot;URL&quot;:[&quot;https://cdn.example/1.jpg&quot;]}"></div>"#
                .to_string(),
            Vec::new(),
        )
    }

    /// HTTP stub tanpa jaringan: balasan berurutan, habis berarti offline.
    #[derive(Debug)]
    struct Stub {
        balasan: Mutex<Vec<Balasan>>,
        hitung: Mutex<usize>,
        panggilan: Mutex<Vec<HttpCall>>,
    }

    impl Stub {
        fn baru(balasan: Vec<Balasan>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
                panggilan: Mutex::new(Vec::new()),
            })
        }

        fn offline() -> Arc<Stub> {
            Stub::baru(Vec::new())
        }
    }

    impl HttpGet for Stub {
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.panggilan.lock().unwrap().push(req.clone());
            let i = {
                let mut hitung = self.hitung.lock().unwrap();
                let i = *hitung;
                *hitung += 1;
                i
            };
            match self.balasan.lock().unwrap().get(i).cloned() {
                Some((status, teks, headers)) => Ok(HttpResponse {
                    status,
                    body: HttpBody::Bytes {
                        data: Arc::from(teks.into_bytes()),
                        mimetype: None,
                    },
                    headers,
                }),
                None => Err(HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.call(&HttpCall::get(url))
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tiktok2".into(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("tiktok2").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    /// `without_mutations` hanya membuang `Mutate`, jadi `Reaction { reply }`
    /// masih harus dibuka satu lapis.
    fn isi(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            lain => lain,
        }
    }

    /// Daftar isi `Reply::Sequence` (dibuka dari reaksi kalau ada).
    fn urutan(r: &Reply) -> &[Reply] {
        match isi(r) {
            Reply::Sequence(list) => list,
            lain => panic!("harus sequence, dapat {lain:?}"),
        }
    }

    #[test]
    fn terdaftar_dengan_config_js() {
        let p = oktz_plugin_api::find("tiktok2").unwrap();
        assert_eq!(p.meta.name, "tiktok2");
        assert_eq!(p.meta.alias, ["tt2", "tiktokdl2", "ttdown2"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video/slide TikTok tanpa watermark");
        assert_eq!(p.meta.usage, ".tiktok2 <url>");
        assert_eq!(p.meta.example, ".tiktok2 https://vt.tiktok.com/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn semua_alias_menunjuk_plugin_ini() {
        for a in ["tt2", "tiktokdl2", "ttdown2"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "tiktok2");
        }
    }

    #[test]
    fn argumen_kosong_atau_spasi_menampilkan_kartu_pakai() {
        for text in ["", "   "] {
            let http = Stub::offline();
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), bantuan("."), "input {text:?}");
            assert_eq!(reaction_of(&r), None, "input {text:?}");
            assert!(http.panggilan.lock().unwrap().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn url_bukan_tiktok_ditolak_tanpa_jaringan() {
        for text in ["https://contoh.example/video", "tiktok", "youtube.com/watch?v=1"] {
            let http = Stub::offline();
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), URL_TIDAK_VALID, "input {text:?}");
            assert_eq!(reaction_of(&r), None, "input {text:?}");
            assert!(http.panggilan.lock().unwrap().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn url_hanya_perlu_memuat_penanda_tiktok_tanpa_peka_huruf_besar_kecil() {
        // `/tiktok\.com|vt\.tiktok/i` — pencocokan substring di teks yang
        // sudah di-trim, jadi host aneh dan huruf besar pun lolos.
        for text in [
            "https://www.tiktok.com/@budi/video/1",
            "https://vt.tiktok.com/abc",
            "https://contoh.example/?u=tiktok.com",
            "  HTTPS://VT.TIKTOK.COM/ABC  ",
        ] {
            let r = run(Stub::offline(), text);
            assert_ne!(text_of(&r), URL_TIDAK_VALID, "input {text:?} harus lolos");
            assert_eq!(reaction_of(&r), Some("☢"), "input {text:?}");
        }
    }

    #[test]
    fn kartu_pakai_memuat_prefix_dua_kali() {
        let teks = bantuan("#");
        assert_eq!(teks.matches("#tiktok2").count(), 2);
        assert!(teks.starts_with("╭┈┈⬡「 🎵 *ᴛɪᴋᴛᴏᴋ ᴅᴏᴡɴʟᴏᴀᴅ* 」"));
        assert!(teks.ends_with("> Contoh: #tiktok2 https://vt.tiktok.com/xxx"));
    }

    #[test]
    fn video_savett_dikirim_dengan_caption_dan_header_js() {
        let http = Stub::baru(vec![
            halaman_token(),
            halaman_video(),
            (200, "VIDEO".to_string(), Vec::new()),
        ]);
        let r = run(http.clone(), "https://www.tiktok.com/@budi/video/1");
        assert_eq!(reaction_of(&r), Some("✅"));
        match isi(&r) {
            Reply::Video { media, caption, .. } => {
                assert_eq!(caption, CAPTION_BUDI);
                assert_eq!(
                    media,
                    &Some(MediaSource::bytes(b"VIDEO".to_vec()).with_mimetype("video/mp4"))
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
        let p = http.panggilan.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].url, "https://savett.cc/en1/download");
        assert_eq!(p[2].method, HttpMethod::Get);
        assert_eq!(p[2].url, "https://cdn.example/v.mp4");
        assert_eq!(cari_header(&p[2], "user-agent").as_deref(), Some(UA_VIDEO));
        assert_eq!(cari_header(&p[2], "referer").as_deref(), Some(REFERER_VIDEO));
    }

    #[test]
    fn unduhan_video_gagal_http_masuk_catch() {
        let http = Stub::baru(vec![
            halaman_token(),
            halaman_video(),
            (500, "gagal".to_string(), Vec::new()),
        ]);
        let r = run(http, "https://www.tiktok.com/@budi/video/1");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "tiktok2"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn foto_dan_mp3_mengirim_note_lalu_audio_tanpa_reaksi() {
        let http = Stub::baru(vec![halaman_token(), halaman_foto_dan_mp3()]);
        let r = run(http.clone(), "https://www.tiktok.com/@budi/photo/1");
        assert_eq!(reaction_of(&r), None, "reaksi ✅ hilang di dalam Sequence");
        let bagian = urutan(&r);
        assert_eq!(bagian.len(), 2);
        assert_eq!(bagian[0].without_mutations().text_body(), Some(NOTE_MP3));
        match bagian[1].without_mutations() {
            Reply::Audio { media, .. } => assert_eq!(
                media,
                &Some(MediaSource::url("https://cdn.example/a.mp3").with_mimetype("audio/mpeg"))
            ),
            lain => panic!("harus audio, dapat {lain:?}"),
        }
        assert_eq!(http.panggilan.lock().unwrap().len(), 2);
    }

    #[test]
    fn foto_tanpa_mp3_masuk_catch() {
        // Album slide tidak bisa dikirim (lihat header), jadi satu-satunya
        // foto itu berakhir di template error.
        let http = Stub::baru(vec![halaman_token(), halaman_foto_saja()]);
        let r = run(http.clone(), "https://www.tiktok.com/@budi/photo/1");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "tiktok2"));
        assert_eq!(reaction_of(&r), Some("☢"));
        assert_eq!(http.panggilan.lock().unwrap().len(), 2);
    }

    #[test]
    fn savett_gagal_jatuh_ke_ttdown() {
        let http = Stub::baru(vec![
            (200, "<p>tanpa form</p>".to_string(), Vec::new()),
            (200, TIKWM_VIDEO.to_string(), Vec::new()),
            (200, "VIDEO".to_string(), Vec::new()),
        ]);
        let r = run(http.clone(), "https://www.tiktok.com/@tok/video/2");
        assert_eq!(reaction_of(&r), Some("✅"));
        match isi(&r) {
            Reply::Video { media, caption, .. } => {
                assert_eq!(caption, CAPTION_TOK);
                assert_eq!(
                    media,
                    &Some(MediaSource::bytes(b"VIDEO".to_vec()).with_mimetype("video/mp4"))
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
        let p = http.panggilan.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert!(
            p[1].url.starts_with("https://www.tikwm.com/api/?url="),
            "provider cadangan dipakai, dapat {}",
            p[1].url
        );
        assert_eq!(p[2].url, "https://www.tikwm.com/sd.mp4");
    }

    #[test]
    fn semua_provider_gagal_masuk_catch() {
        let http = Stub::offline();
        let r = run(http.clone(), "https://www.tiktok.com/@budi/video/1");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "tiktok2"));
        assert_eq!(reaction_of(&r), Some("☢"));
        // Urutannya: token savett, TikWM, savett, YuuLabs, MusicalDown — url
        // panjang tidak punya varian `resolved`, jadi tepat 5 percobaan.
        assert_eq!(http.panggilan.lock().unwrap().len(), 5);
    }

    #[test]
    fn caption_memakai_username_atau_strip() {
        let dengan = serde_json::json!({ "author": { "username": "Budi" } });
        assert_eq!(caption(&dengan), CAPTION_BUDI);
        // `tiktokdl2.js:143` memuat U+FFFD (byte ef bf bd) sebelum " Comments:".
        assert!(
            caption(&dengan)
                .as_bytes()
                .windows(3)
                .any(|w| w == b"\xef\xbf\xbd"),
            "caption harus memuat U+FFFD seperti JS"
        );
        for h in [
            serde_json::json!({}),
            serde_json::json!({ "author": {} }),
            serde_json::json!({ "author": { "username": "" } }),
            serde_json::json!({ "author": { "username": null } }),
        ] {
            assert!(caption(&h).contains("👤 *-*"), "bentuk {h}");
        }
    }

    #[test]
    fn unduhan_video_melewati_watermark_mp3_dan_foto() {
        let hasil = serde_json::json!({
            "downloads": [
                { "type": "watermark", "url": "https://a/wm.mp4" },
                { "type": "mp3", "url": "https://a/m.mp3" },
                { "type": "photo", "url": "https://a/1.jpg" },
                { "type": "nowatermark_hd", "url": "https://a/hd.mp4" },
                { "type": "nowatermark", "url": "https://a/sd.mp4" }
            ]
        });
        assert_eq!(unduhan_video(&hasil).unwrap()["url"], "https://a/hd.mp4");
        for h in [
            serde_json::json!({ "downloads": [] }),
            serde_json::json!({ "downloads": [{ "url": "https://a/x" }] }),
            serde_json::json!({ "downloads": [{ "type": "audio", "url": "https://a/m" }] }),
            serde_json::json!({}),
            serde_json::json!({ "downloads": "bukan array" }),
        ] {
            assert!(unduhan_video(&h).is_none(), "bentuk {h}");
        }
    }

    #[test]
    fn unduhan_mp3_mengambil_yang_pertama() {
        let hasil = serde_json::json!({
            "downloads": [
                { "type": "nowatermark", "url": "https://a/v.mp4" },
                { "type": "mp3", "url": "https://a/1.mp3" },
                { "type": "mp3", "url": "https://a/2.mp3" }
            ]
        });
        assert_eq!(unduhan_mp3(&hasil), Some("https://a/1.mp3"));
        assert_eq!(unduhan_mp3(&serde_json::json!({ "downloads": [] })), None);
        assert_eq!(
            unduhan_mp3(&serde_json::json!({ "downloads": [{ "type": "mp3" }] })),
            None,
            "url hilang berarti tidak ada yang bisa dikirim"
        );
    }

    #[test]
    fn plugin_tidak_pernah_mutasi_store() {
        let http = Stub::baru(vec![
            halaman_token(),
            halaman_video(),
            (200, "V".to_string(), Vec::new()),
        ]);
        let r = run(http, "https://www.tiktok.com/@budi/video/1");
        assert!(r.mutations().is_empty());
    }
}
