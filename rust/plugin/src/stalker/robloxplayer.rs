//! `plugins/stalker/robloxplayer.js` — cari pemain Roblox lewat API Neoxr.
//!
//! Kunci `config.APIkey.neoxr` dibaca dari `ctx.config()`.
//! Endpoint tunggal: `GET https://api.neoxr.eu/api/roblox-search?q=…&apikey=…`
//! (docs/rust-rewrite/ENDPOINTS.md §2).
//!
//! `ponytail:` JS bereaksi dua kali (`🔍` di awal, `✅`/`❌` di akhir).
//! `Reply::with_reaction` hanya boleh satu emoji, jadi hanya reaksi akhir yang
//! dibawa — pola yang sama sudah dicatat di `ytstalk.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;
use crate::tools::phisingataubukan::encode_uri_component;

pub(crate) const ENDPOINT: &str = "https://api.neoxr.eu/api/roblox-search";

/// `ytstalk`-style: kartu pakai saat `m.args[0]` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎮 *ʀᴏʙʟᴏx ᴘʟᴀʏᴇʀ sᴇᴀʀᴄʜ*\n\n> Masukkan username untuk dicari\n\n`{prefix}robloxplayer linkmon`"
    )
}

pub(crate) const NOT_FOUND: &str = "❌ Tidak ditemukan player dengan username: {query}";

/// `res.data.data[i].previousUsernames` — array string, digabung koma.
fn previous_usernames(value: &Value) -> Option<String> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return None;
    }
    Some(
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn caption(data: &Value, query: &str) -> String {
    let players = data
        .as_array()
        .map(|a| a.iter().take(10).collect::<Vec<_>>())
        .unwrap_or_default();

    let mut text = format!(
        "🎮 *ʀᴏʙʟᴏx ᴘʟᴀʏᴇʀ sᴇᴀʀᴄʜ*\n\n> Query: `{query}`\n> Ditemukan: *{}* player\n\n",
        players.len()
    );
    for (i, p) in players.iter().enumerate() {
        let display = p
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("");
        let name = p.get("name").and_then(Value::as_str).unwrap_or("");
        let id = p.get("id").and_then(Value::as_str).unwrap_or("");
        let verified = if p
            .get("hasVerifiedBadge")
            .is_some_and(js_truthy_value)
        {
            "Ya"
        } else {
            "Tidak"
        };
        text.push_str(&format!(
            "╭┈┈⬡「 {}. *{}* 」\n┃ 🆔 ID: `{}`\n┃ 👤 Username: `{}`\n┃ 📛 Display: *{}*\n┃ ✅ Verified: {}\n",
            i + 1,
            display,
            id,
            name,
            display,
            verified
        ));
        if let Some(prev) = previous_usernames(&p["previousUsernames"]) {
            text.push_str(&format!("┃ 📜 Previous: {prev}\n"));
        }
        text.push_str("╰┈┈⬡\n\n");
    }
    text.push_str(
        "> _Gunakan `.robloxstalk <username>` untuk info detail_",
    );
    text
}

#[plugin(
    name = "robloxplayer",
    alias = ["robloxsearch", "searchroblox", "robloxfind"],
    category = "stalker",
    description = "Search Roblox player by username",
    usage = ".robloxplayer <username>",
    example = ".robloxplayer linkmon",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn robloxplayer(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = format!(
        "{ENDPOINT}?q={}&apikey={}",
        encode_uri_component(query),
        api_key
    );

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text("❌ Gagal mencari player Roblox.").with_reaction("❌");
    };
    let body = res.json().unwrap_or(Value::Null);

    let status_ok = body
        .get("status")
        .is_some_and(js_truthy_value);
    let Some(data) = body
        .get("data")
        .filter(|v| status_ok && !v.is_null() && v.as_array().is_some_and(|a| !a.is_empty()))
    else {
        return Reply::text(NOT_FOUND.replace("{query}", query))
            .with_reaction("❌");
    };

    Reply::text(caption(data, query)).with_reaction("✅")
}

#[cfg(test)]
use serde_json::json;

#[cfg(test)]
fn contoh_data() -> Value {
    serde_json::json!({
        "status": true,
        "data": [
            {
                "displayName": "Linkmon99",
                "id": "123456",
                "name": "linkmon99",
                "hasVerifiedBadge": true,
                "previousUsernames": ["old1", "old2"]
            },
            {
                "displayName": "Builderman",
                "id": "789012",
                "name": "builderman",
                "hasVerifiedBadge": false,
                "previousUsernames": []
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
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
        fn offline() -> Arc<Stub> {
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

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "robloxplayer".into(),
            http,
            ..Ctx::default()
        };
        robloxplayer(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "robloxplayer".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    neoxr: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        robloxplayer(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_query_menghasilkan_kartu_pakai() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_masih_mengirim_request() {
        // `Ctx::default()` punya config kosong, jadi apikey kosong — tetap
        // memanggil endpoint (pengambil keputusan ada di API, bukan sini).
        let http = Stub::body(contoh_data());
        let r = run(http.clone(), "linkmon");
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &[format!("{ENDPOINT}?q=linkmon&apikey=")]
        );
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn username_di_encode_dan_dikirim_dengan_kunci() {
        let http = Stub::body(contoh_data());
        let r = run_dengan_kunci(http.clone(), "linkmon99");
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &[format!("{ENDPOINT}?q=linkmon99&apikey=KUNCI")]
        );
        assert_eq!(text_of(&r), caption(&contoh_data()["data"], "linkmon99"));
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn status_false_atau_data_kosong_menghasilkan_tidak_ditemukan() {
        for body in [
            serde_json::json!({ "status": false, "data": [] }),
            serde_json::json!({ "status": true, "data": null }),
            serde_json::json!({ "status": true }),
            Value::Null,
            serde_json::json!({ "status": true, "data": [] }),
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "nope");
            assert_eq!(
                text_of(&r),
                "❌ Tidak ditemukan player dengan username: nope",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn gagal_jaringan_membalaskan_pesan_gagal() {
        let r = run_dengan_kunci(Stub::offline(), "linkmon");
        assert_eq!(text_of(&r), "❌ Gagal mencari player Roblox.");
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn previous_usernames_digabung_koma() {
        let p = serde_json::json!({
            "displayName": "X",
            "id": "1",
            "name": "x",
            "hasVerifiedBadge": false,
            "previousUsernames": ["a", "b", "c"]
        });
        assert_eq!(
            previous_usernames(&p["previousUsernames"]),
            Some("a, b, c".to_string())
        );
        let kosong = serde_json::json!({
            "previousUsernames": []
        });
        assert_eq!(previous_usernames(&kosong["previousUsernames"]), None);
        // `previousUsernames?.length > 0`: array kosong -> baris tidak dicetak.
        assert_eq!(previous_usernames(&Value::Null), None);
    }

    #[test]
    fn ambil_maksimal_10_pemain() {
        let mut data = vec![];
        for i in 0..15 {
            data.push(serde_json::json!({
                "displayName": format!("P{i}"),
                "id": i.to_string(),
                "name": format!("p{i}"),
                "hasVerifiedBadge": false,
                "previousUsernames": []
            }));
        }
        let body = serde_json::json!({ "status": true, "data": data });
        let r = run_dengan_kunci(Stub::body(body), "q");
        let t = text_of(&r);
        // Harusnya ada P0..P9 (10 pemain), tidak ada P10..P14.
        for i in 0..10 {
            assert!(t.contains(&format!("{}.", i + 1)), "{t}");
        }
        assert!(!t.contains("11."), "{t}");
    }
}
