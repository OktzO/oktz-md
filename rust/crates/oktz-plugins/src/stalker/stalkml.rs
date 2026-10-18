//! `plugins/stalker/stalkml.js` — stalk nickname Mobile Legends.
//! Ponytail: `m.pushName` tidak ada di `Ctx`, tidak pakai; JS bereaksi 🕕 lalu
//! ✅/❌, Rust hanya satu reaksi (akhir).

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const REGION: &[(&str, &str)] = &[
    ("1", "Indonesia"), ("2", "Indonesia"), ("3", "Indonesia"), ("4", "Indonesia"),
    ("5", "Indonesia"), ("6", "Indonesia"), ("7", "Indonesia"), ("8", "Indonesia"),
    ("9", "Indonesia"), ("10", "Malaysia / SG / BN"), ("11", "Philippines"),
    ("12", "Thailand"), ("13", "Vietnam"), ("14", "Cambodia"), ("15", "Myanmar"),
    ("16", "Laos"), ("17", "Timor-Leste"), ("19", "Middle East"), ("20", "North America"),
    ("21", "Europe"), ("22", "South America"),
];

fn get_region(sid: &str) -> &'static str {
    let s = sid;
    REGION
        .iter()
        .find(|(k, _)| s.starts_with(k))
        .map(|(_, v)| *v)
        .unwrap_or("Unknown")
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎮 *STALKER MOBILE LEGENDS* 🎮\n\nFitur ini akan membantumu untuk melacak dan mengetahui *nickname* atau nama dari akun Mobile Legends seseorang hanya dengan menggunakan *ID* dan *Server* nya saja!\n\n*CARA PENGGUNAAN:*\n- Ketik `{prefix}stalkml <ID> | <Server>`\n- Contoh: `{prefix}stalkml 1264042367 | 15139`\n\n_Pemisahan ID dan Server juga bisa menggunakan spasi atau format tanda kurung seperti 1264042367(15139)._",
        prefix = prefix
    )
}

fn parse_input(text: &str) -> (String, String) {
    let mut userId = String::new();
    let mut serverId = String::new();
    if text.contains('|') {
        let bagian: Vec<&str> = text.split('|').map(|v| v.trim()).collect();
        userId = bagian.first().copied().unwrap_or("").to_string();
        serverId = bagian.get(1).copied().unwrap_or("").to_string();
    } else if text.contains('(') && text.contains(')') {
        if let Some(cap) = regex_capture(text) {
            userId = cap.0;
            serverId = cap.1;
        } else {
            let angka: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
            if angka.len() > 4 {
                serverId = angka[angka.len() - 4..].to_string();
                userId = angka[..angka.len() - 4].to_string();
            } else {
                userId = angka;
            }
        }
    } else {
        let bagian: Vec<&str> = text.split_whitespace().filter(|v| !v.is_empty()).collect();
        if bagian.len() >= 2 {
            userId = bagian[0].to_string();
            serverId = bagian[1].to_string();
        } else {
            let angka: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
            if angka.len() > 4 {
                serverId = angka[angka.len() - 4..].to_string();
                userId = angka[..angka.len() - 4].to_string();
            } else {
                userId = angka;
            }
        }
    }
    (userId, serverId)
}

/// Padanan `(\d+)\s*\(\s*(\d+)\s*()` tanpa dependensi `regex`.
fn regex_capture(text: &str) -> Option<(String, String)> {
    let li = text.find('(')?;
    let ri = text[li..].find(')')?;
    let dalam = text[li + 1..li + ri].trim();
    let mut parts = dalam.split_whitespace();
    let a = parts.next()?;
    let b = parts.next()?;
    if parts.next().is_some() || !a.chars().all(|c| c.is_ascii_digit()) || !b.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((a.to_string(), b.to_string()))
}

#[plugin(
    name = "stalkml",
    alias = ["mlstalk", "ceknickml", "nickml"],
    category = "stalker",
    description = "Mengetahui nama/nickname akun Mobile Legends berdasarkan ID dan Server.",
    usage = ".stalkml <id> | <server>",
    example = ".stalkml 1264042367 | 15139",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stalkml(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    let (user_id, server_id) = parse_input(text);
    if user_id.is_empty() || server_id.is_empty() {
        return Reply::text(
            "❌ *FORMAT SALAH*\n\nPastikan kamu memasukkan ID dan Server yang lengkap.\nContoh: `.stalkml 1264042367 | 15139`"
        ).with_reaction("❌");
    }

    let url = format!("https://api.isan.eu.org/nickname/ml?id={user_id}&server={server_id}");
    let json = match ctx.http.get(&url) {
        Ok(r) => r.json().unwrap_or(Value::Null),
        Err(_) => {
            return Reply::text(
                "❌ *GAGAL MELACAK AKUN*\n\nMaaf, sistem sedang mengalami gangguan saat memanggil API untuk melacak akun tersebut. Silakan coba beberapa saat lagi."
            ).with_reaction("❌");
        }
    };

    if json.get("success").is_some_and(|v| v.as_bool().unwrap_or(false)) {
        let region = get_region(&server_id);
        let nama = json.get("name").and_then(Value::as_str).unwrap_or("-");
        let caption = format!(
            "🎮 *MOBILE LEGENDS STALKER* 🎮\n\nPencarian berhasil! Berikut adalah detail akun yang kamu lacak:\n\n👤 *Nickname:* {nama}\n🆔 *User ID:* {uid}\n🌐 *Server ID:* {sid}\n🗺️ *Region:* {region}",
            nama = nama, uid = user_id, sid = server_id, region = region
        );
        Reply::text(caption).with_reaction("✅")
    } else {
        Reply::text(
            format!("❌ *AKUN TIDAK DITEMUKAN*\n\nMaaf, sistem tidak dapat menemukan akun dengan ID *{user_id}* dan Server *{server_id}*. Pastikan ID dan Server sudah diketik dengan benar.")
        ).with_reaction("❌")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpGet, HttpResponse};
    use serde_json::json;
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
        let ctx = Ctx { text: text.to_string(), args: args.by_ref().map(str::to_string).collect(), sender: "x".into(), chat: "x".into(), prefix: ".".into(), command: "stalkml".into(), http, ..Ctx::default() };
        stalkml(ctx)
    }

    fn text_of(r: &Reply) -> String { r.without_mutations().text_body().unwrap_or_default().to_string() }
    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() { Reply::Reaction { emoji, .. } => Some(emoji.clone()), _ => None }
    }

    #[test]
    fn kosong_tampilkan_bantuan() {
        let r = run(Stub::json(serde_json::json!({})), "");
        assert!(text_of(&r).contains("STALKER MOBILE LEGENDS"));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn berhasil_mendapatkan_nickname() {
        let http = Stub::json(serde_json::json!({ "success": true, "name": "ZannPlayer" }));
        let r = run(http.clone(), "1264042367 | 15139");
        let t = text_of(&r);
        assert!(t.contains("👤 *Nickname:* ZannPlayer"), "{t}");
        assert!(t.contains("🆔 *User ID:* 1264042367"), "{t}");
        assert!(t.contains("🌐 *Server ID:* 15139"), "{t}");
        assert_eq!(reaction_of(&r), Some("✅".to_string()));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("id=1264042367&server=15139"), "{seen:?}");
    }

    #[test]
    fn format_salah_dengan_angka_gabung() {
        let http = Stub::json(serde_json::json!({ "success": false }));
        let r = run(http.clone(), "126404236715139");
        assert_eq!(text_of(&r), "❌ *AKUN TIDAK DITEMUKAN*\n\nMaaf, sistem tidak dapat menemukan akun dengan ID *1264042367* dan Server *15139*. Pastikan ID dan Server sudah diketik dengan benar.");
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn gagal_jaringan() {
        let r = run(Stub::offline(), "1 | 1");
        assert!(text_of(&r).contains("GAGAL MELACAK AKUN"));
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }
}
