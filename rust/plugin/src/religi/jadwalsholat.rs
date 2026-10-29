//! `plugins/religi/jadwalsholat.js` — jadwal sholat real-time dari myquran.com.
//!
//! `ponytail:` `saluranCtx()` dari `src/lib/context.js` tidak tersedia di API
//! plugin, jadi `contextInfo` (newsletter) diabaikan. Audio adzan dan teks
//! dikirim berurutan lewat `Reply::sequence` (audio dulu, teks sesudahnya,
//! mirip `await` berurutan di JS). Satu reaksi akhir (`✅`/`❌`/`☢`).

use serde_json::Value;
use oktz_plugin_api::{Ctx, HttpError, MediaSource, Reply, plugin};

use crate::common::{days_from_civil, jakarta_parts, js_truthy, js_truthy_value, MONTHS_ID};
use crate::tools::phisingataubukan::encode_uri_component;

const BASE_URL: &str = "https://api.myquran.com/v2/sholat";
const ADZAN_URL: &str = "https://files.catbox.moe/z2bj5s.mp3";

const HARI_ID: [&str; 7] = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];

/// `index Hari Minggu=0..Sabtu=6` dari tanggal civil (1970-01-01 = Kamis).
fn hari_index_ms(ms: i64) -> usize {
    let (y, m, d, ..) = jakarta_parts(ms);
    let days = days_from_civil(y, m, d);
    ((days + 4) % 7) as usize
}

/// `formatNow("dddd, DD MMMM YYYY")` di Asia/Jakarta.
fn tanggal_lengkap(ms: i64) -> String {
    let (y, m, d, ..) = jakarta_parts(ms);
    let hari = HARI_ID[hari_index_ms(ms)];
    format!("{}, {:02} {} {}", hari, d, MONTHS_ID[(m - 1) as usize], y)
}

fn kota_url(query: &str) -> String {
    format!("{BASE_URL}/kota/cari/{}", encode_uri_component(query))
}

fn jadwal_url(kota_id: &str, (y, m, d): (i64, u32, u32)) -> String {
    format!(
        "{BASE_URL}/jadwal/{kota_id}/{y}/{:02}/{:02}",
        m, d
    )
}

fn not_found_kota(city: &str) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n> Kota \"{city}\" tidak ditemukan\n> Coba nama kabupaten/kota lain"
    )
}

/// Nilai waktu sholat: `j.<waktu> || "-"`.
fn waktu_val(jadwal: &Value, kunci: &str) -> String {
    jadwal
        .get(kunci)
        .and_then(Value::as_str)
        .map(|s| if s.is_empty() { "-".to_string() } else { s.to_string() })
        .unwrap_or_else(|| "-".to_string())
}

/// Ambil objek jadwal (pakai `.jadwal` kalau ada, kalau tidak data itu sendiri).
fn ambil_jadwal(data: &Value) -> &Value {
    match data.get("jadwal") {
        Some(v) if js_truthy_value(v) => v,
        _ => data,
    }
}

#[plugin(
    name = "jadwalsholat",
    alias = ["sholat", "prayertime", "jadwalsolat", "waktusolat", "waktusholat"],
    category = "religi",
    description = "Menampilkan jadwal sholat real-time dari myquran.com",
    usage = ".jadwalsholat <kota>",
    example = ".jadwalsholat Jakarta",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn jadwalsholat(ctx: Ctx) -> Reply {
    let argumen = ctx.args.join(" ");
    let city = argumen.trim();
    let city = if city.is_empty() { "Jakarta" } else { city };

    let Ok(kota) = cari_kota(&ctx, city) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let Some(kota) = kota else {
        return Reply::text(not_found_kota(city)).with_reaction("❌");
    };

    let kota_id = kota.get("id").and_then(Value::as_str).unwrap_or("");
    let (y, m, d, ..) = jakarta_parts(crate::common::now_ms());
    let Ok(res) = ctx.http.get(&jadwal_url(kota_id, (y, m, d))) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(serde_json::Value::Null);
    if !js_truthy(body.get("status")) {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    }
    let jadwal_data = body.get("data").filter(|d| js_truthy_value(d));
    let jadwal_data = match jadwal_data {
        Some(d) => d,
        None => {
            return Reply::text(crate::common::error_template(&ctx))
                .with_reaction("☢");
        }
    };

    let jadwal = ambil_jadwal(jadwal_data);
    let lokasi = jadwal_data
        .get("lokasi")
        .or_else(|| kota.get("lokasi"))
        .and_then(Value::as_str)
        .unwrap_or("-");
    let daerah = jadwal_data
        .get("daerah")
        .and_then(Value::as_str)
        .unwrap_or("");

    let today = tanggal_lengkap(crate::common::now_ms());
    let caption = format!(
        "🕌 *ᴊᴀᴅᴡᴀʟ sʜᴏʟᴀᴛ*\n\
         ╭┈┈⬡「 📍 *{lokasi}* 」\n\
         ┃ 📅 {today}\n\
         ┃ 🗺️ {daerah}\n\
         ╰┈┈⬡\n\
         ╭┈┈⬡「 ⏰ *ᴡᴀᴋᴛᴜ sʜᴏʟᴀᴛ* 」\n\
         ┃ 🌙 ɪᴍsᴀᴋ: `{}`\n\
         ┃ 🌅 sᴜʙᴜʜ: `{}`\n\
         ┃ ☀️ ᴛᴇʀʙɪᴛ: `{}`\n\
         ┃ 🌤️ ᴅʜᴜʜᴀ: `{}`\n\
         ┃ 🌞 ᴅᴢᴜʜᴜʀ: `{}`\n\
         ┃ 🌇 ᴀsʜᴀʀ: `{}`\n\
         ┃ 🌆 ᴍᴀɢʜʀɪʙ: `{}`\n\
         ┃ 🌃 ɪsʏᴀ: `{}`\n\
         ╰┈┈⬡\n\
         > _Sumber: myquran.com | Jangan lupa sholat ya! 🤲_",
        waktu_val(jadwal, "imsak"),
        waktu_val(jadwal, "subuh"),
        waktu_val(jadwal, "terbit"),
        waktu_val(jadwal, "dhuha"),
        waktu_val(jadwal, "dzuhur"),
        waktu_val(jadwal, "ashar"),
        waktu_val(jadwal, "maghrib"),
        waktu_val(jadwal, "isya")
    );

    // Coba unduh audio adzan; kalau gagal, hanya kirim teks.
    let adzan_ok = ctx.http.get(ADZAN_URL).is_ok();

    if adzan_ok {
        Reply::sequence(vec![
            Reply::audio(MediaSource::url(ADZAN_URL.to_string())),
            Reply::text(caption),
        ])
    } else {
        Reply::text(caption).with_reaction("✅")
    }
}

/// `searchKota` (`sholat-api.js:6-18`): `Ok(None)` kalau `status` falsy atau
// `data` kosong; `Err` kalau request gagal.
fn cari_kota(ctx: &Ctx, query: &str) -> Result<Option<serde_json::Value>, HttpError> {
    let res = ctx.http.get(&kota_url(query))?;
    let Some(body) = res.json() else {
        return Ok(None);
    };
    if !js_truthy(body.get("status")) {
        return Ok(None);
    }
    Ok(body
        .get("data")
        .and_then(serde_json::Value::as_array)
        .and_then(|list| list.first())
        .cloned())
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// `jakarta_parts` ke milidetik (untuk test hari).
#[cfg(test)]
fn jakarta_parts_to_ms(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    let days = days_from_civil(y, m, d);
    let secs = days * 86_400 + (h as i64) * 3600 + (mi as i64) * 60 + s as i64;
    (secs - 7 * 3600) * 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn with_balasan(balasan: Vec<String>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(balasan),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut b = self.balasan.lock().unwrap();
            let teks = if b.is_empty() {
                String::new()
            } else {
                b.remove(0)
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(teks.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
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

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "jadwalsholat".into(),
            http,
            ..Ctx::default()
        };
        jadwalsholat(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn kota_json() -> String {
        r#"{"status":true,"data":[{"id":"1301","lokasi":"KOTA JAKARTA","nama":"Jakarta","daerah":"DKI Jakarta"}]}"#.to_string()
    }

    fn jadwal_json() -> String {
        r#"{"status":true,"data":{"jadwal":{"imsak":"04:10","subuh":"04:20","terbit":"05:45","dhuha":"06:30","dzuhur":"11:50","ashar":"15:10","maghrib":"18:05","isya":"19:20"},"lokasi":"KOTA JAKARTA","daerah":"DKI Jakarta"}}"#.to_string()
    }

    #[test]
    fn default_kota_jakarta() {
        let http = Stub::with_balasan(vec![kota_json(), jadwal_json()]);
        let r = run(http, "");
        let t = text_of(&r);
        assert!(t.contains("📍 *KOTA JAKARTA*"), "{t}");
        assert!(t.contains("📅 "), "{t}");
        assert!(t.contains("🌙 ɪᴍsᴀᴋ: `04:10`"), "{t}");
        assert!(t.contains("🌃 ɪsʏᴀ: `19:20`"), "{t}");
        assert!(t.contains("> _Sumber: myquran.com"), "{t}");
    }

    #[test]
    fn kota_tidak_ditemukan() {
        let http = Stub::with_balasan(vec![r#"{"status":true,"data":[]}"#.to_string()]);
        let r = run(http, "KotaXyz");
        assert_eq!(
            text_of(&r),
            not_found_kota("KotaXyz"),
            "{:?}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn lokasi_fallback_ke_kota() {
        // jadwal data tanpa `lokasi` → pakai `kota.lokasi`.
        let http = Stub::with_balasan(vec![
            kota_json(),
            r#"{"status":true,"data":{"jadwal":{"imsak":"04:10"},"daerah":"DKI Jakarta"}}"#.to_string(),
        ]);
        let r = run(http, "Jakarta");
        assert!(text_of(&r).contains("📍 *KOTA JAKARTA*"), "{:?}", text_of(&r));
    }

    #[test]
    fn tanggal_menggunakan_format_id_dengan_nama_hari() {
        let http = Stub::with_balasan(vec![kota_json(), jadwal_json()]);
        let r = run(http, "Jakarta");
        let t = text_of(&r);
        assert!(t.starts_with("🕌 *ᴊᴀᴅᴡᴀʟ sʜᴏʟᴀᴛ*\n╭┈┈⬡「 📍 *KOTA JAKARTA* 」\n┃ 📅 "), "{t}");
        // Nama bulan tidak boleh di-hardcode ke awal tahun: `formatNow` JS
        // (`plugins/religi/jadwalsholat.js:40`) memakai bulan tanggal berjalan,
        // jadi test ini bisa dijalankan kapan saja.
        assert!(
            HARI_ID.iter().any(|h| t.contains(*h))
                && MONTHS_ID.iter().any(|m| t.contains(*m)),
            "{t}"
        );
    }

    #[test]
    fn status_falsy_menggunakan_error_template() {
        // `{"status":false}` di respons *kota* berarti `searchKota` → null
        // (pesan tidak ditemukan, `sholat-api.js:12-17`). Yang masuk `catch`
        // (`jadwalsholat.js:90-92`) adalah status falsy dari `getTodaySchedule`
        // (`sholat-api.js:43-47`), jadi stub kedua harus respons jadwal.
        let http = Stub::with_balasan(vec![kota_json(), r#"{"status":false}"#.to_string()]);
        let r = run(http, "Jakarta");
        assert_eq!(
            text_of(&r),
            crate::common::error_template_bawaan(".", "jadwalsholat"),
            "{:?}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn jadwal_tanpa_status_menghasilkan_error_template() {
        let http = Stub::with_balasan(vec![kota_json(), r#"{}"#.to_string()]);
        let r = run(http, "Jakarta");
        assert_eq!(
            text_of(&r),
            crate::common::error_template_bawaan(".", "jadwalsholat"),
            "{:?}",
            text_of(&r)
        );
    }

    #[test]
    fn waktu_kosong_dijadikan_tanda_hubung() {
        // Stub lama bukan JSON valid (ada `\"` di dalam raw string), jadi kena
        // cabang error sebelum sempat memeriksa `|| "-"` (`sholat-api.js:53`).
        let http = Stub::with_balasan(vec![
            kota_json(),
            r#"{"status":true,"data":{"jadwal":{"imsak":"","subuh":"04:20"},"lokasi":"X"}}"#.to_string(),
        ]);
        let r = run(http, "Jakarta");
        let t = text_of(&r);
        assert!(t.contains("🌙 ɪᴍsᴀᴋ: `-`"), "{t}");
        assert!(t.contains("🌅 sᴜʙᴜʜ: `04:20`"), "{t}");
    }

    #[test]
    fn adzan_url_dipanggil_setelah_jadwal() {
        let http = Stub::with_balasan(vec![kota_json(), jadwal_json(), "audio".to_string()]);
        let r = run(http.clone(), "Jakarta");
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert!(seen[2].contains(ADZAN_URL), "{:?}", seen);
        // Sequence: audio lalu teks → text_body adalah teks.
        assert_eq!(reaction_of(&r), None);
        assert!(text_of(&r).contains("04:10"), "{:?}", text_of(&r));
    }

    #[test]
    fn hari_index_mirip_expektasi() {
        // 2026-01-18 adalah Minggu.
        let ms = jakarta_parts_to_ms(2026, 1, 18, 0, 0, 0);
        assert_eq!(HARI_ID[hari_index_ms(ms)], "Minggu");
    }
}
