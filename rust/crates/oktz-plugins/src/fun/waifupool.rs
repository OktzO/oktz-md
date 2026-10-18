//! `plugins/fun/waifupool.js` — jelajahi pool waifu dengan paginasi.
//!
//! `ponytail:` `searchPool` (`data/waifu/index.js`) dan `sock.sendButton`
//! (tombol interaktif) tidak tersedia di `Ctx`/`Reply`; handler diakhiri
//! `error_template` + `☢` (pola `swm.rs`). Emoji tier, aritmatika paginasi,
//! parsing nav/query, dan rendering satu entri di-port agar bisa diuji.
//! Saat data pool + tombol tersedia, tambahkan `pub mod waifupool;` di
//! `src/fun/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `PAGE_SIZE` (`waifupool.js:44`).
pub(crate) const HALAMAN_SIZE: usize = 10;

/// Satu entri waifu dari pool (`waifupool.js:61-63`).
pub(crate) struct Waifu {
    pub tier: String,
    pub name: String,
    pub series: String,
    pub personality: String,
}

/// `tierEmoji(t)` (`waifupool.js:30-32`).
pub(crate) fn emoji_tier(tier: &str) -> &'static str {
    match tier {
        "Common" => "🟢",
        "Rare" => "🔵",
        "Epic" => "🟣",
        "Legendary" => "🟡",
        "Mythic" => "🔴",
        _ => "",
    }
}

/// Parsing nav + query (`waifupool.js:36-39`).
pub(crate) struct NavQuery {
    pub nav: Option<String>,
    pub query: String,
}

pub(crate) fn parse_nav_query(args: &[String]) -> NavQuery {
    let first = args.first().map(|a| a.to_ascii_lowercase()).unwrap_or_default();
    let is_nav = (first == "prev" || first == "next") && args.len() > 1;
    let nav = if is_nav { Some(first.clone()) } else { None };
    let bagian = if is_nav { &args[1..] } else { args };
    let query = bagian.join(" ").trim().to_string();
    NavQuery { nav, query }
}

/// Hitung halaman (`waifupool.js:40-43`).
pub(crate) fn hitung_halaman(page: usize, total: usize, nav: &Option<String>) -> usize {
    let mut page = page;
    match nav.as_deref() {
        Some("next") => page += 1,
        Some("prev") => {
            if page > 0 {
                page -= 1;
            }
        }
        _ => {}
    }
    page
}

/// Total halaman (`waifupool.js:53`): `Math.max(1, Math.ceil(pool.length / 10))`.
pub(crate) fn total_halaman(pool_len: usize) -> usize {
    pool_len.div_ceil(HALAMAN_SIZE).max(1)
}

/// Satu baris entri waifu (`waifupool.js:61-63`).
pub(crate) fn render_entri(index: usize, w: &Waifu) -> String {
    format!(
        "{}. {} *{}* — {}\n   🏷️ {} | 🎭 {}\n",
        index,
        emoji_tier(&w.tier),
        w.name,
        w.tier,
        w.series,
        w.personality
    )
}

/// Header teks (`waifupool.js:57-59`).
pub(crate) fn render_header(query: Option<&str>, pool_len: usize, cur: usize, total: usize) -> String {
    let mut out = String::from("📖 *WAIFU POOL* 📖\n\n");
    if let Some(q) = query {
        out.push_str(&format!("🔍 Pencarian: *\"{q}\"*\n"));
    }
    out.push_str(&format!(
        "📊 Total: *{pool_len}* waifu | Halaman *{cur}/{total}*\n\n"
    ));
    out
}

/// Tidak ada kecocokan (`waifupool.js:50-51`).
pub(crate) const TIDAK_ADA_WAIFU: &str = "🔍 Tidak ada waifu cocok dengan \"{query}\". Coba nama/tier/franchise lain.";

pub(crate) fn tidak_ada_waifu(query: &str) -> String {
    TIDAK_ADA_WAIFU.replace("{query}", query)
}

#[plugin(
    name = "waifupool",
    alias = [],
    category = "fun",
    description = "Jelajahi pool waifu — cari nama/franchise/tier!",
    usage = ".waifupool [nama|tier|franchise]",
    example = ".waifupool rare",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn waifupool(ctx: Ctx) -> Reply {
    // `searchPool` data dan `sendButton` tidak tersedia — diakhiri error
    // template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("waifupool").unwrap();
        assert_eq!(p.meta.name, "waifupool");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn emoji_tier_semua_tier() {
        assert_eq!(emoji_tier("Common"), "🟢");
        assert_eq!(emoji_tier("Rare"), "🔵");
        assert_eq!(emoji_tier("Epic"), "🟣");
        assert_eq!(emoji_tier("Legendary"), "🟡");
        assert_eq!(emoji_tier("Mythic"), "🔴");
        assert_eq!(emoji_tier("TidakDiketahui"), "");
    }

    #[test]
    fn parse_nav_query() {
        let nq = parse_nav_query(&["prev".into(), "naruto".into()]);
        assert_eq!(nq.nav, Some("prev".to_string()));
        assert_eq!(nq.query, "naruto");

        let nq = parse_nav_query(&["next".into(), "rare".into()]);
        assert_eq!(nq.nav, Some("next".to_string()));
        assert_eq!(nq.query, "rare");

        let nq = parse_nav_query(&["naruto".into()]);
        assert_eq!(nq.nav, None);
        assert_eq!(nq.query, "naruto");

        let nq = parse_nav_query(&["prev".into()]);
        assert_eq!(nq.nav, None);
        assert_eq!(nq.query, "");

        let nq = parse_nav_query(&[]);
        assert_eq!(nq.nav, None);
        assert_eq!(nq.query, "");
    }

    #[test]
    fn hitung_halaman_nav() {
        assert_eq!(hitung_halaman(0, 100, &Some("next".into())), 1);
        assert_eq!(hitung_halaman(5, 100, &Some("prev".into())), 4);
        assert_eq!(hitung_halaman(0, 100, &Some("prev".into())), 0);
        assert_eq!(hitung_halaman(3, 100, &None), 3);
    }

    #[test]
    fn total_halaman_minimal_1() {
        assert_eq!(total_halaman(0), 1);
        assert_eq!(total_halaman(1), 1);
        assert_eq!(total_halaman(10), 1);
        assert_eq!(total_halaman(11), 2);
        assert_eq!(total_halaman(20), 2);
        assert_eq!(total_halaman(21), 3);
    }

    #[test]
    fn render_entri_bentuk_js() {
        let w = Waifu {
            tier: "Rare".into(),
            name: "Nagatoro".into(),
            series: "Don't Toy With Me".into(),
            personality: "Tsundere".into(),
        };
        let out = render_entri(5, &w);
        assert_eq!(out, "5. 🔵 *Nagatoro* — Rare\n   🏷️ Don't Toy With Me | 🎭 Tsundere\n");
    }

    #[test]
    fn render_header_dan_query() {
        let out = render_header(Some("naruto"), 25, 2, 3);
        assert!(out.starts_with("📖 *WAIFU POOL* 📖\n\n"));
        assert!(out.contains("🔍 Pencarian: *\"naruto\"*"));
        assert!(out.contains("📊 Total: *25* waifu | Halaman *2/3*"));
    }

    #[test]
    fn tidak_ada_waifu_pakai_query() {
        assert_eq!(
            tidak_ada_waifu("saitama"),
            "🔍 Tidak ada waifu cocok dengan \"saitama\". Coba nama/tier/franchise lain."
        );
    }

    #[test]
    fn handler_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "waifupool".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = waifupool(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "waifupool")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
