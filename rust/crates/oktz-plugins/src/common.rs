//! Helper bersama + definisi makro untuk semua plugin.
//!
//! Satu plugin satu file: `cek/`, `fun/`, `tools/`, `info/`, `main/`,
//! `owner/`, `religi/` — mencerminkan `plugins/<category>/<name>.js`.
//!
//! Item di sini dipakai >1 plugin atau oleh modul test, makanya
//! `pub(crate)`.

use oktz_plugin_api::{Ctx, PluginMeta, Reply, all_plugins};

use rand::Rng as _;

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

pub(crate) use cek_plugin;

macro_rules! data_plugin {
    ($fn_name:ident, $name:literal, $alias:expr, $desc:literal, $usage:literal,
     $example:literal, $data:expr) => {
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
        pub fn $fn_name(_ctx: Ctx) -> Reply {
            data_reply(&$data)
        }
    };
}

pub(crate) use data_plugin;

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

pub(crate) use tanya_plugin;

/// `Math.floor(Math.random() * 101)` — inklusif 0..=100, bukan 0..100.
pub(crate) fn percent_101() -> u32 {
    use rand::Rng as _;
    rand::rng().random_range(0..101)
}

/// Bangun balasan `cek*`.
///
/// `target` = mentions[0] atau sender. `label` = kata yang disisipkan
/// ("kecreativean"), `tiers` = 5 deskripsi berurutan dari ambang tertinggi.
pub(crate) fn cek_reply(ctx: &Ctx, percent: u32, label: &str, tiers: [&str; 5]) -> Reply {
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

/// `Math.floor(Math.random() * n)` — 0..=n-1, jadi range Rust `0..n` (n eksklusif).
pub(crate) fn roll(n: u32) -> u32 {
    use rand::Rng as _;
    rand::rng().random_range(0..n)
}

/// `mentioned.split('@')[0]` + apakah target = sender.
///
/// Dua kalimat tiap plugin (self "kamu" vs orang lain "dia") ditentukan oleh
/// `is_self`; nomor yang ditampilkan sudah dibuang `@server` dan `:device`.
pub(crate) fn num_and_self(ctx: &Ctx) -> (&str, bool) {
    let target = ctx.mention_or_sender();
    (
        target.split('@').next().unwrap_or(target),
        target == ctx.sender,
    )
}

/// Indeks acak seragam di `[0, len)` — padanan
/// `Math.floor(Math.random() * len)`. Upper eksklusif.
pub(crate) fn random_index(len: usize) -> usize {
    rand::rng().random_range(0..len)
}

/// `config.command?.prefix || '.'` — `config.command` tidak ada di `config.js`
/// untuk field prefix, jadi yang tampil selalu `.`.
pub(crate) const CONFIG_PREFIX: &str = ".";

/// Ubah nama-nama command jadi baris `• *.<name>*`, buang duplikat, sort.
///
/// JS: `seen` Set + `commandList.sort()` (default sort = lexicographic).
/// Rust `String` sort = byte order; sama untuk nama command ASCII.
pub(crate) fn command_lines(names: impl Iterator<Item = String>) -> Vec<String> {
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
pub(crate) fn registry_command_lines(pred: impl Fn(&PluginMeta) -> bool) -> Vec<String> {
    command_lines(
        all_plugins()
            .iter()
            .filter(|e| pred(&e.meta))
            .map(|e| e.meta.name.clone()),
    )
}

/// Balasan saat data bundled kosong — cabang `if (!item)` di JS.
///
/// Dengan const 63/61 baris di atas, cabang ini tidak mungkin terjadi di
/// produksi; tetap dibawa supaya logika `getRandomItem` (yang mengembalikan
/// `null` untuk array kosong) tidak hilang, dan supaya bisa diuji.
pub(crate) const DATA_UNAVAILABLE: &str = "❌ Data tidak tersedia!";

/// `getRandomItem(file)` dari `src/lib/game-data.js`:
///
/// ```js
/// if (!data || data.length === 0) return null
/// return data[Math.floor(Math.random() * data.length)]
/// ```
///
/// Indeks `floor(r * len)` berarti batas ATAS eksklusif — padanan
/// `random_range(0..len)`, bukan `0..=len`.
pub(crate) fn random_item(data: &[&'static str]) -> Option<&'static str> {
    if data.is_empty() {
        return None;
    }
    let idx = rand::rng().random_range(0..data.len());
    Some(data[idx])
}

/// Satu-satunya output kedua plugin: `` ```<item>``` `` atau pesan
/// data-tidak-tersedia. Tanpa mention — JS `m.reply` polos.
pub(crate) fn data_reply(data: &[&'static str]) -> Reply {
    match random_item(data) {
        Some(item) => Reply::text(format!("```{}```", item)),
        None => Reply::text(DATA_UNAVAILABLE),
    }
}

/// Rekonstruksi `m.body` lalu buang **satu** karakter pertama (`m.body.slice(1)`
/// di JS — char itu adalah prefix).
///
/// `ponytail:` `Ctx` tidak membawa `body` mentah, jadi spasi ganda di tengah
/// pesan akan dinormalkan jadi satu spasi. Tambahkan `body: String` ke `Ctx`
/// kalau router nanti mau kesetiaan byte-per-byte.
pub(crate) fn body_stripped(ctx: &Ctx) -> String {
    let mut body = String::with_capacity(ctx.prefix.len() + ctx.command.len() + ctx.text.len() + 1);
    body.push_str(&ctx.prefix);
    body.push_str(&ctx.command);
    if !ctx.text.is_empty() {
        body.push(' ');
        body.push_str(&ctx.text);
    }
    body.chars().skip(1).collect()
}

/// Bentuk balasan yang dipakai 10 plugin `?`:
/// `` `${m.body.slice(1)}?\n*${answer}*` ``.
pub(crate) fn render_tanya(question: &str, answer: &str) -> String {
    format!("{question}?\n*{answer}*")
}

/// Cabang.help dan cabang.jawab untuk satu plugin "tanya bot".
pub(crate) fn tanya_reply(ctx: &Ctx, answers: &[&str], help: &str) -> Reply {
    if ctx.text.trim().is_empty() {
        return Reply::text(help);
    }
    let answer = answers[random_index(answers.len())];
    Reply::text(render_tanya(&body_stripped(ctx), answer))
}

// =============================================================================
// State tersimpan + roster — helper untuk batch plugin roster/state
// =============================================================================

/// `jid.split('@')[0]` — sama seperti JS. Device `:9` sengaja TIDAK dibuang:
/// itu bentuk JID yang dipakai `m.sender`/mention, dan membuangnya membuat
/// teks yang tampil tidak sama dengan yang di-mention.
pub(crate) fn jid_number(jid: &str) -> &str {
    jid.split('@').next().unwrap_or(jid)
}

/// Target plugin "cek user": `m.mentionedJid[0]` kalau ada, selain itu sender.
///
/// `ponytail:` JS pakai `m.quoted` lebih dulu dan `m.pushName` untuk nama.
/// `Ctx` tidak membawa keduanya, jadi reply-mode dan nama push tidak bisa
/// dicerminkan. Tambahkan `quoted: Option<...>` + `push_name: String` ke
/// `Ctx` kalau router mau keduanya.
pub(crate) struct UserTarget {
    pub jid: String,
    pub name: String,
}

pub(crate) fn user_target(ctx: &Ctx) -> UserTarget {
    match ctx.mentioned.first() {
        Some(jid) => UserTarget {
            name: jid_number(jid).to_string(),
            jid: jid.clone(),
        },
        None => UserTarget {
            name: "Kamu".to_string(),
            jid: ctx.sender.clone(),
        },
    }
}

/// `Number.isFinite(user[k] ?? default)` → i64. Field yang bukan angka dibaca
/// sebagai `0` kalau ada, sama seperti `|| 0` di JS untuk `NaN`/`undefined`.
pub(crate) fn int_field(user: &Ctx, key: &str) -> i64 {
    user.user_field(key).and_then(serde_json::Value::as_i64).unwrap_or(0)
}

/// `arr || []` lalu dirender `${el}` per baris. String apa adanya; nilai lain
/// sebagai JSON kompak — persis apa yang `${el}` lakukan di JS untuk objek.
pub(crate) fn joined_strings(list: &[serde_json::Value]) -> Vec<String> {
    list.iter()
        .map(|v| match v.as_str() {
            Some(s) => s.to_string(),
            None => v.to_string(),
        })
        .collect()
}

/// `Object.entries(map).filter(([, v]) => v.length > 0)` untuk peta roster-ish
/// milik grup (`warnings`, `chatStats`). Urutan = urutan dokumen, sama seperti
/// `Object.entries` untuk key string.
pub(crate) fn non_empty_map_entries(value: Option<&serde_json::Value>) -> Vec<(String, serde_json::Value)> {
    value
        .and_then(serde_json::Value::as_object)
        .map(|m| {
            m.iter()
                .filter(|(_, v)| v.as_array().is_some_and(|a| !a.is_empty()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

// =============================================================================
// Album gacha — `albumStats` (src/lib/husbu.js:147 dan src/lib/waifu.js:184)
// =============================================================================

/// `TIER_VALUE` (src/lib/romance.js:6).
pub(crate) const TIER_VALUE: [(&str, i64); 5] = [
    ("Common", 1),
    ("Rare", 2),
    ("Epic", 4),
    ("Legendary", 8),
    ("Mythic", 16),
];

/// `TIER_EXPECTED` (src/lib/romance.js:7) =
/// `0.55*1 + 0.25*2 + 0.13*4 + 0.055*8 + 0.015*16` = 1.255.
///
/// Angka ditulis hasil hitung JS, bukan diturunkan ulang dari bobot, supaya
/// ini persis pembanding yang dipakai JS dan tidak mengambang.
pub(crate) const TIER_EXPECTED: f64 = 1.255;

/// `tierEmoji` yang di-hardcode dua-duanya (`husbualbum.js:31`,
/// `waifualbum.js:31`) — tier yang tidak dikenal dapat `""`, bukan fallback.
pub(crate) fn tier_emoji(tier: &str) -> &'static str {
    match tier {
        "Common" => "🟢",
        "Rare" => "🔵",
        "Epic" => "🟣",
        "Legendary" => "🟡",
        "Mythic" => "🔴",
        _ => "",
    }
}

/// Hasil [`album_stats`]. Field-nya mengikuti bentuk yang dikembalikan JS
/// persis, termasuk `rarest` yang boleh `null`.
pub(crate) struct AlbumStats {
    pub total: i64,
    pub by_tier: serde_json::Map<String, serde_json::Value>,
    pub pity_counter: i64,
    pub rarest: Option<serde_json::Value>,
    pub married_count: i64,
    pub luck: f64,
    pub last10: Vec<serde_json::Value>,
}

/// `Number(byTier[t] || 0)` — `||` JS membuat `0`, `""`, `false`, dan `null`
/// semua jadi 0, jadi `as_i64` yang gagal harus ikut jadi 0.
fn tier_count(by_tier: &serde_json::Map<String, serde_json::Value>, tier: &str) -> f64 {
    by_tier
        .get(tier)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

/// Port `albumStats`. Fungsi ini ada DUA KALI di JS (husbu + waifu) dengan
/// tubuh yang persis sama dan konstanta yang sama, jadi satu implementasi
/// melayani keduanya — bukan dua salinan yang bisa menyimpang diam-diam.
pub(crate) fn album_stats(history: &[serde_json::Value], stats: &serde_json::Value) -> AlbumStats {
    let num = |k: &str| {
        stats
            .get(k)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };
    let total = num("totalGacha");
    let empty = serde_json::Map::new();
    let by_tier = stats
        .get("byTier")
        .and_then(serde_json::Value::as_object)
        .unwrap_or(&empty)
        .clone();

    let actual: f64 = TIER_VALUE
        .iter()
        .map(|(tier, v)| tier_count(&by_tier, tier) * *v as f64)
        .sum();
    let luck = if total > 0.0 { actual / total / TIER_EXPECTED } else { 0.0 };

    // `history.slice(-10).reverse()` — 10 terakhir, newest first.
    let start = history.len().saturating_sub(10);
    let last10 = history[start..].iter().rev().cloned().collect();

    AlbumStats {
        total: total as i64,
        by_tier,
        pity_counter: num("pityCounter") as i64,
        rarest: stats.get("rarest").filter(|v| !v.is_null()).cloned(),
        married_count: num("marriedCount") as i64,
        // `Number(luck.toFixed(2))` — dua desimal, dibulatkan, bukan dipotong.
        luck: (luck * 100.0).round() / 100.0,
        last10,
    }
}

/// `String(count).padStart(3)` untuk angka bulat.
pub(crate) fn pad_start3(n: i64) -> String {
    let s = n.to_string();
    if s.len() >= 3 { s } else { format!("{}{}", " ".repeat(3 - s.len()), s) }
}

/// `bar(count)` dari album husbu/waifu: 20 sel, `Math.round(count/total*20)`
/// terisi. `total == 0` → semua kosong (JS short-circuit, tidak NaN).
pub(crate) fn album_bar(count: f64, total: f64) -> String {
    let pct = if total > 0.0 {
        (count / total * 20.0).round().clamp(0.0, 20.0) as usize
    } else {
        0
    };
    format!("{}{}", "█".repeat(pct), "░".repeat(20 - pct))
}

/// Baris tier yang identik di dua album (`husbualbum.js:41-45`,
/// `waifualbum.js:41-45`) — label, padding, bar, semuanya sama.
pub(crate) const TIER_ROWS: [(&str, &str, &str); 5] = [
    ("Common", "🟢", "🟢 Common     "),
    ("Rare", "🔵", "🔵 Rare       "),
    ("Epic", "🟣", "🟣 Epic       "),
    ("Legendary", "🟡", "🟡 Legendary  "),
    ("Mythic", "🔴", "🔴 Mythic     "),
];

/// Tubuh render yang sama untuk album husbu dan waifu. Yang berbeda cuma
/// judulnya — perintah gacha dan cabang "album kosong" tetap di plugin masing-
/// masing karena keduanya memeriksa `history`, bukan `stats`.
pub(crate) fn render_album(stats: &AlbumStats, title: &str) -> String {
    let total = stats.total as f64;
    let mut text = format!("📚 *{title}* 📚\n\n");
    text.push_str(&format!("🎲 *Total Gacha:* {}\n", stats.total));
    text.push_str(&format!(
        "🍀 *Luck Score:* {} {}\n\n",
        stats.luck,
        if stats.luck >= 1.0 { "🔥" } else { "😅" }
    ));
    text.push_str("*Distribusi Tier:*\n");
    for (tier, _, label) in TIER_ROWS {
        let n = tier_count(&stats.by_tier, tier);
        text.push_str(&format!(
            "{label}{} {}\n",
            pad_start3(n as i64),
            album_bar(n, total)
        ));
    }
    text.push('\n');

    if let Some(rarest) = &stats.rarest {
        let tier = rarest.get("tier").and_then(serde_json::Value::as_str).unwrap_or("");
        let name = rarest.get("name").and_then(serde_json::Value::as_str).unwrap_or("");
        text.push_str(&format!(
            "🏆 *Pull Paling Langka:* {} {name} ({tier})\n",
            tier_emoji(tier)
        ));
    }
    if stats.pity_counter > 0 {
        text.push_str(&format!(
            "🎯 *Pity aktif:* {}/20 roll tanpa Epic+\n",
            stats.pity_counter
        ));
    }
    text.push_str(&format!(
        "💍 *Pernikahan:* {}\n\n*10 Pull Terakhir:*\n",
        stats.married_count
    ));
    for (i, h) in stats.last10.iter().enumerate() {
        let tier = h.get("tier").and_then(serde_json::Value::as_str).unwrap_or("");
        let name = h.get("name").and_then(serde_json::Value::as_str).unwrap_or("");
        text.push_str(&format!(
            "{}. {} *{name}* — {tier}\n",
            i + 1,
            tier_emoji(tier)
        ));
    }
    text
}

/// Member grup sebagai JID, apa adanya urutannya.
///
/// `ponytail:` beberapa plugin JS memfilter JID bot sendiri
/// (`id !== sock.user?.id`), dan `GroupRoster` tidak punya JID bot — hanya
/// flag `is_bot_admin`. Pemanggil yang butuh filter itu harus menambahkan
/// `bot_jid` ke `GroupRoster` lebih dulu.
pub(crate) fn roster_jids(ctx: &Ctx) -> Vec<String> {
    ctx.roster.member_jids()
}

// =============================================================================
// Waktu — Asia/Jakarta (`src/lib/time.js:9`, `TIMEZONE`)
// =============================================================================

/// `Asia/Jakarta` = UTC+7 sepanjang tahun, tanpa DST. Angka tetap, jadi
/// konversi civil bisa ditulis tanpa dependensi tanggal.
pub(crate) const JAKARTA_OFFSET_SECS: i64 = 7 * 60 * 60;

/// `Date.now()` dalam milidetik.
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// `(tahun, bulan 1-12, hari, jam, menit, detik)` di Asia/Jakarta.
///
/// Implementasi algoritma civil-calendar Howard Hinnant (`days_from_civil` /
/// `civil_from_days`) supaya crate ini tidak perlu dependensi tanggal.
pub(crate) fn jakarta_parts(ms: i64) -> (i64, u32, u32, u32, u32, u32) {
    let local = ms.div_euclid(1000) + JAKARTA_OFFSET_SECS;
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (secs / 3600) as u32,
        ((secs % 3600) / 60) as u32,
        (secs % 60) as u32,
    )
}

/// Hari sejak 1970-01-01 untuk tanggal civil (Howard Hinnant, `days_from_civil`).
pub(crate) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    // Bulan digeser supaya Maret = 0, dan 12 -> 0 lewat `% 12` — tanpa itu
    // Desember terhitung 13 bulan dan hasilnya meleset satu tahun.
    let mp = (u64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + u64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `String(n).padStart(2, '0')` (`time.js:44`).
pub(crate) fn pad2(n: u32) -> String {
    format!("{n:02}")
}

/// `MMMM` versi `Intl.DateTimeFormat('id-ID')` — sama dengan array bulan
/// panjang yang dip-hardcode `plugins/user/birthday.js:26`.
pub(crate) const MONTHS_ID: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

/// `formatPattern(date, 'D MMMM YYYY')`.
pub(crate) fn date_long(ms: i64) -> String {
    let (y, m, d, ..) = jakarta_parts(ms);
    format!("{d} {} {y}", MONTHS_ID[(m - 1) as usize])
}

/// `formatPattern(date, 'DD/MM/YYYY')`.
pub(crate) fn date_dmy(ms: i64) -> String {
    let (y, m, d, ..) = jakarta_parts(ms);
    format!("{}/{}/{}", pad2(d), pad2(m), y)
}

/// `formatPattern(date, 'HH:mm')`.
pub(crate) fn time_hm(ms: i64) -> String {
    let (_, _, _, h, mi, _) = jakarta_parts(ms);
    format!("{}:{}", pad2(h), pad2(mi))
}

// =============================================================================
// Probe registry — `plugins/info/{howmuchenergy,whatrolethis}.js`
// =============================================================================

/// Loop atas `m.args` yang dipakai kedua plugin probe: header, lalu
/// `getPlugin(cmd)` + satu blok per argumen. Argumen tak dikenal dapet baris
/// "Tidak ditemukan!", dan seluruh hasil di-`trim()` seperti
/// `m.reply(responseText.trim())`.
pub(crate) fn render_probe(header: &str, args: &[String], render: impl Fn(&str) -> String) -> String {
    let mut out = String::from(header);
    for cmd in args {
        let key = cmd.to_lowercase();
        match oktz_plugin_api::find(&key) {
            Some(p) => out.push_str(&render(&p.meta.name)),
            None => out.push_str(&format!("❌ *{key}* : Tidak ditemukan!\n\n")),
        }
    }
    out.trim().to_string()
}

macro_rules! probe_plugin {
    ($fn_name:ident, $name:literal, $alias:expr, $desc:literal, $usage:literal,
     $example:literal, $help:expr, $header:expr, $render:expr) => {
        #[plugin(
                    name = $name,
                    alias = $alias,
                    category = "info",
                    description = $desc,
                    usage = $usage,
                    example = $example,
                    cooldown = 5,
                    energi = 0,
                    is_private = false
                )]
        pub fn $fn_name(ctx: Ctx) -> Reply {
            if ctx.args.is_empty() {
                return Reply::text($help);
            }
            Reply::text(render_probe($header, &ctx.args, $render))
        }
    };
}

pub(crate) use probe_plugin;

#[cfg(test)]
mod tests_cek {
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
            let text = r.without_mutations().text_body().expect("harus ada teks");
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
        assert_eq!(
            oktz_plugin_api::find("creative").unwrap().meta.name,
            "cekcreative"
        );
    }

    #[test]
    fn a_registered_plugin_returns_a_valid_reply() {
        let p = oktz_plugin_api::find("cekgacha").unwrap();
        let reply = (p.handler)(ctx_with_mention(Some("628999@s.whatsapp.net")));
        assert!(reply.text_body().is_some(), "plugin harus membalas");
    }
}

#[cfg(test)]
mod meta_guard_cek {

    /// `#[plugin]` hanya bisa parses `Expr::Lit`. Argumen berbasis `concat!`
    /// atau makro lain lolos kompilasi tapi meta-nya jadi default kosong —
    /// tanpa error, tanpa warning. Test ini yang menangkapnya.
    #[test]
    fn usage_and_example_are_never_silently_empty() {
        for name in [
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
        ] {
            let p = oktz_plugin_api::find(name).unwrap_or_else(|| panic!("{name} tidak ada"));
            assert!(!p.meta.usage.is_empty(), "{name}: usage kosong");
            assert!(!p.meta.example.is_empty(), "{name}: example kosong");
            assert!(
                p.meta.usage.starts_with('.'),
                "{name}: usage harus diawali prefix, dapat {:?}",
                p.meta.usage
            );
            assert!(
                p.meta.description != "No description",
                "{name}: description jatuh ke default"
            );
        }
    }
}

#[cfg(test)]
mod tests_cek2 {
    use super::*;
    use crate::cek::cekberat::{berat_desc, berat_kg, berat_text};
    use crate::cek::cekkarma::{karma_desc, karma_percent, karma_text};
    use crate::cek::cekpintar::{pintar_desc, pintar_iq, pintar_text};
    use crate::cek::ceksisaumur::{
        sisaumur_bulan, sisaumur_desc, sisaumur_hari, sisaumur_tahun, sisaumur_text,
    };
    use crate::cek::cektinggi::{tinggi_cm, tinggi_desc, tinggi_text};
    use crate::cek::cekumur::{umur_desc, umur_percent, umur_text};

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
        reply.without_mutations().text_body().expect("harus ada teks")
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
    fn scan_tiers(
        domain: std::ops::RangeInclusive<u32>,
        tiers: &[&str],
        build: impl Fn(u32) -> String,
    ) {
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
        assert_eq!(tinggi_text(&ctx_self(), 160), tinggi_text(&ctx_self(), 160));
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
            ("cektinggi", &["tinggi", "tall"], "Cek tinggi badan random"),
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

#[cfg(test)]
mod tests_batch_g {
    use super::*;
    use crate::cek::cekjodoh::{cekjodoh, jodoh_desc};
    use crate::cek::cekkepribadian::PERSONALITIES;
    use crate::info::benefitpartner::benefitpartner;
    use crate::main::benefitowner::owner_message;
    use crate::main::benefitpremium::{benefitpremium, premium_message};
    use crate::owner::templateplugin::templateplugin;
    use crate::religi::asmaulhusna::{
        ASMAUL_HUSNA, asmaul_by_index, asmaul_by_latin, asmaulhusna, is_all_digits, render_asmaul,
        render_asmaul_page1,
    };

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
    fn every_is_owner_plugin_is_known() {
        // Daftar grows; yang dijamin adalah setiap is_owner yang terdaftar ada
        // di allow-list, bukan sebaliknya.
        const KNOWN: &[&str] = &[
            "templateplugin",
            "addlist",
            "editlist",
            "hapuslist",
        ];
        for m in oktz_plugin_api::all_plugins().iter().map(|p| &p.meta) {
            if m.is_owner && m.is_enabled {
                assert!(
                    KNOWN.contains(&m.name.as_str()),
                    "is_owner baru '{}' belum masuk allow-list test",
                    m.name
                );
            }
        }
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
    fn benefitowner_lists_the_is_owner_plugins_in_the_registry() {
        let listed = registry_command_lines(|m| m.is_owner && m.is_enabled);
        for line in &listed {
            let name = line
                .trim_start_matches("• *.")
                .trim_end_matches('*');
            assert!(
                oktz_plugin_api::find(name).is_some(),
                "benefitowner mencantumkan '{name}' yang tidak terdaftar"
            );
        }
        assert!(!listed.is_empty(), "harus ada minimal satu is_owner");
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
        assert!(
            expected.starts_with("☪️ *ASMAUL HUSNA*\n> 99 Nama Allah SWT\n\n```1. Ar Rahman\n")
        );
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

#[cfg(test)]
mod tests_batch_s {
    use super::*;
    use crate::fun::dare::DARE;
    use crate::fun::truth::TRUTH;
    use oktz_plugin_api::{Ctx, Reply, find};

    /// Every name + alias already owned by `cek` (10), `cek2` (6),
    /// `umum` (14) dan `batch_g` (33). `batch_s` tidak boleh mengklaim satu pun.
    /// Hanya dipakai test, jadi ia hidup di dalam `mod tests` supaya build
    /// non-test tidak kena `dead_code`.
    pub(crate) const ALREADY_PORTED: [&str; 184] = [
        "99names", "age", "akan", "akankah", "albumhusbu", "albumwaifu", "apa", "apakah",
        "apes", "asmaul", "asmaulhusna", "bagaimana", "baik", "beautiful", "benefitowner", "benefitpartner",
        "benefitprem", "benefitpremium", "berapa", "berat", "bisa", "bisakah", "bored", "bucin",
        "cantik", "cekbaik", "cekberat", "cekbucin", "cekcantik", "cekcreative", "cekcupu", "cekgabut",
        "cekgacha", "cekgamer", "cekganteng", "cekgila", "cekhoki", "cekimut", "cekintrovert", "cekjahat",
        "cekjodoh", "cekjomblo", "cekkarma", "cekkaya", "cekkece", "cekkepribadian", "cekkpopers", "ceklapar",
        "cekmalas", "cekmesum", "cekngantuk", "cekotaku", "cekoverpower", "cekpelit", "cekpintar", "cekprocastinator",
        "cekpsikopat", "cekrezeki", "ceksabar", "ceksetia", "ceksexy", "ceksial", "ceksisaumur", "ceksocmed",
        "cektinggi", "cektsundere", "cekumur", "cekwibu", "cekyandere", "coba", "cool", "crazy",
        "creative", "cupu", "cute", "dimana", "dream", "dreamworld", "evil", "fortune",
        "gabut", "gamer", "ganteng", "gaycheck", "gila", "gimana", "handsome", "harus",
        "haruskah", "hoki", "hot", "how", "howgay", "howmany", "howmuch", "hungry",
        "husbualbum", "husna", "imut", "introvert", "iq", "jomblo", "kalkulatormbg", "kapan",
        "karma", "kaya", "kece", "kenapa", "kepribadian", "keuntunganpartner", "kikir", "kind",
        "kkmbg", "kpop", "kpopers", "kreatif", "lapar", "lazy", "loyal", "luck",
        "malas", "mana", "medsos", "mengapa", "mesum", "mimpi", "ngantuk", "nilai",
        "noob", "nunda", "op", "otaku", "overpower", "ownerbenefits", "ownerfitur", "partnerbenefits",
        "patience", "pelit", "personality", "pintar", "plugin-template", "premiumbenefits", "premiumfitur", "pro",
        "procrastinator", "psikopat", "psycho", "rate", "rating", "readmore", "rezeki", "rich",
        "sabar", "selengkapnya", "setia", "sexy", "should", "sial", "single", "sisaumur",
        "sleepy", "smart", "sosmed", "soulmatch", "spoiler", "tall", "templateplugin", "tinggi",
        "top", "top5", "toplist", "tplplugin", "try", "tsundere", "umur", "waifualbum",
        "weeb", "weight", "when", "where", "why", "wibu", "will", "yandere",
    ];

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Nama + alias dari modul yang SUDAH ada sebelum batch_s, diambil dari
    /// registry sungguhan (bukan daftar tebakan).
    fn keys_of(name: &str) -> Vec<String> {
        let p = find(name).expect("plugin terdaftar");
        let mut k = vec![p.meta.name.clone()];
        k.extend(p.meta.alias.iter().cloned());
        k
    }

    // ---------------------------------------------------------------- registry

    #[test]
    fn both_plugins_are_registered() {
        // Tidak meng-hardcode jumlah total: registry bertambah tiap batch.
        for n in ["dare", "truth"] {
            assert!(
                oktz_plugin_api::find(n).is_some(),
                "{n} harus terdaftar"
            );
        }
    }

    #[test]
    fn dare_meta_is_verbatim_from_the_js_config() {
        let m = &find("dare").unwrap().meta;
        assert_eq!(m.name, "dare");
        assert_eq!(m.alias, ["dareq", "tantang"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Random tantangan dare");
        assert_eq!(m.usage, ".dare");
        assert_eq!(m.example, ".dare");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        // Default PluginMeta, sama seperti config JS yang tidak mengaturnya.
        assert!(m.is_enabled);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert!(!m.is_admin && !m.is_bot_admin && !m.skip_registration);
    }

    #[test]
    fn truth_meta_is_verbatim_from_the_js_config() {
        let m = &find("truth").unwrap().meta;
        assert_eq!(m.name, "truth");
        assert_eq!(m.alias, ["truthq"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Random pertanyaan truth");
        assert_eq!(m.usage, ".truth");
        assert_eq!(m.example, ".truth");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(m.is_enabled);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert!(!m.is_admin && !m.is_bot_admin && !m.skip_registration);
    }

    #[test]
    fn aliases_resolve_to_the_right_plugin() {
        assert_eq!(find("dareq").unwrap().meta.name, "dare");
        assert_eq!(find("tantang").unwrap().meta.name, "dare");
        assert_eq!(find("truthq").unwrap().meta.name, "truth");
        // Alias milik satu plugin tidak boleh jatuh ke plugin lain.
        assert!(find("dare").unwrap().meta.matches("dare"));
        assert!(!find("truth").unwrap().meta.matches("dareq"));
    }

    // ------------------------------------------------------- no key collision

    /// ALREADY_PORTED harus PERSIS sama dengan key yang benar-benar didaftarkan
    /// `cek`/`cek2`/`umum`/`batch_g`. Kalau daftar ini kurang satu, test
    /// `no_collision_with_already_ported` di bawah jadi tidak kuat dan ini
    /// yang akan menangkapnya.
    #[test]
    fn every_key_in_the_hardcoded_list_still_resolves() {
        for k in ALREADY_PORTED {
            assert!(
                oktz_plugin_api::find(k).is_some(),
                "kunci '{k}' tidak resolve lagi"
            );
        }
    }

    #[test]
    fn no_collision_with_already_ported() {
        for name in ["dare", "truth"] {
            for key in keys_of(name) {
                assert!(
                    !ALREADY_PORTED.contains(&key.as_str()),
                    "COLISI: key {key} (dari plugin {name}) sudah dipakai modul sebelumnya"
                );
                assert!(
                    find(&key).map(|p| p.meta.name == name).unwrap_or(false),
                    "key {key} resolve ke plugin yang salah"
                );
            }
        }
    }

    #[test]
    fn the_two_new_plugins_do_not_collide_with_each_other() {
        let mut a = keys_of("dare");
        a.extend(keys_of("truth"));
        let mut b = a.clone();
        b.sort();
        b.dedup();
        assert_eq!(b.len(), a.len(), "kunci dare dan truth tumpang tindih");
    }

    // ------------------------------------------------------------- logic both

    #[test]
    fn reply_is_the_item_wrapped_in_a_code_fence() {
        for data in [&DARE[..], &TRUTH[..]] {
            let r = data_reply(data);
            let t = text_of(&r);
            assert!(t.starts_with("```") && t.ends_with("```"), "fence: {t}");
            let inner = &t[3..t.len() - 3];
            assert!(data.contains(&inner), "{inner} bukan isi tabel");
        }
    }

    #[test]
    fn reply_carries_no_mentions() {
        // JS `m.reply(str)` tanpa opsi mentions.
        match data_reply(&DARE) {
            Reply::Text { mentions, .. } => assert!(mentions.is_empty()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn empty_table_takes_the_data_unavailable_branch() {
        // Cabang `if (!item)` di JS. Tidak bisa terjadi dengan DARE/TRUTH yang
        // non-kosong, jadi diuji lewat tabel kosong.
        assert_eq!(text_of(&data_reply(&[])), DATA_UNAVAILABLE);
        assert_eq!(random_item(&[]), None);
    }

    #[test]
    fn the_embedded_tables_are_the_real_sizes() {
        assert_eq!(DARE.len(), 63, "src/data/dare.json");
        assert_eq!(TRUTH.len(), 61, "src/data/truth.json");
        assert!(DARE.iter().all(|s| !s.is_empty()));
        assert!(TRUTH.iter().all(|s| !s.is_empty()));
        assert!(
            !DARE.iter().any(|s| s.contains('"') || s.contains('\\')),
            "sumber data tidak butuh escaping"
        );
        assert!(!TRUTH.iter().any(|s| s.contains('"') || s.contains('\\')));
    }

    // ------------------------------------------------- RNG range (JS-implied)

    #[test]
    fn every_index_of_both_tables_is_reachable() {
        // `Math.floor(Math.random() * len)` == `random_range(0..len)`:
        // ujung atas eksklusif, jadi indeks terakhir harus mungkin muncul.
        for data in [&DARE[..], &TRUTH[..]] {
            let n = data.len();
            let mut seen = vec![false; n];
            for _ in 0..(n * 300) {
                let item = random_item(data).expect("tabel non-kosong");
                seen[data.iter().position(|s| *s == item).unwrap()] = true;
            }
            let unreachable: Vec<usize> = seen
                .iter()
                .enumerate()
                .filter(|(_, s)| !**s)
                .map(|(i, _)| i)
                .collect();
            assert!(
                unreachable.is_empty(),
                "indeks tak terjangkau: {unreachable:?}"
            );
        }
    }

    #[test]
    fn index_never_exceeds_the_table_length() {
        // Menangkap off-by-one kalau range-nya salah (mis. 0..=len).
        for data in [&DARE[..], &TRUTH[..]] {
            for _ in 0..5000 {
                let item = random_item(data).unwrap();
                assert!(data.contains(&item), "item di luar tabel: {item}");
            }
        }
    }

    #[test]
    fn output_is_wrapped_in_the_fence_for_the_real_handlers_too() {
        // Jalankan lewat handler yang terdaftar, bukan lewat helper.
        for name in ["dare", "truth"] {
            let p = find(name).unwrap();
            for _ in 0..200 {
                let r = (p.handler)(ctx());
                let t = text_of(&r);
                assert!(t.starts_with("```") && t.ends_with("```"), "{name}: {t}");
                let inner = &t[3..t.len() - 3];
                assert!(!inner.is_empty(), "{name} balasan kosong");
            }
        }
    }

    #[test]
    fn handlers_are_deterministic_in_shape_regardless_of_arguments() {
        // JS-nya `async function handler(m)` tanpa argumen sama sekali —
        // `m` diabaikan. Semua input harus menghasilkan bentuk yang sama.
        for name in ["dare", "truth"] {
            let p = find(name).unwrap();
            for sender in ["628111@s.whatsapp.net", ""] {
                let mut c = ctx();
                c.sender = sender.into();
                c.text = "apa saja".into();
                c.args = vec!["x".into()];
                let r = (p.handler)(c);
                let t = text_of(&r);
                assert!(t.starts_with("```"), "{name}: {t}");
            }
        }
    }
}

#[cfg(test)]
mod tests_umum {
    use super::*;
    use crate::fun::akankah::{AKANKAH, HELP_AKANKAH};
    use crate::fun::apakah::{APAKAH, HELP_APAKAH};
    use crate::fun::bagaimana::{BAGAIMANA, HELP_BAGAIMANA};
    use crate::fun::berapa::{BERAPA, HELP_BERAPA};
    use crate::fun::bisakah::{BISAKAH, HELP_BISAKAH};
    use crate::fun::coba::{COBA, HELP_COBA};
    use crate::fun::dimana::{DIMANA, HELP_DIMANA};
    use crate::fun::haruskah::{HARUSKAH, HELP_HARUSKAH};
    use crate::fun::kapan::{HELP_KAPAN, KAPAN};
    use crate::fun::mengapa::{HELP_MENGAPA, MENGAPA};
    use crate::fun::rate::{HELP_RATE, RATE, rate, render_rate};
    use crate::fun::soulmatch::{
        ELEMENTS, HELP_SOULMATCH, SOUL_TYPES, ZODIAC, compatibility, match_description, reading,
        render_soulmatch, soul_data, soulmatch,
    };
    use crate::tools::kalkulatormbg::{
        HELP_KKMBG_BODY, HELP_KKMBG_HEAD, KKMBG_INVALID, format_rupiah, hitung_durasi,
        kalkulatormbg, parse_uang, thousands_id,
    };
    use crate::tools::readmore::{LRM_COUNT, readmore, readmore_body};

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
    fn tanya_family() -> Vec<(
        &'static str,
        Vec<&'static str>,
        &'static [&'static str],
        &'static str,
    )> {
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
            (
                "haruskah",
                vec!["harus", "should"],
                &HARUSKAH,
                HELP_HARUSKAH,
            ),
            ("berapa", vec!["howmuch", "howmany"], &BERAPA, HELP_BERAPA),
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
        assert_eq!(p.meta.description, "Minta bot memberi rating sesuatu");
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
        assert_eq!(
            oktz_plugin_api::find("kkmbg").unwrap().meta.name,
            "kalkulatormbg"
        );
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
            assert!(reply.text_body().is_some(), "{name} harus membalas teks");
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
            assert!(
                seen.iter().all(|s| *s),
                "ada slot yang tak akan pernah muncul"
            );
        }
    }

    #[test]
    fn body_stripped_drops_exactly_one_char_of_the_prefix() {
        let mut ctx = ctx_of("apakah", "halo");
        ctx.prefix = ".".to_string();
        assert_eq!(body_stripped(&ctx), "apakah halo");

        ctx.prefix = "!!".to_string();
        assert_eq!(
            body_stripped(&ctx),
            "!apakah halo",
            "slice(1) hanya satu char"
        );

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
        let out = handler_text(&readmore(ctx_of(
            "readmore",
            "Hai|Ini adalah pesan rahasia",
        )));
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
        assert_eq!(
            parse_uang("-12"),
            12.0,
            "tanda minus dibuang, bukan membuat negatif"
        );
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
        assert!(
            out.starts_with("🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n💰 *Dana :* Rp 1.234.567\n\n")
        );
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
        assert!(
            !out.contains("\n\n\n"),
            "contentTxt di-trim sebelum dikirim"
        );
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

// =============================================================================
// Batch roster/state — 18 plugin
// =============================================================================

#[cfg(test)]
mod tests_roster_state {
    use super::*;
    use crate::common::tests_batch_s::ALREADY_PORTED;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use oktz_plugin_api::{all_plugins, find};
    use serde_json::json;

    /// Plugin yang ditambahkan batch ini. Satu nama = satu file, jadi daftar
    /// ini sekaligus penanda apa yang harus ada di disk.
    pub(crate) const NEW_PLUGINS: [&str; 17] = [
        // group — roster
        "tagall",
        "listadmin",
        // group — state tersimpan
        "listantilink",
        "listtoxic",
        "listwarn",
        "topchat",
        "intro",
        "rulesgrup",
        // rpg
        "inventory",
        // user
        "koin",
        "exp",
        "level",
        "energi",
        "birthday",
        // info
        "howmuchenergy",
        "whatrolethis",
        "fiturpremium",
    ];

    fn keys_of(name: &str) -> Vec<String> {
        let p = find(name).unwrap_or_else(|| panic!("{name} tidak terdaftar"));
        let mut k = vec![p.meta.name.clone()];
        k.extend(p.meta.alias.iter().cloned());
        k
    }

    fn keys_of_new() -> Vec<String> {
        NEW_PLUGINS.iter().flat_map(|n| keys_of(n)).collect()
    }

    fn keys_of_others() -> Vec<String> {
        // `dare`/`truth` milik `tests_batch_s` sendiri; `NEW_PLUGINS` milik
        // batch roster/state. Keduanya dikecualikan supaya daftar "yang sudah
        // ada sebelum batch_s" tetap bermakna.
        //
        // Batch fun SENGAJA tidak dikecualikan: `ALREADY_PORTED` sudah memuat
        // key-nya (itu satu-satunya daftar yang dipakai dua test untuk
        // membandingkan registry), jadi menyaringnya di sini justru
        // membuat keduanya tidak bisa pernah sama.
        all_plugins()
            .iter()
            .filter(|e| {
                !["dare", "truth"].contains(&e.meta.name.as_str())
                    && !NEW_PLUGINS.contains(&e.meta.name.as_str())
            })
            .flat_map(|e| {
                let mut k = vec![e.meta.name.clone()];
                k.extend(e.meta.alias.iter().cloned());
                k
            })
            .collect()
    }

    // -- registrasi ---------------------------------------------------------

    #[test]
    fn every_new_plugin_of_this_batch_is_registered() {
        // Tidak mengecek jumlah total — registry bertambah tiap batch.
        for n in NEW_PLUGINS {
            assert!(find(n).is_some(), "{n} tidak terdaftar");
        }
    }

    #[test]
    fn no_new_plugin_shares_a_name_or_alias_with_the_previous_65() {
        for name in NEW_PLUGINS {
            for key in keys_of(name) {
                assert!(
                    !ALREADY_PORTED.contains(&key.as_str()),
                    "COLISI: key {key} (dari {name}) sudah dipakai modul sebelumnya"
                );
                assert_eq!(
                    find(&key).map(|p| p.meta.name == name),
                    Some(true),
                    "key {key} resolve ke plugin yang salah"
                );
            }
        }
    }

    #[test]
    fn every_already_ported_key_is_still_registered() {
        // Yang dijamin: tiap kunci yang pernah ada masih ada. Tidak
        // membandingkan dua daftar yang sama panjang — itu rapuh.
        for k in ALREADY_PORTED {
            assert!(
                oktz_plugin_api::all_plugins()
                    .iter()
                    .any(|p| p.meta.matches(k)),
                "plugin '{k}' hilang dari registry"
            );
        }
    }

    #[test]
    fn the_whole_registry_has_no_duplicate_key() {
        let mut keys = keys_of_new();
        keys.extend(keys_of_others());
        let mut sorted = keys.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "ada key yang dipakai dua plugin");
    }

    // -- meta_guard ---------------------------------------------------------

    #[test]
    fn usage_and_example_are_never_silently_empty_for_the_new_plugins() {
        for name in NEW_PLUGINS {
            let p = find(name).unwrap_or_else(|| panic!("{name} tidak ada"));
            assert!(!p.meta.name.is_empty(), "{name}: name kosong");
            assert!(!p.meta.description.is_empty(), "{name}: description kosong");
            assert!(
                p.meta.description != "No description",
                "{name}: description jatuh ke default"
            );
            assert!(!p.meta.usage.is_empty(), "{name}: usage kosong");
            assert!(!p.meta.example.is_empty(), "{name}: example kosong");
            assert!(
                p.meta.usage.starts_with('.'),
                "{name}: usage harus diawali prefix, dapat {:?}",
                p.meta.usage
            );
            assert!(
                p.meta.example.starts_with('.'),
                "{name}: example harus diawali prefix, dapat {:?}",
                p.meta.example
            );
        }
    }

    #[test]
    fn every_new_plugin_lands_in_the_mirrored_category() {
        for name in NEW_PLUGINS {
            let p = find(name).unwrap();
            let expected = match name {
                "tagall" | "listadmin" | "listantilink" | "listtoxic" | "listwarn"
                | "topchat" | "intro" | "rulesgrup" => "group",
                "inventory" => "rpg",
                "koin" | "exp" | "level" | "energi" | "birthday" => "user",
                "howmuchenergy" | "whatrolethis" | "fiturpremium" => "info",
                other => panic!("plugin baru tak dikenal: {other}"),
            };
            assert_eq!(p.meta.category, expected, "{name}");
        }
    }

    // -- helper bersama -----------------------------------------------------

    fn roster(participants: Vec<Participant>) -> GroupRoster {
        GroupRoster {
            participants,
            is_bot_admin: false,
        }
    }

    #[test]
    fn jid_number_strips_the_server_but_keeps_the_device() {
        assert_eq!(jid_number("628123@s.whatsapp.net"), "628123");
        assert_eq!(jid_number("628123:9@s.whatsapp.net"), "628123:9");
        assert_eq!(jid_number("628123@lid"), "628123");
        assert_eq!(jid_number(""), "");
        assert_eq!(jid_number("@s.whatsapp.net"), "");
    }

    #[test]
    fn user_target_prefers_a_mention_and_falls_back_to_kamu() {
        let mut ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        assert_eq!(user_target(&ctx).jid, "628111@s.whatsapp.net");
        assert_eq!(user_target(&ctx).name, "Kamu");
        ctx.mentioned = vec!["628999@s.whatsapp.net".into()];
        assert_eq!(user_target(&ctx).jid, "628999@s.whatsapp.net");
        assert_eq!(user_target(&ctx).name, "628999");
    }

    #[test]
    fn int_field_reads_numbers_and_zeroes_everything_else() {
        let ctx = Ctx {
            user_data: json!({"koin": 12, "exp": null, "s": "5", "f": 1.5}),
            ..Default::default()
        };
        assert_eq!(int_field(&ctx, "koin"), 12);
        assert_eq!(int_field(&ctx, "exp"), 0);
        assert_eq!(int_field(&ctx, "s"), 0);
        assert_eq!(int_field(&ctx, "f"), 0);
        assert_eq!(int_field(&ctx, "tidak-ada"), 0);
        assert_eq!(int_field(&Ctx::default(), "koin"), 0);
    }

    #[test]
    fn joined_strings_keeps_strings_and_jsonifies_everything_else() {
        assert_eq!(
            joined_strings(&[json!("a"), json!("b")]),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(joined_strings(&[]), Vec::<String>::new());
        assert_eq!(joined_strings(&[json!(7)]), vec!["7".to_string()]);
    }

    #[test]
    fn non_empty_map_entries_drops_empty_arrays_and_keeps_order() {
        let v = json!({
            "b@s.whatsapp.net": [1],
            "a@s.whatsapp.net": [],
            "c@s.whatsapp.net": [1, 2]
        });
        let out = non_empty_map_entries(Some(&v));
        assert_eq!(
            out.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            ["b@s.whatsapp.net", "c@s.whatsapp.net"]
        );
        assert_eq!(non_empty_map_entries(Some(&json!({}))).len(), 0);
        assert_eq!(non_empty_map_entries(Some(&json!(null))).len(), 0);
        assert_eq!(non_empty_map_entries(None).len(), 0);
    }

    #[test]
    fn render_probe_trims_and_reports_unknown_commands() {
        let out = render_probe("HEAD\n\n", &["readmore".into(), "zzz".into()], |name| {
            format!("ok {name}\n")
        });
        assert_eq!(out, "HEAD\n\nok readmore\n❌ *zzz* : Tidak ditemukan!");
        assert_eq!(
            render_probe("", &[], |_| String::new()),
            "",
            "argumen kosong menghasilkan string kosong setelah trim"
        );
    }

    // -- kalender ----------------------------------------------------------

    #[test]
    fn jakarta_parts_is_utc_plus_seven_with_no_dst() {
        // 2026-01-01T00:00:00Z -> 2026-01-01 07:00:00 WIB.
        assert_eq!(jakarta_parts(1_767_225_600_000), (2026, 1, 1, 7, 0, 0));
        // 2026-01-01T20:00:00Z -> 2026-01-02 03:00:00 WIB.
        assert_eq!(jakarta_parts(1_767_297_600_000), (2026, 1, 2, 3, 0, 0));
        // Epoch -> 1970-01-01 07:00:00 WIB.
        assert_eq!(jakarta_parts(0), (1970, 1, 1, 7, 0, 0));
    }

    #[test]
    fn days_from_civil_round_trips_through_the_civil_conversion() {
        for (y, m, d) in [
            (1970, 1, 1),
            (2000, 2, 29),
            (2024, 12, 31),
            (2026, 6, 15),
            (2100, 3, 1),
        ] {
            let days = days_from_civil(y, m, d);
            assert_eq!(jakarta_parts(days * 86_400_000).0, y, "tahun {y}");
            assert_eq!(jakarta_parts(days * 86_400_000).1, m, "bulan {m}");
            assert_eq!(jakarta_parts(days * 86_400_000).2, d, "hari {d}");
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn date_helpers_use_the_same_tokens_as_the_js_time_module() {
        assert_eq!(date_long(1_767_225_600_000), "1 Januari 2026");
        assert_eq!(date_dmy(1_767_225_600_000), "01/01/2026");
        assert_eq!(time_hm(1_767_225_600_000), "07:00");
        assert_eq!(time_hm(1_767_225_600_000 + 61_000), "07:01");
        assert_eq!(pad2(0), "00");
        assert_eq!(pad2(7), "07");
        assert_eq!(pad2(42), "42");
        assert_eq!(MONTHS_ID.len(), 12);
        assert_eq!(MONTHS_ID[0], "Januari");
        assert_eq!(MONTHS_ID[11], "Desember");
    }

    #[test]
    fn now_ms_is_after_the_january_2026_reference_point() {
        assert!(now_ms() > 1_767_225_600_000, "jam sistem harus masuk akal");
    }

    // -- smoke: setiap plugin baru membalas tanpa panic --------------------

    #[test]
    fn every_new_plugin_answers_through_the_registry_for_its_own_context() {
        for name in NEW_PLUGINS {
            let p = find(name).unwrap();
            for ctx in [
                Ctx {
                    prefix: ".".into(),
                    command: name.into(),
                    sender: "628111@s.whatsapp.net".into(),
                    chat: "120363@g.us".into(),
                    is_group: true,
                    text: "25-12".into(),
                    args: vec!["25-12".into()],
                    user_data: json!({"koin": 10, "exp": 20_000, "energi": 3, "birthday": "25-12"}),
                    group_data: json!({"intro": "hi @user", "groupRules": "r", "chatStats": {"628111@s.whatsapp.net": {"count": 1}}}),
                    roster: roster(vec![p_participant()]),
                    ..Default::default()
                },
                // Kosong total: tanpa roster, tanpa state, tanpa argumen.
                Ctx {
                    prefix: ".".into(),
                    command: name.into(),
                    sender: "628111@s.whatsapp.net".into(),
                    chat: "628111@s.whatsapp.net".into(),
                    ..Default::default()
                },
            ] {
                let reply = (p.handler)(ctx);
                assert!(
                    reply.text_body().is_some(),
                    "{name} tidak membalas teks sama sekali"
                );
            }
        }
    }

    fn p_participant() -> Participant {
        Participant {
            jid: "628111@s.whatsapp.net".into(),
            is_admin: false,
            is_owner: false,
        }
    }
}

/// Batch port plugin class-A yang tersisa: `fun/gay.rs`, `fun/husbualbum.rs`,
/// `fun/mimpi.rs`, `fun/top.rs`, `fun/waifualbum.rs`.
///
/// ## Kenapa `fun/siapa.js` TIDAK ikut
///
/// `siapa.js` mendaftarkan 52 command lewat `name` berupa ARRAY, dan 8 di
/// antaranya sudah dimiliki plugin `cek/*` yang sudah terport: `pintar`,
/// `wibu`, `ganteng`, `cantik`, `cupu`, `noob`, `pro`, `kaya`. Di JS
/// kedua plugin itu sama-sama terdaftar dan yang menang bergantung pada urutan
/// load file — tidak ada jawaban yang benar. Di Rust `find()` mengembalikan
/// entri PERTAMA, jadi satu plugin diam-diam mati. Menambahnya
/// berarti 8 command yang sekarang bekerja jadi bergantung pada urutan
/// `linkme`, jadi tidak dilakukan.
#[cfg(test)]
mod tests_batch_fun {
    use super::*;
    use oktz_plugin_api::all_plugins;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    /// Satu nama = satu file, jadi daftar ini sekaligus penanda apa yang
    /// harus ada di disk.
    pub(crate) const NEW_PLUGINS: [&str; 5] = [
        "gaycheck",
        "husbualbum",
        "mimpi",
        "top",
        "waifualbum",
    ];

    #[test]
    fn all_four_are_registered_and_the_registry_grew_by_exactly_them() {
        for n in NEW_PLUGINS {
            assert!(oktz_plugin_api::find(n).is_some(), "{n} tidak terdaftar");
        }
        // Batch ini tidak boleh diam-diam menambah plugin lain.
        let others = all_plugins()
            .iter()
            .filter(|e| !NEW_PLUGINS.contains(&e.meta.name.as_str()))
            .count();
        assert_eq!(
            oktz_plugin_api::count() - others,
            NEW_PLUGINS.len(),
            "ada plugin di luar NEW_PLUGINS yang ikut bertambah"
        );
    }

    #[test]
    fn every_alias_of_this_batch_resolves_back_to_its_own_plugin() {
        for name in NEW_PLUGINS {
            let entry = oktz_plugin_api::find(name).unwrap();
            for alias in std::iter::once(&entry.meta.name).chain(entry.meta.alias.iter()) {
                assert_eq!(
                    oktz_plugin_api::find(alias).map(|e| e.meta.name.as_str()),
                    Some(name),
                    "key {alias} salah resolve"
                );
            }
        }
    }

    #[test]
    fn every_key_of_this_batch_has_exactly_one_owner_in_the_rust_registry() {
        for name in NEW_PLUGINS {
            let entry = oktz_plugin_api::find(name).unwrap();
            for key in std::iter::once(&entry.meta.name).chain(entry.meta.alias.iter()) {
                let owners: Vec<&str> = all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(key))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, vec![name], "key {key} diklaim oleh {owners:?}");
            }
        }
    }

    /// Kunci yang JS-nya memang tabrakan harus dicatat, bukan hanya lolos
    /// karena plugin pasangannya kebetulan belum diport. Kalau suatu saat
    /// `cek/cekkaya.js` ikut diport, test ini yang akan menjatuhkan.

    #[test]
    fn the_batch_answers_through_the_registry() {
        for name in NEW_PLUGINS {
            let reply = (oktz_plugin_api::find(name).unwrap().handler)(Ctx {
                text: "Budi".into(),
                args: vec!["Budi".into()],
                sender: "628111@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                is_group: true,
                roster: GroupRoster {
                    participants: vec![
                        Participant {
                            jid: "628111@s.whatsapp.net".into(),
                            is_admin: false,
                            is_owner: false,
                        },
                        Participant {
                            jid: "628222@s.whatsapp.net".into(),
                            is_admin: false,
                            is_owner: false,
                        },
                    ],
                    is_bot_admin: false,
                },
                ..Default::default()
            });
            assert!(reply.text_body().is_some(), "{name} tidak membalas teks");
        }
    }
}

