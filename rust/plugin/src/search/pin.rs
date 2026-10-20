//! `plugins/search/pin.js` — cari gambar Pinterest dan kirimnya sebagai album.
//!
//! `ponytail:` `resolver.resolve("pinterest", { q })` (`src/lib/resolve.js`)
//! belum di-port, jadi `resolve_pinterest` selalu `Err` dan yang menyala
//! adalah cabang `catch` biasa — pola `download::pindl`. Guard argumen dan
//! seluruh susunan pesan tetap dipor supaya utuh.
//!
//! `ponytail:` album `albumMessage` + `messageContextInfo.messageSecret` +
//! `sock.relayMessage` + `sock.waUploadToServer` tidak punya padanan di
//! `Reply`, dan `Reply` hanya bisa satu media. Fallback `sock.sendMessage`
//! satu per satu juga tidak ada padanannya.

#![allow(dead_code)] // jalur album tidak terjangkau tanpa resolver

use oktz_plugin_api::{Ctx, HttpCall, Reply, plugin};


/// `pin.js:30-35` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔍 *ᴘɪɴᴛᴇʀᴇsᴛ sᴇᴀʀᴄʜ*\n\n\
         > Contoh:\n\
         `{prefix}pin Zhao Lusi`"
    )
}

/// `pin.js:49` — resolver menjawab tapi tidak ada pin.
pub(crate) fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Tidak ditemukan hasil untuk: {query}")
}

/// `pin.js:73` — ada pin tapi tidak satu pun buffer yang lolos unduhan.
pub(crate) const GAGAL_MEMUAT: &str = "❌ Gagal memuat gambar";

/// `pin.js:63` — ambang ukuran; di bawah itu buffer diabaikan diam-diam.
pub(crate) const MIN_BYTES: usize = 1000;

/// `axios.get(imageUrl, { headers: { Referer } })` (`pin.js:56-59`).
pub(crate) fn download_request(url: &str) -> HttpCall {
    HttpCall::get(url).header("Referer", "https://www.pinterest.com/")
}

/// `data?.pins ?? []` → `pin?.image` → `filter(Boolean)` → `slice(0, 10)`
/// (`pin.js:43-46`).
pub(crate) fn pick_images(pins: &[serde_json::Value]) -> Vec<String> {
    pins.iter()
        .filter_map(|p| p.get("image"))
        .filter_map(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .take(10)
        .map(str::to_string)
        .collect()
}

/// `resolver.resolve("pinterest", { q })` — lapisan kapabilitas belum
/// di-port. `Err` = melempar, persis seperti backend yang gagal.
fn resolve_pinterest(_query: &str) -> Result<Vec<serde_json::Value>, ()> {
    Err(())
}

#[plugin(
    name = "pin",
    alias = ["pinsearch", "pinterestsearch", "pins"],
    category = "search",
    description = "Cari gambar di Pinterest (album)",
    usage = ".pin <query>",
    example = ".pin Zhao Lusi",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pin(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `await m.react("🕕")` di `:36` sudah jalan sebelum resolver.
    match resolve_pinterest(query) {
        Ok(pins) => {
            let results = pick_images(&pins);
            if results.is_empty() {
                return Reply::text(tidak_ditemukan(query)).with_reaction("❌");
            }
            // `pin.js:52-125` — tiap URL diunduh lewat `download_request`
            // (butuh header `Referer`), lalu dikirim sebagai album WA lewat
            // `sock.relayMessage` + `sock.waUploadToServer`, dengan fallback
            // `sock.sendMessage` satu per satu. Tidak ada padanannya di
            // `Reply`, jadi `mediaList` selalu kosong dan pesan "gagal memuat"
            // yang tersisa — persis cabang `mediaList.length === 0`.
            Reply::text(GAGAL_MEMUAT).with_reaction("❌")
        }
        // `pin.js:127-131` — `m.pushName` tidak ada di `Ctx`, jadi "User".
        Err(()) => Reply::text(crate::common::error_template(&ctx)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("pin").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pin").unwrap();
        assert_eq!(p.meta.name, "pin");
        assert_eq!(p.meta.alias, ["pinsearch", "pinterestsearch", "pins"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari gambar di Pinterest (album)");
        assert_eq!(p.meta.usage, ".pin <query>");
        assert_eq!(p.meta.example, ".pin Zhao Lusi");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pinsearch", "pinterestsearch", "pins"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pin");
        }
    }

    #[test]
    fn an_empty_query_shows_the_help_card() {
        for text in ["", "  ", "\t\n"] {
            assert_eq!(
                text_of(&run(text)),
                "🔍 *ᴘɪɴᴛᴇʀᴇsᴛ sᴇᴀʀᴄʜ*\n\n> Contoh:\n`.pin Zhao Lusi`",
                "input {text:?}"
            );
        }
        assert!(help("!").contains("`!pin Zhao Lusi`"));
        assert_eq!(reaction_of(&run("")), None);
    }

    #[test]
    fn a_query_reaches_the_error_branch_while_the_resolver_is_missing() {
        let r = run("Zhao Lusi");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "pin"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", &"x".repeat(5000)] {
            let r = run(input);
            assert!(!text_of(&r).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn pin_urls_are_collected_dropping_blanks_and_capped_at_ten() {
        let pins: Vec<serde_json::Value> = (0..13)
            .map(|i| serde_json::json!({ "image": format!("https://i.pinimg.com/{i}.jpg") }))
            .chain([serde_json::json!({ "image": "" }), serde_json::json!({}), serde_json::json!({ "image": null })])
            .collect();
        let urls = pick_images(&pins);
        assert_eq!(urls.len(), 10, "slice(0, 10) sebelum filter kedua");
        assert_eq!(urls[0], "https://i.pinimg.com/0.jpg");
        assert_eq!(urls[9], "https://i.pinimg.com/9.jpg");
    }

    #[test]
    fn an_empty_or_scalar_pin_list_yields_no_urls() {
        assert!(pick_images(&[]).is_empty());
        assert!(pick_images(&[serde_json::json!({})]).is_empty());
        assert!(pick_images(&[serde_json::json!({ "image": 42 })]).is_empty());
    }

    #[test]
    fn the_not_found_and_load_failure_messages_are_kept() {
        assert_eq!(
            tidak_ditemukan("Zhao Lusi"),
            "❌ Tidak ditemukan hasil untuk: Zhao Lusi"
        );
        assert_eq!(GAGAL_MEMUAT, "❌ Gagal memuat gambar");
    }

    #[test]
    fn the_pinterest_request_carries_the_referer_header() {
        let call = download_request("https://i.pinimg.com/originals/x.jpg");
        assert_eq!(call.method, oktz_plugin_api::HttpMethod::Get);
        assert!(call
            .headers
            .iter()
            .any(|(k, v)| k == "Referer" && v == "https://www.pinterest.com/"));
    }
}