//! `plugins/stalker/countrystalk.js` — informasi negara lewat API Neoxr.
//!
//! `category` di `pluginConfig` adalah `"stalker  "` dengan DUA spasi di
//! belakang (`countrystalk.js:7`). Itu disalin apa adanya karena plugin JS
//! produksi memakai nilai itu, dan membulatkan whitespace di sini akan
//! membuat daftar kategori tidak cocok.
//!
//! `ponytail:` `c.population.toLocaleString('id-ID')` memakai pemisah ribuan
//! titik dan pemisah desimal koma (dikonfirmasi dengan
//! `(1234567).toLocaleString('id-ID')` di Node). `toLocaleString` juga
//! membulatkan ke tiga angka desimal untuk angka pecahan; itu ikut.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use super::npmstalk::{js_str, or_dash};
use crate::tools::phisingataubukan::encode_uri_component;

/// `countrystalk.js:20` — pesan tanpa negara.
pub(crate) fn help(prefix: &str, command: &str) -> String {
    format!("🌍 Harap masukkan nama negara!\nContoh: `{prefix}{command} indonesia`")
}

/// `countrystalk.js:33` — data tidak ditemukan.
pub(crate) fn not_found(text: &str) -> String {
    format!("Maaf, data negara untuk \"{text}\" tidak ditemukan.")
}

/// `countrystalk.js:30` — endpoint + query `q`.
pub(crate) fn api_url(query: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/country?q={query}&apikey={api_key}"
    )
}

/// `(n).toLocaleString('id-ID')` — pemisah ribuan titik, desimal koma.
///
/// `Intl.NumberFormat` maximumFractionDigits default-nya 3, jadi pecahan
/// dibulatkan ke tiga digit dan nol diakhiri dibuang.
pub(crate) fn locale_id(v: &Value) -> String {
    let n = match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    let Some(n) = n else {
        return "NaN".to_string();
    };
    let bulat = n.abs().trunc();
    let digit = format!("{bulat:.0}");
    let mut groups: Vec<String> = Vec::new();
    let mut sisa = digit.len();
    while sisa > 3 {
        groups.push(digit[sisa - 3..sisa].to_string());
        sisa -= 3;
    }
    groups.push(digit[..sisa].to_string());
    groups.reverse();
    let bulat_str = groups.join(".");
    let bulat_str = if n < 0.0 { format!("-{bulat_str}") } else { bulat_str };

    // maximumFractionDigits default 3: pecahan dibulatkan ke tiga digit dan
    // nol diakhiri dibuang. Pecahan nol tidak dicetak sama sekali.
    let pecahan = ((n.abs() - bulat) * 1000.0).round() / 1000.0;
    if pecahan == 0.0 {
        return bulat_str;
    }
    let mut desimal = format!("{pecahan:.3}");
    // Buang nol di depan koma karena bagian bulat sudah tercetak.
    desimal = desimal.trim_start_matches('0').to_string();
    desimal = desimal.trim_end_matches('0').to_string();
    // Setelah nol depan dibuang, desimal diawali titik; titik itu ganti koma.
    format!("{bulat_str}{}", desimal.replacen('.', ",", 1))
}

/// `list.length > 0 ? list.map(f).join(", ") : "-"` untuk array objek.
fn gabung_objek(v: &Value, ambil: impl Fn(&Value) -> String) -> String {
    let Some(arr) = v.as_array() else {
        return "-".to_string();
    };
    if arr.is_empty() {
        return "-".to_string();
    }
    arr.iter().map(ambil).collect::<Vec<_>>().join(", ")
}

/// `list.length > 0 ? list.join(", ") : "-"` untuk array apa adanya.
fn gabung(v: &Value) -> String {
    let Some(arr) = v.as_array() else {
        return "-".to_string();
    };
    if arr.is_empty() {
        return "-".to_string();
    }
    arr.iter().map(js_str).collect::<Vec<_>>().join(", ")
}

/// `countrystalk.js:46-61` — caption.
pub(crate) fn caption(c: &Value, nama_mentah: &str) -> String {
    let name = {
        let n = js_str(&c["names"]["common"]);
        if n.is_empty() || n == "null" {
            nama_mentah.to_string()
        } else {
            n
        }
    };
    let population = if crate::common::js_truthy_value(&c["population"]) {
        locale_id(&c["population"])
    } else {
        "-".to_string()
    };
    let capitals = gabung_objek(&c["capitals"], |x| js_str(&x["name"]));
    let currencies = gabung_objek(&c["currencies"], |x| {
        format!("{} ({})", js_str(&x["name"]), js_str(&x["symbol"]))
    });
    let languages = gabung_objek(&c["languages"], |x| js_str(&x["name"]));
    let timezones = gabung(&c["timezones"]);

    let mut txt = format!("🌍 *INFORMASI NEGARA: {}* 🌍\n\n", name.to_uppercase());
    txt.push_str(&format!("🏢 *Ibukota:* {capitals}\n"));
    txt.push_str(&format!(
        "📜 *Nama Resmi:* {}\n",
        or_dash(&c["names"]["official"])
    ));
    txt.push_str(&format!(
        "🗺️ *Wilayah:* {} ({})\n",
        or_dash(&c["region"]),
        or_dash(&c["subregion"])
    ));
    txt.push_str(&format!("🗣️ *Bahasa Resmi:* {languages}\n"));
    txt.push_str(&format!("👥 *Populasi:* {population} jiwa\n"));
    txt.push_str(&format!("💰 *Mata Uang:* {currencies}\n"));
    txt.push_str(&format!("🕰️ *Zona Waktu:* {timezones}\n"));
    txt.push_str(&format!(
        "🚗 *Posisi Mengemudi:* {}\n",
        or_dash(&c["cars"]["driving_side"])
    ));
    // `c.flag?.emoji` — hanya ditambahkan kalau ada.
    if crate::common::js_truthy_value(&c["flag"]["emoji"]) {
        txt.push_str(&format!("\n*Bendera:* {}", js_str(&c["flag"]["emoji"])));
    }
    txt
}

#[plugin(
    name = "countrystalk",
    alias = ["stalknegara", "infonegara"],
    // Dua spasi di belakang, sama seperti `countrystalk.js:7`.
    category = "stalker  ",
    description = "Mencari informasi detail tentang suatu negara",
    usage = ".country <nama negara>",
    example = ".country indonesia",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn countrystalk(ctx: Ctx) -> Reply {
    let text = ctx.text.trim().to_string();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix, &ctx.command));
    }

    let api_key = ctx.config().api_keys.neoxr.clone();
    let query = encode_uri_component(&text);
    let Ok(res) = ctx.http.get(&api_url(&query, &api_key)) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    let body = res.json().unwrap_or(Value::Null);
    let ok = crate::common::js_truthy_value(&body)
        && body.get("status").is_some_and(crate::common::js_truthy_value)
        && body.get("data").is_some_and(|v| !v.is_null());
    if !ok {
        return Reply::text(not_found(&text)).with_reaction("❌");
    }

    let c = &body["data"];
    let teks = caption(c, &text);
    let balasan = if crate::common::js_truthy_value(&c["flag"]["url_png"]) {
        Reply::image(MediaSource::url(js_str(&c["flag"]["url_png"])), teks)
    } else {
        Reply::text(teks)
    };
    balasan.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn gagal() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
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

    fn ctx(http: Arc<Stub>, text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "country".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    neoxr: "KUNCI".to_string(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn category_dua_spasi_konus_javascript() {
        // `countrystalk.js:7` menulis "stalker  ". Dibulatkan jadi satu
        // spasi akaningen tidak cocok dengan produksi.
        let teks = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../test/fixtures/countrystalk.js"),
        )
        .expect("baca JS");
        let baris = teks
            .lines()
            .find(|l| l.contains("category:"))
            .expect("ada baris category");
        assert!(
            baris.contains("\"stalker  \""),
            "dua spasi harus tetap: {baris}"
        );
    }

    #[test]
    fn tanpa_negara_menghasilkan_contoh_dengan_command_asli() {
        let r = countrystalk(ctx(Stub::gagal(), "  "));
        assert_eq!(
            r.without_mutations().text_body(),
            Some("🌍 Harap masukkan nama negara!\nContoh: `.country indonesia`")
        );
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn locale_id_mengikuti_pemisah_id_id() {
        assert_eq!(locale_id(&serde_json::json!(1234567)), "1.234.567");
        assert_eq!(locale_id(&serde_json::json!(1000)), "1.000");
        assert_eq!(locale_id(&serde_json::json!(999)), "999");
        assert_eq!(locale_id(&serde_json::json!(0.5)), "0,5");
        assert_eq!(locale_id(&serde_json::json!(1234.5678)), "1.234,568");
        assert_eq!(locale_id(&serde_json::json!(-1234)), "-1.234");
    }

    #[test]
    fn url_membawa_query_terencode_dan_kunci() {
        assert_eq!(
            api_url("indonesia%20raya", "K"),
            "https://api.neoxr.eu/api/country?q=indonesia%20raya&apikey=K"
        );
        assert_eq!(encode_uri_component("a b&c"), "a%20b%26c");
    }

    #[test]
    fn body_tidak_lengkap_menghasilkan_pesan_tidak_ditemukan() {
        for body in [
            Value::Null,
            serde_json::json!({}),
            serde_json::json!({ "status": false }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": null }),
        ] {
            let r = countrystalk(ctx(Stub::body(body.clone()), "x"));
            assert_eq!(r.without_mutations().text_body(), Some(not_found("x").as_str()), "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn caption_menyalin_sembilan_baris_dan_bendera_opsional() {
        let c = serde_json::json!({
            "names": { "common": "Indonesia", "official": "Republic of Indonesia" },
            "population": 275501339,
            "capitals": [{ "name": "Jakarta" }],
            "currencies": [{ "name": "Indonesian Rupiah", "symbol": "Rp" }],
            "languages": [{ "name": "Indonesian" }],
            "timezones": ["UTC+07:00"],
            "region": "South-Eastern Asia",
            "subregion": "Southern Asia",
            "cars": { "driving_side": "left" },
            "flag": { "emoji": "🇮🇩", "url_png": "https://cdn/id.png" }
        });
        let txt = caption(&c, "indonesia");
        assert!(txt.starts_with("🌍 *INFORMASI NEGARA: INDONESIA* 🌍\n\n"));
        assert!(txt.contains("🏢 *Ibukota:* Jakarta"));
        assert!(txt.contains("📜 *Nama Resmi:* Republic of Indonesia"));
        assert!(txt.contains("🗺️ *Wilayah:* South-Eastern Asia (Southern Asia)"));
        assert!(txt.contains("🗣️ *Bahasa Resmi:* Indonesian"));
        assert!(txt.contains("👥 *Populasi:* 275.501.339 jiwa"));
        assert!(txt.contains("💰 *Mata Uang:* Indonesian Rupiah (Rp)"));
        assert!(txt.contains("🕰️ *Zona Waktu:* UTC+07:00"));
        assert!(txt.contains("🚗 *Posisi Mengemudi:* left"));
        assert!(txt.ends_with("\n*Bendera:* 🇮🇩"));

        // Tanpa bendera, baris terakhir tidak ditambahkan.
        let tanpa = serde_json::json!({ "names": { "common": "X" } });
        assert!(!caption(&tanpa, "x").contains("Bendera"));
    }

    #[test]
    fn nama_kosong_jatuh_ke_teks_yang_diketik_user() {
        let c = serde_json::json!({ "names": {} });
        assert!(caption(&c, "input user")
            .starts_with("🌍 *INFORMASI NEGARA: INPUT USER* 🌍"));
    }

    #[test]
    fn jalur_berhasil_mengirim_bendera_lalu_reaksi_centang() {
        let http = Stub::body(serde_json::json!({
            "status": true,
            "data": { "names": { "common": "Indonesia" }, "flag": { "url_png": "https://cdn/id.png" } }
        }));
        let r = countrystalk(ctx(http, "indonesia"));
        assert_eq!(reaction_of(&r), Some("✅"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Image { media, .. } => assert_eq!(
                    media.as_ref().unwrap(),
                    &MediaSource::Url {
                        url: "https://cdn/id.png".to_string(),
                        mimetype: None,
                    }
                ),
                lain => panic!("harus image, dapat {lain:?}"),
            },
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn tanpa_bendera_mengirim_teks_dan_kegagalan_jaringan_membungkus_error() {
        let http = Stub::body(serde_json::json!({ "status": true, "data": { "names": { "common": "X" } } }));
        let r = countrystalk(ctx(http, "x"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { reply, .. } if matches!(reply.as_ref(), Reply::Text { .. })
        ));
        let r = countrystalk(ctx(Stub::gagal(), "x"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some(crate::common::error_template_bawaan(".", "country").as_str())
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
