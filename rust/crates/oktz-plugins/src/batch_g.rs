//! 30 plugin class A: 25 `cek*` (bentuk sama dengan `cek.rs`), `benefitpartner`,
//! `benefitowner`, `benefitpremium`, `templateplugin`, dan `asmaulhusna`.
//!
//! Catatan cakupan: dari 82 plugin `group` di `rest.json`, hanya 6 yang lolos
//! hard filter (tanpa db/sock/fs/http), dan 4 di antaranya butuh
//! `m.groupMetadata.participants` yang tidak ada di `Ctx` — jadi tidak ada
//! plugin `group` yang bisa diporthanding. `info`/`user`/`tools` yang lolos
//! filter juga butuh state/DB. Laporan lengkap ada di pesan commit.
//!
//! Yang di-share di sini: RNG + kerangka teks `cek*` lewat `cek_plugin!`.

use oktz_plugin_api::{all_plugins, plugin, Ctx, PluginMeta, Reply};
use rand::Rng as _;

/// `Math.floor(Math.random() * 101)` — inklusif 0..=100, bukan 0..100.
fn percent_101() -> u32 {
    rand::rng().random_range(0..101)
}

/// Indeks acak seragam di `[0, len)` — padanan
/// `Math.floor(Math.random() * len)`. Upper eksklusif.
fn random_index(len: usize) -> usize {
    rand::rng().random_range(0..len)
}

/// Bangun balasan `cek*` 4-tier.
///
/// `target` = mentions[0] atau sender. `label` = kata yang disisipkan
/// ("kekayaan"), `tiers` = 5 deskripsi berurutan dari ambang tertinggi.
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
    ($fn_name:ident, $name:literal, $alias:expr, $desc:literal, $usage:literal,
     $example:literal, $label:literal,
     [$t0:literal, $t1:literal, $t2:literal, $t3:literal, $t4:literal]) => {
        #[plugin(
            name = $name,
            alias = $alias,
            category = "cek",
            description = $desc,
            usage = $usage,
            example = $example,
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
    cekhoki,
    "cekhoki",
    ["hoki"],
    "Cek seberapa hoki kamu",
    ".cekhoki <nama>",
    ".cekhoki Budi",
    "kehokian",
    [
        "HOKI DEWA! Main gacha pasti menang! 🍀✨",
        "Hoki banget! 🎰",
        "Lumayan hoki 🍀",
        "Sedikit hoki 😊",
        "Sabar ya, lagi apes 😅"
    ]
);

cek_plugin!(
    cekimut,
    "cekimut",
    ["imut", "cute"],
    "Cek seberapa imut kamu",
    ".cekimut <nama>",
    ".cekimut Ani",
    "keimutan",
    [
        "IMUT BANGET! Kawaii~~ 🥺💕",
        "Imutnya kebangetan! 😍",
        "Lumayan imut~ 🌸",
        "Ada imutnya dikit 😊",
        "Mungkin cool bukan imut? 😎"
    ]
);

cek_plugin!(
    cekintrovert,
    "cekintrovert",
    ["introvert"],
    "Cek tingkat introvert kamu",
    ".cekintrovert <nama>",
    ".cekintrovert Budi",
    "keintrovertan",
    [
        "Rumah adalah surga! Stay home~ 🏠",
        "Social battery terbatas 🔋",
        "Ambivert, balance~ ⚖️",
        "Cukup social butterfly 🦋",
        "Extrovert mode ON! 🎉"
    ]
);

cek_plugin!(
    cekjahat,
    "cekjahat",
    ["evil"],
    "Cek seberapa jahat kamu",
    ".cekjahat <nama>",
    ".cekjahat Budi",
    "kejahatan",
    [
        "VILLAIN LEVEL! 😈👿",
        "Jahat banget! 💀",
        "Lumayan jahat 😏",
        "Sedikit nakal 😊",
        "Baik kok, gak jahat! 😇"
    ]
);

cek_plugin!(
    cekjomblo,
    "cekjomblo",
    ["jomblo", "single"],
    "Cek tingkat kejombloan kamu",
    ".cekjomblo <nama>",
    ".cekjomblo Budi",
    "kejombloan",
    [
        "Jomblo abadi! Single is happiness~ 💔😎",
        "Strong independent person! 💪",
        "MasihPDKT mode ON 😍",
        "Ada yang naksir kayaknya~ 👀",
        "Soon taken! 💕"
    ]
);

cek_plugin!(
    cekkaya,
    "cekkaya",
    ["kaya", "rich"],
    "Cek seberapa kaya kamu",
    ".cekkaya <nama>",
    ".cekkaya Budi",
    "kekayaan",
    [
        "Sultan! Crazy rich! 💎",
        "Tajir melintir! 💰",
        "Lumayan berada 💵",
        "Cukup lah buat hidup 😊",
        "Semangat nabung! 🙏"
    ]
);

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

cek_plugin!(
    ceklapar,
    "ceklapar",
    ["lapar", "hungry"],
    "Cek tingkat kelaparan kamu",
    ".ceklapar <nama>",
    ".ceklapar Budi",
    "kelaparan",
    [
        "LAPARRR! Makan sekarang! 🍔🍕🍜",
        "Perut keroncongan~ 😋",
        "Bisa lah ngemil 🍿",
        "Masih kenyang 😊",
        "Kekenyangan! 🤰"
    ]
);

cek_plugin!(
    cekmalas,
    "cekmalas",
    ["malas", "lazy"],
    "Cek seberapa malas kamu",
    ".cekmalas <nama>",
    ".cekmalas Budi",
    "kemalasan",
    [
        "SUPER MALAS! Raja rebahan! 🛏️",
        "Malas banget! 😴",
        "Lumayan malas 🥱",
        "Sedikit malas 😊",
        "Rajin banget! 💪"
    ]
);

cek_plugin!(
    cekmesum,
    "cekmesum",
    ["mesum"],
    "Cek seberapa mesum kamu",
    ".cekmesum <nama>",
    ".cekmesum Budi",
    "kemesuman",
    [
        "MESUM AKUT! Tobat mas! 😳🔞",
        "Mesum banget! 👀",
        "Lumayan mesum 😏",
        "Sedikit mesum 🙈",
        "Polos dan suci! 😇"
    ]
);

cek_plugin!(
    cekngantuk,
    "cekngantuk",
    ["ngantuk", "sleepy"],
    "Cek tingkat ngantuk kamu",
    ".cekngantuk <nama>",
    ".cekngantuk Budi",
    "kengantukan",
    [
        "ZZZZZ... Tidur sana! 😴💤",
        "Mata 5 watt nih~ 😪",
        "Agak ngantuk dikit 🥱",
        "Masih fresh! ☕",
        "Melek banget! Insomnia? 👀"
    ]
);

cek_plugin!(
    cekotaku,
    "cekotaku",
    ["otaku"],
    "Cek tingkat otaku kamu",
    ".cekotaku <nama>",
    ".cekotaku Budi",
    "keotakuan",
    [
        "SUGOI! True otaku desu! 🎌✨",
        "Weeb level tinggi~ 🇯🇵",
        "Casual anime enjoyer 📺",
        "Tau anime dikit-dikit 🤔",
        "Normie detected! 😂"
    ]
);

cek_plugin!(
    cekoverpower,
    "cekoverpower",
    ["overpower", "op"],
    "Cek tingkat overpower kamu",
    ".cekoverpower <nama>",
    ".cekoverpower Budi",
    "keoverpoweran",
    [
        "OVERPOWER BANGET! LEGEND! 👑🔥",
        "Kuat banget nih! 💪",
        "Lumayan strong~ 😎",
        "Biasa aja sih 🤔",
        "Masih perlu latihan 📝"
    ]
);

cek_plugin!(
    cekpelit,
    "cekpelit",
    ["pelit", "kikir"],
    "Cek seberapa pelit kamu",
    ".cekpelit <nama>",
    ".cekpelit Budi",
    "kepelitan",
    [
        "SUPER PELIT! Duit dijaga mati-matian! 💸",
        "Pelit banget! 🙊",
        "Lumayan pelit 😅",
        "Sedikit hemat 😊",
        "Dermawan banget! 🎁"
    ]
);

cek_plugin!(
    cekprocastinator,
    "cekprocastinator",
    ["procrastinator", "nunda"],
    "Cek tingkat suka menunda",
    ".cekprocastinator <nama>",
    ".cekprocastinator Budi",
    "keprocastinatoran",
    [
        "Deadline? Besok aja deh~ 😴",
        "Master procrastination! 🦥",
        "Kadang nunda, kadang rajin 😅",
        "Cukup produktif! 💪",
        "Disiplin tinggi! Salut! 🏆"
    ]
);

cek_plugin!(
    cekpsikopat,
    "cekpsikopat",
    ["psikopat", "psycho"],
    "Cek seberapa psikopat kamu",
    ".cekpsikopat <nama>",
    ".cekpsikopat Budi",
    "kepsikopatan",
    [
        "PSIKOPAT AKUT! Jauhi! 😈",
        "Hati-hati sama orang ini 👀",
        "Ada sisi gelapnya 🌑",
        "Sedikit misterius 🤔",
        "Normal dan baik hati 😇"
    ]
);

cek_plugin!(
    cekrezeki,
    "cekrezeki",
    ["rezeki", "fortune"],
    "Cek tingkat rezeki kamu hari ini",
    ".cekrezeki <nama>",
    ".cekrezeki Budi",
    "kerezekian",
    [
        "Rezeki melimpah! Jackpot! 💰🎉",
        "Rezeki lancar hari ini~ 💵",
        "Rezeki cukup, bersyukurlah 🙏",
        "Rezeki pas-pasan 😅",
        "Sabar ya, rezeki akan datang~ 🫂"
    ]
);

cek_plugin!(
    ceksabar,
    "ceksabar",
    ["sabar", "patience"],
    "Cek tingkat kesabaran kamu",
    ".ceksabar <nama>",
    ".ceksabar Budi",
    "kesabaran",
    [
        "Sabar level dewa! Zen master~ 🧘",
        "Sangat sabar! Terpuji 👏",
        "Cukup sabar 😊",
        "Kadang emosian dikit 😅",
        "Gampang marah nih... 😤"
    ]
);

cek_plugin!(
    ceksetia,
    "ceksetia",
    ["setia", "loyal"],
    "Cek tingkat kesetiaan kamu",
    ".ceksetia <nama>",
    ".ceksetia Budi",
    "kesetiaan",
    [
        "Setia sampai mati! 💍💕",
        "Sangat setia dan tulus! ❤️",
        "Cukup setia~ 😊",
        "Hmm... kadang goyah 😅",
        "Playboy/Playgirl mode? 😏"
    ]
);

cek_plugin!(
    ceksexy,
    "ceksexy",
    ["sexy", "hot"],
    "Cek seberapa sexy kamu",
    ".ceksexy <nama>",
    ".ceksexy Budi",
    "kesexyan",
    [
        "SEXY ABIS! 🔥🔥🔥",
        "Hot banget! 😏",
        "Lumayan menggoda~ 😊",
        "Biasa aja sih 🙂",
        "Mungkin cute bukan sexy 😅"
    ]
);

cek_plugin!(
    ceksial,
    "ceksial",
    ["sial", "apes"],
    "Cek seberapa sial kamu",
    ".ceksial <nama>",
    ".ceksial Budi",
    "kesialan",
    [
        "SIAL BANGET! Mending di rumah aja! 😭",
        "Lagi apes nih~ 😢",
        "Lumayan sial 😓",
        "Sedikit sial 😕",
        "Gak sial, hoki dong! 🍀"
    ]
);

cek_plugin!(
    cekwibu,
    "cekwibu",
    ["wibu", "weeb"],
    "Cek seberapa wibu kamu",
    ".cekwibu <nama>",
    ".cekwibu Budi",
    "kewibuan",
    [
        "WIBU SEJATI! Ara ara~ 🎌",
        "Wibu parah! Kimochi~ 😍",
        "Lumayan wibu 🌸",
        "Sedikit wibu 😊",
        "Bukan wibu, normal! 😎"
    ]
);

cek_plugin!(
    cekyandere,
    "cekyandere",
    ["yandere"],
    "Cek tingkat yandere kamu",
    ".cekyandere <nama>",
    ".cekyandere Budi",
    "keyanderean",
    [
        "Kamu milikku selamanya~ 🔪💕",
        "Jangan dekati dia ya... 👁️",
        "Overprotective sedikit~ 🫂",
        "Agak posesif 😅",
        "Normal kok, santai~ 😊"
    ]
);

// ------------------------------------------------------------------- cekjodoh

/// Tiga nilai yang dirender `cekjodoh` — dipisah dari `cek_reply` karena
/// input-nya bukan mention: `cekjodoh` menolak kalau teks tidak punya 2 nama.
fn jodoh_desc(percent: u32) -> &'static str {
    if percent >= 90 {
        "Jodoh banget! Langsung nikah aja! 💍"
    } else if percent >= 70 {
        "Cocok banget! 💕"
    } else if percent >= 50 {
        "Lumayan cocok~ 😊"
    } else if percent >= 30 {
        "Hmm, perlu usaha lebih 🤔"
    } else {
        "Mungkin cari yang lain? 😅"
    }
}

#[plugin(
    name = "cekjodoh",
    alias = [],
    category = "cek",
    description = "Cek kecocokan jodoh",
    usage = ".cekjodoh <nama1> & <nama2>",
    example = ".cekjodoh Budi & Ani",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekjodoh(ctx: Ctx) -> Reply {
    // JS: `m.text?.trim().split(/[&,]/).map(trim).filter(Boolean)` — nama yang
    // dipisah tidak pernah dipakai lagi, handler cuma butuh jumlahnya.
    let name_count = ctx
        .text
        .trim()
        .split(['&', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .count();

    if name_count < 2 {
        return Reply::text(format!(
            "💕 *ᴄᴇᴋ ᴊᴏᴅᴏʜ*\n\n> Masukkan 2 nama!\n\n> Contoh: {}cekjodoh Budi & Ani",
            ctx.prefix
        ));
    }

    let percent = percent_101();
    let mentioned = ctx.mention_or_sender().to_string();
    let number = mentioned.split('@').next().unwrap_or(&mentioned);
    let desc = jodoh_desc(percent);

    let txt = if mentioned == ctx.sender {
        format!("Hai @{number}\n    \nTingkat kejodohan kamu *{percent}%*\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kejodohan @{number} yak? \n    \nTingkat kejodohan dia sebesar *{percent}%*\n```{desc}```"
        )
    };

    Reply::Text {
        text: txt,
        mentions: vec![mentioned],
    }
}

// ------------------------------------------------------------ cekkepribadian

/// `(type, title, desc)` — urutan persis seperti `personalities` di JS;
/// indeks hasil undian bergantung urutan ini.
const PERSONALITIES: [(&str, &str, &str); 16] = [
    (
        "INTJ",
        "The Architect",
        "Visioner, strategis, dan independen",
    ),
    ("INTP", "The Logician", "Analitis, inovatif, dan ingin tahu"),
    (
        "ENTJ",
        "The Commander",
        "Tegas, ambisius, dan pemimpin alami",
    ),
    (
        "ENTP",
        "The Debater",
        "Cerdas, penasaran, dan suka tantangan",
    ),
    (
        "INFJ",
        "The Advocate",
        "Idealis, bijaksana, dan penuh empati",
    ),
    ("INFP", "The Mediator", "Kreatif, idealis, dan setia"),
    (
        "ENFJ",
        "The Protagonist",
        "Karismatik, inspiratif, dan peduli",
    ),
    ("ENFP", "The Campaigner", "Antusias, kreatif, dan sosial"),
    (
        "ISTJ",
        "The Logistician",
        "Bertanggung jawab, praktis, dan teliti",
    ),
    ("ISFJ", "The Defender", "Setia, suportif, dan reliable"),
    (
        "ESTJ",
        "The Executive",
        "Terorganisir, tegas, dan tradisional",
    ),
    ("ESFJ", "The Consul", "Peduli, sosial, dan loyal"),
    ("ISTP", "The Virtuoso", "Fleksibel, observan, dan praktis"),
    ("ISFP", "The Adventurer", "Artistik, sensitif, dan spontan"),
    (
        "ESTP",
        "The Entrepreneur",
        "Energik, perceptive, dan berani",
    ),
    ("ESFP", "The Entertainer", "Spontan, energik, dan fun"),
];

#[plugin(
    name = "cekkepribadian",
    alias = ["kepribadian", "personality"],
    category = "cek",
    description = "Cek kepribadian kamu",
    usage = ".cekkepribadian <nama>",
    example = ".cekkepribadian Budi",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekkepribadian(ctx: Ctx) -> Reply {
    let mentioned = ctx.mention_or_sender().to_string();
    let number = mentioned.split('@').next().unwrap_or(&mentioned);
    let (p_type, p_title, p_desc) = PERSONALITIES[random_index(PERSONALITIES.len())];

    let txt = if mentioned == ctx.sender {
        format!(
            "Hai @{number}\n    \nTingkat kepribadian kamu *{p_type} - {p_title}*\n```{p_desc}```"
        )
    } else {
        format!(
            "Kamu ingin ngecek kepribadian @{number} yak? \n    \nKepribadian dia adalah *{p_type} - {p_title}*\n```{p_desc}```"
        )
    };

    Reply::Text {
        text: txt,
        mentions: vec![mentioned],
    }
}

// ------------------------------------------------------------- benefitpartner

// `ponytail:` `config.bot.name` / `config.owner.name` di-hardcode dari
// `config.js` karena `Ctx` tidak membawa config. Ganti dengan config runtime
// kalau router someday menyediakannya.
const BOT_NAME: &str = "Oktz Bot";
const OWNER_NAME: &str = "Oktz";

#[plugin(
    name = "benefitpartner",
    alias = ["partnerbenefits", "keuntunganpartner"],
    category = "info",
    description = "Lihat keuntungan menjadi partner bot",
    usage = ".benefitpartner",
    example = ".benefitpartner",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn benefitpartner(ctx: Ctx) -> Reply {
    let prefix = ctx.prefix.as_str();

    let mut txt = String::from("🤝 *BENEFIT PARTNER*\n\n");
    txt.push_str(&format!("Keuntungan menjadi partner {BOT_NAME}:\n\n"));

    txt.push_str("🔓 *Akses Fitur*\n");
    txt.push_str("├ Semua fitur premium terbuka\n");
    txt.push_str("├ Energi & koin unlimited\n");
    txt.push_str("├ Akses command owner tertentu\n");
    txt.push_str("└ Prioritas support\n\n");

    txt.push_str("📦 *Panel Pterodactyl*\n");
    txt.push_str("├ Bisa create server sendiri\n");
    txt.push_str("├ Akses panel management\n");
    txt.push_str("└ Bisa jualan panel (reseller)\n\n");

    txt.push_str("💎 *Bonus*\n");
    txt.push_str("├ +200.000 EXP saat aktivasi\n");
    txt.push_str("├ +20.000 Koin saat aktivasi\n");
    txt.push_str("├ Badge partner di profil\n");
    txt.push_str("└ Akses early feature\n\n");

    txt.push_str("💰 *Cara Jadi Partner*\n");
    txt.push_str(&format!("├ Hubungi owner: {OWNER_NAME}\n"));
    txt.push_str("├ Durasi: 30/60/90 hari\n");
    txt.push_str(&format!("└ Command: `{prefix}addpartner` (owner only)\n\n"));

    txt.push_str("📋 *Command Partner*\n");
    txt.push_str(&format!("├ `{prefix}cekpartner` — Cek status partner\n"));
    txt.push_str(&format!("├ `{prefix}cekprem` — Cek status premium\n"));
    txt.push_str(&format!("├ `{prefix}cekowner` — Cek role user\n"));
    txt.push_str(&format!("└ `{prefix}listpartner` — Daftar partner\n\n"));

    txt.push_str("> _Hubungi owner untuk info lebih lanjut_");

    Reply::text(txt)
}

// ------------------------------------------------------------ benefitowner

/// `config.command?.prefix || '.'` — `config.command` tidak ada di `config.js`
/// untuk field prefix, jadi yang tampil selalu `.`.
const CONFIG_PREFIX: &str = ".";

/// Ubah nama-nama command jadi baris `• *.<name>*`, buang duplikat, sort.
///
/// JS: `seen` Set + `commandList.sort()` (default sort = lexicographic).
/// Rust `String` sort = byte order; sama untuk nama command ASCII.
fn command_lines(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for name in names {
        if name.is_empty() || seen.contains(&name) {
            continue;
        }
        seen.push(name);
    }
    seen.sort();
    seen.iter()
        .map(|n| format!("• *{CONFIG_PREFIX}{n}*"))
        .collect()
}

/// Baris daftar command dari registry yang lolos `pred` — padanan
/// `getAllPlugins().filter(pred)` di JS.
fn registry_command_lines(pred: impl Fn(&PluginMeta) -> bool) -> Vec<String> {
    command_lines(
        all_plugins()
            .iter()
            .filter(|e| pred(&e.meta))
            .map(|e| e.meta.name.clone()),
    )
}

#[plugin(
    name = "benefitowner",
    alias = ["ownerbenefits", "ownerfitur"],
    category = "main",
    description = "Lihat penjelasan dan daftar fitur khusus Owner",
    usage = ".benefitowner",
    example = "",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn benefitowner(_ctx: Ctx) -> Reply {
    Reply::text(owner_message(&registry_command_lines(|m| {
        m.is_owner && m.is_enabled
    })))
}

/// Badan pesan `benefitowner` — dipisah dari handler supaya cabang daftar
/// kosong / terisi bisa diuji tanpa mengubah registry.
fn owner_message(command_list: &[String]) -> String {
    let total_commands = command_list.len();

    let mut message = String::new();
    message.push_str("👑 *ᴀᴘᴀ ɪᴛᴜ ᴏᴡɴᴇʀ?*\n\n");
    message.push_str(
        "Owner adalah *pemilik bot* yang memiliki akses penuh ke semua fitur dan kontrol sistem.\n\n",
    );
    message.push_str("╭┈┈⬡「 🔐 *ᴋᴇɪꜱᴛɪᴍᴇᴡᴀᴀɴ ᴏᴡɴᴇʀ* 」\n");
    message.push_str("┃ ✦ ```Akses semua command tanpa batasan```\n");
    message.push_str("┃ ✦ ```Limit tidak terbatas (-1)```\n");
    message.push_str("┃ ✦ ```Bypass semua cooldown```\n");
    message.push_str("┃ ✦ ```Kontrol penuh sistem bot```\n");
    message.push_str("┃ ✦ ```Manajemen user & group```\n");
    message.push_str("┃ ✦ ```Akses panel & server```\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 ⚙️ *ᴄᴀʀᴀ ᴋᴇʀᴊᴀ* 」\n");
    message.push_str("┃ `Owner ditambahkan melalui:`\n");
    message.push_str(&format!("┃ • ```{CONFIG_PREFIX}addowner <nomor>```\n"));
    message.push_str("┃ • Atau langsung di config.js\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴄᴏᴍᴍᴀɴᴅ ᴏᴡɴᴇʀ* 」\n");
    message.push_str(&format!("┃ `Total: {total_commands} command`\n"));
    message.push_str("┃\n");
    // JS: `commandList.map(cmd => `┃ ${cmd}`).join('\n')` — tiap baris
    // dapat awalan "┃ ", bukan cuma dipisah newline.
    let listed: Vec<String> = command_list.iter().map(|c| format!("┃ {c}")).collect();
    message.push_str(&listed.join("\n"));
    message.push_str("\n╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("> Hubungi owner untuk mendapatkan akses!");
    message
}

// ---------------------------------------------------------- benefitpremium

// `config.limits` tidak ada di `config.js`, jadi `|| 25` dan `|| 100` yang
// selalu dipakai — angka di bawah bukan tebakan, itu hasil runtime JS.
const LIMIT_DEFAULT: u32 = 25;
const LIMIT_PREMIUM: u32 = 100;

// `ponytail:` `config.owner.number` di-hardcode dari `config.js`.
const OWNER_NUMBERS: [&str; 1] = ["6285608953677"];

#[plugin(
    name = "benefitpremium",
    alias = ["premiumbenefits", "premiumfitur", "benefitprem"],
    category = "main",
    description = "Lihat penjelasan dan daftar fitur khusus Premium",
    usage = ".benefitpremium",
    example = "",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn benefitpremium(_ctx: Ctx) -> Reply {
    Reply::text(premium_message(&registry_command_lines(|m| {
        m.is_premium && m.is_enabled
    })))
}

/// Badan pesan `benefitpremium` — cabang `total > 0`vs`total == 0` di sini,
/// bukan di handler, supaya kedua-duanya bisa diuji.
fn premium_message(command_list: &[String]) -> String {
    let total_commands = command_list.len();

    let mut message = String::new();
    message.push_str("⭐ *ᴀᴘᴀ ɪᴛᴜ ᴘʀᴇᴍɪᴜᴍ?*\n\n");
    message.push_str(
        "Premium adalah *user berbayar* yang mendapatkan akses ke fitur eksklusif dan keuntungan lebih.\n\n",
    );
    message.push_str("╭┈┈⬡「 💎 *ᴋᴇᴜɴᴛᴜɴɢᴀɴ ᴘʀᴇᴍɪᴜᴍ* 」\n");
    message.push_str(&format!(
        "┃ ✦ ```Limit harian: {LIMIT_PREMIUM}x (vs {LIMIT_DEFAULT}x user biasa)```\n"
    ));
    message.push_str("┃ ✦ ```Cooldown lebih rendah```\n");
    message.push_str("┃ ✦ ```Akses fitur eksklusif```\n");
    message.push_str("┃ ✦ ```Prioritas response```\n");
    message.push_str("┃ ✦ ```No watermark di beberapa fitur```\n");
    message.push_str("┃ ✦ ```Support prioritas```\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 ⚙️ *ᴄᴀʀᴀ ᴍᴇɴᴅᴀᴘᴀᴛᴋᴀɴ* 」\n");
    message.push_str("┃ `Premium didapatkan melalui:`\n");
    message.push_str("┃ • Hubungi owner bot\n");
    message.push_str(&format!(
        "┃ • ```{CONFIG_PREFIX}addprem <nomor> <durasi>```\n"
    ));
    message.push_str("┃ • Contoh: .addprem 628xxx 30d\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴄᴏᴍᴍᴀɴᴅ ᴘʀᴇᴍɪᴜᴍ* 」\n");
    message.push_str(&format!("┃ `Total: {total_commands} command`\n"));
    message.push_str("┃\n");
    if total_commands > 0 {
        // Sama seperti benefitowner: `┃ ` per baris.
        let listed: Vec<String> = command_list.iter().map(|c| format!("┃ {c}")).collect();
        message.push_str(&listed.join("\n"));
    } else {
        message.push_str("┃ Semua command bisa diakses user biasa");
    }
    message.push_str("\n╰┈┈┈┈┈┈┈┈⬡\n\n");
    // Spasi penutup milik `join()` di dalam literal JS, bukan bagian output —
    // balasannya TIDAK berakhir dengan spasi.
    let wa_links: Vec<String> = OWNER_NUMBERS
        .iter()
        .map(|n| format!("- wa.me/{n}"))
        .collect();
    message.push_str(&format!(
        "Mau Upgrade? silahkan hubungi owner bot\n{}",
        wa_links.join("\n")
    ));
    message
}

// ---------------------------------------------------------- templateplugin

/// Template JS yang dibungkus ```` ``` ````. Diawali dan diakhiri newline
/// persis seperti literal di `templateplugin.js`.
const TEMPLATE: &str = r#"
const pluginConfig = {
    name: 'example',
    alias: ['ex'],
    category: 'general',
    description: 'Example plugin',
    usage: '.example',
    example: '.example',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 3,
    energi: 1,
    isEnabled: true
}
async function handler(m, { sock }) {
    try {
        await m.reply('This is an example plugin!')
    } catch (error) {
        console.error('Example Plugin Error:', error)
        await m.reply('❌ *GAGAL*\n\n> ' + error.message)
    }
}
export { pluginConfig as config, handler }
"#;

#[plugin(
    name = "templateplugin",
    alias = ["tplplugin", "plugin-template"],
    category = "owner",
    description = "Generate plugin template (Owner Only)",
    usage = ".templateplugin",
    example = ".templateplugin",
    cooldown = 0,
    energi = 0,
    is_owner = true,
    is_private = false
)]
pub fn templateplugin(_ctx: Ctx) -> Reply {
    Reply::text(format!("```{TEMPLATE}```"))
}

// ------------------------------------------------------------- asmaulhusna

/// Satu entri `src/data/asmaulhusna.json` — disalin persis, urutan asli.
#[derive(Clone, Copy)]
struct AsmaulHusna {
    index: u32,
    latin: &'static str,
    arabic: &'static str,
    translation_id: &'static str,
    translation_en: &'static str,
}

const ASMAUL_HUSNA: [AsmaulHusna; 99] = [
    AsmaulHusna {
        index: 1,
        latin: "Ar Rahman",
        arabic: "الرَّحْمَنُ",
        translation_id: "Yang Memiliki Mutlak sifat Pemurah",
        translation_en: "The All Beneficent",
    },
    AsmaulHusna {
        index: 2,
        latin: "Ar Rahiim",
        arabic: "الرَّحِيمُ",
        translation_id: "Yang Memiliki Mutlak sifat Penyayang",
        translation_en: "The Most Merciful",
    },
    AsmaulHusna {
        index: 3,
        latin: "Al Malik",
        arabic: "الْمَلِكُ",
        translation_id: "Yang Memiliki Mutlak sifat Merajai/Memerintah",
        translation_en: "The King, The Sovereign",
    },
    AsmaulHusna {
        index: 4,
        latin: "Al Quddus",
        arabic: "الْقُدُّوسُ",
        translation_id: "Yang Memiliki Mutlak sifat Suci",
        translation_en: "The Most Holy",
    },
    AsmaulHusna {
        index: 5,
        latin: "As Salaam",
        arabic: "السَّلاَمُ",
        translation_id: "Yang Memiliki Mutlak sifat Memberi Kesejahteraan",
        translation_en: "Peace and Blessing",
    },
    AsmaulHusna {
        index: 6,
        latin: "Al Mu’min",
        arabic: "الْمُؤْمِنُ",
        translation_id: "Yang Memiliki Mutlak sifat Memberi Keamanan",
        translation_en: "The Guarantor",
    },
    AsmaulHusna {
        index: 7,
        latin: "Al Muhaimin",
        arabic: "الْمُهَيْمِنُ",
        translation_id: "Yang Memiliki Mutlak sifat Pemelihara",
        translation_en: "The Guardian, the Preserver",
    },
    AsmaulHusna {
        index: 8,
        latin: "Al ‘Aziiz",
        arabic: "الْعَزِيزُ",
        translation_id: "Yang Memiliki Mutlak Kegagahan",
        translation_en: "The Almighty, the Self Sufficient",
    },
    AsmaulHusna {
        index: 9,
        latin: "Al Jabbar",
        arabic: "الْجَبَّارُ",
        translation_id: "Yang Memiliki Mutlak sifat Perkasa",
        translation_en: "The Powerful, the Irresistible",
    },
    AsmaulHusna {
        index: 10,
        latin: "Al Mutakabbir",
        arabic: "الْمُتَكَبِّرُ",
        translation_id: "Yang Memiliki Mutlak sifat Megah,Yang Memiliki Kebesaran",
        translation_en: "The Tremendous",
    },
    AsmaulHusna {
        index: 11,
        latin: "Al Khaliq",
        arabic: "الْخَالِقُ",
        translation_id: "Yang Memiliki Mutlak sifat Pencipta",
        translation_en: "The Creator",
    },
    AsmaulHusna {
        index: 12,
        latin: "Al Baari’",
        arabic: "الْبَارِئُ",
        translation_id:
            "Yang Memiliki Mutlak sifat Yang Melepaskan(Membuat, Membentuk, Menyeimbangkan)",
        translation_en: "The Maker",
    },
    AsmaulHusna {
        index: 13,
        latin: "Al Mushawwir",
        arabic: "الْمُصَوِّرُ",
        translation_id: "Yang Memiliki Mutlak sifat YangMembentuk Rupa (makhluknya)",
        translation_en: "The Fashioner of Forms",
    },
    AsmaulHusna {
        index: 14,
        latin: "Al Ghaffaar",
        arabic: "الْغَفَّارُ",
        translation_id: "Yang Memiliki Mutlak sifat Pengampun",
        translation_en: "The Ever Forgiving",
    },
    AsmaulHusna {
        index: 15,
        latin: "Al Qahhaar",
        arabic: "الْقَهَّارُ",
        translation_id: "Yang Memiliki Mutlak sifat Memaksa",
        translation_en: "The All Compelling Subduer",
    },
    AsmaulHusna {
        index: 16,
        latin: "Al Wahhaab",
        arabic: "الْوَهَّابُ",
        translation_id: "Yang Memiliki Mutlak sifat Pemberi Karunia",
        translation_en: "The Bestower",
    },
    AsmaulHusna {
        index: 17,
        latin: "Ar Razzaaq",
        arabic: "الرَّزَّاقُ",
        translation_id: "Yang Memiliki Mutlak sifat Pemberi Rejeki",
        translation_en: "The Ever Providing",
    },
    AsmaulHusna {
        index: 18,
        latin: "Al Fattaah",
        arabic: "الْفَتَّاحُ",
        translation_id: "Yang Memiliki Mutlak sifat Pembuka Rahmat",
        translation_en: "The Opener, the Victory Giver",
    },
    AsmaulHusna {
        index: 19,
        latin: "Al ‘Aliim",
        arabic: "اَلْعَلِيْمُ",
        translation_id: "Yang Memiliki Mutlak sifatMengetahui (Memiliki Ilmu)",
        translation_en: "The All Knowing, the Omniscient",
    },
    AsmaulHusna {
        index: 20,
        latin: "Al Qaabidh",
        arabic: "الْقَابِضُ",
        translation_id: "Yang Memiliki Mutlak sifat YangMenyempitkan (makhluknya)",
        translation_en: "The Restrainer, the Straightener",
    },
    AsmaulHusna {
        index: 21,
        latin: "Al Baasith",
        arabic: "الْبَاسِطُ",
        translation_id: "Yang Memiliki Mutlak sifat YangMelapangkan (makhluknya)",
        translation_en: "The Expander, the Munificent",
    },
    AsmaulHusna {
        index: 22,
        latin: "Al Khaafidh",
        arabic: "الْخَافِضُ",
        translation_id: "Yang Memiliki Mutlak sifat YangMerendahkan (makhluknya)",
        translation_en: "The Abaser",
    },
    AsmaulHusna {
        index: 23,
        latin: "Ar Raafi’",
        arabic: "الرَّافِعُ",
        translation_id: "Yang Memiliki Mutlak sifat YangMeninggikan (makhluknya)",
        translation_en: "The Exalter",
    },
    AsmaulHusna {
        index: 24,
        latin: "Al Mu’izz",
        arabic: "الْمُعِزُّ",
        translation_id: "Yang Memiliki Mutlak sifat YangMemuliakan (makhluknya)",
        translation_en: "The Giver of Honor",
    },
    AsmaulHusna {
        index: 25,
        latin: "Al Mudzil",
        arabic: "المُذِلُّ",
        translation_id: "Yang Memiliki Mutlak sifatYang Menghinakan (makhluknya)",
        translation_en: "The Giver of Dishonor",
    },
    AsmaulHusna {
        index: 26,
        latin: "Al Samii’",
        arabic: "السَّمِيعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mendengar",
        translation_en: "The All Hearing",
    },
    AsmaulHusna {
        index: 27,
        latin: "Al Bashiir",
        arabic: "الْبَصِيرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Melihat",
        translation_en: "The All Seeing",
    },
    AsmaulHusna {
        index: 28,
        latin: "Al Hakam",
        arabic: "الْحَكَمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Menetapkan",
        translation_en: "The Judge, the Arbitrator",
    },
    AsmaulHusna {
        index: 29,
        latin: "Al ‘Adl",
        arabic: "الْعَدْلُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Adil",
        translation_en: "The Utterly Just",
    },
    AsmaulHusna {
        index: 30,
        latin: "Al Lathiif",
        arabic: "اللَّطِيفُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Lembut",
        translation_en: "The Subtly Kind",
    },
    AsmaulHusna {
        index: 31,
        latin: "Al Khabiir",
        arabic: "الْخَبِيرُ",
        translation_id: "Yang Memiliki Mutlak sifatMaha Mengetahui Rahasia",
        translation_en: "The All Aware",
    },
    AsmaulHusna {
        index: 32,
        latin: "Al Haliim",
        arabic: "الْحَلِيمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Penyantun",
        translation_en: "The Forbearing, the Indulgent",
    },
    AsmaulHusna {
        index: 33,
        latin: "Al ‘Azhiim",
        arabic: "الْعَظِيمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Agung",
        translation_en: "The Magnificent, the Infinite",
    },
    AsmaulHusna {
        index: 34,
        latin: "Al Ghafuur",
        arabic: "الْغَفُورُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pengampun",
        translation_en: "The All Forgiving",
    },
    AsmaulHusna {
        index: 35,
        latin: "As Syakuur",
        arabic: "الشَّكُورُ",
        translation_id: "Yang Memiliki Mutlak sifat MahaPembalas Budi (Menghargai)",
        translation_en: "The Grateful",
    },
    AsmaulHusna {
        index: 36,
        latin: "Al ‘Aliy",
        arabic: "الْعَلِيُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Tinggi",
        translation_en: "The Sublimely Exalted",
    },
    AsmaulHusna {
        index: 37,
        latin: "Al Kabiir",
        arabic: "الْكَبِيرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Besar",
        translation_en: "The Great",
    },
    AsmaulHusna {
        index: 38,
        latin: "Al Hafizh",
        arabic: "الْحَفِيظُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Menjaga",
        translation_en: "The Preserver",
    },
    AsmaulHusna {
        index: 39,
        latin: "Al Muqiit",
        arabic: "المُقيِت",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pemberi Kecukupan",
        translation_en: "The Nourisher",
    },
    AsmaulHusna {
        index: 40,
        latin: "Al Hasiib",
        arabic: "الْحسِيبُ",
        translation_id: "Yang Memiliki Mutlak sifat MahaMembuat Perhitungan",
        translation_en: "The Reckoner",
    },
    AsmaulHusna {
        index: 41,
        latin: "Al Jaliil",
        arabic: "الْجَلِيلُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mulia",
        translation_en: "The Majestic",
    },
    AsmaulHusna {
        index: 42,
        latin: "Al Kariim",
        arabic: "الْكَرِيمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pemurah",
        translation_en: "The Bountiful, the Generous",
    },
    AsmaulHusna {
        index: 43,
        latin: "Ar Raqiib",
        arabic: "الرَّقِيبُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mengawasi",
        translation_en: "The Watchful",
    },
    AsmaulHusna {
        index: 44,
        latin: "Al Mujiib",
        arabic: "الْمُجِيبُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mengabulkan",
        translation_en: "The Responsive, the Answerer",
    },
    AsmaulHusna {
        index: 45,
        latin: "Al Waasi’",
        arabic: "الْوَاسِعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Luas",
        translation_en: "The Vast, the All Encompassing",
    },
    AsmaulHusna {
        index: 46,
        latin: "Al Hakiim",
        arabic: "الْحَكِيمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maka Bijaksana",
        translation_en: "The Wise",
    },
    AsmaulHusna {
        index: 47,
        latin: "Al Waduud",
        arabic: "الْوَدُودُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pencinta",
        translation_en: "The Loving, the Kind One",
    },
    AsmaulHusna {
        index: 48,
        latin: "Al Majiid",
        arabic: "الْمَجِيدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mulia",
        translation_en: "The All Glorious",
    },
    AsmaulHusna {
        index: 49,
        latin: "Al Baa’its",
        arabic: "الْبَاعِثُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Membangkitkan",
        translation_en: "The Raiser of the Dead",
    },
    AsmaulHusna {
        index: 50,
        latin: "As Syahiid",
        arabic: "الشَّهِيدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Menyaksikan",
        translation_en: "The Witness",
    },
    AsmaulHusna {
        index: 51,
        latin: "Al Haqq",
        arabic: "الْحَقُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Benar",
        translation_en: "The Truth, the Real",
    },
    AsmaulHusna {
        index: 52,
        latin: "Al Wakiil",
        arabic: "الْوَكِيلُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memelihara",
        translation_en: "The Trustee, the Dependable",
    },
    AsmaulHusna {
        index: 53,
        latin: "Al Qawiyyu",
        arabic: "الْقَوِيُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Kuat",
        translation_en: "The Strong",
    },
    AsmaulHusna {
        index: 54,
        latin: "Al Matiin",
        arabic: "الْمَتِينُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Kokoh",
        translation_en: "The Firm, the Steadfast",
    },
    AsmaulHusna {
        index: 55,
        latin: "Al Waliyy",
        arabic: "الْوَلِيُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Melindungi",
        translation_en: "The Protecting Friend, Patron, and Helper",
    },
    AsmaulHusna {
        index: 56,
        latin: "Al Hamiid",
        arabic: "الْحَمِيدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Terpuji",
        translation_en: "The All Praiseworthy",
    },
    AsmaulHusna {
        index: 57,
        latin: "Al Mushii",
        arabic: "الْمُحْصِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mengkalkulasi",
        translation_en: "The Accounter, the Numberer of All",
    },
    AsmaulHusna {
        index: 58,
        latin: "Al Mubdi’",
        arabic: "الْمُبْدِئُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memulai",
        translation_en: "The Producer, Originator, and Initiator of all",
    },
    AsmaulHusna {
        index: 59,
        latin: "Al Mu’iid",
        arabic: "الْمُعِيدُ",
        translation_id: "Yang Memiliki Mutlak sifat MahaMengembalikan Kehidupan",
        translation_en: "The Reinstater Who Brings Back All",
    },
    AsmaulHusna {
        index: 60,
        latin: "Al Muhyii",
        arabic: "الْمُحْيِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Menghidupkan",
        translation_en: "The Giver of Life",
    },
    AsmaulHusna {
        index: 61,
        latin: "Al Mumiitu",
        arabic: "اَلْمُمِيتُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mematikan",
        translation_en: "The Bringer of Death, the Destroyer",
    },
    AsmaulHusna {
        index: 62,
        latin: "Al Hayyu",
        arabic: "الْحَيُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Hidup",
        translation_en: "The Ever Living",
    },
    AsmaulHusna {
        index: 63,
        latin: "Al Qayyuum",
        arabic: "الْقَيُّومُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mandiri",
        translation_en: "The Self Subsisting Sustainer of All",
    },
    AsmaulHusna {
        index: 64,
        latin: "Al Waajid",
        arabic: "الْوَاجِدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Penemu",
        translation_en: "The Perceiver, the Finder, the Unfailing",
    },
    AsmaulHusna {
        index: 65,
        latin: "Al Maajid",
        arabic: "الْمَاجِدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mulia",
        translation_en: "The Illustrious, the Magnificent",
    },
    AsmaulHusna {
        index: 66,
        latin: "Al Wahiid",
        arabic: "الْواحِدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Tunggal",
        translation_en: "The One, The Unique, Manifestation of Unity",
    },
    AsmaulHusna {
        index: 67,
        latin: "Al ‘Ahad",
        arabic: "اَلاَحَدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Esa",
        translation_en: "The One, the All Inclusive, the Indivisible",
    },
    AsmaulHusna {
        index: 68,
        latin: "As Shamad",
        arabic: "الصَّمَدُ",
        translation_id: "Yang Memiliki Mutlak sifat MahaDibutuhkan, Tempat Meminta",
        translation_en:
            "The Self Sufficient, the Impregnable,the Eternally Besought of All, the Everlasting",
    },
    AsmaulHusna {
        index: 69,
        latin: "Al Qaadir",
        arabic: "الْقَادِرُ",
        translation_id: "Yang Memiliki Mutlak sifat MahaMenentukan, Maha Menyeimbangkan",
        translation_en: "The All Able",
    },
    AsmaulHusna {
        index: 70,
        latin: "Al Muqtadir",
        arabic: "الْمُقْتَدِرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Berkuasa",
        translation_en: "The All Determiner, the Dominant",
    },
    AsmaulHusna {
        index: 71,
        latin: "Al Muqaddim",
        arabic: "الْمُقَدِّمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mendahulukan",
        translation_en: "The Expediter, He who brings forward",
    },
    AsmaulHusna {
        index: 72,
        latin: "Al Mu’akkhir",
        arabic: "الْمُؤَخِّرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mengakhirkan",
        translation_en: "The Delayer, He who puts far away",
    },
    AsmaulHusna {
        index: 73,
        latin: "Al Awwal",
        arabic: "الأوَّلُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Awal",
        translation_en: "The First",
    },
    AsmaulHusna {
        index: 74,
        latin: "Al Aakhir",
        arabic: "الآخِرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Akhir",
        translation_en: "The Last",
    },
    AsmaulHusna {
        index: 75,
        latin: "Az Zhaahir",
        arabic: "الظَّاهِرُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Nyata",
        translation_en: "The Manifest; the All Victorious",
    },
    AsmaulHusna {
        index: 76,
        latin: "Al Baathin",
        arabic: "الْبَاطِنُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Ghaib",
        translation_en: "The Hidden; the All Encompassing",
    },
    AsmaulHusna {
        index: 77,
        latin: "Al Waali",
        arabic: "الْوَالِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memerintah",
        translation_en: "The Patron",
    },
    AsmaulHusna {
        index: 78,
        latin: "Al Muta’aalii",
        arabic: "الْمُتَعَالِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Tinggi",
        translation_en: "The Self Exalted",
    },
    AsmaulHusna {
        index: 79,
        latin: "Al Barri",
        arabic: "الْبَرُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Penderma",
        translation_en: "The Most Kind and Righteous",
    },
    AsmaulHusna {
        index: 80,
        latin: "At Tawwaab",
        arabic: "التَّوَابُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Penerima Tobat",
        translation_en: "The Ever Returning, Ever Relenting",
    },
    AsmaulHusna {
        index: 81,
        latin: "Al Muntaqim",
        arabic: "الْمُنْتَقِمُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Penuntut Balas",
        translation_en: "The Avenger",
    },
    AsmaulHusna {
        index: 82,
        latin: "Al Afuww",
        arabic: "العَفُوُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pemaaf",
        translation_en: "The Pardoner, the Effacer of Sins",
    },
    AsmaulHusna {
        index: 83,
        latin: "Ar Ra`uuf",
        arabic: "الرَّؤُوفُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pengasih",
        translation_en: "The Compassionate, the All Pitying",
    },
    AsmaulHusna {
        index: 84,
        latin: "Malikul Mulk",
        arabic: "مَالِكُ الْمُلْكِ",
        translation_id: "Yang Memiliki Mutlak sifatPenguasa Kerajaan (Semesta)",
        translation_en: "The Owner of All Sovereignty",
    },
    AsmaulHusna {
        index: 85,
        latin: "Dzul JalaaliWal Ikraam",
        arabic: "ذُوالْجَلاَلِوَالإكْرَامِ",
        translation_id: "Yang Memiliki Mutlak sifat PemilikKebesaran dan Kemuliaan",
        translation_en: "The Lord of Majesty and Generosity",
    },
    AsmaulHusna {
        index: 86,
        latin: "Al Muqsith",
        arabic: "الْمُقْسِطُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Adil",
        translation_en: "The Equitable, the Requiter",
    },
    AsmaulHusna {
        index: 87,
        latin: "Al Jamii’",
        arabic: "الْجَامِعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mengumpulkan",
        translation_en: "The Gatherer, the Unifier",
    },
    AsmaulHusna {
        index: 88,
        latin: "Al Ghaniyy",
        arabic: "الْغَنِيُّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Berkecukupan",
        translation_en: "The All Rich, the Independent",
    },
    AsmaulHusna {
        index: 89,
        latin: "Al Mughnii",
        arabic: "الْمُغْنِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memberi Kekayaan",
        translation_en: "The Enricher, the Emancipator",
    },
    AsmaulHusna {
        index: 90,
        latin: "Al Maani",
        arabic: "اَلْمَانِعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Mencegah",
        translation_en: "The Withholder, the Shielder, the Defender",
    },
    AsmaulHusna {
        index: 91,
        latin: "Ad Dhaar",
        arabic: "الضَّارَّ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memberi Derita",
        translation_en: "The Distressor, the Harmer",
    },
    AsmaulHusna {
        index: 92,
        latin: "An Nafii’",
        arabic: "النَّافِعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Memberi Manfaat",
        translation_en: "The Propitious, the Benefactor",
    },
    AsmaulHusna {
        index: 93,
        latin: "An Nuur",
        arabic: "النُّورُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Bercahaya(Menerangi, Memberi Cahaya)",
        translation_en: "The Light",
    },
    AsmaulHusna {
        index: 94,
        latin: "Al Haadii",
        arabic: "الْهَادِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pemberi Petunjuk",
        translation_en: "The Guide",
    },
    AsmaulHusna {
        index: 95,
        latin: "Al Baadii",
        arabic: "الْبَدِيعُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pencipta",
        translation_en: "Incomparable, the Originator",
    },
    AsmaulHusna {
        index: 96,
        latin: "Al Baaqii",
        arabic: "اَلْبَاقِي",
        translation_id: "Yang Memiliki Mutlak sifat Maha Kekal",
        translation_en: "The Ever Enduring and Immutable",
    },
    AsmaulHusna {
        index: 97,
        latin: "Al Waarits",
        arabic: "الْوَارِثُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pewaris",
        translation_en: "The Heir, the Inheritor of All",
    },
    AsmaulHusna {
        index: 98,
        latin: "Ar Rasyiid",
        arabic: "الرَّشِيدُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Pandai",
        translation_en: "The Guide, Infallible Teacher, and Knower",
    },
    AsmaulHusna {
        index: 99,
        latin: "As Shabuur",
        arabic: "الصَّبُورُ",
        translation_id: "Yang Memiliki Mutlak sifat Maha Sabar",
        translation_en: "The Patient",
    },
];

/// `data.find(item => item.index === index)` — cari lewat nomor.
fn asmaul_by_index(index: u32) -> Option<AsmaulHusna> {
    ASMAUL_HUSNA.iter().find(|n| n.index == index).copied()
}

/// `searchItem(..., 'latin')` — `item[field].toLowerCase().includes(query)`.
/// `query` harus sudah lowercase (JS: `query.toLowerCase()`).
fn asmaul_by_latin(query: &str) -> Option<AsmaulHusna> {
    ASMAUL_HUSNA
        .iter()
        .find(|n| n.latin.to_lowercase().contains(query))
        .copied()
}

fn render_asmaul(n: &AsmaulHusna) -> String {
    let mut text = String::from("☪️ *ASMAUL HUSNA*\n\n");
    text.push_str("```");
    text.push_str(&format!("📍 Nomor : {}\n", n.index));
    text.push_str(&format!("🔤 Latin : {}\n", n.latin));
    text.push_str(&format!("📜 Arab  : {}", n.arabic));
    text.push_str("```\n\n");
    text.push_str(&format!("> 🇮🇩 Arti (ID): {}\n", n.translation_id));
    text.push_str(&format!("> 🇬🇧 Arti (EN): {}", n.translation_en));
    text
}

/// Halaman 1/3 — 33 entri pertama.
fn render_asmaul_page1() -> String {
    let mut text = String::from("☪️ *ASMAUL HUSNA*\n");
    text.push_str("> 99 Nama Allah SWT\n\n");
    text.push_str("```");
    for n in &ASMAUL_HUSNA[..33] {
        text.push_str(&format!("{}. {}\n", n.index, n.latin));
    }
    text.push_str("```\n");
    text.push_str("> Halaman 1/3\n\n");
    text.push_str("_Gunakan .asmaulhusna [nomor] untuk detail_");
    text
}

/// `/^\d+$/` — hanya ascii digit, minimal satu karakter.
fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[plugin(
    name = "asmaulhusna",
    alias = ["asmaul", "husna", "99names"],
    category = "religi",
    description = "99 Nama Allah (Asmaul Husna)",
    usage = ".asmaulhusna [nomor/nama]",
    example = ".asmaulhusna 1\n.asmaulhusna ar rahman",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn asmaulhusna(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim().to_string();

    let name = if query.is_empty() {
        // JS: `getRandomItem('asmaulhusna.json')`.
        ASMAUL_HUSNA.get(random_index(ASMAUL_HUSNA.len())).copied()
    } else if is_all_digits(&query) {
        // JS `parseInt` di string yang sudah lolos `/^\d+$/`, jadi tidak pernah
        // gagal parse. `.parse()` diasumsikan berhasil setelah dicek digits.
        let index: u32 = query.parse().unwrap_or(0);
        if !(1..=99).contains(&index) {
            return Reply::text("❌ Nomor harus antara 1-99!");
        }
        asmaul_by_index(index)
    } else if query.to_lowercase() == "all" || query.to_lowercase() == "semua" {
        return Reply::text(render_asmaul_page1());
    } else {
        asmaul_by_latin(&query.to_lowercase())
    };

    match name {
        Some(n) => Reply::text(render_asmaul(&n)),
        None => Reply::text("❌ Nama tidak ditemukan!\n_Coba nomor 1-99 atau nama latin_"),
    }
} // ---------------------------------------------------------------------------
  // Tests
  // ---------------------------------------------------------------------------

cek_plugin!(
    cekkpopers,
    "cekkpopers",
    ["kpopers", "kpop"],
    "Cek tingkat kpopers kamu",
    ".cekkpopers <nama>",
    ".cekkpopers Budi",
    "kekpopersan",
    ["ARMY/BLINK level max! 💜💗", "Stan berat nih! 🎤", "Casual listener~ 🎵", "Tau dikit-dikit aja 😅", "Bukan kpopers 🤷"]
    );

cek_plugin!(
    ceksocmed,
    "ceksocmed",
    ["sosmed", "medsos"],
    "Cek tingkat kecanduan sosmed",
    ".ceksocmed <nama>",
    ".ceksocmed Budi",
    "kesocmedan",
    ["Kecanduan parah! Detox needed! 📱💀", "Scroll terus tanpa henti~ 📲", "Normal usage 👍", "Cukup sehat nih 🌿", "Digital detox master! 🧘"]
    );

cek_plugin!(
    cektsundere,
    "cektsundere",
    ["tsundere"],
    "Cek tingkat tsundere kamu",
    ".cektsundere <nama>",
    ".cektsundere Budi",
    "ketsunderean",
    ["BAKA! B-BUKAN BERARTI AKU SUKA! 😤💢", "Hmph! Jangan salah paham ya! 😳", "Y-yah terserah kamu deh... 👉👈", "Agak tsundere dikit~ 😊", "Bukan tsundere, jujur aja kok 💕"]
    );

#[cfg(test)]
mod tests {
    use super::*;

    // TABEL META — dari rest.json (manifest plugin yang belum di-port). Field yang
    // tidak di-set plugin memakai defaultConfig JS: cooldown 3, energi 0, example "".
    type MetaRow = (
        &'static str,
        &'static [&'static str],
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        u32,
        i64,
    );

    const META: [MetaRow; 30] = [
        (
            "cekhoki",
            &["hoki"],
            "cek",
            "Cek seberapa hoki kamu",
            ".cekhoki <nama>",
            ".cekhoki Budi",
            5,
            0,
        ),
        (
            "cekimut",
            &["imut", "cute"],
            "cek",
            "Cek seberapa imut kamu",
            ".cekimut <nama>",
            ".cekimut Ani",
            5,
            0,
        ),
        (
            "cekintrovert",
            &["introvert"],
            "cek",
            "Cek tingkat introvert kamu",
            ".cekintrovert <nama>",
            ".cekintrovert Budi",
            5,
            0,
        ),
        (
            "cekjahat",
            &["evil"],
            "cek",
            "Cek seberapa jahat kamu",
            ".cekjahat <nama>",
            ".cekjahat Budi",
            5,
            0,
        ),
        (
            "cekjomblo",
            &["jomblo", "single"],
            "cek",
            "Cek tingkat kejombloan kamu",
            ".cekjomblo <nama>",
            ".cekjomblo Budi",
            5,
            0,
        ),
        (
            "cekkaya",
            &["kaya", "rich"],
            "cek",
            "Cek seberapa kaya kamu",
            ".cekkaya <nama>",
            ".cekkaya Budi",
            5,
            0,
        ),
        (
            "cekkece",
            &["kece", "cool"],
            "cek",
            "Cek seberapa kece kamu",
            ".cekkece <nama>",
            ".cekkece Budi",
            5,
            0,
        ),
        (
            "ceklapar",
            &["lapar", "hungry"],
            "cek",
            "Cek tingkat kelaparan kamu",
            ".ceklapar <nama>",
            ".ceklapar Budi",
            5,
            0,
        ),
        (
            "cekmalas",
            &["malas", "lazy"],
            "cek",
            "Cek seberapa malas kamu",
            ".cekmalas <nama>",
            ".cekmalas Budi",
            5,
            0,
        ),
        (
            "cekmesum",
            &["mesum"],
            "cek",
            "Cek seberapa mesum kamu",
            ".cekmesum <nama>",
            ".cekmesum Budi",
            5,
            0,
        ),
        (
            "cekngantuk",
            &["ngantuk", "sleepy"],
            "cek",
            "Cek tingkat ngantuk kamu",
            ".cekngantuk <nama>",
            ".cekngantuk Budi",
            5,
            0,
        ),
        (
            "cekotaku",
            &["otaku"],
            "cek",
            "Cek tingkat otaku kamu",
            ".cekotaku <nama>",
            ".cekotaku Budi",
            5,
            0,
        ),
        (
            "cekoverpower",
            &["overpower", "op"],
            "cek",
            "Cek tingkat overpower kamu",
            ".cekoverpower <nama>",
            ".cekoverpower Budi",
            5,
            0,
        ),
        (
            "cekpelit",
            &["pelit", "kikir"],
            "cek",
            "Cek seberapa pelit kamu",
            ".cekpelit <nama>",
            ".cekpelit Budi",
            5,
            0,
        ),
        (
            "cekprocastinator",
            &["procrastinator", "nunda"],
            "cek",
            "Cek tingkat suka menunda",
            ".cekprocastinator <nama>",
            ".cekprocastinator Budi",
            5,
            0,
        ),
        (
            "cekpsikopat",
            &["psikopat", "psycho"],
            "cek",
            "Cek seberapa psikopat kamu",
            ".cekpsikopat <nama>",
            ".cekpsikopat Budi",
            5,
            0,
        ),
        (
            "cekrezeki",
            &["rezeki", "fortune"],
            "cek",
            "Cek tingkat rezeki kamu hari ini",
            ".cekrezeki <nama>",
            ".cekrezeki Budi",
            5,
            0,
        ),
        (
            "ceksabar",
            &["sabar", "patience"],
            "cek",
            "Cek tingkat kesabaran kamu",
            ".ceksabar <nama>",
            ".ceksabar Budi",
            5,
            0,
        ),
        (
            "ceksetia",
            &["setia", "loyal"],
            "cek",
            "Cek tingkat kesetiaan kamu",
            ".ceksetia <nama>",
            ".ceksetia Budi",
            5,
            0,
        ),
        (
            "ceksexy",
            &["sexy", "hot"],
            "cek",
            "Cek seberapa sexy kamu",
            ".ceksexy <nama>",
            ".ceksexy Budi",
            5,
            0,
        ),
        (
            "ceksial",
            &["sial", "apes"],
            "cek",
            "Cek seberapa sial kamu",
            ".ceksial <nama>",
            ".ceksial Budi",
            5,
            0,
        ),
        (
            "cekwibu",
            &["wibu", "weeb"],
            "cek",
            "Cek seberapa wibu kamu",
            ".cekwibu <nama>",
            ".cekwibu Budi",
            5,
            0,
        ),
        (
            "cekyandere",
            &["yandere"],
            "cek",
            "Cek tingkat yandere kamu",
            ".cekyandere <nama>",
            ".cekyandere Budi",
            5,
            0,
        ),
        (
            "cekjodoh",
            &[],
            "cek",
            "Cek kecocokan jodoh",
            ".cekjodoh <nama1> & <nama2>",
            ".cekjodoh Budi & Ani",
            5,
            0,
        ),
        (
            "cekkepribadian",
            &["kepribadian", "personality"],
            "cek",
            "Cek kepribadian kamu",
            ".cekkepribadian <nama>",
            ".cekkepribadian Budi",
            5,
            0,
        ),
        (
            "benefitpartner",
            &["partnerbenefits", "keuntunganpartner"],
            "info",
            "Lihat keuntungan menjadi partner bot",
            ".benefitpartner",
            ".benefitpartner",
            5,
            0,
        ),
        (
            "benefitowner",
            &["ownerbenefits", "ownerfitur"],
            "main",
            "Lihat penjelasan dan daftar fitur khusus Owner",
            ".benefitowner",
            "",
            3,
            0,
        ),
        (
            "benefitpremium",
            &["premiumbenefits", "premiumfitur", "benefitprem"],
            "main",
            "Lihat penjelasan dan daftar fitur khusus Premium",
            ".benefitpremium",
            "",
            3,
            0,
        ),
        (
            "templateplugin",
            &["tplplugin", "plugin-template"],
            "owner",
            "Generate plugin template (Owner Only)",
            ".templateplugin",
            ".templateplugin",
            0,
            0,
        ),
        (
            "asmaulhusna",
            &["asmaul", "husna", "99names"],
            "religi",
            "99 Nama Allah (Asmaul Husna)",
            ".asmaulhusna [nomor/nama]",
            ".asmaulhusna 1\n.asmaulhusna ar rahman",
            3,
            0,
        ),
    ];

    /// (nama, label, tier teratas, tier terbawah) — kata & deskripsi diambil
    /// dari file JS aslinya.
    type FourTierRow = (&'static str, &'static str, &'static str, &'static str);

    const FOUR_TIER_CASES: [FourTierRow; 23] = [
        (
            "cekhoki",
            "kehokian",
            "HOKI DEWA! Main gacha pasti menang! 🍀✨",
            "Sabar ya, lagi apes 😅",
        ),
        (
            "cekimut",
            "keimutan",
            "IMUT BANGET! Kawaii~~ 🥺💕",
            "Mungkin cool bukan imut? 😎",
        ),
        (
            "cekintrovert",
            "keintrovertan",
            "Rumah adalah surga! Stay home~ 🏠",
            "Extrovert mode ON! 🎉",
        ),
        (
            "cekjahat",
            "kejahatan",
            "VILLAIN LEVEL! 😈👿",
            "Baik kok, gak jahat! 😇",
        ),
        (
            "cekjomblo",
            "kejombloan",
            "Jomblo abadi! Single is happiness~ 💔😎",
            "Soon taken! 💕",
        ),
        (
            "cekkaya",
            "kekayaan",
            "Sultan! Crazy rich! 💎",
            "Semangat nabung! 🙏",
        ),
        (
            "cekkece",
            "kekecean",
            "KECE BADAI! 😎🔥",
            "Biasa aja, tapi tetep keren! 🙂",
        ),
        (
            "ceklapar",
            "kelaparan",
            "LAPARRR! Makan sekarang! 🍔🍕🍜",
            "Kekenyangan! 🤰",
        ),
        (
            "cekmalas",
            "kemalasan",
            "SUPER MALAS! Raja rebahan! 🛏️",
            "Rajin banget! 💪",
        ),
        (
            "cekmesum",
            "kemesuman",
            "MESUM AKUT! Tobat mas! 😳🔞",
            "Polos dan suci! 😇",
        ),
        (
            "cekngantuk",
            "kengantukan",
            "ZZZZZ... Tidur sana! 😴💤",
            "Melek banget! Insomnia? 👀",
        ),
        (
            "cekotaku",
            "keotakuan",
            "SUGOI! True otaku desu! 🎌✨",
            "Normie detected! 😂",
        ),
        (
            "cekoverpower",
            "keoverpoweran",
            "OVERPOWER BANGET! LEGEND! 👑🔥",
            "Masih perlu latihan 📝",
        ),
        (
            "cekpelit",
            "kepelitan",
            "SUPER PELIT! Duit dijaga mati-matian! 💸",
            "Dermawan banget! 🎁",
        ),
        (
            "cekprocastinator",
            "keprocastinatoran",
            "Deadline? Besok aja deh~ 😴",
            "Disiplin tinggi! Salut! 🏆",
        ),
        (
            "cekpsikopat",
            "kepsikopatan",
            "PSIKOPAT AKUT! Jauhi! 😈",
            "Normal dan baik hati 😇",
        ),
        (
            "cekrezeki",
            "kerezekian",
            "Rezeki melimpah! Jackpot! 💰🎉",
            "Sabar ya, rezeki akan datang~ 🫂",
        ),
        (
            "ceksabar",
            "kesabaran",
            "Sabar level dewa! Zen master~ 🧘",
            "Gampang marah nih... 😤",
        ),
        (
            "ceksetia",
            "kesetiaan",
            "Setia sampai mati! 💍💕",
            "Playboy/Playgirl mode? 😏",
        ),
        (
            "ceksexy",
            "kesexyan",
            "SEXY ABIS! 🔥🔥🔥",
            "Mungkin cute bukan sexy 😅",
        ),
        (
            "ceksial",
            "kesialan",
            "SIAL BANGET! Mending di rumah aja! 😭",
            "Gak sial, hoki dong! 🍀",
        ),
        (
            "cekwibu",
            "kewibuan",
            "WIBU SEJATI! Ara ara~ 🎌",
            "Bukan wibu, normal! 😎",
        ),
        (
            "cekyandere",
            "keyanderean",
            "Kamu milikku selamanya~ 🔪💕",
            "Normal kok, santai~ 😊",
        ),
    ];

    fn ctx_with(mentioned: Option<&str>) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            mentioned: mentioned.map(|m| vec![m.to_string()]).unwrap_or_default(),
            ..Default::default()
        }
    }

    fn ctx_text(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            ..Default::default()
        }
    }

    fn text_of(reply: &Reply) -> String {
        reply
            .text_body()
            .expect("plugin harus membalas teks")
            .to_string()
    }

    // -- registrasi: satu test per plugin -----------------------------------

    macro_rules! meta_test {
        ($fn_name:ident, $name:literal) => {
            #[test]
            fn $fn_name() {
                let (table_name, alias, category, description, usage, example, cooldown, energi) =
                    META.iter()
                        .find(|(n, ..)| *n == $name)
                        .copied()
                        .unwrap_or_else(|| panic!("$name tidak ada di tabel META"));
                assert_eq!(table_name, $name, "baris tabel META salah untuk {}", $name);
                let p = oktz_plugin_api::find($name)
                    .unwrap_or_else(|| panic!("plugin tidak terdaftar: {}", $name));
                assert_eq!(p.meta.name, $name);
                assert_eq!(p.meta.alias, alias);
                assert_eq!(p.meta.category, category);
                assert_eq!(p.meta.description, description);
                assert_eq!(p.meta.usage, usage);
                assert_eq!(p.meta.example, example);
                assert_eq!(p.meta.cooldown, cooldown);
                assert_eq!(p.meta.energi, energi);
                assert!(p.meta.is_enabled);
                assert!(!p.meta.is_private, "defaultConfig JS isPrivate=false");
                assert!(!p.meta.is_group);
                assert!(!p.meta.is_admin);
                assert!(!p.meta.is_bot_admin);
            }
        };
    }

    meta_test!(meta_cekhoki, "cekhoki");
    meta_test!(meta_cekimut, "cekimut");
    meta_test!(meta_cekintrovert, "cekintrovert");
    meta_test!(meta_cekjahat, "cekjahat");
    meta_test!(meta_cekjomblo, "cekjomblo");
    meta_test!(meta_cekkaya, "cekkaya");
    meta_test!(meta_cekkece, "cekkece");
    meta_test!(meta_ceklapar, "ceklapar");
    meta_test!(meta_cekmalas, "cekmalas");
    meta_test!(meta_cekmesum, "cekmesum");
    meta_test!(meta_cekngantuk, "cekngantuk");
    meta_test!(meta_cekotaku, "cekotaku");
    meta_test!(meta_cekoverpower, "cekoverpower");
    meta_test!(meta_cekpelit, "cekpelit");
    meta_test!(meta_cekprocastinator, "cekprocastinator");
    meta_test!(meta_cekpsikopat, "cekpsikopat");
    meta_test!(meta_cekrezeki, "cekrezeki");
    meta_test!(meta_ceksabar, "ceksabar");
    meta_test!(meta_ceksetia, "ceksetia");
    meta_test!(meta_ceksexy, "ceksexy");
    meta_test!(meta_ceksial, "ceksial");
    meta_test!(meta_cekwibu, "cekwibu");
    meta_test!(meta_cekyandere, "cekyandere");
    meta_test!(meta_cekjodoh, "cekjodoh");
    meta_test!(meta_cekkepribadian, "cekkepribadian");
    meta_test!(meta_benefitpartner, "benefitpartner");
    meta_test!(meta_benefitowner, "benefitowner");
    meta_test!(meta_benefitpremium, "benefitpremium");
    meta_test!(meta_templateplugin, "templateplugin");
    meta_test!(meta_asmaulhusna, "asmaulhusna");

    #[test]
    fn all_thirty_plugins_are_registered() {
        assert_eq!(META.len(), 30);
        for (name, ..) in META {
            assert!(
                oktz_plugin_api::find(name).is_some(),
                "{name} tidak terdaftar"
            );
        }
    }

    #[test]
    fn only_templateplugin_is_owner_only() {
        // `isOwner: true` hanya di templateplugin dari batch ini, jadi
        // `benefitowner` tidak memasukkan dirinya sendiri ke daftarnya.
        for (name, ..) in META {
            let p = oktz_plugin_api::find(name).unwrap();
            assert_eq!(
                p.meta.is_owner,
                name == "templateplugin",
                "{name} punya isOwner yang salah"
            );
        }
        let list = registry_command_lines(|m| m.is_owner && m.is_enabled);
        assert_eq!(list, vec!["• *.templateplugin*".to_string()]);
    }

    // -- alias & tabrakan ---------------------------------------------------

    #[test]
    fn every_alias_resolves_to_its_own_plugin() {
        for (name, aliases, ..) in META {
            for alias in aliases.iter() {
                let found = oktz_plugin_api::find(alias)
                    .unwrap_or_else(|| panic!("alias {alias} tidak resolve"));
                assert_eq!(&found.meta.name, name, "alias {alias} salah resolve");
            }
        }
    }

    /// `find()` mengembalikan entri pertama yang cocok, jadi satu alias yang
    /// diklaim dua plugin akan membuat `find(alias)` mengembalikan plugin yang
    /// salah. Semua key (nama + alias) batch ini harus eksklusif.
    #[test]
    fn no_key_collides_with_another_registered_plugin() {
        for (name, aliases, ..) in META {
            for key in std::iter::once(name).chain(aliases.iter().copied()) {
                let owners: Vec<&str> = all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(key))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(
                    owners,
                    vec![name],
                    "key {key} diklaim lebih dari satu plugin: {owners:?}"
                );
            }
        }
    }

    /// Alias yang sudah diklaim cek.rs / cek2.rs / umum.rs. Kalau salah satu
    /// masuk ke batch ini, `find()` jadi ambigu.
    #[test]
    fn no_alias_is_already_claimed_by_cek_cek2_or_umum() {
        let previously_claimed = [
            "cekbaik",
            "cekbucin",
            "cekcantik",
            "cekcreative",
            "cekcupu",
            "cekgabut",
            "cekgacha",
            "cekgamer",
            "cekganteng",
            "cekgila",
            "cekkarma",
            "cekumur",
            "cekberat",
            "cektinggi",
            "ceksisaumur",
            "cekpintar",
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
            "baik",
            "kind",
            "bucin",
            "cantik",
            "beautiful",
            "creative",
            "kreatif",
            "cupu",
            "noob",
            "gabut",
            "bored",
            "luck",
            "gamer",
            "pro",
            "ganteng",
            "handsome",
            "gila",
            "crazy",
            "karma",
            "umur",
            "age",
            "berat",
            "weight",
            "tinggi",
            "tall",
            "sisaumur",
            "pintar",
            "iq",
            "smart",
            "apa",
            "kenapa",
            "why",
            "when",
            "akan",
            "will",
            "gimana",
            "how",
            "bisa",
            "try",
            "where",
            "mana",
            "harus",
            "should",
            "howmuch",
            "howmany",
            "nilai",
            "rating",
            "selengkapnya",
            "spoiler",
            "kkmbg",
        ];
        for (name, aliases, ..) in META {
            assert!(
                !previously_claimed.contains(&name),
                "nama {name} sudah dipakai batch lain"
            );
            for alias in aliases.iter() {
                assert!(
                    !previously_claimed.contains(alias),
                    "alias {alias} ({name}) tabrakan dengan batch lain"
                );
            }
        }
    }

    #[test]
    fn every_ported_plugin_answers_through_the_registry() {
        for (name, ..) in META {
            let p = oktz_plugin_api::find(name).unwrap();
            let reply = (p.handler)(ctx_text("Budi & Ani"));
            assert!(reply.text_body().is_some(), "{name} tidak membalas teks");
        }
    }

    // -- cek_reply: ambang & cabang -----------------------------------------

    const T: [&str; 5] = ["tier-a", "tier-b", "tier-c", "tier-d", "tier-e"];

    fn desc_for(percent: u32) -> &'static str {
        if percent >= 90 {
            T[0]
        } else if percent >= 70 {
            T[1]
        } else if percent >= 50 {
            T[2]
        } else if percent >= 30 {
            T[3]
        } else {
            T[4]
        }
    }

    #[test]
    fn tier_bands_match_the_javascript_thresholds_exactly() {
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
    fn all_101_percents_pick_exactly_one_tier() {
        for percent in 0..=100u32 {
            let out = text_of(&cek_reply(&ctx_with(None), percent, "x", T));
            assert!(
                out.contains(&format!("*{percent}%*")),
                "percent {percent} tidak muncul: {out}"
            );
            let hits = T.iter().filter(|t| out.contains(**t)).count();
            assert_eq!(hits, 1, "percent {percent} memilih {hits} tier: {out}");
            assert!(out.contains(desc_for(percent)));
        }
    }

    #[test]
    fn self_branch_says_kamu_and_mentions_self() {
        let r = cek_reply(&ctx_with(None), 95, "kekayaan", T);
        match &r {
            Reply::Text { text, mentions } => {
                assert_eq!(
                    text,
                    "Hai @628111\n    \nTingkat kekayaan kamu *95%*\n```tier-a```"
                );
                assert!(!text.contains("yak?"));
                assert_eq!(mentions, &["628111@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn other_branch_says_dia_and_mentions_the_target() {
        let r = cek_reply(&ctx_with(Some("628999@s.whatsapp.net")), 12, "kekayaan", T);
        match &r {
            Reply::Text { text, mentions } => {
                assert_eq!(
                    text,
                    "Kamu ingin ngecek tingkat kekayaan @628999 yak? \n    \nTingkat kekayaan dia sebesar *12%*\n```tier-e```"
                );
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    /// JS `mentioned.split('@')[0]` hanya membuang bagian `@server`; device
    /// `:3` ikut tertinggal karena itu bagian hasil split pertama.
    #[test]
    fn display_number_keeps_the_device_suffix_like_js_split() {
        let r = cek_reply(&ctx_with(Some("628999:3@s.whatsapp.net")), 75, "x", T);
        match &r {
            Reply::Text { text, mentions } => {
                assert!(text.contains("@628999:3 yak?"), "{text}");
                assert_eq!(mentions, &["628999:3@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn percent_101_can_reach_both_ends() {
        // `Math.floor(Math.random() * 101)` inklusif 100. Kalau range-nya
        // 0..100, tier teratas jadi cabang mati.
        let (mut saw_high, mut saw_low) = (false, false);
        for _ in 0..4000 {
            let p = percent_101();
            assert!(p <= 100, "diluar range: {p}");
            saw_high |= p == 100;
            saw_low |= p == 0;
        }
        assert!(saw_high && saw_low, "0 dan 100 harus mungkin terjadi");
    }

    #[test]
    fn random_index_never_returns_the_length() {
        for len in [1usize, 2, 5, 16, 99] {
            let mut seen = vec![false; len];
            for _ in 0..4000 {
                let i = random_index(len);
                assert!(i < len, "indeks {i} keluar dari range {len}");
                seen[i] = true;
            }
            assert!(
                seen.iter().all(|s| *s),
                "ada slot yang tak akan pernah muncul"
            );
        }
    }

    #[test]
    fn every_four_tier_plugin_uses_its_own_label_and_tier_pool() {
        for (name, label, top, bottom) in FOUR_TIER_CASES {
            let p = oktz_plugin_api::find(name).unwrap();
            let (mut saw_top, mut saw_bottom) = (false, false);
            for _ in 0..600 {
                let out = text_of(&(p.handler)(ctx_with(Some("628999@s.whatsapp.net"))));
                assert!(
                    out.contains(&format!("tingkat {label} @628999")),
                    "{name}: {out}"
                );
                assert!(!out.contains("Tingkat kepribadian"), "{name} label nyasar");
                saw_top |= out.contains(top);
                saw_bottom |= out.contains(bottom);
            }
            assert!(saw_top, "{name} tidak pernah memakai tier teratas");
            assert!(saw_bottom, "{name} tidak pernah memakai tier terbawah");
        }
    }

    // -- cekjodoh -----------------------------------------------------------

    #[test]
    fn jodoh_rejects_fewer_than_two_names() {
        for text in ["", "   ", "Budi", " & ", ",", "& , &"] {
            let out = text_of(&cekjodoh(ctx_text(text)));
            assert_eq!(
                out, "💕 *ᴄᴇᴋ ᴊᴏᴅᴏʜ*\n\n> Masukkan 2 nama!\n\n> Contoh: .cekjodoh Budi & Ani",
                "input {text:?} harus ditolak"
            );
        }
    }

    #[test]
    fn jodoh_error_branch_uses_the_message_prefix() {
        let mut c = ctx_text("Budi");
        c.prefix = "!".into();
        assert!(text_of(&cekjodoh(c)).ends_with("Contoh: !cekjodoh Budi & Ani"));
    }

    #[test]
    fn jodoh_accepts_ampersand_comma_and_extra_spaces() {
        for text in [
            "Budi & Ani",
            "Budi, Ani",
            "   Budi &   Ani   ",
            "Budi ,Ani",
            "Budi&C",
        ] {
            let out = text_of(&cekjodoh(ctx_text(text)));
            assert!(
                out.starts_with("Hai @628111\n    \nTingkat kejodohan kamu *"),
                "{text:?}: {out}"
            );
        }
    }

    #[test]
    fn jodoh_ignores_the_typed_names_and_mentions_the_target() {
        let mut c = ctx_text("Budi & Ani");
        c.mentioned = vec!["628999@s.whatsapp.net".into()];
        let r = cekjodoh(c);
        let text = text_of(&r);
        assert!(
            !text.contains("Budi"),
            "nama yang diketik tidak dirender: {text}"
        );
        assert!(text.contains("@628999"));
        match r {
            Reply::Text { mentions, .. } => {
                assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()]);
            }
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn jodoh_desc_bands_match_javascript() {
        for (percent, expected) in [
            (100, "Jodoh banget! Langsung nikah aja! 💍"),
            (90, "Jodoh banget! Langsung nikah aja! 💍"),
            (89, "Cocok banget! 💕"),
            (70, "Cocok banget! 💕"),
            (69, "Lumayan cocok~ 😊"),
            (50, "Lumayan cocok~ 😊"),
            (49, "Hmm, perlu usaha lebih 🤔"),
            (30, "Hmm, perlu usaha lebih 🤔"),
            (29, "Mungkin cari yang lain? 😅"),
            (0, "Mungkin cari yang lain? 😅"),
        ] {
            assert_eq!(jodoh_desc(percent), expected, "percent {percent}");
        }
    }

    #[test]
    fn jodoh_percent_always_stays_inside_the_js_range() {
        for _ in 0..2000 {
            let out = text_of(&cekjodoh(ctx_text("Budi & Ani")));
            let percent: u32 = out
                .split('*')
                .nth(1)
                .and_then(|s| s.strip_suffix('%'))
                .unwrap_or_else(|| panic!("bentuk persen berubah: {out}"))
                .parse()
                .expect("persen harus angka");
            assert!(percent <= 100, "persen {percent} di luar range");
            assert_eq!(jodoh_desc(percent), out.split("```").nth(1).unwrap());
        }
    }

    // -- cekkepribadian -----------------------------------------------------

    #[test]
    fn personalities_table_matches_the_javascript_order() {
        assert_eq!(PERSONALITIES.len(), 16);
        assert_eq!(
            PERSONALITIES[0],
            (
                "INTJ",
                "The Architect",
                "Visioner, strategis, dan independen"
            )
        );
        assert_eq!(
            PERSONALITIES[7],
            ("ENFP", "The Campaigner", "Antusias, kreatif, dan sosial")
        );
        assert_eq!(
            PERSONALITIES[15],
            ("ESFP", "The Entertainer", "Spontan, energik, dan fun")
        );
    }

    #[test]
    fn every_personality_is_reachable_and_both_branches_render() {
        let p = oktz_plugin_api::find("cekkepribadian").unwrap();
        let mut seen = vec![false; PERSONALITIES.len()];
        for _ in 0..3000 {
            let out = text_of(&(p.handler)(ctx_with(None)));
            let idx = PERSONALITIES
                .iter()
                .enumerate()
                .find(|(_, (t, title, desc))| {
                    out.contains(&format!("*{t} - {title}*\n```{desc}```"))
                })
                .map(|(i, _)| i)
                .unwrap_or_else(|| panic!("kepribadian di luar tabel: {out}"));
            seen[idx] = true;
            assert!(out.starts_with("Hai @628111\n    \nTingkat kepribadian kamu *"));
        }
        assert!(
            seen.iter().all(|s| *s),
            "ada kepribadian yang tak akan pernah muncul"
        );

        let other = text_of(&(p.handler)(ctx_with(Some("628999@s.whatsapp.net"))));
        assert!(other.contains("@628999 yak?"));
        assert!(other.contains("\n    \nKepribadian dia adalah *"));
    }

    // -- benefitpartner -----------------------------------------------------

    #[test]
    fn benefitpartner_is_byte_exact_and_interpolates_the_prefix() {
        // Output diambil dari menjalankan handler JS asli.
        let expected = concat!(
            "🤝 *BENEFIT PARTNER*\n\n",
            "Keuntungan menjadi partner Oktz Bot:\n\n",
            "🔓 *Akses Fitur*\n",
            "├ Semua fitur premium terbuka\n",
            "├ Energi & koin unlimited\n",
            "├ Akses command owner tertentu\n",
            "└ Prioritas support\n\n",
            "📦 *Panel Pterodactyl*\n",
            "├ Bisa create server sendiri\n",
            "├ Akses panel management\n",
            "└ Bisa jualan panel (reseller)\n\n",
            "💎 *Bonus*\n",
            "├ +200.000 EXP saat aktivasi\n",
            "├ +20.000 Koin saat aktivasi\n",
            "├ Badge partner di profil\n",
            "└ Akses early feature\n\n",
            "💰 *Cara Jadi Partner*\n",
            "├ Hubungi owner: Oktz\n",
            "├ Durasi: 30/60/90 hari\n",
            "└ Command: `.addpartner` (owner only)\n\n",
            "📋 *Command Partner*\n",
            "├ `.cekpartner` — Cek status partner\n",
            "├ `.cekprem` — Cek status premium\n",
            "├ `.cekowner` — Cek role user\n",
            "└ `.listpartner` — Daftar partner\n\n",
            "> _Hubungi owner untuk info lebih lanjut_",
        );
        assert_eq!(text_of(&benefitpartner(ctx_with(None))), expected);

        let mut c = ctx_with(None);
        c.prefix = "!".into();
        let bang = text_of(&benefitpartner(c));
        assert!(bang.contains("└ Command: `!addpartner` (owner only)"));
        assert!(bang.contains("├ `!cekpartner` — Cek status partner"));
        assert!(!bang.contains("`.cekpartner"));
    }

    // -- benefitowner / benefitpremium --------------------------------------

    #[test]
    fn command_lines_dedupes_skips_empty_and_sorts() {
        let out = command_lines(
            [
                "zeta".to_string(),
                "alpha".to_string(),
                "zeta".to_string(),
                String::new(),
                "beta".to_string(),
            ]
            .into_iter(),
        );
        assert_eq!(out, vec!["• *.alpha*", "• *.beta*", "• *.zeta*"]);
    }

    #[test]
    fn command_lines_of_an_empty_registry_is_empty() {
        assert!(command_lines(Vec::<String>::new().into_iter()).is_empty());
    }

    #[test]
    fn owner_message_renders_the_empty_list_branch() {
        let out = owner_message(&[]);
        assert!(out.starts_with("👑 *ᴀᴘᴀ ɪᴛᴜ ᴏᴡɴᴇʀ?*\n\n"));
        assert!(out.contains("┃ `Total: 0 command`\n┃\n\n╰┈┈┈┈┈┈┈┈⬡\n\n"));
        assert!(out.ends_with("> Hubungi owner untuk mendapatkan akses!"));
        assert!(!out.ends_with('\n'));
    }

    #[test]
    fn owner_message_renders_the_populated_list_branch() {
        let out = owner_message(&["• *.autobackup*".to_string(), "• *.stop*".to_string()]);
        assert!(out.contains("┃ `Total: 2 command`\n┃\n"));
        assert!(out.contains("┃ • *.autobackup*\n┃ • *.stop*\n╰┈┈┈┈┈┈┈┈⬡\n\n"));
    }

    #[test]
    fn benefitowner_lists_exactly_the_is_owner_plugins_in_the_registry() {
        let mut expected: Vec<String> = all_plugins()
            .iter()
            .filter(|e| e.meta.is_owner && e.meta.is_enabled)
            .map(|e| format!("• *{CONFIG_PREFIX}{}*", e.meta.name))
            .collect();
        expected.sort();
        let list = registry_command_lines(|m| m.is_owner && m.is_enabled);
        assert_eq!(list, expected);
        let out = text_of(&benefitowner(ctx_with(None)));
        assert!(out.contains("┃ `Total: 1 command`\n┃\n┃ • *.templateplugin*\n╰┈┈┈┈┈┈┈┈⬡\n\n"));
    }

    #[test]
    fn premium_message_renders_both_list_branches() {
        let empty = premium_message(&[]);
        assert!(empty.starts_with("⭐ *ᴀᴘᴀ ɪᴛᴜ ᴘʀᴇᴍɪᴜᴍ?*\n\n"));
        assert!(
            empty.contains("┃ `Total: 0 command`\n┃\n┃ Semua command bisa diakses user biasa\n╰")
        );
        assert!(empty.contains("```Limit harian: 100x (vs 25x user biasa)```"));
        assert!(empty.ends_with("Mau Upgrade? silahkan hubungi owner bot\n- wa.me/6285608953677"));
        assert!(
            !empty.ends_with(' '),
            "spasi penutup join() bukan bagian output"
        );

        let filled = premium_message(&["• *.premiumcmd*".to_string()]);
        assert!(filled.contains("┃ `Total: 1 command`\n┃\n┃ • *.premiumcmd*\n╰"));
        assert!(!filled.contains("Semua command bisa diakses user biasa"));
    }

    #[test]
    fn benefitpremium_uses_the_empty_branch_with_the_current_registry() {
        // Tidak ada plugin `is_premium` yang terport, jadi cabangnya yang
        // kosong - persis yang akan terlihat di registry sekarang.
        assert!(registry_command_lines(|m| m.is_premium && m.is_enabled).is_empty());
        assert_eq!(
            text_of(&benefitpremium(ctx_with(None))),
            premium_message(&[])
        );
    }

    // -- templateplugin -----------------------------------------------------

    #[test]
    fn templateplugin_is_the_js_template_wrapped_in_a_fence() {
        let out = text_of(&templateplugin(ctx_with(None)));
        assert!(out.starts_with("```\nconst pluginConfig = {"));
        assert!(out.ends_with("export { pluginConfig as config, handler }\n```"));
        assert!(out.contains("name: 'example'"));
        assert!(out.contains("await m.reply('This is an example plugin!')"));
        // `\n\n` di dalam literal JS harus tetap dua karakter, bukan newline.
        assert!(out.contains("await m.reply('❌ *GAGAL*\\n\\n> ' + error.message)"));
        assert!(!out.contains("*GAGAL*\n\n>"));
    }

    // -- asmaulhusna --------------------------------------------------------

    #[test]
    fn asmaulhusna_table_is_complete_and_consistent() {
        assert_eq!(ASMAUL_HUSNA.len(), 99);
        for (i, n) in ASMAUL_HUSNA.iter().enumerate() {
            assert_eq!(n.index as usize, i + 1, "urutan index bocor di entri {i}");
            assert!(!n.latin.is_empty() && !n.arabic.is_empty());
            assert!(!n.translation_id.is_empty() && !n.translation_en.is_empty());
        }
        assert_eq!(asmaul_by_index(1).unwrap().latin, "Ar Rahman");
        assert_eq!(asmaul_by_index(99).unwrap().latin, "As Shabuur");
        assert!(asmaul_by_index(0).is_none());
        assert!(asmaul_by_index(100).is_none());
    }

    #[test]
    fn is_all_digits_behaves_like_the_js_regex() {
        for yes in ["1", "99", "007", "12345"] {
            assert!(is_all_digits(yes), "{yes}");
        }
        for no in ["", " ", "1a", "a1", "-1", "1.0", "+1", "0x10"] {
            assert!(!is_all_digits(no), "{no}");
        }
        // Non-ASCII digit tidak lolos `/\d/` di JS maupun `is_ascii_digit`.
        assert!(!is_all_digits("\u{661}\u{662}"));
    }

    #[test]
    fn asmaulhusna_numeric_branch_renders_the_requested_entry() {
        assert_eq!(
            text_of(&asmaulhusna(ctx_text("1"))),
            render_asmaul(&ASMAUL_HUSNA[0])
        );
        assert!(render_asmaul(&ASMAUL_HUSNA[0]).starts_with(&format!(
            "☪️ *ASMAUL HUSNA*\n\n```📍 Nomor : 1\n🔤 Latin : {}\n📜 Arab  : {}",
            ASMAUL_HUSNA[0].latin, ASMAUL_HUSNA[0].arabic
        )));
        assert!(text_of(&asmaulhusna(ctx_text("99"))).contains("As Shabuur"));
        assert!(text_of(&asmaulhusna(ctx_text("50"))).contains("📍 Nomor : 50"));
        assert!(text_of(&asmaulhusna(ctx_text("007"))).contains("📍 Nomor : 7"));
    }

    #[test]
    fn asmaulhusna_numeric_branch_rejects_out_of_range_numbers() {
        for q in ["0", "100", "101", "9999"] {
            assert_eq!(
                text_of(&asmaulhusna(ctx_text(q))),
                "❌ Nomor harus antara 1-99!",
                "nomor {q} harus ditolak"
            );
        }
    }

    #[test]
    fn asmaulhusna_numeric_branch_survives_overflowing_numbers() {
        // `parseInt` JS menghasilkan angka presisi ganda yang tetap > 99;
        // `u32::from_str` gagal lalu jatuh ke 0 - kedua-duanya ditolak.
        assert_eq!(
            text_of(&asmaulhusna(ctx_text("99999999999999999999"))),
            "❌ Nomor harus antara 1-99!"
        );
    }

    #[test]
    fn asmaulhusna_list_branch_lists_exactly_33_entries() {
        let expected = render_asmaul_page1();
        for q in ["all", "semua", "ALL", "Semua"] {
            assert_eq!(text_of(&asmaulhusna(ctx_text(q))), expected, "query {q}");
        }
        assert!(expected.starts_with("☪️ *ASMAUL HUSNA*\n> 99 Nama Allah SWT\n\n```1. Ar Rahman\n"));
        assert!(expected.contains("\n33. Al ‘Azhiim\n```\n> Halaman 1/3\n\n"));
        assert!(!expected.contains("34. "), "halaman 1 hanya 33 entri");
        assert!(expected.ends_with("_Gunakan .asmaulhusna [nomor] untuk detail_"));
    }

    #[test]
    fn asmaulhusna_search_branch_is_case_insensitive_and_substring() {
        for q in ["ar rahman", "AR RAHMAN", "Ar Rahman", "rahman", "RAHMA"] {
            assert!(
                text_of(&asmaulhusna(ctx_text(q))).contains("Ar Rahman"),
                "query {q}"
            );
        }
        assert_eq!(
            text_of(&asmaulhusna(ctx_text("zzz"))),
            "❌ Nama tidak ditemukan!\n_Coba nomor 1-99 atau nama latin_"
        );
    }

    #[test]
    fn asmaulhusna_search_returns_the_first_matching_entry() {
        // "al" cocok dengan banyak entri; JS `find` ambil yang pertama.
        assert_eq!(asmaul_by_latin("al").unwrap().index, 3);
        assert_eq!(asmaul_by_latin("as").unwrap().index, 5);
    }

    #[test]
    fn asmaulhusna_empty_query_picks_a_random_entry_from_the_table() {
        let p = oktz_plugin_api::find("asmaulhusna").unwrap();
        let mut seen = [false; 99];
        for _ in 0..4000 {
            let out = text_of(&(p.handler)(ctx_text("")));
            let idx: usize = out
                .split("📍 Nomor : ")
                .nth(1)
                .and_then(|s| s.split('\n').next())
                .unwrap_or_else(|| panic!("bentuk nomor berubah: {out}"))
                .parse()
                .expect("nomor harus angka");
            assert!((1..=99).contains(&idx), "nomor {idx} di luar tabel");
            seen[idx - 1] = true;
            assert!(out.starts_with("☪️ *ASMAUL HUSNA*\n\n```"));
            assert!(out.ends_with(&format!(
                "> 🇬🇧 Arti (EN): {}",
                ASMAUL_HUSNA[idx - 1].translation_en
            )));
        }
        assert!(
            seen.iter().all(|s| *s),
            "ada entri yang tak akan pernah muncul"
        );
    }

    #[test]
    fn render_asmaul_closes_the_fence_right_after_the_arabic_line() {
        let out = render_asmaul(&ASMAUL_HUSNA[0]);
        assert!(out.starts_with("☪️ *ASMAUL HUSNA*\n\n```"));
        assert!(out.contains("```\n\n> 🇮🇩 Arti (ID): "));
        assert!(out.contains("\n> 🇬🇧 Arti (EN): The All Beneficent"));
    }
}
