//! `plugins/owner/capenergi.js` — override biaya energi beberapa plugin.
//!
//! `pub(crate) fn apply_cap` di sini juga dipakai `capfree`/`capprem`: ketiga
//! plugin JS punya tubuh yang sama persis (loop argumen → `getPlugin` → tulis
//! override → balas daftar berhasil/gagal), hanya nama setting, nilai yang
//! ditulis, dan teksnya yang berbeda. Helper-nya diletakkan di file plugin
//! sendiri, bukan `common.rs`, karena `common.rs` milik batch lain.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::parse_int;

fn help(prefix: &str) -> String {
    format!(
        "⚙️ *SISTEM CAP ENERGI*\n\n\
         Sistem untuk mengubah secara dinamis jumlah energi yang dipotong untuk banyak fitur sekaligus.\n\n\
         *PENGGUNAAN:*\n\
         - *{p}capenergi <nama_fitur1> <nama_fitur2> ... <jumlah>*\n\n\
         *CONTOH PENGGUNAAN:*\n\
         - *{p}capenergi hd jpm 5* (Fitur HD & JPM memotong 5 energi)\n\
         - *{p}capenergi hd 0* (Fitur HD menjadi gratis energi)\n\n\
         *PENJELASAN:*\n\
         1. Masukkan satu atau banyak nama fitur yang ingin diubah.\n\
         2. Di akhiran (kata terakhir) harus berupa ANGKA (jumlah potongan energi).",
        p = prefix
    )
}

pub(crate) const BAD_COST: &str =
    "❌ *GAGAL*\n\nJumlah energi (di argumen paling akhir) harus berupa angka 0 atau lebih.";

/// `getPlugin(cmd)` → nama kanonik plugin. `None` kalau tidak dikenal.
fn canonical(cmd: &str) -> Option<String> {
    oktz_plugin_api::find(&cmd.to_lowercase()).map(|p| p.meta.name.clone())
}

/// Pisahkan argumen jadi (nama-nama fitur, biaya). Argumen terakhir selalu
/// biayanya, jadi satu fitur saja tetap butuh dua argumen.
pub(crate) fn split_args(args: &[String]) -> Option<(&[String], i64)> {
    let (last, commands) = args.split_last()?;
    let cost = parse_int(last)?;
    (cost >= 0).then_some((commands, cost))
}

/// Tulung yang sama untuk `capenergi`/`capfree`/`capprem`: setiap `cmd`
/// dipecah jadi (berhasil, gagal), override lama dipertahankan, dan reply
/// berisi keduanya.
pub(crate) fn apply_cap(
    ctx: &Ctx,
    setting_key: &str,
    commands: &[String],
    value: serde_json::Value,
    success_block: impl Fn(&[String]) -> Option<String>,
    footer: &str,
) -> Reply {
    let mut overrides = ctx
        .setting(setting_key)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let (mut success, mut failed) = (Vec::new(), Vec::new());
    for cmd in commands {
        match canonical(cmd) {
            Some(name) => {
                overrides.insert(name.clone(), value.clone());
                success.push(name);
            }
            None => failed.push(cmd.to_lowercase()),
        }
    }

    let mut msg = String::from("✅ *STATUS BERHASIL DIUBAH*\n\n");
    if let Some(block) = success_block(&success) {
        msg.push_str(&block);
        msg.push_str("\n\n");
    }
    if !failed.is_empty() {
        msg.push_str("*Gagal (Tidak ditemukan):*\n");
        for f in &failed {
            msg.push_str(&format!("- {f}\n"));
        }
        msg.push('\n');
    }
    msg.push_str(footer);

    Reply::text(msg.trim().to_string())
        .with_reaction("🕕")
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: setting_key.into(),
            value: serde_json::Value::Object(overrides),
        })
}

/// Blok "berhasil" untuk `capenergi`: kosong kalau tidak ada yang berubah.
fn success_block(cost: i64) -> impl Fn(&[String]) -> Option<String> {
    move |success: &[String]| {
        (!success.is_empty()).then(|| {
            format!(
                "*Berhasil diubah jadi {cost} Energi:*\n{}",
                success
                    .iter()
                    .map(|f| format!("- {f}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
    }
}

#[plugin(
    name = "capenergi",
    alias = ["setenergi"],
    category = "owner",
    description = "Mengubah potongan energi banyak fitur sekaligus",
    usage = ".capenergi <nama_fitur1> <nama_fitur2> ... <jumlah>",
    example = ".capenergi hd fakedev 5",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn capenergi(ctx: Ctx) -> Reply {
    if ctx.args.len() < 2 {
        return Reply::text(help(&ctx.prefix));
    }
    let (commands, cost) = match split_args(&ctx.args) {
        Some(v) => v,
        None => return Reply::text(BAD_COST),
    };

    apply_cap(
        &ctx,
        "capenergi",
        commands,
        json!(cost),
        success_block(cost),
        "_Pengaturan tersimpan ke dalam database dan langsung berlaku._",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "capenergi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("capenergi").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "capenergi");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("capenergi").unwrap();
        assert_eq!(p.meta.name, "capenergi");
        assert_eq!(p.meta.alias, ["setenergi"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengubah potongan energi banyak fitur sekaligus"
        );
        assert_eq!(
            p.meta.usage,
            ".capenergi <nama_fitur1> <nama_fitur2> ... <jumlah>"
        );
        assert_eq!(p.meta.example, ".capenergi hd fakedev 5");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("setenergi").unwrap().meta.name, "capenergi");
    }

    #[test]
    fn fewer_than_two_arguments_shows_help_and_writes_nothing() {
        for text in ["", "5", "   "] {
            let r = run(text, json!({}));
            assert!(text_of(&r).starts_with("⚙️ *SISTEM CAP ENERGI*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_non_numeric_or_negative_cost_is_refused() {
        for text in ["hd abc", "hd -1"] {
            let r = run(text, json!({}));
            assert_eq!(text_of(&r), BAD_COST, "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_known_feature_is_written_under_its_canonical_name() {
        // Alias `baik` → plugin `cekbaik`; yang ditulis bukan aliasnya.
        let r = run("baik 5", json!({}));
        assert_eq!(stored(&r)["cekbaik"], json!(5));
        let t = text_of(&r);
        assert!(t.contains("*Berhasil diubah jadi 5 Energi:*"), "{t}");
        assert!(t.contains("- cekbaik"), "{t}");
    }

    #[test]
    fn an_unknown_feature_lands_in_the_failed_list_and_writes_nothing_for_it() {
        let r = run("tidakada 3", json!({}));
        assert_eq!(stored(&r), json!({}));
        let t = text_of(&r);
        assert!(t.contains("*Gagal (Tidak ditemukan):*"), "{t}");
        assert!(t.contains("- tidakada"), "{t}");
    }

    #[test]
    fn a_zero_cost_is_valid_and_makes_the_feature_free() {
        let r = run("baik 0", json!({}));
        assert_eq!(stored(&r)["cekbaik"], json!(0));
    }

    #[test]
    fn several_features_share_one_cost() {
        let r = run("baik cekgacha 2", json!({}));
        let s = stored(&r);
        assert_eq!(s["cekbaik"], json!(2));
        assert_eq!(s["cekgacha"], json!(2));
    }

    #[test]
    fn existing_overrides_are_preserved() {
        let settings = json!({ "capenergi": { "cekbaik": 9, "cekgacha": 2 } });
        let s = stored(&run("cekcantik 4", settings));
        assert_eq!(s["cekbaik"], json!(9), "override lain tidak hilang");
        assert_eq!(s["cekgacha"], json!(2));
        assert_eq!(s["cekcantik"], json!(4));
    }

    #[test]
    fn a_non_object_setting_is_replaced_rather_than_merged() {
        // `db.setting("capenergi") || {}` tidak dijamin objek di JS; Rust
        // memakai objek kosong supaya tidak menimpa data lain diam-diam.
        let r = run("baik 5", json!({ "capenergi": 5 }));
        assert_eq!(stored(&r), json!({ "cekbaik": 5 }));
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run("baik 5", json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn split_args_peels_the_cost_off_the_last_argument() {
        let args: Vec<String> = ["a", "b", "5"].map(String::from).into_iter().collect();
        let (names, cost) = split_args(&args).unwrap();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(cost, 5);

        let bad: Vec<String> = ["a", "-1"].map(String::from).into_iter().collect();
        assert_eq!(split_args(&bad), None);
    }
}