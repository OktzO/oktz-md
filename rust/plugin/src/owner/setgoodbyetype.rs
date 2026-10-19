//! `plugins/owner/setgoodbyetype.js` — mengatur variant tampilan goodbye message.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod setgoodbyetype;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/owner/mod.rs`.
//!
//! `getAssetBuffer("foto")` dan `sock.sendButton` tidak punya padanan di
//! `Ctx` / `Reply`. Tombol interaktif diabaikan; handler mengembalikan teks
//! yang berisi daftar varian. `db.setting` ditulis lewat
//! `Mutation::SetSetting`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const VARIANTS: &[(&str, &str, &str, &str)] = &[
    ("1", "Canvas Image", "🎨", "Gambar canvas dengan foto profil"),
    ("2", "Carousel Cards", "🃏", "Kartu carousel interaktif dengan tombol"),
    ("3", "Text Only", "📝", "Pesan teks minimalis tanpa gambar"),
    ("4", "Group", "👥", "ContextInfo group style"),
    ("5", "Simple", "✨", "Pesan teks simple + foto profile"),
    ("6", "Video", "🎥", "Kirim video ucapan selamat tinggal"),
    ("7", "Interactive Quoted", "💬", "Interactive message dengan fake quoted"),
    ("8", "Super Simple", "👋", "Pesan teks sangat singkat tanpa hiasan"),
];

fn variant_nama(id: &str) -> Option<&'static str> {
    VARIANTS.iter().find(|(i, ..)| *i == id).map(|(_, n, ..)| *n)
}

fn variant_desc(id: &str) -> Option<&'static str> {
    VARIANTS.iter().find(|(i, ..)| *i == id).map(|(_, _, _, d)| *d)
}

fn variant_emoji(id: &str) -> Option<&'static str> {
    VARIANTS.iter().find(|(i, ..)| *i == id).map(|(_, _, e, _)| *e)
}

fn daftar_varian(prefix: &str, current: i64) -> String {
    let mut rows = String::new();
    for (id, nama, emoji, desc) in VARIANTS {
        let mark = if id.parse::<i64>().unwrap_or(0) == current {
            " ✓"
        } else {
            ""
        };
        rows.push_str(&format!(
            "{} V{}{mark} — {}\n   {}\n",
            emoji, id, nama, desc
        ));
    }
    format!(
        "👋🚪 *GOODBYE TYPE*\n\n\
         Atur tampilan pesan goodbye saat member keluar dari grup 🚶💨\n\
         Tipe aktif saat ini: *V{current} — {}* 🎯\n\n\
         *PENJELASAN TIPE:*\n\n{rows}\
         > Ketik `{prefix}setgoodbyetype v<N>` untuk memilih (N = 1–8).",
        variant_nama(&current.to_string()).unwrap_or("Tidak diketahui")
    )
}

#[plugin(
    name = "setgoodbyetype",
    alias = ["goodbyetype", "goodbyevariant", "goodbyestyle"],
    category = "owner",
    description = "Mengatur variant tampilan goodbye message",
    usage = ".setgoodbyetype",
    example = ".setgoodbyetype",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setgoodbyetype(ctx: Ctx) -> Reply {
    let args = &ctx.args;
    let variant = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let current = ctx
        .setting("goodbyeType")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);

    if !variant.is_empty() {
        let v = variant.trim_start_matches('v');
        if VARIANTS.iter().any(|(i, ..)| *i == v) {
            let id: i64 = v.parse().unwrap_or(1);
            return Reply::text(format!(
                "✅ *GOODBYE TYPE DIUBAH*\n\n\
                 {} *V{id} — {}*\n\
                 _{}_",
                variant_emoji(v).unwrap_or(""),
                variant_nama(v).unwrap_or("Tidak diketahui"),
                variant_desc(v).unwrap_or("")
            ))
            .with_mutation(Mutation::SetSetting {
                key: "goodbyeType".to_string(),
                value: serde_json::json!(id),
            });
        }
    }

    Reply::text(daftar_varian(&ctx.prefix, current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "setgoodbyetype".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("setgoodbyetype").unwrap();
        assert_eq!(p.meta.name, "setgoodbyetype");
        assert_eq!(
            p.meta.alias,
            ["goodbyetype", "goodbyevariant", "goodbyestyle"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["goodbyetype", "goodbyevariant", "goodbyestyle"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "setgoodbyetype"
            );
        }
    }

    #[test]
    fn tanpa_args_mengembalikan_daftar_varian() {
        let r = setgoodbyetype(ctx());
        assert!(text_of(&r).contains("👋🚪 *GOODBYE TYPE*"));
        assert!(text_of(&r).contains("V1"));
        assert!(text_of(&r).contains("V8"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn variant_valid_mengubah_setting() {
        let mut c = ctx();
        c.args = vec!["v3".into()];
        let r = setgoodbyetype(c);
        assert!(text_of(&r).contains("GOODBYE TYPE DIUBAH"));
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "goodbyeType");
                assert_eq!(*value, serde_json::json!(3));
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn variant_dengan_prefix_v_bisa_dipakai() {
        let mut c = ctx();
        c.args = vec!["v5".into()];
        let r = setgoodbyetype(c);
        assert!(text_of(&r).contains("V5 — Simple"));
    }

    #[test]
    fn variant_tidak_valid_mengembalikan_daftar() {
        let mut c = ctx();
        c.args = vec!["v99".into()];
        let r = setgoodbyetype(c);
        assert!(text_of(&r).contains("👋🚪 *GOODBYE TYPE*"));
        assert!(r.mutations().is_empty());
    }
}
