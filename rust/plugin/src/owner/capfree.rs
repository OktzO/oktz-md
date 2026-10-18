//! `plugins/owner/capfree.js` — kembalikan fitur menjadi gratis untuk semua.
//!
//! Loop dan balasannya identik dengan `capenergi`/`capprem`, jadi keduanya
//! dipakai lewat `capenergi::apply_cap`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::owner::capenergi::apply_cap;

fn help(prefix: &str) -> String {
    format!(
        "🆓 *SISTEM CAP FREE*\n\n\
         Sistem untuk mengembalikan status akses banyak fitur sekaligus menjadi gratis secara publik.\n\n\
         *PENGGUNAAN:*\n\
         - *{p}capfree <nama_fitur1> <nama_fitur2> ...* — Bisa banyak sekaligus\n\n\
         *CONTOH PENGGUNAAN:*\n\
         - *{p}capfree hd jpm warn*\n\n\
         *PENJELASAN:*\n\
         Masukkan satu atau lebih nama fitur yang ingin digratiskan. Pisahkan dengan spasi.",
        p = prefix
    )
}

#[plugin(
    name = "capfree",
    alias = ["capgratis", "setfree"],
    category = "owner",
    description = "Mengecap banyak fitur sekaligus menjadi gratis",
    usage = ".capfree <nama_fitur1> <nama_fitur2> ...",
    example = ".capfree hd jpm warn",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn capfree(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    apply_cap(
        &ctx,
        "capprem",
        &ctx.args,
        serde_json::json!(false),
        |success| {
            (!success.is_empty()).then(|| {
                format!(
                    "*Berhasil (FREE 🆓):*\n{}",
                    success
                        .iter()
                        .map(|f| format!("- {f}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
        },
        "_Fitur di atas (yang berhasil) sekarang sudah bebas diakses semua member._",
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
            command: "capfree".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("capfree").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("capfree").unwrap();
        assert_eq!(p.meta.name, "capfree");
        assert_eq!(p.meta.alias, ["capgratis", "setfree"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengecap banyak fitur sekaligus menjadi gratis"
        );
        assert_eq!(p.meta.usage, ".capfree <nama_fitur1> <nama_fitur2> ...");
        assert_eq!(p.meta.example, ".capfree hd jpm warn");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["capgratis", "setfree"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "capfree");
        }
    }

    #[test]
    fn no_argument_shows_help_and_writes_nothing() {
        for text in ["", "   "] {
            let r = run(text, serde_json::json!({}));
            assert!(text_of(&r).starts_with("🆓 *SISTEM CAP FREE*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_known_feature_is_written_as_false_under_its_canonical_name() {
        let r = run("baik", serde_json::json!({}));
        assert_eq!(stored(&r), serde_json::json!({ "cekbaik": false }));
        let t = text_of(&r);
        assert!(t.contains("Berhasil (FREE 🆓):"), "{t}");
        assert!(t.contains("- cekbaik"), "{t}");
        assert!(
            t.contains("_Fitur di atas (yang berhasil) sekarang sudah bebas diakses semua member._"),
            "{t}"
        );
    }

    #[test]
    fn several_features_are_freed_in_one_call() {
        let r = run("baik cekgacha cekcupu", serde_json::json!({}));
        assert_eq!(
            stored(&r),
            serde_json::json!({ "cekbaik": false, "cekgacha": false, "cekcupu": false })
        );
    }

    #[test]
    fn an_unknown_feature_only_appears_in_the_failed_list() {
        let r = run("tidakada", serde_json::json!({}));
        assert_eq!(stored(&r), serde_json::json!({}));
        assert!(text_of(&r).contains("Gagal (Tidak ditemukan):"), "{}", text_of(&r));
    }

    #[test]
    fn an_existing_premium_override_is_kept() {
        // Dua plugin ini menulis key yang sama; menimpa satu sama lain akan
        // mematikan override premium yang sudah dipasang.
        let settings = serde_json::json!({ "capprem": { "cekweit": true } });
        let __balasan = run("baik", settings);
        let s = stored(&__balasan);
        assert_eq!(s["cekweit"], serde_json::json!(true), "override premium lain hilang");
        assert_eq!(s["cekbaik"], serde_json::json!(false));
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run("baik", serde_json::json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}