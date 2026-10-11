//! 6 plugin `cek*` yang tidak cocok dengan makro 4-tier di `cek.rs` —
//! masing-masing punya ambang dan offset RNG sendiri. Tabel aslinya ada
//! di commit message; angka asli dipin test tiap plugin.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `Math.floor(Math.random() * n)` — 0..=n-1, jadi range Rust `0..n` (n eksklusif).
fn roll(n: u32) -> u32 {
    use rand::Rng as _;
    rand::rng().random_range(0..n)
}

/// `mentioned.split('@')[0]` + apakah target = sender.
///
/// Dua kalimat tiap plugin (self "kamu" vs orang lain "dia") ditentukan oleh
/// `is_self`; nomor yang ditampilkan sudah dibuang `@server` dan `:device`.
fn num_and_self(ctx: &Ctx) -> (&str, bool) {
    let target = ctx.mention_or_sender();
    (
        target.split('@').next().unwrap_or(target),
        target == ctx.sender,
    )
}

// ---------------------------------------------------------------- cekkarma

/// `floor(rand * 101)` -> 0..=100.
fn karma_percent() -> u32 {
    roll(101)
}

/// Ambang persis `cekkarma.js:22-26` — semua `>=`, semua kelipatan 20.
fn karma_desc(percent: u32) -> &'static str {
    if percent >= 80 {
        "Karma baik! Surga menantimu~ ✨"
    } else if percent >= 60 {
        "Cukup baik, terus tingkatkan! 🙏"
    } else if percent >= 40 {
        "Netral, perbanyak kebaikan~ ⚖️"
    } else if percent >= 20 {
        "Hati-hati dengan karma buruk! ⚠️"
    } else {
        "Wah perlu banyak tobat nih... 😱"
    }
}

fn karma_text(ctx: &Ctx, percent: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = karma_desc(percent);
    if is_self {
        format!("Hai @{number}\n    \nTingkat kekarmaan kamu *{percent}%\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kekarmaan @{number} yak? \n    \nTingkat kekarmaan dia sebesar *{percent}%\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekkarma",
    alias = ["karma"],
    category = "cek",
    description = "Cek tingkat karma kamu",
    usage = ".cekkarma <nama>",
    example = ".cekkarma Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekkarma(ctx: Ctx) -> Reply {
    Reply::Text {
        text: karma_text(&ctx, karma_percent()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

// ----------------------------------------------------------------- cekumur

/// `floor(rand * 80) + 5` -> 5..=84. Offset ditambahkan SETELAH floors.
fn umur_percent() -> u32 {
    roll(80) + 5
}

/// Empat tier — `cekumur.js:22-25`, semua `>=`.
fn umur_desc(percent: u32) -> &'static str {
    if percent >= 60 {
        "Bijaksana seperti orang tua! 🧓"
    } else if percent >= 40 {
        "Dewasa dan matang~ 🧑"
    } else if percent >= 20 {
        "Jiwa muda! 🧒"
    } else {
        "Masih seperti anak kecil~ 👶"
    }
}

fn umur_text(ctx: &Ctx, percent: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = umur_desc(percent);
    if is_self {
        format!("Hai @{number}\n    \nTingkat keumuran kamu *{percent}%\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat keumuran @{number} yak? \n    \nTingkat keumuran dia sebesar *{percent}%\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekumur",
    alias = ["umur", "age"],
    category = "cek",
    description = "Cek umur mental kamu",
    usage = ".cekumur <nama>",
    example = ".cekumur Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekumur(ctx: Ctx) -> Reply {
    Reply::Text {
        text: umur_text(&ctx, umur_percent()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

// ---------------------------------------------------------------- cekberat

/// `floor(rand * 60) + 40` -> 40..=99, yaitu rentang Rust `40..100`.
fn berat_kg() -> u32 {
    roll(60) + 40
}

/// `cekberat.js:22-32`. Ambang 55 dan 45 (bukan kelipatan 10) — inilah yang
/// membedakannya dari `cek.rs`.
fn berat_desc(kg: u32) -> &'static str {
    if kg >= 90 {
        "Big boy/girl! 💪"
    } else if kg >= 70 {
        "Berisi dan sehat! 😊"
    } else if kg >= 55 {
        "Ideal banget! 👍"
    } else if kg >= 45 {
        "Langsing nih~ 🌸"
    } else {
        "Kurus banget, makan yang banyak! 🍔"
    }
}

fn berat_text(ctx: &Ctx, kg: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = berat_desc(kg);
    if is_self {
        format!("Hai @{number}\n    \nBerat badan kamu *{kg} kg\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek berat badan @{number} yak? \n    \nBerat badan dia sebesar *{kg} kg\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekberat",
    alias = ["berat", "weight"],
    category = "cek",
    description = "Cek berat badan random",
    usage = ".cekberat <nama>",
    example = ".cekberat Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekberat(ctx: Ctx) -> Reply {
    Reply::Text {
        text: berat_text(&ctx, berat_kg()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

// --------------------------------------------------------------- cektinggi

/// `floor(rand * 50) + 150` -> 150..=199, yaitu rentang Rust `150..200`.
fn tinggi_cm() -> u32 {
    roll(50) + 150
}

/// `cektinggi.js:23-33`.
fn tinggi_desc(cm: u32) -> &'static str {
    if cm >= 190 {
        "TINGGI BANGET! Model basketball! 🏀"
    } else if cm >= 175 {
        "Tinggi ideal! 😎"
    } else if cm >= 165 {
        "Lumayan tinggi 👍"
    } else if cm >= 155 {
        "Standard kok 🙂"
    } else {
        "Imut dan mungil! 🥺"
    }
}

/// Kalimat "orang lain" di JS memang berbeda dari kalimat self: `tingkat
/// ketinggian` vs `Tinggi badan`. Ketidakkonsistenan itu dibiarkan — itu isi
/// file aslinya, bukan bug yang boleh dirapikan diam-diam.
fn tinggi_text(ctx: &Ctx, cm: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = tinggi_desc(cm);
    if is_self {
        format!("Hai @{number}\n    \nTinggi badan kamu *{cm} cm\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat ketinggian @{number} yak? \n    \nTinggi badan dia sebesar *{cm} cm\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cektinggi",
    alias = ["tinggi", "tall"],
    category = "cek",
    description = "Cek tinggi badan random",
    usage = ".cektinggi <nama>",
    example = ".cektinggi Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cektinggi(ctx: Ctx) -> Reply {
    Reply::Text {
        text: tinggi_text(&ctx, tinggi_cm()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

// ------------------------------------------------------------ ceksisaumur

/// Tiga undian terpisah — `ceksisaumur.js:22-24`. Semuanya 0..=n-1; hanya
/// tahun yang diberi offset.
fn sisaumur_tahun() -> u32 {
    roll(80) + 20
}
fn sisaumur_bulan() -> u32 {
    roll(12)
}
fn sisaumur_hari() -> u32 {
    roll(30)
}

/// SATU-SATUNYA plugin di batch ini yang memakai `>` bukan `>=`
/// (`ceksisaumur.js:27-34`).
///
/// Konsekuensi: `tahun == 80` -> tier kedua, `tahun == 60` -> tier ketiga,
/// `tahun == 40` -> tier keempat. Kalau dibikin `>=`, tiga nilai itu lompat satu
/// tingkat — dan bukan probabilitas kecil (masing-masing 1 dari 80 nilai).
fn sisaumur_desc(tahun: u32) -> &'static str {
    if tahun > 80 {
        "Panjang umur banget! 🎉"
    } else if tahun > 60 {
        "Lumayan panjang~ ✨"
    } else if tahun > 40 {
        "Cukup lah ya 😊"
    } else {
        "Jaga kesehatan ya! 🙏"
    }
}

fn sisaumur_text(ctx: &Ctx, tahun: u32, bulan: u32, hari: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = sisaumur_desc(tahun);
    let durasi = format!("{tahun} Tahun {bulan} Bulan {hari} Hari");
    if is_self {
        format!("Hai @{number}\n    \nSisa umur kamu *{durasi}\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kesisaumuran @{number} yak? \n    \nSisa umur dia sebesar *{durasi}\n```{desc}```"
        )
    }
}

#[plugin(
    name = "ceksisaumur",
    alias = ["sisaumur"],
    category = "cek",
    description = "Cek sisa umur kamu",
    usage = ".ceksisaumur <nama>",
    example = ".ceksisaumur Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn ceksisaumur(ctx: Ctx) -> Reply {
    Reply::Text {
        text: sisaumur_text(
            &ctx,
            sisaumur_tahun(),
            sisaumur_bulan(),
            sisaumur_hari(),
        ),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

// --------------------------------------------------------------- cekpintar

/// `floor(rand * 100) + 70` -> 70..=169, yaitu rentang Rust `70..170`.
///
/// Offset +70 inilah yang membuat kelima tier terjangkau: tanpa itu, range
/// 0..=99 membuat tier `>= 150` dan `>= 130` jadi dead branch permanen.
fn pintar_iq() -> u32 {
    roll(100) + 70
}

/// `cekpintar.js:23-33`.
fn pintar_desc(iq: u32) -> &'static str {
    if iq >= 150 {
        "JENIUS! Einstein level! 🧠✨"
    } else if iq >= 130 {
        "Sangat cerdas! 🎓"
    } else if iq >= 110 {
        "Di atas rata-rata! 👍"
    } else if iq >= 90 {
        "Normal, rata-rata 😊"
    } else {
        "Tetap semangat belajar! 📚"
    }
}

fn pintar_text(ctx: &Ctx, iq: u32) -> String {
    let (number, is_self) = num_and_self(ctx);
    let desc = pintar_desc(iq);
    if is_self {
        format!("Hai @{number}\n    \nKepintaran (IQ) kamu *{iq}\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek kepintaran @{number} yak? \n    \nKepintaran (IQ) dia sebesar *{iq}\n```{desc}```"
        )
    }
}

#[plugin(
    name = "cekpintar",
    alias = ["pintar", "iq", "smart"],
    category = "cek",
    description = "Cek seberapa pintar kamu",
    usage = ".cekpintar <nama>",
    example = ".cekpintar Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekpintar(ctx: Ctx) -> Reply {
    Reply::Text {
        text: pintar_text(&ctx, pintar_iq()),
        mentions: vec![ctx.mention_or_sender().to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn ctx_self() -> Ctx {
        Ctx {
            sender: SENDER.into(),
            ..Default::default()
        }
    }

    /// Mention orang lain. `with_device` menambah `:3` supaya display-vs-mention
    /// punya bahan: yang ditampilkan polos, yang di-mention harus persis.
    fn ctx_other(with_device: bool) -> Ctx {
        Ctx {
            sender: SENDER.into(),
            mentioned: vec![if with_device {
                "628999:3@s.whatsapp.net".to_string()
            } else {
                OTHER.to_string()
            }],
            ..Default::default()
        }
    }

    fn text_reply(reply: &Reply) -> &str {
        reply.text_body().expect("harus ada teks")
    }

    fn mentions_of(reply: &Reply) -> &[String] {
        match reply {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn plain(text: String) -> Reply {
        Reply::Text {
            text,
            mentions: vec![SENDER.to_string()],
        }
    }

    /// Sampel cukup banyak untuk hampir pasti menyentuh kedua ujung range.
    /// Untuk range tersempit di file ini (`sisaumur_bulan`, 12 nilai) peluang
    /// gagal menyentuh satu ujung setelah 5.000 sampel mendekati nol.
    const SAMPLES: usize = 5_000;

    /// Kumpulkan `n` sampel undian, kembalikan `(min, max)`.
    fn observed(n: usize, mut f: impl FnMut() -> u32) -> (u32, u32) {
        let mut lo = u32::MAX;
        let mut hi = 0;
        for _ in 0..n {
            let v = f();
            lo = lo.min(v);
            hi = hi.max(v);
        }
        (lo, hi)
    }

    // ------------------------------------------------------------- RNG range

    #[test]
    fn roll_is_zero_inclusive_and_n_exclusive() {
        // Penegak aturan `Math.floor(Math.random() * N)` = 0..=N-1.
        let (lo, hi) = observed(SAMPLES, || roll(101));
        assert_eq!(lo, 0, "0 harus mungkin terjadi");
        assert_eq!(hi, 100, "batas atas = n-1, bukan n");
    }

    #[test]
    fn cekkarma_percent_spans_0_to_100() {
        assert_eq!(observed(SAMPLES, karma_percent), (0, 100));
    }

    #[test]
    fn cekumur_percent_spans_5_to_84() {
        // floor(r*80)+5 -> 5..=84. Offset yang kelipolar jadi 0..=79.
        assert_eq!(observed(SAMPLES, umur_percent), (5, 84));
    }

    #[test]
    fn cekberat_always_lands_in_40_to_100() {
        // Diminta eksplisit: hasil selalu di 40..100 (100 eksklusif).
        for _ in 0..SAMPLES {
            let kg = berat_kg();
            assert!(
                (40..100).contains(&kg),
                "berat di luar 40..100: {kg} — floor(r*60)+40 harus 40..=99"
            );
        }
        assert_eq!(
            observed(SAMPLES, berat_kg),
            (40, 99),
            "kedua ujung harus terjangkau"
        );
    }

    #[test]
    fn cektinggi_always_lands_in_150_to_200() {
        for _ in 0..SAMPLES {
            let cm = tinggi_cm();
            assert!(
                (150..200).contains(&cm),
                "tinggi di luar 150..200: {cm} — floor(r*50)+150 harus 150..=199"
            );
        }
        assert_eq!(observed(SAMPLES, tinggi_cm), (150, 199));
    }

    #[test]
    fn ceksisaumur_three_draws_hit_their_exact_domains() {
        // floor(r*80)+20 -> 20..=99; floor(r*12) -> 0..=11; floor(r*30) -> 0..=29.
        for _ in 0..SAMPLES {
            let (t, b, h) = (sisaumur_tahun(), sisaumur_bulan(), sisaumur_hari());
            assert!((20..100).contains(&t), "tahun di luar 20..=99: {t}");
            assert!(b <= 11, "bulan di luar 0..=11: {b}");
            assert!((0..30).contains(&h), "hari di luar 0..=29: {h}");
        }
        assert_eq!(observed(SAMPLES, sisaumur_tahun), (20, 99));
        assert_eq!(observed(SAMPLES, sisaumur_bulan), (0, 11));
        assert_eq!(observed(SAMPLES, sisaumur_hari), (0, 29));
    }

    #[test]
    fn cekpintar_iq_spans_70_to_170() {
        // floor(r*100)+70 -> 70..=169. Tanpa offset +70, tier 150/130 mati.
        for _ in 0..SAMPLES {
            let iq = pintar_iq();
            assert!((70..170).contains(&iq), "iq di luar 70..=169: {iq}");
        }
        assert_eq!(observed(SAMPLES, pintar_iq), (70, 169));
    }

    #[test]
    fn every_tier_of_every_plugin_is_reachable() {
        // Guard utama: kalau offset atau ambang ada yang salah, ada tier yang
        // jadi dead branch dan test ini gagal.
        assert!(
            (0..=100).any(|p| karma_desc(p) == "Karma baik! Surga menantimu~ ✨"),
            "tier teratas cekkarma mati"
        );
        assert!(
            (0..=100).any(|p| umur_desc(p) == "Bijaksana seperti orang tua! 🧓"),
            "tier teratas cekumur mati"
        );
        assert!(
            (40..100).any(|p| berat_desc(p) == "Big boy/girl! 💪"),
            "tier teratas cekberat mati"
        );
        assert!(
            (150..200).any(|p| tinggi_desc(p) == "TINGGI BANGET! Model basketball! 🏀"),
            "tier teratas cektinggi mati"
        );
        assert!(
            (20..100).any(|t| sisaumur_desc(t) == "Panjang umur banget! 🎉"),
            "tier teratas ceksisaumur mati"
        );
        assert!(
            (70..170).any(|p| pintar_desc(p) == "JENIUS! Einstein level! 🧠✨"),
            "tier teratas cekpintar mati"
        );
    }

    // ------------------------------------------------- batas tiap threshold

    #[test]
    fn cekkarma_thresholds_are_80_60_40_20() {
        // Setiap batas diuji dari KEDUA sisi — nilai tepat di ambang dan satu
        // di bawahnya. Kalau ada ambang yang bergeser, salah satu assert gagal.
        for (percent, expected) in [
            (100, "Karma baik! Surga menantimu~ ✨"),
            (80, "Karma baik! Surga menantimu~ ✨"),
            (79, "Cukup baik, terus tingkatkan! 🙏"),
            (60, "Cukup baik, terus tingkatkan! 🙏"),
            (59, "Netral, perbanyak kebaikan~ ⚖️"),
            (40, "Netral, perbanyak kebaikan~ ⚖️"),
            (39, "Hati-hati dengan karma buruk! ⚠️"),
            (20, "Hati-hati dengan karma buruk! ⚠️"),
            (19, "Wah perlu banyak tobat nih... 😱"),
            (0, "Wah perlu banyak tobat nih... 😱"),
        ] {
            assert_eq!(karma_desc(percent), expected, "karma percent {percent}");
        }
    }

    #[test]
    fn cekumur_thresholds_are_60_40_20() {
        for (percent, expected) in [
            (84, "Bijaksana seperti orang tua! 🧓"),
            (60, "Bijaksana seperti orang tua! 🧓"),
            (59, "Dewasa dan matang~ 🧑"),
            (40, "Dewasa dan matang~ 🧑"),
            (39, "Jiwa muda! 🧒"),
            (20, "Jiwa muda! 🧒"),
            (19, "Masih seperti anak kecil~ 👶"),
            (5, "Masih seperti anak kecil~ 👶"),
        ] {
            assert_eq!(umur_desc(percent), expected, "umur percent {percent}");
        }
    }

    #[test]
    fn cekberat_thresholds_are_90_70_55_45() {
        // 55 dan 45 bukan kelipatan 10 — inilah yang paling gampang salah ketik.
        for (kg, expected) in [
            (99, "Big boy/girl! 💪"),
            (90, "Big boy/girl! 💪"),
            (89, "Berisi dan sehat! 😊"),
            (70, "Berisi dan sehat! 😊"),
            (69, "Ideal banget! 👍"),
            (55, "Ideal banget! 👍"),
            (54, "Langsing nih~ 🌸"),
            (45, "Langsing nih~ 🌸"),
            (44, "Kurus banget, makan yang banyak! 🍔"),
            (40, "Kurus banget, makan yang banyak! 🍔"),
        ] {
            assert_eq!(berat_desc(kg), expected, "berat {kg} kg");
        }
    }

    #[test]
    fn cektinggi_thresholds_are_190_175_165_155() {
        for (cm, expected) in [
            (199, "TINGGI BANGET! Model basketball! 🏀"),
            (190, "TINGGI BANGET! Model basketball! 🏀"),
            (189, "Tinggi ideal! 😎"),
            (175, "Tinggi ideal! 😎"),
            (174, "Lumayan tinggi 👍"),
            (165, "Lumayan tinggi 👍"),
            (164, "Standard kok 🙂"),
            (155, "Standard kok 🙂"),
            (154, "Imut dan mungil! 🥺"),
            (150, "Imut dan mungil! 🥺"),
        ] {
            assert_eq!(tinggi_desc(cm), expected, "tinggi {cm} cm");
        }
    }

    #[test]
    fn ceksisaumur_uses_strict_greater_than_not_greater_equal() {
        // Penegak pembeda `>` vs `>=` — nilai PERSIS di ambang jatuh ke tier
        // BAWAH. Kalau operatornya diganti jadi `>=`, test ini gagal.
        for (tahun, expected) in [
            (99, "Panjang umur banget! 🎉"),
            (81, "Panjang umur banget! 🎉"),
            (80, "Lumayan panjang~ ✨"),
            (61, "Lumayan panjang~ ✨"),
            (60, "Cukup lah ya 😊"),
            (41, "Cukup lah ya 😊"),
            (40, "Jaga kesehatan ya! 🙏"),
            (20, "Jaga kesehatan ya! 🙏"),
        ] {
            assert_eq!(
                sisaumur_desc(tahun),
                expected,
                "sisa umur {tahun} tahun — harus `>`, bukan `>=`"
            );
        }
    }

    #[test]
    fn cekpintar_thresholds_are_150_130_110_90() {
        for (iq, expected) in [
            (169, "JENIUS! Einstein level! 🧠✨"),
            (150, "JENIUS! Einstein level! 🧠✨"),
            (149, "Sangat cerdas! 🎓"),
            (130, "Sangat cerdas! 🎓"),
            (129, "Di atas rata-rata! 👍"),
            (110, "Di atas rata-rata! 👍"),
            (109, "Normal, rata-rata 😊"),
            (90, "Normal, rata-rata 😊"),
            (89, "Tetap semangat belajar! 📚"),
            (70, "Tetap semangat belajar! 📚"),
        ] {
            assert_eq!(pintar_desc(iq), expected, "iq {iq}");
        }
    }

    // --------------------------------------- scan penuh: satu tier per nilai

    /// Untuk tiap nilai dalam domainnya, balasan harus memuat PERSIS satu teks
    /// tier. Menutup semua nilai (bukan sampel), jadi ambang yang salah ketik
    /// pasti ketahuan.
    fn scan_tiers(domain: std::ops::RangeInclusive<u32>, tiers: &[&str], build: impl Fn(u32) -> String) {
        for value in domain {
            let text = build(value);
            let hits: Vec<&&str> = tiers.iter().filter(|t| text.contains(**t)).collect();
            assert_eq!(
                hits.len(),
                1,
                "nilai {value} memuat {} tier, harus tepat 1: {text}",
                hits.len()
            );
        }
    }

    #[test]
    fn full_scan_cekkarma_picks_exactly_one_tier() {
        scan_tiers(
            0..=100,
            &[
                "Karma baik! Surga menantimu~ ✨",
                "Cukup baik, terus tingkatkan! 🙏",
                "Netral, perbanyak kebaikan~ ⚖️",
                "Hati-hati dengan karma buruk! ⚠️",
                "Wah perlu banyak tobat nih... 😱",
            ],
            |p| karma_text(&ctx_self(), p),
        );
    }

    #[test]
    fn full_scan_cekumur_picks_exactly_one_tier() {
        scan_tiers(
            5..=84,
            &[
                "Bijaksana seperti orang tua! 🧓",
                "Dewasa dan matang~ 🧑",
                "Jiwa muda! 🧒",
                "Masih seperti anak kecil~ 👶",
            ],
            |p| umur_text(&ctx_self(), p),
        );
    }

    #[test]
    fn full_scan_cekberat_picks_exactly_one_tier() {
        scan_tiers(
            40..=99,
            &[
                "Big boy/girl! 💪",
                "Berisi dan sehat! 😊",
                "Ideal banget! 👍",
                "Langsing nih~ 🌸",
                "Kurus banget, makan yang banyak! 🍔",
            ],
            |kg| berat_text(&ctx_self(), kg),
        );
    }

    #[test]
    fn full_scan_cektinggi_picks_exactly_one_tier() {
        scan_tiers(
            150..=199,
            &[
                "TINGGI BANGET! Model basketball! 🏀",
                "Tinggi ideal! 😎",
                "Lumayan tinggi 👍",
                "Standard kok 🙂",
                "Imut dan mungil! 🥺",
            ],
            |cm| tinggi_text(&ctx_self(), cm),
        );
    }

    #[test]
    fn full_scan_ceksisaumur_picks_exactly_one_tier() {
        scan_tiers(
            20..=99,
            &[
                "Panjang umur banget! 🎉",
                "Lumayan panjang~ ✨",
                "Cukup lah ya 😊",
                "Jaga kesehatan ya! 🙏",
            ],
            |tahun| sisaumur_text(&ctx_self(), tahun, 0, 0),
        );
    }

    #[test]
    fn full_scan_cekpintar_picks_exactly_one_tier() {
        scan_tiers(
            70..=169,
            &[
                "JENIUS! Einstein level! 🧠✨",
                "Sangat cerdas! 🎓",
                "Di atas rata-rata! 👍",
                "Normal, rata-rata 😊",
                "Tetap semangat belajar! 📚",
            ],
            |iq| pintar_text(&ctx_self(), iq),
        );
    }

    // ----------------------------------------- kalimat self vs orang lain

    #[test]
    fn self_reply_says_kamu_for_all_six() {
        for (reply, needle) in [
            (karma_text(&ctx_self(), 85), "Tingkat kekarmaan kamu *85%"),
            (umur_text(&ctx_self(), 65), "Tingkat keumuran kamu *65%"),
            (berat_text(&ctx_self(), 75), "Berat badan kamu *75 kg"),
            (tinggi_text(&ctx_self(), 180), "Tinggi badan kamu *180 cm"),
            (
                sisaumur_text(&ctx_self(), 50, 3, 7),
                "Sisa umur kamu *50 Tahun 3 Bulan 7 Hari",
            ),
            (pintar_text(&ctx_self(), 120), "Kepintaran (IQ) kamu *120"),
        ] {
            assert!(
                reply.contains(needle),
                "kalimat self harus pakai 'kamu': belum ketemu {needle:?} di {reply:?}"
            );
            assert!(
                !reply.contains(" dia sebesar"),
                "self tidak boleh pakai 'dia': {reply:?}"
            );
        }
    }

    #[test]
    fn other_reply_says_dia_and_targets_the_mention() {
        for (reply, needle) in [
            (
                karma_text(&ctx_other(false), 85),
                "Tingkat kekarmaan dia sebesar *85%",
            ),
            (
                umur_text(&ctx_other(false), 65),
                "Tingkat keumuran dia sebesar *65%",
            ),
            (
                berat_text(&ctx_other(false), 75),
                "Berat badan dia sebesar *75 kg",
            ),
            (
                tinggi_text(&ctx_other(false), 180),
                "Tinggi badan dia sebesar *180 cm",
            ),
            (
                sisaumur_text(&ctx_other(false), 50, 3, 7),
                "Sisa umur dia sebesar *50 Tahun 3 Bulan 7 Hari",
            ),
            (
                pintar_text(&ctx_other(false), 120),
                "Kepintaran (IQ) dia sebesar *120",
            ),
        ] {
            assert!(
                reply.contains(needle),
                "kalimat orang lain harus pakai 'dia': belum ketemu {needle:?} di {reply:?}"
            );
            assert!(
                reply.contains("@628999 yak?"),
                "harus menyapa target, bukan sender: {reply:?}"
            );
            assert!(
                !reply.contains(&format!("@{SENDER} yak?")),
                "tidak boleh menyapa sender: {reply:?}"
            );
        }
    }

    #[test]
    fn cektinggi_other_variant_keeps_the_js_inconsistent_wording() {
        // JS: kalimat self = "Tinggi badan", kalimat other = "tingkat ketinggian".
        // Bukan typo yang perlu dirapikan — itu isi file aslinya.
        assert!(tinggi_text(&ctx_self(), 180).contains("Tinggi badan kamu"));
        assert!(tinggi_text(&ctx_other(false), 180).contains("tingkat ketinggian"));
    }

    /// `mentioned.split('@')[0]` membuang `@server`, TAPI tidak membuang `:3`.
    ///
    /// Jadi `628999:3@s.whatsapp.net` ditampilkan sebagai `@628999:3` — persis
    /// seperti JS. Tes ini menjaga itu; kalau suatu saat `:3` ikut dibuang,
    /// berarti port sudah menyimpang dari JS.
    #[test]
    fn mention_drops_server_but_keeps_device_in_both_display_and_mentions() {
        let ctx = ctx_other(true);
        let reply = Reply::Text {
            text: berat_text(&ctx, 75),
            mentions: vec![ctx.mention_or_sender().to_string()],
        };
        assert!(
            text_reply(&reply).contains("berat badan @628999:3 yak?"),
            "display harus menyisakan device seperti JS: {:?}",
            text_reply(&reply)
        );
        assert_eq!(mentions_of(&reply), ["628999:3@s.whatsapp.net"]);
    }

    #[test]
    fn no_mention_falls_back_to_sender() {
        let ctx = ctx_self();
        assert_eq!(ctx.mention_or_sender(), SENDER);
        let reply = plain(karma_text(&ctx, 85));
        assert!(
            text_reply(&reply).contains("Hai @628111"),
            "{:?}",
            text_reply(&reply)
        );
        assert_eq!(mentions_of(&reply), [SENDER]);
    }

    #[test]
    fn text_builders_are_pure_functions_of_draw() {
        // Menahan regresi stateful: Ctx sama + undian sama -> teks sama.
        assert_eq!(karma_text(&ctx_self(), 42), karma_text(&ctx_self(), 42));
        assert_eq!(umur_text(&ctx_self(), 42), umur_text(&ctx_self(), 42));
        assert_eq!(berat_text(&ctx_self(), 42), berat_text(&ctx_self(), 42));
        assert_eq!(
            tinggi_text(&ctx_self(), 160),
            tinggi_text(&ctx_self(), 160)
        );
        assert_eq!(
            sisaumur_text(&ctx_self(), 42, 1, 2),
            sisaumur_text(&ctx_self(), 42, 1, 2)
        );
        assert_eq!(pintar_text(&ctx_self(), 42), pintar_text(&ctx_self(), 42));
    }

    // ------------------------------------------------------------- registry

    /// Tiap plugin didaftarkan dengan meta yang benar-benar tertulis di
    /// `pluginConfig` masing-masing — bukan nilai default macro.
    #[test]
    fn all_six_register_with_their_real_meta() {
        let expected: [(&str, &[&str], &str); 6] = [
            ("cekkarma", &["karma"], "Cek tingkat karma kamu"),
            ("cekumur", &["umur", "age"], "Cek umur mental kamu"),
            ("cekberat", &["berat", "weight"], "Cek berat badan random"),
            (
                "cektinggi",
                &["tinggi", "tall"],
                "Cek tinggi badan random",
            ),
            ("ceksisaumur", &["sisaumur"], "Cek sisa umur kamu"),
            (
                "cekpintar",
                &["pintar", "iq", "smart"],
                "Cek seberapa pintar kamu",
            ),
        ];

        for (name, alias, description) in expected {
            let p = oktz_plugin_api::find(name)
                .unwrap_or_else(|| panic!("plugin {name} tidak terdaftar"));
            assert_eq!(p.meta.name, name);
            assert_eq!(p.meta.category, "cek", "{name} category");
            assert_eq!(p.meta.alias, alias, "{name} alias");
            assert_eq!(p.meta.description, description, "{name} description");
            // Semua enam menulis `cooldown: 5, energi: 0` eksplisit di JS.
            assert_eq!(p.meta.cooldown, 5, "{name} cooldown");
            assert_eq!(p.meta.energi, 0, "{name} energi");
            assert!(!p.meta.is_private, "{name} isPrivate=false");
            assert!(!p.meta.is_owner, "{name} isOwner=false");
            assert!(!p.meta.is_premium, "{name} isPremium=false");
            assert!(!p.meta.is_group, "{name} isGroup=false");
            assert!(p.meta.is_enabled, "{name} isEnabled=true");
        }
    }

    #[test]
    fn usage_and_example_match_js_verbatim() {
        for (name, usage, example) in [
            ("cekkarma", ".cekkarma <nama>", ".cekkarma Budi"),
            ("cekumur", ".cekumur <nama>", ".cekumur Budi"),
            ("cekberat", ".cekberat <nama>", ".cekberat Budi"),
            ("cektinggi", ".cektinggi <nama>", ".cektinggi Budi"),
            ("ceksisaumur", ".ceksisaumur <nama>", ".ceksisaumur Budi"),
            ("cekpintar", ".cekpintar <nama>", ".cekpintar Budi"),
        ] {
            let p = oktz_plugin_api::find(name).unwrap();
            assert_eq!(p.meta.usage, usage, "{name} usage");
            assert_eq!(p.meta.example, example, "{name} example");
        }
    }

    #[test]
    fn every_alias_resolves_via_find() {
        for (alias, name) in ALIAS_OWNERS {
            let found = oktz_plugin_api::find(alias)
                .unwrap_or_else(|| panic!("alias {alias} tidak resolve"));
            assert_eq!(found.meta.name, name, "alias {alias} salah resolve");
        }
    }

    /// Kalau plugin dari batch lain memakai alias yang sama, `find()` mengembalikan
    /// yang menang duluan —alias hilang tanpa error. Test ini yang menangkapnya.
    #[test]
    fn every_alias_of_this_batch_is_claimed_by_exactly_one_plugin() {
        for (alias, name) in ALIAS_OWNERS {
            let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                .iter()
                .filter(|p| p.meta.alias.iter().any(|a| a == alias))
                .map(|p| p.meta.name.as_str())
                .collect();
            assert_eq!(
                owners,
                vec![name],
                "alias {alias} dipakai lebih dari satu plugin: {owners:?}"
            );
        }
    }

    const ALIAS_OWNERS: [(&str, &str); 11] = [
        ("karma", "cekkarma"),
        ("umur", "cekumur"),
        ("age", "cekumur"),
        ("berat", "cekberat"),
        ("weight", "cekberat"),
        ("tinggi", "cektinggi"),
        ("tall", "cektinggi"),
        ("sisaumur", "ceksisaumur"),
        ("pintar", "cekpintar"),
        ("iq", "cekpintar"),
        ("smart", "cekpintar"),
    ];

    #[test]
    fn every_registered_handler_returns_a_well_formed_reply() {
        for name in [
            "cekkarma",
            "cekumur",
            "cekberat",
            "cektinggi",
            "ceksisaumur",
            "cekpintar",
        ] {
            let p = oktz_plugin_api::find(name).unwrap();

            // Tanpa mention -> target = sender, kalimat self.
            let reply = (p.handler)(ctx_self());
            assert!(!reply.is_none(), "{name} harus membalas saat dipanggil");
            assert_eq!(mentions_of(&reply), [SENDER], "{name} mention default");
            let text = text_reply(&reply).to_string();
            assert!(text.starts_with("Hai @628111"), "{name}: {text}");
            assert!(
                text.contains("\n```") && text.ends_with("```"),
                "{name} tier harus dibungkus pagar tiga: {text}"
            );

            // Dengan mention -> target = orang lain, kalimat 'dia'.
            let reply = (p.handler)(ctx_other(false));
            assert_eq!(mentions_of(&reply), [OTHER], "{name} mention target");
            let text = text_reply(&reply).to_string();
            assert!(text.starts_with("Kamu ingin ngecek "), "{name}: {text}");
            assert!(text.contains("dia sebesar"), "{name}: {text}");
        }
    }

    /// Scan ulang seluruh registry: setiap plugin dari batch ini harus punya
    /// deskripsi yang terisi. Plugin dengan `description` kosong lolos semua
    /// test di atas kalau hanya dicek per-nama, karena `find()`bynama tetap
    /// menemukan entry-nya.
    #[test]
    fn no_plugin_from_this_batch_has_empty_meta_fields() {
        for name in [
            "cekkarma",
            "cekumur",
            "cekberat",
            "cektinggi",
            "ceksisaumur",
            "cekpintar",
        ] {
            let m = &oktz_plugin_api::find(name).unwrap().meta;
            assert!(!m.name.is_empty(), "{name} name kosong");
            assert!(!m.description.is_empty(), "{name} description kosong");
            assert!(!m.usage.is_empty(), "{name} usage kosong");
            assert!(!m.example.is_empty(), "{name} example kosong");
            assert!(!m.alias.is_empty(), "{name} alias kosong");
        }
    }
}