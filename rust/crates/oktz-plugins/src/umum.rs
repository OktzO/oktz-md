//! Port batch kedua plugin class A — `plugins/fun/*` dan `plugins/tools/*`.
//!
//! Dipilih dari `docs/rust-rewrite/plugin-manifest.json` (`class: "A"`), yang
//! semuanya constructor statis: tanpa HTTP, tanpa media, tanpa fitur WA
//! khusus, tanpa akses database. Total 14 plugin di sini.
//!
//! Tiga bentuk yang berulang di sumber:
//!
//! 1. **Keluarga "tanya bot" (10 plugin)** — `apakah`, `mengapa`, `kapan`,
//!    `akankah`, `bagaimana`, `bisakah`, `coba`, `dimana`, `haruskah`,
//!    `berapa`. Kerangkanya identik: teks kosong -> teks bantuan; teks ada ->
//!    `` `${m.body.slice(1)}?\n*${answers[...]}*` ``. Yang berbeda cuma 20 baris
//!    jawaban + teks bantuan per plugin, jadi dipisah jadi konstanta.
//! 2. **`rate`** — bentuk sama tapi tanpa echoed question; keluar sebagai
//!    pasangan `(score, comment)`.
//! 3. **`readmore`, `soulmatch`, `kalkulatormbg`** — masing-masing satu logic
//!    sendiri, tetap pure.
//!
//! Catatan semantik JS→Rust yang dipakai di file ini:
//!
//! - `answers[Math.floor(Math.random() * answers.length)]` → indeks **inklusif
//!   0, eksklusif `len`**. Kalau pakai `0..=len` maka baris terakhir jadi
//!   dead dan `pick` bisa panic di `len`.
//! - `m.body` adalah body pesan mentah (prefix ikut). `Ctx` tidak menyimpan
//!   `body`, jadi [`body_stripped`] menyusun ulang dari `prefix + command +
//!   " " + text`.
//! - `Array.from(name.toLowerCase()).reduce((a,c)=>a+c.charCodeAt(0),0)` memakai
//!   code unit UTF-16; `chars()` memberi scalar value. Sama untuk seluruh BMP
//!   (semua nama yang masuk ke `soulmatch`).

use oktz_plugin_api::{Ctx, Reply, plugin};
use rand::Rng as _;

/// Indeks acak seragam di `[0, len)` — padanan
/// `Math.floor(Math.random() * len)`.
fn random_index(len: usize) -> usize {
    rand::rng().random_range(0..len)
}

/// Rekonstruksi `m.body` lalu buang **satu** karakter pertama (`m.body.slice(1)`
/// di JS — char itu adalah prefix).
///
/// `ponytail:` `Ctx` tidak membawa `body` mentah, jadi spasi ganda di tengah
/// pesan akan dinormalkan jadi satu spasi. Tambahkan `body: String` ke `Ctx`
/// kalau router nanti mau kesetiaan byte-per-byte.
fn body_stripped(ctx: &Ctx) -> String {
    let mut body = String::with_capacity(ctx.prefix.len() + ctx.command.len() + ctx.text.len() + 1);
    body.push_str(&ctx.prefix);
    body.push_str(&ctx.command);
    if !ctx.text.is_empty() {
        body.push(' ');
        body.push_str(&ctx.text);
    }
    body.chars().skip(1).collect()
}

// ---------------------------------------------------------------------------
// 1. Keluarga "tanya bot"
// ---------------------------------------------------------------------------

/// Bentuk balasan yang dipakai 10 plugin `?`:
/// `` `${m.body.slice(1)}?\n*${answer}*` ``.
fn render_tanya(question: &str, answer: &str) -> String {
    format!("{question}?\n*{answer}*")
}

/// Cabang.help dan cabang.jawab untuk satu plugin "tanya bot".
fn tanya_reply(ctx: &Ctx, answers: &[&str], help: &str) -> Reply {
    if ctx.text.trim().is_empty() {
        return Reply::text(help);
    }
    let answer = answers[random_index(answers.len())];
    Reply::text(render_tanya(&body_stripped(ctx), answer))
}

macro_rules! tanya_plugin {
    ($fn_name:ident, $name:literal, $alias:expr, $desc:literal, $usage:literal,
     $example:literal, $answers:ident, $help:expr) => {
        #[plugin(
            name = $name,
            alias = $alias,
            category = "fun",
            description = $desc,
            usage = $usage,
            example = $example,
            cooldown = 3,
            energi = 0,
            is_private = false
        )]
        pub fn $fn_name(ctx: Ctx) -> Reply {
            tanya_reply(&ctx, &$answers, $help)
        }
    };
}

const APAKAH: [&str; 20] = [
    "Ya, tentu saja!",
    "Tidak, sepertinya tidak.",
    "Mungkin saja, coba lagi nanti.",
    "Hmm... aku rasa iya.",
    "Aku ragu, tapi bisa jadi.",
    "Pasti! 100%!",
    "Tidak mungkin.",
    "Bisa jadi, siapa yang tau?",
    "Menurutku sih iya.",
    "Wah, kayaknya nggak deh.",
    "Tentu, kenapa tidak?",
    "Aku nggak tau, coba tanya yang lain.",
    "Ya ampun, pasti lah!",
    "Hmm... sepertinya tidak.",
    "Aku yakin iya!",
    "Nggak mungkin banget.",
    "Mungkin, tapi jangan berharap terlalu tinggi.",
    "Iya dong!",
    "Nggak, maaf ya.",
    "Bisa! Semangat!",
];

const MENGAPA: [&str; 20] = [
    "Karena memang sudah takdirnya begitu.",
    "Hmm, pertanyaan bagus! Aku juga bingung.",
    "Karena itulah cara kerjanya.",
    "Karena Tuhan berkehendak demikian.",
    "Aku nggak tau, cari di Google aja.",
    "Karena ya gitu aja.",
    "Mungkin karena kebetulan?",
    "Karena dunia memang penuh misteri.",
    "Hmm, sulit dijelaskan sih.",
    "Karena alam semesta bekerja dengan cara yang misterius.",
    "Aku juga penasaran, kenapa ya?",
    "Karena hal tersebut memang seharusnya terjadi.",
    "Pertanyaan yang bagus! Sayangnya aku nggak punya jawabannya.",
    "Karena itulah keunikan hidup.",
    "Karena setiap hal punya alasannya masing-masing.",
    "Hmm... aku butuh waktu untuk memikirkannya.",
    "Karena begitulah logikanya.",
    "Aku rasa karena memang harus begitu.",
    "Karena segala sesuatu saling berhubungan.",
    "Nah itu aku juga mikir!",
];

const KAPAN: [&str; 20] = [
    "Besok mungkin?",
    "Tahun depan kayaknya.",
    "3 hari lagi!",
    "Hmm, masih lama sih.",
    "Sebentar lagi kok!",
    "Kalau sudah waktunya, pasti terjadi.",
    "Bulan depan!",
    "Entah kapan, yang penting sabar.",
    "Dalam waktu dekat!",
    "10 tahun lagi mungkin?",
    "Nggak lama lagi!",
    "Kalau jodoh, pasti ketemu.",
    "Hmm, susah diprediksi.",
    "Minggu depan!",
    "Kalau usahanya lebih keras, lebih cepat!",
    "Pas waktunya tepat.",
    "Secepatnya, tenang aja.",
    "Ntar kalo udah siap.",
    "Dalam hitungan hari!",
    "Saat kamu sudah siap menerimanya.",
];

const AKANKAH: [&str; 20] = [
    "Ya, pasti akan terjadi!",
    "Tidak, sepertinya tidak akan.",
    "Mungkin akan, mungkin tidak.",
    "InsyaAllah akan terjadi!",
    "Hmm, sulit diprediksi.",
    "Pasti! Yakin saja!",
    "Kayaknya nggak deh.",
    "Akan terjadi kalau kamu mau berusaha.",
    "Suatu saat nanti, pasti.",
    "Nggak akan, maaf.",
    "Tentu akan! Tunggu saja!",
    "Hmm, aku ragu.",
    "Akan! Percaya sama proses!",
    "Kemungkinannya kecil.",
    "Pasti akan, aku yakin!",
    "Nggak akan, cari yang lain aja.",
    "Akan, tapi butuh waktu.",
    "InsyaAllah!",
    "Kalau jodoh, pasti akan.",
    "Akan terjadi di saat yang tepat!",
];

const BAGAIMANA: [&str; 20] = [
    "Caranya gampang, ya tinggal dilakuin aja!",
    "Hmm, susah dijelasin sih. Coba aja dulu!",
    "Dengan usaha dan doa pastinya.",
    "Ya begitulah caranya.",
    "Aku kurang tau sih, coba cari referensi lain.",
    "Pelan-pelan aja, nanti juga bisa.",
    "Dengan kerja keras dan pantang menyerah!",
    "Pertama, percaya sama diri sendiri dulu.",
    "Hmm, tiap orang beda-beda sih caranya.",
    "Ikutin kata hatimu aja.",
    "Belajar dari yang sudah berpengalaman.",
    "Step by step, jangan terburu-buru.",
    "Dengan tekad yang kuat!",
    "Mulai dari yang kecil dulu.",
    "Konsisten aja, nanti juga bisa.",
    "Jangan overthinking, langsung action!",
    "Gampang! Tinggal mulai aja!",
    "Caranya? Ya dicoba dulu!",
    "Dengan strategi yang tepat.",
    "Hmm, aku juga masih belajar sih.",
];

const BISAKAH: [&str; 20] = [
    "Bisa banget! Percaya diri aja!",
    "Hmm, kayaknya susah deh.",
    "Tentu bisa! Semangat!",
    "Nggak bisa, maaf.",
    "Mungkin bisa, kalau usaha keras.",
    "Pasti bisa! Jangan menyerah!",
    "Agak susah sih, tapi bisa dicoba.",
    "Bisa kok! Yakin deh!",
    "Kayaknya nggak deh.",
    "Bisa! Ayo buktikan!",
    "Hmm... aku ragu.",
    "Bisa banget! Gas terus!",
    "Nggak bisa, coba yang lain.",
    "Bisa! Percaya sama diri sendiri!",
    "Susah, tapi bukan berarti nggak mungkin.",
    "Absolutely! Kamu pasti bisa!",
    "Kayaknya perlu usaha ekstra nih.",
    "Bisa! Jangan ragukan dirimu!",
    "Hmm, coba lagi nanti deh.",
    "Bisa! Aku percaya kamu!",
];

const COBA: [&str; 20] = [
    "Hmm, aku coba ya... Kamu lagi mikirin makanan!",
    "Aku teebak... Kamu lagi gabut!",
    "Coba ya... Kayaknya kamu lagi seneng!",
    "Hmm, aku rasa kamu lagi bingung.",
    "Aku coba nebak... Kamu lagi kangen seseorang?",
    "Kayaknya kamu lagi santai deh.",
    "Aku teebak kamu lagi scroll HP terus.",
    "Hmm, pasti lagi bosan ya?",
    "Coba diteebak... Kamu lagi pengen jalan-jalan!",
    "Aku rasa kamu lagi butuh hiburan.",
    "Hmm, kayaknya kamu lagi happy!",
    "Aku coba... Kamu pasti lagi penasaran!",
    "Tebakan aku: kamu lagi rebahan.",
    "Hmm, kamu mungkin lagi mikirin seseorang spesial.",
    "Aku coba: kamu lagi mau curhat?",
    "Kayaknya kamu lagi pengen main game!",
    "Hmm, aku teebak kamu lagi dengerin musik.",
    "Coba aku teebak... Kamu lagi di kamar!",
    "Aku rasa kamu lagi waiting for something.",
    "Hmm, tebakan aku: kamu butuh temen ngobrol!",
];

const DIMANA: [&str; 20] = [
    "Di dekatmu!",
    "Jauh di sana.",
    "Di tempat yang tidak kamu duga.",
    "Di hatimu.",
    "Di sekitar sini.",
    "Hmm, coba cari di kamar.",
    "Di luar sana, menunggumu.",
    "Di tempat yang sama denganmu.",
    "Di suatu tempat yang indah.",
    "Di balik pintu.",
    "Di sebelah kirimu.",
    "Di depan matamu!",
    "Jauh banget, di luar negeri mungkin?",
    "Di tempat yang penuh kenangan.",
    "Di mana-mana!",
    "Di dunia maya.",
    "Di alam mimpi.",
    "Di tempat rahasia.",
    "Hmm, susah dijelaskan lokasinya.",
    "Di tempat yang akan membuatmu bahagia.",
];

const HARUSKAH: [&str; 20] = [
    "Ya, harus!",
    "Tidak usah.",
    "Hmm, terserah kamu sih.",
    "Harus banget! Jangan ragu!",
    "Nggak harus juga.",
    "Kalau menurutmu perlu, lakukan!",
    "Pikir dulu baik-baik.",
    "Harus! Sekarang!",
    "Jangan, mending tunggu dulu.",
    "Harus, tapi hati-hati.",
    "Nggak harus, tapi boleh.",
    "Wajib!",
    "Hmm, skip aja deh.",
    "Lakukan kalau sudah yakin.",
    "Harus, demi masa depanmu!",
    "Nggak harus, santai aja.",
    "Go for it!",
    "Jangan buru-buru, pikir lagi.",
    "Tentu harus!",
    "Lihat situasinya dulu.",
];

const BERAPA: [&str; 21] = [
    "1",
    "7",
    "12",
    "21",
    "99",
    "69",
    "100",
    "50",
    "25",
    "1000",
    "5",
    "17",
    "88",
    "33",
    "nothing (jawabannya selalu nothing)",
    "Banyak banget!",
    "Cuma sedikit.",
    "Tak terhitung!",
    "Hmm, sekitar 10-an.",
    "Lebih dari yang kamu kira!",
    "Gak tau ah, males",
];

const HELP_APAKAH: &str =
    "❓ *ᴀᴘᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .apakah aku bisa jadi kaya?";
const HELP_MENGAPA: &str =
    "🤔 *ᴍᴇɴɢᴀᴘᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .mengapa langit biru?";
const HELP_KAPAN: &str =
    "⏰ *ᴋᴀᴘᴀɴ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .kapan aku nikah?";
const HELP_AKANKAH: &str =
    "🔮 *ᴀᴋᴀɴᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .akankah aku sukses?";
const HELP_BAGAIMANA: &str =
    "📋 *ʙᴀɢᴀɪᴍᴀɴᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bagaimana cara jadi sukses?";
const HELP_BISAKAH: &str =
    "💪 *ʙɪsᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bisakah aku lulus ujian?";
const HELP_COBA: &str =
    "🎯 *ᴄᴏʙᴀ*\n\n> Masukkan sesuatu!\n\n*Contoh:*\n> .coba tebak apa yang aku pikirkan";
const HELP_DIMANA: &str =
    "📍 *ᴅɪᴍᴀɴᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .dimana jodohku berada?";
const HELP_HARUSKAH: &str =
    "⚖️ *ʜᴀʀᴜsᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .haruskah aku menyatakan cinta?";
const HELP_BERAPA: &str =
    "🔢 *ʙᴇʀᴀᴘᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .berapa umur jodohku?";

tanya_plugin!(
    apakah,
    "apakah",
    ["apa"],
    "Tanya bot apakah sesuatu",
    ".apakah <pertanyaan>",
    ".apakah aku bisa kaya?",
    APAKAH,
    HELP_APAKAH
);
tanya_plugin!(
    mengapa,
    "mengapa",
    ["kenapa", "why"],
    "Tanya bot mengapa sesuatu",
    ".mengapa <pertanyaan>",
    ".mengapa langit biru?",
    MENGAPA,
    HELP_MENGAPA
);
tanya_plugin!(
    kapan,
    "kapan",
    ["when"],
    "Tanya bot kapan sesuatu",
    ".kapan <pertanyaan>",
    ".kapan aku nikah?",
    KAPAN,
    HELP_KAPAN
);
tanya_plugin!(
    akankah,
    "akankah",
    ["akan", "will"],
    "Tanya bot akankah sesuatu terjadi",
    ".akankah <pertanyaan>",
    ".akankah aku sukses?",
    AKANKAH,
    HELP_AKANKAH
);
tanya_plugin!(
    bagaimana,
    "bagaimana",
    ["gimana", "how"],
    "Tanya bot bagaimana sesuatu",
    ".bagaimana <pertanyaan>",
    ".bagaimana cara jadi sukses?",
    BAGAIMANA,
    HELP_BAGAIMANA
);
tanya_plugin!(
    bisakah,
    "bisakah",
    ["bisa"],
    "Tanya bot bisakah sesuatu",
    ".bisakah <pertanyaan>",
    ".bisakah aku lulus ujian?",
    BISAKAH,
    HELP_BISAKAH
);
tanya_plugin!(
    coba,
    "coba",
    ["try"],
    "Coba tanyakan sesuatu ke bot",
    ".coba <pertanyaan>",
    ".coba tebak apa yang aku pikirkan",
    COBA,
    HELP_COBA
);
tanya_plugin!(
    dimana,
    "dimana",
    ["where", "mana"],
    "Tanya bot dimana sesuatu",
    ".dimana <pertanyaan>",
    ".dimana jodohku berada?",
    DIMANA,
    HELP_DIMANA
);
tanya_plugin!(
    haruskah,
    "haruskah",
    ["harus", "should"],
    "Tanya bot haruskah sesuatu",
    ".haruskah <pertanyaan>",
    ".haruskah aku menyatakan cinta?",
    HARUSKAH,
    HELP_HARUSKAH
);
tanya_plugin!(
    berapa,
    "berapa",
    ["howmuch", "howmany"],
    "Tanya bot berapa sesuatu",
    ".berapa <pertanyaan>",
    ".berapa umur jodohku?",
    BERAPA,
    HELP_BERAPA
);

// ---------------------------------------------------------------------------
// 2. rate — pasangan (score, comment)
// ---------------------------------------------------------------------------

const RATE: [(&str, &str); 20] = [
    ("10/10", "Sempurna! Nggak ada duanya!"),
    ("9/10", "Hampir sempurna! Keren banget!"),
    ("8/10", "Bagus banget! Mantap!"),
    ("7/10", "Cukup bagus, di atas rata-rata!"),
    ("6/10", "Lumayan, bisa lebih baik lagi."),
    ("5/10", "Biasa aja sih, standar."),
    ("4/10", "Hmm, kurang sedikit."),
    ("3/10", "Perlu banyak perbaikan."),
    ("2/10", "Aduh, masih jauh dari bagus."),
    ("1/10", "Maaf, tapi ini parah."),
    ("100/10", "LEGEND! Beyond perfect!"),
    ("11/10", "Melebihi ekspektasi!"),
    ("69/100", "Nice..."),
    ("420/10", "BLAZING!"),
    ("∞/10", "Gacor kang"),
    ("7.5/10", "Solid! Good job!"),
    ("8.5/10", "Impressive!"),
    ("9.5/10", "Near perfection!"),
    ("-1/10", "Aku nggak tau harus ngomong apa..."),
    ("???/10", "Error 404: Rating not found."),
];

const HELP_RATE: &str =
    "⭐ *ʀᴀᴛᴇ*\n\n> Masukkan sesuatu untuk Dinilai!\n\n*Contoh:*\n> .rate wajahku";

fn render_rate(score: &str, comment: &str) -> String {
    format!("Rating dari aku: *{score}*\n{comment}")
}

#[plugin(
    name = "rate",
    alias = ["nilai", "rating"],
    category = "fun",
    description = "Minta bot memberi rating sesuatu",
    usage = ".rate <sesuatu>",
    example = ".rate wajahku",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn rate(ctx: Ctx) -> Reply {
    if ctx.text.trim().is_empty() {
        return Reply::text(HELP_RATE);
    }
    let (score, comment) = RATE[random_index(RATE.len())];
    Reply::text(render_rate(score, comment))
}

// ---------------------------------------------------------------------------
// 3. readmore — sisipkan 4001 U+200E di antara dua bagian
// ---------------------------------------------------------------------------

/// `String.fromCharCode(8206)` = U+200E LEFT-TO-RIGHT MARK.
const LRM: &str = "\u{200e}";
const LRM_COUNT: usize = 4001;

const HELP_READMORE: &str = "⚠️ Masukan text!";

/// `l + LRM*4001 + r` dengan `l`/`r` = dua segmen pertama hasil `text.split('|')`.
fn readmore_body(text: &str) -> String {
    let mut parts = text.split('|');
    let l = parts.next().unwrap_or("");
    let r = parts.next().unwrap_or("");

    let mut out = String::with_capacity(l.len() + r.len() + LRM_COUNT * LRM.len());
    out.push_str(l);
    for _ in 0..LRM_COUNT {
        out.push_str(LRM);
    }
    out.push_str(r);
    out
}

#[plugin(
    name = "readmore",
    alias = ["selengkapnya", "spoiler"],
    category = "tools",
    description = "Membuat teks baca selengkapnya (spoiler)",
    usage = ".readmore <text_awal>|<text_akhir>",
    example = ".readmore Hai|Ini adalah pesan rahasia",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn readmore(ctx: Ctx) -> Reply {
    if ctx.text.is_empty() {
        return Reply::text(format!(
            "{HELP_READMORE}\nContoh: `{}{} Halo|Ini teks tersembunyi`",
            ctx.prefix, ctx.command
        ));
    }
    Reply::text(readmore_body(&ctx.text))
}

// ---------------------------------------------------------------------------
// 4. soulmatch
// ---------------------------------------------------------------------------

const ELEMENTS: [&str; 8] = [
    "Api 🔥",
    "Air 💧",
    "Tanah 🌍",
    "Angin 🌪️",
    "Petir ⚡",
    "Es ❄️",
    "Cahaya ✨",
    "Bayangan 🌑",
];

const ZODIAC: [&str; 12] = [
    "♈ Aries",
    "♉ Taurus",
    "♊ Gemini",
    "♋ Cancer",
    "♌ Leo",
    "♍ Virgo",
    "♎ Libra",
    "♏ Scorpio",
    "♐ Sagittarius",
    "♑ Capricorn",
    "♒ Aquarius",
    "♓ Pisces",
];

const SOUL_TYPES: [&str; 9] = [
    "Pemimpin Yang Berani",
    "Penyeimbang Bijaksana",
    "Kreator Ekspresif",
    "Pembangun Solid",
    "Petualang Bebas",
    "Pelindung Setia",
    "Pemikir Mistis",
    "Penakluk Kuat",
    "Humanitarian Murni",
];

const HELP_SOULMATCH: &str = "💫 *sᴏᴜʟ ᴍᴀᴛᴄʜ*\n\n> Cek kecocokan jiwa 2 orang!\n\n*Format:*\n> `.soulmatch nama1|nama2`\n\n*Contoh:*\n> `.soulmatch Raiden|Mei`";

/// Jumlah sum of char codes — `Array.from(s.toLowerCase()).reduce((a,c)=>a+c.charCodeAt(0),0)`.
fn name_value(name: &str) -> u32 {
    name.to_lowercase().chars().map(|c| c as u32).sum()
}

/// `(element, zodiac, soulType)` untuk satu nama + seed.
fn soul_data(name: &str, seed: u32) -> (&'static str, &'static str, &'static str) {
    let n = name_value(name);
    let idx = |len: usize, mul: u32| ((n.wrapping_add(seed.wrapping_mul(mul))) % len as u32) as usize;
    (
        ELEMENTS[idx(ELEMENTS.len(), 1)],
        ZODIAC[idx(ZODIAC.len(), 2)],
        SOUL_TYPES[idx(SOUL_TYPES.len(), 3)],
    )
}

/// `(baseScore % 51) + 50` → selalu di `[50, 100]`.
fn compatibility(nama1: &str, nama2: &str) -> u32 {
    (name_value(&format!("{nama1}{nama2}")) % 51) + 50
}

fn match_description(score: u32) -> &'static str {
    if score >= 90 {
        "💫 Takdir Sejati"
    } else if score >= 80 {
        "✨ Harmoni Sempurna"
    } else if score >= 70 {
        "🌟 Koneksi Kuat"
    } else if score >= 60 {
        "⭐ Potensi Bagus"
    } else if score >= 50 {
        "🌙 Perlu Perjuangan"
    } else {
        "🌑 Tantangan Berat"
    }
}

fn reading(score: u32) -> &'static str {
    if score >= 80 {
        "Jiwa kalian memiliki koneksi yang sangat istimewa dan langka. Takdir telah merencanakan pertemuan ini."
    } else if score >= 60 {
        "Ada chemistry yang kuat di antara kalian. Perbedaan kalian justru menciptakan harmoni."
    } else if score >= 40 {
        "Butuh waktu untuk saling memahami. Setiap tantangan akan memperkuat ikatan kalian."
    } else {
        "Perbedaan signifikan dalam energi jiwa. Butuh banyak adaptasi dan pengertian."
    }
}

/// Seed acak 0..100 — JS memakai `Date.now() % 100`; `Date.now()` tidak ada di
/// `Ctx`, jadi disamakan dengan undian seragam.
fn soul_seed() -> u32 {
    rand::rng().random_range(0..100)
}

fn render_soulmatch(
    nama1: &str,
    nama2: &str,
    s1: (&str, &str, &str),
    s2: (&str, &str, &str),
    score: u32,
) -> String {
    let mut t = String::new();
    t.push_str("╭═══❯ *💫 SOUL MATCH* ❮═══\n");
    t.push_str("│\n");
    t.push_str(&format!("│ 👤 *{nama1}*\n"));
    t.push_str(&format!("│ ├ 🔮 Soul: {}\n", s1.2));
    t.push_str(&format!("│ ├ 🌟 Element: {}\n", s1.0));
    t.push_str(&format!("│ └ 🎯 Zodiac: {}\n", s1.1));
    t.push_str("│\n");
    t.push_str(&format!("│ 👤 *{nama2}*\n"));
    t.push_str(&format!("│ ├ 🔮 Soul: {}\n", s2.2));
    t.push_str(&format!("│ ├ 🌟 Element: {}\n", s2.0));
    t.push_str(&format!("│ └ 🎯 Zodiac: {}\n", s2.1));
    t.push_str("│\n");
    t.push_str("│ 💕 *COMPATIBILITY*\n");
    t.push_str(&format!("│ ├ 📊 Score: *{score}%*\n"));
    t.push_str(&format!("│ └ 🎭 Status: {}\n", match_description(score)));
    t.push_str("│\n");
    t.push_str("│ 🔮 *Reading:*\n");
    t.push_str(&format!("│ {}\n", reading(score)));
    t.push_str("│\n");
    t.push_str("╰════════════════════");
    t
}

#[plugin(
    name = "soulmatch",
    alias = [],
    category = "fun",
    description = "Cek kecocokan jiwa dengan seseorang",
    usage = ".soulmatch nama1|nama2",
    example = ".soulmatch Raiden|Mei",
    cooldown = 15,
    energi = 1,
    is_private = false
)]
pub fn soulmatch(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() || !text.contains('|') {
        return Reply::text(HELP_SOULMATCH);
    }

    let mut names = text.split('|').map(str::trim);
    let nama1 = names.next().unwrap_or("");
    let nama2 = names.next().unwrap_or("");
    if nama1.is_empty() || nama2.is_empty() {
        return Reply::text(format!(
            "❌ Masukkan 2 nama dengan format: `{}soulmatch nama1|nama2`",
            ctx.prefix
        ));
    }

    let s1 = soul_data(nama1, soul_seed());
    let s2 = soul_data(nama2, soul_seed());
    Reply::text(render_soulmatch(nama1, nama2, s1, s2, compatibility(nama1, nama2)))
}

// ---------------------------------------------------------------------------
// 5. kalkulatormbg
// ---------------------------------------------------------------------------

const PENGELUARAN_PER_HARI: f64 = 319_600_000_000.0;
const HARGA_PORSI: f64 = 15_000.0;

const UMR_DKI: f64 = 5_400_000.0;
const UMR_JATENG: f64 = 2_040_000.0;
const GAJI_GURU_HONORER: f64 = 300_000.0;

const PEMAIN: [(&str, f64); 10] = [
    ("Cristiano Ronaldo (Al Nassr)", 4_500_000_000_000.0),
    ("Lionel Messi (Inter Miami)", 2_100_000_000_000.0),
    ("Karim Benzema (Al-Ittihad)", 1_700_000_000_000.0),
    ("Kylian Mbappé (Real Madrid)", 1_500_000_000_000.0),
    ("Erling Haaland (Man City)", 1_300_000_000_000.0),
    ("Vinícius Jr. (Real Madrid)", 960_000_000_000.0),
    ("Mohamed Salah (Liverpool)", 880_000_000_000.0),
    ("Sadio Mané (Al Nassr)", 864_000_000_000.0),
    ("Jude Bellingham (Real Madrid)", 704_000_000_000.0),
    ("Lamine Yamal (Barcelona)", 688_000_000_000.0),
];

const HELP_KKMBG_HEAD: &str = "🧮 *KALKULATOR MBG (Makan Bergizi Gratis)* 🧮";
const HELP_KKMBG_BODY: &str = "Halo kak! Penasaran berapa lama uang kamu bisa nyuplai program Makan Bergizi Gratis se-Indonesia?\n\n*Cara Pakai:*\n👉 `{prefix}kkmbg <nominal uang>`\n\n*Contoh:*\n`.kkmbg 1000000000`";
const KKMBG_INVALID: &str =
    "❌ Kak, tolong masukin angka uang yang valid ya! (Cuma angka aja, misal 500000)";

struct Durasi {
    tahun: u64,
    bulan: u64,
    hari: u64,
    jam: u64,
    menit: u64,
    detik: f64,
}

/// `Number(raw.replace(/[^0-9]/g, ''))` — buang semua non-digit, lalu parse.
/// String kosong jadi `0` (`Number("") === 0`), sama seperti JS.
fn parse_uang(raw: &str) -> f64 {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<f64>().unwrap_or(0.0)
}

/// `Number.prototype.toLocaleString('id-ID')` untuk bilangan bulat: pemisah
/// ribuan titik (`1000000000` → `1.000.000.000`).
fn thousands_id(n: f64) -> String {
    let n = n.abs() as u64;
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// `angka < 1000 ? "… Perak" : "Rp …"`.
fn format_rupiah(angka: f64) -> String {
    let formatted = thousands_id(angka);
    if angka < 1000.0 {
        format!("{formatted} Perak")
    } else {
        format!("Rp {formatted}")
    }
}

fn hitung_durasi(hari_float: f64) -> Durasi {
    Durasi {
        tahun: (hari_float / 365.0).floor() as u64,
        bulan: ((hari_float % 365.0) / 30.0).floor() as u64,
        // JS: `hariFloat % 30` — BUKAN `(hariFloat % 365) % 30`.
        hari: (hari_float % 30.0).floor() as u64,
        jam: ((hari_float % 1.0) * 24.0).floor() as u64,
        menit: (((hari_float * 24.0) % 1.0) * 60.0).floor() as u64,
        detik: ((hari_float * 24.0 * 60.0) % 1.0) * 60.0,
    }
}

fn render_mbg(uang: f64) -> String {
    let durasi = hitung_durasi(uang / PENGELUARAN_PER_HARI);
    let porsi = (uang / HARGA_PORSI).floor() as u64;

    let mut c = String::new();
    c.push_str(&format!("💰 *Dana :* {}\n\n", format_rupiah(uang)));
    c.push_str("⏳ *Durasi MBG:*\n");
    c.push_str(&format!(
        "{} TAHUN, {} BULAN, {} HARI\n",
        durasi.tahun, durasi.bulan, durasi.hari
    ));
    c.push_str(&format!(
        "{} JAM, {} MENIT, {:.2} DETIK\n",
        durasi.jam, durasi.menit, durasi.detik
    ));
    c.push_str(&format!(
        "_(Berdasarkan pengeluaran ~Rp {:.1} Miliar/hari)_\n\n",
        PENGELUARAN_PER_HARI / 1_000_000_000.0
    ));

    c.push_str("🍱 *Setara Porsi Makan:*\n");
    c.push_str(&format!(
        "{} porsi (@ Rp 15.000/porsi)\n\n",
        thousands_id(porsi as f64)
    ));

    c.push_str("📊 *Perbandingan Gaji Indonesia:*\n");
    c.push_str(&format!(
        "🏢 UMR DKI Jakarta (Rp 5,4 Jt/bulan): {:.1}%\n",
        uang / UMR_DKI * 100.0
    ));
    c.push_str(&format!(
        "🏭 UMR Jawa Tengah (Rp 2,04 Jt/bulan): {:.1}%\n",
        uang / UMR_JATENG * 100.0
    ));
    c.push_str(&format!(
        "👨‍🏫 Gaji Guru Honorer (Rp 300rb/bulan): {:.1}x\n\n",
        uang / GAJI_GURU_HONORER
    ));

    c.push_str("⚽ *Perbandingan Gaji Pesepakbola:*\n");
    for (nama, gaji) in PEMAIN {
        let persen = uang / gaji * 100.0;
        c.push_str(&format!("🏆 {nama}\n"));
        c.push_str(&format!("💵 {}/tahun\n", format_rupiah(gaji)));
        // JS: `persen < 0.0001 ? "0%" : persen.toFixed(4) + "%"`
        c.push_str(&format!(
            "📈 Persentase: {}\n\n",
            if persen < 0.0001 {
                "0%".to_string()
            } else {
                format!("{persen:.4}%")
            }
        ));
    }

    format!(
        "🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n{}",
        c.trim()
    )
}

#[plugin(
    name = "kalkulatormbg",
    alias = ["kkmbg"],
    category = "tools",
    description = "Hitung durasi dan perbandingan dana Makan Bergizi Gratis (MBG)",
    usage = ".kkmbg <jumlah_uang>",
    example = ".kkmbg 1000000000",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn kalkulatormbg(ctx: Ctx) -> Reply {
    let Some(raw) = ctx.args.first() else {
        return Reply::text(format!(
            "{HELP_KKMBG_HEAD}\n\n{}",
            HELP_KKMBG_BODY.replace("{prefix}", &ctx.prefix)
        ));
    };

    // JS: `isNaN(uang) || uang <= 0`. `parse_uang` tidak pernah menghasilkan
    // NaN (digit kosong -> 0.0), jadi cukup cek <= 0.
    let uang = parse_uang(raw);
    if uang <= 0.0 {
        return Reply::text(KKMBG_INVALID);
    }

    Reply::text(render_mbg(uang))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(command: &str, text: &str) -> Ctx {
        Ctx {
            prefix: ".".to_string(),
            command: command.to_string(),
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".to_string(),
            ..Default::default()
        }
    }

    /// `(command, alias[], answers, help)` untuk 10 plugin "tanya bot".
    fn tanya_family() -> Vec<(&'static str, Vec<&'static str>, &'static [&'static str], &'static str)> {
        vec![
            ("apakah", vec!["apa"], &APAKAH, HELP_APAKAH),
            ("mengapa", vec!["kenapa", "why"], &MENGAPA, HELP_MENGAPA),
            ("kapan", vec!["when"], &KAPAN, HELP_KAPAN),
            ("akankah", vec!["akan", "will"], &AKANKAH, HELP_AKANKAH),
            (
                "bagaimana",
                vec!["gimana", "how"],
                &BAGAIMANA,
                HELP_BAGAIMANA,
            ),
            ("bisakah", vec!["bisa"], &BISAKAH, HELP_BISAKAH),
            ("coba", vec!["try"], &COBA, HELP_COBA),
            ("dimana", vec!["where", "mana"], &DIMANA, HELP_DIMANA),
            ("haruskah", vec!["harus", "should"], &HARUSKAH, HELP_HARUSKAH),
            (
                "berapa",
                vec!["howmuch", "howmany"],
                &BERAPA,
                HELP_BERAPA,
            ),
        ]
    }

    /// Ekstrak `Reply` jadi `String` milik sendiri, supaya bisa dibaca tanpa
    /// menahan pinjaman ke temporary.
    fn handler_text(reply: &Reply) -> String {
        reply
            .text_body()
            .expect("plugin harus membalas teks")
            .to_string()
    }

    /// Ambil isi `*<answer>*` paling kanan dari `render_tanya`.
    fn tail_answer(text: &str) -> &str {
        let inner = text.strip_suffix('*').expect("jawaban ditutup *");
        inner
            .rsplit_once('*')
            .map(|(_, answer)| answer)
            .expect("jawaban dibuka *")
    }

    // -- registrasi & meta --------------------------------------------------

    #[test]
    fn tanya_family_registers_with_correct_meta() {
        for (name, _, _, _) in tanya_family() {
            let p = oktz_plugin_api::find(name).unwrap_or_else(|| panic!("{name} tidak terdaftar"));
            assert_eq!(p.meta.name, name);
            assert_eq!(p.meta.category, "fun");
            assert_eq!(p.meta.description, meta_description(name));
            assert_eq!(p.meta.cooldown, 3);
            assert_eq!(p.meta.energi, 0);
            assert!(!p.meta.is_private);
            assert!(!p.meta.is_group);
            assert!(p.meta.is_enabled);
        }
    }

    fn meta_description(name: &str) -> &'static str {
        match name {
            "apakah" => "Tanya bot apakah sesuatu",
            "mengapa" => "Tanya bot mengapa sesuatu",
            "kapan" => "Tanya bot kapan sesuatu",
            "akankah" => "Tanya bot akankah sesuatu terjadi",
            "bagaimana" => "Tanya bot bagaimana sesuatu",
            "bisakah" => "Tanya bot bisakah sesuatu",
            "coba" => "Coba tanyakan sesuatu ke bot",
            "dimana" => "Tanya bot dimana sesuatu",
            "haruskah" => "Tanya bot haruskah sesuatu",
            "berapa" => "Tanya bot berapa sesuatu",
            other => panic!("meta tak dikenal untuk {other}"),
        }
    }

    #[test]
    fn tanya_family_aliases_resolve_to_the_right_plugin() {
        for (name, aliases, _, _) in tanya_family() {
            for alias in aliases {
                let found = oktz_plugin_api::find(alias)
                    .unwrap_or_else(|| panic!("alias {alias} tidak resolve"));
                assert_eq!(found.meta.name, name, "alias {alias} salah resolve");
            }
        }
    }

    #[test]
    fn rate_registers_with_correct_meta() {
        let p = oktz_plugin_api::find("rate").unwrap();
        assert_eq!(p.meta.name, "rate");
        assert_eq!(p.meta.category, "fun");
        assert_eq!(
            p.meta.description,
            "Minta bot memberi rating sesuatu"
        );
        assert_eq!(p.meta.usage, ".rate <sesuatu>");
        assert_eq!(p.meta.example, ".rate wajahku");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_private);
    }

    #[test]
    fn rate_aliases_resolve() {
        assert_eq!(oktz_plugin_api::find("nilai").unwrap().meta.name, "rate");
        assert_eq!(oktz_plugin_api::find("rating").unwrap().meta.name, "rate");
    }

    #[test]
    fn readmore_registers_with_correct_meta() {
        let p = oktz_plugin_api::find("readmore").unwrap();
        assert_eq!(p.meta.name, "readmore");
        assert_eq!(p.meta.category, "tools");
        assert_eq!(
            p.meta.description,
            "Membuat teks baca selengkapnya (spoiler)"
        );
        assert_eq!(p.meta.usage, ".readmore <text_awal>|<text_akhir>");
        assert_eq!(p.meta.example, ".readmore Hai|Ini adalah pesan rahasia");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_private);
        assert!(!p.meta.is_group);
        assert!(!p.meta.is_admin);
        assert!(!p.meta.is_bot_admin);
    }

    #[test]
    fn readmore_aliases_resolve() {
        for alias in ["selengkapnya", "spoiler"] {
            assert_eq!(oktz_plugin_api::find(alias).unwrap().meta.name, "readmore");
        }
    }

    #[test]
    fn soulmatch_registers_with_correct_meta() {
        let p = oktz_plugin_api::find("soulmatch").unwrap();
        assert_eq!(p.meta.name, "soulmatch");
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Cek kecocokan jiwa dengan seseorang");
        assert_eq!(p.meta.usage, ".soulmatch nama1|nama2");
        assert_eq!(p.meta.example, ".soulmatch Raiden|Mei");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.alias.is_empty());
        assert!(!p.meta.is_private);
    }

    #[test]
    fn kalkulatormbg_registers_with_correct_meta() {
        let p = oktz_plugin_api::find("kalkulatormbg").unwrap();
        assert_eq!(p.meta.name, "kalkulatormbg");
        assert_eq!(p.meta.category, "tools");
        assert_eq!(
            p.meta.description,
            "Hitung durasi dan perbandingan dana Makan Bergizi Gratis (MBG)"
        );
        assert_eq!(p.meta.usage, ".kkmbg <jumlah_uang>");
        assert_eq!(p.meta.example, ".kkmbg 1000000000");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_private);
        assert_eq!(oktz_plugin_api::find("kkmbg").unwrap().meta.name, "kalkulatormbg");
    }

    #[test]
    fn every_ported_plugin_answers_through_the_registry() {
        for name in [
            "apakah",
            "mengapa",
            "kapan",
            "akankah",
            "bagaimana",
            "bisakah",
            "coba",
            "dimana",
            "haruskah",
            "berapa",
            "rate",
            "readmore",
            "soulmatch",
            "kalkulatormbg",
        ] {
            let p = oktz_plugin_api::find(name).unwrap();
            let reply = (p.handler)(ctx_of(name, "apa nih"));
            assert!(
                reply.text_body().is_some(),
                "{name} harus membalas teks"
            );
        }
    }

    // -- keluarga tanya bot -------------------------------------------------

    #[test]
    fn empty_text_returns_each_plugins_own_help_verbatim() {
        for (name, _, _, help) in tanya_family() {
            let reply = (oktz_plugin_api::find(name).unwrap().handler)(ctx_of(name, ""));
            assert_eq!(handler_text(&reply), help, "{name} cabang.help");
        }
    }

    #[test]
    fn whitespace_only_text_also_takes_the_help_branch() {
        let reply = tanya_reply(&ctx_of("apakah", "   \t "), &APAKAH, HELP_APAKAH);
        assert_eq!(handler_text(&reply), HELP_APAKAH);
    }

    #[test]
    fn answer_branch_echoes_command_plus_question_with_a_question_mark() {
        // Referensi dari menjalankan handler JS dengan body ".apakah aku bisa kaya?"
        // -> "apakah aku bisa kaya??\n*<jawaban>*" (dua tanda tanya, JS tidak
        // mengecek apakah input sudah berakhir dengan `?`).
        let expected = format!("apakah aku bisa kaya??\n*{}*", APAKAH[5]);
        let ctx = ctx_of("apakah", "aku bisa kaya?");
        // Reply acak, jadi hanya bisa diuji lewat `render_tanya` yang deterministik.
        assert_eq!(render_tanya("apakah aku bisa kaya?", APAKAH[5]), expected);
        let out = handler_text(&tanya_reply(&ctx, &APAKAH, HELP_APAKAH));
        assert!(out.starts_with("apakah aku bisa kaya??\n*"));
        assert!(APAKAH.contains(&tail_answer(&out)));
    }

    #[test]
    fn answer_is_always_drawn_from_that_plugins_own_pool() {
        for (name, _, answers, _) in tanya_family() {
            let entry = oktz_plugin_api::find(name).unwrap();
            for _ in 0..400 {
                let ctx = ctx_of(name, "apakah kamu bisa");
                let out = handler_text(&(entry.handler)(ctx));
                let answer = tail_answer(&out);
                assert!(
                    answers.contains(&answer),
                    "{name} membalas {answer:?} yang bukan dari pool-nya"
                );
                assert!(out.starts_with(&format!("{name} apakah kamu bisa?")));
            }
        }
    }

    #[test]
    fn random_index_covers_every_slot_and_never_the_length() {
        // `Math.floor(Math.random() * len)` -> [0, len). Kalau jadi 0..=len,
        // slot terakhir jadi tidak pernah terpilih.
        for len in [APAKAH.len(), BERAPA.len(), RATE.len(), 1usize] {
            let mut seen = vec![false; len];
            for _ in 0..4000 {
                let i = random_index(len);
                assert!(i < len, "indeks {i} keluar dari range {len}");
                seen[i] = true;
            }
            assert!(seen.iter().all(|s| *s), "ada slot yang tak akan pernah muncul");
        }
    }

    #[test]
    fn body_stripped_drops_exactly_one_char_of_the_prefix() {
        let mut ctx = ctx_of("apakah", "halo");
        ctx.prefix = ".".to_string();
        assert_eq!(body_stripped(&ctx), "apakah halo");

        ctx.prefix = "!!".to_string();
        assert_eq!(body_stripped(&ctx), "!apakah halo", "slice(1) hanya satu char");

        // Mode noprefix: `m.body.slice(1)` memotong command, bukan prefix.
        ctx.prefix = String::new();
        assert_eq!(body_stripped(&ctx), "pakah halo");
    }

    #[test]
    fn body_stripped_is_char_safe_on_non_ascii() {
        let mut ctx = ctx_of("apakah", "bisa 🎉 tidak");
        ctx.prefix = "😀".to_string();
        assert_eq!(body_stripped(&ctx), "apakah bisa 🎉 tidak");
        assert_eq!(body_stripped(&ctx).chars().count(), 19);
    }

    #[test]
    fn body_stripped_without_text_has_no_trailing_space() {
        let ctx = ctx_of("apakah", "");
        assert_eq!(body_stripped(&ctx), "apakah");
    }

    // -- rate ---------------------------------------------------------------

    #[test]
    fn rate_empty_text_shows_help() {
        let reply = rate(ctx_of("rate", ""));
        assert_eq!(handler_text(&reply), HELP_RATE);
    }

    #[test]
    fn rate_answer_is_a_whole_score_comment_pair_from_the_table() {
        for _ in 0..400 {
            let out = handler_text(&rate(ctx_of("rate", "wajahku")));
            let (score, comment) = out
                .strip_prefix("Rating dari aku: *")
                .and_then(|rest| rest.split_once("*\n"))
                .unwrap_or_else(|| panic!("bentuk balasan berubah: {out:?}"));
            assert!(
                RATE.iter().any(|(s, c)| *s == score && *c == comment),
                "pasangan tak dikenal: {score} / {comment}"
            );
        }
    }

    #[test]
    fn rate_render_matches_the_javascript_shape() {
        assert_eq!(
            render_rate("11/10", "Melebihi ekspektasi!"),
            "Rating dari aku: *11/10*\nMelebihi ekspektasi!"
        );
        assert_eq!(RATE[0], ("10/10", "Sempurna! Nggak ada duanya!"));
        assert_eq!(RATE[14], ("∞/10", "Gacor kang"));
        assert_eq!(RATE[19], ("???/10", "Error 404: Rating not found."));
    }

    // -- readmore -----------------------------------------------------------

    #[test]
    fn readmore_empty_text_shows_usage_with_prefix_and_command() {
        let reply = readmore(ctx_of("readmore", ""));
        assert_eq!(
            handler_text(&reply),
            "⚠️ Masukan text!\nContoh: `.readmore Halo|Ini teks tersembunyi`"
        );
    }

    #[test]
    fn readmore_inserts_exactly_4001_lr_marks() {
        let out = readmore_body("Hai|Ini adalah pesan rahasia");
        assert!(out.starts_with("Hai"));
        assert!(out.ends_with("Ini adalah pesan rahasia"));
        assert_eq!(out.chars().filter(|c| *c == '\u{200e}').count(), LRM_COUNT);
        // "Hai" (3) + 4001 + "Ini adalah pesan rahasia" (24)
        assert_eq!(out.chars().count(), 4028);
    }

    #[test]
    fn readmore_keeps_only_the_first_two_pipe_segments() {
        let out = readmore_body("A|B|C");
        assert!(out.starts_with('A'));
        assert!(out.ends_with('B'));
        assert!(!out.contains('C'), "segmen ketiga harus dibuang");

        // Tanpa `|` sama sekali: bagian kanan kosong.
        let only_left = readmore_body("HanyaAwal");
        assert!(only_left.starts_with("HanyaAwal"));
        assert_eq!(only_left.chars().count(), 9 + LRM_COUNT); // "HanyaAwal" = 9
        assert!(only_left.ends_with('\u{200e}'));

        // Hanya pembatas: bagian kiri kosong.
        let only_right = readmore_body("|HanyaAkhir");
        assert!(only_right.starts_with('\u{200e}'));
        assert_eq!(only_right.chars().count(), LRM_COUNT + 10);
        assert!(only_right.ends_with("HanyaAkhir"));
    }

    #[test]
    fn readmore_handler_is_byte_exact_for_the_js_reference_input() {
        let out = handler_text(&readmore(ctx_of("readmore", "Hai|Ini adalah pesan rahasia")));
        assert_eq!(out.chars().count(), 4028);
        assert!(out.starts_with("Hai\u{200e}"));
        assert!(out.ends_with("\u{200e}Ini adalah pesan rahasia"));
    }

    // -- soulmatch ----------------------------------------------------------

    #[test]
    fn soulmatch_without_pipe_returns_help() {
        assert_eq!(
            handler_text(&soulmatch(ctx_of("soulmatch", ""))),
            HELP_SOULMATCH
        );
        assert_eq!(
            handler_text(&soulmatch(ctx_of("soulmatch", "Raiden"))),
            HELP_SOULMATCH
        );
    }

    #[test]
    fn soulmatch_with_an_empty_half_returns_the_error_line() {
        assert_eq!(
            handler_text(&soulmatch(ctx_of("soulmatch", "Raiden|"))),
            "❌ Masukkan 2 nama dengan format: `.soulmatch nama1|nama2`"
        );
        assert_eq!(
            handler_text(&soulmatch(ctx_of("soulmatch", "|Mei"))),
            "❌ Masukkan 2 nama dengan format: `.soulmatch nama1|nama2`"
        );
    }

    #[test]
    fn soul_data_matches_the_js_reference_values() {
        assert_eq!(
            soul_data("Raiden", 10),
            ("Es ❄️", "♓ Pisces", "Pemimpin Yang Berani")
        );
        assert_eq!(
            soul_data("Raiden", 42),
            ("Es ❄️", "♋ Cancer", "Pemikir Mistis")
        );
        assert_eq!(
            soul_data("Mei", 60),
            ("Bayangan 🌑", "♋ Cancer", "Pemimpin Yang Berani")
        );
        assert_eq!(
            soul_data("budi", 7),
            ("Angin 🌪️", "♊ Gemini", "Pemimpin Yang Berani")
        );
    }

    #[test]
    fn soul_data_always_lands_inside_each_table() {
        for name in ["raiden", "mei", "budi", "", "😀 emoji"] {
            for seed in 0..120u32 {
                let (e, z, s) = soul_data(name, seed);
                assert!(ELEMENTS.contains(&e), "element di luar tabel: {e}");
                assert!(ZODIAC.contains(&z), "zodiac di luar tabel: {z}");
                assert!(SOUL_TYPES.contains(&s), "soulType di luar tabel: {s}");
            }
        }
    }

    #[test]
    fn compatibility_is_deterministic_and_inside_50_to_100() {
        // Reference dari JS: "Raiden|Mei" -> 74.
        assert_eq!(compatibility("Raiden", "Mei"), 74);
        for _ in 0..50 {
            assert_eq!(compatibility("Raiden", "Mei"), 74, "harus deterministik");
        }
        for (a, b) in [("a", "b"), ("ani", "budi"), ("", ""), ("😀", "🎉")] {
            let c = compatibility(a, b);
            assert!((50..=100).contains(&c), "score {c} di luar [50,100]");
        }
    }

    #[test]
    fn soulmatch_status_and_reading_follow_the_score_bands() {
        for (score, status) in [
            (100, "💫 Takdir Sejati"),
            (90, "💫 Takdir Sejati"),
            (89, "✨ Harmoni Sempurna"),
            (80, "✨ Harmoni Sempurna"),
            (79, "🌟 Koneksi Kuat"),
            (70, "🌟 Koneksi Kuat"),
            (69, "⭐ Potensi Bagus"),
            (60, "⭐ Potensi Bagus"),
            (59, "🌙 Perlu Perjuangan"),
            (50, "🌙 Perlu Perjuangan"),
            (49, "🌑 Tantangan Berat"),
            (0, "🌑 Tantangan Berat"),
        ] {
            assert_eq!(match_description(score), status, "score {score}");
        }

        for (score, head) in [
            (100, "Jiwa kalian memiliki koneksi"),
            (80, "Jiwa kalian memiliki koneksi"),
            (79, "Ada chemistry yang kuat"),
            (60, "Ada chemistry yang kuat"),
            (59, "Butuh waktu untuk saling memahami"),
            (40, "Butuh waktu untuk saling memahami"),
            (39, "Perbedaan signifikan dalam energi jiwa"),
            (0, "Perbedaan signifikan dalam energi jiwa"),
        ] {
            assert!(reading(score).starts_with(head), "score {score}: {head}");
        }
    }

    #[test]
    fn soulmatch_render_has_the_full_box_and_the_score() {
        let s1 = soul_data("Raiden", 10);
        let s2 = soul_data("Mei", 60);
        let out = render_soulmatch("Raiden", "Mei", s1, s2, compatibility("Raiden", "Mei"));
        assert!(out.starts_with("╭═══❯ *💫 SOUL MATCH* ❮═══\n│\n"));
        assert!(out.ends_with("╰════════════════════"));
        assert!(!out.ends_with('\n'), "tanpa newline di akhir");
        assert!(out.contains("│ 👤 *Raiden*"));
        assert!(out.contains("│ 👤 *Mei*"));
        assert!(out.contains("│ ├ 🔮 Soul: Pemimpin Yang Berani"));
        assert!(out.contains("│ ├ 🌟 Element: Es ❄️"));
        assert!(out.contains("│ └ 🎯 Zodiac: ♓ Pisces"));
        assert!(out.contains("│ ├ 📊 Score: *74%*"));
        assert!(out.contains("│ └ 🎭 Status: 🌟 Koneksi Kuat"));
        assert!(out.contains(
            "│ Ada chemistry yang kuat di antara kalian. Perbedaan kalian justru menciptakan harmoni."
        ));
    }

    #[test]
    fn soulmatch_handler_always_reports_the_same_score_for_a_given_pair() {
        let entry = oktz_plugin_api::find("soulmatch").unwrap();
        for _ in 0..20 {
            let out = handler_text(&(entry.handler)(ctx_of("soulmatch", "Raiden|Mei")));
            assert!(out.contains("│ ├ 📊 Score: *74%*"), "score berubah: {out}");
            assert!(out.contains("│ 👤 *Raiden*"));
            assert!(out.contains("│ 👤 *Mei*"));
        }
    }

    // -- kalkulatormbg ------------------------------------------------------

    #[test]
    fn parse_uang_strips_every_non_digit_and_defaults_to_zero() {
        assert_eq!(parse_uang("1.234.567"), 1_234_567.0);
        assert_eq!(parse_uang("500000"), 500_000.0);
        assert_eq!(parse_uang("abc"), 0.0);
        assert_eq!(parse_uang(""), 0.0);
        assert_eq!(parse_uang("-12"), 12.0, "tanda minus dibuang, bukan membuat negatif");
        assert_eq!(parse_uang("Rp 1.000"), 1_000.0);
        assert_eq!(parse_uang("12,5"), 125.0, "koma bukan pemisah ribuan");
    }

    #[test]
    fn thousands_id_uses_dots_every_three_digits() {
        assert_eq!(thousands_id(0.0), "0");
        assert_eq!(thousands_id(82.0), "82");
        assert_eq!(thousands_id(500.0), "500");
        assert_eq!(thousands_id(2_700.0), "2.700");
        assert_eq!(thousands_id(66_666.0), "66.666");
        assert_eq!(thousands_id(1_234_567.0), "1.234.567");
        assert_eq!(thousands_id(1_000_000_000.0), "1.000.000.000");
        assert_eq!(thousands_id(4_500_000_000_000.0), "4.500.000.000.000");
    }

    #[test]
    fn format_rupiah_switches_from_perak_to_rupiah_at_1000() {
        assert_eq!(format_rupiah(0.0), "0 Perak");
        assert_eq!(format_rupiah(999.0), "999 Perak");
        assert_eq!(format_rupiah(1_000.0), "Rp 1.000");
        assert_eq!(format_rupiah(2_700.0), "Rp 2.700");
        assert_eq!(format_rupiah(1_000_000_000.0), "Rp 1.000.000.000");
    }

    #[test]
    fn kalkulatormbg_without_args_shows_usage_with_prefix() {
        assert_eq!(
            handler_text(&kalkulatormbg(ctx_of("kalkulatormbg", ""))),
            format!(
                "{HELP_KKMBG_HEAD}\n\n{}",
                HELP_KKMBG_BODY.replace("{prefix}", ".")
            )
        );
    }

    #[test]
    fn kalkulatormbg_rejects_non_positive_and_alphabetic_input() {
        for bad in ["0", "abc", "-", "Rp 0"] {
            let ctx = ctx_of("kalkulatormbg", bad);
            assert_eq!(
                handler_text(&kalkulatormbg(ctx)),
                KKMBG_INVALID,
                "input {bad} harus ditolak"
            );
        }
    }

    #[test]
    fn kalkulatormbg_accepts_the_javascript_reference_inputs() {
        // Output diambil persis dari menjalankan handler JS.
        assert_eq!(
            handler_text(&kalkulatormbg(ctx_of("kalkulatormbg", "1000000000"))),
            concat!(
                "🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n",
                "💰 *Dana :* Rp 1.000.000.000\n\n",
                "⏳ *Durasi MBG:*\n",
                "0 TAHUN, 0 BULAN, 0 HARI\n",
                "0 JAM, 4 MENIT, 30.34 DETIK\n",
                "_(Berdasarkan pengeluaran ~Rp 319.6 Miliar/hari)_\n\n",
                "🍱 *Setara Porsi Makan:*\n",
                "66.666 porsi (@ Rp 15.000/porsi)\n\n",
                "📊 *Perbandingan Gaji Indonesia:*\n",
                "🏢 UMR DKI Jakarta (Rp 5,4 Jt/bulan): 18518.5%\n",
                "🏭 UMR Jawa Tengah (Rp 2,04 Jt/bulan): 49019.6%\n",
                "👨‍🏫 Gaji Guru Honorer (Rp 300rb/bulan): 3333.3x\n\n",
                "⚽ *Perbandingan Gaji Pesepakbola:*\n",
                "🏆 Cristiano Ronaldo (Al Nassr)\n",
                "💵 Rp 4.500.000.000.000/tahun\n",
                "📈 Persentase: 0.0222%\n\n",
                "🏆 Lionel Messi (Inter Miami)\n",
                "💵 Rp 2.100.000.000.000/tahun\n",
                "📈 Persentase: 0.0476%\n\n",
                "🏆 Karim Benzema (Al-Ittihad)\n",
                "💵 Rp 1.700.000.000.000/tahun\n",
                "📈 Persentase: 0.0588%\n\n",
                "🏆 Kylian Mbappé (Real Madrid)\n",
                "💵 Rp 1.500.000.000.000/tahun\n",
                "📈 Persentase: 0.0667%\n\n",
                "🏆 Erling Haaland (Man City)\n",
                "💵 Rp 1.300.000.000.000/tahun\n",
                "📈 Persentase: 0.0769%\n\n",
                "🏆 Vinícius Jr. (Real Madrid)\n",
                "💵 Rp 960.000.000.000/tahun\n",
                "📈 Persentase: 0.1042%\n\n",
                "🏆 Mohamed Salah (Liverpool)\n",
                "💵 Rp 880.000.000.000/tahun\n",
                "📈 Persentase: 0.1136%\n\n",
                "🏆 Sadio Mané (Al Nassr)\n",
                "💵 Rp 864.000.000.000/tahun\n",
                "📈 Persentase: 0.1157%\n\n",
                "🏆 Jude Bellingham (Real Madrid)\n",
                "💵 Rp 704.000.000.000/tahun\n",
                "📈 Persentase: 0.1420%\n\n",
                "🏆 Lamine Yamal (Barcelona)\n",
                "💵 Rp 688.000.000.000/tahun\n",
                "📈 Persentase: 0.1453%",
            )
        );
    }

    #[test]
    fn kalkulatormbg_below_1000_uses_perak_and_zero_porsi() {
        assert_eq!(
            handler_text(&kalkulatormbg(ctx_of("kalkulatormbg", "500"))),
            concat!(
                "🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n",
                "💰 *Dana :* 500 Perak\n\n",
                "⏳ *Durasi MBG:*\n",
                "0 TAHUN, 0 BULAN, 0 HARI\n",
                "0 JAM, 0 MENIT, 0.00 DETIK\n",
                "_(Berdasarkan pengeluaran ~Rp 319.6 Miliar/hari)_\n\n",
                "🍱 *Setara Porsi Makan:*\n",
                "0 porsi (@ Rp 15.000/porsi)\n\n",
                "📊 *Perbandingan Gaji Indonesia:*\n",
                "🏢 UMR DKI Jakarta (Rp 5,4 Jt/bulan): 0.0%\n",
                "🏭 UMR Jawa Tengah (Rp 2,04 Jt/bulan): 0.0%\n",
                "👨‍🏫 Gaji Guru Honorer (Rp 300rb/bulan): 0.0x\n\n",
                "⚽ *Perbandingan Gaji Pesepakbola:*\n",
                "🏆 Cristiano Ronaldo (Al Nassr)\n",
                "💵 Rp 4.500.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Lionel Messi (Inter Miami)\n",
                "💵 Rp 2.100.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Karim Benzema (Al-Ittihad)\n",
                "💵 Rp 1.700.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Kylian Mbappé (Real Madrid)\n",
                "💵 Rp 1.500.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Erling Haaland (Man City)\n",
                "💵 Rp 1.300.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Vinícius Jr. (Real Madrid)\n",
                "💵 Rp 960.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Mohamed Salah (Liverpool)\n",
                "💵 Rp 880.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Sadio Mané (Al Nassr)\n",
                "💵 Rp 864.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Jude Bellingham (Real Madrid)\n",
                "💵 Rp 704.000.000.000/tahun\n",
                "📈 Persentase: 0%\n\n",
                "🏆 Lamine Yamal (Barcelona)\n",
                "💵 Rp 688.000.000.000/tahun\n",
                "📈 Persentase: 0%",
            )
        );
    }

    #[test]
    fn kalkulatormbg_dotted_input_is_digit_stripped_first() {
        let out = handler_text(&kalkulatormbg(ctx_of("kalkulatormbg", "1.234.567")));
        assert!(out.starts_with(
            "🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n💰 *Dana :* Rp 1.234.567\n\n"
        ));
        assert!(out.contains("0 JAM, 0 MENIT, 0.33 DETIK"));
        assert!(out.contains("82 porsi (@ Rp 15.000/porsi)"));
        assert!(out.contains("🏢 UMR DKI Jakarta (Rp 5,4 Jt/bulan): 22.9%"));
        assert!(out.contains("🏭 UMR Jawa Tengah (Rp 2,04 Jt/bulan): 60.5%"));
        assert!(out.contains("👨‍🏫 Gaji Guru Honorer (Rp 300rb/bulan): 4.1x"));
        // 1.234.567 / 960 Milyar masih di bawah ambang 0.0001 -> "0%".
        assert!(out.contains("📈 Persentase: 0%\n"));
        // ...tapi Lamine Yamal (688 Milyar) sudah di atas ambang.
        assert!(out.ends_with("📈 Persentase: 0.0002%"));
    }

    #[test]
    fn kalkulatormbg_never_leaks_a_trailing_blank_line() {
        let out = handler_text(&kalkulatormbg(ctx_of("kalkulatormbg", "1000000000")));
        assert!(!out.ends_with('\n'));
        assert!(!out.contains("\n\n\n"), "contentTxt di-trim sebelum dikirim");
    }

    #[test]
    fn durasi_uses_the_js_modulo_bases_verbatim() {
        // `hari` memakai `hariFloat % 30`, bukan `(hariFloat % 365) % 30`.
        let d = hitung_durasi(1.0);
        assert_eq!(d.tahun, 0);
        assert_eq!(d.bulan, 0);
        assert_eq!(d.hari, 1);
        assert_eq!(d.jam, 0);
        assert_eq!(d.menit, 0);

        // 400 hari: tahun 1, sisa 35 hari, bulan 1 (35/30), hari 400 % 30 = 10.
        let d = hitung_durasi(400.0);
        assert_eq!(d.tahun, 1);
        assert_eq!(d.bulan, 1);
        assert_eq!(d.hari, 10);
        assert_eq!(d.jam, 0);
    }
}