//! `plugins/stalker/robloxstalk.js` — stalk akun Roblox lewat beberapa endpoint
//! Roblox API secara berurutan.
//! Ponytail: `toLocaleString` angka tidak direplikasi (angka ditulis polos),
//! dan `created` ditulis mentah karena `Ctx` tidak punya parser tanggal ISO
//! bawaan (ponytail: tambahkan field tanggal ke `Ctx` kalau diperlukan).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::{json, Value};

const NOT_FOUND: &str = "❌ Username *{username}* tidak ditemukan";

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎮 *ʀᴏʙʟᴏx sᴛᴀʟᴋ*\n\n> Masukkan username Roblox\n\n`Contoh: {prefix}robloxstalk Linkmon99`",
        prefix = prefix
    )
}

fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for seg in path {
        cur = cur.get(*seg)?;
    }
    Some(cur)
}

fn num(v: Option<&Value>) -> i64 {
    v.and_then(Value::as_i64).unwrap_or(0)
}

fn top_groups(groups: &Value) -> String {
    let daftar = groups.get("data").and_then(Value::as_array).unwrap_or(&Vec::new());
    let mut out = String::new();
    for g in daftar.iter().take(5) {
        let nama = g.get("group").and_then(|x| x.get("name")).and_then(Value::as_str).unwrap_or("-");
        let anggota = num(g.get("group").and_then(|x| x.get("memberCount")));
        let peran = g.get("role").and_then(|x| x.get("name")).and_then(Value::as_str).unwrap_or("-");
        out.push_str(&format!("  ◦ {nama} ({anggota} members) — {peran}\n"));
    }
    if out.is_empty() { out.push_str("  ◦ Tidak ada\n"); }
    out
}

fn top_games(games: &Value) -> String {
    let daftar = games.get("data").and_then(Value::as_array).unwrap_or(&Vec::new());
    let mut out = String::new();
    for g in daftar.iter().take(5) {
        let nama = g.get("name").and_then(Value::as_str).unwrap_or("-");
        let kunjungan = num(g.get("placeVisits"));
        out.push_str(&format!("  ◦ {nama} ({kunjungan} visits)\n"));
    }
    if out.is_empty() { out.push_str("  ◦ Tidak ada\n"); }
    out
}

fn top_badges(badges: &Value) -> String {
    let daftar = badges.get("data").and_then(Value::as_array).unwrap_or(&Vec::new());
    let mut out = String::new();
    for b in daftar.iter().take(5) {
        let nama = b.get("name").and_then(Value::as_str).unwrap_or("-");
        let awarded = b.get("statistics").and_then(|x| x.get("awardedCount")).and_then(Value::as_i64).unwrap_or(0);
        out.push_str(&format!("  ◦ {nama} ({awarded} awarded)\n"));
    }
    if out.is_empty() { out.push_str("  ◦ Tidak ada\n"); }
    out
}

fn top_inventory(inventory: &Value) -> String {
    if !inventory.is_array() {
        return format!("  ◦ {}\n", inventory.as_str().unwrap_or("private / tidak tersedia"));
    }
    let daftar = inventory.as_array().unwrap();
    let mut out = String::new();
    for it in daftar.iter().take(5) {
        let nama = it.get("name").and_then(Value::as_str).unwrap_or("-");
        let rap = num(it.get("recentAveragePrice"));
        out.push_str(&format!("  ◦ {nama} (RAP: {rap})\n"));
    }
    if out.is_empty() { out.push_str("  ◦ Tidak ada\n"); }
    out
}

fn presence_text(presence: &Value) -> String {
    let tipe = num(presence.get("userPresenceType"));
    let label = match tipe {
        0 => "Offline",
        1 => "Online",
        2 => "In Game",
        3 => "In Studio",
        _ => "",
    };
    let loc = presence.get("lastLocation").and_then(Value::as_str).unwrap_or("-");
    let place = presence.get("placeId").and_then(Value::as_str).unwrap_or("-");
    let game = presence.get("gameId").and_then(Value::as_str).unwrap_or("-");
    if label.is_empty() {
        format!("tidak tersedia\n")
    } else {
        format!("Status: {label}\n  Last Location: {loc}\n  PlaceId: {place}\n  GameId: {game}\n")
    }
}

#[plugin(
    name = "robloxstalk",
    alias = ["rblxstalk", "rbxstalk", "stalkroblox", "stalkrbx"],
    category = "stalker",
    description = "Stalk akun Roblox berdasarkan username",
    usage = ".robloxstalk <username>",
    example = ".robloxstalk Linkmon99",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn robloxstalk(ctx: Ctx) -> Reply {
    let username = match ctx.args.first() {
        Some(u) => u.trim(),
        None => "",
    };
    if username.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let http = &*ctx.http;
    let search_url = format!("https://users.roblox.com/v1/users/search?keyword={username}&limit=10");
    let search = match http.get(&search_url).and_then(|r| r.json().ok_or_else(|| oktz_plugin_api::HttpError { message: "bukan JSON".into() })) {
        Ok(v) => v,
        Err(_) => return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    };
    let data = match search.get("data").and_then(Value::as_array) {
        Some(d) if !d.is_empty() => d,
        _ => return Reply::text(NOT_FOUND.replace("{username}", username)).with_reaction("❌"),
    };
    let user = &data[0];
    let userId = num(user.get("id"));

    let detail_url = format!("https://users.roblox.com/v1/users/{userId}");
    let detail = http.get(&detail_url).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let avatar = http.get(&format!("https://thumbnails.roblox.com/v1/users/avatar?userIds={userId}&size=420x420&format=Png")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let followers = http.get(&format!("https://friends.roblox.com/v1/users/{userId}/followers/count")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let following = http.get(&format!("https://friends.roblox.com/v1/users/{userId}/followings/count")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let friends = http.get(&format!("https://friends.roblox.com/v1/users/{userId}/friends/count")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let groups = http.get(&format!("https://groups.roblox.com/v2/users/{userId}/groups/roles")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let games = http.get(&format!("https://games.roblox.com/v2/users/{userId}/games?limit=50")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let badges = http.get(&format!("https://badges.roblox.com/v1/users/{userId}/badges?limit=50")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);
    let inventory = http.get(&format!("https://inventory.roblox.com/v1/users/{userId}/assets/collectibles?limit=50")).ok().and_then(|r| r.json()).unwrap_or(Value::Null);

    let presence = http.post_json("https://presence.roblox.com/v1/presence/users", &serde_json::json!({ "userIds": [userId] })).ok().and_then(|r| r.json()).unwrap_or(Value::Null);

    let verified = user.get("hasVerifiedBadge").is_some_and(|v| v.as_bool().unwrap_or(false));
    let created = detail.get("created").and_then(Value::as_str).unwrap_or("-");
    let display_name = detail.get("displayName").and_then(Value::as_str).unwrap_or("-");
    let desc = detail.get("description").and_then(Value::as_str).unwrap_or("-");
    let avatar_url = avatar.get("data").and_then(|a| a.first()).and_then(|x| x.get("imageUrl")).and_then(Value::as_str).unwrap_or("");
    let nama = user.get("name").and_then(Value::as_str).unwrap_or(username);

    let caption = format!(
        "🎮 *ʀᴏʙʟᴏx sᴛᴀʟᴋ*\n\n*PROFILE*\n🆔 *ID*: {id}\n🎄 *Username*: {nama}\n📛 *Display*: {display}\n✅ *Verified*: {verified}\n📅 *Created*: {created}\n\n*SOCIAL*\n👥 *Friends*: {friends}\n👤 *Followers*: {followers}\n➕ *Following*: {following}\n\n*PRESENCE*\n{presence}\n\n📝 *Bio:*\n{bio}\n👥 *Groups* ({gcount}):\n{groups}🎮 *Games* ({gamest}):\n{games}🏆 *Badges* ({badgest}):\n{badges}🎒 *Inventory*:\n{inventory}🔗 https://roblox.com/users/{id}/profile",
        id = userId,
        nama = nama,
        display = display_name,
        verified = if verified { "Ya" } else { "Tidak" },
        created = created,
        friends = num(friends.get("count")),
        followers = num(followers.get("count")),
        following = num(following.get("count")),
        presence = presence.get("userPresences").and_then(|a| a.first()).map_or("tidak tersedia".to_string(), |p| presence_text(p)),
        bio = if desc.is_empty() { "-".to_string() } else { desc.chars().take(300).collect() },
        gcount = groups.get("data").and_then(Value::as_array).map_or(0, |a| a.len()),
        groups = top_groups(&groups),
        gamest = games.get("data").and_then(Value::as_array).map_or(0, |a| a.len()),
        games = top_games(&games),
        badgest = badges.get("data").and_then(Value::as_array).map_or(0, |a| a.len()),
        badges = top_badges(&badges),
        inventory = top_inventory(&inventory),
    );

    if !avatar_url.is_empty() {
        return Reply::image(MediaSource::url(avatar_url), caption).with_reaction("✅");
    }
    Reply::text(caption).with_reaction("✅")
}

use oktz_plugin_api::HttpError;

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// Balasan 11 permintaan secara berurutan; setelah habis, balasan terakhir dipakai lagi.
    #[derive(Debug, Default)]
    struct Stub {
        balasan: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
        hitung: Mutex<usize>,
    }

    impl Stub {
        fn balasan(daftar: Vec<&str>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(daftar.iter().map(|s| (*s).to_string()).collect()),
                seen: Mutex::new(Vec::new()),
                hitung: Mutex::new(0),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut h = self.hitung.lock().unwrap();
            let b = self.balasan.lock().unwrap();
            let i = (*h).min(b.len().saturating_sub(1));
            *h += 1;
            drop(h);
            drop(b);
            let teks = self.balasan.lock().unwrap()[i].clone();
            Ok(HttpResponse { status: 200, body: HttpBody::Bytes { data: Arc::from(teks.into_bytes()), mimetype: None }, headers: Vec::new() })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            let teks = self.balasan.lock().unwrap().last().cloned().unwrap_or_default();
            Ok(HttpResponse { status: 200, body: HttpBody::Bytes { data: Arc::from(teks.into_bytes()), mimetype: None }, headers: Vec::new() })
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> { unimplemented!() }
    }

    fn run(text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let http = Stub::balasan(vec![
            // search
            r#"{"data":[{"id":12345,"name":"Linkmon99","hasVerifiedBadge":true}]}"#,
            // detail
            r#"{"id":12345,"name":"Linkmon99","displayName":"Linkmon","description":"Halo saya","created":"2026-01-01T00:00:00Z"}"#,
            // avatar
            r#"{"data":[{"imageUrl":"https://cdn/avatar.png"}]}}"#,
            // followers/following/friends
            r#"{"count":100}"#, r#"{"count":50}"#, r#"{"count":20}"#,
            // groups/games/badges/inventory
            r#"{"data":[{"group":{"name":"G1","memberCount":10},"role":{"name":"Member"}}]}"#,
            r#"{"data":[{"name":"Game1","placeVisits":1000}]}"#,
            r#"{"data":[{"name":"Badge1","statistics":{"awardedCount":5}}]}"#,
            r#"{"data":[{"name":"Item1","recentAveragePrice":100}]}"#,
            // presence
            r#"{"userPresences":[{"userPresenceType":1,"lastLocation":"Lobby","placeId":"1","gameId":"2"}]}"#,
        ]);
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "robloxstalk".into(),
            http,
            ..Ctx::default()
        };
        robloxstalk(ctx)
    }

    fn text_of(r: &Reply) -> String { r.without_mutations().text_body().unwrap_or_default().to_string() }
    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() { Reply::Reaction { emoji, .. } => Some(emoji.clone()), _ => None }
    }

    #[test]
    fn username_kosong_tampilkan_bantuan() {
        let r = run("");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn berhasil_mengambil_profil_dengan_avatar() {
        let r = run("Linkmon99");
        let t = text_of(&r);
        assert!(t.contains("🆔 *ID*: 12345"), "{t}");
        assert!(t.contains("🎄 *Username*: Linkmon99"), "{t}");
        assert!(t.contains("✅ *Verified*: Ya"), "{t}");
        assert!(t.contains("👥 *Groups* (1):"), "{t}");
        assert!(t.contains("🎒 *Inventory*:"), "{t}");
        assert_eq!(reaction_of(&r), Some("✅".to_string()));
        match r.without_mutations() {
            Reply::Image { media, .. } => match media {
                Some(oktz_plugin_api::MediaSource::Url { url, .. }) => assert_eq!(url, "https://cdn/avatar.png"),
                m => panic!("harus URL avatar, dapat {m:?}"),
            },
            other => panic!("harus image, dapat {other:?}"),
        }
    }

    #[test]
    fn username_tidak_ditemukan() {
        let http = Stub::balasan(vec![r#"{"data":[]}"#]);
        let mut args = "Nobody123".split_whitespace();
        let ctx = Ctx { text: "Nobody123".into(), args: args.by_ref().map(str::to_string).collect(), sender: "x".into(), chat: "x".into(), prefix: ".".into(), command: "robloxstalk".into(), http, ..Ctx::default() };
        let r = robloxstalk(ctx);
        assert_eq!(text_of(&r), "❌ Username *Nobody123* tidak ditemukan");
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }
}
