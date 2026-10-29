//! `plugins/tools/dafont.js` — cari dan download font dari DaFont.
//!
//! Scraper ada di `oktz_scraper::dafont` (bukan disalin ke sini). Sesi
//! interaktif JS (`global.dafontSessions`) dipetakan ke `Ctx::session_*`, dan
//! `dafontAnswerHandler` didaftarkan lewat `#[oktz_plugin_api::answer_handler]`
//! (`dafont.js:105-142`).
//!
//! Semantik yang disalin dari JS:
//! - Kunci sesi = digit dari JID sender (`dafont.js:7-9`). Di sini kuncinya
//!   diberi awalan `dafont:` karena store sesi Rust dipakai bersama semua
//!   plugin, bukan per-plugin seperti `global.dafontSessions`.
//! - TTL 120 detik (`:5,23-26`) dicek manual dari `startedAt`; `setTimeout`
//!   tidak ada di handler sinkron.
//! - `result.results.slice(0, 10)` (`:79`), `fileName: v.name + ".zip"`
//!   (`:133`), `mimetype: "application/zip"` (`:134`).
//! - Balasan angka di luar daftar / chat berbeda / sesi tidak ada → `None`,
//!   padanan `return false` (`:106-113`).
//! - Reaksi berurutan JS (`🕕` di `:69`, lalu `✅`/`☢`/`❌`) tidak mungkin:
//!   satu `Reply` membawa satu reaksi, jadi yang dipegang reaksi penutup.
//! - Jalur `catch` handler (`:98-102`, "Gagal mencari font, coba lagi nanti")
//!   tidak terjangkau: `DaFont` menangkap semua galat jaringan sendiri dan
//!   selalu mengembalikan `{ status, error }`, sedangkan `status: true` selalu
//!   membawa `results` array. `Err` dari scraper Rust adalah pesan itu.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::{Value, json};

/// `SESSION_TIMEOUT` (`dafont.js:5`).
const SESSION_TIMEOUT_MS: i64 = 120_000;
/// `result.results.slice(0, 10)` (`dafont.js:79`).
const MAKS_ITEM: usize = 10;
/// Awalan kunci di store sesi bersama (JS-nya peta khusus dafont).
const SESSION_PREFIX: &str = "dafont:";

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔤 *DaFont Search*\n\nCari font dari DaFont, lalu reply nomor buat download.\n\n*PENGGUNAAN:*\n> *{prefix}dafont <nama font>*\n\n*CONTOH:*\n> *{prefix}dafont arial*\n> *{prefix}dafont horror*\n\n_Setelah daftar muncul, reply pesan bot dengan nomor font buat download file-nya_",
        prefix = prefix
    )
}

/// `getSessionKey(jid)` (`dafont.js:7-9`) + awalan milik plugin ini.
fn kunci_sesi(jid: &str) -> String {
    let angka: String = jid.chars().filter(char::is_ascii_digit).collect();
    if angka.is_empty() {
        format!("{SESSION_PREFIX}{}", jid.to_lowercase())
    } else {
        format!("{SESSION_PREFIX}{angka}")
    }
}

/// `getSession(jid)` (`dafont.js:11-14`) dengan TTL manual.
///
/// Sesi yang sudah lewat 120 detik ikut dihapus dari store, persis seperti
/// `setTimeout` di JS (`:23-26`).
fn sesi_aktif(ctx: &Ctx, sender: &str) -> Option<Value> {
    let key = kunci_sesi(sender);
    let sesi = ctx.session_get(&key)?;
    let mulai = sesi.get("startedAt").and_then(Value::as_i64).unwrap_or(0);
    if crate::common::now_ms().saturating_sub(mulai) >= SESSION_TIMEOUT_MS {
        ctx.session_take(&key);
        return None;
    }
    Some(sesi)
}

/// `${v}` JS untuk string yang scraper selalu isi (`name`, `author`).
fn teks(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or_default().to_string()
}

/// `v.downloads || "-"` / `v.license || "-"` (`dafont.js:87-88,119-120`).
///
/// Nilai falsy (`null`, `""`) jadi `"-"`; scraper hanya menghasilkan string
/// atau `null` di dua field ini.
fn atau_strip(v: Option<&Value>) -> String {
    if crate::common::js_truthy(v) {
        teks(v)
    } else {
        "-".to_string()
    }
}

#[plugin(
    name = "dafont",
    alias = ["daffont", "carifont"],
    category = "tools",
    description = "Cari dan download font dari DaFont",
    usage = ".dafont <nama font>",
    example = ".dafont arial",
    cooldown = 10,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dafont(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        // `m.react("❌")` lalu `m.reply(...)` (`dafont.js:56-67`).
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    let hasil = match oktz_scraper::dafont::dafont(&*ctx.http, &text) {
        Ok(v) => v,
        Err(e) => {
            return Reply::text(format!("❌ *DaFont Gagal*\n\n> {}", e.message))
                .with_reaction("☢");
        }
    };

    if !crate::common::js_truthy(hasil.get("status")) {
        // `m.react("☢")` + `❌ *DaFont Gagal*\n\n> ${result.error}` (`dafont.js:74-77`).
        let error = hasil.get("error").and_then(Value::as_str).unwrap_or_default();
        return Reply::text(format!("❌ *DaFont Gagal*\n\n> {error}")).with_reaction("☢");
    }

    let items: Vec<Value> = hasil
        .get("results")
        .and_then(Value::as_array)
        .map(|a| a.iter().take(MAKS_ITEM).cloned().collect())
        .unwrap_or_default();
    let count = hasil.get("count").and_then(Value::as_u64).unwrap_or(0);

    let mut txt = format!("🔤 *DaFont — {count} Font Ditemukan*\n\n> Pencarian: *{text}*\n\n");
    for (i, v) in items.iter().enumerate() {
        txt.push_str(&format!("*{}.* {}\n", i + 1, teks(v.get("name"))));
        txt.push_str(&format!("   ├ 👤 Author: {}\n", teks(v.get("author"))));
        txt.push_str(&format!(
            "   ├ 📥 Downloads: {}\n",
            atau_strip(v.get("downloads"))
        ));
        txt.push_str(&format!("   └ 📜 License: {}\n", atau_strip(v.get("license"))));
    }
    txt.push_str("\n_Reply pesan ini dengan nomor font buat download file-nya_");

    // `setSession(m.sender, items)` + `session.chat = m.chat` (`dafont.js:93-94`).
    ctx.session_set(
        &kunci_sesi(&ctx.sender),
        json!({ "data": items, "chat": ctx.chat, "startedAt": crate::common::now_ms() }),
    );

    Reply::text(txt).with_reaction("✅")
}

/// `dafontAnswerHandler` (`dafont.js:105-142`) — balasan nomor font.
///
/// `None` = `return false` di JS: bukan balasan angka yang sah, jadi rantai
/// answer handler lain lanjut.
#[oktz_plugin_api::answer_handler]
pub fn dafont_answer(ctx: Ctx) -> Option<Reply> {
    let sesi = sesi_aktif(&ctx, &ctx.sender)?;
    // `if (m.chat !== session.chat) return false;` (`dafont.js:108`).
    if sesi.get("chat").and_then(Value::as_str) != Some(ctx.chat.as_str()) {
        return None;
    }

    // `parseInt(text) - 1` lalu `isNaN || < 0 || >= data.length` (`dafont.js:110-113`).
    let index = crate::common::parse_int(ctx.text.trim())? - 1;
    let data = sesi.get("data").and_then(Value::as_array)?;
    if index < 0 || index as usize >= data.len() {
        return None;
    }
    let v = &data[index as usize];

    let nama = teks(v.get("name"));
    let detail = format!(
        "🔤 *{nama}*\n\n> 👤 Author: {}\n> 📥 Downloads: {}\n> 📜 License: {}",
        teks(v.get("author")),
        atau_strip(v.get("downloads")),
        atau_strip(v.get("license"))
    );

    let mut balasan = Vec::new();
    // `if (v.preview) sendMedia(image) else m.reply(detail)` (`dafont.js:122-126`).
    match v
        .get("preview")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        Some(url) => balasan.push(Reply::image(MediaSource::url(url), detail)),
        None => balasan.push(Reply::text(detail)),
    }
    // `if (v.download) sendMessage({ document, fileName: name + ".zip" })`
    // (`dafont.js:128-138`).
    if let Some(url) = v
        .get("download")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        balasan.push(Reply::document(
            MediaSource::url(url).with_mimetype("application/zip"),
            format!("{nama}.zip"),
        ));
    }

    // `clearSession(m.sender)` (`dafont.js:140`).
    ctx.session_take(&kunci_sesi(&ctx.sender));
    Some(Reply::sequence(balasan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, SessionStore, find};
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    const HALAMAN: &str = r#"<html><body>
      <div class="lv1left dfbg">
        <div class="fontname">Bebas Neue</div>
        <div class="author">by <a href="/a/nunito">Nunito</a></div>
      </div>
      <div class="lv2right">
        <span class="light"> 12,345 downloads (Yesterday: 12:00 PM)</span>
        <div class="lv2font"><a class="help" href="/licenses/free">Free for personal use</a></div>
      </div>
      <div class="dlbox"><a class="dl" href="//dl.dafont.com/dl.php?f=nunito" title="Download">Download</a></div>
      <div class="preview" style="background:url(/img/nunito.png) no-repeat"></div>
      <div class="lv1left dfbg"><div class="fontname">Tanpa Detail</div></div>
    </body></html>"#;

    /// Stub HTTP: satu balasan tetap untuk semua GET.
    #[derive(Debug)]
    struct StubHttp(String);

    impl StubHttp {
        fn baru(halaman: &str) -> Arc<StubHttp> {
            Arc::new(StubHttp(halaman.to_string()))
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.0.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper dafont cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper dafont cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper dafont cuma GET")
        }
    }

    fn ctx_of(sesi: &SessionStore, text: &str, http: Arc<dyn HttpGet>) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "dafont".into(),
            sessions: sesi.clone(),
            http,
            ..Ctx::default()
        }
    }

    fn balasan_dari(sesi: &SessionStore, body: &str) -> Ctx {
        Ctx {
            text: body.to_string(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: String::new(),
            sessions: sesi.clone(),
            ..Ctx::default()
        }
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn urutan(r: &Reply) -> Vec<Reply> {
        match r.without_mutations().tanpa_reaksi() {
            Reply::Sequence(list) => list.clone(),
            other => vec![other.clone()],
        }
    }

    fn sesi_mentah(sesi: &SessionStore) -> Option<Value> {
        sesi.lock().unwrap().get(&kunci_sesi(ME)).cloned()
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("dafont").unwrap();
        assert_eq!(p.meta.name, "dafont");
        assert_eq!(p.meta.alias, ["daffont", "carifont"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Cari dan download font dari DaFont");
        assert_eq!(p.meta.usage, ".dafont <nama font>");
        assert_eq!(p.meta.example, ".dafont arial");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn tanpa_argumen_mengembalikan_bantuan_dengan_reaksi_silang() {
        let sesi = oktz_plugin_api::new_session_store();
        let r = dafont(ctx_of(&sesi, "", StubHttp::baru(HALAMAN)));
        assert_eq!(r.text_body().unwrap(), help("."));
        assert_eq!(reaksi(&r), Some("❌"));
        assert!(sesi_mentah(&sesi).is_none(), "bantuan tidak membuat sesi");
    }

    #[test]
    fn hasil_pencarian_diformat_persis_seperti_js() {
        let sesi = oktz_plugin_api::new_session_store();
        let http = StubHttp::baru(HALAMAN);
        let r = dafont(ctx_of(&sesi, "bebas neue", http));

        let expected = concat!(
            "🔤 *DaFont — 2 Font Ditemukan*\n\n",
            "> Pencarian: *bebas neue*\n\n",
            "*1.* Bebas Neue\n",
            "   ├ 👤 Author: Nunito\n",
            "   ├ 📥 Downloads: 12,345 downloads\n",
            "   └ 📜 License: Free for personal use\n",
            "*2.* Tanpa Detail\n",
            "   ├ 👤 Author: \n",
            "   ├ 📥 Downloads: -\n",
            "   └ 📜 License: -\n\n",
            "_Reply pesan ini dengan nomor font buat download file-nya_",
        );
        assert_eq!(r.text_body().unwrap(), expected);
        assert_eq!(reaksi(&r), Some("✅"));

        let data = sesi_mentah(&sesi).unwrap().get("data").cloned().unwrap();
        assert_eq!(data.as_array().unwrap().len(), 2);
        assert_eq!(data[0]["download"], "https://dl.dafont.com/dl.php?f=nunito");
        assert_eq!(data[0]["preview"], "https://www.dafont.com/img/nunito.png");
    }

    #[test]
    fn hanya_sepuluh_item_pertama_yang_disimpan_tapi_count_apa_adanya() {
        let mut halaman = String::from("<html><body>");
        for i in 1..=12 {
            halaman.push_str(&format!(r#"<div class="lv1left dfbg">Font {i}</div>"#));
        }
        halaman.push_str("</body></html>");

        let sesi = oktz_plugin_api::new_session_store();
        let r = dafont(ctx_of(&sesi, "font", StubHttp::baru(&halaman)));
        assert!(
            r.text_body().unwrap().contains("*DaFont — 12 Font Ditemukan*"),
            "{}",
            r.text_body().unwrap()
        );

        let data = sesi_mentah(&sesi).unwrap().get("data").cloned().unwrap();
        assert_eq!(data.as_array().unwrap().len(), 10, "slice(0, 10)");
        assert_eq!(data[9]["name"], "Font 10");
        // Index 10 (nomor balasan "11") di luar daftar sesi.
        assert!(dafont_answer(balasan_dari(&sesi, "11")).is_none());
    }

    #[test]
    fn pencarian_gagal_membalas_pesan_galat_dan_tidak_membuat_sesi() {
        let sesi = oktz_plugin_api::new_session_store();
        let r = dafont(ctx_of(
            &sesi,
            "zzz",
            StubHttp::baru("<html><body><p>Tidak ada</p></body></html>"),
        ));
        assert_eq!(
            r.text_body().unwrap(),
            "❌ *DaFont Gagal*\n\n> Font tidak ditemukan"
        );
        assert_eq!(reaksi(&r), Some("☢"));
        assert!(sesi_mentah(&sesi).is_none());
    }

    #[test]
    fn galat_jaringan_memakai_kartu_gagal_yang_sama() {
        // `Ctx` default memakai `NoHttp` (selalu `Err`) — di JS jalur ini
        // `{status:false, error: e.message}` dari `DaFont`.
        let sesi = oktz_plugin_api::new_session_store();
        let r = dafont(ctx_of(&sesi, "arial", Arc::new(oktz_plugin_api::NoHttp)));
        assert!(
            r.text_body().unwrap().starts_with("❌ *DaFont Gagal*\n\n> "),
            "{}",
            r.text_body().unwrap()
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn kunci_sesi_hanya_ambil_digit_dan_fallback_huruf_kecil() {
        assert_eq!(kunci_sesi("628111@s.whatsapp.net"), "dafont:628111");
        // `[^0-9]` juga membuang `:` dan `@`; `:3` jadi ikut.
        assert_eq!(kunci_sesi("628111:3@s.whatsapp.net"), "dafont:6281113");
        assert_eq!(kunci_sesi("AbC@Newsletter"), "dafont:abc@newsletter");
        assert_eq!(kunci_sesi(""), "dafont:");
    }

    #[test]
    fn balasan_nomor_mengirim_detail_lalu_dokumen_zip() {
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));

        let r = dafont_answer(balasan_dari(&sesi, " 1 ")).expect("nomor 1 harus ditangani");
        let isi = urutan(&r);
        assert_eq!(isi.len(), 2, "detail + dokumen");

        match &isi[0] {
            Reply::Image {
                media: Some(MediaSource::Url { url, .. }),
                caption,
                ..
            } => {
                assert_eq!(url, "https://www.dafont.com/img/nunito.png");
                assert_eq!(
                    caption,
                    "🔤 *Bebas Neue*\n\n> 👤 Author: Nunito\n> 📥 Downloads: 12,345 downloads\n> 📜 License: Free for personal use"
                );
            }
            other => panic!("harus Image URL, dapat {other:?}"),
        }

        match &isi[1] {
            Reply::Document {
                media: Some(MediaSource::Url { url, mimetype }),
                file_name,
                caption,
                ..
            } => {
                assert_eq!(url, "https://dl.dafont.com/dl.php?f=nunito");
                assert_eq!(mimetype.as_deref(), Some("application/zip"));
                assert_eq!(file_name, "Bebas Neue.zip");
                assert_eq!(caption, "", "JS tidak mengirim caption dokumen");
            }
            other => panic!("harus Document, dapat {other:?}"),
        }

        assert!(
            sesi_mentah(&sesi).is_none(),
            "clearSession setelah balasan sukses (`dafont.js:140`)"
        );
    }

    #[test]
    fn nomor_di_luar_rentang_atau_bukan_angka_tidak_ditangani() {
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));

        for body in ["0", "3", "9999", "abc", "", "-2"] {
            assert!(
                dafont_answer(balasan_dari(&sesi, body)).is_none(),
                "body {body:?} tidak boleh ditangani"
            );
        }
        let mentah = sesi_mentah(&sesi).expect("sesi belum dibersihkan");
        assert_eq!(mentah["chat"], ME);
    }

    #[test]
    fn parse_int_berhenti_di_karakter_pertama_yang_bukan_angka() {
        // `parseInt` JS: "1.5" dan "1xyz" sama-sama jadi 1 (`dafont.js:111`).
        for body in ["1.5", "1xyz"] {
            let sesi = oktz_plugin_api::new_session_store();
            dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));
            assert!(
                dafont_answer(balasan_dari(&sesi, body)).is_some(),
                "body {body:?} harus dianggap nomor 1"
            );
        }
    }

    #[test]
    fn chat_berbeda_tidak_ditangani() {
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));

        let mut ctx = balasan_dari(&sesi, "1");
        ctx.chat = "120363@g.us".into();
        assert!(dafont_answer(ctx).is_none());
        assert!(sesi_mentah(&sesi).is_some(), "chat lain tidak menghapus sesi");
    }

    #[test]
    fn tanpa_sesi_tidak_ditangani() {
        let sesi = oktz_plugin_api::new_session_store();
        assert!(dafont_answer(balasan_dari(&sesi, "1")).is_none());
    }

    #[test]
    fn sesi_kedaluwarsa_pada_120_detik_tidak_ditangani_dan_dihapus() {
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));

        // Mundurkan `startedAt` sampai tepat TTL; `setTimeout` JS sudah
        // menghapus sesi pada titik ini.
        sesi.lock().unwrap().entry(kunci_sesi(ME).to_string()).and_modify(|s| {
            s["startedAt"] = json!(crate::common::now_ms() - SESSION_TIMEOUT_MS);
        });

        assert!(dafont_answer(balasan_dari(&sesi, "1")).is_none());
        assert!(sesi_mentah(&sesi).is_none(), "sesi basi ikut dibuang");
    }

    #[test]
    fn pencarian_baru_mengganti_sesi_lama() {
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));
        sesi.lock().unwrap().entry(kunci_sesi(ME).to_string()).and_modify(|s| {
            s["startedAt"] = json!(0);
        });

        dafont(ctx_of(&sesi, "bebas neue", StubHttp::baru(HALAMAN)));
        let s = sesi_mentah(&sesi).unwrap();
        let mulai = s["startedAt"].as_i64().unwrap();
        assert!(
            crate::common::now_ms() - mulai < SESSION_TIMEOUT_MS,
            "startedAt harus di-reset, bukan melanjutkan yang lama"
        );
        assert!(dafont_answer(balasan_dari(&sesi, "2")).is_some());
    }

    #[test]
    fn tanpa_preview_dibalas_teks_dan_tanpa_download_tanpa_dokumen() {
        let halaman = r#"<div class="lv1left dfbg">Polos</div>"#;
        let sesi = oktz_plugin_api::new_session_store();
        dafont(ctx_of(&sesi, "polos", StubHttp::baru(halaman)));

        let r = dafont_answer(balasan_dari(&sesi, "1")).unwrap();
        let isi = urutan(&r);
        assert_eq!(isi.len(), 1);
        assert_eq!(
            isi[0].text_body().unwrap(),
            "🔤 *Polos*\n\n> 👤 Author: \n> 📥 Downloads: -\n> 📜 License: -"
        );
    }
}
