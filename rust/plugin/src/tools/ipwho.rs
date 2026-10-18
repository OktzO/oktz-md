//! `plugins/tools/ipwho.js` — lookup IP lewat ipwho.is.
//! Ponytail: `Reply` tidak punya variant lokasi, jadi petak lokasi JSON
//! (latitude/longitude) diabaikan; hanya teks informasi yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴜɪ*\n\n> `{prefix}ipwho <ip>`\n\n> Contoh:\n> `{prefix}ipwho 8.8.8.8`",
        prefix = prefix
    )
}

const INVALID: &str = "❌ *ғᴏʀᴍᴀᴛ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Contoh: `8.8.8.8`";

fn valid_ip(ip: &str) -> bool {
    let mut ok = true;
    let mut n = 0;
    for (i, part) in ip.split('.').enumerate() {
        if i >= 4 { ok = false; break; }
        match part.parse::<u32>() {
            Ok(v) if v <= 255 => n += 1,
            _ => { ok = false; break; }
        }
    }
    ok && n == 4 && !ip.contains("..")
}

#[plugin(
    name = "ipwho",
    alias = ["ip", "iplookup", "ipinfo"],
    category = "tools",
    description = "Lookup informasi IP address",
    usage = ".ipwho <ip>",
    example = ".ipwho 8.8.8.8",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ipwho(ctx: Ctx) -> Reply {
    let ip = match ctx.args.first() {
        Some(a) => a.trim(),
        None => "",
    };
    if ip.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    if !valid_ip(ip) {
        return Reply::text(INVALID);
    }

    let url = format!("https://ipwho.is/{ip}");
    let Some(data) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        // `fetch` gagal ATAU `res.json()` gagal (badan kosong/bukan JSON)
        // sama-sama masuk `catch` JS (`ipwho.js:40-41`, `ipwho.js:87-90`).
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢");
    };

    if !data.get("success").is_some_and(|v| v.as_bool().unwrap_or(false)) {
        return Reply::text(format!("❌ *ɪᴘ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> IP {ip} tidak valid")).with_reaction("❌");
    }

    let conn = data.get("connection").unwrap_or(&serde_json::Value::Null);
    let sec = data.get("security").unwrap_or(&serde_json::Value::Null);
    let yes = |v: &serde_json::Value| if v.as_bool().unwrap_or(false) { "✅ Yes" } else { "❌ No" };
    let caption = format!(
        "🌐 *ɪᴘ ʟᴏᴏᴋᴜᴘ*\n\n╭┈┈⬡「 📍 *ʟᴏᴋᴀsɪ* 」\n┃ 🔢 IP: {}\n┃ 🌍 Country: {} {}\n┃ 🏙️ City: {}\n┃ 📍 Region: {}\n┃ 🌐 Continent: {}\n┃ 📮 Postal: {}\n┃ ⏰ Timezone: {}\n╰┈┈┈┈┈┈┈┈⬡\n\n╭┈┈⬡「 🔌 *ᴋᴏɴᴇᴋsɪ* 」\n┃ 🏢 ISP: {}\n┃ 🌐 ORG: {}\n┃ 📡 ASN: {}\n╰┈┈┈┈┈┈┈┈⬡\n\n╭┈┈⬡「 🛡️ *sᴇᴄᴜʀɪᴛʏ* 」\n┃ 🔒 VPN: {}\n┃ 🌐 Proxy: {}\n┃ 🤖 Tor: {}\n╰┈┈┈┈┈┈┈┈⬡",
        data.get("ip").and_then(Value::as_str).unwrap_or(ip),
        data.get("country").and_then(Value::as_str).unwrap_or("-"),
        data.get("country_code").and_then(Value::as_str).unwrap_or("-"),
        data.get("city").and_then(Value::as_str).unwrap_or("-"),
        data.get("region").and_then(Value::as_str).unwrap_or("-"),
        data.get("continent").and_then(Value::as_str).unwrap_or("-"),
        data.get("postal").and_then(Value::as_str).unwrap_or("-"),
        data.get("timezone").and_then(|x| x.get("id")).and_then(Value::as_str).unwrap_or("-"),
        conn.get("isp").and_then(Value::as_str).unwrap_or("-"),
        conn.get("org").and_then(Value::as_str).unwrap_or("-"),
        conn.get("asn").and_then(Value::as_str).unwrap_or("-"),
        yes(sec.get("vpn").unwrap_or(&serde_json::Value::Null)),
        yes(sec.get("proxy").unwrap_or(&serde_json::Value::Null)),
        yes(sec.get("tor").unwrap_or(&serde_json::Value::Null)),
    );
    Reply::text(caption).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Mutex<Option<String>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: Value) -> Arc<Stub> {
            Arc::new(Stub { body: Mutex::new(Some(v.to_string())), seen: Mutex::new(Vec::new()) })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub { body: Mutex::new(None), seen: Mutex::new(Vec::new()) })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let teks = self.body.lock().unwrap().clone().unwrap_or_default();
            Ok(HttpResponse { status: 200, body: HttpBody::Bytes { data: Arc::from(teks.into_bytes()), mimetype: None }, headers: Vec::new() })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> { unimplemented!() }
    }

    use oktz_plugin_api::HttpError;

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx { text: text.to_string(), args: args.by_ref().map(str::to_string).collect(), sender: "x".into(), chat: "x".into(), prefix: ".".into(), command: "ipwho".into(), http, ..Ctx::default() };
        ipwho(ctx)
    }

    fn text_of(r: &Reply) -> String { r.without_mutations().text_body().unwrap_or_default().to_string() }
    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() { Reply::Reaction { emoji, .. } => Some(emoji.clone()), _ => None }
    }

    #[test]
    fn ip_kosong_tampilkan_pakainya() {
        let http = Stub::json(serde_json::json!({}));
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), usage("."));
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn ip_tidak_valid() {
        let http = Stub::json(serde_json::json!({}));
        let r = run(http.clone(), "999.999.999.999");
        assert_eq!(text_of(&r), INVALID);
        assert!(http.seen.lock().unwrap().is_empty());
        let r = run(http, "abc");
        assert_eq!(text_of(&r), INVALID);
    }

    #[test]
    fn berhasil_tampilkan_info() {
        let http = Stub::json(serde_json::json!({
            "success": true, "ip": "8.8.8.8", "country": "United States", "country_code": "US",
            "city": "Mountain View", "region": "California", "continent": "North America",
            "postal": "94040", "timezone": {"id": "America/Los_Angeles"},
            "connection": {"isp": "Google LLC", "org": "Google", "asn": "AS15169"},
            "security": {"vpn": false, "proxy": false, "tor": false}
        }));
        let r = run(http.clone(), "8.8.8.8");
        let t = text_of(&r);
        assert!(t.contains("🔢 IP: 8.8.8.8"), "{t}");
        assert!(t.contains("🌍 Country: United States US"), "{t}");
        assert!(t.contains("🏢 ISP: Google LLC"), "{t}");
        assert!(t.contains("🔒 VPN: ❌ No"), "{t}");
        assert_eq!(reaction_of(&r), Some("✅".to_string()));
        assert_eq!(http.seen.lock().unwrap().as_slice(), &["https://ipwho.is/8.8.8.8"]);
    }

    #[test]
    fn ip_tidak_ditemukan() {
        let http = Stub::json(serde_json::json!({ "success": false }));
        let r = run(http, "1.2.3.4");
        assert_eq!(text_of(&r), "❌ *ɪᴘ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> IP 1.2.3.4 tidak valid");
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn gagal_jaringan() {
        let r = run(Stub::offline(), "8.8.8.8");
        assert!(text_of(&r).contains("lagi ada kendala"));
        assert_eq!(reaction_of(&r), Some("☢".to_string()));
    }
}
