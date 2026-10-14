//! `plugins/owner/capprem.js` — jadikan fitur premium-only.
//!
//! Loop dan balasannya identik dengan `capenergi`/`capfree`, jadi keduanya
//! dipakai lewat `capenergi::apply_cap`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::json;

use crate::owner::capenergi::apply_cap;

fn help(prefix: &str) -> String {
    format!(
        "💎 *SISTEM CAP PREMIUM*\n\n\
         Sistem eksklusif untuk mengubah status akses banyak fitur sekaligus menjadi Premium.\n\n\
         *PENGGUNAAN:*\n\
         - *{p}capprem <nama_fitur1> <nama_fitur2> ...* — Bisa banyak sekaligus\n\n\
         *CONTOH PENGGUNAAN:*\n\
         - *{p}capprem hd jpm warn*\n\n\
         *PENJELASAN:*\n\
         Masukkan satu atau lebih nama fitur yang ingin dijadikan Premium. Pisahkan dengan spasi.",
        p = prefix
    )
}

#[plugin(
    name = "capprem",
    alias = ["cappremium"],
    category = "owner",
    description = "Mengecap banyak fitur sekaligus menjadi premium",
    usage = ".capprem <nama_fitur1> <nama_fitur2> ...",
    example = ".capprem hd jpm warn",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn capprem(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    apply_cap(
        &ctx,
        "capprem",
        &ctx.args,
        json!(true),
        |success| {
            (!success.is_empty()).then(|| {
                format!(
                    "*Berhasil (PREMIUM 💎):*\n{}",
                    success
                        .iter()
                        .map(|f| format!("- {f}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
        },
        "_Fitur di atas (yang berhasil) sekarang hanya bisa diakses oleh member Premium._",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Mutation;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "capprem".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("capprem").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "capprem");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("capprem").unwrap();
        assert_eq!(p.meta.name, "capprem");
        assert_eq!(p.meta.alias, ["cappremium"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengecap banyak fitur sekaligus menjadi premium"
        );
        assert_eq!(p.meta.usage, ".capprem <nama_fitur1> <nama_fitur2> ...");
        assert_eq!(p.meta.example, ".capprem hd jpm warn");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("cappremium").unwrap().meta.name, "capprem");
    }

    #[test]
    fn no_argument_shows_help_and_writes_nothing() {
        for text in ["", "   "] {
            let r = run(text, json!({}));
            assert!(text_of(&r).starts_with("💎 *SISTEM CAP PREMIUM*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_known_feature_is_written_as_true_under_its_canonical_name() {
        let r = run("baik", json!({}));
        assert_eq!(stored(&r), json!({ "cekbaik": true }));
        let t = text_of(&r);
        assert!(t.contains("Berhasil (PREMIUM 💎):"), "{t}");
        assert!(t.contains("- cekbaik"), "{t}");
        assert!(
            t.contains("_Fitur di atas (yang berhasil) sekarang hanya bisa diakses oleh member Premium._"),
            "{t}"
        );
    }

    #[test]
    fn several_features_are_marked_premium_in_one_call() {
        let r = run("baik cekgacha", json!({}));
        assert_eq!(
            stored(&r),
            json!({ "cekbaik": true, "cekgacha": true })
        );
    }

    #[test]
    fn an_unknown_feature_only_appears_in_the_failed_list() {
        let r = run("tidakada", json!({}));
        assert_eq!(stored(&r), json!({}));
        assert!(text_of(&r).contains("Gagal (Tidak ditemukan):"), "{}", text_of(&r));
    }

    #[test]
    fn an_existing_free_override_is_kept() {
        // `capfree` menulis key yang sama; menimpanya di sini akan membatalkan
        // status gratis yang sudah dipasang.
        let settings = json!({ "capprem": { "cekweit": false } });
        let __balasan = run("baik", settings);
        let s = stored(&__balasan);
        assert_eq!(s["cekweit"], json!(false), "override gratis lain hilang");
        assert_eq!(s["cekbaik"], json!(true));
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run("baik", json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}