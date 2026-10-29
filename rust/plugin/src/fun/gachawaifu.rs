//! `plugins/fun/gachawaifu.js` — gacha waifu, jaga perasaan, nikahi.
//!
//! Engine `src/lib/waifu.js` + `src/lib/romance.js` + potongan
//! `addExpWithLevelCheck` (`src/lib/level.js:217`) di-inline ke berkas ini
//! supaya `lib/` tidak perlu modul baru. Helper RPG yang sudah ada dipakai
//! ulang lewat `crate::rpg::heal` (`apply_exp`, `math_random`, `rpg_map`,
//! `thousands_en`, `MAX_KOIN`).
//!
//! Yang tidak bisa dicerminkan persis (semua ditandai `ponytail:` di tempat
//! pemakaiannya):
//!
//! 1. Pool `data/waifu/*.js` belum ter-port → `WAIFU_ENTRIES` kosong, jadi
//!    cabang gacha selalu jatuh ke `te()` + reaksi ☢ seperti `waifupool.rs`.
//! 2. Kartu `viewOnceMessage` + `nativeFlowMessage` + gambar Pinterest
//!    (`getWaifuImage`/`getBuffer`) tidak ada di `Reply` → balasan teks polos
//!    dengan isi `body` yang sama; opsi tombol dibuang, `imageUrl` di store
//!    memakai konstanta fallback `gachawaifu.js:40`.
//! 3. Kartu level-up canvas di `checkAndNotifyLevelUp` tidak ikut — hanya
//!    `exp`/`rpg` yang tersimpan (`apply_exp`).
//! 4. `delete user.waifu` → `waifu: null` (SetUser merge, bukan delete).
//! 5. Satu `Reply` hanya bisa satu reaksi — urutan JS (`🕕` lalu `✅`)
//!    dipadatkan ke reaksi TERAKHIR.
//!
//! `pub mod gachawaifu;` perlu ditambah manual ke `src/fun/mod.rs`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::common::{JAKARTA_OFFSET_SECS, date_dmy, error_template, jakarta_parts, js_truthy, now_ms};
use crate::rpg::heal::{MAX_KOIN, apply_exp, math_random, rpg_map, thousands_en};

/// `TIER_ORDER` (`src/lib/romance.js:5`).
const TIER_ORDER: [&str; 5] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];

/// `TIER_WEIGHTS` (`data/waifu/index.js:45`) — bobot per tier sebelum dibagi
/// jumlah entri tier itu.
const TIER_WEIGHTS: [(&str, f64); 5] = [
    ("Common", 55.0),
    ("Rare", 25.0),
    ("Epic", 13.0),
    ("Legendary", 5.5),
    ("Mythic", 1.5),
];

/// Satu baris `DOWRY` (`src/lib/romance.js:12-18`).
pub(crate) struct Dowry {
    limit: i64,
    koin: i64,
    exp: i64,
}

const DOWRY: [(&str, Dowry); 5] = [
    ("Common", Dowry { limit: 1000, koin: 20000, exp: 500 }),
    ("Rare", Dowry { limit: 3000, koin: 60000, exp: 1500 }),
    ("Epic", Dowry { limit: 8000, koin: 200000, exp: 5000 }),
    ("Legendary", Dowry { limit: 15000, koin: 500000, exp: 12000 }),
    ("Mythic", Dowry { limit: 30000, koin: 1000000, exp: 30000 }),
];

/// `MOOD_MULT` (`src/lib/romance.js:20`).
const MOOD_MULT: [(&str, f64); 5] = [
    ("ceria", 1.3),
    ("romantis", 1.2),
    ("biasa", 1.0),
    ("sedih", 0.7),
    ("marah", 0.5),
];

/// `PITY_THRESHOLD` (`src/lib/romance.js:8`).
const PITY_THRESHOLD: i64 = 20;
/// `EVENT_CHANCE` (`src/lib/romance.js:9`).
const EVENT_CHANCE: f64 = 0.18;
/// `ANGER_THRESHOLD` (`src/lib/romance.js:10`).
const ANGER_THRESHOLD: f64 = 50.0;

/// Fallback `getWaifuImage` (`plugins/fun/gachawaifu.js:40`) — dipakai sebagai
/// `imageUrl` tersimpan karena HTTP Pinterest tidak tersedia di `Ctx`.
const IMAGE_FALLBACK: &str =
    "https://i.pinimg.com/736x/8f/3e/2a/8f3e2a77ec65cdbcfad4ff3bc17e825f.jpg";

/// Satu aksi `ACTIONS` (`src/lib/waifu.js:8-74`). `text` memakai placeholder
/// `{n}` yang di-`replace` dengan nama waifu, padanan `text: (n) => ...`.
struct ActionDef {
    key: &'static str,
    phase: &'static str,
    base: (f64, f64),
    exp: i64,
    likes: &'static [&'static str],
    dislikes: &'static [&'static str],
    text: &'static str,
}

const ACTIONS_APPROACH: &[ActionDef] = &[
    ActionDef { key: "jalan_taman", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["tsundere", "dandere", "kuudere", "genki", "amayadori"],
        dislikes: &["himedere", "ojou-sama"],
        text: "🌳 Suasana taman sejuk. *{n}* tersenyum riang sambil menggandeng lenganmu." },
    ActionDef { key: "jalan_mall", phase: "approach", base: (4.0, 10.0), exp: 25,
        likes: &["himedere"], dislikes: &["dandere", "kuudere", "genki"],
        text: "🏢 Mall ramai dan bising. *{n}* hanya berkeliling tanpa terlalu menikmatinya." },
    ActionDef { key: "jalan_pantai", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["genki", "amayadori", "kuudere"], dislikes: &["himedere"],
        text: "🏖️ Ombak pantai menenangkan. *{n}* menikmati angin laut dan menutup mata." },
    ActionDef { key: "jalan_kota", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "deredere"], dislikes: &["dandere", "amayadori"],
        text: "🌆 Kalian berjalan di tepi kota saat lampu mulai menyala. *{n}* terlihat kagum." },
    ActionDef { key: "kafe_kopi", phase: "approach", base: (4.0, 8.0), exp: 25,
        likes: &["kuudere"], dislikes: &["himedere"],
        text: "☕ *{n}* meringis meneguk kopi pahit. Sepertinya ia kurang suka." },
    ActionDef { key: "kafe_matcha", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["dandere", "deredere", "kuudere"], dislikes: &[],
        text: "🍵 Matcha latte manis bikin *{n}* tersenyum manis padamu. Pilihan tepat!" },
    ActionDef { key: "kafe_kue", phase: "approach", base: (8.0, 15.0), exp: 35,
        likes: &["deredere", "himedere", "genki"], dislikes: &[],
        text: "🍰 Berbagi sepotong kue di kafe. *{n}* terlihat bahagia menikmatinya." },
    ActionDef { key: "kafe_susu", phase: "approach", base: (5.0, 10.0), exp: 25,
        likes: &["amayadori", "dandere"], dislikes: &[],
        text: "🥛 Segelas susu hangat. *{n}* menyeruput pelan dan mengangguk puas." },
    ActionDef { key: "bioskop_romantis", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["deredere", "kuudere", "onee-san"], dislikes: &["genki"],
        text: "🎬 Film romantis membuat *{n}* menyenderkan kepala di bahumu." },
    ActionDef { key: "bioskop_horor", phase: "approach", base: (10.0, 20.0), exp: 40,
        likes: &["genki", "tsundere"], dislikes: &["dandere", "kuudere"],
        text: "👻 JUMPSCARE! *{n}* menjerit dan memeluk lenganmu erat sepanjang film." },
    ActionDef { key: "bioskop_animasi", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "amayadori"], dislikes: &["ojou-sama"],
        text: "🎨 Film animasi seru. *{n}* tertawa lepas di beberapa adegan." },
    ActionDef { key: "belanja_baju", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "himedere"], dislikes: &["amayadori"],
        text: "👗 Kamu membelikannya baju lucu. *{n}* memamerkannya sambil pipi merona." },
    ActionDef { key: "belanja_perhiasan", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["himedere", "ojou-sama"], dislikes: &["kuudere"],
        text: "💎 Perhiasan mahal membuat mata *{n}* berbinar bahagia." },
    ActionDef { key: "belanja_boneka", phase: "approach", base: (5.0, 10.0), exp: 25,
        likes: &["deredere", "dandere"], dislikes: &["ojou-sama"],
        text: "🧸 Boneka kecil mungil. *{n}* memeluknya gemas sepanjang perjalanan." },
    ActionDef { key: "karaoke_duet", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["genki", "deredere"], dislikes: &["kuudere", "himedere"],
        text: "🎤 Duet lagu favorit membuat *{n}* semangat tinggi. Kalian tertawa bersama." },
    ActionDef { key: "karaoke_solo", phase: "approach", base: (4.0, 10.0), exp: 25,
        likes: &["tsundere"], dislikes: &["genki"],
        text: "🎵 Kamu menyanyi sendirian sementara *{n}* menonton dengan tatapan datar." },
    ActionDef { key: "arcade_duo", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "tsundere"], dislikes: &["kuudere"],
        text: "🕹️ Adu skor di arcade. *{n}* menantangmu lagi dan lagi." },
    ActionDef { key: "arcade_boneka", phase: "approach", base: (8.0, 14.0), exp: 35,
        likes: &["genki", "deredere"], dislikes: &[],
        text: "🎯 Kamu memenangkan boneka untuk *{n}*. Wajahnya berseri-seri." },
    ActionDef { key: "piknik_taman", phase: "approach", base: (8.0, 15.0), exp: 35,
        likes: &["kuudere", "amayadori", "dandere"], dislikes: &["himedere"],
        text: "🧺 Piknik di atas tikar rumput. *{n}* bersandar santai dan menatap awan." },
    ActionDef { key: "piknik_pantai", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["genki", "amayadori"], dislikes: &["ojou-sama"],
        text: "🏝️ Piknik pantai dengan bekal buatan sendiri. *{n}* sangat menikmatinya." },
    ActionDef { key: "masak_bareng", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["dandere", "onee-san", "deredere"], dislikes: &[],
        text: "🍳 Kalian masak bareng di dapur. *{n}* antusias mengaduk sausnya." },
    ActionDef { key: "masak_kue", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["dandere", "onee-san", "himedere"], dislikes: &[],
        text: "🍰 Memanggang kue bersama. *{n}* bangga dengan hasil kreasimu berdua." },
    ActionDef { key: "restoran_makan", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["deredere", "onee-san", "amayadori"], dislikes: &["kuudere"],
        text: "🍽️ Makan malam di restoran romantis. *{n}* tersenyum menatapmu dari seberang meja." },
    ActionDef { key: "restoran_dimsum", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "deredere"], dislikes: &["ojou-sama"],
        text: "🥟 Cicip dimsum bareng! *{n}* rebutan denganmu sampai tawa." },
    ActionDef { key: "restoran_bbq", phase: "approach", base: (8.0, 14.0), exp: 35,
        likes: &["genki", "onee-san"], dislikes: &["himedere"],
        text: "🍖 Bakar-bakar di BBQ. *{n}* memangangkan daging untukmu." },
    ActionDef { key: "olahraga_hiking", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["genki", "kuudere"], dislikes: &["himedere", "ojou-sama"],
        text: "⛰️ Mendaki bukit. *{n}* terengah-engah tapi semangat!" },
    ActionDef { key: "olahraga_lari", phase: "approach", base: (8.0, 14.0), exp: 35,
        likes: &["genki", "tsundere"], dislikes: &["dandere"],
        text: "🏃 Lari pagi keliling taman. *{n}* menantangmu balapan." },
    ActionDef { key: "olahraga_panjat", phase: "approach", base: (5.0, 10.0), exp: 25,
        likes: &["genki"], dislikes: &["himedere", "ojou-sama"],
        text: "🧗 Panjat tebing indoor. *{n}* gemetaran tapi tak mau menyerah." },
    ActionDef { key: "alam_camping", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["kuudere", "amayadori", "dandere"], dislikes: &["himedere"],
        text: "🏕️ Camping di bawah bintang. *{n}* bersandar santai menatap langit malam." },
    ActionDef { key: "alam_mancing", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["kuudere", "onee-san"], dislikes: &["genki"],
        text: "🎣 Memancing di danau tenang. *{n}* senang saat akhirnya dapat ikan." },
    ActionDef { key: "alam_perahu", phase: "approach", base: (8.0, 16.0), exp: 35,
        likes: &["genki", "amayadori"], dislikes: &["tsundere"],
        text: "⛵ Menyusuri sungai naik perahu. *{n}* menutup mata menikmati angin." },
    ActionDef { key: "seni_museum", phase: "approach", base: (6.0, 12.0), exp: 30,
        likes: &["kuudere", "dandere"], dislikes: &["genki"],
        text: "🖼️ Menjelajahi museum. *{n}* takjub di depan lukisan tua." },
    ActionDef { key: "seni_melukis", phase: "approach", base: (8.0, 15.0), exp: 35,
        likes: &["deredere", "dandere", "onee-san"], dislikes: &[],
        text: "🎨 Melukis bareng di studio. Karya *{n}* kacau balau tapi lucu." },
    ActionDef { key: "seni_konser", phase: "approach", base: (10.0, 18.0), exp: 40,
        likes: &["genki", "deredere", "amayadori"], dislikes: &["kuudere"],
        text: "🎸 Nonton konser. *{n}* bernyanyi keras tanpa malu." },
];

const ACTIONS_INTIM: &[ActionDef] = &[
    ActionDef { key: "peluk_belakang", phase: "intim", base: (10.0, 18.0), exp: 45,
        likes: &["yandere", "deredere", "tsundere"], dislikes: &[],
        text: "🤗 Kamu memeluk *{n}* dari belakang. Ia terkejut sesaat lalu tersenyum aman." },
    ActionDef { key: "peluk_depan", phase: "intim", base: (12.0, 20.0), exp: 50,
        likes: &["yandere", "deredere", "onee-san"], dislikes: &[],
        text: "💑 Kalian saling berhadapan dan berpelukan erat. Detak jantung berdebar seirama." },
    ActionDef { key: "cium_kening", phase: "intim", base: (10.0, 18.0), exp: 45,
        likes: &["dandere", "tsundere", "yandere"], dislikes: &[],
        text: "😚 Kecupan lembut di kening membuat *{n}* memejamkan mata bahagia." },
    ActionDef { key: "cium_bibir", phase: "intim", base: (14.0, 24.0), exp: 55,
        likes: &["yandere", "deredere"], dislikes: &["kuudere"],
        text: "💋 Ciuman lembut dan panjang. *{n}* membalas dengan desahan kecil." },
    ActionDef { key: "tidur_kelon", phase: "intim", base: (10.0, 16.0), exp: 45,
        likes: &["amayadori", "onee-san"], dislikes: &[],
        text: "🛏️ *{n}* merebahkan kepalanya di dadamu dan tertidur pulas. Tenang sekali." },
    ActionDef { key: "tidur_serandu", phase: "intim", base: (12.0, 20.0), exp: 50,
        likes: &["onee-san", "amayadori", "yandere"], dislikes: &[],
        text: "🌙 Kalian berpelukan erat di ranjang. *{n}* berbisik bahwa ia merasa aman." },
    ActionDef { key: "mandi_punggung", phase: "intim", base: (10.0, 16.0), exp: 45,
        likes: &["onee-san", "kuudere"], dislikes: &[],
        text: "🛁 Kamu menggosok punggung *{n}* dengan lembut. Ia mendesah rileks." },
    ActionDef { key: "mandi_bahu", phase: "intim", base: (12.0, 20.0), exp: 50,
        likes: &["deredere", "onee-san"], dislikes: &[],
        text: "🧼 *{n}* menunduk malu saat kamu mengusap bahunya di air hangat." },
    ActionDef { key: "gendong_putri", phase: "intim", base: (10.0, 18.0), exp: 45,
        likes: &["deredere", "yandere", "tsundere"], dislikes: &[],
        text: "👸 Kamu menggendong *{n}* dengan gaya putri. Ia memeluk lehermu erat." },
    ActionDef { key: "gendong_punggung", phase: "intim", base: (8.0, 14.0), exp: 40,
        likes: &["genki", "amayadori"], dislikes: &[],
        text: "🏃 *{n}* menaiki punggungmu tertawa-tawa sepanjang jalan pulang." },
    ActionDef { key: "tepuk_kepala", phase: "intim", base: (6.0, 12.0), exp: 35,
        likes: &["dandere", "tsundere"], dislikes: &[],
        text: "🖐️ Kamu menepuk kepala *{n}* pelan. Ia mendengus tapi pipinya memerah." },
    ActionDef { key: "pijat_bahu", phase: "intim", base: (10.0, 18.0), exp: 45,
        likes: &["onee-san", "yandere", "deredere"], dislikes: &[],
        text: "💆 Kamu memijat bahu *{n}* yang tegang. Ia mendesah rileks." },
];

const ACTIONS_MARRIED: &[ActionDef] = &[
    ActionDef { key: "mesra", phase: "married", base: (6.0, 12.0), exp: 35,
        likes: &["deredere", "onee-san"], dislikes: &[],
        text: "👩‍❤️‍👨 Kalian menghabiskan malam dengan mesra sebagai pasangan suami istri." },
    ActionDef { key: "rayu", phase: "married", base: (8.0, 14.0), exp: 40,
        likes: &["tsundere", "deredere"], dislikes: &[],
        text: "💌 Kata-kata rayuanmu membuat *{n}* tersipu dan memelukmu." },
    ActionDef { key: "hadiah", phase: "married", base: (10.0, 16.0), exp: 45,
        likes: &["himedere", "ojou-sama"], dislikes: &[],
        text: "🎁 Hadiah kejutan membuat *{n}* melompat bahagia seperti anak kecil." },
    ActionDef { key: "bulanmadu_pantai", phase: "married", base: (12.0, 20.0), exp: 50,
        likes: &["genki", "amayadori"], dislikes: &[],
        text: "🌴 Bulan madu di pantai tropis. *{n}* menggandengmu menikmati sunset." },
    ActionDef { key: "bulanmadu_hotel", phase: "married", base: (14.0, 24.0), exp: 55,
        likes: &["onee-san", "yandere"], dislikes: &[],
        text: "🏨 Suite hotel mewah. Malam penuh kehangatan berdua saja." },
    ActionDef { key: "nontonrumah", phase: "married", base: (8.0, 14.0), exp: 40,
        likes: &["deredere", "dandere", "tsundere"], dislikes: &[],
        text: "📺 Nonton maraton film di sofa. *{n}* nyender di bahumu sampai ketiduran." },
    ActionDef { key: "jalanpagi", phase: "married", base: (10.0, 16.0), exp: 45,
        likes: &["genki", "onee-san", "amayadori"], dislikes: &[],
        text: "🌅 Jalan santai pagi hari sambil beli sarapan. *{n}* menggandengmu." },
];

/// `ACTIONS[key]` (`src/lib/waifu.js:126`) — cari lintas tiga fase.
fn find_action(key: &str) -> Option<&'static ActionDef> {
    ACTIONS_APPROACH
        .iter()
        .chain(ACTIONS_INTIM)
        .chain(ACTIONS_MARRIED)
        .find(|a| a.key == key)
}

/// `PERSONALITY_TAG` (`src/lib/waifu.js:77-88`) — `(like, dislike)`.
const PERSONALITY_TAG: [(&str, &[&str], &[&str]); 10] = [
    ("tsundere", &["jalan_taman", "piknik_taman"], &["rayu"]),
    ("deredere", &["peluk_depan", "cium_bibir", "mesra"], &[]),
    ("kuudere", &["kafe_kopi", "bioskop_romantis", "piknik_taman"], &["arcade_duo"]),
    ("dandere", &["kafe_matcha", "masak_bareng"], &["jalan_mall"]),
    ("yandere", &["peluk_belakang", "cium_kening", "cium_bibir"], &["jalan_mall"]),
    ("onee-san", &["masak_bareng", "tidur_kelon"], &[]),
    ("genki", &["karaoke_duet", "arcade_duo", "piknik_pantai"], &["kafe_kopi"]),
    ("himedere", &["belanja_perhiasan", "hadiah"], &["jalan_taman"]),
    ("ojou-sama", &["belanja_perhiasan", "kafe_kue"], &["jalan_pantai"]),
    ("amayadori", &["tidur_kelon", "piknik_taman"], &["karaoke_solo"]),
];

/// Varian `aff` di `EVENTS` (`src/lib/waifu.js:90-111`).
enum EventAff {
    Fixed(f64),
    /// `aff: (p) => (p === "deredere" || p === "genki" ? 9 : 4)` — id `cat`.
    Cat,
}

/// Varian `koin` di `EVENTS`.
enum EventKoin {
    Wallet,
    Lottery,
    Gift,
    Lomba,
    Rezeki,
}

struct EventDef {
    id: &'static str,
    phase: &'static str,
    married_only: bool,
    aff: EventAff,
    yandere_aff: f64,
    koin: Option<EventKoin>,
    mood: Option<&'static str>,
    next_mult: f64,
    anger: f64,
    text: &'static str,
}

const EVENTS: &[EventDef] = &[
    EventDef { id: "rain", phase: "any", married_only: false, aff: EventAff::Fixed(5.0), koin: None,
        yandere_aff: 0.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "🌧️ Hujan deras turun! Kalian berbagi satu payung, bahu menempel bahu. *{n}* tersenyum malu." },
    EventDef { id: "wallet", phase: "any", married_only: false, aff: EventAff::Fixed(0.0),
        yandere_aff: 0.0, koin: Some(EventKoin::Wallet), mood: None, next_mult: 1.0, anger: 0.0,
        text: "💸 Kamu menemukan dompet di jalan dan mengembalikannya. Rezeki mengalir!" },
    EventDef { id: "rival", phase: "any", married_only: false, aff: EventAff::Fixed(-4.0), koin: None,
        yandere_aff: 5.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "😠 Seorang rival mendekati *{n}*! Ia gelisah dan mood-nya turun." },
    EventDef { id: "idol", phase: "any", married_only: false, aff: EventAff::Fixed(6.0), koin: None,
        yandere_aff: 0.0, mood: Some("ceria"), next_mult: 1.0, anger: 0.0,
        text: "🎤 Idola favoritnya lewat di jalanan! *{n}* kegirangan dan mood-nya naik." },
    EventDef { id: "cat", phase: "any", married_only: false, aff: EventAff::Cat, koin: None,
        yandere_aff: 0.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "🐱 Seekor kucing lucu tersangkut di pohon. Kalian menyelamatkannya!" },
    EventDef { id: "lottery", phase: "any", married_only: false, aff: EventAff::Fixed(0.0),
        yandere_aff: 0.0, koin: Some(EventKoin::Lottery), mood: None, next_mult: 1.0, anger: 0.0,
        text: "🎰 Tiket lotre jatuh dari langit! Kamu coba keberuntunganmu..." },
    EventDef { id: "late", phase: "any", married_only: false, aff: EventAff::Fixed(0.0), koin: None,
        yandere_aff: 0.0, mood: Some("sedih"), next_mult: 0.8, anger: 0.0,
        text: "📉 Kamu hampir telat janji temu! *{n}* cemberut sepanjang hari." },
    EventDef { id: "anniv", phase: "any", married_only: true, aff: EventAff::Fixed(6.0), koin: None,
        yandere_aff: 0.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "💍 Kenangan hari pernikahan kalian teringat kembali. Kalian terharu bersama." },
    EventDef { id: "intimate", phase: "intim", married_only: false, aff: EventAff::Fixed(10.0),
        yandere_aff: 0.0, koin: None, mood: None, next_mult: 1.0, anger: 0.0,
        text: "🔥 Momen kalian berlanjut. *{n}* berbisik pelan, \"jangan berhenti...\"" },
    EventDef { id: "rainbow", phase: "approach", married_only: false, aff: EventAff::Fixed(5.0),
        yandere_aff: 0.0, koin: None, mood: None, next_mult: 1.0, anger: 0.0,
        text: "🌈 Pelangi muncul setelah hujan reda. Kalian berhenti untuk mengabadikannya." },
    EventDef { id: "confess", phase: "intim", married_only: false, aff: EventAff::Fixed(8.0),
        yandere_aff: 0.0, koin: None, mood: None, next_mult: 1.0, anger: 0.0,
        text: "💞 *{n}* mengaku bahwa ia mulai benar-benar mencintaimu." },
    EventDef { id: "gift", phase: "married", married_only: true, aff: EventAff::Fixed(0.0),
        yandere_aff: 0.0, koin: Some(EventKoin::Gift), mood: None, next_mult: 1.0, anger: 0.0,
        text: "🎁 Hadiah kejutan kecil untukmu sebagai pasangan." },
    EventDef { id: "foto", phase: "any", married_only: false, aff: EventAff::Fixed(4.0), koin: None,
        yandere_aff: 0.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "📸 Foto bareng! *{n}* melotot saat kejepret momen memalukan." },
    EventDef { id: "lomba", phase: "any", married_only: false, aff: EventAff::Fixed(0.0),
        yandere_aff: 0.0, koin: Some(EventKoin::Lomba), mood: None, next_mult: 1.0, anger: 0.0,
        text: "🏅 Kamu ikut lomba jalan santai dan menang hadiah receh!" },
    EventDef { id: "salahpaham", phase: "any", married_only: false, aff: EventAff::Fixed(-5.0),
        yandere_aff: 0.0, koin: None, mood: None, next_mult: 1.0, anger: 12.0,
        text: "📵 *{n}* melihat notif dari akun lain di ponselmu dan salah paham!" },
    EventDef { id: "pujian", phase: "any", married_only: false, aff: EventAff::Fixed(5.0), koin: None,
        yandere_aff: 0.0, mood: None, next_mult: 1.0, anger: 0.0,
        text: "🗣️ Seseorang memuji kalian: \"cocok banget!\" *{n}* tersipu." },
    EventDef { id: "sunset", phase: "any", married_only: false, aff: EventAff::Fixed(6.0), koin: None,
        yandere_aff: 0.0, mood: Some("romantis"), next_mult: 1.0, anger: 0.0,
        text: "🌇 Kalian berhenti menikmati matahari terbenam. Momen tak terlupakan." },
    EventDef { id: "kenangan", phase: "any", married_only: true, aff: EventAff::Fixed(7.0),
        yandere_aff: 0.0, koin: None, mood: None, next_mult: 1.0, anger: 0.0,
        text: "📿 Kamu menemukan foto lama kalian. *{n}* terharu dan memelukmu erat." },
    EventDef { id: "badai", phase: "any", married_only: false, aff: EventAff::Fixed(-4.0), koin: None,
        yandere_aff: 0.0, mood: Some("marah"), next_mult: 1.0, anger: 0.0,
        text: "⛈️ Hujan badai datang! *{n}* kehujanan dan mood-nya rusak." },
    EventDef { id: "rezeki", phase: "any", married_only: false, aff: EventAff::Fixed(0.0),
        yandere_aff: 0.0, koin: Some(EventKoin::Rezeki), mood: None, next_mult: 1.0, anger: 0.0,
        text: "🍀 Rezeki nomplok! Kamu dapat uang tak terduga." },
];

/// `MENUS` (`gachawaifu.js:303-327`) — hanya judul yang tampil; label opsi
/// dibuang karena `Reply` tidak punya tombol `nativeFlowMessage`.
const MENUS: [(&str, &str); 19] = [
    ("menu_jalanjalan", "🚶 Jalan-jalan ke..."),
    ("menu_kafe", "☕ Pesan untuknya..."),
    ("menu_bioskop", "🎬 Pilih genre film..."),
    ("menu_belanja", "🛍️ Belikan dia apa?.."),
    ("menu_karaoke", "🎤 Karaoke..."),
    ("menu_arcade", "🕹️ Di arcade..."),
    ("menu_piknik", "🧺 Piknik..."),
    ("menu_masak", "🍳 Masak bareng..."),
    ("menu_kuliner", "🍽️ Makan di..."),
    ("menu_olahraga", "⚽ Olahraga..."),
    ("menu_alam", "🏕️ Petualangan alam..."),
    ("menu_seni", "🎨 Seni & budaya..."),
    ("menu_lainnya", "🎮 Aksi lainnya..."),
    ("menu_peluk", "🫂 Cara memeluk..."),
    ("menu_cium", "💋 Cium di..."),
    ("menu_tidur", "🛏️ Tidur bareng..."),
    ("menu_mandi", "🛁 Mandi bareng..."),
    ("menu_gendong", "🤗 Gendong..."),
    ("menu_bulanmadu", "🌴 Bulan madu..."),
];

/// Daftar pemicu tamparan (`gachawaifu.js:334`).
const MESUM_ACTIONS: [&str; 15] = [
    "peluk",
    "cium",
    "tidur",
    "mandi",
    "gendong_putri",
    "gendong_punggung",
    "tepuk_kepala",
    "intim_belakang",
    "intim_depan",
    "intim_kening",
    "intim_bibir",
    "intim_kelon",
    "intim_panas",
    "intim_punggung",
    "intim_bahu",
];

/// `VALID_PERSONALITIES` (`data/waifu/index.js:46-49`).
const VALID_PERSONALITIES: [&str; 10] = [
    "tsundere",
    "deredere",
    "kuudere",
    "dandere",
    "yandere",
    "onee-san",
    "genki",
    "himedere",
    "ojou-sama",
    "amayadori",
];

/// Satu entri mentah pool waifu — bentuk `data/waifu/*.js` sebelum validasi.
pub(crate) struct RawWaifu {
    pub name: &'static str,
    pub series: &'static str,
    pub tier: &'static str,
    pub personality: &'static str,
    pub keyword: Option<&'static str>,
    pub age: i64,
    pub height: i64,
    pub weight: i64,
}

/// Hasil `buildPool` (`data/waifu/index.js:61-80`).
pub(crate) struct Waifu {
    pub name: String,
    pub series: String,
    pub tier: String,
    pub personality: String,
    pub keyword: String,
    pub age: i64,
    pub height: i64,
    pub weight: i64,
    pub roll_weight: f64,
}

/// `ponytail: data/waifu/*.js` belum ter-port (brief melarang menyentuh
/// berkas di luar plugin), jadi pool kosong dan cabang gacha selalu jatuh ke
/// `te()` + ☢. Isi array ini kalau loader pool sudah ada di Rust.
const WAIFU_ENTRIES: &[RawWaifu] = &[];

/// `buildPool` — validasi, dedupe nama, trim, keyword default, bobot tier.
pub(crate) fn build_pool(raw: &[RawWaifu]) -> Vec<Waifu> {
    let mut seen: Vec<String> = Vec::new();
    let mut all: Vec<Waifu> = Vec::new();
    for w in raw {
        if w.name.is_empty()
            || w.series.is_empty()
            || w.age == 0
            || w.height == 0
            || w.weight == 0
            || w.tier.is_empty()
            || w.personality.is_empty()
        {
            continue;
        }
        if !TIER_WEIGHTS.iter().any(|(t, _)| *t == w.tier) {
            continue;
        }
        if !VALID_PERSONALITIES.contains(&w.personality) {
            continue;
        }
        let name = w.name.trim().to_string();
        let key = name.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let series = w.series.trim().to_string();
        let keyword = w
            .keyword
            .filter(|k| !k.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{name} {series} anime"));
        all.push(Waifu {
            name,
            series,
            tier: w.tier.to_string(),
            personality: w.personality.to_string(),
            keyword,
            age: w.age,
            height: w.height,
            weight: w.weight,
            roll_weight: 0.0,
        });
    }
    let mut tier_count: Vec<(String, f64)> = Vec::new();
    for w in &all {
        match tier_count.iter_mut().find(|(t, _)| *t == w.tier) {
            Some((_, n)) => *n += 1.0,
            None => tier_count.push((w.tier.clone(), 1.0)),
        }
    }
    for w in &mut all {
        let weight = TIER_WEIGHTS
            .iter()
            .find(|(t, _)| *t == w.tier)
            .map_or(0.0, |(_, w0)| *w0);
        let count = tier_count
            .iter()
            .find(|(t, _)| *t == w.tier)
            .map_or(1.0, |(_, c)| *c);
        w.roll_weight = weight / count;
    }
    all
}

/// `rollWaifu` (`src/lib/waifu.js:113-123`). `None` = pool kosong, padanan
/// `undefined` yang bikin `picked.keyword` melempar di JS.
pub(crate) fn roll_waifu<'a>(
    pool: &'a [Waifu],
    pity_counter: i64,
    rng: &mut dyn FnMut() -> f64,
) -> Option<&'a Waifu> {
    let eligible: Vec<&Waifu> = if pity_counter >= PITY_THRESHOLD {
        pool.iter().filter(|w| tier_index(&w.tier) >= 2).collect()
    } else {
        pool.iter().collect()
    };
    let total: f64 = eligible.iter().map(|w| w.roll_weight).sum();
    let mut r = rng() * total;
    for w in &eligible {
        r -= w.roll_weight;
        if r <= 0.0 {
            return Some(w);
        }
    }
    eligible.last().copied()
}

/// `TIER_ORDER.indexOf(tier)` — `-1` untuk tier tak dikenal, seperti JS.
fn tier_index(tier: &str) -> i64 {
    TIER_ORDER
        .iter()
        .position(|t| *t == tier)
        .map_or(-1, |i| i as i64)
}

/// `Number` JSON: integral disimpan integer (`JSON.stringify` menulis `50`,
/// bukan `50.0`).
fn jnum(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.0e15 {
        Value::from(f as i64)
    } else {
        Value::from(f)
    }
}

/// `${n}` JS: integral tanpa desimal, selain itu apa adanya.
fn fmt_num(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.0e15 {
        (f as i64).to_string()
    } else {
        format!("{f}")
    }
}

/// `Math.round` JS: setengah selalu ke ATAS (`-5.5` → `-5`), beda dari
/// `f64::round` Rust yang menjauh dari nol (`-5.5` → `-6`).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn mnum(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

fn mnum_or(v: Option<&Value>, d: f64) -> f64 {
    mnum(v).unwrap_or(d)
}

fn mstr(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

/// Objek JSON dari `Value`, kosong kalau bukan object (JS `{}` saat field
/// hilang).
fn map_of(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

/// `new Date(ms).toISOString()` — UTC dengan `Z`, bukan waktu Jakarta.
fn iso_now(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = jakarta_parts(ms - JAKARTA_OFFSET_SECS * 1000);
    format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{:03}Z",
        ms.rem_euclid(1000)
    )
}

/// `new Date(iso).getTime()` untuk bentuk yang ditulis [`iso_now`]. `None`
/// kalau tidak bisa diparse (JS: `NaN`, semua perbandingan `false`).
fn iso_to_ms(iso: &str) -> Option<i64> {
    let b = iso.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |a: usize, c: usize| iso.get(a..c)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let millis = if b.len() >= 24 && b[19] == b'.' { num(20, 23)? } else { 0 };
    let days = crate::common::days_from_civil(y, mo as u32, d as u32);
    Some(days * 86_400_000 + (h * 3600 + mi * 60 + s) * 1000 + millis)
}

/// `todayStr()` (`gachawaifu.js:56-58`) — `toLocaleDateString("id-ID")` di
/// Asia/Jakarta, bentuk `DD/MM/YYYY` = [`date_dmy`].
fn today_str(now: i64) -> String {
    date_dmy(now)
}

/// `marrageDay` (`gachawaifu.js:157-162`).
///
/// `ponytail:` tanggal yang tidak bisa diparse di JS jadi `NaN` dan hasilnya
/// `NaN` juga; di sini jatuh ke `0`.
fn marrage_day(married_date: Option<&str>, now: i64) -> i64 {
    let Some(date) = married_date else { return 0 };
    let Some(a) = iso_to_ms(date) else { return 0 };
    (((now - a) as f64) / 86_400_000.0).floor() as i64 + 1
}

/// `'waifu_' + name.replace(/\s+/g, '') + '@s.whatsapp.net'`.
fn waifu_jid(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    format!("waifu_{name}@s.whatsapp.net")
}

/// Satu aksi yang sudah dihitung (`applyAction`, `src/lib/waifu.js:125-139`).
pub(crate) struct ActionResult {
    pub phase: &'static str,
    pub change: i64,
    pub exp: i64,
    pub text: String,
    pub like: bool,
    pub dislike: bool,
}

/// `finalGain` (`src/lib/romance.js:88-101`).
pub(crate) struct Gain {
    pub change: i64,
    pub drain: i64,
    pub extra: Vec<String>,
    pub angry: bool,
}

/// Hasil `rollEvent` (`src/lib/waifu.js:141-153`).
pub(crate) struct EventRoll {
    pub text: String,
    pub aff: f64,
    pub koin: i64,
    pub mood: Option<&'static str>,
    pub next_mult: f64,
    pub anger: f64,
}

/// `diminish` (`src/lib/romance.js:43-46`).
pub(crate) fn diminish(change: f64, current: f64) -> i64 {
    // `(current || 50)`: affection `0` di JS jatuh ke 50, bukan 1.05.
    let current = if current == 0.0 { 50.0 } else { current };
    let f = (1.05 - current / 100.0).max(0.15);
    (js_round(change * f) as i64).max(1)
}

/// `fatigueMult` (`src/lib/romance.js:49-53`).
pub(crate) fn fatigue_mult(actions_today: i64) -> f64 {
    if actions_today <= 3 {
        return 1.0;
    }
    (1.0 - (actions_today - 3) as f64 * 0.12).max(0.25)
}

/// `neglectDecay` (`src/lib/romance.js:56-62`) → `(decay, hours)`.
pub(crate) fn neglect_decay(last_interaction: Option<i64>, now: i64) -> (i64, f64) {
    // `if (!last)` — `0` ikut dianggap belum pernah interaksi, seperti JS.
    let Some(last) = last_interaction.filter(|v| *v != 0) else {
        return (0, 0.0);
    };
    let hours = (now - last) as f64 / 3_600_000.0;
    if hours < 24.0 {
        return (0, hours);
    }
    (((hours / 24.0).floor() as i64 * 3).min(20), hours)
}

/// `applyNeglect` (`src/lib/romance.js:77-85`) — MUTASI `waifu`, kembalikan
/// `decay`. Dipanggil walau aksi nanti tidak dikenal, persis JS.
pub(crate) fn apply_neglect(waifu: &mut Map<String, Value>, now: i64) -> i64 {
    let last = mnum(waifu.get("lastInteractionAt")).map(|f| f as i64);
    let (decay, _) = neglect_decay(last, now);
    if decay > 0 {
        let affection = mnum(waifu.get("affection")).unwrap_or(50.0);
        let anger = mnum(waifu.get("anger")).unwrap_or(0.0);
        waifu.insert("affection".into(), jnum((affection - decay as f64).max(0.0)));
        waifu.insert("anger".into(), jnum((anger - 10.0).max(0.0)));
    }
    waifu.insert("lastInteractionAt".into(), jnum(now as f64));
    decay
}

/// `finalGain` — `Math.round(change * fm)` setelah diminish, lalu drain anger.
pub(crate) fn final_gain(
    raw_change: i64,
    affection: f64,
    actions_today: i64,
    anger: f64,
) -> Gain {
    let angry = anger >= ANGER_THRESHOLD;
    let mut extra = Vec::new();
    let mut change = diminish(raw_change as f64, affection);
    let fm = fatigue_mult(actions_today);
    if fm < 1.0 {
        extra.push(format!(
            "😮‍💨 Dia mulai lelah karena interaksi terus-menerus... (×{fm:.2})"
        ));
    }
    change = js_round(change as f64 * fm) as i64;
    let mut drain = 0;
    if angry {
        drain = (anger / 25.0).floor() as i64;
        change -= drain;
    }
    Gain {
        change: change.max(-50),
        drain,
        extra,
        angry,
    }
}

/// `angerUpdate` (`src/lib/romance.js:69-74`): dislike +12, like -8, clamp.
pub(crate) fn anger_update(anger: f64, like: bool, dislike: bool) -> f64 {
    let mut a = anger;
    if dislike {
        a += 12.0;
    } else if like {
        a -= 8.0;
    }
    a.clamp(0.0, 100.0)
}

/// `angerEffMood` (`src/lib/romance.js:65-67`).
pub(crate) fn anger_eff_mood(mood: &str, anger: f64) -> String {
    if anger >= ANGER_THRESHOLD {
        "marah".to_string()
    } else {
        mood.to_string()
    }
}

/// `c.charCodeAt(0)`: unit UTF-16 pertama; emoji astral menyumbang high
/// surrogate-nya saja.
fn unit_utf16(c: char) -> u32 {
    let u = c as u32;
    if u > 0xFFFF {
        (u - 0x1_0000) / 0x400 + 0xD800
    } else {
        u
    }
}

/// `getDailyMood` (`src/lib/romance.js:30-40`).
pub(crate) fn get_daily_mood(sender_jid: &str, date_str: &str) -> &'static str {
    let seed = format!("{date_str}_{sender_jid}");
    let mut h: u32 = 0;
    for c in seed.chars() {
        let unit = unit_utf16(c);
        h = h.wrapping_mul(31).wrapping_add(unit);
    }
    let r = (h % 10000) as f64 / 10000.0;
    if r < 0.3 {
        "ceria"
    } else if r < 0.55 {
        "romantis"
    } else if r < 0.8 {
        "biasa"
    } else if r < 0.92 {
        "sedih"
    } else {
        "marah"
    }
}

/// `moodState` (`gachawaifu.js:64-68`).
pub(crate) fn mood_state(waifu: &Map<String, Value>, sender: &str, now: i64) -> String {
    if let Some(until) = mstr(waifu.get("moodUntil"))
        && iso_to_ms(&until).is_some_and(|t| now < t)
        && let Some(kind) = waifu
            .get("mood")
            .and_then(|m| m.get("type"))
            .and_then(Value::as_str)
    {
        return kind.to_string();
    }
    get_daily_mood(sender, &today_str(now)).to_string()
}

/// `moodLabel` (`gachawaifu.js:60-62`).
fn mood_label(mood: &str) -> String {
    match mood {
        "ceria" => "ceria 😄".to_string(),
        "romantis" => "romantis 💘".to_string(),
        "biasa" => "biasa 🙂".to_string(),
        "sedih" => "sedih 😢".to_string(),
        "marah" => "marah 😡".to_string(),
        other => other.to_string(),
    }
}

/// `angerMeter` (`gachawaifu.js:70-74`).
pub(crate) fn anger_meter(waifu: &Map<String, Value>) -> String {
    let a = mnum(waifu.get("anger")).unwrap_or(0.0);
    // `ponytail:` `repeat` JS melempar untuk n negatif (anger negatif yang
    // tersimpan rusak); di sini n dijepit 0..=10.
    let n = (js_round(a / 10.0) as i64).clamp(0, 10) as usize;
    format!(
        "{}{} ({}/100)",
        "█".repeat(n),
        "░".repeat(10 - n),
        fmt_num(a)
    )
}

/// Personality darurat saat `waifu.personality` kosong (`gachawaifu.js:294-297`).
pub(crate) fn backfill_personality(name: &str) -> &'static str {
    let mut h: u32 = 0;
    for c in name.chars() {
        let unit = unit_utf16(c);
        h = h.wrapping_mul(31).wrapping_add(unit);
    }
    ["tsundere", "deredere", "kuudere", "dandere", "genki"][(h % 5) as usize]
}

/// `applyAction` (`src/lib/waifu.js:125-139`).
pub(crate) fn apply_action(
    key: &str,
    waifu: &Map<String, Value>,
    mood: &str,
    mult_override: f64,
    rng: &mut dyn FnMut() -> f64,
) -> Option<ActionResult> {
    let action = find_action(key)?;
    // `!waifu.personality` — string kosong juga gugur, seperti JS.
    if !js_truthy(waifu.get("personality")) {
        return None;
    }
    let personality = mstr(waifu.get("personality")).unwrap_or_default();
    let name = mstr(waifu.get("name")).unwrap_or_default();

    let (min, max) = action.base;
    let base = min + rng() * (max - min);
    let mut mult = MOOD_MULT
        .iter()
        .find(|(m, _)| *m == mood)
        .map_or(1.0, |(_, v)| *v);
    mult *= mult_override;

    let tag = PERSONALITY_TAG.iter().find(|(p, _, _)| *p == personality);
    let (tag_like, tag_dislike) = tag.map_or((&[][..], &[][..]), |(_, l, d)| (&l[..], &d[..]));
    let liked = action.likes.contains(&personality.as_str()) || tag_like.contains(&key);
    let disliked =
        action.dislikes.contains(&personality.as_str()) || tag_dislike.contains(&key);
    if liked {
        mult *= 1.2;
    } else if disliked {
        mult *= 0.7;
    }

    Some(ActionResult {
        phase: action.phase,
        change: (js_round(base * mult) as i64).max(1),
        exp: action.exp,
        text: action.text.replace("{n}", &name),
        like: liked,
        dislike: disliked,
    })
}

/// `rollEvent` (`src/lib/waifu.js:141-153`).
pub(crate) fn roll_event(
    married: bool,
    phase: &str,
    personality: &str,
    name: &str,
    rng: &mut dyn FnMut() -> f64,
) -> Option<EventRoll> {
    if rng() > EVENT_CHANCE {
        return None;
    }
    let pool: Vec<&EventDef> = EVENTS
        .iter()
        .filter(|e| {
            if e.married_only && !married {
                return false;
            }
            e.phase == "any" || e.phase == phase
        })
        .collect();
    if pool.is_empty() {
        return None;
    }
    let e = pool[(rng() * pool.len() as f64).floor() as usize % pool.len()];
    let mut aff = match e.aff {
        EventAff::Fixed(v) => v,
        EventAff::Cat => {
            if personality == "deredere" || personality == "genki" {
                9.0
            } else {
                4.0
            }
        }
    };
    // `if (e.id === "rival" && personality === "yandere") aff = e.yandereAff;`
    if e.id == "rival" && personality == "yandere" {
        aff = e.yandere_aff;
    }
    let koin = match e.koin {
        None => 0,
        Some(EventKoin::Wallet) => 1000 + (rng() * 24001.0).floor() as i64,
        Some(EventKoin::Lottery) => {
            if rng() < 0.5 {
                20000 + (rng() * 40001.0).floor() as i64
            } else {
                0
            }
        }
        Some(EventKoin::Gift) => 2000 + (rng() * 9001.0).floor() as i64,
        Some(EventKoin::Lomba) => 500 + (rng() * 5001.0).floor() as i64,
        Some(EventKoin::Rezeki) => 3000 + (rng() * 8001.0).floor() as i64,
    };
    Some(EventRoll {
        text: e.text.replace("{n}", name),
        aff,
        koin,
        mood: e.mood,
        next_mult: e.next_mult,
        anger: e.anger,
    })
}

/// `initStats` (`gachawaifu.js:164-167`): `{ totalGacha: 0, byTier: {},
/// pityCounter: 0, rarest: null, marriedCount: 0 }`.
pub(crate) fn default_stats() -> Map<String, Value> {
    let mut stats = Map::new();
    stats.insert("totalGacha".into(), jnum(0.0));
    stats.insert("byTier".into(), Value::Object(Map::new()));
    stats.insert("pityCounter".into(), jnum(0.0));
    stats.insert("rarest".into(), Value::Null);
    stats.insert("marriedCount".into(), jnum(0.0));
    stats
}

/// `initStats(user)` — buat `user.waifuStats` kalau belum ada, kembalikan
/// salinannya (pemanggil menulis balik lewat patch `SetUser`).
pub(crate) fn init_stats(user: &mut Map<String, Value>) -> Map<String, Value> {
    if !js_truthy(user.get("waifuStats")) {
        user.insert("waifuStats".into(), Value::Object(default_stats()));
    }
    map_of(user.get("waifuStats"))
}

/// `recordPull` (`gachawaifu.js:169-180`) — MUTASI `stats` + `user.waifuHistory`.
pub(crate) fn record_pull(
    stats: &mut Map<String, Value>,
    user: &mut Map<String, Value>,
    name: &str,
    series: &str,
    tier: &str,
    at: &str,
) {
    let total = mnum(stats.get("totalGacha")).unwrap_or(0.0) + 1.0;
    stats.insert("totalGacha".into(), jnum(total));

    let mut by_tier = map_of(stats.get("byTier"));
    let count = mnum(by_tier.get(tier)).unwrap_or(0.0) + 1.0;
    by_tier.insert(tier.to_string(), jnum(count));
    stats.insert("byTier".into(), Value::Object(by_tier));

    let is_epic_plus = matches!(tier, "Epic" | "Legendary" | "Mythic");
    let pity = if is_epic_plus {
        0.0
    } else {
        mnum(stats.get("pityCounter")).unwrap_or(0.0) + 1.0
    };
    stats.insert("pityCounter".into(), jnum(pity));

    let rarest_better = match stats.get("rarest").filter(|v| !v.is_null()) {
        None => true,
        Some(r) => {
            let old = mstr(r.get("tier")).unwrap_or_default();
            tier_index(tier) > tier_index(&old)
        }
    };
    if rarest_better {
        stats.insert(
            "rarest".into(),
            serde_json::json!({ "name": name, "tier": tier }),
        );
    }

    let mut history = user
        .get("waifuHistory")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    history.push(serde_json::json!({ "name": name, "series": series, "tier": tier, "at": at }));
    if history.len() > 100 {
        let keep = history.len() - 100;
        history.drain(..keep);
    }
    user.insert("waifuHistory".into(), Value::Array(history));
}

/// `DOWRY[tier]` (`src/lib/romance.js:12-18`).
pub(crate) fn dowry_of(tier: &str) -> Option<&'static Dowry> {
    DOWRY.iter().find(|(t, _)| *t == tier).map(|(_, d)| d)
}

/// `{ Common: 1, Rare: 1.2, Epic: 1.5, Legendary: 2, Mythic: 3 }[tier] || 1`
/// (`gachawaifu.js:376`).
pub(crate) fn tier_mult(tier: &str) -> f64 {
    match tier {
        "Common" => 1.0,
        "Rare" => 1.2,
        "Epic" => 1.5,
        "Legendary" => 2.0,
        "Mythic" => 3.0,
        _ => 1.0,
    }
}

/// `db.updateEnergi` (`database.js:627-644`) — `None` = tidak menulis.
/// `unlimited` = `user.energi === -1` ATAU owner (`config.energi.owner`,
/// default `-1`). Premium tidak masuk: default `config.energi.premium`
/// `99999999`, bukan `-1` (lihat `buyenergi.rs`).
pub(crate) fn energi_gain(user: &Map<String, Value>, amount: i64, unlimited: bool) -> Option<i64> {
    if mnum(user.get("energi")) == Some(-1.0) || unlimited {
        return None;
    }
    Some((mnum_or(user.get("energi"), 0.0) as i64 + amount).max(0))
}

/// `db.updateKoin` (`database.js:646-654`) — `None` = koin `-1` (tak terbatas).
pub(crate) fn koin_gain(user: &Map<String, Value>, amount: i64) -> Option<i64> {
    if mnum(user.get("koin")) == Some(-1.0) {
        return None;
    }
    Some((mnum_or(user.get("koin"), 0.0) as i64 + amount).clamp(0, MAX_KOIN))
}

/// `user.exp || 0` (`level.js:222`) — `-1` tetap `-1` (truthy di JS).
fn old_exp(user: &Map<String, Value>) -> i64 {
    mnum(user.get("exp")).unwrap_or(0.0) as i64
}

/// `db.setUser(jid, patch)`.
fn set_user(jid: &str, patch: Map<String, Value>) -> Mutation {
    Mutation::SetUser {
        jid: jid.to_string(),
        data: Value::Object(patch),
    }
}

/// Balasan teks + opsional satu patch `SetUser` + opsional reaksi.
fn balas(jid: &str, text: String, patch: Option<Map<String, Value>>, reaction: Option<&str>) -> Reply {
    let reply = match patch {
        Some(p) => Reply::text(text).with_mutation(set_user(jid, p)),
        None => Reply::text(text),
    };
    match reaction {
        Some(emoji) => reply.with_reaction(emoji),
        None => reply,
    }
}

/// `${v}` JS: number integral tanpa `.0`, string apa adanya, `undefined`
/// untuk field hilang — bukan string kosong.
fn js_str(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "undefined".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => fmt_num(n.as_f64().unwrap_or(f64::NAN)),
        Some(other) => other.to_string(),
    }
}

/// `handler` (`gachawaifu.js:182-474`) — urutan cabang persis JS.
fn dispatch(ctx: &Ctx, rng: &mut dyn FnMut() -> f64) -> Reply {
    let Some(root) = ctx.user_data.as_object() else {
        // `db.getUser(m.sender)` mengembalikan null → `if (!user) return`.
        return Reply::None;
    };
    let user = root.clone();
    match ctx.command.as_str() {
        "waifuku" | "istriku" => status_waifu(ctx, &user),
        "tinggalinwaifu" => tinggalin_waifu(ctx, &user),
        "gachawaifu" | "gachaistri" => gacha_waifu(ctx, &user, rng),
        "waifuaction" => waifu_action(ctx, &user, rng),
        _ => Reply::None,
    }
}

/// Cabang `waifuku`/`istriku` (`gachawaifu.js:189-215`).
fn status_waifu(ctx: &Ctx, user: &Map<String, Value>) -> Reply {
    let waifu_value = user.get("waifu");
    if !js_truthy(waifu_value) {
        return Reply::text(format!(
            "⚠️ *Kamu belum memiliki waifu!*\nSilakan ketik *{}gachawaifu* untuk memulainya!",
            ctx.prefix
        ));
    }
    let mut w = map_of(waifu_value);
    let now = now_ms();
    let mood = mood_state(&w, &ctx.sender, now);
    // `applyNeglect` MUTASI `w` — ikut tertulis lewat patch di bawah, persis
    // `db.setUser(m.sender, user)` setelahnya.
    let decay = apply_neglect(&mut w, now);
    let married = js_truthy(w.get("married"));
    let day = marrage_day(mstr(w.get("marriedDate")).as_deref(), now);
    let mood_line = if married {
        let badge = if day >= 100 {
            " 🏆"
        } else if day >= 30 {
            " 🎖️"
        } else if day >= 7 {
            " 🎉"
        } else {
            ""
        };
        format!("\n🗓️ *Hari ke-{day}*{badge}")
    } else {
        String::new()
    };

    let mut text = format!(
        "📸 *STATUS WAIFU KAMU* 📸\n\n\
         💖 *Nama:* {}\n\
         💎 *Tier:* {}\n\
         🎭 *Personality:* {}\n\
         🌤️ *Mood hari ini:* {}\n\
         😡 *Anger:* {}\n\
         💞 *Affection:* {}/100\n\
         💍 *Status:* {}{mood_line}\n",
        js_str(w.get("name")),
        js_str(w.get("tier")),
        js_str(w.get("personality")),
        mood_label(&mood),
        anger_meter(&w),
        js_str(w.get("affection")),
        if married {
            "Telah Menikah 💍"
        } else {
            "Pendekatan 💖"
        }
    );
    if decay > 0 {
        text.push_str(&format!(
            "📉 *Neglect:* Affection turun -{decay} karena kamu jarang interaksi!\n"
        ));
    }
    if mnum_or(w.get("anger"), 0.0) >= ANGER_THRESHOLD {
        text.push_str("⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n");
    }
    text.push_str("\nLanjutkan interaksi dengan memilih aksi di bawah!");

    let mut patch = Map::new();
    patch.insert("waifu".into(), Value::Object(w));
    balas(&ctx.sender, text, Some(patch), Some("✅"))
}

/// Cabang `tinggalinwaifu` (`gachawaifu.js:217-229`).
fn tinggalin_waifu(ctx: &Ctx, user: &Map<String, Value>) -> Reply {
    let waifu_value = user.get("waifu");
    if !js_truthy(waifu_value) {
        return Reply::text("⚠️ *Kamu bahkan belum punya waifu!* Halu ya?");
    }
    let w = map_of(waifu_value);
    let name = mstr(w.get("name")).unwrap_or_default();
    let jid = waifu_jid(&name);
    let mut mutations = Vec::new();
    let mut patch = Map::new();
    if js_truthy(w.get("married")) {
        if map_of(user.get("fun")).get("pasangan").and_then(Value::as_str) == Some(jid.as_str()) {
            let mut fun = map_of(user.get("fun"));
            fun.insert("pasangan".into(), Value::from(""));
            patch.insert("fun".into(), Value::Object(fun));
        }
        let mut fun = Map::new();
        fun.insert("pasangan".into(), Value::from(""));
        let mut data = Map::new();
        data.insert("fun".into(), Value::Object(fun));
        mutations.push(set_user(&jid, data));
    }
    // `delete user.waifu` → `null` (SetUser merge, bukan delete).
    patch.insert("waifu".into(), Value::Null);
    mutations.push(set_user(&ctx.sender, patch));

    let text = format!(
        "💔 *KAMU MEMUTUSKAN UNTUK MENINGGALKAN {}!*\n\n\
         Kamu mengemas barang-barangmu dan pergi. Dia menangis tersedu-sedu. Kalian kini resmi berpisah.",
        name.to_uppercase()
    );
    Reply::text(text).mutating(mutations).with_reaction("💔")
}

/// Panduan `panduan` (`gachawaifu.js:238-246`) — kartu viewOnce-nya tidak
/// ikut, hanya `body`.
fn guide(prefix: &str) -> String {
    format!(
        "💕 *SISTEM GACHA WAIFU* 💕\n\n\
         Simulasi kencan virtual interaktif. Dapatkan waifu impianmu, dekati hatinya, dan nikahi dia!\n\n\
         *PENGGUNAAN:*\n\
         • *{prefix}gachawaifu* — Panduan ini\n\
         • *{prefix}waifuku* — Panel interaksi\n\
         • *{prefix}waifualbum* — Riwayat & statistik luck\n\
         • *{prefix}waifupool* — Jelajahi pool waifu\n\
         • *{prefix}tinggalinwaifu* — Putuskan hubungan\n\n\
         *ALUR:*\n1. Tekan tombol **Mulai Gacha**.\n2. 3 Fase: Pendekatan (<80) → Intim (80–99) → Menikah (100).\n3. Mood & personality memengaruhi poin affection.\n4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+.\n5. Aksi memberi EXP + reward ekonomi."
    )
}

/// Cabang `gachawaifu`/`gachaistri` (`gachawaifu.js:231-287`).
fn gacha_waifu(ctx: &Ctx, user: &Map<String, Value>, rng: &mut dyn FnMut() -> f64) -> Reply {
    if js_truthy(user.get("waifu")) {
        let w = map_of(user.get("waifu"));
        return balas(
            &ctx.sender,
            format!(
                "⚠️ *Kamu sudah memiliki waifu!*\n\
                 Nama: *{}*\nTier: *{}*\nAffection: *{}/100*\n\n\
                 Jangan serakah! Ketik *{}waifuku* untuk berinteraksi dengannya.",
                js_str(w.get("name")),
                js_str(w.get("tier")),
                js_str(w.get("affection")),
                ctx.prefix
            ),
            None,
            Some("😡"),
        );
    }
    let sub = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    if sub != "start" {
        return Reply::text(guide(&ctx.prefix));
    }

    let mut user = user.clone();
    let mut stats = init_stats(&mut user);
    let pool = build_pool(WAIFU_ENTRIES);
    let Some(picked) = roll_waifu(&pool, mnum_or(stats.get("pityCounter"), 0.0) as i64, rng) else {
        // `picked.keyword` melempar di JS → cabang catch: `te()` + ☢.
        // `ponytail:` `initStats` sudah menulis `user.waifuStats` di memori JS
        // yang ikut ter-flush nanti; di sini tidak ada mutasi.
        return Reply::text(error_template(ctx)).with_reaction("☢");
    };

    let now = now_ms();
    let mut w = Map::new();
    w.insert("name".into(), Value::from(picked.name.clone()));
    w.insert("series".into(), Value::from(picked.series.clone()));
    w.insert("tier".into(), Value::from(picked.tier.clone()));
    w.insert("personality".into(), Value::from(picked.personality.clone()));
    w.insert("keyword".into(), Value::from(picked.keyword.clone()));
    w.insert("age".into(), jnum(picked.age as f64));
    w.insert("height".into(), jnum(picked.height as f64));
    w.insert("weight".into(), jnum(picked.weight as f64));
    w.insert("rollWeight".into(), Value::from(picked.roll_weight));
    w.insert("affection".into(), jnum(50.0));
    w.insert("married".into(), Value::from(false));
    // `getWaifuImage` (Pinterest) tidak dipanggil: tanpa HTTP, `imageUrl`
    // langsung pakai konstanta fallback persis `gachawaifu.js:40`.
    w.insert("imageUrl".into(), Value::from(IMAGE_FALLBACK));

    record_pull(
        &mut stats,
        &mut user,
        &picked.name,
        &picked.series,
        &picked.tier,
        &iso_now(now),
    );
    user.insert("waifu".into(), Value::Object(w.clone()));
    user.insert("waifuStats".into(), Value::Object(stats.clone()));

    let mut patch = Map::new();
    patch.insert("waifu".into(), Value::Object(w.clone()));
    patch.insert("waifuStats".into(), Value::Object(stats));
    if let Some(history) = user.get("waifuHistory") {
        patch.insert("waifuHistory".into(), history.clone());
    }

    let text = format!(
        "🎉 *SELAMAT! KAMU MENDAPATKAN WAIFU BARU!* 🎉\n\n\
         💖 *Nama:* {}\n🎂 *Usia:* {} tahun\n📏 *Tinggi:* {}\n⚖️ *Berat:* {}\n\
         💎 *Tier:* {}\n🎭 *Personality:* {}\n💞 *Affection:* {}/100\n\n\
         Pilih interaksi untuk mulai PDKT. Hati-hati jangan sampai affection habis!",
        js_str(w.get("name")),
        js_str(w.get("age")),
        js_str(w.get("height")),
        js_str(w.get("weight")),
        js_str(w.get("tier")),
        js_str(w.get("personality")),
        js_str(w.get("affection")),
    );
    balas(&ctx.sender, text, Some(patch), Some("✅"))
}

/// Patch `SetUser` minimal yang menulis balik objek `waifu`.
fn waifu_patch(w: &Map<String, Value>) -> Map<String, Value> {
    let mut patch = Map::new();
    patch.insert("waifu".into(), Value::Object(w.clone()));
    patch
}

/// Cabang `waifuaction` (`gachawaifu.js:289-473`).
fn waifu_action(ctx: &Ctx, user: &Map<String, Value>, rng: &mut dyn FnMut() -> f64) -> Reply {
    if !js_truthy(user.get("waifu")) {
        return balas(
            &ctx.sender,
            format!(
                "Kamu belum memiliki waifu! Ketik *{}gachawaifu* untuk memulai.",
                ctx.prefix
            ),
            None,
            Some("❌"),
        );
    }
    let action = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let mut w = map_of(user.get("waifu"));
    let now = now_ms();

    // Backfill personality (`gachawaifu.js:294-300`) — setUser langsung di JS;
    // di sini ikut menempel ke balasan mana pun yang membawa patch `waifu`.
    let mut dirty = false;
    if !js_truthy(w.get("personality")) {
        let name = mstr(w.get("name")).unwrap_or_default();
        w.insert(
            "personality".into(),
            Value::from(backfill_personality(&name)),
        );
        dirty = true;
    }
    let dirty_patch = |w: &Map<String, Value>| -> Option<Map<String, Value>> {
        if dirty {
            Some(waifu_patch(w))
        } else {
            None
        }
    };

    if let Some(title) = MENUS.iter().find(|(k, _)| *k == action) {
        // `sendMenu` → `sendWaifuMessage(title, ...)`: hanya `body` yang
        // terkirim; label tombol tidak ada di `Reply`.
        return balas(&ctx.sender, title.1.to_string(), dirty_patch(&w), None);
    }

    if MESUM_ACTIONS.contains(&action.as_str()) && mnum_or(w.get("affection"), 0.0) < 80.0 {
        let drop = (rng() * 30.0).floor() as i64 + 30;
        let affection = mnum_or(w.get("affection"), 50.0);
        w.insert(
            "affection".into(),
            jnum((affection - drop as f64).max(0.0)),
        );
        let text = format!(
            "💢 *PLAKK!!* Kamu mencoba bersikap mesum kepada *{}*, tapi kalian belum sedekat itu! Dia menamparmu keras.\n💞 *Affection -{drop}*",
            js_str(w.get("name"))
        );
        return balas(&ctx.sender, text, Some(waifu_patch(&w)), Some("💢"));
    }

    if action == "nikah" {
        let affection = mnum_or(w.get("affection"), 50.0);
        if affection < 100.0 {
            return balas(
                &ctx.sender,
                "⚠️ Affection belum 100! Jangan terburu-buru melamar!".to_string(),
                dirty_patch(&w),
                None,
            );
        }
        if js_truthy(w.get("married")) {
            return balas(
                &ctx.sender,
                "⚠️ Kalian kan sudah menikah!".to_string(),
                dirty_patch(&w),
                None,
            );
        }
        let name = mstr(w.get("name")).unwrap_or_default();
        let tier = mstr(w.get("tier")).unwrap_or_default();
        w.insert("married".into(), Value::from(true));
        w.insert("marriedDate".into(), Value::from(iso_now(now)));

        let mut patch = Map::new();
        patch.insert("waifu".into(), Value::Object(w.clone()));
        // `ponytail:` tier tak dikenal bikin JS melempar (`DOWRY[tier]`
        // undefined); di sini dowry-nya dilewati. Tier selalu valid dari pool.
        let (limit, koin, exp) = dowry_of(&tier).map_or((0, 0, 0), |d| (d.limit, d.koin, d.exp));
        if let Some(new_energi) = energi_gain(user, limit, ctx.is_owner) {
            patch.insert("energi".into(), jnum(new_energi as f64));
        }
        if let Some(new_koin) = koin_gain(user, koin) {
            patch.insert("koin".into(), jnum(new_koin as f64));
        }
        let mut rpg = rpg_map(ctx);
        let new_exp = apply_exp(&mut rpg, old_exp(user), exp);
        patch.insert("exp".into(), jnum(new_exp as f64));
        patch.insert("rpg".into(), Value::Object(rpg));

        let mut user = user.clone();
        user.insert("waifu".into(), Value::Object(w.clone()));
        let mut stats = init_stats(&mut user);
        let married_count = mnum(stats.get("marriedCount")).unwrap_or(0.0) + 1.0;
        stats.insert("marriedCount".into(), jnum(married_count));
        patch.insert("waifuStats".into(), Value::Object(stats));

        let mut fun = map_of(user.get("fun"));
        let jid = waifu_jid(&name);
        fun.insert("pasangan".into(), Value::from(jid.clone()));
        patch.insert("fun".into(), Value::Object(fun));

        let mut partner = Map::new();
        partner.insert("fun".into(), serde_json::json!({ "pasangan": ctx.sender }));
        partner.insert("name".into(), Value::from(name.clone()));

        let text = format!(
            "💍 *KAMU RESMI MENIKAH DENGAN {}!* 💍\n\n\
             Sebagai dowry:\n- ⚡ {limit} Limit/Energi\n- 💰 {} Koin\n- ✨ {} EXP\n\n\
             Status `.cekpacar` kini resmi berpasangan!",
            name.to_uppercase(),
            thousands_en(koin),
            thousands_en(exp),
        );
        return balas(&ctx.sender, text, Some(patch), Some("💍"))
            .with_mutation(set_user(&jid, partner));
    }

    if action == "hadiah" {
        let affection = mnum_or(w.get("affection"), 50.0);
        if affection < 100.0 {
            return balas(
                &ctx.sender,
                "⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!".to_string(),
                dirty_patch(&w),
                None,
            );
        }
        let g = 300 + (rng() * 401.0).floor() as i64;
        let mut patch = dirty_patch(&w);
        if let Some(new_energi) = energi_gain(user, g, ctx.is_owner) {
            patch.get_or_insert_with(Map::new)
                .insert("energi".into(), jnum(new_energi as f64));
        }
        let text = format!(
            "💝 *{}* memberimu hadiah!\nKamu mendapat ⚡ {g} Limit/Energi!",
            js_str(w.get("name"))
        );
        balas(&ctx.sender, text, patch, Some("💝"))
    } else if action == "klaim" {
        if !js_truthy(w.get("married")) {
            return balas(
                &ctx.sender,
                "⚠️ Hanya istri sah yang bisa klaim hadiah harian!".to_string(),
                dirty_patch(&w),
                None,
            );
        }
        let today = today_str(now);
        if mstr(w.get("lastClaimDate")).as_deref() == Some(today.as_str()) {
            return balas(
                &ctx.sender,
                "⚠️ Kamu sudah klaim hari ini! Coba lagi besok.".to_string(),
                dirty_patch(&w),
                None,
            );
        }
        let tier = mstr(w.get("tier")).unwrap_or_default();
        let mult = tier_mult(&tier);
        let koin = ((5000.0 + rng() * 15001.0) * mult).floor() as i64;
        let exp = ((200.0 + rng() * 601.0) * mult).floor() as i64;
        let day = marrage_day(mstr(w.get("marriedDate")).as_deref(), now);

        let mut cur = user.clone();
        let mut new_koin = None;
        let mut bonus = String::new();
        if day == 7 || day == 30 || day == 100 {
            let b_koin = (day as f64 * 1000.0 * mult).floor() as i64;
            if let Some(v) = koin_gain(&cur, b_koin) {
                cur.insert("koin".into(), jnum(v as f64));
                new_koin = Some(v);
            }
            bonus = format!(
                "\n🎉 *MILESTONE HARI KE-{day}!* Bonus +{} Koin!",
                thousands_en(b_koin)
            );
        }
        w.insert("lastClaimDate".into(), Value::from(today));
        let mut patch = waifu_patch(&w);
        if let Some(v) = koin_gain(&cur, koin) {
            new_koin = Some(v);
        }
        if let Some(v) = new_koin {
            patch.insert("koin".into(), jnum(v as f64));
        }
        let mut rpg = rpg_map(ctx);
        let new_exp = apply_exp(&mut rpg, old_exp(user), exp);
        patch.insert("exp".into(), jnum(new_exp as f64));
        patch.insert("rpg".into(), Value::Object(rpg));

        let text = format!(
            "🎁 *{}* memberi hadiah harian!\n💰 +{} Koin\n✨ +{exp} EXP{bonus}",
            js_str(w.get("name")),
            thousands_en(koin),
        );
        balas(&ctx.sender, text, Some(patch), Some("🎁"))
    } else {
        action_engine(ctx, user, &mut w, &action, now, rng)
    }
}

/// Aksi `approach`/`intim` biasa (`gachawaifu.js:395-472`).
fn action_engine(
    ctx: &Ctx,
    user: &Map<String, Value>,
    w: &mut Map<String, Value>,
    action: &str,
    now: i64,
    rng: &mut dyn FnMut() -> f64,
) -> Reply {
    let neg_decay = apply_neglect(w, now);
    let mult = if mstr(w.get("nextMultUntil"))
        .and_then(|s| iso_to_ms(&s))
        .is_some_and(|t| now < t)
    {
        0.8
    } else {
        1.0
    };
    let anger = mnum_or(w.get("anger"), 0.0);
    let mood = anger_eff_mood(&mood_state(w, &ctx.sender, now), anger);
    let Some(result) = apply_action(action, w, &mood, mult, rng) else {
        // Aksi tak dikenal. `applyNeglect` sudah memutasi waifu sebelum ini
        // dan di JS ikut ter-flush; mutasinya dibawa balasan ❓.
        return balas(
            &ctx.sender,
            "Aksi tidak dikenali. Gunakan tombol waifu.".to_string(),
            Some(waifu_patch(w)),
            Some("❓"),
        );
    };
    let married = js_truthy(w.get("married"));
    if result.phase == "married" && !married {
        return balas(
            &ctx.sender,
            "Aksi ini hanya untuk pasangan suami istri!".to_string(),
            Some(waifu_patch(w)),
            Some("⛔"),
        );
    }
    let affection = mnum_or(w.get("affection"), 50.0);
    if result.phase == "intim" && !married && affection < 80.0 {
        // JS `return;` — tanpa pesan, tapi mutasi `applyNeglect` tetap ada.
        return Reply::None.with_mutation(set_user(&ctx.sender, waifu_patch(w)));
    }

    if result.phase == "married" {
        let eff = final_gain(result.change, affection, 0, mnum_or(w.get("anger"), 0.0));
        w.insert(
            "anger".into(),
            jnum(anger_update(
                mnum_or(w.get("anger"), 0.0),
                result.like,
                result.dislike,
            )),
        );
        let new_aff = (affection + eff.change as f64).clamp(0.0, 100.0);
        w.insert("affection".into(), jnum(new_aff));
        let koin = eff.change.max(0) * 100;

        let mut patch = waifu_patch(w);
        if koin > 0
            && let Some(v) = koin_gain(user, koin)
        {
            patch.insert("koin".into(), jnum(v as f64));
        }
        let mut rpg = rpg_map(ctx);
        let new_exp = apply_exp(&mut rpg, old_exp(user), result.exp);
        patch.insert("exp".into(), jnum(new_exp as f64));
        patch.insert("rpg".into(), Value::Object(rpg));

        let aff_line = if eff.change >= 0 {
            format!("💞 *Affection +{}*", eff.change)
        } else {
            format!("💞 *Affection {}*", eff.change)
        };
        let text = format!(
            "{}\n\n{} (Total: {}/100)\n💰 +{koin} Koin\n✨ +{} EXP",
            result.text,
            aff_line,
            fmt_num(new_aff),
            result.exp
        );
        return balas(&ctx.sender, text, Some(patch), Some("❤️"));
    }

    let today = today_str(now);
    if mstr(w.get("lastActionDate")).as_deref() != Some(today.as_str()) {
        w.insert("actionsToday".into(), jnum(0.0));
        w.insert("lastActionDate".into(), Value::from(today));
    }
    let actions_today = mnum_or(w.get("actionsToday"), 0.0) as i64 + 1;
    w.insert("actionsToday".into(), jnum(actions_today as f64));

    let eff = final_gain(result.change, affection, actions_today, anger);
    w.insert(
        "anger".into(),
        jnum(anger_update(anger, result.like, result.dislike)),
    );

    let mut new_aff = affection + eff.change as f64;
    let mut event_koin = 0i64;
    let mut event_exp = false;
    let mut event_block = String::new();
    let personality = mstr(w.get("personality")).unwrap_or_default();
    let name = mstr(w.get("name")).unwrap_or_default();
    if let Some(ev) = roll_event(married, result.phase, &personality, &name, rng) {
        new_aff += ev.aff;
        event_koin = ev.koin;
        if ev.anger > 0.0 {
            let a = (mnum_or(w.get("anger"), 0.0) + ev.anger).min(100.0);
            w.insert("anger".into(), jnum(a));
        }
        if let Some(ev_mood) = ev.mood {
            w.insert(
                "mood".into(),
                serde_json::json!({ "type": ev_mood, "since": iso_now(now) }),
            );
            w.insert(
                "moodUntil".into(),
                Value::from(iso_now(now + 12 * 3_600_000)),
            );
        }
        if ev.next_mult < 1.0 {
            w.insert(
                "nextMultUntil".into(),
                Value::from(iso_now(now + 24 * 3_600_000)),
            );
        }
        let aff_part = if ev.aff == 0.0 {
            String::new()
        } else {
            format!(
                " ({}{} aff)",
                if ev.aff > 0.0 { "+" } else { "" },
                fmt_num(ev.aff)
            )
        };
        let koin_part = if event_koin == 0 {
            String::new()
        } else {
            format!(" (+{} koin)", thousands_en(event_koin))
        };
        event_block = format!("\n\n✨ *EVENT:* {}{aff_part}{koin_part}", ev.text);
        event_exp = true;
    }
    let new_aff = new_aff.clamp(0.0, 100.0);
    w.insert("affection".into(), jnum(new_aff));

    let mut patch = waifu_patch(w);
    if event_koin != 0
        && let Some(v) = koin_gain(user, event_koin)
    {
        patch.insert("koin".into(), jnum(v as f64));
    }
    let mut rpg = rpg_map(ctx);
    let gain_exp = result.exp + if event_exp { 15 } else { 0 };
    let new_exp = apply_exp(&mut rpg, old_exp(user), gain_exp);
    patch.insert("exp".into(), jnum(new_exp as f64));
    patch.insert("rpg".into(), Value::Object(rpg));

    let mut aff_text = format!("💞 *Affection:* {}/100", fmt_num(new_aff));
    if eff.change != 0 {
        aff_text = format!(
            "💞 *Affection berubah:* {}{} (Total: {}/100)",
            if eff.change > 0 { "+" } else { "" },
            eff.change,
            fmt_num(new_aff)
        );
    }
    if new_aff == 100.0 {
        aff_text = "💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Nikahi dia sekarang!*".to_string();
    }
    if neg_decay > 0 {
        aff_text.push_str(&format!("\n📉 *Neglect: -{neg_decay} aff* (jarang interaksi)"));
    }
    if eff.angry {
        aff_text.push_str(&format!("\n😡 *Dia MARAH!* Drain -{} aff", eff.drain));
    }
    if !eff.extra.is_empty() {
        aff_text.push('\n');
        aff_text.push_str(&eff.extra.join("\n"));
    }

    if new_aff <= 0.0 {
        let mut mutations = Vec::new();
        if married {
            let jid = waifu_jid(&name);
            if map_of(user.get("fun")).get("pasangan").and_then(Value::as_str) == Some(jid.as_str())
            {
                let mut fun = map_of(user.get("fun"));
                fun.insert("pasangan".into(), Value::from(""));
                patch.insert("fun".into(), Value::Object(fun));
            }
            let mut fun = Map::new();
            fun.insert("pasangan".into(), Value::from(""));
            let mut data = Map::new();
            data.insert("fun".into(), Value::Object(fun));
            mutations.push(set_user(&jid, data));
        }
        patch.insert("waifu".into(), Value::Null);
        mutations.push(set_user(&ctx.sender, patch));
        let text = format!(
            "💔 *{} MENINGGALKANMU!* 💔\n\n\
             Karena kasih sayangnya habis (0), dia pergi meninggalkan surat basah air mata. \
             Kamu kehilangan waifumu! *(Ketik {}gachawaifu untuk memulai ulang)*",
            name.to_uppercase(),
            ctx.prefix
        );
        return Reply::text(text).mutating(mutations).with_reaction("💔");
    }

    let koin_line = if event_koin != 0 {
        format!("\n💰 +{} Koin", thousands_en(event_koin))
    } else {
        String::new()
    };
    let text = format!(
        "{}{}\n\n{}\n✨ +{} EXP{koin_line}",
        result.text, event_block, aff_text, result.exp
    );
    let emoji = if new_aff == 100.0 { "💍" } else { "✨" };
    balas(&ctx.sender, text, Some(patch), Some(emoji))
}

#[plugin(
    name = "gachawaifu",
    alias = ["waifuaction", "tinggalinwaifu", "waifuku", "istriku", "gachaistri"],
    category = "fun",
    description = "Gacha waifu impianmu, jaga perasaannya, dan jadikan dia pasanganmu!",
    usage = ".gachawaifu | .waifuku | .tinggalinwaifu",
    example = ".gachawaifu",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn gachawaifu(ctx: Ctx) -> Reply {
    let mut rng = math_random;
    dispatch(&ctx, &mut rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::error_template_bawaan;
    use std::collections::VecDeque;

    const ME: &str = "628111@s.whatsapp.net";
    const DAY: i64 = 86_400_000;

    /// RNG deterministik: tiap panggilan memakai angka berikutnya.
    fn scripted(values: Vec<f64>) -> impl FnMut() -> f64 {
        let mut q: VecDeque<f64> = values.into();
        move || q.pop_front().expect("rng habis — urutan undian berubah")
    }

    fn ctx_of(command: &str, args: &[&str], user: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: ME.into(),
            chat: ME.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        }
    }

    fn run(command: &str, args: &[&str], user: Value, seq: Vec<f64>) -> Reply {
        let mut rng = scripted(seq);
        dispatch(&ctx_of(command, args, user), &mut rng)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.clone()),
            _ => None,
        }
    }

    fn written_jids(r: &Reply) -> Vec<String> {
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, .. } => jid.clone(),
                other => panic!("mutasi harus SetUser, dapat {other:?}"),
            })
            .collect()
    }

    fn patch_to(r: &Reply, jid: &str) -> Map<String, Value> {
        for m in r.mutations() {
            if let Mutation::SetUser { jid: j, data } = m
                && j == jid
            {
                return data
                    .as_object()
                    .cloned()
                    .unwrap_or_else(|| panic!("data bukan object: {data}"));
            }
        }
        panic!("tidak ada SetUser untuk {jid}");
    }

    fn patch_of(r: &Reply) -> Map<String, Value> {
        patch_to(r, ME)
    }

    /// Waifu siap pakai; mood dipatok `biasa` lewat `moodUntil` jauh di depan
    /// supaya `getDailyMood` tidak ikut main.
    fn waifu_json(name: &str, tier: &str, affection: i64, personality: &str) -> Value {
        serde_json::json!({
            "name": name,
            "series": "Seri",
            "tier": tier,
            "personality": personality,
            "keyword": "kw",
            "age": 17,
            "height": 160,
            "weight": 50,
            "affection": affection,
            "married": false,
            "imageUrl": IMAGE_FALLBACK,
            "mood": { "type": "biasa", "since": "2020-01-01T00:00:00.000Z" },
            "moodUntil": "2999-01-01T00:00:00.000Z"
        })
    }

    fn user_with(waifu: Value) -> Value {
        serde_json::json!({ "waifu": waifu, "exp": 0, "energi": 5, "koin": 100 })
    }

    // ------------------------------------------------------------- meta

    #[test]
    fn meta_sesuai_config_js() {
        let p = oktz_plugin_api::find("gachawaifu").unwrap();
        assert_eq!(p.meta.name, "gachawaifu");
        assert_eq!(
            p.meta.alias,
            ["waifuaction", "tinggalinwaifu", "waifuku", "istriku", "gachaistri"]
        );
        assert_eq!(p.meta.category, "fun");
        assert_eq!(
            p.meta.description,
            "Gacha waifu impianmu, jaga perasaannya, dan jadikan dia pasanganmu!"
        );
        assert_eq!(p.meta.usage, ".gachawaifu | .waifuku | .tinggalinwaifu");
        assert_eq!(p.meta.example, ".gachawaifu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_private && !p.meta.is_group);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn semua_alias_dan_command_js_resolve_ke_plugin_ini() {
        for key in [
            "gachawaifu",
            "gachaistri",
            "waifuaction",
            "tinggalinwaifu",
            "waifuku",
            "istriku",
        ] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("gachawaifu"),
                "key {key} salah resolve"
            );
        }
    }

    // ------------------------------------------------------------ tabel

    #[test]
    fn tabel_actions_dan_konstanta_sesuai_waifu_js() {
        assert_eq!(ACTIONS_APPROACH.len(), 34);
        assert_eq!(ACTIONS_INTIM.len(), 12);
        assert_eq!(ACTIONS_MARRIED.len(), 7);
        assert_eq!(PERSONALITY_TAG.len(), 10);
        assert_eq!(EVENTS.len(), 20);
        assert_eq!(MENUS.len(), 19);
        assert_eq!(MESUM_ACTIONS.len(), 15);
        assert_eq!(PITY_THRESHOLD, 20);
        assert_eq!(EVENT_CHANCE, 0.18);
        assert_eq!(ANGER_THRESHOLD, 50.0);
    }

    #[test]
    fn tiap_teks_aksi_memakai_placeholder_n() {
        // Teks JS yang memang tidak memakai `${n}` (`src/lib/waifu.js:35,38,57`).
        const TANPA_N: [&str; 3] = ["peluk_depan", "mesra", "bulanmadu_hotel"];
        for a in ACTIONS_APPROACH
            .iter()
            .chain(ACTIONS_INTIM)
            .chain(ACTIONS_MARRIED)
        {
            if TANPA_N.contains(&a.key) {
                assert!(!a.text.contains("{n}"), "{}", a.key);
                continue;
            }
            assert!(a.text.contains("{n}"), "{} tanpa {{n}}", a.key);
            assert!(find_action(a.key).is_some(), "{} tidak ketemu", a.key);
        }
    }

    #[test]
    fn phase_tiap_aksi_sesuai_konstanta_js() {
        for a in ACTIONS_APPROACH {
            assert_eq!(a.phase, "approach", "{}", a.key);
        }
        for a in ACTIONS_INTIM {
            assert_eq!(a.phase, "intim", "{}", a.key);
        }
        for a in ACTIONS_MARRIED {
            assert_eq!(a.phase, "married", "{}", a.key);
        }
    }

    #[test]
    fn menu_lainnya_ada_judulnya() {
        assert_eq!(
            MENUS.iter().find(|(k, _)| *k == "menu_lainnya").unwrap().1,
            "🎮 Aksi lainnya..."
        );
    }

    // ------------------------------------------------------------ pool

    fn raw(name: &'static str, tier: &'static str, personality: &'static str) -> RawWaifu {
        RawWaifu {
            name,
            series: "Seri",
            tier,
            personality,
            keyword: None,
            age: 17,
            height: 160,
            weight: 50,
        }
    }

    #[test]
    fn build_pool_menyaring_dan_menghitung_bobot() {
        let pool = build_pool(&[
            raw("Yuki", "Common", "tsundere"),
            raw("Yuki", "Rare", "genki"),
            RawWaifu {
                name: "",
                ..raw("Kosong", "Common", "genki")
            },
            RawWaifu {
                age: 0,
                ..raw("Muda", "Common", "genki")
            },
            raw("Salah", "TidakAdaTier", "genki"),
            raw("SalahOrang", "Common", "tidakada"),
            raw("Miku", "Common", "genki"),
        ]);
        let names: Vec<&str> = pool.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, ["Yuki", "Miku"], "duplikat + invalid dibuang");
        assert_eq!(pool[0].keyword, "Yuki Seri anime", "keyword default JS");
        assert_eq!(pool[0].roll_weight, 55.0 / 2.0, "Common dibagi 2 entri");
        assert_eq!(pool[1].roll_weight, 55.0 / 2.0);
    }

    #[test]
    fn roll_waifu_pity_dan_bobot() {
        let pool = build_pool(&[
            raw("Yuki", "Common", "tsundere"),
            raw("Miku", "Epic", "genki"),
        ]);
        // Pity belum aktif: undian pertama kena entri pertama.
        let mut rng = scripted(vec![0.0]);
        assert_eq!(roll_waifu(&pool, 19, &mut rng).unwrap().name, "Yuki");
        // Pity aktif: Common difilter, hanya Epic yang tersisa.
        let mut rng = scripted(vec![0.0]);
        assert_eq!(roll_waifu(&pool, 20, &mut rng).unwrap().name, "Miku");
        // r = 1.0 * total: semua bobot dikurangi, sisa 0 di entri terakhir.
        let mut rng = scripted(vec![1.0]);
        assert_eq!(roll_waifu(&pool, 0, &mut rng).unwrap().name, "Miku");
        // Pool kosong = `undefined` di JS (picked.keyword melempar).
        let mut rng = scripted(vec![0.0]);
        assert!(roll_waifu(&[], 0, &mut rng).is_none());
    }

    // ------------------------------------------------------- engine

    #[test]
    fn diminish_floor_dan_lantai_15_persen() {
        assert_eq!(diminish(10.0, 50.0), 6, "10 * 0.55 = 5.5 → round 6");
        assert_eq!(diminish(1.0, 100.0), 1, "f = 0.15, hasil dibulatkan 0 → max(1)");
        assert_eq!(diminish(100.0, 0.0), 55, "affection 0 → `(current || 50)` → f 0.55");
    }

    #[test]
    fn fatigue_mult_tiga_aksi_gratis_lalu_turun() {
        assert_eq!(fatigue_mult(0), 1.0);
        assert_eq!(fatigue_mult(3), 1.0);
        assert!((fatigue_mult(4) - 0.88).abs() < 1e-9);
        assert_eq!(fatigue_mult(10), 0.25, "lantai 0.25");
        assert_eq!(fatigue_mult(100), 0.25);
    }

    #[test]
    fn final_gain_diminish_fatigue_dan_drain_anger() {
        let g = final_gain(10, 50.0, 4, 50.0);
        assert_eq!(g.change, 3, "6 * 0.88 = 5.28 → 5, drain floor(50/25) = 2");
        assert_eq!(g.drain, 2);
        assert!(g.angry);
        assert_eq!(g.extra.len(), 1);
        assert!(g.extra[0].contains("(×0.88)"), "{}", g.extra[0]);

        let calm = final_gain(10, 50.0, 0, 0.0);
        assert_eq!(calm.change, 6);
        assert_eq!(calm.drain, 0);
        assert!(calm.extra.is_empty());
        // Drain penuh: 6 - floor(100/25) = 2. (Lantai -50 tak terjangkau —
        // diminish selalu ≥ 1 sedangkan drain maksimum 4.)
        assert_eq!(final_gain(10, 50.0, 0, 100.0).change, 2);
    }

    #[test]
    fn neglect_decay_24_jam_dan_cap_20() {
        let now = 1_000_000_000_000;
        assert_eq!(neglect_decay(None, now), (0, 0.0));
        assert_eq!(neglect_decay(Some(now - 10 * 3_600_000), now), (0, 10.0));
        assert_eq!(neglect_decay(Some(now - 50 * 3_600_000), now).0, 6);
        assert_eq!(neglect_decay(Some(now - 30 * DAY), now).0, 20);
    }

    #[test]
    fn apply_neglect_memutasi_affection_anger_dan_menandai_interaksi() {
        let now = 1_000_000_000_000;
        let mut w = map_of(Some(&serde_json::json!({
            "affection": 50, "anger": 30, "lastInteractionAt": now - 50 * 3_600_000
        })));
        assert_eq!(apply_neglect(&mut w, now), 6);
        assert_eq!(w["affection"], serde_json::json!(44));
        assert_eq!(w["anger"], serde_json::json!(20));
        assert_eq!(w["lastInteractionAt"], serde_json::json!(now));
    }

    #[test]
    fn anger_update_dislike_like_dan_clamp() {
        assert_eq!(anger_update(0.0, false, true), 12.0);
        assert_eq!(anger_update(20.0, true, false), 12.0);
        assert_eq!(anger_update(0.0, true, false), 0.0, "clamp bawah");
        assert_eq!(anger_update(100.0, false, true), 100.0, "clamp atas");
        assert_eq!(anger_update(40.0, false, false), 40.0);
    }

    #[test]
    fn anger_eff_mood_menimpa_mood_50_ke_atas() {
        assert_eq!(anger_eff_mood("ceria", 49.9), "ceria");
        assert_eq!(anger_eff_mood("ceria", 50.0), "marah");
    }

    #[test]
    fn apply_action_urutan_mult_like_menang_atas_dislike() {
        // rayu: `likes` memuat tsundere, TAPI tag tsundere men-dislike `rayu`
        // — JS memakai `if (liked) ... else if (disliked)`, jadi 1.2.
        let w = map_of(Some(&serde_json::json!({ "name": "Yuki", "personality": "tsundere" })));
        let mut rng = scripted(vec![0.0]);
        let r = apply_action("rayu", &w, "biasa", 1.0, &mut rng).unwrap();
        assert_eq!(r.change, 10, "8 * 1.2 = 9.6 → 10");
        assert!(r.like && r.dislike);

        // himedere vs kafe_kopi: hanya dislike → 0.7.
        let w = map_of(Some(&serde_json::json!({ "name": "Yuki", "personality": "himedere" })));
        let mut rng = scripted(vec![0.0]);
        let r = apply_action("kafe_kopi", &w, "biasa", 1.0, &mut rng).unwrap();
        assert_eq!(r.change, 3, "4 * 0.7 = 2.8 → 3");
        assert!(!r.like && r.dislike);

        // Mood & mult override ikut mengali: 4 * 0.5 * 0.8 * 1.2 = 1.92 → 2.
        let w = map_of(Some(&serde_json::json!({ "name": "Yuki", "personality": "tsundere" })));
        let mut rng = scripted(vec![0.0]);
        let r = apply_action("karaoke_solo", &w, "marah", 0.8, &mut rng).unwrap();
        assert_eq!(r.change, 2);
    }

    #[test]
    fn apply_action_none_untuk_aksi_atau_personality_hilang() {
        let w = map_of(Some(&serde_json::json!({ "name": "Yuki", "personality": "genki" })));
        let mut rng = scripted(vec![]);
        assert!(apply_action("tidak_ada", &w, "biasa", 1.0, &mut rng).is_none());
        let kosong = map_of(Some(&serde_json::json!({ "name": "Yuki" })));
        let mut rng = scripted(vec![]);
        assert!(apply_action("mesra", &kosong, "biasa", 1.0, &mut rng).is_none());
    }

    #[test]
    fn roll_event_gate_018_dan_index_pool() {
        // r = 0.19 > 0.18: gugur tanpa undian indeks.
        let mut rng = scripted(vec![0.19]);
        assert!(roll_event(false, "approach", "genki", "Yuki", &mut rng).is_none());
        // Fase approach unmarried: 15 entri; 6.5/15 → index 6 = late.
        let mut rng = scripted(vec![0.0, 6.5 / 15.0]);
        let ev = roll_event(false, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert!(
            ev.text.starts_with("📉 Kamu hampir telat janji temu!"),
            "{}",
            ev.text
        );
        assert_eq!(ev.mood, Some("sedih"));
        assert_eq!(ev.next_mult, 0.8);
        assert_eq!(ev.aff, 0.0);
        // Married: 17 entri (anniv + kenangan ikut); 7.5/17 → index 7 = anniv.
        let mut rng = scripted(vec![0.0, 7.5 / 17.0]);
        let ev = roll_event(true, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert!(
            ev.text.contains("Kenangan hari pernikahan kalian"),
            "{}",
            ev.text
        );
        assert_eq!(ev.aff, 6.0);
    }

    #[test]
    fn roll_event_rival_yandere_dan_cat_deredere() {
        // rival index 2 → 2.5/15.
        let mut rng = scripted(vec![0.0, 2.5 / 15.0]);
        let ev = roll_event(false, "approach", "yandere", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("Seorang rival mendekati"), "{}", ev.text);
        assert_eq!(ev.aff, 5.0, "yandereAff");
        let mut rng = scripted(vec![0.0, 2.5 / 15.0]);
        let ev = roll_event(false, "approach", "kuudere", "Yuki", &mut rng).unwrap();
        assert_eq!(ev.aff, -4.0);

        // cat index 4 → 4.5/15.
        let mut rng = scripted(vec![0.0, 4.5 / 15.0]);
        let ev = roll_event(false, "approach", "deredere", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("Seekor kucing lucu"), "{}", ev.text);
        assert_eq!(ev.aff, 9.0);
        let mut rng = scripted(vec![0.0, 4.5 / 15.0]);
        let ev = roll_event(false, "approach", "kuudere", "Yuki", &mut rng).unwrap();
        assert_eq!(ev.aff, 4.0);
    }

    #[test]
    fn roll_event_koin_dompet_dan_lotre() {
        // wallet index 1 → 1.5/15; koin 1000 + floor(0 * 24001).
        let mut rng = scripted(vec![0.0, 1.5 / 15.0, 0.0]);
        let ev = roll_event(false, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("menemukan dompet"), "{}", ev.text);
        assert_eq!(ev.koin, 1000);

        // lottery index 5 → 5.5/15; menang (0.0 < 0.5) → 20000 + floor(0.5*40001).
        let mut rng = scripted(vec![0.0, 5.5 / 15.0, 0.0, 0.5]);
        let ev = roll_event(false, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("Tiket lotre"), "{}", ev.text);
        assert_eq!(ev.koin, 40000);

        // kalah: satu undian saja setelah cek menang.
        let mut rng = scripted(vec![0.0, 5.5 / 15.0, 0.9]);
        let ev = roll_event(false, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert_eq!(ev.koin, 0);
    }

    #[test]
    fn roll_event_marah_salahpaham_menambah_anger_12() {
        // salahpaham index 10 → 10.5/15.
        let mut rng = scripted(vec![0.0, 10.5 / 15.0]);
        let ev = roll_event(false, "approach", "genki", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("salah paham"), "{}", ev.text);
        assert_eq!(ev.aff, -5.0);
        assert_eq!(ev.anger, 12.0);
    }

    #[test]
    fn roll_event_married_only_saat_married_dan_pool_fase() {
        // Fase intim unmarried: `any` non-marriedOnly (14) + intimate + confess
        // = 16 entri; index 0 = rain.
        let mut rng = scripted(vec![0.0, 0.0]);
        let ev = roll_event(false, "intim", "genki", "Yuki", &mut rng).unwrap();
        assert!(ev.text.starts_with("🌧️ Hujan deras turun!"), "{}", ev.text);
        assert_eq!(ev.aff, 5.0);
        // Fase `married` saat unmarried: gift (marriedOnly) dibuang → 14 entri;
        // 13.5/14 → index 13 = rezeki.
        let mut rng = scripted(vec![0.0, 13.5 / 14.0, 0.0]);
        let ev = roll_event(false, "married", "genki", "Yuki", &mut rng).unwrap();
        assert!(ev.text.contains("Rezeki nomplok"), "{}", ev.text);
        assert_eq!(ev.koin, 3000);
        // Married: pool 17 entri; 8.5/17 → index 8 = gift (marriedOnly).
        let mut rng = scripted(vec![0.0, 8.5 / 17.0, 0.0]);
        let ev = roll_event(true, "married", "genki", "Yuki", &mut rng).unwrap();
        assert_eq!(ev.text, "🎁 Hadiah kejutan kecil untukmu sebagai pasangan.");
        assert_eq!(ev.koin, 2000);
    }

    // ------------------------------------------------ helper waktu & teks

    #[test]
    fn iso_now_dan_iso_to_ms_bolak_balik() {
        for ms in [0i64, 1_700_000_000_123, 1_766_000_000_000] {
            let iso = iso_now(ms);
            assert!(iso.ends_with('Z'), "{iso}");
            assert_eq!(iso_to_ms(&iso), Some(ms), "round-trip {iso}");
        }
        assert_eq!(iso_now(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_to_ms("bukan tanggal"), None);
        assert_eq!(iso_to_ms(""), None);
    }

    #[test]
    fn marrage_day_nol_untuk_tanggal_hilang_atau_rusak() {
        let now = 1_700_000_000_000;
        assert_eq!(marrage_day(None, now), 0);
        assert_eq!(marrage_day(Some("rusak"), now), 0);
        assert_eq!(marrage_day(Some(&iso_now(now - 6 * DAY)), now), 7);
        assert_eq!(marrage_day(Some(&iso_now(now)), now), 1);
    }

    #[test]
    fn waifu_jid_membuang_semua_whitespace() {
        assert_eq!(waifu_jid("Yuki"), "waifu_Yuki@s.whatsapp.net");
        assert_eq!(
            waifu_jid("Hinata Hyuga"),
            "waifu_HinataHyuga@s.whatsapp.net"
        );
        assert_eq!(waifu_jid(" A  B "), "waifu_AB@s.whatsapp.net");
    }

    #[test]
    fn anger_meter_bentuk_bar_dan_nilai() {
        let kosong = map_of(Some(&serde_json::json!({})));
        assert_eq!(anger_meter(&kosong), "░░░░░░░░░░ (0/100)");
        let w = map_of(Some(&serde_json::json!({ "anger": 60 })));
        assert_eq!(anger_meter(&w), "██████░░░░ (60/100)");
        let w = map_of(Some(&serde_json::json!({ "anger": 100 })));
        assert_eq!(anger_meter(&w), "██████████ (100/100)");
    }

    #[test]
    fn backfill_personality_stabil_dan_di_daftar() {
        for name in ["Hinata", "Yuki", "Miku", "Nagatoro", "Asuka"] {
            assert!(
                VALID_PERSONALITIES.contains(&backfill_personality(name)),
                "{name}"
            );
        }
        assert_eq!(backfill_personality("Hinata"), "kuudere");
    }

    #[test]
    fn get_daily_mood_menjangkau_kelima_mood() {
        let mut seen: Vec<&str> = Vec::new();
        for jid in 0..500 {
            let m = get_daily_mood(&format!("6281{jid:03}@s.whatsapp.net"), "1/1/2026");
            if !seen.contains(&m) {
                seen.push(m);
            }
        }
        seen.sort();
        assert_eq!(seen, ["biasa", "ceria", "marah", "romantis", "sedih"]);
    }

    #[test]
    fn dowry_dan_tier_mult_sesuai_romance_js() {
        let d = dowry_of("Mythic").unwrap();
        assert_eq!((d.limit, d.koin, d.exp), (30000, 1_000_000, 30000));
        assert!(dowry_of("TidakAda").is_none());
        assert_eq!(tier_mult("Common"), 1.0);
        assert_eq!(tier_mult("Rare"), 1.2);
        assert_eq!(tier_mult("Epic"), 1.5);
        assert_eq!(tier_mult("Legendary"), 2.0);
        assert_eq!(tier_mult("Mythic"), 3.0);
        assert_eq!(tier_mult("TidakAda"), 1.0);
    }

    #[test]
    fn record_pull_menghitung_tier_pity_rarest_dan_cap_100() {
        let mut stats = default_stats();
        let mut user = map_of(None);
        record_pull(&mut stats, &mut user, "Yuki", "Seri", "Common", "2026-01-01T00:00:00.000Z");
        assert_eq!(stats["totalGacha"], serde_json::json!(1));
        assert_eq!(stats["byTier"]["Common"], serde_json::json!(1));
        assert_eq!(stats["pityCounter"], serde_json::json!(1), "Common tidak reset pity");
        assert_eq!(stats["rarest"]["name"], serde_json::json!("Yuki"));

        record_pull(&mut stats, &mut user, "Miku", "Seri", "Epic", "2026-01-02T00:00:00.000Z");
        assert_eq!(stats["pityCounter"], serde_json::json!(0), "Epic reset pity");
        assert_eq!(stats["rarest"]["name"], serde_json::json!("Miku"));
        // Rarest tidak diturunkan saat tier lebih rendah.
        record_pull(&mut stats, &mut user, "Rin", "Seri", "Rare", "2026-01-03T00:00:00.000Z");
        assert_eq!(stats["rarest"]["name"], serde_json::json!("Miku"));

        // Cap 100: dorong 100 entri lalu satu pull lagi.
        for i in 0..100 {
            record_pull(
                &mut stats,
                &mut user,
                &format!("W{i}"),
                "Seri",
                "Common",
                "2026-01-04T00:00:00.000Z",
            );
        }
        let history = user["waifuHistory"].as_array().unwrap();
        assert_eq!(history.len(), 100, "slice(-100)");
        assert_eq!(history[0]["name"], serde_json::json!("W0"), "yang tertua dibuang");
        assert_eq!(history[99]["name"], serde_json::json!("W99"));
    }

    #[test]
    fn guard_energi_dan_koin_mengikuti_database_js() {
        let user = map_of(Some(&serde_json::json!({ "energi": 5, "koin": 100 })));
        assert_eq!(energi_gain(&user, 1000, false), Some(1005));
        assert_eq!(energi_gain(&user, -100, false), Some(0), "clamp bawah");
        assert_eq!(energi_gain(&user, 1000, true), None, "owner tidak menulis");
        let unlimited = map_of(Some(&serde_json::json!({ "energi": -1 })));
        assert_eq!(energi_gain(&unlimited, 1000, false), None, "energi -1");
        let kosong = map_of(None);
        assert_eq!(energi_gain(&kosong, 7, false), Some(7));

        assert_eq!(koin_gain(&user, 300), Some(400));
        assert_eq!(koin_gain(&user, -1000), Some(0));
        let unlimited = map_of(Some(&serde_json::json!({ "koin": -1 })));
        assert_eq!(koin_gain(&unlimited, 300), None);
        let besar = map_of(Some(&serde_json::json!({ "koin": MAX_KOIN - 1 })));
        assert_eq!(koin_gain(&besar, 300), Some(MAX_KOIN), "clamp MAX_KOIN");
    }

    #[test]
    fn js_round_setengah_ke_atas_bukan_menjauh_dari_nol() {
        assert_eq!(js_round(5.5), 6.0);
        assert_eq!(js_round(-5.5), -5.0);
        assert_eq!(js_round(2.4), 2.0);
    }

    // ---------------------------------------------------------- handler

    #[test]
    fn user_tidak_ada_berarti_diam_total() {
        let r = run("waifuku", &[], Value::Null, vec![]);
        assert!(r.is_none(), "JS `if (!user) return`");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn command_di_luar_daftar_diam() {
        let r = run("putus", &[], serde_json::json!({}), vec![]);
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn status_tanpa_waifu_memberi_arahan_gacha() {
        let r = run("waifuku", &[], serde_json::json!({}), vec![]);
        assert_eq!(
            text_of(&r),
            "⚠️ *Kamu belum memiliki waifu!*\nSilakan ketik *.gachawaifu* untuk memulainya!"
        );
        assert!(r.mutations().is_empty());
        assert!(reaction_of(&r).is_none(), "JS tidak bereaksi di cabang ini");
    }

    #[test]
    fn status_menampilkan_neglect_anger_dan_meter() {
        let now = now_ms();
        let mut w = waifu_json("Yuki", "Rare", 50, "tsundere");
        w["anger"] = serde_json::json!(60);
        w["lastInteractionAt"] = serde_json::json!(now - 50 * 3_600_000);
        let r = run("waifuku", &[], user_with(w), vec![]);
        let text = text_of(&r);
        assert!(text.starts_with("📸 *STATUS WAIFU KAMU* 📸\n\n💖 *Nama:* Yuki\n"), "{text}");
        assert!(text.contains("💎 *Tier:* Rare\n🎭 *Personality:* tsundere\n"), "{text}");
        assert!(text.contains("🌤️ *Mood hari ini:* biasa 🙂\n"), "{text}");
        assert!(text.contains("😡 *Anger:* █████░░░░░ (50/100)\n"), "{text}");
        assert!(text.contains("💞 *Affection:* 44/100\n"), "{text}");
        assert!(text.contains("💍 *Status:* Pendekatan 💖\n"), "{text}");
        assert!(
            text.contains("📉 *Neglect:* Affection turun -6 karena kamu jarang interaksi!\n"),
            "{text}"
        );
        assert!(
            text.contains("⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n"),
            "anger 50 sisa masih kena ambang"
        );
        assert!(text.ends_with("Lanjutkan interaksi dengan memilih aksi di bawah!"), "{text}");
        assert_eq!(reaction_of(&r).as_deref(), Some("✅"));

        let patch = patch_of(&r);
        assert_eq!(patch["waifu"]["affection"], serde_json::json!(44));
        assert_eq!(patch["waifu"]["anger"], serde_json::json!(50));
        assert!(patch["waifu"]["lastInteractionAt"].is_number());
    }

    #[test]
    fn status_menikah_menampilkan_hari_dan_badge_milestone() {
        let now = now_ms();
        let mut w = waifu_json("Yuki", "Rare", 100, "genki");
        w["married"] = serde_json::json!(true);
        w["marriedDate"] = serde_json::json!(iso_now(now - 6 * DAY));
        w["lastInteractionAt"] = serde_json::json!(now);
        let r = run("waifuku", &[], user_with(w), vec![]);
        let text = text_of(&r);
        assert!(text.contains("💍 *Status:* Telah Menikah 💍\n🗓️ *Hari ke-7* 🎉\n"), "{text}");
        assert!(text.contains("💞 *Affection:* 100/100\n"), "{text}");
        assert!(!text.contains("Neglect"), "baru interaksi: {text}");
        assert!(!text.contains("MARAH"));
    }

    #[test]
    fn tinggalin_tanpa_waifu_menolak() {
        let r = run("tinggalinwaifu", &[], serde_json::json!({}), vec![]);
        assert_eq!(text_of(&r), "⚠️ *Kamu bahkan belum punya waifu!* Halu ya?");
        assert!(r.mutations().is_empty());
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn tinggalin_belum_menikah_hanya_menghapus_waifu() {
        let r = run(
            "tinggalinwaifu",
            &[],
            user_with(waifu_json("Yuki", "Rare", 50, "genki")),
            vec![],
        );
        assert_eq!(written_jids(&r), [ME.to_string()]);
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"], Value::Null);
        assert!(patch.get("fun").is_none(), "pasangan tidak ada, tidak ditulis");
        assert!(
            text_of(&r).contains("💔 *KAMU MEMUTUSKAN UNTUK MENINGGALKAN YUKI!*"),
            "{}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("💔"));
    }

    #[test]
    fn tinggalin_saat_menikah_ikut_mengosongkan_pasangan_kedua_sisi() {
        let jid = waifu_jid("Yuki");
        let mut user = user_with(waifu_json("Yuki", "Rare", 50, "genki"));
        user["waifu"]["married"] = serde_json::json!(true);
        user["fun"] = serde_json::json!({ "pasangan": jid });
        let r = run("tinggalinwaifu", &[], user, vec![]);
        assert_eq!(
            written_jids(&r),
            [jid.clone(), ME.to_string()],
            "sisi waifu ditulis lebih dulu"
        );
        let partner = patch_to(&r, &jid);
        assert_eq!(partner["fun"]["pasangan"], serde_json::json!(""));
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"], Value::Null);
        assert_eq!(patch["fun"]["pasangan"], serde_json::json!(""));
    }

    #[test]
    fn gacha_menolak_kalau_sudah_ada_waifu() {
        let r = run(
            "gachawaifu",
            &[],
            user_with(waifu_json("Yuki", "Rare", 50, "genki")),
            vec![],
        );
        assert_eq!(
            text_of(&r),
            "⚠️ *Kamu sudah memiliki waifu!*\nNama: *Yuki*\nTier: *Rare*\nAffection: *50/100*\n\n\
             Jangan serakah! Ketik *.waifuku* untuk berinteraksi dengannya."
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("😡"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn gacha_tanpa_start_menampilkan_panduan() {
        let r = run("gachaistri", &[], serde_json::json!({ "exp": 0 }), vec![]);
        let text = text_of(&r);
        assert!(text.starts_with("💕 *SISTEM GACHA WAIFU* 💕\n\n"), "{text}");
        assert!(text.contains("• *.gachawaifu* — Panduan ini"), "{text}");
        assert!(text.contains("• *.tinggalinwaifu* — Putuskan hubungan"), "{text}");
        assert!(text.contains("4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+."), "{text}");
        assert!(text.ends_with("5. Aksi memberi EXP + reward ekonomi."), "{text}");
        assert!(r.mutations().is_empty());
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn gacha_dengan_pool_kosong_jatuh_ke_error_template_dan_reaksi_can() {
        // `rollWaifu` JS tetap memanggil rng() sekali walau total bobot 0.
        let r = run("gachawaifu", &["start"], serde_json::json!({ "exp": 0 }), vec![0.0]);
        assert_eq!(text_of(&r), error_template_bawaan(".", "gachawaifu"));
        assert_eq!(reaction_of(&r).as_deref(), Some("☢"));
        assert!(r.mutations().is_empty(), "cabang catch tidak menulis apa pun");
    }

    #[test]
    fn waifuaction_tanpa_waifu_ditolak() {
        let r = run("waifuaction", &["menu_peluk"], serde_json::json!({}), vec![]);
        assert_eq!(
            text_of(&r),
            "Kamu belum memiliki waifu! Ketik *.gachawaifu* untuk memulai."
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("❌"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn menu_menampilkan_judul_tanpa_mengubah_store() {
        let r = run(
            "waifuaction",
            &["menu_peluk"],
            user_with(waifu_json("Yuki", "Rare", 50, "genki")),
            vec![],
        );
        assert_eq!(text_of(&r), "🫂 Cara memeluk...");
        assert!(r.mutations().is_empty(), "tanpa perubahan tidak setUser");
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn menu_mengisi_personality_yang_hilang() {
        let mut w = waifu_json("Hinata", "Rare", 50, "genki");
        w["personality"] = Value::Null;
        let r = run("waifuaction", &["menu_peluk"], user_with(w), vec![]);
        assert_eq!(text_of(&r), "🫂 Cara memeluk...");
        assert_eq!(
            patch_of(&r)["waifu"]["personality"],
            serde_json::json!("kuudere")
        );
    }

    #[test]
    fn aksi_mesum_menampar_dan_mengurangi_affection_30_sampai_59() {
        let r = run(
            "waifuaction",
            &["peluk"],
            user_with(waifu_json("Yuki", "Rare", 79, "genki")),
            vec![0.0],
        );
        let text = text_of(&r);
        assert!(
            text.starts_with("💢 *PLAKK!!* Kamu mencoba bersikap mesum kepada *Yuki*,"),
            "{text}"
        );
        assert!(text.ends_with("💞 *Affection -30*"), "{text}");
        assert_eq!(patch_of(&r)["waifu"]["affection"], serde_json::json!(49));
        assert_eq!(reaction_of(&r).as_deref(), Some("💢"));
    }

    #[test]
    fn aksi_intim_di_bawah_80_diam_tapi_tetap_menyimpan() {
        let r = run(
            "waifuaction",
            &["cium_bibir"],
            user_with(waifu_json("Yuki", "Rare", 50, "genki")),
            vec![0.0],
        );
        assert!(r.is_none(), "JS `return;` tanpa pesan");
        assert_eq!(written_jids(&r), [ME.to_string()]);
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"]["affection"], serde_json::json!(50));
        assert!(patch["waifu"]["lastInteractionAt"].is_number());
    }

    #[test]
    fn aksi_married_di_luar_pernikahan_diblokir() {
        let r = run(
            "waifuaction",
            &["mesra"],
            user_with(waifu_json("Yuki", "Rare", 90, "deredere")),
            vec![0.0],
        );
        assert_eq!(text_of(&r), "Aksi ini hanya untuk pasangan suami istri!");
        assert_eq!(reaction_of(&r).as_deref(), Some("⛔"));
        assert_eq!(patch_of(&r)["waifu"]["affection"], serde_json::json!(90));
    }

    #[test]
    fn aksi_tak_dikenal_diberi_reaksi_tanya() {
        let r = run(
            "waifuaction",
            &["tidak_ada"],
            user_with(waifu_json("Yuki", "Rare", 50, "genki")),
            vec![],
        );
        assert_eq!(text_of(&r), "Aksi tidak dikenali. Gunakan tombol waifu.");
        assert_eq!(reaction_of(&r).as_deref(), Some("❓"));
        assert_eq!(patch_of(&r)["waifu"]["affection"], serde_json::json!(50));
    }

    #[test]
    fn nikah_menolak_affection_di_bawah_100() {
        let r = run(
            "waifuaction",
            &["nikah"],
            user_with(waifu_json("Yuki", "Common", 99, "genki")),
            vec![],
        );
        assert_eq!(
            text_of(&r),
            "⚠️ Affection belum 100! Jangan terburu-buru melamar!"
        );
        assert!(r.mutations().is_empty());
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn nikah_menolak_kalau_sudah_menikah() {
        let mut w = waifu_json("Yuki", "Common", 100, "genki");
        w["married"] = serde_json::json!(true);
        let r = run("waifuaction", &["nikah"], user_with(w), vec![]);
        assert_eq!(text_of(&r), "⚠️ Kalian kan sudah menikah!");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn nikah_sukses_memberi_dowry_dan_menautkan_pasangan() {
        let r = run(
            "waifuaction",
            &["nikah"],
            user_with(waifu_json("Yuki", "Common", 100, "genki")),
            vec![],
        );
        assert_eq!(
            text_of(&r),
            "💍 *KAMU RESMI MENIKAH DENGAN YUKI!* 💍\n\n\
             Sebagai dowry:\n- ⚡ 1000 Limit/Energi\n- 💰 20,000 Koin\n- ✨ 500 EXP\n\n\
             Status `.cekpacar` kini resmi berpasangan!"
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("💍"));
        let jid = waifu_jid("Yuki");
        assert_eq!(written_jids(&r), [ME.to_string(), jid.clone()]);
        let patch = patch_of(&r);
        assert_eq!(patch["energi"], serde_json::json!(1005));
        assert_eq!(patch["koin"], serde_json::json!(20100));
        assert_eq!(patch["exp"], serde_json::json!(500));
        assert_eq!(patch["waifu"]["married"], serde_json::json!(true));
        assert!(patch["waifu"]["marriedDate"].is_string());
        assert_eq!(patch["waifuStats"]["marriedCount"], serde_json::json!(1));
        assert_eq!(patch["fun"]["pasangan"], serde_json::json!(jid));
        let partner = patch_to(&r, &jid);
        assert_eq!(partner["fun"]["pasangan"], serde_json::json!(ME));
        assert_eq!(partner["name"], serde_json::json!("Yuki"));
    }

    #[test]
    fn nikah_owner_dan_koin_unlimited_tidak_ikut_ditulis() {
        let mut ctx = ctx_of(
            "waifuaction",
            &["nikah"],
            user_with(waifu_json("Yuki", "Common", 100, "genki")),
        );
        ctx.is_owner = true;
        ctx.user_data["koin"] = serde_json::json!(-1);
        let mut rng = scripted(vec![]);
        let r = dispatch(&ctx, &mut rng);
        let patch = patch_of(&r);
        assert!(patch.get("energi").is_none(), "owner = -1, tak menulis");
        assert!(patch.get("koin").is_none(), "koin -1 tak terbatas");
        assert_eq!(patch["exp"], serde_json::json!(500));
    }

    #[test]
    fn hadiah_menolak_di_bawah_100_dan_memberi_energi_di_atas() {
        let tolak = run(
            "waifuaction",
            &["hadiah"],
            user_with(waifu_json("Yuki", "Rare", 99, "genki")),
            vec![],
        );
        assert_eq!(
            text_of(&tolak),
            "⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!"
        );
        assert!(tolak.mutations().is_empty());

        let r = run(
            "waifuaction",
            &["hadiah"],
            user_with(waifu_json("Yuki", "Rare", 100, "genki")),
            vec![0.5],
        );
        assert_eq!(
            text_of(&r),
            "💝 *Yuki* memberimu hadiah!\nKamu mendapat ⚡ 500 Limit/Energi!"
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("💝"));
        assert_eq!(patch_of(&r)["energi"], serde_json::json!(505));
    }

    #[test]
    fn klaim_monolak_yang_belum_menikah_atau_sudah_klaim() {
        let tolak = run(
            "waifuaction",
            &["klaim"],
            user_with(waifu_json("Yuki", "Rare", 100, "genki")),
            vec![],
        );
        assert_eq!(
            text_of(&tolak),
            "⚠️ Hanya istri sah yang bisa klaim hadiah harian!"
        );
        assert!(tolak.mutations().is_empty());

        let mut w = waifu_json("Yuki", "Rare", 100, "genki");
        w["married"] = serde_json::json!(true);
        w["lastClaimDate"] = serde_json::json!(today_str(now_ms()));
        let sudah = run("waifuaction", &["klaim"], user_with(w), vec![]);
        assert_eq!(text_of(&sudah), "⚠️ Kamu sudah klaim hari ini! Coba lagi besok.");
        assert!(sudah.mutations().is_empty());
    }

    #[test]
    fn klaim_sukses_membayar_koin_exp_dan_menandai_tanggal() {
        let now = now_ms();
        let mut w = waifu_json("Yuki", "Rare", 100, "genki");
        w["married"] = serde_json::json!(true);
        w["marriedDate"] = serde_json::json!(iso_now(now));
        let r = run(
            "waifuaction",
            &["klaim"],
            user_with(w),
            vec![0.0, 0.0],
        );
        assert_eq!(
            text_of(&r),
            "🎁 *Yuki* memberi hadiah harian!\n💰 +6,000 Koin\n✨ +240 EXP"
        );
        assert_eq!(reaction_of(&r).as_deref(), Some("🎁"));
        let patch = patch_of(&r);
        assert_eq!(patch["koin"], serde_json::json!(6100), "100 + 6000");
        assert_eq!(patch["exp"], serde_json::json!(240));
        assert_eq!(
            patch["waifu"]["lastClaimDate"],
            serde_json::json!(today_str(now))
        );
    }

    #[test]
    fn klaim_hari_ke_7_dapat_bonus_milestone() {
        let now = now_ms();
        let mut w = waifu_json("Yuki", "Rare", 100, "genki");
        w["married"] = serde_json::json!(true);
        w["marriedDate"] = serde_json::json!(iso_now(now - 6 * DAY));
        let mut user = user_with(w);
        user["koin"] = serde_json::json!(0);
        let r = run("waifuaction", &["klaim"], user, vec![0.0, 0.0]);
        let text = text_of(&r);
        assert!(text.contains("💰 +6,000 Koin\n✨ +240 EXP"), "{text}");
        assert!(
            text.ends_with("\n🎉 *MILESTONE HARI KE-7!* Bonus +8,400 Koin!"),
            "{text}"
        );
        assert_eq!(
            patch_of(&r)["koin"],
            serde_json::json!(14400),
            "bonus 8400 lalu hadiah 6000"
        );
    }

    #[test]
    fn aksi_approach_memberi_affection_exp_koin_dan_event() {
        let r = run(
            "waifuaction",
            &["jalan_taman"],
            user_with(waifu_json("Yuki", "Rare", 50, "tsundere")),
            vec![0.5, 0.0, 0.1, 0.0],
        );
        let now = now_ms();
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"]["affection"], serde_json::json!(59));
        assert_eq!(patch["waifu"]["anger"], serde_json::json!(0));
        assert_eq!(patch["waifu"]["actionsToday"], serde_json::json!(1));
        assert_eq!(
            patch["waifu"]["lastActionDate"],
            serde_json::json!(today_str(now))
        );
        assert_eq!(patch["koin"], serde_json::json!(1100), "100 + 1000 dompet");
        assert_eq!(patch["exp"], serde_json::json!(55), "40 + 15 event");
        assert_eq!(reaction_of(&r).as_deref(), Some("✨"));

        let text = text_of(&r);
        assert!(
            text.starts_with(
                "🌳 Suasana taman sejuk. *Yuki* tersenyum riang sambil menggandeng lenganmu."
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "\n\n✨ *EVENT:* 💸 Kamu menemukan dompet di jalan dan mengembalikannya. Rezeki mengalir! (+1,000 koin)"
            ),
            "{text}"
        );
        assert!(
            text.contains("💞 *Affection berubah:* +9 (Total: 59/100)"),
            "{text}"
        );
        assert!(text.ends_with("✨ +40 EXP\n💰 +1,000 Koin"), "{text}");
    }

    #[test]
    fn affection_habis_membuat_waifu_pergi() {
        let r = run(
            "waifuaction",
            &["kafe_kopi"],
            user_with(waifu_json("Yuki", "Rare", 1, "himedere")),
            vec![0.0, 0.0, 0.7],
        );
        let text = text_of(&r);
        assert!(text.starts_with("💔 *YUKI MENINGGALKANMU!* 💔\n\n"), "{text}");
        assert!(text.ends_with("*(Ketik .gachawaifu untuk memulai ulang)*"), "{text}");
        assert_eq!(reaction_of(&r).as_deref(), Some("💔"));
        assert_eq!(written_jids(&r), [ME.to_string()]);
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"], Value::Null);
        assert_eq!(patch["exp"], serde_json::json!(40), "25 + 15 event");
        assert!(patch.get("koin").is_none(), "event salahpaham tidak memberi koin");
    }

    #[test]
    fn affection_habis_saat_menikah_ikut_melepas_pasangan() {
        let jid = waifu_jid("Yuki");
        let mut w = waifu_json("Yuki", "Rare", 1, "himedere");
        w["married"] = serde_json::json!(true);
        let mut user = user_with(w);
        user["fun"] = serde_json::json!({ "pasangan": jid });
        let r = run("waifuaction", &["kafe_kopi"], user, vec![0.0, 0.0, 0.7]);
        assert_eq!(written_jids(&r), [jid.clone(), ME.to_string()]);
        assert_eq!(
            patch_to(&r, &jid)["fun"]["pasangan"],
            serde_json::json!("")
        );
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"], Value::Null);
        assert_eq!(patch["fun"]["pasangan"], serde_json::json!(""));
    }

    #[test]
    fn aksi_married_mesra_memberi_koin_dan_exp() {
        let mut w = waifu_json("Yuki", "Rare", 90, "deredere");
        w["married"] = serde_json::json!(true);
        let r = run("waifuaction", &["mesra"], user_with(w), vec![0.5]);
        assert_eq!(reaction_of(&r).as_deref(), Some("❤️"));
        let patch = patch_of(&r);
        assert_eq!(patch["waifu"]["affection"], serde_json::json!(92));
        assert_eq!(patch["koin"], serde_json::json!(300), "100 + 200");
        assert_eq!(patch["exp"], serde_json::json!(35));
        let text = text_of(&r);
        assert!(
            text.contains("👩‍❤️‍👨 Kalian menghabiskan malam dengan mesra sebagai pasangan suami istri."),
            "{text}"
        );
        assert!(
            text.ends_with("💞 *Affection +2* (Total: 92/100)\n💰 +200 Koin\n✨ +35 EXP"),
            "{text}"
        );
    }
}
