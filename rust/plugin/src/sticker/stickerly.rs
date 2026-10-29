//! `plugins/sticker/stickerly.js` — cari dan unduh sticker pack Sticker.ly.
//!
//! Catatan pendaftaran: `pub mod stickerly;` perlu ditambah manual ke
//! `rust/plugin/src/sticker/mod.rs` lewat `scripts/pasang-modul.mjs` (mod.rs
//! milik orchestrator — jangan disentuh).
//!
//! `ponytail:` empat hal JS yang tidak bisa dicerminkan apa adanya:
//! - `sock.sendStickerPack(m.chat, urls, m, {...})` (`stickerly.js:147-154`)
//!   tidak ada padanannya; sticker dikirim per URL lewat `Reply::Sequence`
//!   (maks 20, sama seperti `slice(0, 20)` di `stickerly.js:144`). Metadata
//!   pack (`name`, `packname`, `publisher`, `author`, `description`, `emojis`)
//!   beserta fallback `"Sticker.ly Pack"`/`config.sticker.author`/`"Bot"`
//!   (`stickerly.js:142-143`) hilang karena `Reply::Sticker` tidak membawa
//!   metadata pack.
//! - Reaksi berurutan (`🕕` di `stickerly.js:22,115` lalu `❌`/`✅`/`☢`) tidak
//!   mungkin: satu `Reply` membawa satu reaksi. Jalur teks mengirim reaksi
//!   TERAKHIR (konvensi repo, mis. `rpg/jualan.rs`); jalur sticker bahkan itu
//!   tidak bisa — butir berikut.
//! - Reaksi tidak bisa dibarengkan dengan `Reply::Sequence`: jalur reaksi
//!   pipeline merender isinya lewat `render_reply`, dan `Sequence` ditolak di
//!   sana (`oktz-wa/src/pipeline.rs:2999-3008`, penolakan di `:1647-1649`) —
//!   seluruh balasan jadi error. Sticker pack dikirim sebagai `Sequence`
//!   TANPA reaksi `✅`; mengirim sticker lebih penting daripada reaksi.
//! - `stickerlyAnswerHandler` (`stickerly.js:74-112`) ikut diport, tapi BELUM
//!   BISA JALAN di produksi: `Router::answer` membangun `Ctx` lewat
//!   `oktz-router/src/lib.rs:170-196` tanpa pembungkus `PluginCtxSource::fill`
//!   (`oktz-wa/src/pipeline.rs:2151-2170`), jadi `user_data` Null dan
//!   `http`/`sessions` default; selain itu pipeline jalur answer
//!   (`oktz-wa/src/pipeline.rs:2881-2889`) tidak menulis mutasi balasan.
//!   Perbaikannya ada di crate `lib/` — di luar berkas ini.
//!
//! `delete user.stickerly_session` (`stickerly.js:86,95,107`) ditulis sebagai
//! `null` karena `Mutation::SetUser` menggabung, bukan menghapus key. Untuk
//! pembacaan JS semantiknya sama: `!user.stickerly_session` juga falsy untuk
//! `null`.

use oktz_plugin_api::{Ctx, MediaSource, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::{error_template, js_truthy, now_ms, parse_int};
use crate::tools::phisingataubukan::encode_uri_component;

/// `SESSION_TIMEOUT` (`stickerly.js:83`) — 5 menit.
const SESSION_TIMEOUT_MS: i64 = 5 * 60 * 1000;
/// `Math.min(data.result.length, 10)` (`stickerly.js:38`).
const MAX_PENCARIAN: usize = 10;
/// `stickerUrls.slice(0, 20)` (`stickerly.js:144`).
const MAX_STIKER: usize = 20;

/// Endpoint pencarian (`stickerly.js:29`).
fn url_pencarian(query: &str) -> String {
    format!(
        "https://my.izuka-api.xyz/api/search/stickerly-search?query={}",
        encode_uri_component(query.trim())
    )
}

/// Endpoint detail pack (`stickerly.js:119`).
fn url_pack(pack: &str) -> String {
    format!(
        "https://my.izuka-api.xyz/api/search/stickerly-pack?url={}",
        encode_uri_component(pack)
    )
}

/// `${nilai}` di template literal JS: key yang tidak ada jadi `undefined`,
/// JSON `null` jadi `null`.
///
/// `ponytail:` objek/array dicetak sebagai JSON, sedangkan JS mencetak
/// `[object Object]` — bentuk yang tidak muncul di data API ini.
fn teks_js(nilai: Option<&Value>) -> String {
    match nilai {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

/// `trayResourceUrl.split('/').slice(0, -1).join('/') + '/'`
/// (`stickerly.js:130`).
fn prefix_stiker(tray: &str) -> String {
    match tray.rfind('/') {
        Some(i) => format!("{}/", &tray[..i]),
        None => "/".to_string(),
    }
}

/// `user.stickerly_session = ...; db.save()` (`stickerly.js:59-63`) dan
/// `delete ...; db.save()` (`stickerly.js:86,95,107`) dengan `Value::Null`.
fn mutasi_sesi(ctx: &Ctx, sesi: Value) -> Mutation {
    Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "stickerly_session": sesi }),
    }
}

/// `Date.now() - session.time > SESSION_TIMEOUT` (`stickerly.js:85`).
/// `time` yang bukan angka membuat JS menghitung `NaN`, dan `NaN > X` false.
fn kedaluwarsa(sesi: &Value, sekarang: i64) -> bool {
    sesi
        .get("time")
        .and_then(Value::as_i64)
        .is_some_and(|t| sekarang - t > SESSION_TIMEOUT_MS)
}

/// `downloadStickerlyPack` (`stickerly.js:114-186`).
fn unduh_pack(ctx: &Ctx, pack: &str) -> Reply {
    let gagal_ambil = || Reply::text("❌ Gagal mengambil detail pack.").with_reaction("❌");
    // `packInfo.trayResourceUrl.split(...)` di JS melempar kalau fieldnya tidak
    // ada; lemparan itu ditangkap `catch` luar (`stickerly.js:181-185`).
    let rusak = || Reply::text(error_template(ctx)).with_reaction("☢");

    let Ok(res) = ctx.http.get(&url_pack(pack)) else {
        return rusak();
    };
    let data = res.json().unwrap_or(Value::Null);

    if !js_truthy(data.get("status")) {
        return gagal_ambil();
    }
    let Some(stiker) = data
        .get("result")
        .and_then(|r| r.get("stickers"))
        .and_then(Value::as_array)
        .filter(|s| !s.is_empty())
    else {
        return gagal_ambil();
    };
    let Some(info) = stiker[0].get("stickerPack") else {
        return rusak();
    };
    let Some(tray) = info.get("trayResourceUrl").and_then(Value::as_str) else {
        return rusak();
    };
    let prefix = prefix_stiker(tray);

    // `packInfo.name || "Sticker.ly Pack"` dan `packInfo.authorName ||
    // config.sticker.author || "Bot"` (`stickerly.js:142-143`) tidak dipakai:
    // metadata pack tidak punya kanal di `Reply` (lihat catatan modul).
    let urls: Vec<String> = stiker
        .iter()
        .take(MAX_STIKER)
        .map(|s| format!("{prefix}{}", teks_js(s.get("fileName"))))
        .collect();

    // `m.react("✅")` setelah kirim (`stickerly.js:155`) tidak ikut: lihat
    // catatan modul soal `Sequence` + reaksi.
    Reply::sequence(
        urls.into_iter()
            .map(|u| Reply::sticker(MediaSource::url(u)))
            .collect(),
    )
}

#[plugin(
    name = "stickerly",
    alias = ["stikerly", "stickerlysearch"],
    category = "sticker",
    description = "Cari dan download sticker pack dari Sticker.ly",
    usage = ".stickerly <query / url>",
    example = ".stickerly anime",
    cooldown = 10,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn stickerly(ctx: Ctx) -> Reply {
    let teks = ctx.text.as_str();

    // `if (!text)` (`stickerly.js:18`) — `m.text` sudah ter-trim di
    // `serialize.js:400-407`, sama seperti `Ctx.text` router.
    if teks.is_empty() {
        return Reply::text(format!(
            "⚠️ Harap masukkan kata kunci pencarian atau URL Sticker.ly!\nContoh: `{}{} anime`",
            ctx.prefix, ctx.command
        ));
    }

    if teks.contains("sticker.ly/s/") {
        return unduh_pack(&ctx, teks);
    }

    let Ok(res) = ctx.http.get(&url_pencarian(teks)) else {
        return Reply::text(error_template(&ctx)).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);
    let tidak_ditemukan = || {
        Reply::text(format!(
            "Maaf, sticker pack untuk \"{teks}\" tidak ditemukan. Coba kata kunci lain."
        ))
        .with_reaction("❌")
    };

    if !js_truthy(data.get("status")) {
        return tidak_ditemukan();
    }
    let Some(hasil) = data
        .get("result")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
    else {
        return tidak_ditemukan();
    };

    let mut daftar = format!(
        "🎨 *HASIL PENCARIAN STICKER.LY: {}*\n\nDitemukan beberapa pack, pilih salah satu ya:\n\n",
        teks.to_uppercase()
    );
    let mut sesi = Vec::new();
    for (i, item) in hasil.iter().take(MAX_PENCARIAN).enumerate() {
        daftar.push_str(&format!(
            "*{}.* {} by {}\n",
            i + 1,
            teks_js(item.get("name")),
            teks_js(item.get("authorName"))
        ));
        sesi.push(json!({
            "name": item.get("name").cloned().unwrap_or(Value::Null),
            "url": item.get("shareUrl").cloned().unwrap_or(Value::Null),
            "author": item.get("authorName").cloned().unwrap_or(Value::Null),
        }));
    }
    daftar.push_str(
        "\n> 💡 *Kirim angka (contoh: 1)* untuk mendownload pack, atau ketik `batal` \
         untuk membatalkan pencarian.",
    );

    Reply::text(daftar).with_reaction("✅").with_mutation(mutasi_sesi(
        &ctx,
        json!({ "results": sesi, "time": now_ms() }),
    ))
}

/// `stickerlyAnswerHandler` (`stickerly.js:74-112`) — jawaban angka/batal.
///
/// `None` = `return false` di JS. `m.isCommand` tidak diperiksa karena
/// pipeline hanya memanggil rantai answer untuk pesan non-command
/// (`oktz-wa/src/pipeline.rs:2881`). `session.results` yang hilang di JS
/// melempar TypeError (di luar `try`), di sini menjadi `None`.
#[oktz_plugin_api::answer_handler]
pub fn stickerly_answer(ctx: Ctx) -> Option<Reply> {
    // `if (!m.body || m.isCommand) return false;` (`stickerly.js:75`).
    if ctx.text.trim().is_empty() {
        return None;
    }

    // `db.getUser(m.sender).stickerly_session` (`stickerly.js:77-82`).
    let sesi = ctx.user_field("stickerly_session").cloned()?;
    if !js_truthy(Some(&sesi)) {
        return None;
    }

    if kedaluwarsa(&sesi, now_ms()) {
        return Some(
            Reply::text(
                "⏰ *SESI KEDALUWARSA*\n\nSesi pencarian stickerly sudah berakhir karena \
                 lebih dari 5 menit. Silakan ulangi perintah.",
            )
            .with_mutation(mutasi_sesi(&ctx, Value::Null)),
        );
    }

    let teks = ctx.text.trim().to_lowercase();

    if teks == "batal" || teks == "cancel" {
        return Some(
            Reply::text("🚪 Pencarian stickerly dibatalkan.")
                .with_mutation(mutasi_sesi(&ctx, Value::Null)),
        );
    }

    let pilihan = parse_int(&teks)?;
    let hasil = sesi.get("results").and_then(Value::as_array)?;
    if pilihan < 1 || pilihan as usize > hasil.len() {
        return None;
    }

    // `selectedPack.url` (`stickerly.js:106,110`) — key yang tidak ada
    // diteruskan sebagai `"undefined"` seperti `encodeURIComponent(undefined)`
    // di JS, jadi API tetap dipanggil dan gagal di jalur normal.
    let url = teks_js(hasil[pilihan as usize - 1].get("url"));

    Some(unduh_pack(&ctx, &url).with_mutation(mutasi_sesi(&ctx, Value::Null)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// Antrean balasan; habis = error jaringan.
    #[derive(Debug)]
    struct Stub {
        antrean: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn baru(balasan: Vec<Value>) -> Arc<Stub> {
            Arc::new(Stub {
                antrean: Mutex::new(balasan.into_iter().map(|b| b.to_string()).collect()),
                seen: Mutex::new(Vec::new()),
            })
        }

        fn offline() -> Arc<Stub> {
            Self::baru(Vec::new())
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let berikutnya = {
                let mut q = self.antrean.lock().unwrap();
                if q.is_empty() {
                    None
                } else {
                    Some(q.remove(0))
                }
            };
            match berikutnya {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(HttpError {
                    message: "offline".into(),
                }),
            }
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

    fn ctx_command(http: Arc<Stub>, teks: &str) -> Ctx {
        Ctx {
            text: teks.to_string(),
            args: teks.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "stickerly".into(),
            http,
            ..Ctx::default()
        }
    }

    fn jalankan(http: Arc<Stub>, teks: &str) -> Reply {
        stickerly(ctx_command(http, teks))
    }

    fn ctx_jawaban(http: Arc<Stub>, teks: &str, sesi: Value) -> Ctx {
        Ctx {
            text: teks.to_string(),
            sender: ME.into(),
            chat: ME.into(),
            user_data: json!({ "stickerly_session": sesi }),
            http,
            ..Ctx::default()
        }
    }

    fn teks_dari(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    /// `stickerly_session` yang ditulis mutasi SetUser.
    fn sesi_tertulis(r: &Reply) -> Value {
        let mutasi = r.mutations();
        assert_eq!(mutasi.len(), 1, "harus tepat satu mutasi SetUser");
        match &mutasi[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, ME);
                data["stickerly_session"].clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn hasil_pencarian(n: usize) -> Value {
        let hasil: Vec<Value> = (1..=n)
            .map(|i| {
                json!({
                    "name": format!("Pack {i}"),
                    "shareUrl": format!("https://sticker.ly/s/AAA{i}"),
                    "authorName": format!("Penulis {i}"),
                })
            })
            .collect();
        json!({ "status": true, "result": hasil })
    }

    fn isi_pack(n: usize) -> Value {
        let stiker: Vec<Value> = (1..=n)
            .map(|i| {
                json!({
                    "fileName": format!("{i}.webp"),
                    "stickerPack": {
                        "trayResourceUrl": "https://cdn.stickerly/tray.png",
                        "name": "Pack Lucu",
                        "authorName": "Penulis Pack",
                    },
                })
            })
            .collect();
        json!({ "status": true, "result": { "stickers": stiker } })
    }

    #[test]
    fn meta_dan_alias_sesuai_plugin_config_js() {
        let p = oktz_plugin_api::find("stickerly").unwrap();
        assert_eq!(p.meta.name, "stickerly");
        assert_eq!(p.meta.alias, ["stikerly", "stickerlysearch"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(
            p.meta.description,
            "Cari dan download sticker pack dari Sticker.ly"
        );
        assert_eq!(p.meta.usage, ".stickerly <query / url>");
        assert_eq!(p.meta.example, ".stickerly anime");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        // JS tidak menulis isGroup/isPrivate -> default `plugins.js:70-71` false.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn url_api_ter_encode_mirip_encodeuricomponent() {
        assert_eq!(
            url_pencarian("  anime lucu "),
            "https://my.izuka-api.xyz/api/search/stickerly-search?query=anime%20lucu"
        );
        assert_eq!(
            url_pack("https://sticker.ly/s/AAA"),
            "https://my.izuka-api.xyz/api/search/stickerly-pack?url=https%3A%2F%2Fsticker.ly%2Fs%2FAAA"
        );
    }

    #[test]
    fn prefix_stiker_mirip_split_slice_join() {
        assert_eq!(prefix_stiker("https://cdn/tray.png"), "https://cdn/");
        assert_eq!(prefix_stiker("https://cdn/dir/"), "https://cdn/dir/");
        assert_eq!(prefix_stiker("tray.png"), "/");
    }

    #[test]
    fn teks_js_undefined_dan_null() {
        assert_eq!(teks_js(None), "undefined");
        assert_eq!(teks_js(Some(&Value::Null)), "null");
        assert_eq!(teks_js(Some(&json!("x"))), "x");
        assert_eq!(teks_js(Some(&json!(7))), "7");
    }

    #[test]
    fn teks_kosong_membalas_kartu_pakai() {
        let http = Stub::offline();
        let r = jalankan(http.clone(), "");
        assert_eq!(
            teks_dari(&r),
            "⚠️ Harap masukkan kata kunci pencarian atau URL Sticker.ly!\n\
             Contoh: `.stickerly anime`"
        );
        assert_eq!(reaksi(&r), None);
        assert!(r.mutations().is_empty());
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn pencarian_sukses_menyimpan_sesi_dan_menyusun_daftar() {
        let http = Stub::baru(vec![hasil_pencarian(12)]);
        let r = jalankan(http.clone(), "anime");

        let t = teks_dari(&r);
        assert!(t.starts_with("🎨 *HASIL PENCARIAN STICKER.LY: ANIME*"), "{t}");
        assert!(t.contains("Ditemukan beberapa pack, pilih salah satu ya:"), "{t}");
        assert!(t.contains("*1.* Pack 1 by Penulis 1\n"), "{t}");
        assert!(t.contains("*10.* Pack 10 by Penulis 10\n"), "{t}");
        assert!(!t.contains("*11.*"), "maks 10 hasil: {t}");
        assert!(
            t.contains(
                "> 💡 *Kirim angka (contoh: 1)* untuk mendownload pack, atau ketik \
                 `batal` untuk membatalkan pencarian."
            ),
            "{t}"
        );
        assert_eq!(reaksi(&r), Some("✅"));

        let sesi = sesi_tertulis(&r);
        assert_eq!(sesi["results"].as_array().unwrap().len(), 10);
        assert_eq!(sesi["results"][0]["name"], json!("Pack 1"));
        assert_eq!(sesi["results"][0]["url"], json!("https://sticker.ly/s/AAA1"));
        assert_eq!(sesi["results"][0]["author"], json!("Penulis 1"));
        assert!(sesi["time"].as_i64().unwrap() > 0);

        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://my.izuka-api.xyz/api/search/stickerly-search?query=anime"]
        );
    }

    #[test]
    fn pencarian_tanpa_hasil_atau_status_salah() {
        for body in [
            json!({}),
            json!({ "status": true }),
            json!({ "status": true, "result": [] }),
            json!({
                "status": false,
                "result": [{ "name": "x", "shareUrl": "u", "authorName": "a" }],
            }),
            Value::Null,
        ] {
            let r = jalankan(Stub::baru(vec![body.clone()]), "anime");
            assert_eq!(reaksi(&r), Some("❌"), "{body}");
            assert_eq!(
                teks_dari(&r),
                "Maaf, sticker pack untuk \"anime\" tidak ditemukan. Coba kata kunci lain.",
                "{body}"
            );
            assert!(r.mutations().is_empty(), "{body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_membalas_error_template() {
        let r = jalankan(Stub::offline(), "anime");
        assert_eq!(
            teks_dari(&r),
            "☢ Kayaknya command `.stickerly` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn url_pack_di_teks_langsung_mengirim_stiker() {
        let http = Stub::baru(vec![isi_pack(3)]);
        let r = jalankan(http.clone(), "https://sticker.ly/s/AAA");

        assert_eq!(reaksi(&r), None, "Sequence tidak bisa dibarengi reaksi");
        let Reply::Sequence(list) = r.without_mutations() else {
            panic!("harus Sequence, dapat {:?}", r.without_mutations());
        };
        assert_eq!(list.len(), 3);
        for (i, item) in list.iter().enumerate() {
            let Some(MediaSource::Url { url, .. }) = item.media_source() else {
                panic!("harus sticker URL, dapat {item:?}");
            };
            assert_eq!(url, &format!("https://cdn.stickerly/{}.webp", i + 1));
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://my.izuka-api.xyz/api/search/stickerly-pack?url=https%3A%2F%2Fsticker.ly%2Fs%2FAAA"]
        );
    }

    #[test]
    fn lebih_dari_dua_puluh_stiker_dipotong() {
        let r = jalankan(Stub::baru(vec![isi_pack(25)]), "https://sticker.ly/s/AAA");
        let Reply::Sequence(list) = r.without_mutations() else {
            panic!("harus Sequence, dapat {:?}", r.without_mutations());
        };
        assert_eq!(list.len(), 20);
        let Some(MediaSource::Url { url, .. }) = list[19].media_source() else {
            panic!("harus sticker URL");
        };
        assert_eq!(url, "https://cdn.stickerly/20.webp");
    }

    #[test]
    fn status_pack_salah_atau_tanpa_stiker_dibalas_gagal() {
        for body in [
            json!({}),
            json!({ "status": true, "result": {} }),
            json!({ "status": true, "result": { "stickers": [] } }),
            Value::Null,
        ] {
            let r = jalankan(
                Stub::baru(vec![body.clone()]),
                "https://sticker.ly/s/AAA",
            );
            assert_eq!(reaksi(&r), Some("❌"), "{body}");
            assert_eq!(teks_dari(&r), "❌ Gagal mengambil detail pack.", "{body}");
            assert!(r.mutations().is_empty(), "{body}");
        }
    }

    #[test]
    fn pack_tanpa_tray_atau_stickerpack_ikut_jalur_catch_luar() {
        for body in [
            json!({ "status": true, "result": { "stickers": [{ "fileName": "a.webp" }] } }),
            json!({
                "status": true,
                "result": { "stickers": [{ "fileName": "a.webp", "stickerPack": { "name": "X" } }] },
            }),
            json!({ "status": true, "result": { "stickers": ["bukan objek"] } }),
        ] {
            let r = jalankan(
                Stub::baru(vec![body.clone()]),
                "https://sticker.ly/s/AAA",
            );
            assert_eq!(reaksi(&r), Some("☢"), "{body}");
            assert!(teks_dari(&r).contains("lagi ada kendala"), "{body}");
        }
    }

    #[test]
    fn batas_kedaluwarsa_lima_menit() {
        let t = 1_000_000;
        assert!(!kedaluwarsa(&json!({ "time": t }), t + SESSION_TIMEOUT_MS));
        assert!(kedaluwarsa(&json!({ "time": t }), t + SESSION_TIMEOUT_MS + 1));
        // Tanpa `time` JS menghitung NaN, dan `NaN > X` false.
        assert!(!kedaluwarsa(&json!({ "results": [] }), t + SESSION_TIMEOUT_MS + 1));
        assert!(!kedaluwarsa(&json!({ "time": "x" }), t + SESSION_TIMEOUT_MS + 1));
    }

    #[test]
    fn jawaban_tanpa_sesi_diabaikan() {
        assert!(stickerly_answer(ctx_jawaban(Stub::offline(), "1", Value::Null)).is_none());
        assert!(stickerly_answer(ctx_jawaban(Stub::offline(), "   ", Value::Null)).is_none());
        assert!(stickerly_answer(ctx_jawaban(Stub::offline(), "", Value::Null)).is_none());
        // `user_data` Null (user belum pernah ada di store) — `db.getUser` miss.
        let ctx = Ctx {
            text: "1".into(),
            sender: ME.into(),
            chat: ME.into(),
            http: Stub::offline(),
            ..Ctx::default()
        };
        assert!(stickerly_answer(ctx).is_none());
    }

    #[test]
    fn jawaban_batal_menghapus_sesi() {
        let sesi = json!({
            "results": [{ "name": "Pack", "url": "https://sticker.ly/s/AAA", "author": "A" }],
            "time": now_ms(),
        });
        for kata in ["batal", "CANCEL", "  Batal  "] {
            let r = stickerly_answer(ctx_jawaban(Stub::offline(), kata, sesi.clone()))
                .expect("harus menjawab");
            assert_eq!(teks_dari(&r), "🚪 Pencarian stickerly dibatalkan.");
            assert_eq!(reaksi(&r), None);
            assert_eq!(sesi_tertulis(&r), Value::Null);
        }
    }

    #[test]
    fn jawaban_angka_di_luar_rentang_atau_bukan_angka_diabaikan() {
        let sesi = json!({
            "results": [{ "url": "u" }, { "url": "u2" }],
            "time": now_ms(),
        });
        for teks in ["0", "3", "abc", "-1"] {
            assert!(
                stickerly_answer(ctx_jawaban(Stub::offline(), teks, sesi.clone())).is_none(),
                "{teks}"
            );
        }
        // `parseInt("2abc") = 2` (kuirk parseInt JS) — pack kedua tetap dipilih.
        let r = stickerly_answer(ctx_jawaban(Stub::offline(), "2abc", sesi.clone()))
            .expect("parseInt berhenti di non-digit");
        assert_eq!(reaksi(&r), Some("☢"), "stub offline");
        // `session.results` hilang: JS melempar TypeError, di sini `None`.
        let tanpa_hasil = json!({ "time": now_ms() });
        assert!(stickerly_answer(ctx_jawaban(Stub::offline(), "1", tanpa_hasil)).is_none());
    }

    #[test]
    fn jawaban_angka_mengunduh_pack_dan_menghapus_sesi() {
        let http = Stub::baru(vec![isi_pack(2)]);
        let sesi = json!({
            "results": [
                { "name": "Pack 1", "url": "https://sticker.ly/s/AAA1", "author": "A" },
                { "name": "Pack 2", "url": "https://sticker.ly/s/AAA2", "author": "B" },
            ],
            "time": now_ms(),
        });
        let r = stickerly_answer(ctx_jawaban(http.clone(), "2", sesi)).expect("harus menjawab");

        assert_eq!(reaksi(&r), None, "Sequence tidak bisa dibarengi reaksi");
        let Reply::Sequence(list) = r.without_mutations() else {
            panic!("harus Sequence, dapat {:?}", r.without_mutations());
        };
        assert_eq!(list.len(), 2);
        assert_eq!(sesi_tertulis(&r), Value::Null);
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://my.izuka-api.xyz/api/search/stickerly-pack?url=https%3A%2F%2Fsticker.ly%2Fs%2FAAA2"]
        );
    }

    #[test]
    fn jawaban_sesi_kedaluwarsa_dihapus_dan_diberi_tahu() {
        let http = Stub::offline();
        let sesi = json!({
            "results": [{ "name": "Pack", "url": "https://sticker.ly/s/AAA", "author": "A" }],
            "time": now_ms() - SESSION_TIMEOUT_MS - 1,
        });
        let r = stickerly_answer(ctx_jawaban(http.clone(), "1", sesi)).expect("harus menjawab");

        assert!(teks_dari(&r).contains("⏰ *SESI KEDALUWARSA*"), "{}", teks_dari(&r));
        assert_eq!(reaksi(&r), None);
        assert_eq!(sesi_tertulis(&r), Value::Null);
        assert!(http.seen.lock().unwrap().is_empty(), "tidak ada HTTP saat kedaluwarsa");
    }
}
