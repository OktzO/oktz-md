//! `plugins/tools/nikparser.js` — parse dan validasi NIK KTP.
//! Butuh HTTP (ctx.http) + `config.APIkey.obscura` yang sudah ada di Runtime.
//! Catatan: `tools/mod.rs` sudah milik plugin lain, jangan sentuh;
//! orchestrator harus menambah baris `pub mod nikparser;` ke `tools/mod.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value, json};

use crate::common::error_template;

const OBSCURA_BASE: &str = "https://api.obscuraworks.org/api/v2/tools/nik";
const NEXRAY_BASE: &str = "https://api.nexray.eu.cc/tools/nikparse";

/// `nikparser.js:4-14` — meta plugin.
/// `nikparser.js:54-62` — kartu pakai saat NIK kurang dari 16 digit.
fn kartu_pakai(prefix: &str) -> String {
    format!(
        "🪪 *ɴɪᴋ ᴘᴀʀꜱᴇʀ*\n\n- Parse dan validasi NIK KTP 🇮🇩\n\
         - Masukkan 16 digit angka NIK\n\n`{}nikparser 3517072109020003`",
        prefix
    )
}

/// `nikparser.js:90` — mapping kode provinsi ke nama, persis seperti JS.
fn nama_provinsi(kode: i64) -> &'static str {
    match kode {
        11 => "Aceh",
        12 => "Sumatera Utara",
        13 => "Sumatera Barat",
        14 => "Riau",
        15 => "Jambi",
        16 => "Sumatera Selatan",
        17 => "Bengkulu",
        18 => "Lampung",
        19 => "Kepululauan Bangka Belitung",
        21 => "Kepulauan Riau",
        31 => "DKI Jakarta",
        32 => "Jawa Barat",
        33 => "Jawa Tengah",
        34 => "DI Yogyakarta",
        35 => "Jawa Timur",
        36 => "Banten",
        51 => "Bali",
        52 => "Nusa Tenggara Barat",
        53 => "Nusa Tenggara Timur",
        61 => "Kalimantan Barat",
        62 => "Kalimantan Tengah",
        63 => "Kalimantan Selatan",
        64 => "Kalimantan Timur",
        65 => "Kalimantan Utara",
        71 => "Sulawesi Utara",
        72 => "Sulawesi Tengah",
        73 => "Sulawesi Selatan",
        74 => "Sulawesi Tenggara",
        75 => "Gorontalo",
        76 => "Sulawesi Barat",
        81 => "Maluku",
        82 => "Maluku Utara",
        91 => "Papua",
        92 => "Papua Barat",
        _ => "-",
    }
}

/// `nikparser.js:97-100` — nama hari dalam bahasa Indonesia.
fn nama_hari(idx: u32) -> &'static str {
    match idx {
        0 => "Minggu",
        1 => "Senin",
        2 => "Selasa",
        3 => "Rabu",
        4 => "Kamis",
        5 => "Jumat",
        6 => "Sabtu",
        _ => "",
    }
}

/// `nikparser.js:97-100` — nama bulan dalam bahasa Indonesia.
fn nama_bulan(idx: u32) -> &'static str {
    match idx {
        1 => "Januari",
        2 => "Februari",
        3 => "Maret",
        4 => "April",
        5 => "Mei",
        6 => "Juni",
        7 => "Juli",
        8 => "Agustus",
        9 => "September",
        10 => "Oktober",
        11 => "November",
        12 => "Desember",
        _ => "",
    }
}

/// `nikparser.js:82-88` — format `new Date(birthISO).toLocaleDateString("id-ID", ...)`.
///
/// JS `toLocaleDateString("id-ID", { weekday: 'long', year: 'numeric',
/// month: 'long', day: 'numeric' })` -> "Senin, 21 September 2009".
/// Diverifikasi lewat node: `new Date("2009-09-21").toLocaleDateString("id-ID", ...)`.
/// Di sini dibangun manual supaya deterministik tanpa dependensi ICU.
fn format_tanggal_id(birth_iso: &str) -> String {
    // "YYYY-MM-DD" atau penuh dengan waktu.
    let tanggal = birth_iso.split('T').next().unwrap_or(birth_iso);
    let mut parts = tanggal.split('-');
    let tahun: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let bulan: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let hari: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);

    // Zeller congruence untuk hari (0 = Minggu).
    let (m, y) = if bulan < 3 { (bulan + 12, tahun - 1) } else { (bulan, tahun) };
    let (m, y) = (m as i64, y);
    let k = y % 100;
    let j = y / 100;
    let h = (hari
        + (13 * (m + 1)) / 5
        + k
        + k / 4
        + j / 4
        + 5 * j)
        % 7;
    let idx_hari = (h + 6) % 7; // Zeller 0=Sabtu -> id-ID 0=Minggu

    format!(
        "{}, {} {} {}",
        nama_hari(idx_hari as u32),
        hari,
        nama_bulan(bulan),
        tahun
    )
}

/// `nikparser.js:89-98` — susun teks hasil parse.
fn hasil_teks(data: &Value) -> String {
    let raw = data
        .get("raw")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    // `parseNik` mengirim `birthISO: null` saat API tidak memberi `lahir`,
    // dan `new Date(null)` di JS adalah epoch — bukan "-".
    let tanggal = match data.get("birthISO").and_then(Value::as_str) {
        Some("") => "Invalid Date".to_string(),
        Some(iso) => format_tanggal_id(iso),
        None => format_tanggal_id("1970-01-01"),
    };
    let gender = data.get("gender").and_then(Value::as_str).unwrap_or("-");
    let gender_emoji = if gender == "pria" { "♂️" } else { "♀️" };
    let gender_label = if gender.is_empty() || gender == "-" {
        "-".to_string()
    } else {
        let mut c = gender.chars();
        match c.next() {
            Some(ch) => ch.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    };
    let prov_id = data
        .get("provinceId")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let prov_nama = data
        .get("province")
        .and_then(Value::as_str)
        .unwrap_or("-");
    // `PROVINSI[data.provinceId] || data.province || "-"`.
    let prov = match nama_provinsi(prov_id) {
        "-" => prov_nama.to_string(),
        p => p.to_string(),
    };
    let kab = data
        .get("kabupatenKotaId")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    let kec = data
        .get("kecamatanId")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    let uniq = data
        .get("uniqcode")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();

    format!(
        "🪪 *ɴɪᴋ ᴘᴀʀꜱᴇʀ*\n\n\
         - *NIK* → `{}`\n\
         - *Valid* → ✅ Valid\n\
         - *Tanggal Lahir* → {}\n\
         - *Jenis Kelamin* → {} {}\n\
         - *Provinsi* → {}\n\
         - *Kab/Kota* → Kode `{}`\n\
         - *Kecamatan* → Kode `{}`\n\
         - *Kode Unik* → `{}`",
        raw, tanggal, gender_emoji, gender_label, prov, kab, kec, uniq
    )
}

/// JS `parseNik` mengembalikan `{ value, provider }`. Di sini handler panggil
/// HTTP langsung ke obscura dulu, ke nexray sebagai cadangan, lalu
/// normalisasikan bentuk supaya sama dengan JS.

#[plugin(
    name = "nikparser",
    alias = ["nik", "ceknik"],
    category = "tools",
    description = "Parse dan validasi NIK KTP",
    usage = ".nikparser <16 digit NIK>",
    example = ".nikparser 3517072109020003",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn nikparser(ctx: Ctx) -> Reply {
    let nik = ctx.text.replace(|c: char| !c.is_ascii_digit(), "");

    if nik.len() != 16 {
        return Reply::text(kartu_pakai(&ctx.prefix));
    }

    // `m.react("🕕")` — satu reaksi saja, dilewakkan.
    let _ = &nik;

    let api_key = ctx.config().api_keys.obscura.clone();
    let url = if !api_key.is_empty() {
        format!("{}?nik={}", OBSCURA_BASE, nik)
    } else {
        format!("{}?nik={}", NEXRAY_BASE, nik)
    };

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let data = match res.json() {
        Some(v) => v,
        None => {
            return Reply::text(
                "🪪 *ɴɪᴋ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n- NIK yang kamu masukkan tidak valid\n\
                 - Pastikan 16 digit angka benar",
            )
            .with_reaction("❌")
        }
    };

    // Obscura: `res.data.valid`. Nexray: `res.data.status && res.data.result`.
    let valid = data.get("valid").is_some_and(|v| v.as_bool().unwrap_or(false))
        || (data
            .get("status")
            .is_some_and(|v| v.as_bool().unwrap_or(false))
            && data.get("result").is_some());

    if !valid {
        return Reply::text(
            "🪪 *ɴɪᴋ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n- NIK yang kamu masukkan tidak valid\n\
             - Pastikan 16 digit angka benar",
        )
        .with_reaction("❌");
    }

    // Normalisasi field sesuai bentuk yang nikparser.js pakai (data.*).
    let data = if data.get("result").is_some() {
        // Nexray -> bentuk data.*.
        let r = data.get("result").unwrap_or(&Value::Null);
        let kelamin = r
            .get("kelamin")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_uppercase();
        let gender = if kelamin.contains("PEREMPUAN") {
            "wanita"
        } else {
            "pria"
        };
        let lahir = r.get("lahir").and_then(Value::as_str).unwrap_or("");
        let birth_iso = normalisasi_tanggal_nexray(lahir);
        let provinsi = r.get("provinsi").unwrap_or(&Value::Null);
        let kotakab = r.get("kotakab").unwrap_or(&Value::Null);
        let kecamatan = r.get("kecamatan").unwrap_or(&Value::Null);
        serde_json::json!({
            "valid": true,
            "raw": r.get("nik").and_then(Value::as_str).unwrap_or(&nik),
            "birthISO": birth_iso,
            "gender": gender,
            "provinceId": provinsi.get("kode").and_then(Value::as_i64).unwrap_or(0),
            "province": provinsi.get("nama").and_then(Value::as_str).unwrap_or(""),
            "kabupatenKotaId": kotakab.get("kode").and_then(Value::as_str).unwrap_or(""),
            "kecamatanId": kecamatan.get("kode").and_then(Value::as_str).unwrap_or(""),
            "uniqcode": r.get("nomor_urut").and_then(Value::as_str).unwrap_or(""),
        })
    } else {
        data
    };

    // `m.react("✅")` — satu reaksi.
    Reply::text(hasil_teks(&data)).with_reaction("✅")
}

/// Nexray kirim "DD/MM/YYYY", kadang bulan > 12 (perempuan: bulan+12) dan
/// tahun digeser 2000-an. Normalisasi ke ISO, sama seperti JS.
fn normalisasi_tanggal_nexray(lahir: &str) -> String {
    if lahir.is_empty() {
        return String::new();
    }
    let mut parts = lahir.split('/');
    let dd: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let mm: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let yyyy: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);

    let (bulan, tahun) = if mm > 12 { (mm - 12, yyyy + 2000) } else { (mm, yyyy) };

    // Buat Date seperti JS: new Date(year, month-1, dd).toISOString().
    // Jika tidak valid -> kosong.
    // ponytail: JS `new Date(y, m, d)` akan rollover untuk bulan > 12 atau
    // hari di luar jumlah hari bulan itu (mis. "99/99/9999"), sedangkan di
    // sini nilai tak masuk akal dibuang supaya tidak melahirkan tanggal
    // karangan. Naikkan ke normalisasi penuh kalau data nyata pernah keluar
    // dari rentang wajar.
    if dd == 0 || bulan == 0 || bulan > 12 || dd > days_in_bulan(tahun, bulan) {
        return String::new();
    }
    format!("{tahun:04}-{bulan:02}-{dd:02}T00:00:00.000Z")
}

fn days_in_bulan(tahun: i64, bulan: u32) -> u32 {
    match bulan {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if kabisat(tahun) { 29 } else { 28 }
        }
        _ => 0,
    }
}

fn kabisat(tahun: i64) -> bool {
    (tahun % 4 == 0 && tahun % 100 != 0) || tahun % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx(text: &str, http: Arc<Stub>, api_key: &str) -> Ctx {
        let mut args = text.split_whitespace();
        Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "nikparser".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    obscura: api_key.to_string(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn nik_bukan_16_digit_kembalikan_kartu_pakai() {
        for teks in ["", "123", "35170721090200033"] {
            let r = nikparser(ctx(teks, Stub::offline(), ""));
            assert!(text_of(&r).contains("nikparser 3517072109020003"), "{teks}");
            assert_eq!(reaction_of(&r), None, "{teks}");
        }
    }

    #[test]
    fn non_digit_dibuang_dari_text() {
        // JS: m.text.replace(/\D/g, "") -> angka saja.
        let r = nikparser(ctx("351707 2109020003", Stub::offline(), ""));
        assert!(text_of(&r).contains("3517072109020003") || text_of(&r).contains("nikparser"));
    }

    #[test]
    fn jaringan_gagal_kembalikan_pesan_error_dan_reaksi_sil() {
        let r = nikparser(ctx("3517072109020003", Stub::offline(), ""));
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.nikparser` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn obscura_valid_dihasilkan_hasil() {
        let body = serde_json::json!({
            "valid": true,
            "raw": "3517072109020003",
            "birthISO": "2009-09-21",
            "gender": "pria",
            "provinceId": 35,
            "province": "Jawa Timur",
            "kabupatenKotaId": "3507",
            "kecamatanId": "350707",
            "uniqcode": "0003"
        });
        let http = Stub::body(body.clone());
        let r = nikparser(ctx("3517072109020003", http.clone(), "KUNCI"));
        assert_eq!(reaction_of(&r), Some("✅"));
        let teks = text_of(&r);
        assert!(teks.contains("`3517072109020003`"));
        assert!(teks.contains("✅ Valid"));
        assert!(teks.contains("Jenis Kelamin"));
        assert!(teks.contains("Provinsi"));
        assert!(teks.contains("Kab/Kota"));
        assert!(teks.contains("Kecamatan"));
        assert!(teks.contains("Kode Unik"));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &[format!("{OBSCURA_BASE}?nik=3517072109020003")]
        );
    }

    #[test]
    fn nexray_digunakan_saat_obscura_kosong_dan_dinormalisasi() {
        // API key kosong -> pakai nexray, field result.*.
        let body = serde_json::json!({
            "status": true,
            "result": {
                "nik": "3517072109020003",
                "lahir": "21/09/1989",
                "kelamin": "LAKI-LAKI",
                "provinsi": { "kode": 35, "nama": "Jawa Timur" },
                "kotakab": { "kode": "3507" },
                "kecamatan": { "kode": "350707" },
                "nomor_urut": "0003"
            }
        });
        let http = Stub::body(body.clone());
        let r = nikparser(ctx("3517072109020003", http.clone(), ""));
        assert_eq!(reaction_of(&r), Some("✅"));
        let teks = text_of(&r);
        assert!(teks.contains("♂️ Pria"));
        assert!(teks.contains("Jawa Timur"));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &[format!("{NEXRAY_BASE}?nik=3517072109020003")]
        );
    }

    #[test]
    fn valid_false_atau_status_salah_ditolak() {
        for body in [
            serde_json::json!({ "valid": false }),
            serde_json::json!({ "status": false, "result": { "nik": "x" } }),
            serde_json::json!({}),
            Value::Null,
        ] {
            let r = nikparser(ctx("3517072109020003", Stub::body(body.clone()), ""));
            assert!(text_of(&r).contains("tidak valid"), "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn format_tanggal_id_senin_21_september_2009() {
        // 2009-09-21 adalah Senin.
        assert_eq!(format_tanggal_id("2009-09-21"), "Senin, 21 September 2009");
    }

    #[test]
    fn normalisasi_nexray_bulan_lewat_12_digeser_2000() {
        // Perempuan: bulan 21 -> 09, dan JS menambah 2000 ke tahun: 1989 -> 3989.
        assert_eq!(normalisasi_tanggal_nexray("21/21/1989"), "3989-09-21T00:00:00.000Z");
        assert_eq!(normalisasi_tanggal_nexray("05/05/2000"), "2000-05-05T00:00:00.000Z");
        assert_eq!(normalisasi_tanggal_nexray(""), "");
        assert_eq!(normalisasi_tanggal_nexray("99/99/9999"), "");
    }

    #[test]
    fn nama_provinsi_tidak_dikenal_jadi_tanda_hubung() {
        assert_eq!(nama_provinsi(999), "-");
        assert_eq!(nama_provinsi(31), "DKI Jakarta");
    }
}