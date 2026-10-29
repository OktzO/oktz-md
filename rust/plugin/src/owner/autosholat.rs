//! `plugins/owner/autosholat.js` — handler pengingat waktu sholat.
//!
//! Yang dip-port hanya `handler`. Ekspor kedua `runAutoSholat`
//! (`autosholat.js:181`) adalah penjadwal sisi server — `activeCronJobs`,
//! `sock.groupSettingUpdate`, `setTimeout` — bukan handler plugin, dan tidak
//! punya padanan di API plugin.
//!
//! `ponytail:`
//!
//! 1. `saluranCtx()` tidak ada di `Reply`; tidak dipakai handler ini.
//! 2. Reaksi hilang sebagian di cabang `kota`: JS bereaksi `🔍` sebelum
//!    request lalu `📍` sesudahnya (`autosholat.js:158,168`), sedangkan
//!    `Reply::Reaction` hanya bisa membawa satu emoji. Yang dipasang adalah
//!    reaksi terakhir — satu-satunya yang masih terlihat setelah request
//!    selesai, karena bereaksi dua kali pada pesan yang sama menimpa.
//! 3. Cache `NodeCache` 24 jam di `sholat-api.js` tidak ikut; `ctx.http` tidak
//!    punya lapisan cache dan itu tidak mengubah balasannya.

use oktz_plugin_api::{Ctx, HttpError, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::{
    jakarta_parts, js_truthy, js_truthy_value, now_ms, pad2, parse_int,
};
use crate::download::pixeldraindl::encode_uri_component;

/// `sholat-api.js:3`.
const BASE_URL: &str = "https://api.myquran.com/v2/sholat";

/// `extractPrayerTimes` (`sholat-api.js:50-62`). Urutannya penting: loop
/// status memakai `Object.entries(times)` (`autosholat.js:61`), jadi baris
/// jadwal harus keluar dalam urutan objek itu, bukan alphabetic.
///
/// Key hasil dan field sumber sama-sama nama yang sama untuk kedelapan
/// waktu sholat, jadi satu daftar cukup untuk dua arah.
const WAKTU: [&str; 8] = [
    "imsak", "subuh", "terbit", "dhuha", "dzuhur", "ashar", "maghrib", "isya",
];

/// `${value}` di JS untuk nilai yang datang dari store atau API.
///
/// Beda dari `Display` Rust untuk `null` (`"null"`, bukan string kosong) dan
/// untuk field yang benar-benar tidak ada (`"undefined"`), dan `extractPrayerTimes`
/// bisa menyimpan waktu `"-"` yang perlu melewati utuh.
fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

/// `groupSetting(key, fallback)` (`autosholat.js:32-33`).
///
/// Di grup nilai grup menang kalau field-nya ADA (`!== undefined`) — termasuk
/// `null`, yang tetap falsy nanti. Di private selalu jatuh ke fallback.
fn group_setting<'a>(ctx: &'a Ctx, key: &str) -> Option<&'a Value> {
    if ctx.is_group {
        ctx.group_field(key)
    } else {
        None
    }
}

/// `groupSetting(key, database.setting(key))` — nilai grup, lalu global.
fn setting_or_group<'a>(ctx: &'a Ctx, key: &str) -> Option<&'a Value> {
    group_setting(ctx, key).or_else(|| ctx.setting(key))
}

/// `writeSetting` (`autosholat.js:87-95`): di grup tulis ke store grup, di
/// private ke settings global.
fn write_setting(ctx: &Ctx, key: &str, value: Value) -> Mutation {
    if ctx.is_group {
        Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ key: value }),
        }
    } else {
        Mutation::SetSetting {
            key: key.into(),
            value,
        }
    }
}

/// `|| { id: "1301", nama: "KOTA JAKARTA" }` (`autosholat.js:55`).
fn default_kota() -> Value {
    serde_json::json!({ "id": "1301", "nama": "KOTA JAKARTA" })
}

/// Fallback cabang `on` (`autosholat.js:103`) TIDAK punya `id` — objeknya
/// terpisah dari [`default_kota`], jadi tidak boleh dipakai di sini.
fn kota_for_on(ctx: &Ctx) -> Value {
    match setting_or_group(ctx, "autoSholatKota") {
        Some(v) if js_truthy(Some(v)) => v.clone(),
        _ => serde_json::json!({ "nama": "KOTA JAKARTA" }),
    }
}

fn kota_for_status(ctx: &Ctx) -> Value {
    match setting_or_group(ctx, "autoSholatKota") {
        Some(v) if js_truthy(Some(v)) => v.clone(),
        _ => default_kota(),
    }
}

/// `getTodaySchedule` (`sholat-api.js:32-42`) memakai tanggal Asia/Jakarta,
/// bukan tanggal lokal mesin.
fn jadwal_url(kota_id: &str, (year, month, day): (i64, u32, u32)) -> String {
    format!("{BASE_URL}/jadwal/{kota_id}/{year}/{}/{}", pad2(month), pad2(day))
}

/// `nama.charAt(0).toUpperCase() + nama.slice(1)`.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Loop `Object.entries(times)` (`autosholat.js:61-63`) dengan
/// `j.jadwal || j` dan `j.<waktu> || '-'` (`sholat-api.js:50-62`).
fn render_jadwal(jadwal_data: &Value) -> String {
    let j = match jadwal_data.get("jadwal") {
        Some(v) if js_truthy_value(v) => v,
        _ => jadwal_data,
    };
    let mut out = String::new();
    for key in WAKTU {
        let value = j.get(key);
        let waktu = if js_truthy(value) {
            interp(value)
        } else {
            "-".to_string()
        };
        let label = capitalize(key);
        out.push_str(&format!("- **{label}**: {waktu}\n"));
    }
    out
}

/// `getTodaySchedule` + `extractPrayerTimes` digabung.
///
/// `None` = cabang `throw` di `sholat-api.js:47` (request gagal, atau
/// `status`/`data` kosong), yang ditangkap `catch` di `autosholat.js:64`.
fn jadwal_hari_ini(ctx: &Ctx, kota_id: &str) -> Option<String> {
    let (year, month, day, ..) = jakarta_parts(now_ms());
    let body = ctx.http.get(&jadwal_url(kota_id, (year, month, day))).ok()?.json()?;
    if !js_truthy(body.get("status")) {
        return None;
    }
    let jadwal = body.get("data").filter(|d| js_truthy(Some(d)))?;
    Some(render_jadwal(jadwal))
}

/// `searchKota` (`sholat-api.js:6-18`).
///
/// `Err` = axios melempar (timeout, 4xx/5xx) → ditangkap `catch` di
/// `autosholat.js:173`. `Ok(None)` = 200 tapi `status` falsy atau `data`
/// kosong → cabang "tidak dapat kutemukan".
fn search_kota(ctx: &Ctx, query: &str) -> Result<Option<Value>, HttpError> {
    let url = format!("{BASE_URL}/kota/cari/{}", encode_uri_component(query));
    let res = ctx.http.get(&url)?;
    // axios `silentJSONParsing` tidak melempar saat body bukan JSON — `data`
    // tetap string apa adanya, jadi `data?.status` falsy dan hasilnya `null`.
    let Some(body) = res.json() else {
        return Ok(None);
    };
    if !js_truthy(body.get("status")) {
        return Ok(None);
    }
    Ok(body
        .get("data")
        .and_then(Value::as_array)
        .and_then(|list| list.first())
        .cloned())
}

fn status_reply(ctx: &Ctx) -> String {
    let status = if js_truthy(setting_or_group(ctx, "autoSholat")) {
        "Aktif ✅"
    } else {
        "Nonaktif ❌"
    };
    let close = if js_truthy(setting_or_group(ctx, "autoSholatCloseGroup")) {
        "Ya ✅"
    } else {
        "Tidak ❌"
    };
    // `|| 5` — angka 0 yang tersimpan tetap jadi 5 menit, bukan 0.
    let duration = match setting_or_group(ctx, "autoSholatDuration") {
        Some(v) if js_truthy(Some(v)) => interp(Some(v)),
        _ => "5".to_string(),
    };
    let kota = kota_for_status(ctx);
    let jadwal = jadwal_hari_ini(ctx, &interp(kota.get("id")))
        .unwrap_or_else(|| "- Gagal memuat jadwal dari MyQuran\n".to_string());

    format!(
        "🕌 **Auto Sholat - Sistem Pengingat Waktu Beribadah**\n\n\
         Sistem saat ini telah diatur untuk membantu kamu dan para anggota grup mengingat waktu beribadah secara otomatis. Berikut adalah pengaturan yang sedang berjalan:\n\n\
         - **Status Pengingat**: {status}\n\
         - **Penutupan Grup Otomatis**: {close}\n\
         - **Durasi Penutupan**: {duration} menit\n\
         - **Lokasi Pengingat Saat Ini**: {nama}\n\n\
         **Jadwal Sholat Hari Ini:**\n\
         {jadwal}\n\
         **Panduan Pengaturan Fitur:**\n\
         - Ketik `{p}autosholat on` untuk mengaktifkan sistem pengingat.\n\
         - Ketik `{p}autosholat off` untuk mematikan sistem pengingat.\n\
         - Ketik `{p}autosholat close on` atau `off` untuk menyalakan/mematikan fitur tutup grup otomatis.\n\
         - Ketik `{p}autosholat duration <angka>` untuk menentukan berapa lama grup akan ditutup (dalam menit).\n\
         - Ketik `{p}autosholat kota <nama daerah>` untuk menyinkronkan waktu sholat dengan daerah yang kamu pilih.\n\n\
         _Semua jadwal diambil secara presisi dan langsung dari pusat data MyQuran API._",
        nama = interp(kota.get("nama")),
        p = ctx.prefix
    )
}

#[plugin(
    name = "autosholat",
    alias = ["autoadzan"],
    category = "owner",
    description = "Toggle pengingat waktu sholat otomatis dengan audio adzan dan tutup grup",
    usage = ".autosholat on/off/status/kota <nama>",
    example = ".autosholat on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autosholat(ctx: Ctx) -> Reply {
    let args = ctx.args.first().map(|a| a.to_lowercase());

    // `if (!args || args === 'status')`
    if matches!(args.as_deref(), None | Some("status")) {
        return Reply::text(status_reply(&ctx));
    }

    match args.as_deref().unwrap_or_default() {
        "on" => Reply::text(format!(
            "✅ **Sistem Pengingat Sholat Berhasil Diaktifkan!**\n\n\
             Mulai sekarang, aku akan mengirimkan pesan pemberitahuan beserta rekaman audio adzan tepat saat waktu sholat tiba. Seluruh informasi disesuaikan dengan zona waktu di **{}** ya!",
            interp(kota_for_on(&ctx).get("nama"))
        ))
        .with_reaction("✅")
        .with_mutation(write_setting(&ctx, "autoSholat", serde_json::json!(true))),

        "off" => Reply::text(
            "❌ **Sistem Pengingat Sholat Dinonaktifkan.**\n\n\
             Baiklah, aku tidak akan lagi menyiarkan jadwal sholat dan memutarkan audio adzan secara otomatis ke grup-grup.",
        )
        .with_reaction("❌")
        .with_mutation(write_setting(&ctx, "autoSholat", serde_json::json!(false))),

        "close" => match ctx.args.get(1).map(|a| a.to_lowercase()).as_deref() {
            Some("on") => Reply::text(
                "🔒 **Fitur Tutup Grup Otomatis Diaktifkan!**\n\n\
                 Saat waktu sholat tiba, aku akan secara otomatis menutup akses obrolan grup agar semuanya bisa fokus beribadah terlebih dahulu. Keren, kan?",
            )
            .with_reaction("🔒")
            .with_mutation(write_setting(&ctx, "autoSholatCloseGroup", serde_json::json!(true))),
            Some("off") => Reply::text(
                "🔓 **Fitur Tutup Grup Otomatis Dimatikan.**\n\n\
                 Sekarang grup tidak akan ditutup saat azan berkumandang, sehingga obrolan bisa terus berjalan tanpa hambatan.",
            )
            .with_reaction("🔓")
            .with_mutation(write_setting(&ctx, "autoSholatCloseGroup", serde_json::json!(false))),
            _ => Reply::text(format!(
                "Oh, maaf. Formatnya sedikit keliru. Silakan gunakan `{p}autosholat close on` atau `{p}autosholat close off`.",
                p = ctx.prefix
            )),
        },

        "duration" => {
            // `parseInt(m.args[1])` — argumen yang bukan angka jadi `NaN` dan
            // ditolak, `parseInt` juga berhenti di karakter non-digit pertama.
            let minutes = ctx.args.get(1).and_then(|a| parse_int(a));
            match minutes {
                Some(d) if (1..=60).contains(&d) => Reply::text(format!(
                    "⏱️ **Durasi Penutupan Grup Telah Diperbarui!**\n\n\
                     Nantinya, akses obrolan di grup akan dikunci selama **{d} menit** berturut-turut pada setiap jadwal sholat sebelum kubuka kembali secara otomatis."
                ))
                .with_reaction("⏱️")
                .with_mutation(write_setting(&ctx, "autoSholatDuration", serde_json::json!(d))),
                _ => Reply::text(
                    "Tolong masukkan angka antara 1 sampai 60 untuk durasi penutupan grup (dalam menit).",
                ),
            }
        }

        "kota" => {
            // `m.args.slice(1).join(' ').trim()` — slicing harus aman untuk
            // args kosong, jadi bukan `ctx.args[1..]`.
            let tail: &[String] = if ctx.args.len() > 1 { &ctx.args[1..] } else { &[] };
            let nama = tail.join(" ").trim().to_string();
            if nama.is_empty() {
                return Reply::text(format!(
                    "Tolong sebutkan nama kotanya juga! Misalnya, `{p}autosholat kota Surabaya`.",
                    p = ctx.prefix
                ));
            }
            match search_kota(&ctx, &nama) {
                Ok(Some(result)) => {
                    let lokasi = result.get("lokasi").cloned().unwrap_or(Value::Null);
                    let id = result.get("id").cloned().unwrap_or(Value::Null);
                    Reply::text(format!(
                        "📍 **Lokasi Pengingat Berhasil Diperbarui!**\n\n\
                         Seluruh jadwal sholat sekarang telah dikalibrasi ulang untuk menyesuaikan dengan wilayah **{}**.",
                        interp(result.get("lokasi"))
                    ))
                    .with_reaction("📍")
                    .with_mutation(write_setting(
                        &ctx,
                        "autoSholatKota",
                        serde_json::json!({ "id": id, "nama": lokasi }),
                    ))
                }
                Ok(None) => Reply::text(format!(
                    "Aduh, aku sudah mencari di database MyQuran tapi nama daerah **{}** tidak dapat kutemukan. Coba nama kota yang lain?",
                    nama
                )),
                Err(_) => Reply::text(crate::common::error_template(&ctx)),
            }
        }

        _ => Reply::text(
            "Perintah yang kamu masukkan kurang tepat. Kamu bisa menggunakan parameter seperti `on`, `off`, `status`, `close`, `duration`, atau `kota`.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRUP: &str = "120363000000000000@g.us";

    fn ctx_for(args: &[&str], is_group: bool) -> Ctx {
        let text = args.join(" ");
        Ctx {
            prefix: ".".into(),
            command: "autosholat".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: if is_group { GRUP.into() } else { "628111@s.whatsapp.net".into() },
            text,
            args: args.iter().map(|a| (*a).to_string()).collect(),
            is_group,
            // `Ctx::default()` memakai `NoHttp`, jadi semua jalur API diuji
            // lewat cabang gagalnya — tidak ada jaringan di test.
            ..Default::default()
        }
    }

    fn run(args: &[&str]) -> Reply {
        (oktz_plugin_api::find("autosholat").unwrap().handler)(ctx_for(args, false))
    }

    fn run_group(args: &[&str], group_data: Value, settings: Value) -> Reply {
        let mut ctx = ctx_for(args, true);
        ctx.group_data = group_data;
        ctx.settings = settings;
        (oktz_plugin_api::find("autosholat").unwrap().handler)(ctx)
    }

    fn run_with(args: &[&str], settings: Value) -> Reply {
        let mut ctx = ctx_for(args, false);
        ctx.settings = settings;
        (oktz_plugin_api::find("autosholat").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn emoji_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    fn setting_of(r: &Reply, key: &str) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key: k, value } => {
                assert_eq!(k, key);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn group_write_of(r: &Reply, key: &str) -> (String, Value) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => (jid.clone(), data.get(key).cloned().expect(key)),
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autosholat").unwrap();
        assert_eq!(p.meta.name, "autosholat");
        assert_eq!(p.meta.alias, ["autoadzan"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Toggle pengingat waktu sholat otomatis dengan audio adzan dan tutup grup"
        );
        assert_eq!(p.meta.usage, ".autosholat on/off/status/kota <nama>");
        assert_eq!(p.meta.example, ".autosholat on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("autoadzan").unwrap().meta.name,
            "autosholat"
        );
    }

    #[test]
    fn no_argument_and_status_share_the_same_branch() {
        for args in [vec![], vec!["status"], vec!["STATUS"]] {
            let r = run(&args);
            assert!(
                text_of(&r).starts_with("🕌 **Auto Sholat - Sistem Pengingat Waktu Beribadah**"),
                "input {args:?} -> {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {args:?}");
        }
    }

    #[test]
    fn an_unreachable_api_falls_back_to_the_failed_schedule_line() {
        // `catch { jadwalText = '- Gagal memuat jadwal dari MyQuran\n' }`
        let r = run(&[]);
        let t = text_of(&r);
        assert!(
            t.contains("- Gagal memuat jadwal dari MyQuran\n\n**Panduan Pengaturan Fitur:**"),
            "{t}"
        );
    }

    #[test]
    fn the_default_schedule_reads_the_hardcoded_jakarta_id() {
        // Tidak ada setting kota → `|| { id: '1301', nama: 'KOTA JAKARTA' }`.
        let r = run(&[]);
        let t = text_of(&r);
        assert!(t.contains("- **Lokasi Pengingat Saat Ini**: KOTA JAKARTA"), "{t}");
    }

    #[test]
    fn the_status_block_keeps_every_section() {
        let r = run(&[]);
        let t = text_of(&r);
        assert!(t.contains("- **Status Pengingat**: Nonaktif ❌"), "{t}");
        assert!(t.contains("- **Penutupan Grup Otomatis**: Tidak ❌"), "{t}");
        assert!(t.contains("- **Durasi Penutupan**: 5 menit"), "{t}");
        assert!(t.contains("**Jadwal Sholat Hari Ini:**"), "{t}");
        assert!(t.contains("- Ketik `.autosholat kota <nama daerah>`"), "{t}");
        assert!(
            t.ends_with("_Semua jadwal diambil secara presisi dan langsung dari pusat data MyQuran API._"),
            "{t}"
        );
    }

    #[test]
    fn the_group_value_wins_over_the_global_setting() {
        let r = run_group(
            &[],
            serde_json::json!({ "autoSholat": true, "autoSholatCloseGroup": true, "autoSholatDuration": 12 }),
            serde_json::json!({ "autoSholat": false }),
        );
        let t = text_of(&r);
        assert!(t.contains("- **Status Pengingat**: Aktif ✅"), "{t}");
        assert!(t.contains("- **Penutupan Grup Otomatis**: Ya ✅"), "{t}");
        assert!(t.contains("- **Durasi Penutupan**: 12 menit"), "{t}");
    }

    #[test]
    fn a_null_group_field_still_wins_because_the_check_is_not_equals_undefined() {
        let r = run_group(
            &[],
            serde_json::json!({ "autoSholat": null }),
            serde_json::json!({ "autoSholat": true }),
        );
        assert!(
            text_of(&r).contains("- **Status Pengingat**: Nonaktif ❌"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn a_zero_duration_falls_back_to_five_because_of_the_or_operator() {
        let r = run_with(&[], serde_json::json!({ "autoSholatDuration": 0 }));
        assert!(text_of(&r).contains("- **Durasi Penutupan**: 5 menit"), "{}", text_of(&r));
    }

    #[test]
    fn a_stored_city_name_replaces_the_default() {
        let r = run_with(&[], serde_json::json!({ "autoSholatKota": { "id": "1216", "nama": "KOTA SURABAYA" } }));
        assert!(
            text_of(&r).contains("- **Lokasi Pengingat Saat Ini**: KOTA SURABAYA"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn on_writes_the_group_flag_and_reacts_with_a_check_mark() {
        let r = run_group(&["on"], Value::Null, serde_json::json!({}));
        assert_eq!(emoji_of(&r), "✅");
        let (jid, value) = group_write_of(&r, "autoSholat");
        assert_eq!(jid, GRUP);
        assert_eq!(value, serde_json::json!(true));
        let t = text_of(&r);
        assert!(t.starts_with("✅ **Sistem Pengingat Sholat Berhasil Diaktifkan!**"), "{t}");
        assert!(t.ends_with("sesuaikan dengan zona waktu di **KOTA JAKARTA** ya!"), "{t}");
    }

    #[test]
    fn on_in_private_writes_the_global_setting() {
        let r = run_with(&["ON"], serde_json::json!({}));
        assert_eq!(emoji_of(&r), "✅");
        assert_eq!(setting_of(&r, "autoSholat"), serde_json::json!(true));
    }

    #[test]
    fn off_writes_false_and_reacts_with_a_cross() {
        let r = run_with(&["off"], serde_json::json!({ "autoSholat": true }));
        assert_eq!(emoji_of(&r), "❌");
        assert_eq!(setting_of(&r, "autoSholat"), serde_json::json!(false));
        assert!(
            text_of(&r)
                .starts_with("❌ **Sistem Pengingat Sholat Dinonaktifkan.**"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn close_on_and_close_off_toggle_the_flag() {
        for (args, emoji, expected) in [
            (vec!["close", "on"], "🔒", serde_json::json!(true)),
            (vec!["close", "off"], "🔓", serde_json::json!(false)),
            (vec!["CLOSE", "ON"], "🔒", serde_json::json!(true)),
        ] {
            let r = run_with(&args, serde_json::json!({}));
            assert_eq!(emoji_of(&r), emoji, "input {args:?}");
            assert_eq!(setting_of(&r, "autoSholatCloseGroup"), expected, "input {args:?}");
        }
    }

    #[test]
    fn close_without_a_known_sub_argument_asks_for_the_format_and_writes_nothing() {
        for args in [
            vec!["close"],
            vec!["close", "maybe"],
            vec!["close", "ON!"],
            vec!["close", "1"],
        ] {
            let r = run_with(&args, serde_json::json!({}));
            assert_eq!(
                text_of(&r),
                "Oh, maaf. Formatnya sedikit keliru. Silakan gunakan `.autosholat close on` atau `.autosholat close off`.",
                "input {args:?}"
            );
            assert!(r.mutations().is_empty(), "input {args:?}");
            match r.without_mutations() {
                Reply::Text { .. } => {}
                other => panic!("input {args:?} harus Text, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn duration_accepts_one_through_sixty() {
        for n in [1, 30, 60] {
            let arg = n.to_string();
            let r = run_with(&["duration", &arg], serde_json::json!({}));
            assert_eq!(emoji_of(&r), "⏱️", "input {n}");
            assert_eq!(setting_of(&r, "autoSholatDuration"), serde_json::json!(n), "input {n}");
            assert!(
                text_of(&r).contains(&format!("selama **{n} menit**")),
                "input {n} -> {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn duration_refuses_anything_outside_the_range_or_unparseable() {
        for bad in [
            vec!["duration"],
            vec!["duration", "abc"],
            vec!["duration", "0"],
            vec!["duration", "61"],
            vec!["duration", "-5"],
            vec!["duration", "  "],
        ] {
            let r = run_with(&bad, serde_json::json!({}));
            assert_eq!(
                text_of(&r),
                "Tolong masukkan angka antara 1 sampai 60 untuk durasi penutupan grup (dalam menit).",
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn duration_stops_at_the_first_non_digit_like_parse_int() {
        // `parseInt('30menit')` = 30 dan `parseInt('1.5')` = 1 — keduanya
        // sah, bukan error.
        let r = run_with(&["duration", "30menit"], serde_json::json!({}));
        assert_eq!(setting_of(&r, "autoSholatDuration"), serde_json::json!(30));
        let r = run_with(&["duration", "1.5"], serde_json::json!({}));
        assert_eq!(setting_of(&r, "autoSholatDuration"), serde_json::json!(1));
    }

    #[test]
    fn kota_without_a_name_asks_for_one() {
        for args in [vec!["kota"], vec!["kota", "   "]] {
            let r = run_with(&args, serde_json::json!({}));
            assert_eq!(
                text_of(&r),
                "Tolong sebutkan nama kotanya juga! Misalnya, `.autosholat kota Surabaya`.",
                "input {args:?}"
            );
            assert!(r.mutations().is_empty(), "input {args:?}");
        }
    }

    #[test]
    fn an_unreachable_kota_search_returns_the_error_template() {
        let r = run_with(&["kota", "Surabaya"], serde_json::json!({}));
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "autosholat"));
        assert!(r.mutations().is_empty());
        match r.without_mutations() {
            Reply::Text { .. } => {}
            other => panic!("harus Text tanpa reaksi, dapat {other:?}"),
        }
    }

    #[test]
    fn an_unknown_parameter_lists_the_valid_ones() {
        for bad in ["nyala", "ON!", "1"] {
            let r = run_with(&[bad], serde_json::json!({}));
            assert!(
                text_of(&r).starts_with("Perintah yang kamu masukkan kurang tepat."),
                "input {bad} -> {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn the_status_flag_is_read_from_settings_when_there_is_no_group() {
        let r = run_with(&[], serde_json::json!({ "autoSholat": true, "autoSholatCloseGroup": true, "autoSholatDuration": 45 }));
        let t = text_of(&r);
        assert!(t.contains("- **Status Pengingat**: Aktif ✅"), "{t}");
        assert!(t.contains("- **Penutupan Grup Otomatis**: Ya ✅"), "{t}");
        assert!(t.contains("- **Durasi Penutupan**: 45 menit"), "{t}");
    }

    // ---- helper murni ----

    #[test]
    fn the_schedule_url_pads_the_month_and_keeps_the_id_as_given() {
        assert_eq!(
            jadwal_url("1301", (2026, 3, 7)),
            "https://api.myquran.com/v2/sholat/jadwal/1301/2026/03/07"
        );
        assert_eq!(
            jadwal_url("1216", (2026, 12, 25)),
            "https://api.myquran.com/v2/sholat/jadwal/1216/2026/12/25"
        );
    }

    #[test]
    fn the_schedule_is_rendered_in_the_declared_order_with_a_capital_first_letter() {
        let out = render_jadwal(&serde_json::json!({
            "jadwal": { "imsak": "04:12 (WIB)", "subuh": "04:22 (WIB)", "isya": "18:31 (WIB)" }
        }));
        assert_eq!(
            out,
            "- **Imsak**: 04:12 (WIB)\n\
             - **Subuh**: 04:22 (WIB)\n\
             - **Terbit**: -\n\
             - **Dhuha**: -\n\
             - **Dzuhur**: -\n\
             - **Ashar**: -\n\
             - **Maghrib**: -\n\
             - **Isya**: 18:31 (WIB)\n"
        );
    }

    #[test]
    fn a_schedule_without_the_jadwal_wrapper_is_read_as_the_schedule_itself() {
        // `jadwalData.jadwal || jadwalData`.
        let out = render_jadwal(&serde_json::json!({ "maghrib": "17:55 (WIB)" }));
        assert!(out.contains("- **Maghrib**: 17:55 (WIB)"), "{out}");
        assert!(out.starts_with("- **Imsak**: -"), "{out}");
    }

    #[test]
    fn a_falsy_time_becomes_a_dash_not_an_empty_cell() {
        let out = render_jadwal(&serde_json::json!({ "jadwal": { "subuh": "", "ashar": "15:10 (WIB)" } }));
        assert!(out.contains("- **Subuh**: -\n"), "{out}");
        assert!(out.contains("- **Ashar**: 15:10 (WIB)\n"), "{out}");
    }

    #[test]
    fn capitalize_uppercases_only_the_first_character() {
        assert_eq!(capitalize("imsak"), "Imsak");
        assert_eq!(capitalize("dzuhur"), "Dzuhur");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn interpolation_mirrors_the_js_template_literal() {
        assert_eq!(interp(None), "undefined");
        assert_eq!(interp(Some(&Value::Null)), "null");
        assert_eq!(interp(Some(&serde_json::json!("KOTA JAKARTA"))), "KOTA JAKARTA");
        assert_eq!(interp(Some(&serde_json::json!(1301))), "1301");
        assert_eq!(interp(Some(&serde_json::json!(true))), "true");
    }

    #[test]
    fn writing_goes_to_the_group_store_only_in_a_group() {
        let di_private = write_setting(&ctx_for(&[], false), "autoSholat", serde_json::json!(true));
        assert!(matches!(di_private, Mutation::SetSetting { .. }));
        let di_grup = write_setting(&ctx_for(&[], true), "autoSholat", serde_json::json!(true));
        match di_grup {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, GRUP);
                assert_eq!(data, serde_json::json!({ "autoSholat": true }));
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }
}