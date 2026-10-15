//! `plugins/cek/cekkece.js` — Cek tingkat kekecean 0-100%, lima ambang `>= 90/70/50/30`.

use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekkece,
    "cekkece",
    ["kece", "cool"],
    "Cek seberapa kece kamu",
    ".cekkece <nama>",
    ".cekkece Budi",
    "kekecean",
    [
        "KECE BADAI! 😎🔥",
        "Kece banget! ✨",
        "Lumayan kece~ 👍",
        "Sedikit kece 😊",
        "Biasa aja, tapi tetep keren! 🙂"
    ]
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    /// Tier cekkece.js — urutan dari ambang tertinggi.
    const TIERS: [&str; 5] = [
        "KECE BADAI! 😎🔥",
        "Kece banget! ✨",
        "Lumayan kece~ 👍",
        "Sedikit kece 😊",
        "Biasa aja, tapi tetep keren! 🙂",
    ];

    /// Rentang inklusif tiap tier, dibaca dari ambang `percent >= N` di JS.
    const BANDS: [(u32, u32); 5] = [(90, 100), (70, 89), (50, 69), (30, 49), (0, 29)];

    const ME: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn run(c: Ctx) -> Reply {
        (find("cekkece").unwrap().handler)(c)
    }

    fn ctx_with(mention: Option<&str>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekkece".into(),
            sender: ME.into(),
            mentioned: mention.map(str::to_string).into_iter().collect(),
            ..Default::default()
        }
    }

    fn split(r: &Reply) -> (String, Vec<String>) {
        match r {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Reply::Text, dapat {other:?}"),
        }
    }

    /// Ambil angka di `*<persen>%*` — satu-satunya angka persen di balasan.
    fn percent_of(text: &str) -> u32 {
        let body = text
            .split_once("*")
            .and_then(|(_, rest)| rest.split_once('%'))
            .map(|(head, _)| head.to_string())
            .unwrap_or_else(|| panic!("balasan tanpa persen: {text}"));
        assert!(
            body.chars().all(|c| c.is_ascii_digit()),
            "persen bukan angka bulat: {body:?} di {text}"
        );
        body.parse().expect("persen harus bisa diparse")
    }

    #[test]
    fn plugin_config_disalin_apa_adanya() {
        let p = find("cekkece").unwrap();
        assert_eq!(p.meta.name, "cekkece");
        assert_eq!(p.meta.alias, ["kece", "cool"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek seberapa kece kamu");
        assert_eq!(p.meta.usage, ".cekkece <nama>");
        assert_eq!(p.meta.example, ".cekkece Budi");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn setiap_alias_resolve_kembali_ke_plugin_ini() {
        for key in std::iter::once("cekkece").chain(["kece", "cool"]) {
            assert_eq!(
                find(key).map(|p| p.meta.name.as_str()),
                Some("cekkece"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn tanpa_mention_target_adalah_sender_dan_kalimatnya_kamu() {
        let (text, mentions) = split(&run(ctx_with(None)));
        assert!(text.starts_with("Hai @628111\n"), "{text}");
        assert!(
            text.contains("Tingkat kekecean kamu *"),
            "{text}"
        );
        assert!(!text.contains("dia sebesar"), "self pakai 'kamu': {text}");
        assert_eq!(mentions, vec![ME.to_string()]);
    }

    #[test]
    fn mention_lain_mengambil_kalimat_dia_dan_mention_jid_penuh() {
        let (text, mentions) = split(&run(ctx_with(Some(OTHER))));
        assert!(
            text.starts_with("Kamu ingin ngecek tingkat kekecean @628999 yak? "),
            "{text}"
        );
        assert!(
            text.contains("Tingkat kekecean dia sebesar *"),
            "{text}"
        );
        assert_eq!(mentions, vec![OTHER.to_string()]);
    }

    #[test]
    fn device_ikut_teks_tapi_mention_penuh() {
        let jid = "628999:3@s.whatsapp.net";
        let (text, mentions) = split(&run(ctx_with(Some(jid))));
        // JS `mentioned.split('@')[0]` membuang server saja; `:device` tetap
        // tampil (lihat `jid_number_strips_the_server_but_keeps_the_device`).
        assert!(text.contains("@628999:3 yak?"), "display: {text}");
        assert_eq!(mentions, vec![jid.to_string()]);
    }

    #[test]
    fn deskripsi_selalu_lima_tier_plugin_ini_dan_satu_banded_saja() {
        for _ in 0..2000 {
            let (text, _) = split(&run(ctx_with(Some(OTHER))));
            let hits: Vec<usize> = TIERS
                .iter()
                .enumerate()
                .filter(|(_, t)| text.contains(**t))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(hits.len(), 1, "harus tepat satu tier, dapat {hits:?}: {text}");
            let (lo, hi) = BANDS[hits[0]];
            let percent = percent_of(&text);
            assert!(
                (lo..=hi).contains(&percent),
                "tier {:?} dapat percent {percent}, harusnya {lo}..={hi}: {text}",
                TIERS[hits[0]]
            );
        }
    }

    #[test]
    fn mention_yang_sama_dengan_sender_mengambil_cabang_kamu() {
        // JS: `mentioned === m.sender`, jadi mention milik diri sendiri bukan
        // "orang lain" meski `mentionedJid` terisi.
        let (text, mentions) = split(&run(ctx_with(Some(ME))));
        assert!(text.contains("Tingkat"), "{text}");
        assert!(!text.contains("yak?"), "harus cabang self: {text}");
        assert_eq!(mentions, vec![ME.to_string()]);
    }

    #[test]
    fn plugin_hanya_membaca_tanpa_mutasi() {
        assert!(run(ctx_with(None)).mutations().is_empty());
        assert!(run(ctx_with(Some(OTHER))).mutations().is_empty());
    }
}
