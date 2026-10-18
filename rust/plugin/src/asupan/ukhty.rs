//! `plugins/asupan/ukhty.js` — video ukhty.
//!
//! `pub mod ukhty;` perlu ditambah manual ke `src/asupan/mod.rs` (mod.rs
//! berisi plugin lain, bukan milik saya — lihat PORT-BRIEF §6).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::random_index;

const PATH: &str = "src/tiktok/ukhty.json";

const ERR_KOSONG: &str = "❌ Data tidak tersedia";
const ERR_TIDAK_DITEMUKAN: &str = "❌ *ᴇʀʀᴏʀ*\n\n> Video tidak ditemukan";

/// `loadJsonData('ukhty.json')` — `ukhty.js:19-27`.
fn muat_data(berkas: &dyn oktz_plugin_api::berkas::Berkas) -> Vec<String> {
    let Ok(teks) = berkas.baca_teks(PATH) else {
        return Vec::new();
    };
    let Ok(data): Result<Value, _> = serde_json::from_str(&teks) else {
        return Vec::new();
    };
    let Some(arr) = data.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| item.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

#[plugin(
    name = "ukhty",
    alias = ["ukht"],
    category: "asupan",
    description = "Video ukhty",
    usage = ".ukhty",
    example = ".ukhty",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ukhty(ctx: Ctx) -> Reply {
    let urls = match muat_data(ctx.berkas()) {
        mut u if !u.is_empty() => u,
        _ => return Reply::text(ERR_KOSONG),
    };

    let url = &urls[random_index(urls.len())];

    Reply::video(MediaSource::url(url.to_string()), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::BerkasPalsu;

    fn run(berkas: BerkasPalsu) -> Reply {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "ukhty".into(),
            berkas: std::sync::Arc::new(berkas),
            ..Ctx::default()
        };
        ukhty(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    fn video_url(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Video { media, .. } => media.as_ref().and_then(|m| match m {
                MediaSource::Url { url, .. } => Some(url.clone()),
                _ => None,
            }),
            _ => None,
        }
    }

    #[test]
    fn meta_sesuai_js() {
        let p = oktz_plugin_api::find("ukhty").unwrap();
        assert_eq!(p.meta.name, "ukhty");
        assert_eq!(p.meta.alias, ["ukht"]);
        assert_eq!(p.meta.category, "asupan");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn berkas_kosong_atau_tidak_ada_mengembalikan_pesan_kosong() {
        let r = run(BerkasPalsu::baru());
        assert_eq!(body_of(&r), ERR_KOSONG);
    }

    #[test]
    fn json_salah_dilaporkan_kosong() {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(PATH, "bukan json");
        let r = run(b);
        assert_eq!(body_of(&r), ERR_KOSONG);
    }

    #[test]
    fn data_kosong_dilaporkan_kosong() {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(PATH, "[]");
        let r = run(b);
        assert_eq!(body_of(&r), ERR_KOSONG);
    }

    #[test]
    fn url_dikirim_sebagai_video() {
        let b = BerkasPalsu::baru();
        let _ = b.tulis_teks(
            PATH,
            r#"[{"url":"https://cdn/ukhty1.mp4"},{"url":"https://cdn/ukhty2.mp4"}]"#,
        );
        let r = run(b);
        assert!(matches!(video_url(&r), Some(u) if u.ends_with(".mp4")));
    }
}
