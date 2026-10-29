//! `plugins/search/comparation.js` (`advance-comparation`) — cari perangkat
//! lalu simpan daftar kandidat di `user.compare_session`.
//!
//! `ponytail:` file JS punya DUA handler. `handler` (yang ada di sini)
//! memulai pencarian dan menulis sesi; `comparationAnswerHandler` adalah hook
//! pesan biasa (`if (!m.body || m.isCommand) return false`) yang membaca angka
//! pilihan, kelanjutannya, lalu hasil perbandingan. Hook pesan biasa tidak ada
//! di `oktz-plugin-api` — satu plugin satu command — jadi handler kedua itu
//! tidak ikut, dan sesi yang ditulis di sini tidak punya tahap lanjutan.
//!
//! `ponytail:` `db.getUser(m.sender)` lalu `user.compare_session = {…}` +
//! `db.save()` dipetakan ke `Mutation::SetUser` yang menimpa satu field.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy, now_ms};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `comparation.js:24` — tipe yang boleh jadi argumen pertama.
pub(crate) const VALID_TYPES: [&str; 6] = ["phone", "tablet", "laptop", "cpu", "gpu", "soc"];

/// `comparation.js:50` dan `:147` — endpoint pencarian perangkat.
pub(crate) fn search_url(query: &str, device_type: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/compare-search?q={}&type={device_type}&apikey={NEOXR_APIKEY}",
        encode_uri_component(query)
    )
}

/// `comparation.js:28-33` — kartu saat `m.args` kosong.
pub(crate) fn help() -> String {
    let types = VALID_TYPES
        .iter()
        .map(|t| format!("> - {t}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "📊 *ADVANCE COMPARATION*\n\n\
         Gunakan perintah berikut untuk membandingkan perangkat:\n\
         > *.bandingkan-device <tipe> <query>*\n\n\
         *Tipe yang tersedia:*\n\
         {types}\n\n\
         *Contoh:* .bandingkan-device phone samsung s24"
    )
}

/// `comparation.js:45` — `query` kosong setelah tipe diambil.
pub(crate) const NO_QUERY: &str =
    "Ketik nama perangkat yang mau dicari!\nContoh: .bandingkan-device phone samsung s24";

/// `comparation.js:56` — tidak ada hasil.
pub(crate) fn not_found(query: &str) -> String {
    format!("Perangkat \"{query}\" tidak ditemukan.")
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

/// `data.data` yang lolos guard `!resData.data || resData.data.length === 0`.
///
/// Bentuk selain array lolos guard di JS (`.length` bukan nol), lalu
/// `resData.data[i].label` melempar TypeError di dalam `try` — jadi hasilnya
/// tetap sama: pesan "tidak ditemukan" untuk yang tidak bisa diindeks.
pub(crate) fn items_of(res: &Value) -> Option<Vec<Value>> {
    if !js_truthy(res.get("status")) {
        return None;
    }
    let items = res.get("data")?;
    match items {
        Value::Array(a) if !a.is_empty() => Some(a.clone()),
        Value::String(s) if s.is_empty() => None,
        Value::String(_) | Value::Object(_) | Value::Number(_) => Some(vec![items.clone()]),
        _ => None,
    }
}

/// `Math.min(resData.data.length, 10)` + daftar berlabel.
pub(crate) fn render_list(query: &str, items: &[Value]) -> String {
    let max = items.len().min(10);
    let mut txt = format!("📊 *PENCARIAN DEVICE 1: {}*\n\n", query.to_uppercase());
    txt.push_str("Silahkan Pilih yang lebih spesifik:\n\n");
    for (i, item) in items.iter().take(max).enumerate() {
        txt.push_str(&format!(
            "*{}.* {}\n",
            i + 1,
            interp(item.get("label"))
        ));
    }
    txt.push_str(
        "\n> 💡 *Kirim angka (contoh: 1)* untuk memilih perangkat pertama, atau ketik `batal` untuk membatalkan.",
    );
    txt
}

/// Sesi yang ditulis ke `user.compare_session` (`comparation.js:76-83`).
pub(crate) fn session_value(device_type: &str, items: &[Value], time_ms: i64) -> Value {
    serde_json::json!({
        "step": 1,
        "type": device_type,
        "results": items,
        "device1": Value::Null,
        "time": time_ms,
    })
}

#[plugin(
    name = "advance-comparation",
    alias = ["bandingkan-device", "compare"],
    category = "search",
    description = "Bandingkan dua perangkat (smartphone, tablet, laptop, dll)",
    usage = ".bandingkan-device [type] <query>",
    example = ".bandingkan-device phone samsung s24",
    cooldown = 5,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn advance_comparation(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help());
    }

    // `type` hanya ikut kalau `args[0]` ada di `VALID_TYPES`; kalau tidak,
    // `query` tetap seluruh `m.text` (termasuk kata "phone" yang tidak valid).
    let first = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let (device_type, query) = if VALID_TYPES.contains(&first.as_str()) {
        (
            first.clone(),
            ctx.args.iter().skip(1).cloned().collect::<Vec<_>>().join(" "),
        )
    } else {
        ("phone".to_string(), ctx.text.clone())
    };

    if query.trim().is_empty() {
        return Reply::text(NO_QUERY);
    }

    let Some(resp) = ctx.http.get(&search_url(&query, &device_type)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let res = resp.json().unwrap_or(Value::Null);

    let Some(items) = items_of(&res) else {
        return Reply::text(not_found(&query)).with_reaction("❌");
    };
    let kept = &items[..items.len().min(10)];

    Reply::text(render_list(&query, kept))
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "compare_session": session_value(&device_type, kept, now_ms()) }),
        })
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
            command: "bandingkan-device".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("advance-comparation").unwrap().handler)(ctx)
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

    fn session_of(r: &Reply) -> Value {
        match r.mutations().first() {
            Some(Mutation::SetUser { jid, data }) => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.get("compare_session").cloned().expect("harus ada sesi")
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    const HASIL: &str = r#"{
        "status": true,
        "data": [
            {"label": "Samsung Galaxy S24", "name": "s24"},
            {"label": "Samsung Galaxy S23", "name": "s23"}
        ]
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("advance-comparation").unwrap();
        assert_eq!(p.meta.name, "advance-comparation");
        assert_eq!(p.meta.alias, ["bandingkan-device", "compare"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Bandingkan dua perangkat (smartphone, tablet, laptop, dll)"
        );
        assert_eq!(p.meta.usage, ".bandingkan-device [type] <query>");
        assert_eq!(p.meta.example, ".bandingkan-device phone samsung s24");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["bandingkan-device", "compare"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "advance-comparation",
                "alias {a}"
            );
        }
    }

    #[test]
    fn no_arguments_lists_the_available_types() {
        let http = Stub::empty();
        let out = body_of(&run(http.clone(), ""));
        assert!(out.starts_with("📊 *ADVANCE COMPARATION*\n\n"), "{out}");
        assert!(out.contains("> *.bandingkan-device <tipe> <query>*"), "{out}");
        for t in VALID_TYPES {
            assert!(out.contains(&format!("> - {t}\n")), "tipe {t} hilang: {out}");
        }
        assert!(
            out.ends_with("*Contoh:* .bandingkan-device phone samsung s24"),
            "{out}"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_known_type_is_split_off_the_query() {
        let http = Stub::new(HASIL);
        let r = run(http.clone(), "phone samsung s24");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/compare-search?q=samsung%20s24&type=phone&apikey="
        );
        let session = session_of(&r);
        assert_eq!(session["step"], 1);
        assert_eq!(session["type"], "phone");
        assert_eq!(session["device1"], Value::Null);
        assert_eq!(session["results"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            session["results"][0]["name"],
            Value::from("s24"),
            "item asli harus utuh, bukan hanya label"
        );
        assert!(session["time"].as_i64().is_some_and(|t| t > 0));
    }

    #[test]
    fn an_unknown_first_word_stays_part_of_the_query() {
        let http = Stub::new(HASIL);
        let _ = run(http.clone(), "smartphone samsung s24");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/compare-search?q=smartphone%20samsung%20s24&type=phone&apikey="
        );

        let http = Stub::new(HASIL);
        let _ = run(http.clone(), "GPU rtx 3060");
        assert!(
            http.seen.lock().unwrap()[0].contains("&type=gpu&"),
            "tipe dikenal huruf besar: {}",
            http.seen.lock().unwrap()[0]
        );
    }

    #[test]
    fn a_type_without_a_query_prints_the_hint() {
        let http = Stub::empty();
        let r = run(http.clone(), "laptop");
        assert_eq!(body_of(&r), NO_QUERY);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_list_is_numbered_uppercased_and_capped_at_ten() {
        let items: Vec<Value> = (0..14)
            .map(|i| serde_json::json!({"label": format!("Device {i}")}))
            .collect();
        let res = serde_json::json!({"status": true, "data": items});
        let items = items_of(&res).unwrap();
        let out = render_list("s24 pro", &items[..items.len().min(10)]);
        assert!(
            out.starts_with("📊 *PENCARIAN DEVICE 1: S24 PRO*\n\nSilahkan Pilih yang lebih spesifik:\n\n"),
            "{out}"
        );
        assert!(out.contains("*1.* Device 0\n"), "{out}");
        assert!(out.contains("*10.* Device 9\n"), "{out}");
        assert!(!out.contains("Device 10"), "hasil ke-11 terpotong: {out}");
        assert!(out.ends_with("untuk memilih perangkat pertama, atau ketik `batal` untuk membatalkan."), "{out}");
    }

    #[test]
    fn a_missing_label_is_interpolated_as_undefined() {
        let res = serde_json::json!({"status": true, "data": [{"name": "x"}]});
        let items = items_of(&res).unwrap();
        assert!(render_list("q", &items).contains("*1.* undefined\n"));
    }

    #[test]
    fn an_empty_or_falsy_payload_reports_not_found() {
        for body in [
            r#"{"status": false, "data": [{"label": "A"}]}"#,
            r#"{"status": true, "data": []}"#,
            r#"{"status": true, "data": ""}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "data": null}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Stub::new(body), "samsung s24");
            assert_eq!(body_of(&r), not_found("samsung s24"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
            assert!(r.mutations().is_empty(), "tidak ada sesi: {body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "samsung s24");
        assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "bandingkan-device"));
        assert_eq!(reaction_of(&r), Some("☢"));
        assert!(r.mutations().is_empty(), "sesi tidak boleh ditulis saat gagal");
    }

    #[test]
    fn a_very_long_query_is_sent_whole() {
        let http = Stub::new(HASIL);
        let _ = run(http.clone(), &format!("phone {}", "c".repeat(4_000)));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"c".repeat(4_000)), "{}", seen[0].len());
    }
}