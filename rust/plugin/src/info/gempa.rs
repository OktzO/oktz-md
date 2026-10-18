//! `plugins/info/gempa.js` — info gempa terkini dari BMKG.
//!
//! `ponytail:`
//! 1. `sock.sendMedia(m.chat, Buffer.from(imgRes.data), text, m, {type:'image'})`
//!    (`gempa.js:56`) dijawab `Reply::image(MediaSource::url(..), text)`.
//!    `MediaSource::Url` baru di-fetch saat pipeline mengirim, sedangkan JS
//!    men-download dulu di `:52-55`; akibatnya `catch` di `:59` yang turun ke
//!    `m.reply(text, ..)` tidak bisa terjadi di sini.
//! 2. `contextInfo: saluranCtx()` (`gempa.js:60` dan `:63`) tidak punya field
//!    di `Reply` — badge forwarded-newsletter hilang.
//! 3. `m.react('🕕')` di `:21` tidak ikut: `Reply::Reaction` hanya bisa
//!    membawa satu emoji, jadi yang dipasang adalah reaksi cabang akhir
//!    (`✅` atau `☢`). Sama seperti `crate::info::jadwalbola`.
//!
//! `gempa.js` tidak pernah memanggil `te(...)`, jadi `error_template` tidak
//! dipakai di file ini.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;

const URL_JSON: &str = "https://data.bmkg.go.id/DataMKG/TEWS/autogempa.json";
const URL_SHAKEMAP: &str = "https://data.bmkg.go.id/DataMKG/TEWS/";

/// `const g = response.data.Infogempa.gempa` (`gempa.js:29`) lalu `g.Shakemap`
/// di `:30`. Kalau `gempa` tidak ada di payload, baris pertama tidak melempar
/// — yang melempar adalah pembacaan properti di baris berikutnya, jadi
/// pesannya adalah `TypeError` V8 itu, bukan teks buatan plugin.
const MISSING_PAYLOAD: &str = "Cannot read properties of undefined (reading 'Shakemap')";

/// `${nilai}` di dalam template literal JS — `String(x)`.
///
/// Field BMKG diinterpolasi apa adanya, jadi nilai yang hilang ikut tampil
/// sebagai `undefined`, bukan sebagai string kosong.
pub(crate) fn js_str(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::from("undefined"),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        // `String([1, 2])` = `"1,2"`, dan elemen null/undefined jadi "".
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                other => js_str(Some(other)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => String::from("[object Object]"),
    }
}

/// `const text = ...` (`gempa.js:34-46`).
fn caption(g: &Value) -> String {
    let f = |k: &str| js_str(g.get(k));
    let mut t = String::from("🌍 *Info Gempa Terkini — BMKG*\n\n");
    t.push_str(&format!("> 📅 Tanggal: *{}*\n", f("Tanggal")));
    t.push_str(&format!("> 🕐 Jam: *{}*\n", f("Jam")));
    t.push_str(&format!("> 📐 Koordinat: *{}*\n", f("Coordinates")));
    t.push_str(&format!("> 📍 Lintang: *{}*\n", f("Lintang")));
    t.push_str(&format!("> 📍 Bujur: *{}*\n", f("Bujur")));
    t.push_str(&format!("> 💥 Magnitude: *{}*\n", f("Magnitude")));
    t.push_str(&format!("> 🔽 Kedalaman: *{}*\n", f("Kedalaman")));
    t.push_str(&format!("> 🗺️ Wilayah: *{}*\n", f("Wilayah")));
    t.push_str(&format!("> ⚠️ Potensi: *{}*\n", f("Potensi")));
    t.push_str(&format!("> 🏠 Dirasakan: *{}*\n", f("Dirasakan")));
    t.push_str("\n_Sumber: BMKG Indonesia_");
    t
}

/// `catch (e)` (`gempa.js:65-70`): `❌ *Gagal…*\n\n> ${e.message || "Coba lagi nanti"}`.
fn gagal(pesan: &str) -> Reply {
    let pesan = if pesan.is_empty() {
        "Coba lagi nanti"
    } else {
        pesan
    };
    Reply::text(format!("❌ *Gagal mengambil data gempa*\n\n> {pesan}"))
        .with_reaction("☢")
}

#[plugin(
    name = "gempa",
    alias = ["bmkg", "infogempa", "earthquake"],
    category = "info",
    description = "Info gempa terkini dari BMKG",
    usage = ".gempa",
    example = ".gempa",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn gempa(ctx: Ctx) -> Reply {
    let response = match ctx.http.get(URL_JSON) {
        Ok(r) => r,
        Err(e) => return gagal(&e.message),
    };
    let payload = response.json();
    let Some(g) = payload.as_ref().and_then(|v| v.pointer("/Infogempa/gempa")).cloned() else {
        return gagal(MISSING_PAYLOAD);
    };

    let text = caption(&g);
    // `g.Shakemap ? `…/${g.Shakemap}` : null` (`gempa.js:30-32`) — nilai
    // kosong itu falsy, jadi tidak jadi URL.
    let shakemap = g
        .get("Shakemap")
        .filter(|v| js_truthy(Some(*v)))
        .map(|v| js_str(Some(v)));

    match shakemap {
        Some(file) => Reply::image(MediaSource::url(format!("{URL_SHAKEMAP}{file}")), text)
            .with_reaction("✅"),
        None => Reply::text(text).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse, HttpGet};
    use std::sync::Arc;

    #[derive(Debug)]
    struct Stub {
        body: Option<String>,
        seen: std::sync::Mutex<Vec<String>>,
    }

    impl Stub {
        fn with(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(body.to_string()),
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let Some(text) = &self.body else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.clone().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("gempa hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("gempa hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("gempa hanya GET")
        }
    }

    const CONTOH: &str = r#"{"Infogempa":{"gempa":{
        "Tanggal":"15 Oktober 2026","Jam":"10:23:45 WIB",
        "Coordinates":"-7.42, 110.12","Lintang":"7.42 LS","Bujur":"110.12 BT",
        "Magnitude":"4.9","Kedalaman":"10 km","Wilayah":"Jawa Tengah",
        "Potensi":"Tidak berpotensi tsunami","Dirasakan":"Kabupaten Semarang",
        "Shakemap":"map.png"}}}"#;

    fn run(http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "gempa".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("gempa").unwrap().handler)(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations().text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gempa").unwrap();
        assert_eq!(p.meta.name, "gempa");
        assert_eq!(p.meta.alias, ["bmkg", "infogempa", "earthquake"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Info gempa terkini dari BMKG");
        assert_eq!(p.meta.usage, ".gempa");
        assert_eq!(p.meta.example, ".gempa");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
        for a in ["bmkg", "infogempa", "earthquake"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "gempa");
        }
    }

    #[test]
    fn shakemap_present_sends_the_picture_with_the_same_caption() {
        let r = run(Stub::with(CONTOH));
        assert!(r.has_media(), "Shakemap ada -> image, bukan teks");
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("url"),
            "sumbernya URL BMKG"
        );
        match r.media_source() {
            Some(MediaSource::Url { url, .. }) => {
                assert_eq!(url, "https://data.bmkg.go.id/DataMKG/TEWS/map.png")
            }
            other => panic!("harus Url, dapat {other:?}"),
        }
        assert_eq!(
            teks(&r),
            concat!(
                "🌍 *Info Gempa Terkini — BMKG*\n\n",
                "> 📅 Tanggal: *15 Oktober 2026*\n",
                "> 🕐 Jam: *10:23:45 WIB*\n",
                "> 📐 Koordinat: *-7.42, 110.12*\n",
                "> 📍 Lintang: *7.42 LS*\n",
                "> 📍 Bujur: *110.12 BT*\n",
                "> 💥 Magnitude: *4.9*\n",
                "> 🔽 Kedalaman: *10 km*\n",
                "> 🗺️ Wilayah: *Jawa Tengah*\n",
                "> ⚠️ Potensi: *Tidak berpotensi tsunami*\n",
                "> 🏠 Dirasakan: *Kabupaten Semarang*\n\n",
                "_Sumber: BMKG Indonesia_"
            )
        );
        assert_eq!(r.mutations().len(), 0, "tidak ada yang ditulis ke store");
    }

    #[test]
    fn the_only_request_goes_to_the_bmkg_autogempa_endpoint() {
        let stub = Stub::with(CONTOH);
        run(stub.clone());
        assert_eq!(stub.seen.lock().unwrap().as_slice(), [URL_JSON]);
    }

    #[test]
    fn an_empty_shakemap_falls_back_to_plain_text() {
        let r = run(Stub::with(
            r#"{"Infogempa":{"gempa":{"Tanggal":"X","Shakemap":""}}}"#,
        ));
        assert!(!r.has_media(), "string kosong itu falsy -> tidak ada gambar");
        assert!(teks(&r).ends_with("_Sumber: BMKG Indonesia_"));
    }

    #[test]
    fn a_missing_shakemap_key_also_falls_back_to_plain_text() {
        let r = run(Stub::with(r#"{"Infogempa":{"gempa":{"Tanggal":"X"}}}"#));
        assert!(!r.has_media());
        assert!(teks(&r).contains("> 📅 Tanggal: *X*\n"));
    }

    #[test]
    fn missing_fields_interpolate_as_undefined_not_as_empty_text() {
        let r = run(Stub::with(r#"{"Infogempa":{"gempa":{}}}"#));
        let t = teks(&r);
        assert!(t.contains("> 📅 Tanggal: *undefined*\n"), "{t}");
        assert!(t.contains("> 🏠 Dirasakan: *undefined*\n"), "{t}");
        assert!(!r.has_media());
    }

    #[test]
    fn a_payload_without_the_gempa_node_takes_the_error_branch() {
        let r = run(Stub::with(r#"{"Infogempa":{}}"#));
        assert!(!r.has_media());
        assert_eq!(
            teks(&r),
            format!("❌ *Gagal mengambil data gempa*\n\n> {MISSING_PAYLOAD}")
        );
    }

    #[test]
    fn a_body_that_is_not_json_takes_the_error_branch() {
        let r = run(Stub::with("<html>502</html>"));
        assert_eq!(
            teks(&r),
            format!("❌ *Gagal mengambil data gempa*\n\n> {MISSING_PAYLOAD}")
        );
    }

    #[test]
    fn an_unreachable_api_reports_the_transport_error_verbatim() {
        let r = run(Stub::offline());
        assert_eq!(
            teks(&r),
            "❌ *Gagal mengambil data gempa*\n\n> offline",
            "e.message ikut, bukan pesan bawaan plugin"
        );
    }

    #[test]
    fn an_empty_error_message_falls_back_to_the_js_default() {
        assert_eq!(
            teks(&gagal("")),
            "❌ *Gagal mengambil data gempa*\n\n> Coba lagi nanti",
            "`e.message || \"Coba lagi nanti\"`"
        );
    }

    #[test]
    fn js_str_mirrors_string_coercion_for_the_shapes_bmkg_can_send() {
        assert_eq!(js_str(None), "undefined");
        assert_eq!(js_str(Some(&Value::Null)), "undefined");
        assert_eq!(js_str(Some(&serde_json::json!("-7.42, 110.12"))), "-7.42, 110.12");
        assert_eq!(js_str(Some(&serde_json::json!(4))), "4");
        assert_eq!(js_str(Some(&serde_json::json!(true))), "true");
        assert_eq!(js_str(Some(&serde_json::json!([1, "a", null]))), "1,a,");
        assert_eq!(js_str(Some(&serde_json::json!({}))), "[object Object]");
    }

    #[test]
    fn a_very_long_field_is_not_truncated_by_the_caption() {
        // Tidak ada `slice` di `gempa.js:34-46`; BMKG yang memotong.
        let panjang = "x".repeat(500);
        let body = format!(r#"{{"Infogempa":{{"gempa":{{"Wilayah":"{panjang}"}}}}}}"#);
        assert!(body.starts_with(r#"{"Infogempa":{"gempa":{"Wilayah":""#), "{body}");
        let t = teks(&run(Stub::with(&body)));
        assert!(t.contains(&format!("> 🗺️ Wilayah: *{panjang}*\n")), "{t}");
    }
}
