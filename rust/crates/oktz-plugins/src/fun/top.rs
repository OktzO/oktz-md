//! `plugins/fun/top.js` — "random top 5" dari roster grup.
//!
//! Dua hal dari JS sengaja dipertahankan apa adanya:
//!
//! 1. `members.sort(() => Math.random() - 0.5)` (`top.js:41`) adalah shuffle
//!    **bias**, bukan Fisher-Yates, dan `Array.prototype.sort` di V8 sudah
//!    TimSort — hasilnya tidak boleh dianggap acak sempurna. Di sini memakai
//!    shuffle yang benar (Fisher-Yates) karena tujuannya "memberi urutan
//!    berbeda tiap jalan", dan membandingkan dua implementasi tidak
//!    membuahkan apa pun. Lihat [`shuffled`].
//! 2. Pengecekannya `< 2` tapi pesannya berbunyi "kurang dari 5 orang"
//!    (`top.js:38-40`). Pesan itu tidak pernah akurat di JS juga —
//!    dipertahankan supaya output tidak berubah.
//!
//! `ponytail:` JS memfilter JID bot sendiri dari kandidat
//! (`id !== sock.user?.id`, `top.js:38`) dan `GroupRoster` tidak menyimpan JID
//! bot, jadi bot bisa ikut masuk daftar. Tambahkan `bot_jid` ke `GroupRoster`
//! kalau ini terasa mengganggu.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, roster_jids};

const MEDALS: [&str; 5] = ["🥇", "🥈", "🥉", "4️⃣", "5️⃣"];

/// Fisher-Yates dengan sumber acak dari `rand`.
fn shuffled(members: &mut [String]) {
    use rand::Rng as _;
    let mut rng = rand::rng();
    for i in (1..members.len()).rev() {
        members.swap(i, rng.random_range(0..=i));
    }
}

/// Badan balasan, terpisah dari handler supaya bisa diuji dengan roster
/// dan kategori yang ditentukan.
pub(crate) fn build(members: &mut [String], kategori: &str) -> Result<(String, Vec<String>), String> {
    if members.len() < 2 {
        return Err("❌ Member grup kurang dari 5 orang!".to_string());
    }
    shuffled(members);
    let top5: Vec<String> = members.iter().take(5).cloned().collect();

    let mut list = String::new();
    for (i, jid) in top5.iter().enumerate() {
        list.push_str(&format!("*{}* {} @{}\n", i + 1, MEDALS[i], jid_number(jid)));
    }
    Ok((
        format!("🏆 *ᴛᴏᴘ 5 {}*\n{list}", kategori.to_uppercase()),
        top5,
    ))
}

#[plugin(
    name = "top",
    alias = ["top5", "toplist"],
    category = "fun",
    description = "Random top 5 member untuk kategori tertentu",
    usage = ".top <kategori>",
    example = ".top orang pintar",
    cooldown = 10,
    energi = 0,
    is_private = false,
    is_group = true
)]
pub fn top(ctx: Ctx) -> Reply {
    let kategori = ctx.args.join(" ").trim().to_string();
    if kategori.is_empty() {
        return Reply::text(format!("\\`Contoh: {}top orang pintar\\`", ctx.prefix));
    }
    let mut members = roster_jids(&ctx);
    match build(&mut members, &kategori) {
        Ok((text, mentions)) => Reply::Text { text, mentions },
        Err(msg) => Reply::text(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::all_plugins;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn roster(n: usize) -> GroupRoster {
        GroupRoster {
            participants: (0..n)
                .map(|i| Participant {
                    jid: format!("6281{i:02}@s.whatsapp.net"),
                    is_admin: false,
                    is_owner: false,
                })
                .collect(),
            is_bot_admin: false,
        }
    }

    fn ctx_of(n: usize, args: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "top".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: roster(n),
            ..Default::default()
        }
    }

    fn meta() -> &'static oktz_plugin_api::PluginMeta {
        &oktz_plugin_api::find("top").unwrap().meta
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        // top.js:5-21 persis.
        let p = meta();
        assert_eq!(p.name, "top");
        assert_eq!(p.alias, ["top5", "toplist"]);
        assert_eq!(p.category, "fun");
        assert_eq!(p.description, "Random top 5 member untuk kategori tertentu");
        assert_eq!(p.usage, ".top <kategori>");
        assert_eq!(p.example, ".top orang pintar");
        assert_eq!(p.cooldown, 10);
        assert_eq!(p.energi, 0);
        assert!(p.is_group && !p.is_private && p.is_enabled);
        assert!(!p.is_owner && !p.is_premium && !p.is_admin);
    }

    #[test]
    fn every_alias_resolves_and_nothing_collides() {
        for a in ["top5", "toplist"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "top");
        }
        for key in ["top", "top5", "toplist"] {
            let owners: Vec<&str> = all_plugins()
                .iter()
                .filter(|e| e.meta.matches(key))
                .map(|e| e.meta.name.as_str())
                .collect();
            assert_eq!(owners, ["top"], "key {key} diklaim oleh {owners:?}");
        }
    }

    #[test]
    fn a_missing_category_answers_with_the_usage_hint_and_no_prefix_substitution() {
        // `m.args.join(' ')` untuk `.top` saja = "" → cabang ini.
        for empty in [vec![], vec!["".to_string()]] {
            let reply = top(Ctx {
                args: empty,
                ..ctx_of(10, &[])
            });
            assert_eq!(reply.text_body(), Some("\\`Contoh: .top orang pintar\\`"));
        }
    }

    #[test]
    fn the_category_is_upper_cased_in_the_heading() {
        let mut m: Vec<String> = roster_jids(&ctx_of(8, &[]));
        let (text, _) = build(&mut m, "orang pintar").unwrap();
        assert!(text.starts_with("🏆 *ᴛᴏᴘ 5 ORANG PINTAR*\n"), "{text}");
    }

    #[test]
    fn the_list_is_numbered_with_the_medals_and_mentions_exactly_five() {
        let mut m: Vec<String> = roster_jids(&ctx_of(12, &[]));
        let (text, mentions) = build(&mut m, "x").unwrap();
        assert_eq!(mentions.len(), 5, "harus tepat 5 walau roster 12");
        for (i, medal) in MEDALS.iter().enumerate() {
            assert!(
                text.contains(&format!("*{}* {medal} @", i + 1)),
                "baris {i} hilang: {text}"
            );
        }
        // Lima mention harus muncul persis di teks dan tidak lebih.
        for jid in &mentions {
            assert!(text.contains(&format!("@{}", jid_number(jid))), "{jid} tidak termed: {text}");
        }
        assert_eq!(
            text.lines().filter(|l| {
                let mut c = l.chars();
                c.next() == Some('*') && c.next().is_some_and(|d| d.is_ascii_digit())
            }).count(),
            5,
            "teks harus punya 5 baris nama: {text}"
        );
        // Tidak ada anggota yang terulang di lima teratas.
        let mut uniq = mentions.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 5, "ada anggota dobel: {mentions:?}");
    }

    #[test]
    fn a_roster_of_two_still_produces_a_list_because_js_only_checks_below_two() {
        let mut m: Vec<String> = roster_jids(&ctx_of(2, &[]));
        let (text, mentions) = build(&mut m, "x").unwrap();
        assert_eq!(mentions.len(), 2);
        assert!(!text.contains("kurang dari 5"), "{text}");
    }

    #[test]
    fn zero_or_one_member_hits_the_error_branch_with_the_js_wording() {
        for n in [0usize, 1] {
            let mut m: Vec<String> = roster_jids(&ctx_of(n, &[]));
            assert_eq!(
                build(&mut m, "x").unwrap_err(),
                "❌ Member grup kurang dari 5 orang!"
            );
            let reply = top(ctx_of(n, &["orang", "pintar"]));
            assert_eq!(reply.text_body(), Some("❌ Member grup kurang dari 5 orang!"));
        }
    }

    #[test]
    fn shuffling_reorders_and_never_loses_or_duplicates_a_member() {
        let base: Vec<String> = (0..30).map(|i| format!("6281{i:02}@s.whatsapp.net")).collect();
        for _ in 0..50 {
            let mut m = base.clone();
            shuffled(&mut m);
            assert_eq!(m.len(), base.len());
            assert!(m.iter().all(|x| base.contains(x)), "ada anggota asing");
            // Per iterasi: himpunan BARU dihitung ulang — kalau dipakai ulang,
            // iterasi kedua pasti gagal padahal shuffle-nya benar.
            let mut uniq = m.clone();
            uniq.sort();
            uniq.dedup();
            assert_eq!(uniq.len(), base.len(), "anggota hilang atau dobel: {m:?}");
        }
    }

    #[test]
    fn shuffling_a_short_slice_does_not_panic() {
        for n in 0..5 {
            let mut m: Vec<String> = (0..n).map(|i| format!("6281{i}@s.whatsapp.net")).collect();
            shuffled(&mut m);
            assert_eq!(m.len(), n);
        }
    }

    #[test]
    fn the_registered_handler_always_answers_with_text() {
        for n in [0usize, 1, 2, 9] {
            assert!(top(ctx_of(n, &["x"])).text_body().is_some(), "n={n}");
            assert!(top(ctx_of(n, &[])).text_body().is_some(), "n={n} tanpa args");
        }
    }
}