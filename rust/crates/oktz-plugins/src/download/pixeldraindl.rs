//! `plugins/download/pixeldraindl.js` — unduh file dari Pixeldrain.
//!
//! `ponytail:` `config.APIkey?.neoxr` tidak ada di `Ctx` (yang hanya membawa
//! state store), jadi `NEOXR_APIKEY` selalu kosong — sama dengan `|| ''` di JS
//! saat `.env` tidak diisi, dan query `apikey=` tetap dikirim apa adanya.
//!
//! `f()` (`src/lib/http.js:32`) **tidak pernah melempar**: status `>= 400` dan
//! semua error transport RETURN `null`. Jadi `Err` dari `ctx.http.get` — termasuk
//! status di atas 399 yang memang jadi `Err` di API plugin — dipetakan ke
//! `Value::Null`, bukan ke cabang `catch`. Kalau tidak, kegagalan jaringan akan
//! diam-diam memakai pesan yang berbeda dari `f()` di Node.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_falsy;
use crate::fun::tod::error_template;

/// `config.APIkey?.neoxr || ''`.
pub(crate) const NEOXR_APIKEY: &str = "";

/// `pixeldraindl.js:36` — endpoint aggregator neoxr.
pub(crate) const ENDPOINT: &str = "https://api.neoxr.eu/api/pixeldrain";

/// `pixeldraindl.js:24-29` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📥 *ᴘɪxᴇʟᴅʀᴀɪɴ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n\
         > Download file dari Pixeldrain\n\n\
         *Format:*\n\
         > `{prefix}pixeldraindl <url>`\n\n\
         *Contoh:*\n\
         > `{prefix}pixeldraindl https://pixeldrain.com/u/xxxxx`"
    )
}

/// `pixeldraindl.js:42` — `!data?.status || !data?.data`.
pub(crate) const TIDAK_ADA: &str = "❌ *ɢᴀɢᴀʟ*\n\n> File tidak ditemukan atau link tidak valid";

/// `pixeldraindl.js:71` — `file.size` ditulis ulang mentah dari backend, bukan
/// angka hasil konversi ke MB.
pub(crate) fn terlalu_besar(size: &str) -> String {
    format!(
        "⚠️ *ꜰɪʟᴇ ᴛᴇʀʟᴀʟᴜ ʙᴇsᴀʀ*\n\n\
         > File {size} terlalu besar untuk dikirim\n\
         > Gunakan link download di atas"
    )
}

/// `/^(https?:\/\/[\w.-]*\.?pixeldrain\.com\/)/` (`pixeldraindl.js:22`).
///
/// Regex JS ini TIDAK punya flag `i`, jadi `HTTPS://PIXELDRAIN.COM/` ditolak.
/// `[\w.-]*` juga tidak dikunci ke batas label, jadi `evilpixeldrain.com/`
/// tetap lolos — dua keanehan itu ikut dipertahankan, bukan diperbaiki.
pub(crate) fn is_pixeldrain_url(url: &str) -> bool {
    let rest = if let Some(r) = url.strip_prefix("https://") {
        r
    } else if let Some(r) = url.strip_prefix("http://") {
        r
    } else {
        return false;
    };
    let Some(ix) = rest.find("pixeldrain.com/") else {
        return false;
    };
    // `\w` di JS ASCII saja: `A-Za-z0-9_`.
    rest[..ix]
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// `encodeURIComponent` (`pixeldraindl.js:36`) — BEDA dari `searchParams.set`
/// milik `douyindl::query_encode`: spasi jadi `%20`, bukan `+`.
pub(crate) fn encode_uri_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `pixeldraindl.js:36` — URL aggregator lengkap.
pub(crate) fn api_url(url: &str, apikey: &str) -> String {
    format!(
        "{ENDPOINT}?url={}&apikey={}",
        encode_uri_component(url),
        apikey
    )
}

/// `String(x)` untuk nilai JSON — dipakai untuk `file.url` yang masuk ke
/// `sendMedia`. `null` ikut jadi string kosong, sama seperti di JS.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// `file.size?.match(/([\d.]+)\s*(MB|GB|KB)/i)` (`pixeldraindl.js:48`).
///
/// `?.` hanya short-circuit kalau `size` `null`/`undefined`. Kalau `size` ada
/// tapi BUKAN string (angka, objek, array), `.match` bukan method dan JS melempar
/// `TypeError` — itu masuk cabang `catch`. Bentuk itu dipisah di sini lewat
/// [`SizeField`] supaya tidak hilang.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SizeField<'a> {
    /// `null` atau `undefined` — `?.` berhenti, `sizeMatch` jadi `undefined`.
    Absent,
    /// String — `String.prototype.match` dipanggil.
    Text(&'a str),
    /// Ada tapi bukan string — JS melempar `TypeError`.
    BukanTeks,
}

/// `/([\d.]+)\s*(MB|GB|KB)/i` + konversi ke MB (`pixeldraindl.js:48-56`).
///
/// Hanya kemunculan PERTAMA yang dipakai. Nilai dibaca dengan `parseFloat`, jadi
/// `"1.2.3"` terbaca `1.2`; `"."` jadi `NaN` dan perbandingan mana pun dengan
/// NaN bernilai false, jadi `sizeInMB` praktis tetap 0.
pub(crate) fn size_in_mb(size: SizeField<'_>) -> f64 {
    let SizeField::Text(raw) = size else {
        return 0.0;
    };
    let Some((value, unit)) = ukuran_dalam(raw) else {
        return 0.0;
    };
    match unit {
        Unit::Gb => value * 1024.0,
        Unit::Mb => value,
        Unit::Kb => value / 1024.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Gb,
    Mb,
    Kb,
}

/// Kemunculan pertama `([\d.]+)\s*(MB|GB|KB)` — `None` kalau tidak cocok.
fn ukuran_dalam(raw: &str) -> Option<(f64, Unit)> {
    let b = raw.as_bytes();
    for i in 0..b.len() {
        if !raw.is_char_boundary(i) || !(b[i].is_ascii_digit() || b[i] == b'.') {
            continue;
        }
        // `\s*` — nol spasi pun boleh.
        let mut j = i;
        while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
            j += 1;
        }
        let mut k = j;
        while let Some(c) = raw[k..].chars().next() {
            if !c.is_whitespace() {
                break;
            }
            k += c.len_utf8();
        }
        let Some(unit) = unit_dari(&raw[k..]) else {
            continue;
        };
        return Some((parse_float_js(&raw[i..j]), unit));
    }
    None
}

/// `MB|GB|KB` dengan flag `i`, lalu `.toUpperCase()` jadi kunci pencabutan.
fn unit_dari(raw: &str) -> Option<Unit> {
    let head = raw.get(..2)?;
    if head.eq_ignore_ascii_case("GB") {
        Some(Unit::Gb)
    } else if head.eq_ignore_ascii_case("MB") {
        Some(Unit::Mb)
    } else if head.eq_ignore_ascii_case("KB") {
        Some(Unit::Kb)
    } else {
        None
    }
}

/// `parseFloat` untuk potongan yang hanya berisi digit dan titik: berhenti di
/// karakter pertama yang bukan bagian angka. `"1.2.3"` -> 1.2, `"5."` -> 5,
/// `"."` -> NaN (dan NaN membuat semua perbandingan di handler bernilai false).
fn parse_float_js(raw: &str) -> f64 {
    let mut teks = String::new();
    let mut sudah_titik = false;
    for c in raw.chars() {
        if c == '.' {
            if sudah_titik {
                break;
            }
            sudah_titik = true;
        }
        teks.push(c);
    }
    while teks.ends_with('.') {
        teks.pop();
    }
    if teks.is_empty() {
        return f64::NAN;
    }
    teks.parse().unwrap_or(f64::NAN)
}

/// `f()` (`src/lib/http.js:32`) — selalu resolve, tidak pernah melempar. Status
/// `>= 400` dan error transport keduanya jadi `null`.
fn f_equivalent(ctx: &Ctx, url: &str) -> Value {
    match ctx.http.get(url) {
        Ok(res) => res.json().unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

/// Klasifikasi `file.size` sesuai perilaku `?.match` di JS.
fn size_field(file: Option<&Value>) -> SizeField<'_> {
    match file.and_then(|f| f.get("size")) {
        None | Some(Value::Null) => SizeField::Absent,
        Some(Value::String(s)) => SizeField::Text(s.as_str()),
        Some(_) => SizeField::BukanTeks,
    }
}

#[plugin(
    name = "pixeldraindl",
    alias = ["pddl", "pixeldrain", "pddownload"],
    category = "download",
    description = "Download file dari Pixeldrain",
    usage = ".pixeldraindl <url>",
    example = ".pixeldraindl https://pixeldrain.com/u/xxxxx",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pixeldraindl(ctx: Ctx) -> Reply {
    // `pixeldraindl.js:19-20` — `m.args[0]`, bukan `m.text`.
    let url = ctx.args.first().map(|a| a.trim()).unwrap_or_default();

    if url.is_empty() || !is_pixeldrain_url(url) {
        return Reply::text(help(&ctx.prefix));
    }

    let data = f_equivalent(&ctx, &api_url(url, NEOXR_APIKEY));

    // `!data?.status || !data?.data` — keduanya pakai falsiness JS, bukan
    // `=== true` atau sekadar "key tidak ada".
    if js_falsy(data.get("status")) || js_falsy(data.get("data")) {
        return Reply::text(TIDAK_ADA).with_reaction("❌");
    }

    let file = data.get("data");

    // `file.size?.match(...)` pada nilai non-string melempar `TypeError`, dan
    // itu satu-satunya jalan yang masih hidup ke cabang `catch`.
    let size = size_field(file);
    let mb = size_in_mb(size);
    let size_teks = match size {
        SizeField::Absent => String::new(),
        SizeField::Text(s) => s.to_string(),
        SizeField::BukanTeks => {
            return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
        }
    };

    if mb > 0.0 && mb <= 100.0 {
        // `pixeldraindl.js:60-68` — `sendMedia` document; `Reply` tidak punya
        // variant document, jadi URL filenya yang terkirim sebagai teks.
        let url_file = file.and_then(|f| f.get("url")).map(js_to_string).unwrap_or_default();
        return Reply::text(url_file).with_reaction("✅");
    }
    if mb > 100.0 {
        return Reply::text(terlalu_besar(&size_teks)).with_reaction("✅");
    }

    // `mb == 0`: tidak ada cabang yang menyala, tapi `m.react("✅")` tetap jalan.
    Reply::None.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, find};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const SENDER: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        body: Option<Value>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v),
                seen: Mutex::new(Vec::new()),
            })
        }
        /// `f()` mengembalikan `null` untuk status >= 400 dan error transport.
        fn null() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let Some(v) = &self.body else {
                return Err(HttpError {
                    message: "transport".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(v.to_string().into_bytes()),
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

    fn run(http: Arc<Stub>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pixeldraindl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: args.join(" "),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            http,
            ..Default::default()
        };
        (find("pixeldraindl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> Option<String> {
        r.without_mutations().text_body().map(str::to_string)
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn ok_file(size: Value) -> Value {
        json!({
            "status": true,
            "data": { "url": "https://cdn.example/f.zip", "filename": "f.zip", "size": size }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("pixeldraindl").unwrap();
        assert_eq!(p.meta.name, "pixeldraindl");
        assert_eq!(p.meta.alias, ["pddl", "pixeldrain", "pddownload"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download file dari Pixeldrain");
        assert_eq!(p.meta.usage, ".pixeldraindl <url>");
        assert_eq!(
            p.meta.example,
            ".pixeldraindl https://pixeldrain.com/u/xxxxx"
        );
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
        // `pixeldraindl.js:6-16` tidak menyebut satu pun flag permission.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pddl", "pixeldrain", "pddownload"] {
            assert_eq!(find(a).unwrap().meta.name, "pixeldraindl", "alias {a}");
        }
    }

    #[test]
    fn a_missing_or_foreign_url_shows_the_card_without_reacting() {
        for args in [
            vec![],
            vec!["halo"],
            vec!["https://pixeldrain.com"],
            vec!["https://pixeldrain.com.evil.example/u/x"],
            vec!["pixeldrain.com/u/x"],
        ] {
            let http = Stub::json(ok_file(json!("5 MB")));
            let r = run(http.clone(), &args);
            assert_eq!(text_of(&r).as_deref(), Some(help(".").as_str()), "{args:?}");
            assert_eq!(reaksi(&r), None, "{args:?}");
            assert!(http.seen.lock().unwrap().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "pixeldraindl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("pixeldraindl").unwrap().handler)(ctx);
        assert_eq!(text_of(&r).as_deref(), Some(help("!").as_str()));
    }

    #[test]
    fn the_unanchored_regex_quirks_of_the_js_are_kept() {
        for good in [
            "https://pixeldrain.com/u/x",
            "http://pixeldrain.com/u/x",
            "https://sub.pixeldrain.com/u/x",
            "https://a-b.pixeldrain.com/u/x",
            // `[\w.-]*` tidak dikunci ke batas label.
            "https://evilpixeldrain.com/u/x",
        ] {
            assert!(is_pixeldrain_url(good), "harus lolos: {good}");
        }
        for bad in [
            // Regex tidak punya flag `i`.
            "HTTPS://PIXELDRAIN.COM/u/x",
            "Https://pixeldrain.com/u/x",
            "https://pixeldrain.com.evil.example/u/x",
            "https://pixeldrain.com",
            "pixeldrain.com/u/x",
            "ftp://pixeldrain.com/u/x",
            "",
        ] {
            assert!(!is_pixeldrain_url(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn the_first_argument_is_trimmed_before_the_guard() {
        // `args[0]?.trim()` — spasi di tepi hilang sebelum regex, jadi URL yang
        // sama menolak sebagai mentah tapi lolos setelah di-trim.
        assert!(!is_pixeldrain_url("  https://pixeldrain.com/u/x  "));
        let r = run(
            Stub::json(ok_file(json!("5 MB"))),
            &["  https://pixeldrain.com/u/x  "],
        );
        assert_eq!(reaksi(&r), Some("✅"));
        assert_eq!(text_of(&r).as_deref(), Some("https://cdn.example/f.zip"));
    }

    #[test]
    fn the_query_uses_encode_uri_component_not_form_encoding() {
        assert_eq!(
            api_url("https://pixeldrain.com/u/a b?x=1", NEOXR_APIKEY),
            "https://api.neoxr.eu/api/pixeldrain\
             ?url=https%3A%2F%2Fpixeldrain.com%2Fu%2Fa%20b%3Fx%3D1&apikey="
        );
        // `+` ikut ter-encode; spasi `%20`, bukan `+`.
        assert_eq!(encode_uri_component("a+b c"), "a%2Bb%20c");
        assert_eq!(encode_uri_component("a-b_c.d!~*'()"), "a-b_c.d!~*'()");
        assert_eq!(encode_uri_component("/:@"), "%2F%3A%40");
        assert_eq!(encode_uri_component("\u{1F600}"), "%F0%9F%98%80");
    }

    #[test]
    fn the_size_regex_reads_the_first_unit_it_finds() {
        let mb = |s: &str| size_in_mb(SizeField::Text(s));
        assert_eq!(mb("5 MB"), 5.0);
        assert_eq!(mb("5MB"), 5.0);
        assert_eq!(mb("12 mb"), 12.0, "flag i, lalu di-upper");
        assert_eq!(mb("abc 7 GB"), 7168.0);
        assert_eq!(mb("2048 KB"), 2.0);
        assert_eq!(mb("1024 KB"), 1.0);
        assert_eq!(mb("5  MB"), 5.0, "spasi di \\s* boleh nol");
        assert_eq!(mb("1.2.3 MB"), 1.2, "parseFloat berhenti di titik kedua");
        // `([\d.]+)\s*(MB|GB|KB)` mewajibkan satuan, jadi angka tanpa unit tidak cocok.
assert_eq!(mb("5."), 0.0);
        assert_eq!(mb("5 MB extra GB"), 5.0, "hanya kemunculan pertama");
        for tanpa in ["12345 bytes", "5 XB", "MB", "", "abc"] {
            assert_eq!(mb(tanpa), 0.0, "{tanpa:?}");
        }
        // `parseFloat(".")` = NaN: semua perbandingan dengan NaN false, jadi
        // `sizeInMB` praktis 0 dan tidak ada cabang yang menyala.
        assert!(mb(". MB").is_nan());
        assert!(mb(". MB").partial_cmp(&0.0).is_none());
        assert!(mb(". MB").partial_cmp(&100.0).is_none());
    }

    #[test]
    fn a_size_field_that_is_not_text_is_a_type_error_in_js() {
        assert_eq!(size_in_mb(SizeField::Absent), 0.0);
        assert_eq!(size_in_mb(SizeField::BukanTeks), 0.0);
        for nilai in [json!(5), json!(true), json!([]), json!({})] {
            let r = run(Stub::json(ok_file(nilai.clone())), &["https://pixeldrain.com/u/x"]);
            assert_eq!(
                text_of(&r).as_deref(),
                Some(error_template(".", "pixeldraindl").as_str()),
                "{nilai}"
            );
            assert_eq!(reaksi(&r), Some("☢"), "{nilai}");
        }
        // String dan null tidak melempar — keduanya jalan biasa.
        for nilai in [json!("5 MB"), json!(null)] {
            let r = run(Stub::json(ok_file(nilai.clone())), &["https://pixeldrain.com/u/x"]);
            assert_ne!(text_of(&r).as_deref(), Some(error_template(".", "pixeldraindl").as_str()), "{nilai}");
        }
    }

    #[test]
    fn a_file_of_at_most_100_mb_is_sent() {
        for size in ["5 MB", "100 MB", "0.09 GB", "1024 KB", "12 mb"] {
            let http = Stub::json(ok_file(json!(size)));
            let r = run(http.clone(), &["https://pixeldrain.com/u/x"]);
            assert_eq!(
                text_of(&r).as_deref(),
                Some("https://cdn.example/f.zip"),
                "size {size}"
            );
            assert_eq!(reaksi(&r), Some("✅"), "size {size}");
            assert_eq!(http.seen.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn a_file_over_100_mb_answers_with_the_raw_size_string() {
        for size in ["100.4 MB", "101 MB", "2 GB", "200000 KB"] {
            let http = Stub::json(ok_file(json!(size)));
            let r = run(http, &["https://pixeldrain.com/u/x"]);
            assert_eq!(
                text_of(&r).as_deref(),
                Some(terlalu_besar(size).as_str()),
                "size {size}"
            );
            assert!(
                text_of(&r).unwrap().contains(size),
                "ukuran mentah harus ikut tampil: {size}"
            );
            assert_eq!(reaksi(&r), Some("✅"), "size {size}");
        }
    }

    #[test]
    fn an_unparsable_size_sends_nothing_but_still_reacts() {
        // `sizeInMB == 0` tidak masuk cabang mana pun; `m.react("✅")` tetap
        // dipanggil dan `Reply` tidak boleh mengarang teks.
        for size in ["12345 bytes", "MB", "0 MB", ""] {
            let http = Stub::json(ok_file(json!(size)));
            let r = run(http, &["https://pixeldrain.com/u/x"]);
            assert_eq!(text_of(&r), None, "size {size:?}");
            assert_eq!(reaksi(&r), Some("✅"), "size {size:?}");
        }
    }

    #[test]
    fn a_null_or_missing_envelope_answers_with_the_not_found_message() {
        // `f()` mengembalikan `null` untuk status >= 400 dan error transport,
        // jadi keduanya tidak boleh jatuh ke template error operator.
        for body in [
            Stub::null(),
            Stub::json(Value::Null),
            Stub::json(json!({})),
            Stub::json(json!({ "status": false, "data": { "url": "u" } })),
            Stub::json(json!({ "status": true })),
            Stub::json(json!({ "status": 0, "data": { "url": "u" } })),
            Stub::json(json!({ "status": "" })),
        ] {
            let r = run(body, &["https://pixeldrain.com/u/x"]);
            assert_eq!(text_of(&r).as_deref(), Some(TIDAK_ADA));
            assert_eq!(reaksi(&r), Some("❌"));
        }
    }

    #[test]
    fn a_truthy_non_boolean_status_is_accepted_like_in_js() {
        // `!data?.status` memakai falsiness JS, bukan `status === true`.
        for status in [json!(1), json!("true"), json!([]), json!({})] {
            let body = json!({ "status": status, "data": { "url": "https://cdn/f", "size": "1 MB" } });
            let r = run(Stub::json(body.clone()), &["https://pixeldrain.com/u/x"]);
            assert_eq!(text_of(&r).as_deref(), Some("https://cdn/f"), "{status}");
            assert_eq!(reaksi(&r), Some("✅"), "{status}");
        }
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        let r = run(Stub::json(ok_file(json!("5 MB"))), &["https://pixeldrain.com/u/x"]);
        assert!(r.mutations().is_empty());
    }
}