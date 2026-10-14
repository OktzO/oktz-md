//! `plugins/owner/autobio.js` — setelan bio WhatsApp bot yang diperbarui
//! otomatis.
//!
//! `ponytail:` scheduler `startAutoBioChecker` (yang benar-benar memanggil
//! `sock.updateProfileStatus`) tidak ikut — plugin ini hanya menulis tiga
//! setting yang dibaca scheduler itu. Tambahkan scheduler bio ke router kalau
//! capability-nya tersedia.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

/// `db.setting("autobio_interval") || 60000`.
pub(crate) const DEFAULT_INTERVAL_MS: i64 = 60_000;
pub(crate) const MIN_INTERVAL_MS: i64 = 10_000;
const DEFAULT_TEXT: &str = "Bot aktif | 🕒 {clock} | ⏳ {runtime}";

/// `m.text?.trim()`.
fn arg(ctx: &Ctx) -> String {
    ctx.text.trim().to_string()
}

fn status_text(ctx: &Ctx) -> String {
    let enabled = crate::common::js_truthy(ctx.setting("autobio_status"));
    let text = ctx
        .setting("autobio_text")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_TEXT);
    let interval = ctx
        .setting_i64("autobio_interval")
        .unwrap_or(DEFAULT_INTERVAL_MS);
    format!(
        "📝 *AUTO BIO SETTINGS*\n\n\
         > Status: *{}*\n\
         > Interval: *{} detik*\n\
         > Teks Bio: {text}\n\n\
         *PENGGUNAAN:*\n\
         - *{p}autobio on/off* — Menyalakan/mematikan fitur\n\
         - *{p}autobio ganti_setiap <waktu>* — Mengatur interval. Contoh: `.{p}autobio ganti_setiap 30 detik` atau `1 jam`\n\
         - *{p}autobio <teks>* — Mengatur teks bio\n\n\
         *PLACEHOLDER TERSEDIA:*\n\
         - `{{clock}}` — Menampilkan jam saat ini\n\
         - `{{runtime}}` — Menampilkan lama bot menyala\n\
         - `{{botname}}` — Menampilkan nama bot dari config\n\
         - `{{version}}` — Menampilkan versi bot",
        if enabled { "Aktif ✅" } else { "Nonaktif ❌" },
        interval / 1000,
        p = ctx.prefix
    )
}

pub(crate) const BAD_FORMAT: &str =
    "❌ *Format Salah*\n\nContoh: `.autobio ganti_setiap 30 detik` atau `1 jam`";

/// `parseInt(timeStr)` + pilihan satuan. Semua cek `"d"`/`"detik"` dilakukan
/// pada string lowercase, jadi `"30 Detik"` (huruf besar) tidak kena cabang
/// detik dan jatuh ke default menit — sama seperti JS.
pub(crate) fn interval_from(time_str: &str) -> Option<i64> {
    let value = crate::common::parse_int(time_str)?;
    let has = |needle: &str| time_str.contains(needle);
    let ms = if has("d") || has("detik") {
        value * 1000
    } else if has("m") || has("menit") {
        value * 60_000
    } else if has("j") || has("jam") {
        value * 3_600_000
    } else {
        value * 60_000
    };
    Some(ms)
}

#[plugin(
    name = "autobio",
    alias = ["autostatus"],
    category = "owner",
    description = "Mengatur pembaruan bio WhatsApp bot secara otomatis",
    usage = ".autobio on/off\n.autobio <teks>",
    example = ".autobio on\n.autobio Bot by Owner | 🕒 {clock} | ⏳ {runtime}",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autobio(ctx: Ctx) -> Reply {
    let arg = arg(&ctx);
    if arg.is_empty() {
        return Reply::text(status_text(&ctx));
    }

    let option = arg.to_lowercase();
    if option == "on" || option == "off" {
        let on = option == "on";
        return Reply::text(format!(
            "{} *AUTO BIO {}*\n\nBio WhatsApp bot {}",
            if on { "✅" } else { "❌" },
            if on { "DIAKTIFKAN" } else { "DINONAKTIFKAN" },
            if on {
                "sekarang akan diperbarui secara otomatis setiap menit."
            } else {
                "tidak akan di-update lagi."
            }
        ))
        .with_reaction(if on { "✅" } else { "❌" })
        .with_mutation(Mutation::SetSetting {
            key: "autobio_status".into(),
            value: json!(on),
        });
    }

    if option.starts_with("ganti_setiap") {
        // `arg.replace(/ganti_setiap/i, "").trim().toLowerCase()` — lowercase
        // SEBELUM dicek.contains, jadi huruf besar di satuan ikut diturunkan.
        let time_str = arg
            .to_lowercase()
            .replacen("ganti_setiap", "", 1)
            .trim()
            .to_string();
        if time_str.is_empty() {
            return Reply::text(BAD_FORMAT);
        }
        let ms = match interval_from(&time_str) {
            Some(ms) => ms,
            None => {
                return Reply::text(
                    "❌ *Format Salah*\n\nMasukkan angka yang valid. Contoh: `.autobio ganti_setiap 30 detik`",
                );
            }
        };
        if ms < MIN_INTERVAL_MS {
            return Reply::text(
                "❌ *Gagal*\n\nInterval minimal adalah 10 detik agar tidak terkena spam dari server WhatsApp.",
            );
        }
        // `timeStr.replace(/[0-9\s]/g, "")` — sisa satuan yang ditampilkan.
        let unit: String = time_str
            .chars()
            .filter(|c| !c.is_ascii_digit() && !c.is_whitespace())
            .collect();
        let value = crate::common::parse_int(&time_str).unwrap_or(0);
        return Reply::text(format!(
            "✅ *Interval Diubah*\n\nBio akan di-update otomatis setiap *{value} {unit}* (atau {sec} detik).",
            sec = ms / 1000
        ))
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "autobio_interval".into(),
            value: json!(ms),
        });
    }

    Reply::text(format!(
        "✅ *TEKS BIO DIUBAH*\n\nFormat autobio yang baru telah disimpan:\n> {arg}"
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetSetting {
        key: "autobio_text".into(),
        value: json!(arg),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autobio".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autobio").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> (String, Value) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => (key.clone(), value.clone()),
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autobio").unwrap();
        assert_eq!(p.meta.name, "autobio");
        assert_eq!(p.meta.alias, ["autostatus"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengatur pembaruan bio WhatsApp bot secara otomatis"
        );
        assert_eq!(p.meta.usage, ".autobio on/off\n.autobio <teks>");
        assert_eq!(
            p.meta.example,
            ".autobio on\n.autobio Bot by Owner | 🕒 {clock} | ⏳ {runtime}"
        );
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("autostatus").unwrap().meta.name, "autobio");
    }

    #[test]
    fn no_argument_shows_status_and_writes_nothing() {
        let r = run("", json!({}));
        assert!(text_of(&r).starts_with("📝 *AUTO BIO SETTINGS*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
        // Default: status mati, interval 60 detik, teks bawaan.
        assert!(text_of(&r).contains("Nonaktif ❌"), "{}", text_of(&r));
        assert!(text_of(&r).contains("60 detik"), "{}", text_of(&r));
        assert!(text_of(&r).contains(DEFAULT_TEXT), "{}", text_of(&r));
    }

    #[test]
    fn status_reflects_the_stored_values() {
        let r = run(
            "",
            json!({"autobio_status": true, "autobio_interval": 30000, "autobio_text": "hai"}),
        );
        let t = text_of(&r);
        assert!(t.contains("Aktif ✅"), "{t}");
        assert!(t.contains("30 detik"), "{t}");
        assert!(t.contains("Teks Bio: hai"), "{t}");
    }

    #[test]
    fn on_and_off_write_their_own_flag_and_react_to_match() {
        let (k, v) = stored(&run("on", json!({})));
        assert_eq!(k, "autobio_status");
        assert_eq!(v, json!(true));
        let (k, v) = stored(&run("off", json!({"autobio_status": true})));
        assert_eq!(k, "autobio_status");
        assert_eq!(v, json!(false));
        match run("off", json!({})).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "❌"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn a_unitless_number_defaults_to_minutes() {
        let (k, v) = stored(&run("ganti_setiap 5", json!({})));
        assert_eq!(k, "autobio_interval");
        assert_eq!(v, json!(300_000));
    }

    #[test]
    fn each_unit_maps_to_its_millisecond_count() {
        assert_eq!(interval_from("30 detik"), Some(30_000));
        assert_eq!(interval_from("2 menit"), Some(120_000));
        // "1d" mengandung huruf "d", jadi cabang HARI yang menang lebih dulu
        // dan hasilnya `value * 1000` — bukan 86.400.000. Persis `if/else`
        // di `autobio.js:70-75`; bukan bug port.
        assert_eq!(interval_from("1d"), Some(1_000), "huruf d tidak di-anchor");
        // "1 hari" tidak mengandung d/menit/jam, jadi jatuh ke default menit
        // — `else ms = value * 60000` di JS.
        assert_eq!(interval_from("1 hari"), Some(60_000), "default menit");
        // "1 jam" mengandung huruf "m" (dari "jam" sendiri), jadi cabang
        // menit menang lebih dulu persis seperti `includes("m")` di JS.
        assert_eq!(interval_from("1 jam"), Some(60_000), "huruf m di 'jam'");
        assert_eq!(interval_from("abc"), None, "parseInt NaN");
    }

    #[test]
    fn an_interval_below_ten_seconds_is_refused() {
        // 5.000 ms masih di bawah batas 10.000, walau nilainya > 0.
        let r = run("ganti_setiap 5 detik", json!({}));
        assert!(text_of(&r).starts_with("❌ *Gagal*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_bare_unit_is_rejected_before_parsing() {
        for text in ["ganti_setiap", "ganti_setiap   "] {
            let r = run(text.clone(), json!({}));
            assert_eq!(text_of(&r), BAD_FORMAT, "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn free_text_is_stored_verbatim_as_the_bio_template() {
        let (k, v) = stored(&run("Halo {botname} v{version}", json!({})));
        assert_eq!(k, "autobio_text");
        assert_eq!(v, json!("Halo {botname} v{version}"));
        assert!(text_of(&run("Halo", json!({}))).contains("> Halo"));
    }
}