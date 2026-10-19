//! `plugins/owner/setreply.js` — atur variant tampilan reply.
//!
//! `ponytail:` tombol `single_select` + `sock.sendButton` tidak punya padanan
//! di `Ctx`/`Reply`. Baris tombol JS (`setreply.js:110-127`) dirender jadi
//! daftar teks di dalam body (`Reply::Interactive`), pola yang sama dengan
//! `owner/setgoodbyetype.rs`; penanda ` ✓` dan urutan varian dipertahankan.
//! `config.ui?.replyVariant` tidak ada di `Runtime`, jadi nilai awal variant
//! hanya `db.setting("replyVariant")` (`:108`).
//!
//! Quirk yang disalin apa adanya: pencocokan variant **case-sensitive**
//! (`VARIANTS[m.text]`, `:92`) — `V5` bukan `v5`; penanda ` ✓` hanya muncul
//! kalau nilai tersimpan benar-benar angka yang sama (`val.id === current`,
//! `:112`), jadi `"5"` (string) tidak menandai apa pun.
//!
//! Catatan: `mod.rs` kategori `owner` dimiliki plugin lain (tidak boleh
//! disentuh), jadi baris `pub mod setreply;` perlu ditambahkan manual.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

/// `VARIANTS` (`setreply.js:19-86`) — `(key, id, name, desc, emoji)`.
const VARIANTS: &[(&str, i64, &str, &str, &str)] = &[
    ("v1", 1, "BASIC", "", "✨"),
    ("v2", 2, "PREMIUM", "", "🖼️"),
    ("v3", 3, "TITANIUM", "", "📨"),
    ("v4", 4, "LV", "", "💼"),
    ("v5", 5, "FAKE ORDER", "Teks dengan fake quoted order message", "🛒"),
    (
        "v6",
        6,
        "SIMPLE DOCUMENT",
        "Seperti V2 tapi tanpa fake contact quote (quoted ori)",
        "📄",
    ),
    ("v7", 7, "FAKE LOCATION", "Teks dengan fake quoted location message", "📍"),
    ("v8", 8, "FAKE SIGNUP", "Teks dengan fake quoted signup message", "🥠"),
    (
        "v9",
        9,
        "FAKE ORDER",
        "Teks dengan fake quoted order message ( gak direkomendasikan )",
        "🍙",
    ),
    ("v10", 10, "TAGIHAN / PAYMENT", "Tampilan struk tagihan (Payment Request)", "💳"),
    ("v11", 11, "PREVIEW", "Link preview", "🌀"),
];

/// `VARIANTS[key]` — pencarian persis, bukan case-insensitive.
fn cari(key: &str) -> Option<(&'static str, i64, &'static str, &'static str, &'static str)> {
    VARIANTS.iter().find(|(k, ..)| *k == key).copied()
}

/// `${v}` JS untuk satu nilai JSON — string apa adanya, angka tanpa `.0`.
fn teks(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n
            .as_i64()
            .map_or_else(|| n.as_f64().map(|f| format!("{f}")).unwrap_or_default(), |i| i.to_string()),
        other => other.to_string(),
    }
}

/// `val.id === current` (`setreply.js:112`) — strict: hanya angka yang sama.
fn id_sama(id: i64, current: &Value) -> bool {
    current.as_f64().is_some_and(|c| c == id as f64)
}

/// Body daftar: teks JS apa adanya (`:129-132`) + baris tombol jadi teks
/// (`:110-118`) + petunjuk memilih.
fn daftar(prefix: &str, current: &Value) -> String {
    let kunci_aktif = format!("v{}", teks(current));
    let nama_aktif = cari(&kunci_aktif).map_or("Unknown", |(_, _, nama, ..)| nama);
    let mut body = format!(
        "💬📨 *REPLY VARIANT*\n\n\
         Atur tampilan balasan bot ketika membalas pesan user 💬✨\n\
         Variant aktif saat ini: *V{} — {nama_aktif}* 🎯\n\n",
        teks(current)
    );
    for (key, id, nama, desc, emoji) in VARIANTS {
        let mark = if id_sama(*id, current) { " ✓" } else { "" };
        body.push_str(&format!(
            "{emoji} {}{mark} — {nama}\n   {desc}\n",
            key.to_uppercase()
        ));
    }
    body.push_str(&format!(
        "> Ketik `{prefix}setreply v<N>` untuk memilih (N = 1–11)."
    ));
    body
}

#[plugin(
    name = "setreply",
    alias = ["replyvariant", "replystyle"],
    category = "owner",
    description = "Mengatur variant tampilan reply",
    usage = ".setreply <v1-v11>",
    example = ".setreply v5",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setreply(ctx: Ctx) -> Reply {
    // `const variant = m.text` — TIDAK di-trim dan TIDAK di-lowercase (`:89`).
    let variant = ctx.text.clone();

    if !variant.is_empty() {
        let Some((_, id, nama, desc, emoji)) = cari(&variant) else {
            return Reply::text("❌ *VARIANT TIDAK VALID*\n\nGunakan: *v1* s/d *v11*");
        };
        return Reply::text(format!(
            "✅ *REPLY VARIANT DIUBAH*\n\n{emoji} *V{id} — {nama}*\n_{desc}_"
        ))
        .with_mutation(Mutation::SetSetting {
            key: "replyVariant".to_string(),
            value: serde_json::json!(id),
        });
    }

    // `db.setting("replyVariant") || config.ui?.replyVariant || 1` (`:108`) —
    // `config.ui` tidak ada, jadi nilai falsy jatuh ke 1.
    let current = ctx
        .setting("replyVariant")
        .filter(|v| crate::common::js_truthy_value(v))
        .cloned()
        .unwrap_or(serde_json::json!(1));

    Reply::Interactive {
        body: daftar(&ctx.prefix, &current),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const ME: &str = "628111@s.whatsapp.net";

    fn run(teks: &str, settings: Value) -> Reply {
        setreply(Ctx {
            text: teks.into(),
            args: teks.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "setreply".into(),
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

    fn variant_tersimpan(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "replyVariant" => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("setreply").unwrap();
        assert_eq!(p.meta.name, "setreply");
        assert_eq!(p.meta.alias, ["replyvariant", "replystyle"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Mengatur variant tampilan reply");
        assert_eq!(p.meta.usage, ".setreply <v1-v11>");
        assert_eq!(p.meta.example, ".setreply v5");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
    }

    #[test]
    fn alias_menunjuk_ke_plugin_ini() {
        for a in ["replyvariant", "replystyle"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("setreply"), "{a}");
        }
    }

    #[test]
    fn v5_menyimpan_id_dan_membalas_persis_js() {
        let r = run("v5", Value::Null);
        assert_eq!(
            text_of(&r),
            "✅ *REPLY VARIANT DIUBAH*\n\n\
             🛒 *V5 — FAKE ORDER*\n\
             _Teks dengan fake quoted order message_"
        );
        assert_eq!(variant_tersimpan(&r), Some(serde_json::json!(5)));
    }

    #[test]
    fn v9_punya_desc_dengan_spasi_di_dalam_kurung() {
        // JS menulis "( gak direkomendasikan )" apa adanya (`:71`).
        let r = run("v9", Value::Null);
        assert!(
            text_of(&r)
                .contains("_Teks dengan fake quoted order message ( gak direkomendasikan )_"),
            "{}",
            text_of(&r)
        );
        assert_eq!(variant_tersimpan(&r), Some(serde_json::json!(9)));
    }

    #[test]
    fn desc_kosong_tetap_menulis_dua_garis_bawah() {
        // V1–V4 `desc: ""` -> `_${desc}_` jadi `__`.
        let r = run("v1", Value::Null);
        assert_eq!(
            text_of(&r),
            "✅ *REPLY VARIANT DIUBAH*\n\n✨ *V1 — BASIC*\n__"
        );
    }

    #[test]
    fn v11_kenal_sebagai_varian_terakhir() {
        let r = run("v11", Value::Null);
        assert!(text_of(&r).contains("🌀 *V11 — PREVIEW*"), "{}", text_of(&r));
        assert_eq!(variant_tersimpan(&r), Some(serde_json::json!(11)));
    }

    #[test]
    fn pencocokan_variant_case_sensitive() {
        // `VARIANTS["V5"]` = undefined di JS (`:92`).
        let r = run("V5", Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ *VARIANT TIDAK VALID*\n\nGunakan: *v1* s/d *v11*"
        );
        assert!(variant_tersimpan(&r).is_none());
    }

    #[test]
    fn variant_tidak_dikenal_ditolak() {
        for teks in ["v12", "v0", "abc", "v5 extra", " v5"] {
            let r = run(teks, Value::Null);
            assert_eq!(
                text_of(&r),
                "❌ *VARIANT TIDAK VALID*\n\nGunakan: *v1* s/d *v11*",
                "{teks:?}"
            );
        }
    }

    #[test]
    fn tanpa_teks_menampilkan_daftar_sebelas_varian() {
        let r = run("", Value::Null);
        let body = text_of(&r);
        assert!(body.contains("💬📨 *REPLY VARIANT*"), "{body}");
        assert!(
            body.contains("Variant aktif saat ini: *V1 — BASIC* 🎯"),
            "{body}"
        );
        assert_eq!(body.matches("\n   ").count(), 11, "harus 11 baris tombol: {body}");
        for (_, _, nama, ..) in VARIANTS {
            assert!(body.contains(*nama), "nama varian {nama} tidak ada: {body}");
        }
        assert!(r.mutations().is_empty(), "daftar tidak menulis apa pun");
    }

    #[test]
    fn tanda_aktif_hanya_kalau_nilainya_angka_yang_sama() {
        // `val.id === current` strict: 3 menandai V3.
        let r = run("", serde_json::json!({"replyVariant": 3}));
        let body = text_of(&r);
        assert!(body.contains("📨 V3 ✓ — TITANIUM"), "{body}");
        assert!(!body.contains("V1 ✓"), "hanya V3 yang ditandai: {body}");
        assert!(body.contains("Variant aktif saat ini: *V3 — TITANIUM* 🎯"), "{body}");

        // `"3"` string tidak === 3, jadi tidak ada tanda, tapi nama tetap kebaca.
        let r = run("", serde_json::json!({"replyVariant": "3"}));
        let body = text_of(&r);
        assert!(!body.contains('✓'), "string tidak menandai apa pun: {body}");
        assert!(body.contains("Variant aktif saat ini: *V3 — TITANIUM* 🎯"), "{body}");
    }

    #[test]
    fn nilai_tersimpan_bukan_variant_memberi_unknown() {
        let r = run("", serde_json::json!({"replyVariant": "9x"}));
        let body = text_of(&r);
        assert!(body.contains("Variant aktif saat ini: *V9x — Unknown* 🎯"), "{body}");
        assert!(!body.contains('✓'), "{body}");
    }

    #[test]
    fn nilai_falsy_jatuh_ke_v1() {
        for falsy in [Value::Null, serde_json::json!(0), serde_json::json!("")] {
            let r = run("", serde_json::json!({ "replyVariant": falsy }));
            assert!(
                text_of(&r).contains("Variant aktif saat ini: *V1 — BASIC* 🎯"),
                "{falsy:?}"
            );
        }
    }
}
