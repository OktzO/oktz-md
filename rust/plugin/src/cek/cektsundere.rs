//! `plugins/cek/cektsundere.js` — Cek tingkat ketsunderean 0-100%, lima ambang `>= 90/70/50/30`.

use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cektsundere,
    "cektsundere",
    ["tsundere"],
    "Cek tingkat tsundere kamu",
    ".cektsundere <nama>",
    ".cektsundere Budi",
    "ketsunderean",
    [
        "BAKA! B-BUKAN BERARTI AKU SUKA! 😤💢",
        "Hmph! Jangan salah paham ya! 😳",
        "Y-yah terserah kamu deh... 👉👈",
        "Agak tsundere dikit~ 😊",
        "Bukan tsundere, jujur aja kok 💕"
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    const TIERS: [&str; 5] = ["BAKA! B-BUKAN BERARTI AKU SUKA! 😤💢", "Hmph! Jangan salah paham ya! 😳", "Y-yah terserah kamu deh... 👉👈", "Agak tsundere dikit~ 😊", "Bukan tsundere, jujur aja kok 💕"];
    const LABEL: &str = "ketsunderean";
    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    /// Undian sebanyak ini hampir pasti menyentuh kelima tier: pita
    /// teratas hanya 11/101, jadi peluang gagal satu tier ~1e-10.
    const SAMPLES: usize = 2_000;

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("cektsundere").unwrap().handler)(ctx)
    }

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cektsundere".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        }
    }

    fn ctx_with_mention() -> Ctx {
        Ctx {
            mentioned: vec![OTHER.to_string()],
            ..base()
        }
    }

    fn text_of(r: &Reply) -> (String, Vec<String>) {
        match r {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    /// Baca ulang angka di `*NN%*` supaya batas `floor(random * 101)`
    /// diuji dari balasan plugin yang sungguhan, bukan dari tebakan.
    fn percent_in(text: &str) -> u32 {
        let rest = text.split_once('*').expect("harus ada *percent*").1;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().expect("percent harus angka")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cektsundere").unwrap();
        assert_eq!(p.meta.name, "cektsundere");
        assert_eq!(p.meta.alias, ["tsundere"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek tingkat tsundere kamu");
        assert_eq!(p.meta.usage, ".cektsundere <nama>");
        assert_eq!(p.meta.example, ".cektsundere Budi");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cektsundere","tsundere"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cektsundere"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn without_mention_the_reply_targets_the_sender() {
        let (text, mentions) = text_of(&run(base()));
        assert_eq!(mentions, [SENDER.to_string()]);
        assert!(text.starts_with("Hai @628111"), "{text}");
        assert!(
            text.contains(&format!("Tingkat {LABEL} kamu *{}%", percent_in(&text))),
            "{text}"
        );
        assert!(!text.contains("dia sebesar"), "self tidak pakai 'dia': {text}");
    }

    #[test]
    fn with_mention_the_reply_targets_the_mention_and_says_dia() {
        let (text, mentions) = text_of(&run(ctx_with_mention()));
        assert_eq!(mentions, [OTHER.to_string()]);
        assert!(
            text.contains(&format!("tingkat {LABEL} @628999 yak?")),
            "{text}"
        );
        assert!(
            text.contains(&format!("Tingkat {LABEL} dia sebesar *{}%", percent_in(&text))),
            "{text}"
        );
    }

    #[test]
    fn every_percent_value_picks_exactly_one_tier() {
        // `Math.floor(Math.random() * 101)` = 0..=100 inklusif; semua nilai
        // ditutup supaya ambang yang bergeser langsung ketahuan.
        let ctx = base();
        for percent in 0..=100u32 {
            let (text, _) = text_of(&cek_reply(&ctx, percent, LABEL, TIERS));
            assert!(text.contains(&format!("*{percent}%*")), "percent hilang: {text}");
            let hits: Vec<&&str> = TIERS.iter().filter(|t| text.contains(**t)).collect();
            assert_eq!(hits.len(), 1, "percent {percent} memuat {} tier: {text}", hits.len());
        }
    }

    #[test]
    fn tier_boundaries_are_inclusive_at_90_70_50_30() {
        let ctx = base();
        for (percent, expected) in [
            (100u32, TIERS[0]),
            (90, TIERS[0]),
            (89, TIERS[1]),
            (70, TIERS[1]),
            (69, TIERS[2]),
            (50, TIERS[2]),
            (49, TIERS[3]),
            (30, TIERS[3]),
            (29, TIERS[4]),
            (0, TIERS[4]),
        ] {
            let (text, _) = text_of(&cek_reply(&ctx, percent, LABEL, TIERS));
            assert!(
                text.contains(expected),
                "percent {percent} harusnya {expected:?}: {text}"
            );
        }
    }

    #[test]
    fn handler_stays_in_range_and_reaches_every_tier() {
        let ctx = base();
        let mut seen: Vec<&str> = Vec::new();
        for _ in 0..SAMPLES {
            let (text, mentions) = text_of(&run(ctx.clone()));
            assert_eq!(mentions, [SENDER.to_string()]);
            assert!(text.contains(&format!("Tingkat {LABEL} kamu")), "{text}");
            let percent = percent_in(&text);
            assert!(percent <= 100, "percent di luar 0..=100: {percent}");
            let hits: Vec<&&str> = TIERS.iter().filter(|t| text.contains(**t)).collect();
            assert_eq!(hits.len(), 1, "balasan memuat {} tier: {text}", hits.len());
            if !seen.contains(hits[0]) {
                seen.push(hits[0]);
            }
        }
        assert_eq!(seen.len(), 5, "tier yang tidak terjangkau: {seen:?}");
    }
}
