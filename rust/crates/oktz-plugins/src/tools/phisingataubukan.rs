//! `plugins/tools/phisingataubukan.js` — cek apakah sebuah URL phising/berbahaya.
//!
//! Satu-satunya endpoint: `GET https://api.nexray.eu.cc/tools/webphishing?url=…`
//! (tanpa header khusus, tanpa key — `docs/rust-rewrite/ENDPOINTS.md` §2).
//!
//! `ponytail:` dua hal tidak bisa di-cerminkan persis:
//! - `m.pushName` tidak ada di `Ctx`, jadi cabang "halo" memakai fallback
//!   `'User'` yang sama seperti `te()` di `src/lib/error.js`.
//! - JS bereaksi dua kali pada satu pesan (`m.react("🕕")` di awal, lalu
//!   `✅`/`❌` di akhir). `Reply::with_reaction` hanya membawa satu reaksi,
//!   jadi reaksi penutup yang dikirim dan `🕕` hilang — sama seperti batas
//!   "satu balasan" yang sudah dicatat di HANDOFF §3.5 butir 6.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::jakarta_parts;

pub(crate) const ENDPOINT: &str = "https://api.nexray.eu.cc/tools/webphishing";

/// `m.pushName` tidak ada di `Ctx`; `src/lib/error.js` memakai `'User'` untuk
/// nilai kosong, jadi nama yang sama dipakai di sini.
const NO_NAME: &str = "User";

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "Halo *{NO_NAME}* 👋\n\n\
         Silakan masukkan link URL yang ingin dicek keamanannya.\n\n\
         Contoh:\n- `{prefix}phisingataubukan https://google.com`"
    )
}

pub(crate) const SCAN_FAILED: &str =
    "⚠️ Gagal mengecek URL. Pastikan link yang dimasukkan valid atau coba lagi nanti.";

pub(crate) const SYSTEM_ERROR: &str =
    "😔 Terjadi kesalahan sistem saat mengecek URL tersebut. Mohon coba lagi nanti.";

/// `result.is_phishing ? '🚨 Ya' : '✅ Tidak'` — JS pakai ternary, bukan
/// `&&`/`||`, jadi nilai falsy apa pun (termasuk `undefined`) tetap "Tidak".
fn flag(value: Option<&Value>) -> &'static str {
    if crate::common::js_truthy(value) {
        "🚨 Ya"
    } else {
        "✅ Tidak"
    }
}

fn field<'a>(result: &'a Value, key: &str) -> Option<&'a Value> {
    result.get(key)
}

/// `${value}` di dalam template literal JS: objek/array jadi JSON, `null` dan
/// `undefined` jadi string kosong.
fn stringify(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

pub(crate) fn render(result: &Value) -> String {
    let mut info = String::from("🛡️ *HASIL SCAN URL* 🛡️\n\n");
    info.push_str(&format!(
        "🔗 *URL:* {}\n",
        stringify(field(result, "scanned_url"))
    ));
    info.push_str(&format!(
        "📊 *Status:* {}\n\n",
        stringify(field(result, "status_description"))
    ));
    info.push_str("*DETAIL SCAN:*\n");
    for (label, key) in [
        ("Phising", "is_phishing"),
        ("Mengandung Malware", "contains_malware"),
        (
            "Membawa ke Situs Berbahaya",
            "sends_to_harmful_sites",
        ),
        (
            "Menginstal Software Jahat",
            "installs_malicious_software",
        ),
        ("Unduhan Tidak Wajar", "uncommon_downloads"),
    ] {
        info.push_str(&format!("- {label}: {}\n", flag(field(result, key))));
    }
    info.push_str(&format!(
        "\n_Terakhir dipindai: {}_",
        to_locale_string(field(result, "last_modified"))
    ));
    info
}

/// `encodeURIComponent` — BUKAN `URLSearchParams`: spasi jadi `%20` (bukan
/// `+`), dan `!'()*~` tetap apa adanya.
pub(crate) fn encode_uri_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*'
            | b'\'' | b'(' | b')' => out.push(*b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `new Date(x).toLocaleString("id-ID")`.
///
/// `ponytail:` hasil `toLocaleString` bergantung zona waktu runtime Node.
/// Port ini mengunci `Asia/Jakarta` lewat `crate::common::jakarta_parts`.
/// Kalau bot dijalankan di zona lain, ini ikut bergeser.
///
/// Nilai yang tidak bisa dibaca jadi `"Invalid Date"`, sama seperti V8.
///
/// `ponytail:` V8 juga mengoercive array jadi `Date` dengan aturan aneh
/// (`new Date([1,2])` -> 2001-02-01). Array dan object tidak direproduksi
/// karena tidak mungkin keluar dari API yang dipanggil plugin ini; keduanya
/// menghasilkan `"Invalid Date"`.
pub(crate) fn to_locale_string(value: Option<&Value>) -> String {
    let ms = match value {
        Some(Value::Number(n)) => n.as_f64().filter(|f| f.is_finite()).map(|f| f as i64),
        Some(Value::String(s)) => parse_date_ms(s),
        // `new Date(null)` dan `new Date(true)` bukan `Invalid Date` di V8:
        // keduanya lebih dulu masuk `ToNumber` (0 dan 1).
        Some(Value::Null) => Some(0),
        Some(Value::Bool(b)) => Some(i64::from(*b)),
        _ => None,
    };
    match ms {
        Some(ms) => format_id_datetime(ms),
        None => INVALID_DATE.to_string(),
    }
}

const INVALID_DATE: &str = "Invalid Date";

/// `dd/MM/yyyy HH.mm.ss` — bentuk `id-ID` di Node.
fn format_id_datetime(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = jakarta_parts(ms);
    format!("{d:02}/{mo:02}/{y} {:02}.{:02}.{:02}", h, mi, s)
}

/// Pecah offset `Z` / `+07:00` / `-0500` dari badan ISO.
///
/// `None` = tidak ada offset eksplisit, dan itu BEDA dari `Some(0)`: tanpa
/// offset, `Date.parse` membaca jam sebagai waktu lokal mesin (di sini
/// Asia/Jakarta), sedangkan `Z` berarti UTC.
fn split_offset(s: &str) -> (&str, Option<i64>) {
    if let Some(head) = s.strip_suffix('Z').or_else(|| s.strip_suffix('z')) {
        return (head, Some(0));
    }
    let bytes = s.as_bytes();
    // Mulai dari 11 supaya tanda `-` di `YYYY-MM-DD` tidak dianggap offset.
    for i in (11..bytes.len()).rev() {
        if bytes[i] != b'+' && bytes[i] != b'-' {
            continue;
        }
        let digits: String = s[i + 1..].chars().filter(char::is_ascii_digit).collect();
        let sign = if bytes[i] == b'+' { 1 } else { -1 };
        if digits.len() >= 3 {
            let h: i64 = digits[..2].parse().unwrap_or(0);
            let m: i64 = digits[2..4].parse().unwrap_or(0);
            return (&s[..i], Some(sign * (h * 3600 + m * 60)));
        }
    }
    (s, None)
}

fn frac_millis(frac: &str) -> Option<i64> {
    let mut f: String = frac.chars().take(3).collect();
    while f.len() < 3 {
        f.push('0');
    }
    f.parse().ok()
}

/// `Date.parse` untuk bentuk ISO-8601 — satu-satunya bentuk yang dipakai API
/// ini.
///
/// Tiga aturan ES2015 yang sering dilanggar dan di sini dijaga lewat test:
///
/// - ada offset (`Z`, `+07:00`, `+0700`) -> jam dibaca sebagai UTC lalu
///   digeser ke zona render;
/// - tanpa offset -> jam dibaca sebagai waktu lokal (Asia/Jakarta);
/// - tanggal polos `YYYY-MM-DD` -> **UTC** tengah malam, bukan lokal. V8
///   begitu sejak ES2015; test `date_only_is_utc_midnight` mengunci itu.
///
/// Hari 1-31 saja yang diterima; `2026-02-30` bukan-invalid-date di V8 tapi
/// melorot ke Maret, dan [`days_from_civil`] mereproduksinya karena aritmetikanya
/// cuma penjumlahan. Jam 24 diterima (melorot ke hari berikutnya), menit 60 dan
/// detik 60 tidak.
fn parse_date_ms(raw: &str) -> Option<i64> {
    let (body, offset) = split_offset(raw.trim());
    let (date_part, time_part) = match body.find(['T', 't', ' ']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };

    let mut d = date_part.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let month: u32 = d.next()?.parse().ok()?;
    let day: u32 = d.next()?.parse().ok()?;
    if d.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = crate::common::days_from_civil(y, month, day);

    let Some(t) = time_part else {
        return Some(days * 86_400_000);
    };
    if t.contains(',') {
        // V8 hanya menerima titik untuk pecahan detik.
        return None;
    }

    let mut p = t.split(':');
    let h: u32 = p.next()?.parse().ok()?;
    let mi: u32 = p.next()?.parse().ok()?;
    let (sec, milli) = match p.next() {
        None => (0u32, 0i64),
        Some(secstr) => match secstr.split_once('.') {
            Some((sec, frac)) => (sec.parse().ok()?, frac_millis(frac)?),
            None => (secstr.parse().ok()?, 0),
        },
    };
    if p.next().is_some() || h > 24 || mi > 59 || sec > 59 {
        return None;
    }

    let naive = days * 86_400 + i64::from(h * 3600 + mi * 60 + sec);
    let geser = match offset {
        Some(secs) => secs,
        None => crate::common::JAKARTA_OFFSET_SECS,
    };
    Some((naive - geser) * 1000 + milli)
}

#[plugin(
    name = "phisingataubukan",
    alias = ["cekphising", "checkphishing", "webphishing"],
    category = "tools",
    description = "Cek apakah sebuah link URL merupakan web phising/berbahaya atau aman.",
    usage = ".phisingataubukan <url>",
    example = ".phisingataubukan https://google.com",
    cooldown = 10,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn phisingataubukan(ctx: Ctx) -> Reply {
    let url = ctx.text.trim().to_string();
    if url.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }

    let api_url = format!("{ENDPOINT}?url={}", encode_uri_component(&url));
    let data = match ctx.http.get(&api_url).ok().and_then(|r| r.json()) {
        Some(v) => v,
        None => return Reply::text(SYSTEM_ERROR).with_reaction("❌"),
    };

    // `!data.status || !data.result` — `result` boleh objek kosong: objek itu
    // truthy di JS, jadi tidak boleh disamakan dengan "tidak ada".
    if !crate::common::js_truthy(data.get("status")) || !crate::common::js_truthy(data.get("result")) {
        return Reply::text(SCAN_FAILED).with_reaction("❌");
    }

    let result = data.get("result").cloned().unwrap_or(Value::Null);
    Reply::text(render(&result)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// HTTP offline: Balasan sudah disiapkan, atau gagal total.
    #[derive(Debug)]
    struct Stub {
        result: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                result: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                result: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .result
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
            unreachable!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("plugin ini hanya GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "phisingataubukan".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("phisingataubukan").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    fn reaction(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.clone()),
            _ => None,
        }
    }

    fn scan() -> Value {
        json!({
            "scanned_url": "https://google.com",
            "status_description": "Aman",
            "is_phishing": false,
            "contains_malware": false,
            "sends_to_harmful_sites": false,
            "installs_malicious_software": false,
            "uncommon_downloads": false,
            "last_modified": "2026-10-14T03:30:45Z",
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("phisingataubukan").unwrap();
        assert_eq!(p.meta.name, "phisingataubukan");
        assert_eq!(p.meta.alias, ["cekphising", "checkphishing", "webphishing"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(
            p.meta.description,
            "Cek apakah sebuah link URL merupakan web phising/berbahaya atau aman."
        );
        assert_eq!(p.meta.usage, ".phisingataubukan <url>");
        assert_eq!(p.meta.example, ".phisingataubukan https://google.com");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        // Tidak ada di pluginConfig JS -> false, bukan default `true` makro.
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["cekphising", "checkphishing", "webphishing"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("phisingataubukan"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn an_empty_or_blank_url_shows_the_usage_line_without_any_request() {
        let http = Stub::body(scan());
        for text in ["", "   ", "\n\t "] {
            let r = run(http.clone(), text);
            assert_eq!(body(&r), usage("."), "input {text:?}");
            assert!(body(&r).contains("`.phisingataubukan https://google.com`"));
            assert_eq!(reaction(&r), None, "tanpa reaksi di cabang ini");
        }
        assert!(http.seen.lock().unwrap().is_empty(), "tidak boleh ada request");
    }

    #[test]
    fn a_clean_result_lists_every_five_flags_as_tidak() {
        let http = Stub::body(json!({ "status": true, "result": scan() }));
        let r = run(http.clone(), "https://google.com");
        let t = body(&r);
        assert!(t.starts_with("🛡️ *HASIL SCAN URL* 🛡️"), "{t}");
        assert!(t.contains("🔗 *URL:* https://google.com"), "{t}");
        assert!(t.contains("📊 *Status:* Aman"), "{t}");
        for label in [
            "- Phising:",
            "- Mengandung Malware:",
            "- Membawa ke Situs Berbahaya:",
            "- Menginstal Software Jahat:",
            "- Unduhan Tidak Wajar:",
        ] {
            assert!(t.contains(&format!("{label} ✅ Tidak")), "{t}");
        }
        assert_eq!(t.matches("✅ Tidak").count(), 5, "{t}");
        assert_eq!(reaction(&r), Some("✅".to_string()));
    }

    #[test]
    fn a_single_true_flag_flips_only_that_row() {
        let http = Stub::body(json!({ "status": true, "result": {
            "scanned_url": "https://evil.test",
            "status_description": "Berbahaya",
            "is_phishing": true,
            "contains_malware": false,
            "sends_to_harmful_sites": true,
            "installs_malicious_software": false,
            "uncommon_downloads": false,
            "last_modified": "2026-10-14T03:30:45Z",
        } }));
        let __balasan = run(http.clone(), "https://evil.test");
        let t = body(&__balasan);
        assert!(t.contains("- Phising: 🚨 Ya"), "{t}");
        assert!(t.contains("- Membawa ke Situs Berbahaya: 🚨 Ya"), "{t}");
        assert!(t.contains("- Mengandung Malware: ✅ Tidak"), "{t}");
        assert!(t.contains("- Unduhan Tidak Wajar: ✅ Tidak"), "{t}");
        assert_eq!(t.matches("🚨 Ya").count(), 2, "{t}");
    }

    #[test]
    fn missing_or_falsy_status_and_result_both_report_the_scan_failure() {
        // `!data.status || !data.result` — objek kosong tetap truthy di JS,
        // jadi `result: {}` harus tetap jalan.
        let cases = [
            json!({ "status": false, "result": scan() }),
            json!({ "status": true, "result": null }),
            json!({ "result": scan() }),
            json!({ "status": true }),
        ];
        for body_json in cases {
            let http = Stub::body(body_json.clone());
            let r = run(http.clone(), "https://google.com");
            assert_eq!(body(&r), SCAN_FAILED, "body {body_json}");
            assert_eq!(reaction(&r), Some("❌".to_string()), "body {body_json}");
        }

        let http = Stub::body(json!({ "status": true, "result": {} }));
        assert!(
            !body(&run(http.clone(), "x")).is_empty(),
            "result kosong = objek truthy, bukan kegagalan scan"
        );
    }

    #[test]
    fn a_transport_failure_reports_the_system_error_with_a_cross_emoji() {
        let r = run(Stub::offline(), "https://google.com");
        assert_eq!(body(&r), SYSTEM_ERROR);
        assert_eq!(reaction(&r), Some("❌".to_string()));

        // `Ctx::default()` memakai NoHttp — jalur gagal tanpa jaringan.
        let ctx = Ctx {
            prefix: ".".into(),
            command: "phisingataubukan".into(),
            text: "https://google.com".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("phisingataubukan").unwrap().handler)(ctx);
        assert_eq!(body(&r), SYSTEM_ERROR);
    }

    #[test]
    fn the_scanned_url_goes_through_encode_uri_component() {
        let http = Stub::body(json!({ "status": true, "result": scan() }));
        run(http.clone(), "https://a.test/?x=1 2");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0],
            format!("{ENDPOINT}?url=https%3A%2F%2Fa.test%2F%3Fx%3D1%202")
        );
    }

    #[test]
    fn encode_uri_component_differs_from_url_search_params_on_space() {
        assert_eq!(encode_uri_component("a b"), "a%20b", "encodeURIComponent, bukan '+'");
        assert_eq!(encode_uri_component("a+b"), "a%2Bb", "'+' ikut di-encode");
        assert_eq!(encode_uri_component("küfte"), "k%C3%BCfte");
        assert_eq!(encode_uri_component("-_.!~*'()"), "-_.!~*'()", "unreserved tak di-encode");
    }

    // Semua nilai di bawah diambil dari `TZ=Asia/Jakarta node -e ...`, bukan
    // dari tebakan: expected string = output `toLocaleString("id-ID")` V8.

    #[test]
    fn last_modified_is_rendered_the_way_id_id_does_it() {
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T03:30:45Z"))),
            "14/10/2026 10.30.45",
            "Z = UTC, digeser ke WIB"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T03:30:45.250+00:00"))),
            "14/10/2026 10.30.45"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T03:30:45+07:00"))),
            "14/10/2026 03.30.45",
            "offset +07:00 = sudah WIB, jangan digeser lagi"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T03:30:45+0700"))),
            "14/10/2026 03.30.45"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T10:30:45"))),
            "14/10/2026 10.30.45",
            "tanpa offset dibaca sebagai Asia/Jakarta"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14 03:30:45"))),
            "14/10/2026 03.30.45",
            "spasi juga pemisah yang V8 terima"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T03:30:45.123456Z"))),
            "14/10/2026 10.30.45",
            "pecahan dipotong, bukan dibulatkan"
        );
    }

    #[test]
    fn date_only_is_utc_midnight_not_local_midnight() {
        // ES2015: `YYYY-MM-DD` = UTC. V8 di WIB merender 07.00.00, bukan
        // 00.00.00 — test ini existed menjaga supaya tidak "diperbaiki" nanti.
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14"))),
            "14/10/2026 07.00.00"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-1-1"))),
            "01/01/2026 07.00.00",
            "bulan/hari satu digit tetap diterima"
        );
    }

    #[test]
    fn out_of_range_values_roll_over_exactly_like_v8() {
        assert_eq!(
            to_locale_string(Some(&json!("2026-02-30"))),
            "02/03/2026 07.00.00",
            "hari excessive melorot, bukan Invalid Date"
        );
        assert_eq!(
            to_locale_string(Some(&json!("2026-10-14T24:00:00"))),
            "15/10/2026 00.00.00",
            "jam 24 = tengah malam hari berikutnya"
        );
    }

    #[test]
    fn an_unreadable_date_becomes_invalid_date_never_a_panic() {
        for bad in [
            json!("bukan tanggal"),
            json!(""),
            json!("2026-13-45"),
            json!("2026-00-10"),
            json!("2026-10-00"),
            json!("2026-10-32"),
            json!("2026-10-14T99:99"),
            json!("2026-10-14T23:60:00"),
            json!("2026-10-14T10:30:60"),
            json!("2026-10-14T03:30:45,250Z"),
            json!("2026-10-14T03:30:45-05"),
            json!("2026-10-14T03"),
            json!([1, 2]),
            json!({ "a": 1 }),
        ] {
            assert_eq!(
                to_locale_string(Some(&bad)),
                INVALID_DATE,
                "nilai {bad} tidak boleh bikin panic"
            );
        }
        assert_eq!(to_locale_string(None), INVALID_DATE);
    }

    #[test]
    fn null_and_boolean_go_through_to_number_like_v8() {
        // `new Date(null)` -> epoch 0, bukan Invalid Date. Kalau plugin ini
        // pernah menerima `last_modified: null` dari API, teks yang tampil di
        // produksi adalah "01/01/1970 07.00.00".
        assert_eq!(to_locale_string(Some(&json!(null))), "01/01/1970 07.00.00");
        assert_eq!(to_locale_string(Some(&json!(false))), "01/01/1970 07.00.00");
        assert_eq!(
            to_locale_string(Some(&json!(true))),
            "01/01/1970 07.00.00",
            "1 ms tidak muncul di toLocaleString"
        );
    }

    #[test]
    fn a_missing_result_field_renders_as_an_empty_gap_not_none() {
        let t = render(&json!({}));
        assert!(t.contains("🔗 *URL:* \n"), "{t}");
        assert!(t.contains("📊 *Status:* \n"), "{t}");
        assert!(t.contains("_Terakhir dipindai: Invalid Date_"), "{t}");
        assert_eq!(t.matches("✅ Tidak").count(), 5, "{t}");
    }
}