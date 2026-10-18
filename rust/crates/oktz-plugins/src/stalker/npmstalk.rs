//! `plugins/stalker/npmstalk.js` — stalk akun NPM lewat Firefly atau Nexray.
//!
//! Data diambil dengan `fetchNpmProfile` (`stalker-fallback.js:211-262`) yang
//! mencoba Firefly dulu (butuh `APIKEY_FIREFLY`), lalu Nexray sebagai cadangan
//! (tanpa key). Dua bentuk respons dinormalisasi ke shape yang sama sebelum
//! caption dirakit.
//!
//! `ponytail:` `m.react('🔍')` terjadi sebelum permintaan, jadi pada cabang
//! gagal reaksi terakhir yang ikut adalah `☢`. `Reply::Reaction` hanya bisa
//! membawa satu emoji.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `NEXRAY_BASES[0]` — hanya basis pertama yang dipakai `fetchNpmProfile`.
const NEXRAY: &str = "https://api.nexray.eu.cc";
const FIREFLY: &str = "https://firefly.maiku.my.id";

/// `npmstalk.js:26-29` — kartu pakai saat `m.args[0]` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📦 *ɴᴘᴍ sᴛᴀʟᴋ*\n\n> Masukkan username NPM\n\n`Contoh: {prefix}npmstalk hanya_zann`"
    )
}

/// `shortNum(num)` (`npmstalk.js:17-24`).
///
/// `parseInt` tanpa radix memotong di karakter non-digit; `toFixed(1)` lalu
/// `.replace('.0', '')` menghapus hanya kemunculan PERTAMA, jadi `10.05` tetap
/// `10.05` setelah `.0` di awal string tidak ada.
pub(crate) fn short_num(value: &Value) -> String {
    if !crate::common::js_truthy_value(value) {
        return "0".to_string();
    }
    let n = parse_int_js(value);
    if n >= 1_000_000_000.0 {
        return trim_b(n / 1_000_000_000.0) + "B";
    }
    if n >= 1_000_000.0 {
        return trim_b(n / 1_000_000.0) + "M";
    }
    if n >= 1_000.0 {
        return trim_b(n / 1_000.0) + "K";
    }
    n.to_string()
}

/// `(n / d).toFixed(1).replace(".0", "")` — hanya kemunculan PERTAMA `.0`
/// yang hilang, jadi `"10.05"` tetap `"10.05"`.
fn trim_b(n: f64) -> String {
    let s = format!("{n:.1}");
    s.replacen(".0", "", 1)
}

/// `parseInt(value)` tanpa radix: ambil digit di depan, tanpa tanda dan eksponen.
fn parse_int_js(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n
            .as_f64()
            .map(|f| f.trunc())
            .unwrap_or(f64::NAN),
        Value::String(s) => {
            let s = s.trim_start();
            let mut digits = String::new();
            for (i, c) in s.chars().enumerate() {
                // Tanda hanya di posisi pertama; setelah itu hanya digit.
                if c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')) {
                    digits.push(c);
                } else {
                    break;
                }
            }
            digits.parse::<f64>().unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}

/// `d.name || '-'` dan sejenisnya — nilai falsy jadi tanda hubung.
fn or_dash(v: &Value) -> String {
    let s = js_str(v);
    if s.is_empty() || s == "null" {
        "-".to_string()
    } else {
        s
    }
}

/// `String(v)` untuk nilai JSON di template literal.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// Bentuk ternormalisasi dari `fetchNpmProfile` (`stalker-fallback.js:224-233`).
#[derive(Debug, Clone)]
pub(crate) struct Profil {
    pub username: String,
    pub name: String,
    pub email: String,
    pub description: String,
    pub profile: String,
    pub avatar: String,
    pub packages: Vec<Value>,
}

/// Firefly (`stalker-fallback.js:213-234`). `description` menyalin `d.name`,
/// bukan `d.description` — itu yang ditulis JS dan tidak dibetulkan di sini.
pub(crate) fn dari_firefly(d: &Value) -> Profil {
    Profil {
        username: js_str(&d["username"]),
        name: js_str(&d["name"]),
        email: js_str(&d["email"]),
        description: js_str(&d["name"]),
        profile: js_str(&d["profile"]),
        avatar: js_str(&d["avatar"]),
        packages: d
            .get("packages")
            .filter(|v| crate::common::js_truthy_value(v))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    }
}

/// Nexray (`stalker-fallback.js:237-258`). `avatar` selalu kosong di sini,
/// jadi caption dikirim tanpa gambar.
pub(crate) fn dari_nexray(r: &Value) -> Profil {
    let nama = js_str(&r["name"]);
    Profil {
        username: nama.clone(),
        name: or_dash(&r["author"]["name"]).replace("-", &nama),
        email: js_str(&r["author"]["email"]),
        description: js_str(&r["description"]),
        profile: if crate::common::js_truthy_value(&r["homepage"]) {
            js_str(&r["homepage"])
        } else {
            format!("https://www.npmjs.com/~{nama}")
        },
        avatar: String::new(),
        packages: Vec::new(),
    }
}

/// `npmstalk.js:41-55` — caption. Maksimal lima package, masing-masing tiga
/// baris dan satu baris kosong.
pub(crate) fn caption(p: &Profil) -> String {
    let mut txt = format!(
        "📦 *ɴᴘᴍ sᴛᴀʟᴋ*\n\n👤 *Username:* {}\n📛 *Nama:* {}\n📧 *Email:* {}\n\n📝 *Deskripsi:*\n{}\n\n🔗 {}\n\n",
        p.username,
        or_dash(&Value::String(p.name.clone())),
        or_dash(&Value::String(p.email.clone())),
        or_dash(&Value::String(p.description.clone())),
        p.profile,
    );
    if !p.packages.is_empty() {
        txt.push_str("*Daftar Package:*\n");
        for pkg in p.packages.iter().take(5) {
            txt.push_str(&format!("> 📦 *{}* (v{})\n", js_str(&pkg["name"]), js_str(&pkg["version"])));
            txt.push_str(&format!(
                "> 📉 {} dl/month\n",
                short_num(&pkg["downloads_monthly"])
            ));
            txt.push_str(&format!("> 📝 {}\n\n", js_str(&pkg["description"])));
        }
    }
    txt
}

#[plugin(
    name = "npmstalk",
    alias = ["stalknpm", "npms"],
    category = "stalker",
    description = "Stalk akun NPM (Node Package Manager)",
    usage = ".npmstalk <username>",
    example = ".npmstalk hanya_zann",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn npmstalk(ctx: Ctx) -> Reply {
    let username = ctx.args.first().cloned().unwrap_or_default();
    if username.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let firefly_key = ctx.config().api_keys.firefly.clone();
    let mut errors: Vec<String> = Vec::new();
    let mut profil: Option<Profil> = None;

    // Firefly dulu, hanya kalau kuncinya ada.
    if !firefly_key.is_empty() {
        match ctx.http.get(&format!(
            "{FIREFLY}/api/stalk-npm?apikey={firefly_key}&username={}",
            encode_uri_component(&username)
        )) {
            Ok(res) => {
                let body = res.json().unwrap_or(Value::Null);
                let ok = body.get("status").is_some_and(crate::common::js_truthy_value)
                    && body.get("data").is_some_and(|v| !v.is_null());
                if ok {
                    profil = Some(dari_firefly(&body["data"]));
                } else {
                    let pesan = match body.get("message") {
                        Some(m) if crate::common::js_truthy_value(m) => js_str(m),
                        _ => "tidak ditemukan".to_string(),
                    };
                    errors.push(pesan);
                }
            }
            Err(e) => errors.push(e.message),
        }
    }

    // Nexray cadangan, tanpa key.
    if profil.is_none() {
        match ctx.http.get(&format!(
            "{NEXRAY}/stalker/npmjs?name={}",
            encode_uri_component(&username)
        )) {
            Ok(res) => {
                let body = res.json().unwrap_or(Value::Null);
                let ok = body.get("status").is_some_and(crate::common::js_truthy_value)
                    && body.get("result").is_some_and(|v| !v.is_null());
                if ok {
                    profil = Some(dari_nexray(&body["result"]));
                } else {
                    let pesan = match body.get("error") {
                        Some(e) if crate::common::js_truthy_value(e) => js_str(e),
                        _ => "tidak ditemukan".to_string(),
                    };
                    errors.push(pesan);
                }
            }
            Err(e) => errors.push(e.message),
        }
    }

    let Some(profil) = profil else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let teks = caption(&profil);
    if profil.avatar.is_empty() {
        Reply::text(teks).with_reaction("✅")
    } else {
        Reply::image(MediaSource::url(profil.avatar), teks).with_reaction("✅")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// Stub berbasis URL: balasan berurutan untuk setiap pemanggilan.
    #[derive(Debug, Default)]
    struct Stub {
        balasan: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn baru(balasan: &[&str]) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(balasan.iter().map(|s| s.to_string()).collect()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn gagal() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(Vec::new()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut b = self.balasan.lock().unwrap();
            if b.is_empty() {
                return Err(HttpError {
                    message: "offline".into(),
                });
            }
            let body = if b.len() == 1 {
                b[0].clone()
            } else {
                b.remove(0)
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn ctx(http: Arc<Stub>, text: &str) -> Ctx {
        let mut args = text.split_whitespace();
        Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "npmstalk".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    firefly: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn tanpa_kunci(http: Arc<Stub>, text: &str) -> Ctx {
        Ctx {
            config: Arc::new(oktz_plugin_api::config::Runtime::default()),
            ..ctx(http, text)
        }
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_username_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = npmstalk(tanpa_kunci(Stub::gagal(), ""));
        assert_eq!(
            r.without_mutations().text_body(),
            Some("📦 *ɴᴘᴍ sᴛᴀʟᴋ*\n\n> Masukkan username NPM\n\n`Contoh: .npmstalk hanya_zann`")
        );
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_langsung_lompat_ke_nexray() {
        let http = Stub::baru(&[r#"{"status":true,"result":{"name":"zann"}}"#]);
        npmstalk(tanpa_kunci(http.clone(), "zann"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "firefly tidak dipanggil: {seen:?}");
        assert!(seen[0].starts_with("https://api.nexray.eu.cc/stalker/npmjs?name=zann"));
    }

    #[test]
    fn firefly_dipakai_dulu_saat_kunci_ada_dan_tidak_perlu_nexray() {
        let http = Stub::baru(&[r#"{"status":true,"data":{"username":"zann","name":"Zann","email":"z@x.io","description":"bio","profile":"https://npmjs.com/~zann","avatar":"https://cdn/a.png","packages":[{"name":"pkg","version":"1.2.3","downloads_monthly":2500,"description":"desc"}]}}"#]);
        let r = npmstalk(ctx(http.clone(), "zann"));
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("apikey=KUNCI"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Image { media, caption, .. } => {
                    assert_eq!(
                        media.as_ref().unwrap(),
                        &MediaSource::Url {
                            url: "https://cdn/a.png".to_string(),
                            mimetype: None,
                        }
                    );
                    assert!(caption.contains("👤 *Username:* zann"));
                    assert!(caption.contains("> 📦 *pkg* (v1.2.3)"));
                    assert!(caption.contains("> 📉 2.5K dl/month"));
                }
                lain => panic!("harus image, dapat {lain:?}"),
            },
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn firefly_gagal_lalu_nexray_menyelamkan() {
        let http = Stub::baru(&[
            r#"{"status":false,"message":"kunci firefly"}"#,
            r#"{"status":true,"result":{"name":"zann","author":{"name":"Zann","email":"z@x.io"},"description":"bio"}}"#,
        ]);
        let r = npmstalk(ctx(http.clone(), "zann"));
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2, "dua provider dicoba");
        assert!(seen[0].contains("firefly"));
        assert!(seen[1].contains("nexray"));
        // Nexray selalu tanpa avatar, jadi jawabannya teks biasa.
        assert!(r.without_mutations().text_body().is_some());
    }

    #[test]
    fn semua_provider_gagal_membungkus_pesan_error_dan_reaksi_sil() {
        let http = Stub::gagal();
        let r = npmstalk(ctx(http.clone(), "zann"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some("☢ Kayaknya command `.npmstalk` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_")
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn short_num_mengikuti_pembagian_dan_pemotongan_nol() {
        assert_eq!(short_num(&json!(0)), "0");
        assert_eq!(short_num(&json!(999)), "999");
        assert_eq!(short_num(&json!(1000)), "1K");
        assert_eq!(short_num(&json!(1500)), "1.5K");
        assert_eq!(short_num(&json!(1_000_000)), "1M");
        assert_eq!(short_num(&json!(1_500_000_000)), "1.5B");
        // `parseInt("12.9")` = 12, bukan 12.9.
        assert_eq!(short_num(&json!("12.9")), "12");
    }

    #[test]
    fn caption_maksimal_lima_package_dan_kosong_jadi_tanda_hubung() {
        let paket: Vec<Value> = (0..8)
            .map(|i| json!({"name": format!("p{i}"), "version": "1.0.0", "downloads_monthly": 10, "description": "d"}))
            .collect();
        let p = Profil {
            username: "u".into(),
            name: String::new(),
            email: String::new(),
            description: String::new(),
            profile: "https://npmjs.com/~u".into(),
            avatar: String::new(),
            packages: paket,
        };
        let txt = caption(&p);
        assert!(txt.contains("📛 *Nama:* -"));
        assert!(txt.contains("📧 *Email:* -"));
        assert!(txt.contains("📝 *Deskripsi:*\n-"));
        assert!(txt.contains("*Daftar Package:*"));
        assert!(txt.contains("p4"));
        assert!(!txt.contains("p5"), "maksimal lima");
    }

    #[test]
    fn profil_nexray_menormalkan_bentuk_berbeda() {
        let r = json!({"name":"zann","author":{"name":"Zann","email":"z@x.io"},"description":"bio"});
        let p = dari_nexray(&r);
        assert_eq!(p.username, "zann");
        assert_eq!(p.name, "Zann");
        assert_eq!(p.email, "z@x.io");
        assert_eq!(p.description, "bio");
        assert_eq!(p.profile, "https://www.npmjs.com/~zann", "homepage kosong");
        assert!(p.avatar.is_empty(), "avatar nexray selalu kosong");
        assert!(p.packages.is_empty());
    }

    #[test]
    fn profil_firefly_menyalin_description_dari_name() {
        // `stalker-fallback.js:229` menulis `description: d.name`. Itu yang
        // di-copy, bukan `d.description`.
        let d = json!({"username":"u","name":"Nama","email":"e","description":"BIO","profile":"p"});
        let p = dari_firefly(&d);
        assert_eq!(p.description, "Nama");
    }
}
