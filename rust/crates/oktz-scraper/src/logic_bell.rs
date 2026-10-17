//! `src/scraper/logic-bell.js` — POST ke `api.termai.cc/api/chat/logic-bell`.
//!
//! Config TIDAK dibaca dari berkas atau env di Rust, jadi tiga nilai dari
//! `config.js` jadi argumen: `api_key` (`config.APIkey.logicBell`),
//! `bot_name` (`config.bot.name`), dan `owner_name` (`config.owner.name`).
//! Plugin yang memanggil harus mengisinya; nilai kosong berarti "tidak diisi",
//! sama seperti `config` yang belum dikonfigurasi.
//!
//! Yang ikut disalin dari JS:
//! - `!message?.trim() && !imageBuffer` → `throw new Error("Message is
//!   required.")`; buffer kosong (`Some(&[])`) tetap dianggap ada karena objek
//!   di JS selalu truthy.
//! - `custom_profile: ... || undefined` → kunci tidak ikut `JSON.stringify`
//!   kalau `instruction` kosong.
//! - `image` disisipkan setelah objek payload, jadi urutannya paling akhir.
//! - `energy: data?.data?.energy || null` — falsy jadi `null`, bukan hilang.
//! - `throw new Error(data?.msg || data?.message || "Logic Bell request failed.")`.
//! - `date: new Date().toISOString()` dihitung di sini (tidak ada dependency
//!   tanggal di crate ini), dan `timeout: 60000` axios tidak ditiru — batas
//!   mesin, bukan tampilan.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, truthy};

const API: &str = "https://api.termai.cc/api/chat/logic-bell";

/// `chat({ message, instruction, imageBuffer, ... })` (`logic-bell.js:48-113`).
///
/// `options` berisi seluruh field dari objek JS kecuali `imageBuffer` (byte,
/// tidak bisa jadi JSON) dan tiga nilai config yang jadi argumen terpisah.
pub fn chat(
    http: &dyn HttpGet,
    options: &Value,
    image_buffer: Option<&[u8]>,
    api_key: &str,
    bot_name: &str,
    owner_name: &str,
) -> Result<Value, HttpError> {
    let ada_gambar = image_buffer.is_some();

    // Beda yang diketahui: `message` non-teks di JS membuat `message?.trim()`
    // melempar TypeError; di sini nilainya diubah ke teks lebih dulu.
    let pesan_ada = !string_dari(options.get("message")).trim().is_empty();
    if !pesan_ada && !ada_gambar {
        return Err(HttpError {
            message: "Message is required.".to_string(),
        });
    }
    if api_key.is_empty() {
        return Err(HttpError {
            message: "API key logicBell belum diisi.".to_string(),
        });
    }

    let (full_name, nick_name) = nama_ai(options.get("aiFullName"), bot_name);

    let mut payload = Map::new();
    payload.insert(
        "text".to_string(),
        json!(teks_pesan(
            options.get("message"),
            options.get("history"),
            ada_gambar
        )),
    );
    payload.insert(
        "id".to_string(),
        or_default(options.get("senderId"), "anonymous@s.whatsapp.net"),
    );
    payload.insert("fullainame".to_string(), json!(full_name));
    payload.insert(
        "nickainame".to_string(),
        json!(trim_atau(
            &string_dari(options.get("aiNickName")),
            &nick_name
        )),
    );
    payload.insert(
        "senderName".to_string(),
        json!(trim_atau(&string_dari(options.get("senderName")), "User")),
    );
    payload.insert(
        "ownerName".to_string(),
        json!(trim_atau(&string_dari_atau(owner_name), "Owner")),
    );
    payload.insert("date".to_string(), json!(iso8601_sekarang()));
    payload.insert(
        "role".to_string(),
        json!(trim_atau(
            &string_dari(options.get("role")),
            "Temen deket"
        )),
    );
    payload.insert(
        "msgtype".to_string(),
        json!(if ada_gambar { "image" } else { "text" }),
    );
    // `String(instruction || "").trim() || undefined` — kosong = kunci hilang.
    let custom_profile = string_dari(options.get("instruction")).trim().to_string();
    if !custom_profile.is_empty() {
        payload.insert("custom_profile".to_string(), json!(custom_profile));
    }
    payload.insert(
        "commands".to_string(),
        match options.get("commands") {
            Some(Value::Array(a)) => Value::Array(a.clone()),
            _ => json!([]),
        },
    );
    if let Some(gambar) = image_buffer {
        // Ditambahkan setelah payload, jadi urutannya paling akhir.
        payload.insert("image".to_string(), json!(base64_encode(gambar)));
    }

    let res = http.call(
        &HttpCall::post(format!("{API}?key={}", encode_uri_component(api_key)))
            .header("Content-Type", "application/json")
            .json(&Value::Object(payload)),
    )?;

    let data = res.json().unwrap_or(Value::Null);
    if !data.get("status").map(truthy).unwrap_or(false) {
        // `throw new Error(...)` — teks pesan apa adanya, plugin yang memformat.
        let pesan = data
            .get("msg")
            .filter(|v| truthy(v))
            .or_else(|| data.get("message").filter(|v| truthy(v)))
            .map(teks_js)
            .unwrap_or_else(|| "Logic Bell request failed.".to_string());
        return Err(HttpError { message: pesan });
    }

    // `String(data?.data?.msg || data?.msg || "").trim()`
    let teks_lolos = data
        .get("data")
        .and_then(|d| d.get("msg"))
        .filter(|v| truthy(v))
        .or_else(|| data.get("msg").filter(|v| truthy(v)))
        .map(teks_js)
        .unwrap_or_default()
        .trim()
        .to_string();

    let energy = data
        .get("data")
        .and_then(|d| d.get("energy"))
        .filter(|v| truthy(v))
        .cloned()
        .unwrap_or(Value::Null);

    Ok(json!({
        "text": teks_lolos,
        "raw": data,
        "model": "logic-bell",
        "energy": energy,
    }))
}

/// `getAiNames(aiFullName)` (`logic-bell.js:42-46`) dengan `config.bot.name`
/// sebagai nilai tengah.
fn nama_ai(ai_full_name: Option<&Value>, bot_name: &str) -> (String, String) {
    let penuh = string_dari(ai_full_name);
    let penuh = if !penuh.is_empty() {
        penuh
    } else if !bot_name.is_empty() {
        bot_name.to_string()
    } else {
        "AI Assistant".to_string()
    };
    let penuh = penuh.trim().to_string();
    // `fullName.split(/\s+/)[0] || "Foto"` — string kosong tidak punya kata.
    let nickname = penuh
        .split_whitespace()
        .next()
        .unwrap_or("Foto")
        .to_string();
    (penuh, nickname)
}

/// `buildTextMessage(message, history, imageBuffer)` (`logic-bell.js:25-40`).
fn teks_pesan(message: Option<&Value>, history: Option<&Value>, ada_gambar: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    let riwayat = teks_riwayat(history);
    if !riwayat.is_empty() {
        parts.push(riwayat);
    }
    if ada_gambar {
        parts.push(
            "User mengirim gambar. Jika gambar bisa dianalisis, gunakan konteks itu saat menjawab."
                .to_string(),
        );
    }
    let pesan = string_dari(message);
    let pesan = pesan.trim();
    let isi = if pesan.is_empty() {
        "(gambar tanpa teks)".to_string()
    } else {
        pesan.to_string()
    };
    parts.push(format!("Pesan user saat ini:\n{isi}"));
    // `parts.filter(Boolean)` — dua bagian pertama dijamin tidak kosong.
    parts.join("\n\n")
}

/// `buildHistoryText(history)` (`logic-bell.js:4-23`) — 10 pesan terakhir.
fn teks_riwayat(history: Option<&Value>) -> String {
    let Some(items) = history.and_then(Value::as_array) else {
        return String::new();
    };
    let mulai = items.len().saturating_sub(10);
    let mut lines: Vec<String> = Vec::new();
    for item in &items[mulai..] {
        let role = if item.get("role").and_then(Value::as_str) == Some("assistant") {
            "AI"
        } else {
            "User"
        };
        let content = string_dari(item.get("content")).trim().to_string();
        if !content.is_empty() {
            lines.push(format!("{role}: {content}"));
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    format!("Riwayat percakapan:\n{}", lines.join("\n"))
}

/// `a || bawaan` yang hasilnya tetap nilai JSON mentah.
fn or_default(v: Option<&Value>, bawaan: &str) -> Value {
    match v {
        Some(v) if truthy(v) => v.clone(),
        _ => json!(bawaan),
    }
}

/// `String(v || "")` — `""`, `0`, `false`, `null` jadi teks kosong.
fn string_dari(v: Option<&Value>) -> String {
    match v {
        Some(v) if truthy(v) => teks_js(v),
        _ => String::new(),
    }
}

/// `String(v || "")` untuk string Rust, mis. nilai config.
fn string_dari_atau(s: &str) -> String {
    if s.is_empty() { String::new() } else { s.to_string() }
}

/// `String(x || "").trim() || bawaan`.
fn trim_atau(s: &str, bawaan: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        bawaan.to_string()
    } else {
        t.to_string()
    }
}

/// `String(v)` untuk nilai JSON.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
        Value::Null => "null".to_string(),
    }
}

const ABJAD_B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `Buffer#toString("base64")` — padding `=`. Ditulis tangan karena crate ini
/// tidak punya dependency base64.
fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for blok in data.chunks(3) {
        let b0 = blok[0] as u32;
        let b1 = blok.get(1).copied().unwrap_or(0) as u32;
        let b2 = blok.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        let quartet = [
            ABJAD_B64[(n >> 18) as usize & 63],
            ABJAD_B64[(n >> 12) as usize & 63],
            ABJAD_B64[(n >> 6) as usize & 63],
            ABJAD_B64[n as usize & 63],
        ];
        out.push(quartet[0] as char);
        out.push(quartet[1] as char);
        if blok.len() > 1 {
            out.push(quartet[2] as char);
        } else {
            out.push('=');
        }
        if blok.len() > 2 {
            out.push(quartet[3] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// `new Date().toISOString()` — `2026-01-02T03:04:05.678Z`, selalu milidetik
/// tiga digit dan penanda `Z`.
fn iso8601_sekarang() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default();
    let detik = ms.div_euclid(1000);
    let milidetik = ms.rem_euclid(1000);
    let hari = detik.div_euclid(86_400);
    let sisa = detik.rem_euclid(86_400);
    let (jam, menit, dtk) = (sisa / 3600, (sisa % 3600) / 60, sisa % 60);
    let (tahun, bulan, tanggal) = kalender_dari_hari(hari);
    format!("{tahun:04}-{bulan:02}-{tanggal:02}T{jam:02}:{menit:02}:{dtk:02}.{milidetik:03}Z")
}

/// Hari sejak 1970-01-01 → (tahun, bulan, tanggal) kalender proleptik
/// Gregorian, sama seperti `Date` JS yang menghitung dalam UTC.
fn kalender_dari_hari(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468; // geser epoch ke awal bulan Maret, tempat hitungan dimulai
    let abad = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + abad * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas body tetap.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: String,
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
            unreachable!("scraper ini kirim JSON")
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
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: balasan.to_string(),
        }
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
    fn sukses_mengirim_payload_lengkap() {
        let http = stub(r#"{"status":true,"data":{"msg":"Halo","energy":7}}"#);
        let hasil = chat(
            &http,
            &json!({
                "message": "halo",
                "instruction": "  prophet",
                "senderId": "628@s.whatsapp.net",
                "senderName": " Budi ",
                "role": "Teman dekat",
                "commands": ["a", "b"]
            }),
            None,
            "kunci api",
            "Aria Bot",
            "Owner Name",
        )
        .unwrap();

        assert_eq!(hasil["text"], "Halo");
        assert_eq!(hasil["model"], "logic-bell");
        assert_eq!(hasil["energy"], 7);
        assert_eq!(hasil["raw"]["data"]["msg"], "Halo");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].url,
            "https://api.termai.cc/api/chat/logic-bell?key=kunci%20api"
        );
        let cari = p[0]
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());
        assert_eq!(cari.as_deref(), Some("application/json"));

        let body = body_json(&p[0]);
        assert_eq!(body["id"], "628@s.whatsapp.net");
        assert_eq!(body["fullainame"], "Aria Bot");
        assert_eq!(body["nickainame"], "Aria", "kata pertama dari fullName");
        assert_eq!(body["senderName"], "Budi", "selalu di-trim");
        assert_eq!(body["ownerName"], "Owner Name");
        assert_eq!(body["role"], "Teman dekat");
        assert_eq!(body["msgtype"], "text");
        assert_eq!(body["custom_profile"], "prophet");
        assert_eq!(body["commands"], json!(["a", "b"]));
        assert_eq!(body["text"], "Pesan user saat ini:\nhalo");
        assert!(
            body["date"].as_str().unwrap().ends_with('Z'),
            "date harus ISO dengan Z: {}",
            body["date"]
        );
        assert!(body.get("image").is_none(), "tanpa gambar tidak ada key image");
    }

    #[test]
    fn nilai_bawaan_dipakai_kala_option_kosong() {
        let http = stub(r#"{"status":true}"#);
        chat(&http, &json!({"message": "hi"}), None, "k", "", "").unwrap();
        let p = http.permintaan.lock().unwrap();
        let body = body_json(&p[0]);
        assert_eq!(body["id"], "anonymous@s.whatsapp.net");
        assert_eq!(body["fullainame"], "AI Assistant");
        assert_eq!(body["nickainame"], "AI", "kata pertama dari 'AI Assistant'");
        assert_eq!(body["senderName"], "User");
        assert_eq!(body["ownerName"], "Owner");
        assert_eq!(body["role"], "Temen deket");
        assert_eq!(body["commands"], json!([]));
        assert_eq!(body["text"], "Pesan user saat ini:\nhi");
        assert!(
            body.get("custom_profile").is_none(),
            "instruction kosong = undefined = key hilang"
        );
    }

    #[test]
    fn pesan_kosong_tanpa_gambar_melempar_sebelum_request() {
        let http = stub(r#"{"status":true}"#);
        let salah = chat(&http, &json!({"message": "   "}), None, "k", "", "").unwrap_err();
        assert_eq!(salah.message, "Message is required.");
        assert!(http.permintaan.lock().unwrap().is_empty());

        // `message` tidak ada sama sekali juga ditolak.
        let salah = chat(&http, &json!({}), None, "k", "", "").unwrap_err();
        assert_eq!(salah.message, "Message is required.");
    }

    #[test]
    fn kunci_kosong_melempar_tanpa_request() {
        let http = stub(r#"{"status":true}"#);
        let salah = chat(&http, &json!({"message": "hi"}), None, "", "", "").unwrap_err();
        assert_eq!(salah.message, "API key logicBell belum diisi.");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn status_salah_membawa_pesan_dari_msg_lalu_message_lalu_bawaan() {
        let http = stub(r#"{"status":false,"msg":"kuota habis"}"#);
        let salah = chat(&http, &json!({"message": "hi"}), None, "k", "", "").unwrap_err();
        assert_eq!(salah.message, "kuota habis");

        let http = stub(r#"{"message":"diminta tuyep"}"#);
        let salah = chat(&http, &json!({"message": "hi"}), None, "k", "", "").unwrap_err();
        assert_eq!(salah.message, "diminta tuyep");

        let http = stub(r#"{"status":false}"#);
        let salah = chat(&http, &json!({"message": "hi"}), None, "k", "", "").unwrap_err();
        assert_eq!(salah.message, "Logic Bell request failed.");
    }

    #[test]
    fn gambar_mengubah_msgtype_dan_menambah_base64() {
        let http = stub(r#"{"status":true,"data":{"msg":"ini kucing"}}"#);
        let hasil = chat(
            &http,
            &json!({"message": "", "aiFullName": "Zen Sensei"}),
            Some(b"PNG"),
            "k",
            "",
            "",
        )
        .unwrap();
        assert_eq!(hasil["text"], "ini kucing");
        assert_eq!(hasil["energy"], Value::Null, "energy hilang jadi null");

        let p = http.permintaan.lock().unwrap();
        let body = body_json(&p[0]);
        assert_eq!(body["msgtype"], "image");
        assert_eq!(body["image"], base64_encode(b"PNG"));
        assert_eq!(body["fullainame"], "Zen Sensei");
        assert_eq!(body["nickainame"], "Zen");
        // Tanpa teks pesan, isi pesan memakai placeholder gambar.
        assert_eq!(
            body["text"],
            "User mengirim gambar. Jika gambar bisa dianalisis, gunakan konteks itu saat menjawab.\n\n\
             Pesan user saat ini:\n(gambar tanpa teks)"
        );
    }

    #[test]
    fn riwayat_mengambil_sepuluh_pesan_terakhir_dan_menandai_peran() {
        let mut items: Vec<Value> = (0..12)
            .map(|i| json!({"role": "user", "content": format!("pesan {i}")}))
            .collect();
        items.push(json!({"role": "assistant", "content": "  jawaban AI  "}));
        items.push(json!({"role": "user", "content": "   "}));
        let http = stub(r#"{"status":true,"data":{"msg":"ok"}}"#);
        chat(
            &http,
            &json!({"message": "sekarang", "history": items}),
            None,
            "k",
            "",
            "",
        )
        .unwrap();
        let p = http.permintaan.lock().unwrap();
        let body = body_json(&p[0]);
        let text = body["text"].as_str().unwrap();
        // 10 terakhir dari 14 pesan: mulai dari "pesan 4"; yang kosong terbuang.
        assert!(text.starts_with("Riwayat percakapan:\nUser: pesan 4\n"), "{text}");
        // `parts.join("\n\n")` — riwayat dan pesan sekarang dipisah satu
        // baris kosong, bukan dua.
        assert!(text.contains("AI: jawaban AI\n\nPesan user saat ini:\nsekarang"), "{text}");
        assert!(!text.contains("pesan 3"), "{text}");
    }

    #[test]
    fn riwayat_bukan_array_menghasilkan_teks_tanpa_riwayat() {
        let http = stub(r#"{"status":true,"data":{"msg":"ok"}}"#);
        chat(
            &http,
            &json!({"message": "hi", "history": "bukan array"}),
            None,
            "k",
            "",
            "",
        )
        .unwrap();
        let p = http.permintaan.lock().unwrap();
        let body = body_json(&p[0]);
        assert_eq!(body["text"], "Pesan user saat ini:\nhi");
    }

    #[test]
    fn teks_dan_kalender_memenuhi_aturan_iso() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");

        // Tanggal yang pasti dari `Date` JS (dihitung ulang dengan
        // `new Date(hari * 86400000).toISOString()`).
        assert_eq!(kalender_dari_hari(0), (1970, 1, 1));
        assert_eq!(kalender_dari_hari(19_723), (2024, 1, 1));
        // Batas tahun: 20_454 = 2026-01-01, sehari sebelumnya 31 Desember.
        assert_eq!(kalender_dari_hari(20_453), (2025, 12, 31));
        assert_eq!(kalender_dari_hari(20_454), (2026, 1, 1));
        // 29 Februari — hari kabisat (2028 dibagi 4, bukan 100).
        assert_eq!(kalender_dari_hari(21_243), (2028, 2, 29));
        assert_eq!(kalender_dari_hari(21_244), (2028, 3, 1));

        let iso = iso8601_sekarang();
        assert_eq!(iso.len(), 24, "format YYYY-MM-DDTHH:MM:SS.mmmZ: {iso}");
        assert!(iso.ends_with('Z'), "{iso}");
    }
}