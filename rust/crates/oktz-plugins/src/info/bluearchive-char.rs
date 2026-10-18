//! `plugins/info/bluearchive-char.js` — info character Blue Archive.
//!
//! `ponytail:`
//! 1. `contextInfo` `{forwardingScore, isForwarded, forwardedNewsletter…}`
//!    yang ditempel pada `sock.sendMessage` (`bluearchive-char.js:143-150`)
//!    tidak punya field di `Reply` — badge forwarded-newsletter hilang.
//!    `config.saluran?.id` / `config.saluran?.name` (`config.js:124-127`)
//!    hanya dipakai di `contextInfo` itu, jadi ikut tidak terjangkau.
//! 2. `char.bio.substring(0, 200)` dihitung per unit UTF-16 di JS; di sini
//!    per `char`, jadi batasnya bergeser untuk emoji.
//! 3. Reaksi `🕕` (`:91`) dan `✅` (`:159`) tidak bisa dua-duanya: satu
//!    `Reply` hanya membawa satu `Reaction`. Yang dipasang adalah reaksi
//!    cabang akhir.
//!
//! `list()` (`bluearchive-char.js:33-46`) juga menyatukan ulang `imgSmall`
//! dan `img` ke prefiks CDN, tapi `char()` hanya membaca `.url` dari hasil
//! itu — penyatuan di `list()` tidak pernah dipakai, jadi tidak ditulis.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;


use super::gempa::js_str;

const URL_LIST: &str = "https://api.dotgg.gg/bluearchive/characters";
const URL_CHAR: &str = "https://api.dotgg.gg/bluearchive/characters/";
const CDN: &str = "https://images.dotgg.gg/bluearchive/characters/";

/// Semua kegagalan di plugin ini berakhir di teks yang sama: `throw` apa pun
/// langsung ditangkap `catch` (`bluearchive-char.js:160-163`) yang hanya
/// memanggil `te(...)`, tanpa menyertakan `error.message`.
fn gagal(ctx: &Ctx) -> Reply {
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

/// `if (!name)` (`bluearchive-char.js:80`) — teks literal, bukan `te`.
fn cara_pakai(prefix: &str) -> String {
    format!(
        "🎮 *ʙʟᴜᴇ ᴀʀᴄʜɪᴠᴇ ᴄʜᴀʀᴀᴄᴛᴇʀ*\n\n\
         > Lihat info character Blue Archive\n\n\
         > *Contoh:*\n\
         > {prefix}bluearchive-char shiroko\n\
         > {prefix}bachar hoshino\n\
         > {prefix}ba aru"
    )
}

/// `BluArchive.findUrl` (`bluearchive-char.js:21-31`).
///
/// `urls.includes(clean)` dulu; kalau tidak, setiap kata harus muncul di
/// `url` dan yang pertama menang. `Err` = `null` di JS, artinya `char()`
/// melempar.
fn find_url(input: &str, urls: &[Value]) -> Result<String, ()> {
    // `input.toLowerCase().replace(/\s+/g, "_")` — satu rune whitespace
    // berurutan jadi satu `_`, termasuk di awal dan akhir string.
    let mut clean = String::with_capacity(input.len());
    let mut prev_ws = false;
    for c in input.to_lowercase().chars() {
        if c.is_whitespace() {
            if !prev_ws {
                clean.push('_');
                prev_ws = true;
            }
        } else {
            clean.push(c);
            prev_ws = false;
        }
    }

    if urls.iter().any(|u| u.as_str() == Some(clean.as_str())) {
        return Ok(clean);
    }

    let words: Vec<&str> = clean.split('_').collect();
    for url in urls {
        // `url.toLowerCase()` pada entri non-string melempar TypeError di JS,
        // jadi tidak bisa dilewati diam-diam.
        let Some(asli) = url.as_str() else {
            return Err(());
        };
        let low = asli.to_lowercase();
        if !words.iter().all(|w| low.contains(w)) {
            continue;
        }
        // `matches[0]` mengembalikan url apa adanya, bukan versi lowercase.
        return Ok(asli.to_string());
    }
    Err(())
}

/// `const urls = listc.map((c) => c.url)` — daftar `url` apa adanya.
fn urls_of(list: &Value) -> Option<Vec<Value>> {
    let arr = list.as_array()?;
    Some(arr.iter().map(|c| c.get("url").cloned().unwrap_or(Value::Null)).collect())
}

/// `char(...)` — `list()`, `findUrl()`, lalu `GET /characters/<url>`.
///
/// Satu-satunya teks "Character … tidak ditemukan" di JS
/// (`bluearchive-char.js:57-59`) tidak pernah tampil: `char()` melempar, dan
/// `catch` menggantinya dengan `te(...)`.
fn fetch_char(http: &dyn oktz_plugin_api::HttpGet, name: &str) -> Option<Value> {
    let list = http.get(URL_LIST).ok()?.json()?;
    // `data.map(..)` melempar kalau `data` bukan array — `urls_of` mengembalikan
    // `None` untuk kasus itu, persis seperti catch di `bluearchive-char.js:160`.
    let urls = urls_of(&list)?;
    let found = find_url(name, &urls).ok()?;
    // Sebaliknya `...data` di `:66` tidak melempar untuk non-objek: hasilnya
    // objek kosong, jadi semua field-nya jadi `undefined` di caption.
    http.get(&format!("{URL_CHAR}{found}")).ok()?.json()
}
/// `let caption = ...` (`bluearchive-char.js:100-135`).
fn caption(c: &Value) -> String {
    let s = |v: Option<&Value>| js_str(v);
    // `char.name?.toUpperCase()` — `?.` membuat `undefined` lolos, lalu
    // template literal berubah jadi teks "undefined".
    let name = match c.get("name") {
        Some(Value::String(n)) => n.to_uppercase(),
        other => s(other),
    };
    let mut t = format!("🎮 *{name}*\n\n");

    if let Some(Value::String(bio)) = c.get("bio").filter(|v| crate::common::js_truthy_value(v)) {
        // `substring(0, 200)` dihitung per unit UTF-16 di JS.
        let cut: String = bio.chars().take(200).collect();
        let dots = if bio.chars().count() > 200 { "..." } else { "" };
        t.push_str(&format!("> {cut}{dots}\n\n"));
    }

    let profile = c.get("profile");
    let pf = |k: &str| profile.and_then(|p| p.get(k)).filter(|v| crate::common::js_truthy_value(v)).map(|v| s(Some(v)));

    t.push_str("╭┈┈⬡「 📋 *ᴘʀᴏꜰɪʟᴇ* 」\n");
    if let Some(v) = pf("familyName") {
        t.push_str(&format!("┃ 👤 Family: *{v}*\n"));
    }
    if let Some(v) = pf("age") {
        t.push_str(&format!("┃ 🎂 Age: *{v}*\n"));
    }
    if let Some(v) = pf("height") {
        t.push_str(&format!("┃ 📏 Height: *{v}*\n"));
    }
    if let Some(v) = pf("school") {
        t.push_str(&format!("┃ 🏫 School: *{v}*\n"));
    }
    if let Some(v) = pf("club") {
        t.push_str(&format!("┃ 🎯 Club: *{v}*\n"));
    }
    if let Some(v) = pf("hobby") {
        t.push_str(&format!("┃ ⭐ Hobby: *{v}*\n"));
    }
    if let Some(v) = pf("CV") {
        t.push_str(&format!("┃ 🎤 CV: *{v}*\n"));
    }
    t.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    t.push_str("╭┈┈⬡「 ⚔️ *ʙᴀᴛᴛʟᴇ* 」\n");
    if let Some(v) = c.get("type").filter(|v| crate::common::js_truthy_value(v)) {
        t.push_str(&format!("┃ 🏷️ Type: *{}*\n", s(Some(v))));
    }
    if let Some(v) = c.get("role").filter(|v| crate::common::js_truthy_value(v)) {
        t.push_str(&format!("┃ 🎭 Role: *{}*\n", s(Some(v))));
    }
    if let Some(v) = c.get("position").filter(|v| crate::common::js_truthy_value(v)) {
        t.push_str(&format!("┃ 📍 Position: *{}*\n", s(Some(v))));
    }
    if let Some(v) = pf("weaponType") {
        t.push_str(&format!("┃ 🔫 Weapon: *{v}*\n"));
    }
    if let Some(v) = pf("weaponName") {
        t.push_str(&format!("┃ ⚔️ Weapon Name: *{v}*\n"));
    }
    t.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    if let Some(skills) = c.get("skills").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        t.push_str("╭┈┈⬡「 ✨ *sᴋɪʟʟs* 」\n");
        for skill in skills.iter().take(4) {
            t.push_str(&format!(
                "┃ 🔹 *{}* ({})\n",
                s(skill.get("name")),
                s(skill.get("type"))
            ));
        }
        t.push_str("╰┈┈┈┈┈┈┈┈⬡");
    }
    t
}

#[plugin(
    name = "bluearchive-char",
    alias = ["bachar"],
    category = "info",
    description = "Lihat info character Blue Archive",
    usage = ".bluearchive-char <nama>",
    example = ".bluearchive-char shiroko",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bluearchive_char(ctx: Ctx) -> Reply {
    let name = ctx.text.trim().to_string();
    if name.is_empty() {
        return Reply::text(cara_pakai(&ctx.prefix));
    }

    let Some(char) = fetch_char(&*ctx.http, &name) else {
        return gagal(&ctx);
    };

    let cap = caption(&char);
    // `char.img = data.img ? CDN + data.img : null` (`bluearchive-char.js:70-72`).
    let img = char
        .get("img")
        .filter(|v| crate::common::js_truthy_value(v))
        .map(|v| format!("{CDN}{}", js_str(Some(v))));

    match img {
        Some(url) => Reply::image(MediaSource::url(url), cap).with_reaction("✅"),
        None => Reply::text(cap).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse, HttpGet};
    use std::sync::Arc;

    /// Stub dengan dua halaman: daftar character lalu detail satu character.
    #[derive(Debug)]
    struct Stub {
        list: Option<String>,
        detail: Option<String>,
        seen: std::sync::Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(list: &str, detail: &str) -> Arc<Stub> {
            Arc::new(Stub {
                list: Some(list.to_string()),
                detail: Some(detail.to_string()),
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                list: None,
                detail: None,
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = if url == URL_LIST {
                self.list.clone()
            } else {
                self.detail.clone()
            };
            let Some(text) = body else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
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
            unreachable!("bluearchive-char hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("bluearchive-char hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("bluearchive-char hanya GET")
        }
    }

    const DAFTAR: &str = r#"[
        {"name":"Shiroko","url":"shiroko","img":"shiroko.png"},
        {"name":"Hoshino","url":"hoshino","img":"hoshino.png"},
        {"name":"Aru","url":"aru","img":"aru.png"}
    ]"#;

    const DETAIL: &str = r#"{
        "name":"Hoshino",
        "bio":"Momokung.",
        "type":"Supporter",
        "role":"DPS",
        "position":"Middle",
        "img":"hoshino.png",
        "profile":{"familyName":"Shiroko","age":"16","height":"154cm",
                   "school":"Abydos","club":"Gourmet Research Society",
                   "hobby":"Memasak","CV":"Ai Kayano","weaponType":"AR",
                   "weaponName":"M4A1"},
        "skills":[{"name":"S1","type":"Explosive"},{"name":"S2","type":"Support"},
                  {"name":"S3","type":"Attack"},{"name":"S4","type":"Ultimate"},
                  {"name":"S5","type":"Passive"}]
    }"#;

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bluearchive-char".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("bluearchive-char").unwrap().handler)(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations().text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bluearchive-char").unwrap();
        assert_eq!(p.meta.name, "bluearchive-char");
        assert_eq!(p.meta.alias, ["bachar"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.description, "Lihat info character Blue Archive");
        assert_eq!(p.meta.usage, ".bluearchive-char <nama>");
        assert_eq!(p.meta.example, ".bluearchive-char shiroko");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
        assert_eq!(oktz_plugin_api::find("bachar").unwrap().meta.name, "bluearchive-char");
    }

    #[test]
    fn no_argument_shows_the_three_examples() {
        for kosong in ["", "   ", "\t\n"] {
            let r = run(Stub::new(DAFTAR, DETAIL), kosong);
            assert_eq!(
                teks(&r),
                concat!(
                    "🎮 *ʙʟᴜᴇ ᴀʀᴄʜɪᴠᴇ ᴄʜᴀʀᴀᴄᴛᴇʀ*\n\n",
                    "> Lihat info character Blue Archive\n\n",
                    "> *Contoh:*\n",
                    "> .bluearchive-char shiroko\n",
                    "> .bachar hoshino\n",
                    "> .ba aru"
                ),
                "input {kosong:?}"
            );
            assert!(!r.has_media(), "tidak ada panggilan HTTP sama sekali");
        }
    }

    #[test]
    fn the_help_text_uses_the_live_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "bachar".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("bachar").unwrap().handler)(ctx);
        assert!(teks(&r).contains("> !bachar hoshino\n"), "{}", teks(&r));
    }

    #[test]
    fn a_known_character_becomes_a_picture_with_the_full_caption() {
        let stub = Stub::new(DAFTAR, DETAIL);
        let r = run(stub.clone(), "Hoshino");
        assert_eq!(
            stub.seen.lock().unwrap().as_slice(),
            [URL_LIST, "https://api.dotgg.gg/bluearchive/characters/hoshino"],
            "daftar dulu, lalu detail"
        );
        assert!(r.has_media());
        match r.media_source() {
            Some(MediaSource::Url { url, .. }) => {
                assert_eq!(url, "https://images.dotgg.gg/bluearchive/characters/hoshino.png")
            }
            other => panic!("harus Url, dapat {other:?}"),
        }
        assert_eq!(
            teks(&r),
            concat!(
                "🎮 *HOSHINO*\n\n",
                "> Momokung.\n\n",
                "╭┈┈⬡「 📋 *ᴘʀᴏꜰɪʟᴇ* 」\n",
                "┃ 👤 Family: *Shiroko*\n",
                "┃ 🎂 Age: *16*\n",
                "┃ 📏 Height: *154cm*\n",
                "┃ 🏫 School: *Abydos*\n",
                "┃ 🎯 Club: *Gourmet Research Society*\n",
                "┃ ⭐ Hobby: *Memasak*\n",
                "┃ 🎤 CV: *Ai Kayano*\n",
                "╰┈┈┈┈┈┈┈┈⬡\n\n",
                "╭┈┈⬡「 ⚔️ *ʙᴀᴛᴛʟᴇ* 」\n",
                "┃ 🏷️ Type: *Supporter*\n",
                "┃ 🎭 Role: *DPS*\n",
                "┃ 📍 Position: *Middle*\n",
                "┃ 🔫 Weapon: *AR*\n",
                "┃ ⚔️ Weapon Name: *M4A1*\n",
                "╰┈┈┈┈┈┈┈┈⬡\n\n",
                "╭┈┈⬡「 ✨ *sᴋɪʟʟs* 」\n",
                "┃ 🔹 *S1* (Explosive)\n",
                "┃ 🔹 *S2* (Support)\n",
                "┃ 🔹 *S3* (Attack)\n",
                "┃ 🔹 *S4* (Ultimate)\n",
                "╰┈┈┈┈┈┈┈┈⬡"
            ),
            "skill ke-5 terpotong oleh slice(0, 4)"
        );
    }

    #[test]
    fn a_character_without_an_image_is_sent_as_plain_text() {
        let r = run(Stub::new(DAFTAR, r#"{"name":"Aru","type":"Front"}"#), "aru");
        assert!(!r.has_media());
        assert_eq!(
            teks(&r),
            concat!(
                "🎮 *ARU*\n\n",
                "╭┈┈⬡「 📋 *ᴘʀᴏꜰɪʟᴇ* 」\n",
                "╰┈┈┈┈┈┈┈┈⬡\n\n",
                "╭┈┈⬡「 ⚔️ *ʙᴀᴛᴛʟᴇ* 」\n",
                "┃ 🏷️ Type: *Front*\n",
                "╰┈┈┈┈┈┈┈┈⬡\n\n"
            ),
            "kotak profile kosong tetap digambar, hanya barisnya yang hilang"
        );
    }

    #[test]
    fn the_name_is_matched_case_insensitively_and_with_underscores() {
        for ketik in ["hoshino", "HOSHINO", "Ho Shi No"] {
            let stub = Stub::new(DAFTAR, DETAIL);
            let r = run(stub.clone(), ketik);
            assert!(r.has_media(), "input {ketik:?} tidak ketemu");
            assert_eq!(
                stub.seen.lock().unwrap()[1],
                "https://api.dotgg.gg/bluearchive/characters/hoshino",
                "input {ketik:?}"
            );
        }
    }

    #[test]
    fn an_unknown_name_falls_into_the_shared_error_text() {
        // `char()` melempar; `catch` hanya memanggil `te(..)`, jadi teks
        // "Character … tidak ditemukan" tidak pernah tampil.
        let stub = Stub::new(DAFTAR, DETAIL);
        let r = run(stub.clone(), "sinkronisasi");
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "bluearchive-char"),
            "saran dari JS juga tidak ikut — thrown sebelum sampai ke sana"
        );
        assert_eq!(stub.seen.lock().unwrap().len(), 1, "hanya daftar yang diambil");
    }

    #[test]
    fn an_empty_catalog_finds_nothing_and_errors() {
        let r = run(Stub::new("[]", DETAIL), "shiroko");
        assert_eq!(teks(&r), crate::common::error_template(".", "bluearchive-char"));
    }

    #[test]
    fn a_list_that_is_not_an_array_errors_instead_of_panicking() {
        let r = run(Stub::new(r#"{"error":"nope"}"#, DETAIL), "shiroko");
        assert_eq!(teks(&r), crate::common::error_template(".", "bluearchive-char"));
    }

    #[test]
    fn a_catalog_entry_without_url_errors_like_the_js_typeerror() {
        let r = run(Stub::new(r#"[{"name":"Tanpa URL"}]"#, DETAIL), "apa saja");
        assert_eq!(teks(&r), crate::common::error_template(".", "bluearchive-char"));
    }

    #[test]
    fn an_unreachable_api_errors_without_panicking() {
        let r = run(Stub::offline(), "shiroko");
        assert_eq!(teks(&r), crate::common::error_template(".", "bluearchive-char"));
    }

    #[test]
    fn a_long_bio_is_cut_at_200_characters_with_an_ellipsis() {
        let bio = "a".repeat(250);
        let detail = serde_json::json!({"name":"Aru", "bio": bio}).to_string();
        let t = teks(&run(Stub::new(DAFTAR, &detail), "aru"));
        // `substring(0, 200)` + `...`, lalu baris ditutup `\n\n`.
        assert!(
            t.contains(&format!("> {}...\n\n", "a".repeat(200))),
            "potongannya 200 lalu titik tiga"
        );
    }

    #[test]
    fn a_bio_of_exactly_200_characters_gets_no_ellipsis() {
        let bio = "a".repeat(200);
        let detail = serde_json::json!({"name":"Aru", "bio": bio}).to_string();
        let t = teks(&run(Stub::new(DAFTAR, &detail), "aru"));
        assert!(
            t.contains(&format!("> {}\n\n", "a".repeat(200))),
            "tepat 200 -> tanpa titik tiga"
        );
        assert!(!t.contains("..."));
    }

    #[test]
    fn an_empty_bio_is_skipped_entirely() {
        let detail = serde_json::json!({"name":"Aru", "bio": ""}).to_string();
        let t = teks(&run(Stub::new(DAFTAR, &detail), "aru"));
        assert!(t.starts_with("🎮 *ARU*\n\n╭┈┈⬡"), "{t}");
    }

    #[test]
    fn a_character_without_a_name_renders_the_js_undefined_text() {
        let detail = serde_json::json!({"type":"Front"}).to_string();
        assert!(teks(&run(Stub::new(DAFTAR, &detail), "aru")).starts_with("🎮 *undefined*\n\n"));
    }

    #[test]
    fn falsy_profile_fields_drop_their_row() {
        let detail = serde_json::json!({
            "name":"Aru",
            "profile":{"familyName":"", "age":0, "height":"154cm"}
        })
        .to_string();
        let t = teks(&run(Stub::new(DAFTAR, &detail), "aru"));
        assert!(!t.contains("Family:"), "string kosong itu falsy");
        assert!(!t.contains("Age:"), "0 itu falsy");
        assert!(t.contains("Height: *154cm*"));
    }

    #[test]
    fn four_skills_is_the_cap_and_zero_skill_draws_no_box() {
        let detail = serde_json::json!({"name":"Aru","skills":[]}).to_string();
        let t = teks(&run(Stub::new(DAFTAR, &detail), "aru"));
        assert!(!t.contains("sᴋɪʟʟs"), "array kosong -> tanpa kotak");
        assert!(t.ends_with("╰┈┈┈┈┈┈┈┈⬡\n\n"), "kotak skills tidak ditambah");
    }
}
