//! `plugins/ai/txt2img2.js` — text to image lewat Flux Klein 4B (Gradio).
//!
//! Scraper `Txt2Img2` (`src/scraper/txt2img2.js`) ikut diport di sini karena
//! belum ada di crate ini. Tiga langkahnya: cek prompt ke server action
//! upsampler, join antrean Gradio, lalu baca stream SSE antrean.
//!
//! `ponytail:` `HttpGet` tidak punya POST dengan `Content-Type: text/plain`,
//! jadi `checkPrompt` memakai `post_json` — body `["<prompt>"]` sama persis,
//! hanya header isian yang beda.
//!
//! `ponytail:` `getResult` di JS membaca stream sambil jalan (bisa 180 detik)
//! dan berhenti begitu `process_completed` muncul. `HttpGet` cuma bisa GET
//! penuh, jadi blok SSE yang belum lengkap ikut ikut terbaca — hasil yang
//! dipakai tetap sama, hanya waktunya tidak bisa dipotong.

use oktz_plugin_api::{Ctx, HttpGet, MediaSource, Reply, plugin};
use serde_json::{Value, json};

use crate::common::roll;

const UPSAMPLER_URL: &str = "https://upsampler.com/free-image-generator-no-signup";
const SPACE_URL: &str = "https://black-forest-labs-flux-2-klein-4b.hf.space";

/// Awalan URL hasil yang sudah penuh, dari regex `extractUrl`.
const FULL_URL_PREFIX: &str = "https://black-forest-labs-flux-2-klein-4b.hf.space/gradio_api/file=";
/// Awalan path temporer Gradio, dari regex kedua `extractUrl`.
const TMP_PREFIX: &str = "/tmp/gradio/";

/// Hasil `Txt2Img2`. Kalau `Err`, stringnya = `result.error` di JS.
type Generate = Result<String, String>;

/// `crypto.randomBytes(8).toString("hex")` — 16 digit heksadesimal acak.
fn session_hash() -> String {
    (0..16)
        .map(|_| char::from_digit(roll(16), 16).unwrap_or('0'))
        .collect()
}

/// Akhir rentang `[^"'\\\s]+` — berhenti di kutip, backslash, atau spasi.
fn plain_run_end(text: &str, from: usize) -> usize {
    text[from..]
        .find(|c| matches!(c, '"' | '\'' | '\\') || c.is_whitespace())
        .map_or(text.len(), |i| from + i)
}

/// `/tmp/gradio/[^"'\\\s]+?\.(webp|png|jpg|jpeg)` — `+?` lazy, jadi ambang
/// ekstensi yang paling awal yang menang. Urutan alternasi hanya berpengaruh
/// kalau dua ekstensi cocok di posisi sama; `jpg` tidak pernah mengalahkan
/// `jpeg` karena "jpe" ≠ "jpg".
fn tmp_gradio_url(run: &str) -> Option<String> {
    let mut best: Option<(usize, usize)> = None;
    for ext in [".webp", ".png", ".jpg", ".jpeg"] {
        let mut from = 0usize;
        while let Some(rel) = run[from..].find(ext) {
            let abs = from + rel;
            if best.is_none_or(|(start, _)| abs < start) {
                best = Some((abs, ext.len()));
            }
            from = abs + 1;
        }
    }
    let (start, len) = best?;
    Some(format!(
        "{SPACE_URL}/gradio_api/file={TMP_PREFIX}{}",
        &run[..start + len]
    ))
}

/// `extractUrl(output)` (`src/scraper/txt2img2.js:125-143`).
///
/// `JSON.stringify(output || "")` diganti `serde_json::to_string`; keduanya
/// serialisasi rapat tanpa spasi, dan itu yang dibaca kedua regex.
fn extract_url(output: &Value) -> String {
    let text = serde_json::to_string(output).unwrap_or_default();

    // Regex 1: URL penuh, hasil `fullUrl[0]` INCLUDING prefix-nya. Butuh
    // minimal satu karakter setelah `file=`, kalau tidak ada regex akan coba
    // occurrence berikutnya.
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(FULL_URL_PREFIX) {
        let start = from + rel + FULL_URL_PREFIX.len();
        let end = plain_run_end(&text, start);
        if end > start {
            return text[from + rel..end]
                .replace("\\u0026", "&")
                .replace("\\/", "/");
        }
        from = start;
    }

    // Regex 2: `/tmp/gradio/...` lalu sambungkan ke host Gradio.
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(TMP_PREFIX) {
        let start = from + rel + TMP_PREFIX.len();
        let run = &text[start..plain_run_end(&text, start)];
        if let Some(url) = tmp_gradio_url(run) {
            return url;
        }
        from = start;
    }

    String::new()
}

/// Pembaca SSE `getResult` (`src/scraper/txt2img2.js:41-123`).
///
/// `buffer.split("\n\n")` lalu `blocks.pop()`: blok terakhir yang tidak
/// diakhiri baris kosong kosong TIDAK diproses. Ditiru di sini.
fn read_queue_sse(raw: &str, event_id: &str) -> Generate {
    let blocks: Vec<&str> = raw.split("\n\n").collect();
    for block in blocks.iter().take(blocks.len().saturating_sub(1)) {
        let Some(line) = block.split('\n').find(|l| l.starts_with("data: ")) else {
            continue;
        };
        // `replace("data: ", "")` di JS hanya melepas satu kemunculan pertama.
        let payload = line.strip_prefix("data: ").unwrap_or(line).trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        let Ok(json) = serde_json::from_str::<Value>(payload) else {
            return Err(format!("JSON tidak bisa dibaca: {payload}"));
        };
        if json
            .get("event_id")
            .is_some_and(|v| !matches!(v, Value::Null | Value::Bool(false)))
            && json.get("event_id").and_then(Value::as_str) != Some(event_id)
        {
            continue;
        }
        match json.get("msg").and_then(Value::as_str) {
            Some("process_completed") => {
                let url = extract_url(json.get("output").unwrap_or(&Value::Null));
                if url.is_empty() {
                    return Err("URL hasil tidak ditemukan".to_string());
                }
                return Ok(url);
            }
            Some("process_failed") => return Err("Generate gagal".to_string()),
            _ => {}
        }
    }
    // `res.data.on("end")` di JS menolak kalau tidak ada hasil.
    Err("Stream selesai tanpa hasil".to_string())
}

/// `Txt2Img2(prompt)` (`src/scraper/txt2img2.js:145-195`).
fn txt2img2(http: &dyn HttpGet, prompt: &str) -> Generate {
    // `checkPrompt` (`txt2img2.js:16-39`): kegagalan apa pun di dalamnya
    // ditelan `catch` dan dianggap "tidak flagged, tidak rate limited".
    if let Ok(res) = http.post_json(UPSAMPLER_URL, &json!([prompt])) {
        let body = res.text();
        if body.contains("\"flagged\":true") {
            return Err("Prompt terdeteksi tidak aman".to_string());
        }
        if body.contains("\"rateLimited\":true") {
            return Err("Rate limited, coba lagi nanti".to_string());
        }
    }

    let session = session_hash();
    let join = json!({
        "data": [prompt, [], "Distilled (4 steps)", 0, true, 1024, 1024, 4, 1, false],
        "event_data": Value::Null,
        "fn_index": 6,
        "trigger_id": Value::Null,
        "session_hash": session,
    });
    let joined = http
        .post_json(&format!("{SPACE_URL}/gradio_api/queue/join?"), &join)
        .map_err(|e| e.message)?;
    let Some(event_id) = joined
        .json()
        .as_ref()
        .and_then(|v| v.get("event_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return Err("event_id tidak ditemukan".to_string());
    };

    let queue_url = format!("{SPACE_URL}/gradio_api/queue/data?session_hash={session}");
    let stream = http.get(&queue_url).map_err(|e| e.message)?;
    read_queue_sse(&stream.text(), &event_id)
}

/// `txt2img2.js:21-33`.
fn help(prefix: &str) -> String {
    format!(
        "🎨 *Text to Image (Flux)*\n\n\
         Buat gambar dari deskripsi teks pakai AI Flux Klein 4B.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}txt2img2 <deskripsi>*\n\n\
         *CONTOH:*\n\
         > *{prefix}txt2img2 Mobil Lamborghini revuelto*\n\
         > *{prefix}txt2img2 Kucing lucu pakai topi*\n\n\
         _Proses generate agak lama, sekitar 30-60 detik_"
    )
}

#[plugin(
    name = "text2img4",
    alias = ["t2i2", "imggen2", "flux"],
    category = "ai",
    description = "Buat gambar dari teks pakai Flux Klein 4B",
    usage = ".txt2img2 <deskripsi gambar>",
    example = ".txt2img2 Mobil Lamborghini revuelto",
    cooldown = 30,
    energi = 3,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn text2img2(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    match txt2img2(&*ctx.http, &text) {
        Ok(url) => Reply::image(
            MediaSource::url(url),
            format!("🎨 *Flux Klein 4B*\n\n> Prompt: *{text}*"),
        )
        .with_reaction("✅"),
        // `catch` di `txt2img2.js:50-54` (kalimat tetap "Gagal generate gambar,
        // coba lagi nanti") hanya menutup `sock.sendMedia`. Media di sini cuma
        // URL dan diunggah pipeline, jadi cabang itu tidak ditulis.
        Err(error) => Reply::text(format!("❌ *Generate Gagal*\n\n> {error}")).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub yang memilih respons lewat potongan URL, supaya satu handler bisa
    /// punya balasan berbeda untuk tiap langkah scraper.
    #[derive(Debug)]
    struct Stub {
        routes: Vec<(&'static str, Option<String>)>,
        seen: Mutex<Vec<(String, String)>>,
    }

    impl Stub {
        fn new(routes: Vec<(&'static str, Option<String>)>) -> Arc<Stub> {
            Arc::new(Stub {
                routes,
                seen: Mutex::new(Vec::new()),
            })
        }

        fn pick(&self, url: &str) -> Result<HttpResponse, HttpError> {
            let Some((_, Some(body))) = self.routes.iter().find(|(frag, _)| url.contains(*frag))
            else {
                return Err(HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.clone().into_bytes()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push((url.into(), String::new()));
            self.pick(url)
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.into(), body.to_string()));
            self.pick(url)
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini tidak POST form")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini tidak POST multipart")
        }
    }

    /// Rute lengkap: cek prompt bersih -> join dengan `event_id` -> SSE sukses.
    fn stub_ok(sse: &str) -> Arc<Stub> {
        Stub::new(vec![
            (UPSAMPLER_URL, Some(String::new())),
            ("queue/join", Some(r#"{"event_id":"evt-1"}"#.into())),
            ("queue/data", Some(sse.into())),
        ])
    }

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "txt2img2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("text2img4").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("text2img4").unwrap();
        assert_eq!(p.meta.name, "text2img4");
        assert_eq!(p.meta.alias, ["t2i2", "imggen2", "flux"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Buat gambar dari teks pakai Flux Klein 4B");
        assert_eq!(p.meta.usage, ".txt2img2 <deskripsi gambar>");
        assert_eq!(p.meta.example, ".txt2img2 Mobil Lamborghini revuelto");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 3);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["text2img4", "t2i2", "imggen2", "flux"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("text2img4"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_prompt_shows_the_usage_card_without_a_request() {
        let http = stub_ok("data: [DONE]\n\n");
        let r = run(http.clone(), "");
        assert!(body(&r).contains("🎨 *Text to Image (Flux)*"));
        assert!(body(&r).contains("*.txt2img2 <deskripsi>*"));
        assert_eq!(emoji(&r), "❌");
        assert!(http.seen.lock().unwrap().is_empty(), "tidak boleh request");
    }

    #[test]
    fn a_successful_generate_sends_the_flux_caption() {
        let sse = concat!(
            "data: {\"msg\":\"estimation_started\",\"event_id\":\"evt-1\"}\n\n",
            "data: {\"msg\":\"process_completed\",\"event_id\":\"evt-1\",\
             \"output\":{\"url\":\"/tmp/gradio/aa/out.webp\"}}\n\n",
            "data: [DONE]\n\n"
        );
        let http = stub_ok(sse);
        let r = run(http.clone(), "Mobil Lamborghini revuelto");
        assert!(r.has_media());
        let Some(MediaSource::Url { url, .. }) = r.media_source() else {
            panic!("media harus URL");
        };
        assert_eq!(
            url.as_str(),
            format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/aa/out.webp")
        );
        assert_eq!(
            body(&r),
            "🎨 *Flux Klein 4B*\n\n> Prompt: *Mobil Lamborghini revuelto*"
        );
        assert_eq!(emoji(&r), "✅");

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 3, "cek prompt, join, lalu baca antrean");
        assert_eq!(seen[0].0, UPSAMPLER_URL);
        assert_eq!(seen[0].1, r#"["Mobil Lamborghini revuelto"]"#);
        let joined: Value = serde_json::from_str(&seen[1].1).unwrap();
        assert_eq!(joined["fn_index"], 6);
        assert_eq!(joined["data"][2], "Distilled (4 steps)");
        assert!(seen[2].0.starts_with(&format!("{SPACE_URL}/gradio_api/queue/data?session_hash=")));
    }

    #[test]
    fn a_flagged_prompt_reports_the_prompt_error() {
        let http = Stub::new(vec![(UPSAMPLER_URL, Some(r#"{"flagged":true}"#.into()))]);
        let r = run(http.clone(), "kucing telanjang");
        assert_eq!(
            body(&r),
            "❌ *Generate Gagal*\n\n> Prompt terdeteksi tidak aman"
        );
        assert_eq!(emoji(&r), "☢");
    }

    #[test]
    fn a_rate_limited_prompt_reports_the_rate_limit() {
        let http = Stub::new(vec![(UPSAMPLER_URL, Some(r#"{"rateLimited":true}"#.into()))]);
        let r = run(http.clone(), "kucing");
        assert_eq!(
            body(&r),
            "❌ *Generate Gagal*\n\n> Rate limited, coba lagi nanti"
        );
    }

    #[test]
    fn a_join_without_event_id_stops_before_reading_the_queue() {
        let http = Stub::new(vec![
            (UPSAMPLER_URL, Some(String::new())),
            ("queue/join", Some("{}".into())),
        ]);
        let r = run(http.clone(), "kucing");
        assert_eq!(
            body(&r),
            "❌ *Generate Gagal*\n\n> event_id tidak ditemukan"
        );
    }

    #[test]
    fn a_failed_or_empty_stream_reports_its_own_error() {
        let gagal = Stub::new(vec![
            (UPSAMPLER_URL, Some(String::new())),
            ("queue/join", Some(r#"{"event_id":"evt-1"}"#.into())),
            ("queue/data", Some("data: {\"msg\":\"process_failed\",\"event_id\":\"evt-1\"}\n\n".into())),
        ]);
        assert_eq!(
            body(&run(gagal.clone(), "kucing")),
            "❌ *Generate Gagal*\n\n> Generate gagal"
        );

        let kosong = stub_ok("data: [DONE]\n\n");
        assert_eq!(
            body(&run(kosong.clone(), "kucing")),
            "❌ *Generate Gagal*\n\n> Stream selesai tanpa hasil"
        );
    }

    #[test]
    fn a_transport_failure_before_any_step_reports_the_short_error() {
        // `Txt2Img2` tidak pernah melempar: `catch` di dalamnya mengubah semua
        // kegagalan jadi `result.error`, jadi teks error ikut apa adanya.
        let r = run(Stub::new(vec![]), "kucing");
        assert_eq!(
            body(&r),
            "❌ *Generate Gagal*\n\n> offline",
            "error dari transport ikut tampil apa adanya"
        );
        assert_eq!(emoji(&r), "☢");
    }

    #[test]
    fn sse_from_another_event_id_is_skipped() {
        let sse = concat!(
            "data: {\"msg\":\"process_completed\",\"event_id\":\"lain\",\
             \"output\":{\"url\":\"/tmp/gradio/salah.webp\"}}\n\n",
            "data: {\"msg\":\"process_completed\",\"event_id\":\"evt-1\",\
             \"output\":{\"url\":\"/tmp/gradio/benar.webp\"}}\n\n"
        );
        let url = read_queue_sse(sse, "evt-1").unwrap();
        assert_eq!(
            url,
            format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/benar.webp")
        );
    }

    #[test]
    fn the_last_sse_block_is_dropped_like_js_pop() {
        // Tanpa baris kosong penutup, blok terakhir tidak pernah diproses.
        let sse = "data: {\"msg\":\"process_completed\",\"output\":{\"url\":\"/tmp/gradio/a.png\"}}";
        assert_eq!(
            read_queue_sse(sse, "evt-1").unwrap_err(),
            "Stream selesai tanpa hasil"
        );
    }

    #[test]
    fn extract_url_picks_the_leftmost_gradio_path() {
        let out = json!({"a": "/tmp/gradio/abc.webp", "b": "/tmp/gradio/abc.png"});
        assert_eq!(
            extract_url(&out),
            format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/abc.webp")
        );
    }

    #[test]
    fn the_lazy_extension_stops_at_the_earliest_match() {
        // `img.png.webp`: `+?` memendek dulu ke `img`, lalu `.png` menang —
        // bukan `.webp` yang lebih akhir.
        assert_eq!(
            tmp_gradio_url("gen/img.png.webp").as_deref(),
            Some(&*format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/gen/img.png"))
        );
        // Setelah `.` tinggal "jpeg": `jpg` tidak bisa cocok ("jpe"), jadi
        // alternatif `.jpeg` yang menang meski tertulis belakangan.
        assert_eq!(
            tmp_gradio_url("f.jpeg").as_deref(),
            Some(&*format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/f.jpeg"))
        );
        assert_eq!(tmp_gradio_url("tanpa-ekstensi"), None);
    }

    #[test]
    fn a_full_gradio_url_wins_over_the_tmp_path_regex() {
        let out = json!({
            "tmp": "/tmp/gradio/aaa.png",
            "cdn": format!("{FULL_URL_PREFIX}bbb.webp"),
        });
        assert_eq!(extract_url(&out), "https://black-forest-labs-flux-2-klein-4b.hf.space/gradio_api/file=bbb.webp");
        assert_eq!(extract_url(&json!("tidak ada url")), "");
    }

    #[test]
    fn a_session_hash_is_sixteen_hex_characters() {
        let h = session_hash();
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "{h}");
    }
}