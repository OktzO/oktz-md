//! Pesan teks welcome & goodbye + `resolvePlaceholders` untuk grup.
//!
//! Sumber JS yang diport apa adanya:
//! - `plugins/group/welcome.js:9-32` resolvePlaceholders
//! - `plugins/group/welcome.js:58-93` greetings/quotes/emojis/headers + undian acak
//! - `plugins/group/welcome.js:107-122` susunan teks default
//! - `plugins/group/welcome.js:96-106` cabang custom (groupData.welcomeMsg)
//! - `plugins/group/welcome.js:365-370` cabang TEKS default (sendMessage text + mentions)
//! - `plugins/group/goodbye.js:16-39` resolvePlaceholders
//! - `plugins/group/goodbye.js:65-100` farewells/quotes/emojis/headers + undian acak
//! - `plugins/group/goodbye.js:114-123` susunan teks default
//! - `plugins/group/goodbye.js:103-113` cabang custom (groupData.goodbyeMsg)
//! - `plugins/group/goodbye.js:415-420` fallback TEKS saat kartu canvas gagal
//! - `src/lib/formatter.js:508-515` getTimeGreeting -> [`salam`]
//! - `src/lib/time.js:47-73` pola `DD/MM/YYYY`, `HH:mm`, `dddd` (locale id-ID:
//!   nama hari & bulan bahasa Indonesia). Pemanggil yang memformat tanggal/jam/hari,
//!   sama seperti `formatPattern(now, ...)` di JS.
//!
//! `salam` mengikuti `getTimeGreeting` (formatter.js), BUKAN `greetings` di
//! welcome.js:58-66: daftar itu dipilih acak (`welcome.js:90`), bukan per jam.
//!
//! Batas — sengaja tidak diport, butuh data/objek di luar [`KonteksPesan`] atau
//! jalur pengiriman socket:
//! - `welcomeType`/`goodbyeType` 2..8: kartu carousel + upload PP
//!   (welcome.js:152-231, goodbye.js:173-252), teks + forwardedNewsletterMessageInfo
//!   (welcome.js:232-254, goodbye.js:253-275), preview link (welcome.js:263-279,
//!   goodbye.js:284-300), video gif (welcome.js:280-288, goodbye.js:301-309),
//!   tombol interactive + qFake (welcome.js:289-359, goodbye.js:310-380), teks
//!   pendek type 8 (welcome.js:360-364, goodbye.js:381-385).
//! - Type 4 (welcome.js:255-262, goodbye.js:276-283) mengirim teks yang SAMA
//!   dengan [`welcome`]/[`goodbye`] lewat sendText + contextInfo; bagian
//!   pengiriman/context di luar modul ini.
//! - Kartu canvas goodbye (goodbye.js:387-414, createGoodbyeCard) dan foto profil
//!   (welcome.js:136-140, goodbye.js:157-161) — butuh socket/upload/kanvas.
//! - Identitas JID/LID: resolveAnyLidToJid (welcome.js:129-132),
//!   cacheParticipantLids/lidToJid (goodbye.js:131-153); pemanggil menyerahkan
//!   nomor final lewat [`KonteksPesan::nomor`].
//! - Deskripsi: welcome.js:144 & goodbye.js:165 memakai `groupMeta.descOwner`
//!   untuk teks default, kartu type 2/3 memakai `groupMeta.desc`
//!   (welcome.js:158,238). `KonteksPesan.deskripsi` = pilihan pemanggil;
//!   `None`/`""` melewati blok deskripsi (welcome.js:115).
//! - Quirk `String.replace` JS: replacement yang mengandung pola dolar khusus
//!   diperlakukan istimewa oleh JS; di sini selalu literal. Nomor WA tidak
//!   mengandung dolar.

/// Konteks teks untuk satu peserta yang baru masuk/keluar.
#[derive(Debug, Clone, Copy)]
pub struct KonteksPesan<'a> {
    /// Nomor peserta tanpa akhiran JID (`participant.split("@")[0]`, welcome.js:94).
    pub nomor: &'a str,
    pub nama_grup: &'a str,
    /// `groupMeta.desc`/`descOwner`; `None` kalau tidak ada.
    pub deskripsi: Option<&'a str>,
    pub jumlah_member: usize,
    /// Nomor owner grup; `""` kalau tidak ada -> `{owner}` jadi "Admin".
    pub owner_grup: &'a str,
    pub prefix: &'a str,
    pub bot_nama: &'a str,
    /// Sudah diformat pemanggil: `DD/MM/YYYY` (time.js:59-60).
    pub tanggal: &'a str,
    /// Sudah diformat pemanggil: `HH:mm` (time.js:53).
    pub jam: &'a str,
    /// Sudah diformat pemanggil: `dddd` nama hari Indonesia (time.js:59).
    pub hari: &'a str,
}

/// `greetings` welcome.js:58-66.
const GREETING: [&str; 7] = [
    "Akhirnya datang juga",
    "Selamat datang",
    "Welcome",
    "Halo",
    "Hai",
    "Yokoso~",
    "Ohayou~",
];

/// `quotes` welcome.js:67-74.
const QUOTE_WELCOME: [&str; 6] = [
    "Jangan jadi silent reader ya!",
    "Santai aja, anggap rumah sendiri!",
    "Yuk langsung gas ngobrol!",
    "Siap-siap rame bareng!",
    "Jangan malu-malu, kita semua temen!",
    "Kalau bingung mulai, nyapa aja dulu 😄",
];

/// `emojis` welcome.js:75.
const EMOJI_WELCOME: [&str; 7] = ["🎐", "🌸", "✨", "💫", "🪸", "🔥", "💖"];

/// `headers` welcome.js:76-89.
const HEADER_WELCOME: [&str; 4] = [
    "🎐 Ohayou~ minna-san!\nHari ini kita kedatangan tomodachi baru 🌱\nYuk sambut bareng-bareng~",
    "🌸 Ohayou minna-san!\nSatu teman baru akhirnya join ✨\nSemoga betah dan langsung nimbrung ya~",
    "✨ Ohayou~!\nTomodachi baru datang bawa vibes baru 💫\nYoroshiku ne~ mari seru-seruan bareng!",
    "🪸 Ohayou minna-san!\nGrup ini nambah satu keluarga lagi 🤍\nTanoshii jikan o issho ni sugoso ne~",
];

/// `farewells` goodbye.js:65-73.
const FAREWELL: [&str; 7] = [
    "Sayonara",
    "Sampai jumpa",
    "Bye bye",
    "Dadah",
    "See you",
    "Hati-hati",
    "Oyasumi~",
];

/// `quotes` goodbye.js:74-81.
const QUOTE_GOODBYE: [&str; 6] = [
    "Semoga langkahmu selalu dimudahkan ke depannya.",
    "Terima kasih sudah jadi bagian dari grup ini.",
    "Semoga kita bisa bertemu lagi di lain waktu.",
    "Pintu selalu terbuka kalau suatu saat mau kembali.",
    "Jaga diri baik-baik ya, tomodachi.",
    "Kenangan di sini bakal tetap ada.",
];

/// `emojis` goodbye.js:82.
const EMOJI_GOODBYE: [&str; 6] = ["🌙", "👋", "🥀", "💫", "😢", "🤍"];

/// `headers` goodbye.js:83-96.
const HEADER_GOODBYE: [&str; 4] = [
    "🌙 Oyasumi~ minna-san...\nHari ini satu tomodachi harus berpamitan.\nSemoga perjalanan barunya penuh kebaikan.",
    "🥀 Minna-san...\nAda perpisahan kecil hari ini.\nTerima kasih sudah pernah berjalan bersama.",
    "💫 Sayonara~\nBukan akhir, hanya sampai jumpa.\nSemoga hari-harimu selalu hangat.",
    "🌌 Minna-san...\nSatu bintang berpindah langit malam ini.\nDoakan yang terbaik untuknya ya.",
];

/// Sapaan per jam lokal, meniru `getTimeGreeting` (src/lib/formatter.js:508-515).
/// Jam di luar 0..23 (data rusak) jatuh ke jalur terakhir, sama seperti JS.
pub fn salam(jam: u32) -> String {
    if (4..10).contains(&jam) {
        "Selamat Pagi 🌅".to_string()
    } else if (10..15).contains(&jam) {
        "Selamat Siang ☀️".to_string()
    } else if (15..18).contains(&jam) {
        "Selamat Sore 🌇".to_string()
    } else {
        "Selamat Malam 🌙".to_string()
    }
}

/// Pesan welcome lengkap (bentuk teks/type 1), `custom` = groupData.welcomeMsg.
pub fn welcome(custom: Option<&str>, k: &KonteksPesan<'_>, acak: u64) -> String {
    // `if (customMsg)` (welcome.js:96): string kosong juga falsy -> template default.
    if let Some(c) = custom
        && !c.is_empty()
    {
        return ganti_placeholder(c, k);
    }

    // Urutan undian sama dengan JS (welcome.js:90-93): greeting, quote, emoji, header.
    let greeting = pilih(&GREETING, acak, 0);
    let quote = pilih(&QUOTE_WELCOME, acak, 1);
    let emoji = pilih(&EMOJI_WELCOME, acak, 2);
    let header = pilih(&HEADER_WELCOME, acak, 3);
    susun_welcome(greeting, quote, emoji, header, k)
}

/// Susun teks default (welcome.js:107-122) dari pilihan yang sudah ditentukan.
/// Dipisah dari [`welcome`] supaya test bisa mengunci urutan baris dan pemisah
/// persisnya tanpa ikut mengundi.
fn susun_welcome(
    greeting: &str,
    quote: &str,
    emoji: &str,
    header: &str,
    k: &KonteksPesan<'_>,
) -> String {
    let nama = nama_tampilan(k.nomor);
    let mut msg = format!(
        "👋🏻 *WELCOME MEMBER BARU* 👋🏻\n\n{header}\n{emoji} {greeting}, *@{nama}* 💫\n\n📌 *INFO GROUP*\n> 🏠 *Nama* : {}\n> 👥 *Member* : {}\n> 📅 *Tanggal* : {}\n",
        k.nama_grup, k.jumlah_member, k.tanggal
    );

    if let Some(d) = k.deskripsi.filter(|d| !d.is_empty()) {
        msg.push_str("\n📝 *Deskripsi*\n> ❝ ");
        msg.push_str(&potong_120(d));
        msg.push_str(" ❞\n");
    }

    msg.push_str("\n✨ *Tips Hari Ini*\n> 「 ");
    msg.push_str(quote);
    msg.push_str(" 」\n\n🌸 _Yoroshiku ne~ semoga betah ya!_ 🤍");
    msg
}

/// Pesan goodbye lengkap (bentuk teks), `custom` = groupData.goodbyeMsg.
pub fn goodbye(custom: Option<&str>, k: &KonteksPesan<'_>, acak: u64) -> String {
    // `if (customMsg)` (goodbye.js:103): string kosong juga falsy.
    if let Some(c) = custom
        && !c.is_empty()
    {
        return ganti_placeholder(c, k);
    }

    // Urutan undian sama dengan JS (goodbye.js:97-100): farewell, quote, emoji, header.
    let farewell = pilih(&FAREWELL, acak, 0);
    let quote = pilih(&QUOTE_GOODBYE, acak, 1);
    let emoji = pilih(&EMOJI_GOODBYE, acak, 2);
    let header = pilih(&HEADER_GOODBYE, acak, 3);
    susun_goodbye(farewell, quote, emoji, header, k)
}

/// Susun teks default (goodbye.js:114-123) dari pilihan yang sudah ditentukan.
fn susun_goodbye(
    farewell: &str,
    quote: &str,
    emoji: &str,
    header: &str,
    k: &KonteksPesan<'_>,
) -> String {
    let nama = nama_tampilan(k.nomor);
    format!(
        "👋🏻 *SAYONARA MEMBER* 👋🏻\n\n{header}\n{emoji} {farewell}, *@{nama}* 🤍\n\n📌 *INFO GROUP*\n> 🏠 *Nama* : {}\n> 👥 *Sisa Member* : {}\n> 📅 *Tanggal* : {}\n\n💌 *Pesan*\n> 「 {quote} 」\n\n🌸 _Sampai jumpa lagi, tomodachi._ 🤍",
        k.nama_grup, k.jumlah_member, k.tanggal
    )
}

/// `resolvePlaceholders` (welcome.js:9-32, goodbye.js:16-39) untuk template
/// custom: `gi` = case-insensitive dan ganti semua. Loop sendiri, tanpa crate regex.
///
/// JS merantai 11 `.replace` BERURUTAN (user, number, group, desc, count, owner,
/// date, time, day, bot, prefix), jadi hasil satu penggantian masih bisa cocok
/// dengan pola sesudahnya — mis. nama grup `{desc}` ikut diganti. Urutan pass di
/// sini persis urutan rantai `.replace` itu.
pub fn ganti_placeholder(template: &str, k: &KonteksPesan<'_>) -> String {
    let mut out = template.to_string();
    for nama in [
        "user", "number", "group", "desc", "count", "owner", "date", "time", "day", "bot", "prefix",
    ] {
        out = ganti_satu(&out, nama, k);
    }
    out
}

/// Satu pass `.replace(/{nama}/gi, nilai)`.
fn ganti_satu(teks: &str, nama: &str, k: &KonteksPesan<'_>) -> String {
    let b = teks.as_bytes();
    let mut out = String::with_capacity(teks.len());
    let mut i = 0usize;
    while i < b.len() {
        // Placeholder selalu `{` + nama ASCII + `}`.
        if b[i] == b'{'
            && let Some(tutup) = teks[i + 1..].find('}')
        {
            let kandidat = &teks[i + 1..i + 1 + tutup];
            if kandidat.eq_ignore_ascii_case(nama)
                && let Some(v) = nilai_placeholder(nama, k)
            {
                out.push_str(&v);
                i += tutup + 2; // lewati '{' + nama + '}'.
                continue;
            }
        }
        let c = teks[i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Satu placeholder -> nilai pengganti. `nama` sudah pasti lowercase & salah satu
/// dari daftar di [`ganti_placeholder`]; `None` kalau tidak dikenal.
fn nilai_placeholder(nama: &str, k: &KonteksPesan<'_>) -> Option<String> {
    let v = match nama {
        "user" => format!("@{}", nama_tampilan(k.nomor)),
        "number" => nama_tampilan(k.nomor).to_string(),
        // `groupName || "Grup"` (welcome.js:23).
        "group" => if k.nama_grup.is_empty() { "Grup".to_string() } else { k.nama_grup.to_string() },
        // `groupDesc || ""` (welcome.js:24) — tanpa potong 120.
        "desc" => k.deskripsi.unwrap_or("").to_string(),
        // `memberCount?.toString() || "0"` (welcome.js:25).
        "count" => k.jumlah_member.to_string(),
        // `groupOwner || "Admin"` (welcome.js:26).
        "owner" => if k.owner_grup.is_empty() { "Admin".to_string() } else { k.owner_grup.to_string() },
        "date" => k.tanggal.to_string(),
        "time" => k.jam.to_string(),
        "day" => k.hari.to_string(),
        // `config.bot?.name || "Bot"` (welcome.js:30).
        "bot" => if k.bot_nama.is_empty() { "Bot".to_string() } else { k.bot_nama.to_string() },
        // Tanpa fallback di JS (welcome.js:31); pemanggil sudah memberi "." kalau
        // kosong (welcome.js:148 `config.command?.prefix || "."`).
        "prefix" => k.prefix.to_string(),
        _ => return None,
    };
    Some(v)
}

/// Nomor tampilan: `participant?.split("@")[0] || "User"` (welcome.js:94,
/// goodbye.js:101). Kontrak `KonteksPesan.nomor` sudah tanpa JID; split di sini
/// tetap dilakukan supaya perilakunya sama kalau pemanggil lupa.
fn nama_tampilan(nomor: &str) -> &str {
    let n = nomor.split('@').next().unwrap_or("");
    if n.is_empty() { "User" } else { n }
}

/// `daftar[Math.floor(Math.random() * daftar.length)]` (welcome.js:90-93,
/// goodbye.js:97-100). Modul ini tanpa crate RNG: pemanggil menyuplai `acak`,
/// indeks = hash(acak, langkah) % panjang. SplitMix64 memisahkan undian
/// berurutan supaya empat pilihan tidak saling berkorelasi.
fn pilih(daftar: &[&'static str], acak: u64, langkah: u64) -> &'static str {
    let mut x = acak.wrapping_add(langkah.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    daftar[(x % daftar.len() as u64) as usize]
}

/// `groupDesc.slice(0, 120)` + `...` kalau `groupDesc.length > 120`
/// (welcome.js:116). JS menghitung 120 satuan UTF-16 dan BISA membelah emoji;
/// separuh surrogate itu jadi U+FFFD saat dikirim UTF-8, jadi batasnya dijaga di
/// batas karakter dan U+FFFD ditulis eksplisit supaya hasil yang user lihat sama.
/// `ponytail:` beda yang tersisa cuma representasi di memori JS (surrogate
/// tunggal), dan itu tidak mungkin masuk dari sisi Rust karena input selalu UTF-8
/// valid. Naikkan ke crate unicode-segmentation hanya kalau WA ternyata menampilkan
/// hal lain saat deskripsi kepotong di tengah emoji.
fn potong_120(desc: &str) -> String {
    let mut out = String::with_capacity(desc.len().min(124));
    let mut unit = 0usize;
    for c in desc.chars() {
        let w = c.len_utf16();
        if unit + w > 120 {
            // Hanya mungkin untuk karakter astral di unit ke-119 (pair terbelah).
            if unit < 120 {
                out.push('\u{FFFD}');
            }
            break;
        }
        unit += w;
        out.push(c);
    }
    if desc.encode_utf16().count() > 120 {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod uji {
    use super::*;

    const K: KonteksPesan<'_> = KonteksPesan {
        nomor: "6281234567890",
        nama_grup: "Grup Uji",
        deskripsi: Some("Deskripsi grup"),
        jumlah_member: 42,
        owner_grup: "6289999999999",
        prefix: ".",
        bot_nama: "BotUji",
        tanggal: "19/10/2026",
        jam: "07:05",
        hari: "Senin",
    };

    #[test]
    fn salam_batas_jam() {
        assert_eq!(salam(3), "Selamat Malam 🌙");
        assert_eq!(salam(4), "Selamat Pagi 🌅");
        assert_eq!(salam(9), "Selamat Pagi 🌅");
        assert_eq!(salam(10), "Selamat Siang ☀️");
        assert_eq!(salam(14), "Selamat Siang ☀️");
        assert_eq!(salam(15), "Selamat Sore 🌇");
        assert_eq!(salam(17), "Selamat Sore 🌇");
        assert_eq!(salam(18), "Selamat Malam 🌙");
        assert_eq!(salam(23), "Selamat Malam 🌙");
    }

    #[test]
    fn placeholder_gi_dan_fallback() {
        let t = "{USER} {User} {number} {group} {desc} {count} {owner} {date} {time} {day} {bot} {prefix} {lain}";
        assert_eq!(
            ganti_placeholder(t, &K),
            "@6281234567890 @6281234567890 6281234567890 Grup Uji Deskripsi grup 42 6289999999999 19/10/2026 07:05 Senin BotUji . {lain}"
        );
        let kosong = KonteksPesan {
            nomor: "",
            nama_grup: "",
            deskripsi: None,
            owner_grup: "",
            bot_nama: "",
            ..K
        };
        assert_eq!(
            ganti_placeholder("{user}|{group}|{desc}|{owner}|{bot}", &kosong),
            "@User|Grup||Admin|Bot"
        );
        // Rantai replace JS berurutan: `{group}` diisi "{desc}" lalu pass desc
        // menggantinya; sebaliknya "{user}" yang muncul dari `{group}` TIDAK
        // diganti karena pass user sudah lewat.
        let quirk = KonteksPesan {
            nama_grup: "{desc}",
            deskripsi: Some("D"),
            ..K
        };
        assert_eq!(ganti_placeholder("{group}", &quirk), "D");
        let quirk2 = KonteksPesan {
            nama_grup: "{user}",
            ..K
        };
        assert_eq!(ganti_placeholder("{group}", &quirk2), "{user}");
    }

    #[test]
    fn susunan_default_lengkap() {
        // Pilihan contoh: HEADER[1], EMOJI[0], GREETING[3], QUOTE[2].
        let pilih_h = HEADER_WELCOME[1];
        assert_eq!(
            susun_welcome(GREETING[3], QUOTE_WELCOME[2], EMOJI_WELCOME[0], pilih_h, &K),
            "👋🏻 *WELCOME MEMBER BARU* 👋🏻\n\n🌸 Ohayou minna-san!\nSatu teman baru akhirnya join ✨\nSemoga betah dan langsung nimbrung ya~\n🎐 Halo, *@6281234567890* 💫\n\n📌 *INFO GROUP*\n> 🏠 *Nama* : Grup Uji\n> 👥 *Member* : 42\n> 📅 *Tanggal* : 19/10/2026\n\n📝 *Deskripsi*\n> ❝ Deskripsi grup ❞\n\n✨ *Tips Hari Ini*\n> 「 Yuk langsung gas ngobrol! 」\n\n🌸 _Yoroshiku ne~ semoga betah ya!_ 🤍"
        );
        let tanpa_desc = KonteksPesan {
            deskripsi: None,
            ..K
        };
        assert_eq!(
            susun_welcome(GREETING[3], QUOTE_WELCOME[2], EMOJI_WELCOME[0], pilih_h, &tanpa_desc),
            "👋🏻 *WELCOME MEMBER BARU* 👋🏻\n\n🌸 Ohayou minna-san!\nSatu teman baru akhirnya join ✨\nSemoga betah dan langsung nimbrung ya~\n🎐 Halo, *@6281234567890* 💫\n\n📌 *INFO GROUP*\n> 🏠 *Nama* : Grup Uji\n> 👥 *Member* : 42\n> 📅 *Tanggal* : 19/10/2026\n\n✨ *Tips Hari Ini*\n> 「 Yuk langsung gas ngobrol! 」\n\n🌸 _Yoroshiku ne~ semoga betah ya!_ 🤍"
        );
        assert_eq!(
            susun_goodbye(FAREWELL[3], QUOTE_GOODBYE[4], EMOJI_GOODBYE[4], HEADER_GOODBYE[2], &K),
            "👋🏻 *SAYONARA MEMBER* 👋🏻\n\n💫 Sayonara~\nBukan akhir, hanya sampai jumpa.\nSemoga hari-harimu selalu hangat.\n😢 Dadah, *@6281234567890* 🤍\n\n📌 *INFO GROUP*\n> 🏠 *Nama* : Grup Uji\n> 👥 *Sisa Member* : 42\n> 📅 *Tanggal* : 19/10/2026\n\n💌 *Pesan*\n> 「 Jaga diri baik-baik ya, tomodachi. 」\n\n🌸 _Sampai jumpa lagi, tomodachi._ 🤍"
        );
    }

    #[test]
    fn custom_dipakai_apa_adanya() {
        assert_eq!(
            welcome(Some("Hai {user} di {group}"), &K, 0),
            "Hai @6281234567890 di Grup Uji"
        );
        assert_eq!(
            goodbye(Some("{BOT} pamit dari {GROUP}"), &K, 0),
            "BotUji pamit dari Grup Uji"
        );
        // String kosong = falsy di JS -> template default.
        assert!(welcome(Some(""), &K, 7).starts_with("👋🏻 *WELCOME MEMBER BARU* 👋🏻\n\n"));
        assert!(goodbye(Some(""), &K, 7).starts_with("👋🏻 *SAYONARA MEMBER* 👋🏻\n\n"));
    }

    #[test]
    fn potong_deskripsi_120() {
        let d = "a".repeat(130);
        let p = potong_120(&d);
        assert_eq!(p, format!("{}...", "a".repeat(120)));
        // 119 ASCII + emoji (2 satuan UTF-16): JS `slice(0,120)` menyisakan
        // surrogate atas (jadi U+FFFD saat dikirim), dan panjang asli 121 > 120
        // sehingga `...` tetap ditambahkan.
        let d2 = format!("{}👋", "b".repeat(119));
        assert_eq!(potong_120(&d2), format!("{}\u{FFFD}...", "b".repeat(119)));
        assert_eq!(potong_120(&"b".repeat(120)).len(), 120);
    }
}
