//! `plugins/owner/setaudioallmenu.js` — pilih varian audio All Menu (1-4).
//!
//! `db.setting("allmenuAudioStyle") || 1` dipakai untuk MENAMPILKAN varian
//! yang aktif; nilai `0`/`false` di store ikut jadi `1` karena `||` di JS.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const STORE: &str = "allmenuAudioStyle";

const MIN: i64 = 1;
const MAX: i64 = 4;

fn info(prefix: &str, current: i64) -> String {
    format!(
        "⚠️ *PENGATURAN AUDIO ALL MENU*\n\n\
         Sistem manajemen gaya audio khusus untuk tampilan All Menu.\n\n\
         *PENGGUNAAN:*\n\
         • *{prefix}setaudioallmenu 1* — PTT Voice Note dengan reply pesan asli\n\
         • *{prefix}setaudioallmenu 2* — PTT Voice Note dengan reply fake polling\n\
         • *{prefix}setaudioallmenu 3* — Audio musik biasa dengan reply fake text\n\
         • *{prefix}setaudioallmenu 4* — Audio musik biasa dengan reply fake troli order\n\n\
         *PENJELASAN VARIAN:*\n\
         - *Varian 1 & 2* akan secara otomatis mengkonversi file MP3 menjadi Opus (Voice Note) murni menggunakan ffmpeg, sehingga terlihat lebih natural layaknya rekaman suara asli.\n\
         - *Varian 3 & 4* mengirimkan file dalam format MP3 biasa tanpa konversi, namun menggunakan *Fake Quoted* yang terlihat elegan dan keren di layar obrolan.\n\n\
         Saat ini All Menu menggunakan varian: *{current}*"
    )
}

fn bad(prefix: &str) -> String {
    format!(
        "❌ *GAGAL*\n\nPilihan varian audio harus berupa angka 1 sampai 4.\n\
         Contoh: *{prefix}setaudioallmenu 2*"
    )
}

fn ok(prefix: &str, style: i64) -> String {
    format!(
        "✅ *BERHASIL*\n\nGaya audio All Menu telah sukses diubah menjadi *Varian {style}*. \
         Silakan tes dengan mengetik *{prefix}allmenu*."
    )
}

/// `db.setting(k) || 1` — nilai falsy (tidak ada, `null`, `0`) jadi `1`.
fn current(ctx: &Ctx) -> i64 {
    match ctx.setting_i64(STORE) {
        Some(n) if n != 0 => n,
        _ => 1,
    }
}

#[plugin(
    name = "setaudioallmenu",
    alias = ["setaudioam", "audioallmenu"],
    category = "owner",
    description = "Mengatur gaya audio untuk All Menu",
    usage = ".setaudioallmenu <1-4>",
    example = ".setaudioallmenu 1",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn setaudioallmenu(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(info(&ctx.prefix, current(&ctx)));
    }

    // `parseInt`, bukan parse-angka-penuh: "2x" sah dan berarti varian 2.
    let Some(style) = crate::common::parse_int(text) else {
        return Reply::text(bad(&ctx.prefix));
    };
    if !(MIN..=MAX).contains(&style) {
        return Reply::text(bad(&ctx.prefix));
    }

    Reply::text(ok(&ctx.prefix, style))
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: STORE.into(),
            value: serde_json::json!(style),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setaudioallmenu".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("setaudioallmenu").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn written(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, STORE);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setaudioallmenu").unwrap();
        assert_eq!(p.meta.name, "setaudioallmenu");
        assert_eq!(p.meta.alias, ["setaudioam", "audioallmenu"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Mengatur gaya audio untuk All Menu");
        assert_eq!(p.meta.usage, ".setaudioallmenu <1-4>");
        assert_eq!(p.meta.example, ".setaudioallmenu 1");
        assert_eq!(p.meta.cooldown, 0, "JS cooldown: 0");
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("setaudioam").unwrap().meta.name,
            "setaudioallmenu"
        );
        assert_eq!(
            oktz_plugin_api::find("audioallmenu").unwrap().meta.name,
            "setaudioallmenu"
        );
    }

    #[test]
    fn an_empty_argument_shows_the_legend_with_the_stored_style() {
        let __balasan = run("", json!({"allmenuAudioStyle": 3}));
        let t = body(&__balasan);
        assert!(t.starts_with("⚠️ *PENGATURAN AUDIO ALL MENU*"), "{t}");
        assert!(t.contains("Saat ini All Menu menggunakan varian: *3*"), "{t}");
        assert!(run("", json!({})).mutations().is_empty());
    }

    #[test]
    fn a_falsy_stored_style_is_displayed_as_one() {
        // `db.setting(...) || 1` — `0` dan `null` jadi 1.
        for s in [json!({"allmenuAudioStyle": 0}), json!({"allmenuAudioStyle": null})] {
            assert!(
                body(&run("", s.clone())).contains("menggunakan varian: *1*"),
                "{s}"
            );
        }
    }

    #[test]
    fn a_style_in_range_is_stored_as_a_number() {
        for n in 1..=4 {
            let r = run(&n.to_string(), json!({}));
            let t = body(&r);
            assert!(t.starts_with("✅ *BERHASIL*"), "varian {n}: {t}");
            assert!(t.contains(&format!("*Varian {n}*")), "{t}");
            assert!(t.contains("mengetik *.allmenu*."), "{t}");
            assert_eq!(written(&r), json!(n));
        }
    }

    #[test]
    fn a_style_out_of_range_is_rejected_without_writing() {
        for input in ["0", "5", "-1", "99"] {
            let r = run(input.clone(), json!({}));
            assert!(body(&r).starts_with("❌ *GAGAL*"), "input {input}: {}", body(&r));
            assert!(
                body(&r).contains("harus berupa angka 1 sampai 4"),
                "input {input}"
            );
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn non_numeric_input_is_rejected() {
        // `parseInt` JS: hanya yang benar-benar bukan angka yang ditolak —
        // `"1-4"` bukan salah satunya, dia mulai dari `1`.
        for input in ["abc", "x2", "--3", "e4", "+"] {
            let r = run(input.clone(), json!({}));
            assert!(body(&r).starts_with("❌ *GAGAL*"), "input {input}: {}", body(&r));
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn a_leading_number_wins_like_parse_int_does() {
        // `parseInt("2 apa pun")` = 2 di JS, bukan NaN.
        let r = run("2 apa pun", json!({}));
        assert!(body(&r).contains("*Varian 2*"), "{}", body(&r));
        assert_eq!(written(&r), json!(2));
    }

    #[test]
    fn parse_int_keeps_only_the_digits_before_the_first_rubbish() {
        // `parseInt("1-4")` = 1 dan `parseInt("3px")` = 3 — keduanya sah,
        // meski kelihatannya bukan angka bulat utuh.
        for (input, style) in [("1-4", 1), ("3px", 3), ("  4  ", 4)] {
            let r = run(input.clone(), json!({}));
            assert!(
                body(&r).contains(&format!("*Varian {style}*")),
                "input {input:?}: {}",
                body(&r)
            );
            assert_eq!(written(&r), json!(style), "input {input:?}");
        }
    }

    #[test]
    fn a_successful_write_is_paired_with_the_success_reaction() {
        let r = run("4", json!({}));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}
