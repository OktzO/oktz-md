//! `plugins/search/chords.js` — cari chord gitar lewat `api.neoxr.eu`.
//!
//! `ponytail:` kunci API ada di `config.APIkey?.neoxr`, dan `Ctx` tidak
//! membawa config, jadi nilainya ikut `config.js` yang di-hardcode — persis
//! `search/android1.rs`. Selama router belum menyediakan config, cabang
//! "API key belum diset" (`explainFailure` + `MissingApiKeyError` dari
//! `src/lib/stalker-fallback.js`) yang jadi jalur utama; itu alasan jalur HTTP
//! ditutup dengan satu fungsi murni [`chord_text`] yang bisa diuji tanpa
//! jaringan dan tanpa key.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `explainFailure(new MissingApiKeyError('neoxr', 'APIKEY_NEOXR'), …)`
/// (`src/lib/stalker-fallback.js:29-36`) — cabang `MissingApiKeyError`.
pub(crate) fn missing_key(env_name: &str, command: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `{env_name}` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

/// `chords.js:45` — endpoint chord.
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/chord?q={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(text)
    )
}

/// `chords.js:27-33` — kartu saat `m.text` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎸 *ᴄʜᴏʀᴅs sᴇᴀʀᴄʜ*\n\n\
         > Cari chord/kunci gitar lagu\n\n\
         > Contoh:\n\
         `{prefix}chords komang`\n\
         `{prefix}chord perjalanan terindah`"
    )
}

/// `chords.js:51` — chord tidak ada.
pub(crate) fn not_found(text: &str) -> String {
    format!("❌ Chord tidak ditemukan untuk: `{text}`")
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

/// `!data?.status || !data?.data?.chord` (`chords.js:49`) — `None` berarti
/// cabang "tidak ditemukan".
///
/// `chord` yang bukan string jadi JSON, bukan `[object Object]` seperti di JS.
/// Teks string — satu-satunya bentuk yang dipakai API — tidak terpengaruh.
pub(crate) fn chord_text(data: &Value) -> Option<String> {
    if !js_truthy(data.get("status")) {
        return None;
    }
    let chord = data.get("data").and_then(|d| d.get("chord"));
    if !js_truthy(chord) {
        return None;
    }
    Some(interp(chord))
}

#[plugin(
    name = "chords",
    alias = ["chord", "kunci", "kuncigitar"],
    category = "search",
    description = "Cari chord/kunci gitar lagu",
    usage = ".chords <judul lagu>",
    example = ".chords komang",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn chords(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // Nexray tidak punya endpoint chord (dicek live di `chords.js:38`), jadi
    // tidak ada fallback: tanpa key plugin berhenti di sini.
    if NEOXR_APIKEY.is_empty() {
        return Reply::text(missing_key("APIKEY_NEOXR", &format!("{}chords", ctx.prefix)))
            .with_reaction("❌");
    }

    let Some(resp) = ctx.http.get(&api_url(text)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    match chord_text(&resp.json().unwrap_or(Value::Null)) {
        Some(chord) => Reply::text(chord).with_reaction("✅"),
        None => Reply::text(not_found(text)).with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new() -> Arc<Stub> {
            Arc::new(Stub {
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Err(oktz_plugin_api::HttpError {
                message: "offline".into(),
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
            command: "chords".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("chords").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("chords").unwrap();
        assert_eq!(p.meta.name, "chords");
        assert_eq!(p.meta.alias, ["chord", "kunci", "kuncigitar"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari chord/kunci gitar lagu");
        assert_eq!(p.meta.usage, ".chords <judul lagu>");
        assert_eq!(p.meta.example, ".chords komang");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["chord", "kunci", "kuncigitar"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "chords",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_blank_title_shows_the_usage_card_without_a_request() {
        let http = Stub::new();
        for text in ["", "   "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_missing_key_message_names_the_env_variable_and_the_command() {
        let http = Stub::new();
        let r = run(http.clone(), "komang");
        assert_eq!(body_of(&r), missing_key("APIKEY_NEOXR", ".chords"));
        assert_eq!(
            body_of(&r),
            "❌ *API Key belum diset!*\n\n\
             > Command `.chords` butuh `APIKEY_NEOXR` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty(), "tidak ada request");
    }

    #[test]
    fn the_endpoint_encodes_the_title_and_carries_the_key() {
        assert_eq!(
            api_url("komang"),
            "https://api.neoxr.eu/api/chord?q=komang&apikey="
        );
        assert_eq!(
            api_url("a b&c'd"),
            "https://api.neoxr.eu/api/chord?q=a%20b%26c'd&apikey="
        );
    }

    #[test]
    fn a_filled_chord_comes_back_unchanged() {
        let data = serde_json::json!({"status": true, "data": {"chord": "[C]Am\nAm  yang_ku"}});
        assert_eq!(
            chord_text(&data).as_deref(),
            Some("[C]Am\nAm  yang_ku"),
            "teks chord harus apa adanya"
        );
    }

    #[test]
    fn a_falsy_status_or_missing_chord_is_not_found() {
        for data in [
            serde_json::json!({"status": false, "data": {"chord": "[C]"}}),
            serde_json::json!({"status": true, "data": {}}),
            serde_json::json!({"status": true, "data": {"chord": ""}}),
            serde_json::json!({"status": true, "data": {"chord": null}}),
            serde_json::json!({"data": {"chord": "[C]"}}),
            serde_json::json!({"status": true}),
            Value::Null,
        ] {
            assert_eq!(chord_text(&data), None, "{data}");
        }
    }

    #[test]
    fn a_very_long_title_is_encoded_whole() {
        let title = "a".repeat(4_000);
        assert!(api_url(&title).contains(&"a".repeat(4_000)));
    }
}