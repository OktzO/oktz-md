//! `plugins/main/allmenu.js` — semua command per kategori.
//!
//! Port varian 2 (`allmenu.js:238-292`): kartu native-flow dengan header
//! gambar `foto`, daftar command per kategori, blok info user/bot, satu
//! tombol `quick_reply` "Kembali Ke Menu Utama", dan `limited_time_offer` di
//! `messageParamsJson`.
//!
//! Batas port (`ponytail:`):
//! - `case/foto.js` (command berbasis case) BELUM di-port di Rust, jadi
//!   `cping`, `listallcase`, dan `listallplugin` tidak muncul di daftar dan
//!   total fitur hanya menghitung command plugin (JS menambah 3 command case).
//! - `sharp` dan `ffmpeg` (potong audio) TIDAK dipakai: varian 2 tidak butuh
//!   resize thumbnail seperti varian 5/6, dan pemotongan audio untuk
//!   `allmenuAudioStyle` tidak di-port sama sekali.
//! - Varian 1, 5, dan 6 (teks polos, kartu video, kartu lokasi) tidak
//!   di-port — hanya varian 2, sesuai tujuan plugin ini.
//! - `forwardedNewsletterMessageInfo` tidak ada di `Kartu`, jadi info
//!   newsletter JS tidak bisa dikirim.
//! - Isi `txt` JS dibagi dua: blok info masuk `footer` (footer WA pendek dan
//!   dipotong WA), daftar command masuk `body` (tempat yang dibaca user).
//!   Semua baris JS tetap ada, hanya tempatnya yang dipisah.
//! - `createBracketBox` JS menerima emoji kategori tapi tidak pernah
//!   memakainya, jadi output JS kategori tanpa emoji. Di sini emoji dipakai
//!   di judul kotak — itu maksud `CATEGORY_EMOJIS` di JS.
//! - Varian 2 JS hanya punya satu tombol `quick_reply`. Tidak ada
//!   `single_select` daftar kategori di varian ini, jadi tidak dibuat —
//!   `single_select` ada di varian 5 (`bottom_sheet`).

use std::collections::BTreeMap;

use oktz_plugin_api::{
    Ctx, KartuHeader, MediaSource, PluginMeta, Reply, Tombol, all_plugins, plugin,
};

use crate::common::{jakarta_parts, now_ms, pad2};

/// Gambar header kartu — `assets/image/foto.jpeg` (`allmenu.js:240`).
///
/// Binary-nya di-embed: aset tidak ikut ke paket binary, jadi tanpa ini
/// header bisa kosong saat runtime.
const ASET_FOTO: &[u8] = include_bytes!("../../../assets/image/foto.jpeg");

/// Urutan kategori (`allmenu.js:158-179`, dilengkapi kategori yang belum ada
/// di sana). Kategori di luar daftar ini diurutkan setelahnya, dengan nama
/// kategori sebagai pemilah (`allmenu.js:180-184`).
const URUTAN: &[&str] = &[
    "owner",
    "main",
    "utility",
    "tools",
    "fun",
    "game",
    "download",
    "search",
    "sticker",
    "media",
    "ai",
    "group",
    "religi",
    "info",
    "cek",
    "economy",
    "user",
    "canvas",
    "random",
    "premium",
    "jpm",
    "pushkontak",
    "panel",
    "ephoto",
    "store",
];

/// Emoji kategori (`allmenu.js:39-68`).
const EMOJI: &[(&str, &str)] = &[
    ("owner", "👑"),
    ("main", "🏠"),
    ("utility", "🔧"),
    ("fun", "🎮"),
    ("group", "👥"),
    ("download", "📥"),
    ("search", "🔍"),
    ("tools", "🛠️"),
    ("sticker", "🖼️"),
    ("ai", "🤖"),
    ("game", "🎯"),
    ("media", "🎬"),
    ("info", "ℹ️"),
    ("religi", "☪️"),
    ("panel", "🖥️"),
    ("user", "📊"),
    ("linode", "☁️"),
    ("random", "🎲"),
    ("canvas", "🎨"),
    ("vps", "🌊"),
    ("store", "🏪"),
    ("premium", "💎"),
    ("convert", "🔄"),
    ("economy", "💰"),
    ("cek", "📋"),
    ("ephoto", "🎨"),
    ("jpm", "📢"),
    ("pushkontak", "📱"),
];

/// Prefix command — `ctx.prefix`, atau "." kalau mode tanpa prefix.
fn prefix(ctx: &Ctx) -> &str {
    if ctx.prefix.is_empty() {
        "."
    } else {
        ctx.prefix.as_str()
    }
}

/// `config.bot.name` — nilai kosong replaced nama bawaan.
fn nama_bot(ctx: &Ctx) -> &str {
    let n = &ctx.config().bot.name;
    if n.is_empty() {
        "Bot"
    } else {
        n.as_str()
    }
}

/// `config.bot.developer`.
fn pengembang(ctx: &Ctx) -> &str {
    let d = &ctx.config().bot.developer;
    if d.is_empty() {
        "Owner"
    } else {
        d.as_str()
    }
}

/// `config.bot.version`.
fn versi_bot(ctx: &Ctx) -> &str {
    let v = &ctx.config().bot.version;
    if v.is_empty() {
        "1.2.0"
    } else {
        v.as_str()
    }
}

/// `getTimeGreeting()` (`formatter.js:508-515`), jam Asia/Jakarta.
fn sapa() -> &'static str {
    match jakarta_parts(now_ms()).3 {
        4..=9 => "Selamat Pagi 🌅",
        10..=14 => "Selamat Siang ☀️",
        15..=17 => "Selamat Sore 🌇",
        _ => "Selamat Malam 🌙",
    }
}

/// Emoji kategori, `📋` kalau tidak dikenal (`allmenu.js:223`).
fn emoji(kategori: &str) -> &'static str {
    EMOJI
        .iter()
        .find(|(nama, _)| *nama == kategori)
        .map_or("📋", |(_, e)| *e)
}

/// Mode bot grup — `groupData.botMode` (`allmenu.js:108-109`).
///
/// Di private chat tidak ada data grup, jadi mode selalu `md` — sama seperti
/// JS yang hanya membaca `db.getGroup()` kalau pesannya di grup.
fn mode_bot(ctx: &Ctx) -> String {
    if !ctx.is_group {
        return String::from("md");
    }
    ctx.group_field("botMode")
        .and_then(|v| v.as_str())
        .map(str::to_lowercase)
        .unwrap_or_else(|| String::from("md"))
}

/// Kategori yang boleh ditampilkan untuk mode grup tertentu.
///
/// `md` memakai daftar exclude dari `allmenu.js:191-196` — `panel`,
/// `pushkontak`, `store` — bukan `cpanel` seperti `botmode.js:24`, karena
/// `cpanel` bukan nama kategori plugin. Mode lain memakai daftar
/// `allowedCategories` (`botmode.js:32-54`).
fn kategori_boleh(mode: &str, kategori: &str) -> bool {
    let kategori = kategori.to_lowercase();
    match mode {
        "cpanel" => matches!(
            kategori.as_str(),
            "main" | "group" | "sticker" | "owner" | "tools" | "panel"
        ),
        "pushkontak" => matches!(
            kategori.as_str(),
            "owner" | "main" | "group" | "sticker" | "pushkontak"
        ),
        "store" => matches!(
            kategori.as_str(),
            "main" | "group" | "sticker" | "owner" | "store"
        ),
        "otp" => matches!(
            kategori.as_str(),
            "main" | "group" | "sticker" | "owner" | "otp"
        ),
        "md" => !matches!(kategori.as_str(), "panel" | "pushkontak" | "store"),
        _ => true,
    }
}

/// Simbol permission di samping nama command (`allmenu.js:78-89`).
///
/// `🅛` (limit) tidak ada: `PluginMeta` tidak punya field `limit` — limit
/// user sudah jadi field `energi` di store (`pipeline.rs:617`).
fn simbol(meta: &PluginMeta) -> String {
    let mut s: Vec<&str> = Vec::new();
    if meta.is_owner {
        s.push("🅞");
    }
    if meta.is_premium {
        s.push("🅟");
    }
    if meta.is_admin {
        s.push("🅐");
    }
    if meta.is_group {
        s.push("🅖");
    }
    if meta.is_private {
        s.push("🅟🅡");
    }
    if s.is_empty() {
        String::new()
    } else {
        format!(" {}", s.join(" "))
    }
}

/// Satu command yang ditampilkan.
struct Cmd {
    nama: &'static str,
    simbol: String,
}

/// Nama command per kategori — `getCommandsByCategory()` (`plugins.js:395-410`).
///
/// Plugin `isEnabled = false` dilewati, sama seperti JS. Kategori `owner`
/// hanya untuk owner (`allmenu.js:211`). Nama command diurutkan agar
/// hasilnya sama tiap kali — JS bergantung pada urutan pemuatan plugin.
fn command_per_kategori(ctx: &Ctx) -> Vec<(&'static str, Vec<Cmd>)> {
    let mode = mode_bot(ctx);
    let mut peta: BTreeMap<&'static str, Vec<Cmd>> = BTreeMap::new();
    for p in all_plugins() {
        let meta = &p.meta;
        if !meta.is_enabled {
            continue;
        }
        let kategori = meta.category.as_str();
        if kategori == "owner" && !ctx.is_owner {
            continue;
        }
        if !kategori_boleh(&mode, kategori) {
            continue;
        }
        peta.entry(kategori).or_default().push(Cmd {
            nama: meta.name.as_str(),
            simbol: simbol(meta),
        });
    }
    for cmds in peta.values_mut() {
        cmds.sort_by_key(|c| c.nama);
    }
    let mut hasil: Vec<(&'static str, Vec<Cmd>)> = peta.into_iter().collect();
    hasil.sort_by_key(|(cat, _)| {
        URUTAN
            .iter()
            .position(|u| u == cat)
            .map_or(999, |i| i as i32)
    });
    hasil
}

/// Kotak kategori — `createBracketBox()` (`allmenu.js:70-77`).
///
/// JS mengirim emoji ke fungsi ini tapi tidak pernah memakainya, jadi
/// kategorinya tampil tanpa emoji. Di sini emoji dipakai di judul kotak —
/// itu memang maksud `CATEGORY_EMOJIS` di JS.
fn kotak(emoji: &str, judul: &str, baris: &[String]) -> String {
    let mut t = format!("╭➤--「{emoji} {judul} MENU 」\n");
    for b in baris {
        t.push_str(&format!("┆ ⇝ {b}\n"));
    }
    t.push_str("╰➤-------------------------------\n\n");
    t
}

/// Daftar command per kategori — bagian bawah `txt` (`allmenu.js:210-230`).
fn teks_daftar(ctx: &Ctx, kategori: &[(&'static str, Vec<Cmd>)]) -> String {
    let prefix = prefix(ctx);
    let mut t = String::new();
    for (cat, cmds) in kategori {
        if cmds.is_empty() {
            continue;
        }
        let baris: Vec<String> = cmds
            .iter()
            .map(|c| format!("{prefix}{}{}", c.nama, c.simbol))
            .collect();
        t.push_str(&kotak(emoji(cat), &cat.to_uppercase(), &baris));
    }
    t
}

/// Blok info user + bot (`allmenu.js:142-157`).
///
/// `total_fitur` = jumlah command plugin yang ditampilkan. JS menambah
/// `getCaseCount()` (3 command dari `case/foto.js`) — tidak ada di Rust.
fn teks_info(ctx: &Ctx, total_fitur: usize) -> String {
    let push = if ctx.push_name.is_empty() {
        "User"
    } else {
        ctx.push_name.as_str()
    };
    let peran = if ctx.is_owner {
        "Owner"
    } else if ctx.is_premium {
        "Premium"
    } else {
        "User"
    };
    // JS `user?.limit` sudah jadi field `energi` di store Rust
    // (`oktz-wa/pipeline.rs:617`).
    let limit = if ctx.is_owner || ctx.is_premium {
        String::from("∞ Unlimited")
    } else {
        ctx.user_field("energi")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0)
            .to_string()
    };
    let (_, _, _, jam, menit, detik) = jakarta_parts(now_ms());
    let waktu = format!("{}:{}:{} WIB", pad2(jam), pad2(menit), pad2(detik));

    format!(
        "✦ *Informasi User*\n\
         • Nama   : {push}\n\
         • Status : {peran}\n\
         • Limit  : {limit}\n\n\
         ✦ *Informasi Bot*\n\
         • Nama   : {nama}\n\
         • Fitur  : {total_fitur} fitur\n\
         • Versi  : {versi}\n\
         • Waktu  : {waktu}\n\n\
         ✦ *Kelebihan Bot Ini*\n\
         • Fast response\n\
         • Stable system\n\
         • Multi feature\n\
         • User friendly\n\n\
         Silakan pilih menu di bawah ini.\n\
         Gunakan sesuai kebutuhan dan jangan lupa gunakan dengan bijak.\n\n",
        push = push,
        nama = nama_bot(ctx),
        versi = versi_bot(ctx),
    )
}

/// Body kartu — sapaan (`allmenu.js:254`) + daftar command.
fn body_kartu(ctx: &Ctx, daftar: &str) -> String {
    let push = if ctx.push_name.is_empty() {
        "User"
    } else {
        ctx.push_name.as_str()
    };
    format!(
        "> Halo *{push}* 👋 Perkenalkan aku *{nama}*, buatan *{dev}* \
         Saya siap membantu kebutuhanmu, mulai dari download video, main game, \
         tanya jawab, nyari info/sesuatu, bikin sticker, dan lain-lain.\n\n{daftar}",
        push = push,
        nama = nama_bot(ctx),
        dev = pengembang(ctx),
    )
}

#[plugin(
    name = "allmenu",
    alias = ["fullmenu", "allcommand", "semua"],
    category = "main",
    description = "Menampilkan semua command lengkap per kategori",
    usage = ".allmenu",
    example = ".allmenu",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn allmenu(ctx: Ctx) -> Reply {
    let kategori = command_per_kategori(&ctx);
    let total: usize = kategori.iter().map(|(_, cmds)| cmds.len()).sum();
    let info = teks_info(&ctx, total);
    let daftar = teks_daftar(&ctx, &kategori);
    let pref = prefix(&ctx);

    Reply::interactive(body_kartu(&ctx, &daftar))
        .with_footer(info.clone())
        .with_header(
            KartuHeader::baru()
                .judul("")
                .subjudul(nama_bot(&ctx))
                .media(MediaSource::bytes(ASET_FOTO.to_vec()).with_mimetype("image/jpeg")),
        )
        .with_button(Tombol::balas_cepat(
            "🍅 Kembali Ke Menu Utama",
            format!("{pref}menu"),
        ))
        .with_params(serde_json::json!({
            "limited_time_offer": {
                "text": sapa(),
                "url": "Hai",
                "copy_code": format!("Dibuat oleh {}", pengembang(&ctx)),
                "expiration_time": now_ms() + 1_000_000,
            }
        }))
        .with_mentions(vec![ctx.sender.clone()])
        .with_forwarded()
        .with_forwarding_score(9)
        .with_participant("0@s.whatsapp.net")
        .with_quote_teks(nama_bot(&ctx))
        .with_fallback_teks(format!("{info}{daftar}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Kartu;

    fn ctx() -> Ctx {
        Ctx {
            prefix: ".".into(),
            push_name: "Oktz".into(),
            sender: "628111@s.whatsapp.net".into(),
            config: std::sync::Arc::new(oktz_plugin_api::config::Runtime {
                bot: oktz_plugin_api::config::BotInfo {
                    name: "Oktz Bot".into(),
                    version: "3.3.1".into(),
                    developer: "Zann".into(),
                    website: String::new(),
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn kartu(r: Reply) -> Kartu {
        match r.without_mutations() {
            // `without_mutations()` mengembalikan `&Reply`, jadi `kartu`
            // terikat sebagai `&Kartu` — perlu di-clone.
            Reply::Interactive { kartu } => (**kartu).clone(),
            lain => panic!("harus kartu interaktif, bukan {lain:?}"),
        }
    }

    /// Nama kategori yang tampil untuk `ctx` given.
    fn kategori_dari(c: &Ctx) -> Vec<&'static str> {
        command_per_kategori(c)
            .into_iter()
            .map(|(cat, _)| cat)
            .collect()
    }

    #[test]
    fn kartu_terbentuk_dengan_header_gambar_dan_konteks() {
        let k = kartu(allmenu(ctx()));
        let header = k.header.as_ref().expect("header kartu harus ada");
        assert_eq!(header.title.as_deref(), Some(""));
        assert_eq!(header.subtitle.as_deref(), Some("Oktz Bot"));
        match &header.media {
            Some(MediaSource::Bytes { mimetype, .. }) => {
                assert_eq!(mimetype.as_deref(), Some("image/jpeg"));
            }
            lain => panic!("header harus berisi gambar, bukan {lain:?}"),
        }

        assert_eq!(k.mentions, vec!["628111@s.whatsapp.net"]);
        assert!(k.forwarded);
        assert_eq!(k.forwarding_score, Some(9));
        assert_eq!(k.participant.as_deref(), Some("0@s.whatsapp.net"));
        assert_eq!(k.quote_teks.as_deref(), Some("Oktz Bot"));
        assert!(k.params.is_some());
    }

    #[test]
    fn body_memuat_nama_plugin_dari_registry() {
        let k = kartu(allmenu(ctx()));
        let contoh = all_plugins()
            .iter()
            .find(|p| p.meta.category == "main" && p.meta.is_enabled)
            .map(|p| p.meta.name.clone())
            .expect("ada plugin di kategori main");
        let needle = format!(".{contoh}");
        assert!(
            k.body.contains(needle.as_str()),
            "body harus memuat {needle}: {}",
            k.body
        );
        // Nama plugin kategori lain juga harus muncul.
        let contoh_lain = all_plugins()
            .iter()
            .find(|p| p.meta.category == "utility" && p.meta.is_enabled)
            .map(|p| p.meta.name.clone())
            .expect("ada plugin di kategori utility");
        let needle_lain = format!(".{contoh_lain}");
        assert!(
            k.body.contains(needle_lain.as_str()),
            "body harus memuat {needle_lain}: {}",
            k.body
        );
    }

    #[test]
    fn body_memuat_kotak_kategori_dengan_nama_kategori() {
        let k = kartu(allmenu(ctx()));
        assert!(k.body.contains("MAIN MENU"), "{}", k.body);
        assert!(k.body.contains("┆ ⇝ ."), "{}", k.body);
        assert!(k.body.contains("╭➤--「"), "{}", k.body);
        assert!(k.body.contains("Halo *Oktz*"), "{}", k.body);
    }

    #[test]
    fn tombol_kembali_ke_menu_utama_ada() {
        let k = kartu(allmenu(ctx()));
        let t = k
            .buttons
            .iter()
            .find(|b| b.aksi.as_str() == "quick_reply")
            .expect("harus ada tombol quick_reply");
        assert_eq!(t.params["display_text"], "🍅 Kembali Ke Menu Utama");
        assert_eq!(t.params["id"], ".menu");
    }

    #[test]
    fn footer_memuat_info_user_dan_bot() {
        let k = kartu(allmenu(ctx()));
        let f = k.footer.as_deref().expect("footer harus ada");
        assert!(f.contains("Informasi User"), "{f}");
        assert!(f.contains("Nama   : Oktz"), "{f}");
        assert!(f.contains("Status : User"), "{f}");
        assert!(f.contains("Informasi Bot"), "{f}");
        assert!(f.contains("Nama   : Oktz Bot"), "{f}");
        assert!(f.contains("Fitur  : "), "{f}");
        assert!(f.contains("Versi  : 3.3.1"), "{f}");
        assert!(f.contains("WIB"), "{f}");
    }

    #[test]
    fn teks_cadangan_memuat_daftar_command() {
        let k = kartu(allmenu(ctx()));
        let cadangan = k.fallback_teks.expect("teks cadangan harus ada");
        assert!(cadangan.contains("Informasi Bot"), "{cadangan}");
        assert!(cadangan.contains("MAIN MENU"), "{cadangan}");
        assert!(cadangan.contains(".menu"), "{cadangan}");
    }

    #[test]
    fn kategori_owner_hanya_untuk_owner() {
        let semua = kategori_dari(&ctx());
        assert!(!semua.contains(&"owner"), "{semua:?}");

        let mut c = ctx();
        c.is_owner = true;
        let dengan_owner = kategori_dari(&c);
        assert!(dengan_owner.contains(&"owner"), "{dengan_owner:?}");
        // Kategori owner menambah satu kategori, tidak mengubah yang lain.
        assert_eq!(dengan_owner.len(), semua.len() + 1, "{dengan_owner:?}");
    }

    #[test]
    fn mode_md_menyembunyikan_panel_pushkontak_store() {
        let semua = kategori_dari(&ctx());
        for cat in ["panel", "pushkontak", "store"] {
            assert!(!semua.contains(&cat), "{cat} harus disembunyikan: {semua:?}");
        }
    }

    #[test]
    fn mode_cpanel_menampilkan_kategori_panel_saja_dari_grup() {
        let mut c = ctx();
        c.is_group = true;
        c.group_data = serde_json::json!({ "botMode": "cpanel" });
        let semua = kategori_dari(&c);
        assert!(semua.contains(&"panel"), "{semua:?}");
        assert!(!semua.contains(&"store"), "{semua:?}");
        assert!(!semua.contains(&"pushkontak"), "{semua:?}");
    }

    #[test]
    fn urutan_kategori_ikut_daftar_urutan() {
        let mut c = ctx();
        c.is_owner = true;
        let semua = kategori_dari(&c);
        let posisi = |nama: &str| semua.iter().position(|x| *x == nama);
        let owner = posisi("owner").expect("kategori owner harus ada");
        let main = posisi("main").expect("kategori main harus ada");
        assert!(owner < main, "{semua:?}");
        // Kategori di luar `URUTAN` (mis. `anime`) tetap tampil, tapi setelahnya.
        let utility = posisi("utility").expect("kategori utility harus ada");
        let anime = posisi("anime").expect("kategori anime harus ada");
        assert!(anime > utility, "{semua:?}");
    }

    #[test]
    fn plugin_terdaftar_dengan_meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("allmenu").expect("allmenu harus terdaftar");
        assert_eq!(p.meta.name, "allmenu");
        assert_eq!(p.meta.alias, ["fullmenu", "allcommand", "semua"]);
        assert_eq!(p.meta.category, "main");
        assert_eq!(
            p.meta.description,
            "Menampilkan semua command lengkap per kategori"
        );
        assert_eq!(p.meta.usage, ".allmenu");
        assert_eq!(p.meta.example, ".allmenu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group);
        assert!(!p.meta.is_private);
    }
}
