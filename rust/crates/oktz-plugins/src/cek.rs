//! 10 plugin `cek*` seragam: `floor(r*101)` + ambang 90/70/50/30,
//! target `mentioned[0] || sender`, dua varian kalimat (self vs orang
//! lain). Yang di-share: RNG + kerangka teks.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `Math.floor(Math.random() * 101)` — inklusif 0..=100, bukan 0..100.
fn percent_101() -> u32 {
    use rand::Rng as _;
    rand::rng().random_range(0..101)
}

/// Bangun balasan `cek*`.
///
/// `target` = mentions[0] atau sender. `label` = kata yang disisipkan
/// ("kecreativean"), `tiers` = 5 deskripsi berurutan dari ambang tertinggi.
fn cek_reply(ctx: &Ctx, percent: u32, label: &str, tiers: [&str; 5]) -> Reply {
    let target = ctx.mention_or_sender().to_string();
    let number = target.split('@').next().unwrap_or(&target);
    let is_self = target == ctx.sender;

    let desc = if percent >= 90 {
        tiers[0]
    } else if percent >= 70 {
        tiers[1]
    } else if percent >= 50 {
        tiers[2]
    } else if percent >= 30 {
        tiers[3]
    } else {
        tiers[4]
    };

    let txt = if is_self {
        format!("Hai @{number}\n    \nTingkat {label} kamu *{percent}%*\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat {label} @{number} yak? \n    \nTingkat {label} dia sebesar *{percent}%*\n```{desc}```"
        )
    };

    Reply::Text {
        text: txt,
        mentions: vec![target],
    }
}

macro_rules! cek_plugin {
    ($fn_name:ident, $name:literal, $alias:expr, $desc:literal, $label:literal,
     [$t0:literal, $t1:literal, $t2:literal, $t3:literal, $t4:literal]) => {
        #[plugin(
            name = $name,
            alias = $alias,
            category = "cek",
            description = $desc,
            usage = concat!(".", $name, " <nama>"),
            example = concat!(".", $name, " Budi"),
            cooldown = 5,
            energi = 0,
            is_private = false
        )]
        pub fn $fn_name(ctx: Ctx) -> Reply {
            cek_reply(&ctx, percent_101(), $label, [$t0, $t1, $t2, $t3, $t4])
        }
    };
}

cek_plugin!(
    cekbaik, "cekbaik", ["baik", "kind"], "Cek seberapa baik kamu", "kebaikan",
    [
        "Mantappp! kamu adalah orang paling Baik di dunia ini! 😇✨",
        "Baik hati dan tidak sombong! 💝",
        "Lumayan baik 😊",
        "Sedikit baik 🙂",
        "Hmm, perlu introspeksi ?? 🤔"
    ]
);

cek_plugin!(
    cekbucin, "cekbucin", ["bucin"], "Cek seberapa bucin kamu", "kebucinan",
    [
        "BUCIN AKUT! Udah gabisa diselamatkan 😭💔",
        "Bucin parah nih~ 🥺",
        "Lumayan bucin 💕",
        "Sedikit bucin 😊",
        "Santai aja, gak bucin 😎"
    ]
);

cek_plugin!(
    cekcantik, "cekcantik", ["cantik", "beautiful"], "Cek seberapa cantik kamu", "kecantikan",
    [
        "Cantik banget kayak bidadari! 👸✨",
        "Cantik banget! 💕",
        "Manis dan cantik~ 🌸",
        "Lumayan cantik 😊",
        "Tetep cantik kok! 💖"
    ]
);

cek_plugin!(
    cekcreative, "cekcreative", ["creative", "kreatif"], "Cek tingkat kreativitas kamu", "kecreativean",
    [
        "SUPER KREATIF! Artis sejati! 🎨✨",
        "Imajinatif banget! 💡",
        "Cukup kreatif 😊",
        "Biasa aja sih 🤔",
        "Kurang imajinasi nih 😅"
    ]
);

cek_plugin!(
    cekcupu, "cekcupu", ["cupu", "noob"], "Cek tingkat kecupuan kamu", "kecupuan",
    [
        "CUPU PARAH! NOOB DETECTED! 🤡",
        "Masih newbie nih~ 😅",
        "Biasa aja lah 🤔",
        "Cukup jago! 💪",
        "PRO PLAYER! GG! 🏆"
    ]
);

cek_plugin!(
    cekgabut, "cekgabut", ["gabut", "bored"], "Cek tingkat keGabutan kamu", "kegabutan",
    [
        "GABUT LEVEL MAX! Main bot aja~ 🥱",
        "Gabut parah nih! 😴",
        "Lumayan gabut 😅",
        "Agak sibuk dikit 📝",
        "Sibuk banget! Produktif! 💼"
    ]
);

cek_plugin!(
    cekgacha, "cekgacha", ["luck"], "Cek hoki gacha kamu", "kegachaan",
    [
        "HOKI PARAH! SSR GUARANTEED! ✨💎",
        "Lucky! Pasti dapet SR keatas! 🍀",
        "Hoki-hoki dikit 😊",
        "Hmm... pray harder! 🙏",
        "SIAL! Nanti aja gachanya! 💔"
    ]
);

cek_plugin!(
    cekgamer, "cekgamer", ["gamer", "pro"], "Cek seberapa pro gamer kamu", "kegameran",
    [
        "PRO PLAYER! Esports level! 🏆",
        "Jago banget! 🎮",
        "Lumayan pro 👍",
        "Masih noob nih 😅",
        "Mending main masak-makanan 🍳"
    ]
);

cek_plugin!(
    cekganteng, "cekganteng", ["ganteng", "handsome"], "Cek seberapa ganteng kamu", "kegantengan",
    [
        "Ganteng maksimal! 😍🔥",
        "Ganteng banget! 😎",
        "Lumayan ganteng~ 👍",
        "Biasa aja sih 😅",
        "Mungkin inner beauty? 🤭"
    ]
);

cek_plugin!(
    cekgila, "cekgila", ["gila", "crazy"], "Cek seberapa gila kamu", "kegilaan",
    [
        "GILA BENERAN! Masuk RSJ! 🤪",
        "Hampir gila 😵",
        "Lumayan waras 😅",
        "Normal kok 🙂",
        "Waras banget! 😇"
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_can_reach_both_ends() {
        // Math.random()*101 inklusif 100 — kalau range-nya 0..100, nilai 100
        // tak akan pernah muncul dan tier teratas jadi dead branch.
        let mut saw_high = false;
        for _ in 0..2000 {
            let p = percent_101();
            assert!(p <= 100, "diluar range: {p}");
            if p == 100 {
                saw_high = true;
            }
        }
        assert!(saw_high, "nilai 100 harus mungkin terjadi");
    }

    fn ctx_with_mention(mention: Option<&str>) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            mentioned: mention.map(str::to_string).into_iter().collect(),
            ..Default::default()
        }
    }

    const TIERS: [&str; 5] = ["tier-a", "tier-b", "tier-c", "tier-d", "tier-e"];

    fn desc_for(percent: u32) -> &'static str {
        if percent >= 90 {
            TIERS[0]
        } else if percent >= 70 {
            TIERS[1]
        } else if percent >= 50 {
            TIERS[2]
        } else if percent >= 30 {
            TIERS[3]
        } else {
            TIERS[4]
        }
    }

    #[test]
    fn tier_thresholds_match_js_bands_exactly() {
        // Batas inklusif, persis seperti `percent >= N` di JS.
        for (percent, expected) in [
            (100, "tier-a"),
            (90, "tier-a"),
            (89, "tier-b"),
            (70, "tier-b"),
            (69, "tier-c"),
            (50, "tier-c"),
            (49, "tier-d"),
            (30, "tier-d"),
            (29, "tier-e"),
            (0, "tier-e"),
        ] {
            assert_eq!(desc_for(percent), expected, "percent {percent}");
        }
    }

    #[test]
    fn self_reply_says_kamu_and_mentions_self() {
        let ctx = ctx_with_mention(None);
        let r = cek_reply(&ctx, 95, "kecreativean", TIERS);
        match &r {
            Reply::Text { text, mentions } => {
                assert!(text.contains("Tingkat kecreativean kamu *95%*"), "{text}");
                assert!(!text.contains("dia sebesar"), "self pakai 'kamu': {text}");
                assert_eq!(mentions, &["628111@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn other_target_says_dia_and_mentions_target() {
        let ctx = ctx_with_mention(Some("628999@s.whatsapp.net"));
        let r = cek_reply(&ctx, 12, "kecreativean", TIERS);
        match &r {
            Reply::Text { text, mentions } => {
                assert!(text.contains("@628999 yak?"), "{text}");
                assert!(text.contains("dia sebesar *12%*"), "{text}");
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn mention_strips_server_and_device_from_display() {
        let ctx = ctx_with_mention(Some("628999:3@s.whatsapp.net"));
        let r = cek_reply(&ctx, 75, "x", TIERS);
        match &r {
            Reply::Text { text, mentions } => {
                assert!(text.contains("@628999"), "display harus polos: {text}");
                assert_eq!(mentions, &["628999:3@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn all_101_percent_values_select_exactly_one_tier() {
        // Guard regression: kalau ada ambang yang salah, ada nilai yang jatuh
        // ke tier yang salah — test ini menutup semua nilai, bukan sampel.
        for percent in 0..=100u32 {
            let ctx = ctx_with_mention(None);
            let r = cek_reply(&ctx, percent, "x", TIERS);
            let text = r.text_body().expect("harus ada teks");
            assert!(
                text.contains(&format!("*{percent}%*")),
                "percent {percent} tidak muncul: {text}"
            );
            let expected = desc_for(percent);
            assert!(
                text.contains(expected),
                "percent {percent} harusnya {expected}: {text}"
            );
            // Tepat satu tier yang muncul (tier unik per band).
            let hits = TIERS.iter().filter(|t| text.contains(**t)).count();
            assert_eq!(hits, 1, "percent {percent} memilih {hits} tier: {text}");
        }
    }

    #[test]
    fn all_ten_plugins_register_with_correct_meta() {
        for name in [
            "cekbaik", "cekbucin", "cekcantik", "cekcreative", "cekcupu", "cekgabut",
            "cekgacha", "cekgamer", "cekganteng", "cekgila",
        ] {
            let p = oktz_plugin_api::find(name)
                .unwrap_or_else(|| panic!("plugin {name} tidak terdaftar"));
            assert_eq!(p.meta.name, name);
            assert_eq!(p.meta.category, "cek");
            assert_eq!(p.meta.cooldown, 5);
            assert_eq!(p.meta.energi, 0);
        }
    }

    #[test]
    fn aliases_resolve_to_the_right_plugin() {
        assert_eq!(oktz_plugin_api::find("baik").unwrap().meta.name, "cekbaik");
        assert_eq!(oktz_plugin_api::find("luck").unwrap().meta.name, "cekgacha");
        assert_eq!(oktz_plugin_api::find("creative").unwrap().meta.name, "cekcreative");
    }

    #[test]
    fn a_registered_plugin_returns_a_valid_reply() {
        let p = oktz_plugin_api::find("cekgacha").unwrap();
        let reply = (p.handler)(ctx_with_mention(Some("628999@s.whatsapp.net")));
        assert!(reply.text_body().is_some(), "plugin harus membalas");
    }
}
