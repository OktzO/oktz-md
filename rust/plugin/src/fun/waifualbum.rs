//! `plugins/fun/waifualbum.js` — riwayat gacha waifu + statistik luck.
//!
//! `albumStats` ada di `src/lib/waifu.js:184` DAN `src/lib/waifu.js:184` dengan
//! tubuh yang persis sama dan konstanta yang sama (`TIER_VALUE`/`TIER_EXPECTED`
//! dari `src/lib/romance.js`). Satu implementasi di [`crate::common::album_stats`]
//! melayani keduanya — bukan dua salinan yang bisa menyimpang diam-diam saat
//! ada yang memperbaiki `toFixed` di satu sisi saja.
//!
//! `ponytail:` JS memanggil `db.getUser(m.sender)` dan `if (!user) return;`
//! tanpa balasan sama sekali. `Ctx::user_data` = `Value::Null` untuk user yang
//! belum pernah ada, jadi user null diperlakukan sebagai "album kosong" —
//! pesan yang diberikan JS, bukan diam. Bedanya nyata (user baru dapat cara
//! mulai gacha) dan disengaja: diam tidak pernah bisa dit touted.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{album_stats, render_album};

const TITLE: &str = "ALBUM WAIFU & LUCK STATS";

fn empty(ctx: &Ctx) -> String {
    format!(
        "📭 *Album kosong!*\nKamu belum pernah gacha waifu. Ketik *{}gachawaifu* untuk mulai!",
        ctx.prefix
    )
}

pub(crate) fn reply_text(ctx: &Ctx) -> String {
    let history = ctx.user_list("waifuHistory");
    if history.is_empty() {
        return empty(ctx);
    }
    let stats = ctx.user_field("waifuStats").cloned().unwrap_or_default();
    render_album(&album_stats(history, &stats), TITLE)
}

#[plugin(
    name = "waifualbum",
    alias = ["albumwaifu"],
    category = "fun",
    description = "Lihat riwayat gacha waifu & statistik luck kamu!",
    usage = ".waifualbum",
    example = ".waifualbum",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn waifualbum(ctx: Ctx) -> Reply {
    Reply::text(reply_text(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(user: serde_json::Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "waifualbum".into(),
            sender: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        }
    }

    fn pull(tier: &str, name: &str) -> serde_json::Value {
        serde_json::json!({"tier": tier, "name": name})
    }

    fn meta() -> &'static oktz_plugin_api::PluginMeta {
        &oktz_plugin_api::find("waifualbum").unwrap().meta
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        // `name` di JS adalah ARRAY ["waifualbum", "albumwaifu"] dengan alias
        // kosong; array itu diratakan jadi name + alias. Lihat doc modul.
        let p = meta();
        assert_eq!(p.name, "waifualbum");
        assert_eq!(p.alias, ["albumwaifu"]);
        assert_eq!(p.category, "fun");
        assert_eq!(p.description, "Lihat riwayat gacha waifu & statistik luck kamu!");
        assert_eq!(p.usage, ".waifualbum");
        assert_eq!(p.example, ".waifualbum");
        assert_eq!(p.cooldown, 5);
        assert_eq!(p.energi, 1);
        assert!(!p.is_group && !p.is_private && p.is_enabled);
    }

    #[test]
    fn the_second_name_array_entry_resolves_as_an_alias() {
        assert_eq!(
            oktz_plugin_api::find("albumwaifu").map(|p| p.meta.name.clone()),
            Some("waifualbum".to_string())
        );
    }

    #[test]
    fn no_key_collides_with_another_registered_plugin() {
        for key in ["waifualbum", "albumwaifu"] {
            let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                .iter()
                .filter(|e| e.meta.matches(key))
                .map(|e| e.meta.name.as_str())
                .collect();
            assert_eq!(owners, ["waifualbum"], "key {key} diklaim oleh {owners:?}");
        }
    }

    #[test]
    fn an_absent_or_empty_history_takes_the_empty_album_branch() {
        // `if (!user) return;` JS tidak membalas apa pun; di sini user null
        // ikut diberi cara mulai (lihat doc modul).
        for user in [
            serde_json::json!(null),
            serde_json::json!({}),
            serde_json::json!({"waifuHistory": []}),
            serde_json::json!({"waifuHistory": null}),
        ] {
            assert_eq!(reply_text(&ctx_of(user.clone())), empty(&ctx_of(user.clone())), "{user}");
        }
        assert!(reply_text(&ctx_of(serde_json::json!(null))).contains("gachawaifu"));
    }

    #[test]
    fn the_header_and_every_tier_row_are_always_present() {
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Epic", "Yuki")],
            "waifuStats": {"totalGacha": 4, "byTier": {"Epic": 1, "Common": 3}}
        })));
        assert!(out.starts_with("📚 *ALBUM WAIFU & LUCK STATS* 📚\n\n"), "{out}");
        assert!(out.contains("🎲 *Total Gacha:* 4\n"), "{out}");
        for label in [
            "🟢 Common     ",
            "🔵 Rare       ",
            "🟣 Epic       ",
            "🟡 Legendary  ",
            "🔴 Mythic     ",
        ] {
            assert!(out.contains(label), "baris tier hilang: {label:?}\n{out}");
        }
    }

    #[test]
    fn the_tier_counts_are_right_aligned_in_three_columns() {
        // `String(n).padStart(3)` — ini yang membuat kolom rata kanan.
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Common", "A")],
            "waifuStats": {"totalGacha": 12, "byTier": {"Common": 9, "Mythic": 3}}
        })));
        assert!(out.contains("🟢 Common       9 "), "{out}");
        assert!(out.contains("🔵 Rare         0 "), "{out}");
        assert!(out.contains("🔴 Mythic       3 "), "{out}");
    }

    #[test]
    fn the_bar_is_twenty_cells_and_scales_with_the_count() {
        let one_of_ten = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Rare", "A")],
            "waifuStats": {"totalGacha": 10, "byTier": {"Rare": 1}}
        })));
        let row = one_of_ten.lines().find(|l| l.starts_with("🔵 Rare")).unwrap();
        let cells: String = row.chars().filter(|c| *c == '█' || *c == '░').collect();
        assert_eq!(cells.chars().count(), 20, "bar harus 20 sel: {row:?}");
        assert_eq!(cells.chars().filter(|c| *c == '█').count(), 2, "1/10 → 2 sel: {row:?}");

        let full = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Rare", "A")],
            "waifuStats": {"totalGacha": 5, "byTier": {"Rare": 5}}
        })));
        let full_row = full.lines().find(|l| l.starts_with("🔵 Rare")).unwrap();
        assert_eq!(
            full_row.chars().filter(|c| *c == '█').count(),
            20,
            "100% → 20 sel: {full_row:?}"
        );
    }

    #[test]
    fn a_zero_total_never_produces_a_nan_or_a_full_bar() {
        // JS: `stats.total ? ... : 0` — short-circuit, bukan NaN.
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Common", "A")],
            "waifuStats": {"byTier": {"Common": 5}}
        })));
        assert!(out.contains("🎲 *Total Gacha:* 0\n"), "{out}");
        let row = out.lines().find(|l| l.starts_with("🟢 Common")).unwrap();
        assert_eq!(row.chars().filter(|c| *c == '█').count(), 0, "{row:?}");
        assert_eq!(row.chars().filter(|c| *c == '░').count(), 20, "{row:?}");
        // Luck 0 dan emojihnenyes karena `luck >= 1` false.
        assert!(out.contains("🍀 *Luck Score:* 0 😅"), "{out}");
    }

    #[test]
    fn the_luck_score_is_the_tier_value_average_over_the_expectation_rounded_to_two_decimals() {
        // 1 Mythic saja: (16 / 1) / 1.255 = 12.749... → 12.75.
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Mythic", "A")],
            "waifuStats": {"totalGacha": 1, "byTier": {"Mythic": 1}}
        })));
        assert!(out.contains("🍀 *Luck Score:* 12.75 🔥"), "{out}");

        // 1 Common: (1 / 1) / 1.255 = 0.7968... → 0.8 (bukan 0.80 — `toFixed`
        // menjaga trailing zero-nya, `Number(...)` membuangnya).
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Common", "A")],
            "waifuStats": {"totalGacha": 1, "byTier": {"Common": 1}}
        })));
        assert!(out.contains("*Luck Score:* 0.8 😅"), "{out}");
    }

    #[test]
    fn the_rarest_and_pity_lines_only_appear_when_they_are_set() {
        let bare = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Common", "A")],
            "waifuStats": {"totalGacha": 1, "byTier": {"Common": 1}}
        })));
        assert!(!bare.contains("Pull Paling Langka"), "{bare}");
        assert!(!bare.contains("Pity aktif"), "{bare}");

        let full = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Mythic", "A")],
            "waifuStats": {
                "totalGacha": 25,
                "byTier": {"Mythic": 1},
                "rarest": {"tier": "Mythic", "name": "Ravena"},
                "pityCounter": 7
            }
        })));
        assert!(full.contains("🏆 *Pull Paling Langka:* 🔴 Ravena (Mythic)\n"), "{full}");
        assert!(full.contains("🎯 *Pity aktif:* 7/20 roll tanpa Epic+\n"), "{full}");
        assert!(full.contains("💍 *Pernikahan:* 0\n"), "{full}");
    }

    #[test]
    fn the_last_ten_pulls_are_newest_first_and_capped_at_ten() {
        let history: Vec<serde_json::Value> =
            (1..=14).map(|i| pull("Common", &format!("H{i}"))).collect();
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": history,
            "waifuStats": {"totalGacha": 14, "byTier": {"Common": 14}}
        })));
        let listed = out
            .lines()
            .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && l.contains(" — "))
            .count();
        assert_eq!(listed, 10, "harus tepat 10: {out}");
        assert!(out.contains("1. 🟢 *H14* — Common\n"), "paling baru dulu: {out}");
        assert!(out.contains("10. 🟢 *H5* — Common\n"), "sampai H5: {out}");
        assert!(!out.contains("*H4*"), "H4 sudah di luar 10 terakhir: {out}");
    }

    #[test]
    fn an_unknown_tier_renders_with_no_emoji_rather_than_panicking() {
        // `tierEmoji[tier] || ""` — tier asing jadi string kosong.
        let out = reply_text(&ctx_of(serde_json::json!({
            "waifuHistory": [pull("Legend", "X")],
            "waifuStats": {"totalGacha": 1, "byTier": {}}
        })));
        assert!(out.contains("1.  *X* — Legend\n"), "{out}");
    }

    #[test]
    fn the_registered_handler_always_answers_with_text() {
        for user in [
            serde_json::json!(null),
            serde_json::json!({}),
            serde_json::json!({"waifuHistory": [], "waifuStats": {}}),
            serde_json::json!({"waifuHistory": [pull("Epic", "A")]}),
            serde_json::json!({"waifuHistory": [pull("Epic", "A")], "waifuStats": {"totalGacha": 9}}),
        ] {
            assert!(waifualbum(ctx_of(user.clone())).text_body().is_some(), "{user}");
        }
    }
}