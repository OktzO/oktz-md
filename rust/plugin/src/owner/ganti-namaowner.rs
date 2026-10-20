//! `plugins/owner/ganti-namaowner.js` — ganti nama owner (utama atau tambahan).
//!
//! Yang dipindah: daftar nama (`ownerNames`) + set nama per nomor + prompt
//! `main` tanpa nama. Nama/nomor owner utama dibaca dari `ctx.config()`
//! (`config.owner.name` / `config.owner.number`), dan `getOwnerName`
//! (`config.js:537-552`) yang menentukan nama tampil saat nomor belum pernah
//! diberi nama baru.
//!
//! `ponytail:` cabang `main <nama>` di JS menulis berkas
//! `path.join(process.cwd(), "config.js")` lalu memutasi objek config live.
//! Di produksi `ctx.berkas` masih `BerkasPalsu` (memori) dan `ctx.config`
//! bertipe `Arc` yang tidak bisa dimutasi, jadi penulisan itu tidak punya efek
//! nyata; handler berhenti di jalur `catch` JS sendiri
//! (`te(m.prefix, m.command, m.pushName)`, `ganti-namaowner.js:68-70`).
//! Saat `ctx.berkas` jadi `BerkasNyata` dan ada mutasi config, cabang regex
//! `owner: {...name: '...'}` bisa diisi penuh.
//!
//! Catatan: berkas ini bernama `ganti-namaowner.rs` (pakai dash), jadi mod.rs
//! butuh `#[path = "ganti-namaowner.rs"] pub mod ganti_namaowner;`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

/// `config.owner?.name || "Owner"`.
fn nama_utama(ctx: &Ctx) -> String {
    let nama = &ctx.config().owner.name;
    if nama.is_empty() {
        "Owner".to_string()
    } else {
        nama.clone()
    }
}

/// `${v}` JS untuk nilai peta nama — string apa adanya, angka apa adanya.
fn teks(v: &Value) -> String {
    match v.as_str() {
        Some(s) => s.to_string(),
        None => v.to_string(),
    }
}

/// `normalizeNumber` (`config.js:345-352`).
fn normalisasi(nomor: &str) -> String {
    nomor
        .split('@')
        .next()
        .unwrap_or(nomor)
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

/// `matchesNumber` (`config.js:358-362`) — strict equality setelah normalisasi.
fn nomor_sama(a: &str, b: &str) -> bool {
    let na = normalisasi(a);
    let nb = normalisasi(b);
    !na.is_empty() && !nb.is_empty() && na == nb
}

/// `getOwnerName(number)` (`config.js:537-552`).
fn nama_owner(ctx: &Ctx, nomor: &str) -> String {
    if nomor.is_empty() {
        return nama_utama(ctx);
    }
    let bersih = normalisasi(nomor);
    if let Some(v) = ctx.setting("ownerNames").and_then(|m| m.get(&bersih))
        && crate::common::js_truthy_value(v)
    {
        return teks(v);
    }
    if ctx
        .config()
        .owner
        .number
        .iter()
        .any(|own| nomor_sama(&bersih, own))
    {
        return nama_utama(ctx);
    }
    "Owner".to_string()
}

/// `if (!input[0]) { ... }` (`ganti-namaowner.js:26-44`).
fn daftar_owner(ctx: &Ctx) -> String {
    // `config.owner?.number?.[0] || ""`.
    let nomor_utama = ctx.config().owner.number.first().cloned().unwrap_or_default();
    let mut list = String::from("👤 *ᴏᴡɴᴇʀ ɴᴀᴍᴇ ʟɪsᴛ*\n\n");
    list.push_str(&format!("👑 Main: *{}* ({nomor_utama})\n", nama_utama(ctx)));
    let mut ada = false;
    for (nomor, nama) in ctx.setting_entries("ownerNames") {
        ada = true;
        list.push_str(&format!("👤 {nomor}: *{}*\n", teks(nama)));
    }
    if !ada {
        list.push_str("\n> Belum ada nama custom untuk owner tambahan");
    }
    list.push_str(&format!(
        "\n\n*Penggunaan:*\n\
         `{p}ganti-namaowner <nomor> <nama>`\n\
         `{p}ganti-namaowner main <nama>` — ganti nama owner utama",
        p = ctx.prefix
    ));
    list
}

#[plugin(
    name = "ganti-namaowner",
    alias = ["setnamaowner", "setnameowner", "setownername"],
    category = "owner",
    description = "Ganti nama owner (utama atau tambahan)",
    usage = ".ganti-namaowner <nomor> <nama baru>",
    example = ".ganti-namaowner 628xxx Fauzan",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_namaowner(ctx: Ctx) -> Reply {
    let Some(pertama) = ctx.args.first() else {
        return Reply::text(daftar_owner(&ctx));
    };

    // `input.slice(1).join(" ").trim()`.
    let nama_baru = ctx.args[1..].join(" ").trim().to_string();

    if pertama.to_lowercase() == "main" {
        if nama_baru.is_empty() {
            let sekarang = if ctx.config().owner.name.is_empty() {
                "-".to_string()
            } else {
                ctx.config().owner.name.clone()
            };
            return Reply::text(format!(
                "👤 *ɢᴀɴᴛɪ ɴᴀᴍᴀ ᴏᴡɴᴇʀ ᴜᴛᴀᴍᴀ*\n\n\
                 > Nama saat ini: *{sekarang}*\n\n\
                 `{}ganti-namaowner main <nama baru>`",
                ctx.prefix
            ));
        }
        // ponytail: penulisan config.js tidak punya efek di produksi
        // (`ctx.berkas` = `BerkasPalsu`, `ctx.config` Arc tak termutasi). JS
        // berakhir di `catch` saat penulisan gagal, dan itu yang dikirim.
        return Reply::text(crate::common::error_template(&ctx));
    }

    // `input[0].replace(/[^0-9]/g, "")`.
    let nomor: String = pertama.chars().filter(char::is_ascii_digit).collect();

    if nomor.len() < 10 {
        return Reply::text(format!(
            "❌ *ɢᴀɢᴀʟ*\n\n> Nomor tidak valid\n\n\
             `{}ganti-namaowner 628xxx NamaOwner`",
            ctx.prefix
        ));
    }

    if nama_baru.is_empty() {
        return Reply::text(format!(
            "👤 *ɴᴀᴍᴀ ᴏᴡɴᴇʀ*\n\n> {nomor}: *{}*\n\n\
             `{}ganti-namaowner {nomor} <nama baru>`",
            nama_owner(&ctx, &nomor),
            ctx.prefix
        ));
    }

    // `db.setting("ownerNames") || {}` — nilai bukan objek diperlakukan sebagai
    // peta kosong (peta yang sah selalu objek; bentuk lain tidak punya nama).
    let mut peta = match ctx.setting("ownerNames") {
        Some(Value::Object(m)) => m.clone(),
        _ => serde_json::Map::new(),
    };
    peta.insert(nomor.clone(), Value::String(nama_baru.clone()));

    Reply::text(format!(
        "✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Nama owner *{nomor}* diganti ke: *{nama_baru}*"
    ))
    .with_mutation(Mutation::SetSetting {
        key: "ownerNames".to_string(),
        value: Value::Object(peta),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    fn runtime(nama: &str, nomor: &[&str]) -> Arc<oktz_plugin_api::config::Runtime> {
        Arc::new(oktz_plugin_api::config::Runtime {
            owner: oktz_plugin_api::config::Owner {
                name: nama.into(),
                number: nomor.iter().map(|s| s.to_string()).collect(),
            },
            ..oktz_plugin_api::config::Runtime::default()
        })
    }

    fn run_dengan(
        args: &[&str],
        settings: Value,
        config: Arc<oktz_plugin_api::config::Runtime>,
    ) -> Reply {
        ganti_namaowner(Ctx {
            text: args.join(" "),
            args: args.iter().map(|s| s.to_string()).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-namaowner".into(),
            settings,
            config,
            ..Ctx::default()
        })
    }

    fn run(args: &[&str], settings: Value) -> Reply {
        run_dengan(args, settings, runtime("Oktz", &["6285608953677"]))
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn peta_owner(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "ownerNames" => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = oktz_plugin_api::find("ganti-namaowner").unwrap();
        assert_eq!(p.meta.name, "ganti-namaowner");
        assert_eq!(
            p.meta.alias,
            ["setnamaowner", "setnameowner", "setownername"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Ganti nama owner (utama atau tambahan)");
        assert_eq!(p.meta.usage, ".ganti-namaowner <nomor> <nama baru>");
        assert_eq!(p.meta.example, ".ganti-namaowner 628xxx Fauzan");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
        assert!(!p.meta.is_bot_admin, "tidak ada di JS");
    }

    #[test]
    fn semua_alias_menunjuk_ke_plugin_ini() {
        for a in ["setnamaowner", "setnameowner", "setownername"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("ganti-namaowner"), "{a}");
        }
    }

    #[test]
    fn tanpa_args_menampilkan_daftar_kosong_persis_js() {
        let r = run(&[], Value::Null);
        assert_eq!(
            text_of(&r),
            "👤 *ᴏᴡɴᴇʀ ɴᴀᴍᴇ ʟɪsᴛ*\n\n\
             👑 Main: *Oktz* (6285608953677)\n\n\
             > Belum ada nama custom untuk owner tambahan\n\n\
             *Penggunaan:*\n\
             `.ganti-namaowner <nomor> <nama>`\n\
             `.ganti-namaowner main <nama>` — ganti nama owner utama"
        );
        assert!(r.mutations().is_empty(), "daftar tidak menulis apa pun");
    }

    #[test]
    fn daftar_menampilkan_nama_tersimpan_urut_dokumen() {
        let r = run(
            &[],
            serde_json::json!({"ownerNames": {"628999": "Budi", "628777": "Ani"}}),
        );
        let t = text_of(&r);
        assert!(t.contains("👤 628999: *Budi*\n👤 628777: *Ani*\n"), "{t}");
        assert!(!t.contains("Belum ada nama custom"), "{t}");
    }

    #[test]
    fn nama_utama_kosong_di_config_pakai_fallback_owner() {
        let r = run_dengan(&[], Value::Null, runtime("", &["6285608953677"]));
        assert!(text_of(&r).contains("👑 Main: *Owner* (6285608953677)"), "{}", text_of(&r));
    }

    #[test]
    fn nomor_utama_kosong_ditulis_kurung_kosong() {
        let r = run_dengan(&[], Value::Null, runtime("Oktz", &[]));
        assert!(text_of(&r).contains("👑 Main: *Oktz* ()\n"), "{}", text_of(&r));
    }

    #[test]
    fn main_tanpa_nama_menampilkan_nama_saat_ini() {
        let r = run(&["main"], Value::Null);
        assert_eq!(
            text_of(&r),
            "👤 *ɢᴀɴᴛɪ ɴᴀᴍᴀ ᴏᴡɴᴇʀ ᴜᴛᴀᴍᴀ*\n\n\
             > Nama saat ini: *Oktz*\n\n\
             `.ganti-namaowner main <nama baru>`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn main_tanpa_nama_dan_config_kosong_pakai_strip() {
        let r = run_dengan(&["MAIN"], Value::Null, runtime("", &[]));
        assert!(text_of(&r).contains("> Nama saat ini: *-*"), "{}", text_of(&r));
    }

    #[test]
    fn main_dengan_nama_berhenti_di_template_error() {
        // JS menulis config.js; penulisan itu tidak punya efek di produksi,
        // jadi jawabannya jalur `catch` JS (`ganti-namaowner.js:68-70`).
        let r = run(&["main", "Budi", "Santoso"], Value::Null);
        assert_eq!(
            text_of(&r),
            crate::common::error_template_bawaan(".", "ganti-namaowner")
        );
        assert!(r.mutations().is_empty(), "tidak ada yang bisa disimpan");
    }

    #[test]
    fn nomor_kurang_dari_sepuluh_digit_ditolak() {
        let r = run(&["62812", "Budi"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n> Nomor tidak valid\n\n\
             `.ganti-namaowner 628xxx NamaOwner`"
        );
        assert!(peta_owner(&r).is_none());
    }

    #[test]
    fn nomor_pertama_tanpa_digit_ditolak() {
        let r = run(&["+62", "Budi"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n> Nomor tidak valid\n\n\
             `.ganti-namaowner 628xxx NamaOwner`"
        );
    }

    #[test]
    fn nomor_diambil_digitnya_saja() {
        let r = run(&["+6281234567890", "Budi"], Value::Null);
        // `+` dibuang `[^0-9]`, tapi `+62` saja (argumen lain) tetap < 10 digit.
        let nomor = "+6281234567890";
        assert!(
            text_of(&r).contains("> Nama owner *6281234567890* diganti ke: *Budi*"),
            "{}",
            text_of(&r)
        );
        assert_eq!(peta_owner(&r).unwrap()["6281234567890"], "Budi");
        assert_eq!(normalisasi(nomor), "6281234567890");
    }

    #[test]
    fn nomor_tanpa_nama_baca_getownername() {
        let settings = serde_json::json!({"ownerNames": {"6281234567890": "Budi"}});
        let r = run(&["6281234567890"], settings.clone());
        assert!(
            text_of(&r).contains("6281234567890: *Budi*"),
            "{}",
            text_of(&r)
        );
        assert!(peta_owner(&r).is_none(), "tanpa nama baru tidak menyimpan");

        let r = run(&["6281234567890"], Value::Null);
        assert!(text_of(&r).contains("6281234567890: *Owner*"), "{}", text_of(&r));

        let r = run(&["6285608953677"], Value::Null);
        assert!(text_of(&r).contains("6285608953677: *Oktz*"), "{}", text_of(&r));
    }

    #[test]
    fn set_nama_menyimpan_peta_utuh() {
        let settings = serde_json::json!({"ownerNames": {"628999": "Lama"}});
        let r = run(&["6281234567890", "Budi", "Santoso"], settings);
        assert_eq!(
            text_of(&r),
            "✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Nama owner *6281234567890* diganti ke: *Budi Santoso*"
        );
        let peta = peta_owner(&r).expect("harus menyimpan ownerNames");
        assert_eq!(peta["628999"], "Lama", "entri lama tidak hilang");
        assert_eq!(peta["6281234567890"], "Budi Santoso");
    }

    #[test]
    fn nomor_sama_menormalkan_jid_dan_digit() {
        assert!(nomor_sama("628123", "628123@s.whatsapp.net"));
        assert!(nomor_sama("+62 812-3", "628123"));
        assert!(!nomor_sama("628123", "6281234"));
        assert!(!nomor_sama("", ""));
    }
}
