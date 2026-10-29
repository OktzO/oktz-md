//! `plugins/owner/setwelcometype.js` — atur variant tampilan welcome message.
//!
//! `ponytail:` tombol `single_select` + `sock.sendButton` tidak punya padanan
//! di `Ctx`/`Reply`. Body JS dikirim apa adanya (`setwelcometype.js:72-85`,
//! termasuk blok `*PENJELASAN TIPE:*` yang sudah memuat semua keterangan),
//! lalu baris tombol (`:54-61`) dirender jadi daftar teks — pola yang sama
//! dengan `owner/setgoodbyetype.rs`.
//!
//! Quirk yang disalin: argumen di-lowercase (`:41`) dan wajib cocok
//! `/^v?[1-8]$/` (`:43`); `"v3 extra"` tetap sukses karena hanya argumen
//! pertama yang dibaca. `VARIANTS[current].name` melempar `TypeError` kalau
//! `welcomeType` tersimpan bukan 1–8 (`:75`), jadi cabang itu berakhir di
//! `error_template` seperti jalur catch framework JS.
//!
//! Catatan: `mod.rs` kategori `owner` dimiliki plugin lain (tidak boleh
//! disentuh), jadi baris `pub mod setwelcometype;` perlu ditambahkan manual.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

/// `VARIANTS` (`setwelcometype.js:17-38`) — `(id, name, desc, emoji)`.
const VARIANTS: &[(&str, &str, &str, &str)] = &[
    ("1", "Canvas Image", "Gambar canvas dengan foto profil", "🎨"),
    ("2", "Carousel Cards", "Kartu carousel interaktif dengan tombol", "🃏"),
    ("3", "Text Only", "Pesan teks minimalis tanpa gambar", "📝"),
    ("4", "Group", "ContextInfo group style", "👥"),
    ("5", "Simple", "Pesan teks simple + foto profile", "✨"),
    ("6", "Video", "Kirim video perkenalan", "🎥"),
    ("7", "Interactive Quoted", "Interactive message dengan fake quoted", "💬"),
    ("8", "Super Simple", "Pesan teks sangat singkat tanpa hiasan", "👋"),
];

/// `VARIANTS[id]` — pencarian key persis (`id` sudah di-string-kan JS).
fn cari(id: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    VARIANTS.iter().find(|(i, ..)| *i == id).copied()
}

/// `${v}` JS untuk satu nilai JSON — string apa adanya, angka tanpa `.0`.
fn teks(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_i64().map_or_else(
            || n.as_f64().map(|f| format!("{f}")).unwrap_or_default(),
            |i| i.to_string(),
        ),
        other => other.to_string(),
    }
}

/// `parseInt(id) === current` (`setwelcometype.js:56`) — strict: hanya angka
/// yang sama.
fn id_sama(id: &str, current: &Value) -> bool {
    let id = id.parse::<i64>().unwrap_or_default();
    current.as_f64().is_some_and(|c| c == id as f64)
}

/// `/^v?[1-8]$/` (`setwelcometype.js:43`), argumen sudah lowercase.
fn cocok(variant: &str) -> bool {
    let bersih = variant.strip_prefix('v').unwrap_or(variant);
    matches!(bersih, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8")
}

/// Body daftar, `None` kalau `VARIANTS[current]` tidak ada — padanan
/// `TypeError` JS pada `VARIANTS[current].name` (`:75`).
fn daftar(prefix: &str, current: &Value) -> Option<String> {
    let (_, nama, ..) = cari(&teks(current))?;
    let mut body = format!(
        "👋🎨 *WELCOME TYPE*\n\n\
         Atur tampilan pesan welcome saat member baru masuk grup 🚪✨\n\
         Tipe aktif saat ini: *V{} — {nama}* 🎯\n\n\
         *PENJELASAN TIPE:*\n\n\
         - *V1 Canvas Image* 🎨 — Bot membuat gambar canvas otomatis berisi foto profil dan nama member yang baru join, lalu dikirim sebagai gambar\n\n\
         - *V2 Carousel Cards* 🃏 — Menampilkan kartu carousel interaktif yang bisa di-swipe lengkap dengan tombol action, cocok untuk grup yang ingin tampilan modern\n\n\
         - *V3 Text Only* 📝 — Pesan teks biasa tanpa gambar sama sekali, ringan dan minimalis\n\n\
         - *V4 Group* 👥 — Menggunakan contextInfo bergaya group forward, tampilan rapi dengan label newsletter\n\n\
         - *V5 Simple* ✨ — Pesan teks sederhana disertai foto profile member yang join, tidak terlalu mencolok namun informatif\n\n\
         - *V6 Video* 🎥 — Mengirimkan video sambutan menarik dilengkapi caption selamat datang untuk member\n\n\
         - *V7 Interactive Quoted* 💬 — Mengirimkan pesan interaktif dan fake quoted dari orang yang join\n\n\
         - *V8 Super Simple* 👋 — Pesan teks sangat singkat tanpa hiasan (Contoh: Hai @user, welcome to grup...)\n\n\
         > Pilih tipe welcome dari tombol di bawah 👇",
        teks(current)
    );
    for (id, nama, desc, emoji) in VARIANTS {
        let mark = if id_sama(id, current) { " ✓" } else { "" };
        body.push_str(&format!("\n\n{emoji} V{id}{mark} — {nama}\n   {desc}"));
    }
    body.push_str(&format!(
        "\n\n> Ketik `{prefix}setwelcometype v<N>` untuk memilih (N = 1–8)."
    ));
    Some(body)
}

#[plugin(
    name = "setwelcometype",
    alias = ["welcometype", "welcomevariant", "welcomestyle"],
    category = "owner",
    description = "Mengatur variant tampilan welcome message",
    usage = ".setwelcometype",
    example = ".setwelcometype",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setwelcometype(ctx: Ctx) -> Reply {
    // `args[0]?.toLowerCase()` (`:41`).
    let variant = ctx.args.first().map(|a| a.to_lowercase());
    // `db.setting("welcomeType") || 1` (`:42`).
    let current = ctx
        .setting("welcomeType")
        .filter(|v| crate::common::js_truthy_value(v))
        .cloned()
        .unwrap_or(serde_json::json!(1));

    if let Some(v) = &variant
        && cocok(v)
    {
        // `parseInt(variant.replace("v", ""))` (`:44`) — regex di atas menjamin
        // 1–8, jadi hasilnya selalu id yang ada di `VARIANTS`.
        let id: i64 = v.strip_prefix('v').unwrap_or(v).parse().unwrap_or(1);
        let (_, nama, desc, emoji) = cari(&id.to_string()).unwrap_or(("", "", "", ""));
        return Reply::text(format!(
            "✅ *WELCOME TYPE DIUBAH*\n\n{emoji} *V{id} — {nama}*\n_{desc}_"
        ))
        .with_mutation(Mutation::SetSetting {
            key: "welcomeType".to_string(),
            value: serde_json::json!(id),
        });
    }

    match daftar(&ctx.prefix, &current) {
        Some(body) => Reply::interactive(body),
        None => Reply::text(crate::common::error_template(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const ME: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], settings: Value) -> Reply {
        setwelcometype(Ctx {
            text: args.join(" "),
            args: args.iter().map(|s| s.to_string()).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setwelcometype".into(),
            settings,
            ..Ctx::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn tipe_tersimpan(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "welcomeType" => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("setwelcometype").unwrap();
        assert_eq!(p.meta.name, "setwelcometype");
        assert_eq!(p.meta.alias, ["welcometype", "welcomevariant", "welcomestyle"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Mengatur variant tampilan welcome message");
        assert_eq!(p.meta.usage, ".setwelcometype");
        assert_eq!(p.meta.example, ".setwelcometype");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
    }

    #[test]
    fn alias_menunjuk_ke_plugin_ini() {
        for a in ["welcometype", "welcomevariant", "welcomestyle"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("setwelcometype"),
                "{a}"
            );
        }
    }

    #[test]
    fn v3_menyimpan_id_dan_membalas_persis_js() {
        let r = run(&["v3"], Value::Null);
        assert_eq!(
            text_of(&r),
            "✅ *WELCOME TYPE DIUBAH*\n\n📝 *V3 — Text Only*\n_Pesan teks minimalis tanpa gambar_"
        );
        assert_eq!(tipe_tersimpan(&r), Some(serde_json::json!(3)));
    }

    #[test]
    fn argumen_besar_dan_prefix_v_diterima() {
        // `args[0]?.toLowerCase()` + `/^v?[1-8]$/` (`:41-43`).
        let r = run(&["V5"], Value::Null);
        assert!(text_of(&r).contains("✨ *V5 — Simple*"), "{}", text_of(&r));
        assert_eq!(tipe_tersimpan(&r), Some(serde_json::json!(5)));

        let r = run(&["8"], Value::Null);
        assert!(text_of(&r).contains("👋 *V8 — Super Simple*"), "{}", text_of(&r));
    }

    #[test]
    fn hanya_argumen_pertama_yang_dibaca() {
        // `args[0]` — sisa teks diabaikan, jadi "v2 apa saja" tetap sukses.
        let r = run(&["v2", "apa", "saja"], Value::Null);
        assert!(text_of(&r).contains("🃏 *V2 — Carousel Cards*"), "{}", text_of(&r));
        assert_eq!(tipe_tersimpan(&r), Some(serde_json::json!(2)));
    }

    #[test]
    fn varian_di_luar_regex_mengembalikan_daftar() {
        for teks in ["v9", "0", "v0", "9", "v1v", "abc", "v3extra"] {
            let r = run(&[teks], Value::Null);
            assert!(text_of(&r).contains("👋🎨 *WELCOME TYPE*"), "{teks:?}");
            assert!(tipe_tersimpan(&r).is_none(), "{teks:?} tidak boleh menyimpan");
        }
    }

    #[test]
    fn tanpa_argumen_menampilkan_daftar_lengkap() {
        let r = run(&[], Value::Null);
        let body = text_of(&r);
        assert!(body.contains("Tipe aktif saat ini: *V1 — Canvas Image* 🎯"), "{body}");
        assert!(body.contains("🎨 V1 ✓ — Canvas Image"), "{body}");
        assert!(!body.contains("V2 ✓"), "{body}");
        assert_eq!(body.matches("\n   ").count(), 8, "harus 8 baris tombol: {body}");
        assert!(r.mutations().is_empty(), "daftar tidak menulis apa pun");
    }

    #[test]
    fn blok_penjelasan_js_dan_penutupnya_utuh() {
        let body = text_of(&run(&[], Value::Null));
        for baris in [
            "*PENJELASAN TIPE:*",
            "- *V1 Canvas Image* 🎨 — Bot membuat gambar canvas otomatis berisi foto profil dan nama member yang baru join, lalu dikirim sebagai gambar",
            "- *V8 Super Simple* 👋 — Pesan teks sangat singkat tanpa hiasan (Contoh: Hai @user, welcome to grup...)",
            "> Pilih tipe welcome dari tombol di bawah 👇",
            "> Ketik `.setwelcometype v<N>` untuk memilih (N = 1–8).",
        ] {
            assert!(body.contains(baris), "hilang: {baris}\n{body}");
        }
    }

    #[test]
    fn tanda_aktif_mengikuti_strict_equality_js() {
        // `parseInt(id) === current` (`:56`).
        let body = text_of(&run(&[], serde_json::json!({"welcomeType": 5})));
        assert!(body.contains("✨ V5 ✓ — Simple"), "{body}");
        assert!(!body.contains("V1 ✓"), "{body}");
        assert!(body.contains("Tipe aktif saat ini: *V5 — Simple* 🎯"), "{body}");

        // `"5"` string tidak === 5: tidak ada tanda, nama tetap kebaca.
        let body = text_of(&run(&[], serde_json::json!({"welcomeType": "5"})));
        assert!(!body.contains('✓'), "{body}");
        assert!(body.contains("Tipe aktif saat ini: *V5 — Simple* 🎯"), "{body}");
    }

    #[test]
    fn nilai_falsy_jatuh_ke_tipe_1() {
        for falsy in [Value::Null, serde_json::json!(0), serde_json::json!("")] {
            let body = text_of(&run(&[], serde_json::json!({ "welcomeType": falsy })));
            assert!(body.contains("*V1 — Canvas Image*"), "{falsy:?}");
        }
    }

    #[test]
    fn tipe_tersimpan_di_luar_1_sampai_8_berakhir_error_template() {
        // JS: `VARIANTS[current].name` melempar TypeError (`:75`).
        for rusak in [serde_json::json!(99), serde_json::json!("x"), serde_json::json!(true)] {
            let r = run(&[], serde_json::json!({ "welcomeType": rusak }));
            assert_eq!(
                text_of(&r),
                crate::common::error_template_bawaan(".", "setwelcometype"),
                "{rusak:?}"
            );
            assert!(r.mutations().is_empty(), "{rusak:?}");
        }
    }
}
