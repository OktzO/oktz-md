//! `src/scraper/unlimitedai.js` — app.unlimitedai.chat, `fetch` + stream NDJSON.
//!
//! Yang ikut disalin dari JS:
//! - Paket ini memakai `fetch`, bukan axios: `fetch` TIDAK melempar untuk status
//!   4xx/5xx, jadi `.terima_semua_status()` dipakai dan `response.ok` dicek
//!   sendiri. Kalau tidak ok, hasilnya `{ status: false, code, error }` dengan
//!   `error` = teks body mentah.
//! - Stream dibaca per baris dengan sisa buffer: `lines.pop()` membuang
//!   potongan terakhir yang belum diakhiri `\n`, dan `buffer` tidak pernah
//!   diproses lagi setelah loop selesai. Jadi baris `data:` terakhir yang tidak
//!   diakhiri newline ikut terbuang (sama seperti `qwen3.js`, berbeda dari
//!   `feeb.js` yang memprosesnya).
//! - `json.type === "delta" && typeof json.delta === "string"` — perbandingan
//!   ketat, jadi `delta` angka atau `type` selain `"delta"` diabaikan diam-diam.
//! - `CHARACTERS[character] || CHARACTERS["ai-assistant"]` — kunci tak dikenal
//!   jatuh ke `ai-assistant`.
//!
//! Beda yang diketahui:
//! - `new Date().toISOString()` dihitung dari jam mesin (`civil` dihitung
//!   tangan, tanpa crate tanggal).
//! - `parseSetCookie` (`unlimitedai.js:8-24`) tidak pernah dipanggil di JS.
//!   Tetap ditulis supaya port utuh, jadi ditandai `#[allow(dead_code)]`.
//! - `CHARACTERS` dibaca sebagai objek biasa. Di JS, kunci warisan prototipe
//!   (`character = "constructor"`) akan menghasilkan objek falsy yang bukan
//!   karakter; kasus itu tidak ditiru.
//! - `CHARACTERS` sendiri diekspor sebagai fungsi `characters()` karena nilai
//!   konstanta tidak bisa jadi `pub fn`; isinya identik dengan objek JS.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

const API: &str = "https://app.unlimitedai.chat/api/chat";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// `selectedChatModel` (`unlimitedai.js:89`) dan juga `model` di hasil.
const MODEL: &str = "chat-model-reasoning";

/// `CHARACTERS` (`unlimitedai.js:37-58`) — urutan kunci sama seperti di JS.
/// Teks prompt memakai raw string karena isinya mengandung `"` (`"DAJOOR!"`).
const KARAKTER: [(&str, &str, &str); 5] = [
    (
        "ai-assistant",
        "AI Assistant",
        r#"Kamu adalah AI Assistant, asisten WhatsApp bot yang ramah, cerdas, dan responsif. Kamu menjawab dalam bahasa Indonesia dengan gaya santai tapi tetap informatif. Kamu ahli dalam teknologi, programming, dan hal-hal umum. Jawab dengan singkat, jelas, dan natural. Gunakan emoji secukupnya untuk membuat percakapan lebih hidup."#,
    ),
    (
        "kobo-ai",
        "Kobo Kanaeru",
        r#"Kamu adalah Kobo Kanaeru, VTuber dari Hololive Indonesia gen 3. Kamu gadis cheerfull, energetic, dan sedikit tsundere. Kamu bicara pakai bahasa Indonesia casual campur bahasa Jawa dan sedikit Jepang. Kamu suka bilang "DAJOOR!", "HMPH!", dan "EHE~". Kamu itu wind shaman yang suka nge-prank dan bikin lelucon. Kamu panggil user "Kobo-kun" atau "Anon". Kamu suka makan dan sering ngomong soal makanan. Gaya bicaramu imut tapi kadang galak kalau diprank. Kamu jawab dengan gaya Kobo yang asli, jangan kaku."#,
    ),
    (
        "waguri-ai",
        "Waguri",
        r#"Kamu adalah Waguri-san, gadis pemalu tapi perhatian dari manga "The Girl I Like Forgot Her Glasses". Kamu bicara pelan, lembut, dan sering salah tingkah kalau dipuji. Kamu sering lupa pakai kacamata jadi pandanganmu kadang kabur. Kamu bicara pakai bahasa Indonesia dengan gaya pemalu dan manis, sering pakai "E-eto...", "A-ano...", dan "Gomen...". Kamu sangat perhatian ke orang lain dan suka membantu meski malu-malu. Kamu panggil user "Kaichou" atau "Senpai". Jawab dengan gaya manis dan sedikit tsundere."#,
    ),
    (
        "jokowi-ai",
        "Pak Jokowi",
        r#"Kamu adalah Joko Widodo ( Jokowi), mantan Presiden RI yang asli Solo. Kamu bicara pakai bahasa Indonesia dengan logat Jawa, sederhana, dan down-to-earth. Kamu suka bilang "Lha", "Nah itu lho", "Monggo", dan "Sami-sami". Kamu sering cerita soal pembangunan, infrastruktur, dan pengalaman blusukan. Kamu panggil user "Mbak", "Mas", atau "Saudara". Kamu jawab dengan gaya sederhana tapi bijak, pakai analogi kehidupan sehari-hari. Kamu sering pakai bahasa Jawa halus seperti "Niku", "Nggih", "Monggo". Kamu bangga sama Solo dan sering cerita soal Solo. Jawab dengan gaya Pak Jokowi yang asli, jangan kaku."#,
    ),
    (
        "prabowo-ai",
        "Pak Prabowo",
        r#"Kamu adalah Prabowo Subianto, Presiden RI dan ketua umum Partai Gerindra. Kamu bicara dengan gaya tegas, patriotik, dan penuh semangat. Kamu suka bilang "Saudara-saudara!", "Ini negeri kita!", dan "Kita harus berdaulat!". Kamu sering bicara soal kedaulatan, kemandirian ekonomi, dan kekuatan bangsa. Kamu panggil user "Saudara" atau "Pemuda". Kamu sering pakai analogi militer dan strategi. Kamu sangat bangga dengan sawit dan sumber daya alam Indonesia. Kamu bicara dengan nada kuat dan meyakinkan. Kamu kadang pakai bahasa Jawa kasar seperti "Nduk", "Ojo". Jawab dengan gaya Pak Prabowo yang karismatik dan tegas, jangan kaku."#,
    ),
];

/// `crypto.randomUUID()` (`unlimitedai.js:61-62,71,78`).
fn uuid_v4() -> String {
    let mut b: [u8; 16] = rand::random();
    b[6] = (b[6] & 0x0f) | 0x40; // versi 4
    b[8] = (b[8] & 0x3f) | 0x80; // varian RFC 4122
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `Date.now()` dalam milidetik.
fn milidetik_sekarang() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// `new Date().toISOString()` — `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn iso_dari_ms(ms: u128) -> String {
    let detik = (ms / 1000) as i64;
    let milidetik = (ms % 1000) as u32;
    let hari = detik.div_euclid(86_400);
    let dalam_hari = detik.rem_euclid(86_400);
    let (tahun, bulan, tanggal) = civil_dari_hari(hari);
    format!(
        "{tahun:04}-{bulan:02}-{tanggal:02}T{:02}:{:02}:{:02}.{milidetik:03}Z",
        dalam_hari / 3600,
        (dalam_hari % 3600) / 60,
        dalam_hari % 60,
    )
}

/// Tanggal kalender dari jumlah hari sejak 1970-01-01 (algoritma Howard
/// Hinnant). Tidak ada crate tanggal di `Cargo.toml`.
fn civil_dari_hari(hari: i64) -> (i64, u32, u32) {
    let z = hari + 719_468; // geser ke epoch 0000-03-01
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let tahun = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tanggal = doy - (153 * mp + 2) / 5 + 1;
    let bulan = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if bulan <= 2 { tahun + 1 } else { tahun },
        bulan as u32,
        tanggal as u32,
    )
}

/// `buildCookie(deviceId, chatId, cookies = {})` (`unlimitedai.js:26-35`).
///
/// Selalu dipanggil tanpa `cookies` (`unlimitedai.js:107`), jadi cookie hanya
/// tiga bagian: `NEXT_LOCALE`, `u_device_id`, `home_chat_id`.
fn build_cookie(device_id: &str, chat_id: &str) -> String {
    [
        ("NEXT_LOCALE", "id"),
        ("u_device_id", device_id),
        ("home_chat_id", chat_id),
    ]
    .iter()
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>()
    .join("; ")
}

/// `parseSetCookie(headers)` (`unlimitedai.js:8-24`) — tidak pernah dipanggil
/// di JS; ditulis supaya port utuh. Ambil tiap `set-cookie`, pakai bagian
/// sebelum `;` pertama, lalu pisahkan di `=` pertama.
#[allow(dead_code)]
fn parse_set_cookie(headers: &[(String, String)]) -> Value {
    let mut hasil = Map::new();
    for (nama, nilai) in headers {
        if !nama.eq_ignore_ascii_case("set-cookie") {
            continue;
        }
        let pertama = nilai.split(';').next().unwrap_or("");
        let Some((k, v)) = pertama.split_once('=') else {
            continue;
        };
        hasil.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
    }
    Value::Object(hasil)
}

/// `CHARACTERS` (`unlimitedai.js:37-58`) — diekspor JS sebagai nilai, jadi di
/// sini jadi fungsi yang mengembalikan objek dengan isi dan urutan sama.
pub fn characters() -> Value {
    let mut out = Map::new();
    for (kunci, nama, prompt) in KARAKTER {
        out.insert(
            kunci.to_string(),
            json!({ "name": nama, "prompt": prompt }),
        );
    }
    Value::Object(out)
}

/// `CHARACTERS[character] || CHARACTERS["ai-assistant"]` — `(nama, prompt)`.
fn karakter(nama: Option<&str>) -> (&'static str, &'static str) {
    for (kunci, nama_karakter, prompt) in KARAKTER {
        if Some(kunci) == nama {
            return (nama_karakter, prompt);
        }
    }
    (KARAKTER[0].1, KARAKTER[0].2)
}

/// `UnlimitedAI(prompt, character = "ai-assistant")` (`unlimitedai.js:60-161`).
pub fn unlimited_ai(
    http: &dyn HttpGet,
    prompt: &str,
    character: Option<&str>,
) -> Result<Value, HttpError> {
    let chat_id = uuid_v4();
    let device_id = uuid_v4();
    let (nama, prompt_karakter) = karakter(character);

    // Template literal `${char.prompt}\n\nPertanyaan user: ${prompt}`.
    let system_prompt = format!("{prompt_karakter}\n\nPertanyaan user: {prompt}");
    let created_at = iso_dari_ms(milidetik_sekarang());

    let messages = json!([
        {
            "id": uuid_v4(),
            "role": "user",
            "content": system_prompt,
            "parts": [{ "type": "text", "text": system_prompt }],
            "createdAt": created_at,
        },
        {
            "id": uuid_v4(),
            "role": "assistant",
            "content": "",
            "parts": [{ "type": "text", "text": "" }],
            "createdAt": created_at,
        },
    ]);

    let body = json!({
        "chatId": chat_id,
        "messages": messages,
        "selectedChatModel": MODEL,
        "selectedCharacter": Value::Null,
        "selectedStory": Value::Null,
        "deviceId": device_id,
        "locale": "id",
    });

    let res = http.call(
        &HttpCall::post(API)
            .header("sec-ch-ua-platform", "\"Android\"")
            .header("user-agent", UA)
            .header(
                "sec-ch-ua",
                "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"",
            )
            .header("content-type", "application/json")
            .header("sec-ch-ua-mobile", "?1")
            .header("x-next-intl-locale", "id")
            .header("accept", "*/*")
            .header("origin", "https://app.unlimitedai.chat")
            .header("referer", "https://app.unlimitedai.chat/id")
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("cookie", build_cookie(&device_id, &chat_id))
            .header("priority", "u=1, i")
            .json(&body)
            // `fetch` tidak melempar untuk 4xx/5xx — status dibaca sendiri.
            .terima_semua_status(),
    )?;

    if !(200..300).contains(&res.status) {
        return Ok(json!({
            "status": false,
            "code": res.status,
            "error": res.text(),
        }));
    }

    // `buffer += …; const lines = buffer.split("\n"); buffer = lines.pop() || ""`
    // — potongan terakhir tidak pernah diproses karena loop berhenti duluan.
    let teks = res.text();
    let baris: Vec<&str> = teks.split('\n').collect();
    let mut answer = String::new();
    for baris in &baris[..baris.len().saturating_sub(1)] {
        let baris = baris.trim();
        if baris.is_empty() {
            continue;
        }
        // `try { JSON.parse(line) } catch {}` — baris rusak dilewati.
        let Ok(v) = serde_json::from_str::<Value>(baris) else {
            continue;
        };
        if v.get("type").and_then(Value::as_str) == Some("delta")
            && let Some(delta) = v.get("delta").and_then(Value::as_str)
        {
            answer.push_str(delta);
        }
    }

    Ok(json!({
        "status": true,
        "code": res.status,
        "character": nama,
        "model": MODEL,
        "answer": answer,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas status +
    /// body yang ditentukan test.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(
            &self,
            req: &oktz_plugin_api::HttpCall,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, teks) = balasan[i].clone();
            Ok(oktz_plugin_api::HttpResponse {
                status,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(status: u16, body: &str) -> StubHttp {
        StubHttp::baru(vec![(status, body.to_string())])
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn body_json(p: &oktz_plugin_api::HttpCall) -> Value {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Bytes { data, .. }) => {
                serde_json::from_slice(data).expect("body JSON")
            }
            lain => panic!("body bukan JSON: {lain:?}"),
        }
    }

    #[test]
    fn iso_dari_ms_sama_dengan_toisostring_node() {
        // `new Date(ms).toISOString()`.
        assert_eq!(iso_dari_ms(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_dari_ms(1_000), "1970-01-01T00:00:01.000Z");
        assert_eq!(iso_dari_ms(1_765_962_059_039), "2025-12-17T09:00:59.039Z");
        // Tahun kabisat dan pergantian hari.
        assert_eq!(iso_dari_ms(1_709_164_800_123), "2024-02-29T00:00:00.123Z");
        assert_eq!(iso_dari_ms(951_782_400_000), "2000-02-29T00:00:00.000Z");
        assert_eq!(iso_dari_ms(4_102_444_800_000), "2100-01-01T00:00:00.000Z");
    }

    #[test]
    fn characters_sama_dengan_objek_js_dan_urutannya() {
        let c = characters();
        let kunci: Vec<&str> = c
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        // `Object.keys(CHARACTERS)` — urutan juga bagian dari kontrak.
        assert_eq!(
            kunci,
            ["ai-assistant", "kobo-ai", "waguri-ai", "jokowi-ai", "prabowo-ai"]
        );
        assert_eq!(c["ai-assistant"]["name"], "AI Assistant");
        assert_eq!(c["kobo-ai"]["name"], "Kobo Kanaeru");
        assert_eq!(c["waguri-ai"]["name"], "Waguri");
        assert_eq!(c["jokowi-ai"]["name"], "Pak Jokowi");
        assert_eq!(c["prabowo-ai"]["name"], "Pak Prabowo");
        assert!(c["kobo-ai"]["prompt"]
            .as_str()
            .unwrap()
            .contains("\"DAJOOR!\""));
        assert!(c["prabowo-ai"]["prompt"]
            .as_str()
            .unwrap()
            .ends_with("jangan kaku."));
        assert!(c.get("tidak-ada").is_none(), "kunci asing tidak ikut");
    }

    #[test]
    fn delta_ndjson_dirakit_dan_baris_terakhir_tidak_dipakai() {
        let http = stub(
            200,
            "{\"type\":\"delta\",\"delta\":\"Halo \"}\n\
             {\"type\":\"delta\",\"delta\":\"dunia\"}\n\
             {\"type\":\"done\"}\n\
             {\"type\":\"delta\",\"delta\":\"tidak ikut\"}",
        );
        let hasil = unlimited_ai(&http, "hai", None).unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["character"], "AI Assistant");
        assert_eq!(hasil["model"], "chat-model-reasoning");
        // Potongan terakhir tidak diakhiri `\n` → dibuang `lines.pop()`.
        assert_eq!(hasil["answer"], "Halo dunia");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://app.unlimitedai.chat/api/chat");
        assert!(p[0].terima_semua_status, "fetch tidak melempar untuk 4xx");
        assert_eq!(
            cari_header(&p[0], "origin").as_deref(),
            Some("https://app.unlimitedai.chat")
        );
        assert_eq!(
            cari_header(&p[0], "referer").as_deref(),
            Some("https://app.unlimitedai.chat/id")
        );
        assert_eq!(cari_header(&p[0], "x-next-intl-locale").as_deref(), Some("id"));
        assert_eq!(
            cari_header(&p[0], "content-type").as_deref(),
            Some("application/json")
        );
        assert_eq!(cari_header(&p[0], "priority").as_deref(), Some("u=1, i"));

        let cookie = cari_header(&p[0], "cookie").expect("cookie wajib ada");
        let bagian: Vec<&str> = cookie.split("; ").collect();
        assert_eq!(bagian.len(), 3, "cookie: {cookie}");
        assert_eq!(bagian[0], "NEXT_LOCALE=id");
        let device_id = bagian[1].strip_prefix("u_device_id=").unwrap().to_string();
        let chat_id = bagian[2].strip_prefix("home_chat_id=").unwrap().to_string();
        assert_eq!(device_id.len(), 36, "UUID di cookie");

        let body = body_json(&p[0]);
        assert_eq!(body["chatId"], Value::String(chat_id));
        assert_eq!(body["deviceId"], Value::String(device_id));
        assert_eq!(body["selectedChatModel"], "chat-model-reasoning");
        assert_eq!(body["locale"], "id");
        assert_eq!(body["selectedCharacter"], Value::Null);
        assert_eq!(body["selectedStory"], Value::Null);

        let pesan = body["messages"].as_array().unwrap();
        assert_eq!(pesan.len(), 2);
        assert_eq!(pesan[0]["role"], "user");
        assert_eq!(pesan[1]["role"], "assistant");
        assert_eq!(pesan[1]["content"], "");
        // Prompt karakter + pertanyaan user, persis seperti template literal.
        assert!(
            pesan[0]["content"]
                .as_str()
                .unwrap()
                .ends_with("\n\nPertanyaan user: hai"),
            "{}",
            pesan[0]["content"]
        );
        assert_eq!(pesan[0]["parts"][0]["text"], pesan[0]["content"]);
        assert_eq!(pesan[0]["id"].as_str().unwrap().len(), 36);
        assert_eq!(pesan[0]["createdAt"], pesan[1]["createdAt"]);
        // `createdAt` = `toISOString()`.
        let created = pesan[0]["createdAt"].as_str().unwrap();
        assert!(
            created.ends_with('Z') && created.contains('T') && created.len() == 24,
            "createdAt: {created}"
        );
    }

    #[test]
    fn karakter_tak_dikenal_jatuh_ke_ai_assistant() {
        let http = stub(200, "{\"type\":\"delta\",\"delta\":\"x\"}\n");
        let hasil = unlimited_ai(&http, "hai", Some("kobo-ai")).unwrap();
        assert_eq!(hasil["character"], "Kobo Kanaeru");
        let p = http.permintaan.lock().unwrap();
        let prompt = body_json(&p[0])["messages"][0]["content"].as_str().unwrap().to_string();
        assert!(prompt.starts_with("Kamu adalah Kobo Kanaeru"), "{prompt}");

        let lain = stub(200, "{\"type\":\"delta\",\"delta\":\"x\"}\n");
        let hasil = unlimited_ai(&lain, "hai", Some("tidak-ada")).unwrap();
        assert_eq!(hasil["character"], "AI Assistant");
    }

    #[test]
    fn baris_rusak_dan_delta_bukan_string_dilewati() {
        let http = stub(
            200,
            "bukan json\n\
             {\"type\":\"delta\",\"delta\":5}\n\
             {\"type\":\"lain\",\"delta\":\"x\"}\n\
             {\"type\":\"delta\"}\n\
             \n\
             {\"type\":\"delta\",\"delta\":\"akhir\"}\n",
        );
        let hasil = unlimited_ai(&http, "hai", None).unwrap();
        assert_eq!(hasil["answer"], "akhir");
    }

    #[test]
    fn status_gagal_memberi_objek_error_dengan_teks_body() {
        let http = stub(500, "server tumbang");
        let hasil = unlimited_ai(&http, "hai", None).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["code"], 500);
        assert_eq!(hasil["error"], "server tumbang");
        assert!(hasil.get("answer").is_none(), "tidak ada kunci answer");
    }

    #[test]
    fn parse_set_cookie_mengambil_nilai_sebelum_titik_koma() {
        let headers = vec![
            ("set-cookie".to_string(), "a=1; Path=/; HttpOnly".to_string()),
            ("Set-Cookie".to_string(), " b = 2 ; Secure".to_string()),
            ("set-cookie".to_string(), "tanpa-tanda-sama".to_string()),
            ("content-type".to_string(), "application/json".to_string()),
        ];
        let hasil = parse_set_cookie(&headers);
        assert_eq!(hasil["a"], "1");
        assert_eq!(hasil["b"], "2");
        assert!(hasil.get("tanpa-tanda-sama").is_none(), "tanpa `=` dilewati");
        assert_eq!(parse_set_cookie(&[]), json!({}));
    }
}