//! `plugins/fun/mimpi.js` — port 1:1, termasuk data inline-nya.
//!
//! Dua detail yang sengaja dipertahankan:
//!
//! 1. `pick(arr)` memakai `seedNum % arr.length` (`mimpi.js:79`) — SEED dari
//!    nama, jadi dreamworld orang yang sama selalu sama. Yang acak hanya
//!    `pickMulti`. Port ini memakai `wrapping_rem` untuk meniru `%` JS yang
//!    negatif-tetap-negatif, walau `charCodeAt` tidak pernah negatif sehingga
//!    hasilnya sama persis untuk input apa pun.
//! 2. `pickMulti` mengacak dengan `sort(() => Math.random() - 0.5)`
//!    (`mimpi.js:80`), shuffle bias. Di sini Fisher-Yates — tujuannya
//!    " ambil N yang berbeda", dan kedua implementasi memenuhi itu.
//!
//! `ponytail:` `m.pushName` tidak ada di `Ctx`, jadi nama bawaannya hanya
//! nomor pengirim (`m.sender.split('@')[0]`), sama seperti cabang yang tidak
//! punya args di JS (`mimpi.js:101`).

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::jid_number;

const DREAM_LEVELS: [&str; 5] = [
    "Lucid ✨",
    "Mystic 🌟",
    "Ethereal 💫",
    "Divine 🌙",
    "Legendary 🎇",
];
const DREAM_QUALITIES: [&str; 5] = [
    "Peaceful 😌",
    "Adventure 🚀",
    "Mystical 🔮",
    "Prophecy 📖",
    "Epic 🗺️",
];
const ELEMENTS: [&str; 8] = [
    "🌊 Lautan Kristal Bercahaya",
    "🌈 Pelangi Mengambang",
    "🌺 Taman Melayang",
    "⭐ Konstelasi Hidup",
    "🌙 Bulan Kembar",
    "🏰 Kastil Awan",
    "🌋 Gunung Prisma",
    "🎭 Theater Bayangan",
];
const EVENTS: [&str; 8] = [
    "🦋 Kupu-kupu membawa pesan rahasia",
    "🎭 Topeng menari sendiri",
    "🌊 Hujan bintang jatuh ke laut",
    "🎪 Parade makhluk ajaib",
    "🌺 Bunga bernyanyi lagu kuno",
    "🎨 Lukisan menjadi hidup",
    "🎵 Musik terlihat sebagai warna",
    "⚡ Petir membentuk tangga ke langit",
];
const ENCOUNTERS: [&str; 9] = [
    "🐉 Naga Pelangi Bijaksana",
    "🧙‍♂️ Penyihir Bintang",
    "🦊 Rubah Spirit Sembilan Ekor",
    "🧝‍♀️ Peri Pembawa Mimpi",
    "🦁 Singa Kristal",
    "🐋 Paus Terbang Mistis",
    "🦅 Burung Phoenix Waktu",
    "🐢 Kura-kura Pembawa Dunia",
    "🦄 Unicorn Dimensi",
];
const POWERS: [&str; 8] = [
    "✨ Mengendalikan Waktu",
    "🌊 Berbicara dengan Elemen",
    "🎭 Shapeshifting",
    "🌈 Manipulasi Realitas",
    "👁️ Penglihatan Masa Depan",
    "🎪 Teleportasi Dimensi",
    "🌙 Penyembuhan Spiritual",
    "⚡ Energi Kosmik",
];
const MESSAGES: [&str; 8] = [
    "Perjalananmu akan membawa perubahan besar",
    "Rahasia kuno akan terungkap dalam waktu dekat",
    "Kekuatan tersembunyi akan segera bangkit",
    "Takdir baru menanti di horizon",
    "Koneksi spiritual akan menguat",
    "Transformasi besar akan terjadi",
    "Pencerahan akan datang dari arah tak terduga",
    "Misi penting akan segera dimulai",
];

/// `Array.from(seed).reduce((acc, c) => acc + c.charCodeAt(0), 0)`.
///
/// Rust mengiterasi `char` (UTF-8 scalar), JS mengiterasi unit UTF-16. Untuk
/// nama Indonesia keduanya sama; untuk emoji atau Tuits (supes plan), satu
/// emoji di JS = 2 unit surrogate dan di Rust = 1 char, sehingga seed-nya
/// berbeda dan dreamworld-nya berbeda.
pub(crate) fn seed_num(seed: &str) -> u32 {
    seed.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32))
}

/// `pick(arr) = arr[seedNum % arr.length]`
fn pick_by_seed<T>(arr: &[T], seed: u32) -> &T {
    &arr[(seed % arr.len() as u32) as usize]
}

/// `pickMulti(arr, n)` — n elemen berbeda, urutan acak.
fn pick_multi<T: Clone>(arr: &[T], n: usize) -> Vec<T> {
    use rand::Rng as _;
    let mut pool: Vec<T> = arr.to_vec();
    let mut rng = rand::rng();
    for i in (1..pool.len()).rev() {
        pool.swap(i, rng.random_range(0..=i));
    }
    pool.truncate(n);
    pool
}

/// Badan reply, terpisah supaya bisa diuji tanpa `Ctx`.
pub(crate) fn render(name: &str) -> String {
    let seed = seed_num(name);
    let level = pick_by_seed(&DREAM_LEVELS, seed);
    let quality = pick_by_seed(&DREAM_QUALITIES, seed);
    let message = pick_by_seed(&MESSAGES, seed);

    let mut t = String::from("╭═══❯ *🌙 DREAM WORLD* ❮═══\n");
    t.push_str(&format!("│ 👤 *Explorer:* {name}\n"));
    t.push_str(&format!("│ ⭐ *Level:* {level}\n"));
    t.push_str(&format!("│ 💫 *Quality:* {quality}\n"));
    for (header, items) in [
        ("🌈 Elements:", pick_multi(&ELEMENTS, 3)),
        ("🎪 Events:", pick_multi(&EVENTS, 2)),
        ("🌟 Encounters:", pick_multi(&ENCOUNTERS, 2)),
        ("💫 Powers:", pick_multi(&POWERS, 2)),
    ] {
        t.push_str(&format!("│ {header}\n"));
        for item in items {
            t.push_str(&format!("│ ├ {item}\n"));
        }
    }
    t.push_str("│ 🔮 *Message:*\n");
    t.push_str(&format!("│ {message}\n"));
    t.push_str("╰════════════════════");
    t
}

/// Nama yang dipakai: `m.args.join(' ')`, kalau kosong `m.pushName`, kalau
/// masih kosong nomor pengirim (`mimpi.js:101`).
///
/// Penting: bukan [`crate::common::body_stripped`]. Helper itu merekonstruksi
/// `m.body.slice(1)` yang MASIH memuat kata command — benar untuk plugin `?`
/// yang memang menulis ulang command-nya, tapi salah di sini: tanpa argumen
/// hasilnya `"mimpi"`, bukan nomor pengirim.
pub(crate) fn explorer_name(ctx: &Ctx) -> String {
    let body = ctx.args.join(" ");
    if body.is_empty() {
        jid_number(&ctx.sender).to_string()
    } else {
        body
    }
}

/// Seluruh jalur handler, sebagai teks.
pub(crate) fn reply_for(ctx: &Ctx) -> String {
    render(&explorer_name(ctx))
}

#[plugin(
    name = "mimpi",
    alias = ["dream", "dreamworld"],
    category = "fun",
    description = "Jelajahi dunia mimpimu berdasarkan nama",
    usage = ".mimpi <nama>",
    example = ".mimpi Keisya",
    cooldown = 15,
    energi = 1,
    is_private = false
)]
pub fn mimpi(ctx: Ctx) -> Reply {
    Reply::text(reply_for(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::all_plugins;

    fn ctx_of(args: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "mimpi".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    fn meta() -> &'static oktz_plugin_api::PluginMeta {
        &oktz_plugin_api::find("mimpi").unwrap().meta
    }

    /// Jumlah baris `│ ├ ` milik satu bagian, dihitung dari header sampai
    /// header berikutnya.
    fn bullets_under(text: &str, header: &str) -> usize {
        let start = text.find(header).expect("header tidak ada") + header.len();
        text[start..]
            .lines()
            .skip_while(|l| l.is_empty())
            .take_while(|l| l.starts_with("│ ├ "))
            .count()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        // `mimpi.js:8-23` persis, termasuk `energi: 1`.
        let p = meta();
        assert_eq!(p.name, "mimpi");
        assert_eq!(p.alias, ["dream", "dreamworld"]);
        assert_eq!(p.category, "fun");
        assert_eq!(p.description, "Jelajahi dunia mimpimu berdasarkan nama");
        assert_eq!(p.usage, ".mimpi <nama>");
        assert_eq!(p.example, ".mimpi Keisya");
        assert_eq!(p.cooldown, 15);
        assert_eq!(p.energi, 1);
        assert!(!p.is_group && !p.is_private && p.is_enabled);
        assert!(!p.is_owner && !p.is_premium && !p.is_admin);
    }

    #[test]
    fn every_alias_resolves_and_nothing_collides() {
        for key in ["mimpi", "dream", "dreamworld"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.clone()),
                Some("mimpi".to_string()),
                "alias {key} tidak resolve"
            );
            let owners: Vec<&str> = all_plugins()
                .iter()
                .filter(|e| e.meta.matches(key))
                .map(|e| e.meta.name.as_str())
                .collect();
            assert_eq!(owners, ["mimpi"], "key {key} diklaim oleh {owners:?}");
        }
    }

    // -- seed: cabang yang membuat dreamworld deterministik ---------------

    #[test]
    fn the_seed_is_the_sum_of_char_codes() {
        assert_eq!(seed_num(""), 0);
        assert_eq!(seed_num("A"), 65);
        assert_eq!(seed_num("AB"), 65 + 66);
        assert_eq!(seed_num("Keisya"), 75 + 101 + 105 + 115 + 121 + 97);
    }

    #[test]
    fn the_seeded_pick_never_leaves_the_table() {
        // Hanya 5 slot level tapi seed bisa nilai apa pun — ini yang membuat
        // `mod` (bukan `random_index`) wajib dipakai.
        for name in ["", "a", "Keisya", "😀 emoji", "超长名字测试"] {
            let seed = seed_num(name);
            assert!(DREAM_LEVELS.contains(pick_by_seed(&DREAM_LEVELS, seed)), "{name}");
            assert!(DREAM_QUALITIES.contains(pick_by_seed(&DREAM_QUALITIES, seed)), "{name}");
            assert!(MESSAGES.contains(pick_by_seed(&MESSAGES, seed)), "{name}");
        }
    }

    #[test]
    fn the_seeded_rows_are_identical_for_the_same_name() {
        // `pick` sengaja tidak memakai acak: nama sama -> dreamworld sama.
        let out1 = render("Keisya");
        let out2 = render("Keisya");
        for row in [
            format!("│ ⭐ *Level:* {}", pick_by_seed(&DREAM_LEVELS, seed_num("Keisya"))),
            format!("│ 💫 *Quality:* {}", pick_by_seed(&DREAM_QUALITIES, seed_num("Keisya"))),
            format!("│ {}", pick_by_seed(&MESSAGES, seed_num("Keisya"))),
        ] {
            assert!(out1.contains(&row), "baris seed hilang: {row}\n{out1}");
        }
        // Bagian acak boleh berbeda, jadi yang dibandingkan hanya baris seed.
        let seeded = |t: &str| -> Vec<String> {
            t.lines()
                .filter(|l| l.contains("Level:") || l.contains("Quality:") || l.contains("│ 🔮"))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(seeded(&out1), seeded(&out2));
    }

    #[test]
    fn different_names_can_land_on_different_levels() {
        // Kalau semua nama jatuh ke level yang sama, seed-nya tidak dipakai.
        let levels: std::collections::HashSet<&str> = (0..40)
            .flat_map(|_| DREAM_LEVELS)
            .filter(|l| (0..40).any(|i| render(&format!("user{i}")).contains(*l)))
            .collect();
        assert!(levels.len() > 1, "seed tidak memengaruhi pilihan: {levels:?}");
    }

    // -- struktur kartu ---------------------------------------------------

    #[test]
    fn the_card_has_every_section_and_the_closing_border() {
        let out = render("Keisya");
        for h in [
            "╭═══❯ *🌙 DREAM WORLD* ❮═══",
            "│ 👤 *Explorer:* Keisya",
            "│ ⭐ *Level:* ",
            "│ 💫 *Quality:* ",
            "│ 🌈 Elements:",
            "│ 🎪 Events:",
            "│ 🌟 Encounters:",
            "│ 💫 Powers:",
            "│ 🔮 *Message:*",
        ] {
            assert!(out.contains(h), "bagian hilang: {h:?}\n{out}");
        }
        assert!(out.ends_with("╰════════════════════"), "{out}");
    }

    #[test]
    fn the_item_counts_are_exactly_three_two_two_and_two() {
        let out = render("Apu");
        assert_eq!(bullets_under(&out, "│ 🌈 Elements:"), 3);
        assert_eq!(bullets_under(&out, "│ 🎪 Events:"), 2);
        assert_eq!(bullets_under(&out, "│ 🌟 Encounters:"), 2);
        assert_eq!(bullets_under(&out, "│ 💫 Powers:"), 2);
    }

    #[test]
    fn pick_multi_never_repeats_respects_the_count_and_stays_inside_the_table() {
        for n in [0usize, 1, 2, 3, 8, 9] {
            let picked = pick_multi(&ELEMENTS, n);
            assert_eq!(picked.len(), n.min(ELEMENTS.len()), "n={n}");
            for e in &picked {
                assert!(ELEMENTS.contains(e), "elemen asing: {e}");
            }
            let mut uniq = picked.clone();
            uniq.sort();
            uniq.dedup();
            assert_eq!(uniq.len(), picked.len(), "n={n} ada elemen dobel");
        }
        // ENCOUNTERS cuma 9 item — minta lebih dari 9 tidak boleh panic.
        assert_eq!(pick_multi(&ENCOUNTERS, 20).len(), 9);
    }

    // -- cabang nama ------------------------------------------------------

    #[test]
    fn no_argument_falls_back_to_the_sender_number_with_the_device_suffix_kept() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mimpi".into(),
            sender: "628111:5@s.whatsapp.net".into(),
            ..Default::default()
        };
        assert_eq!(explorer_name(&ctx), "628111:5");
        assert!(reply_for(&ctx).contains("│ 👤 *Explorer:* 628111:5"));
    }

    #[test]
    fn a_multi_word_argument_keeps_its_spaces_and_drives_the_seed() {
        // Router sudah memotong prefix+command sebelum mengisi `Ctx::text`,
        // jadi yang dipakai persis seperti diketik setelah command.
        let out = reply_for(&ctx_of(&["Keisya", "Putri"]));
        assert!(out.contains("│ 👤 *Explorer:* Keisya Putri"), "{out}");
        // Seed dihitung dari nama GABUNGAN, bukan dari `args[0]` saja.
        assert!(out.contains(&format!(
            "│ ⭐ *Level:* {}",
            pick_by_seed(&DREAM_LEVELS, seed_num("Keisya Putri"))
        )));
    }

    #[test]
    fn an_empty_argument_is_the_fallback_case_not_a_blank_name() {
        // `m.args.join(' ')` = "" → `pushName` → nomor pengirim. `Ctx` tidak
        // punya pushName, jadi lompatan kedua yang hilang; nomor pengirim yang
        // dipakai, sama seperti cabang JS yang tidak punya args.
        assert_eq!(explorer_name(&ctx_of(&[])), "628111");
    }

    #[test]
    fn the_registered_handler_always_answers_with_text() {
        for args in [vec![], vec!["Keisya"], vec!["a", "long", "name"], vec!["😀"]] {
            let reply = mimpi(ctx_of(&args));
            let body = reply.text_body().unwrap_or_else(|| panic!("{args:?} tidak membalas"));
            assert!(body.starts_with("╭═══❯ *🌙 DREAM WORLD*"), "{body}");
            // Nama command tidak boleh bocor ke kartu.
            assert!(!body.contains("*Explorer:* mimpi"), "{body}");
        }
    }
}
