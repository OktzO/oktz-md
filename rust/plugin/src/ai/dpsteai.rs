//! `plugins/ai/dpsteai.js` — chat AI dipastebin dengan sesi per pengguna.
//!
//! Sesi disimpan di `database/ai_chat/dpsteaai.json` (sama seperti JS) lewat
//! `ctx.berkas`, jadi plugin ini tidak menyentuh disk sendiri.

use oktz_plugin_api::{Ctx, HttpCall, Reply, plugin};
use rand::Rng as _;
use serde_json::{Value, json};

const FILE_SESI: &str = "database/ai_chat/dpsteaai.json";
const HOST: &str = "dipastebin.web.id";

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Kartu pakai (`dpsteai.js:95-103`).
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🤖 *DPSTE AI*\n\n\
         > Halo! Aku asisten cerdas dari dipastebin.\n\n\
         *Cara pakai:*\n\
         > `{prefix}dpsteai halo`\n\
         > `{prefix}dpsteai reset` (hapus memori chat)"
    )
}

/// `generateSessionId()` — `dpste_ai_` + 22 karakter `[a-z0-9]`.
fn sesi_baru() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    let acak: String = (0..22)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect();
    format!("dpste_ai_{acak}")
}

/// `httpsPost(pathname, body)` (`dpsteai.js:59-83`).
///
/// Balikan JSON; body non-JSON dibungkus `{"raw": teks}` persis JS.
fn https_post(ctx: &Ctx, path: &str, body: &Value) -> Result<Value, String> {
    let res = ctx
        .http
        .call(
            &HttpCall::post(format!("https://{HOST}{path}"))
                .header("User-Agent", UA)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/plain, */*")
                .header("Origin", format!("https://{HOST}"))
                .header("Referer", format!("https://{HOST}/"))
                .json(body),
        )
        .map_err(|e| e.message)?;
    Ok(res.json().unwrap_or_else(|| json!({ "raw": res.text() })))
}

/// `loadSessions()` — berkas rusak/tidak ada berarti objek kosong.
fn muat_sesi(ctx: &Ctx) -> Value {
    match ctx
        .berkas()
        .baca_teks(FILE_SESI)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        Some(v) if v.is_object() => v,
        _ => json!({}),
    }
}

/// `saveSessions()` — `JSON.stringify(data, null, 2)`, kegagalan diabaikan.
fn simpan_sesi(ctx: &Ctx, sesi: &Value) {
    let teks = serde_json::to_string_pretty(sesi).unwrap_or_default();
    let _ = ctx.berkas().tulis_teks(FILE_SESI, &teks);
}

/// Nilai sesi yang "truthy" versi JS (`sessions[userJid]`).
fn teks_dari(v: &Value) -> Option<String> {
    crate::common::js_truthy(Some(v)).then(|| v.as_str().unwrap_or_default().to_string())
}

#[plugin(
    name = "dpsteai",
    alias = ["dpste", "dipasteai"],
    category = "ai",
    description = "Chat dengan AI dari dipastebin.web.id (mendukung session per-user)",
    usage = ".dpsteai <pertanyaan> atau .dpsteai reset (untuk hapus sesi)",
    example = ".dpsteai halo",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dpsteai(ctx: Ctx) -> Reply {
    let teks = ctx.text.trim().to_string();

    if teks.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let jam = Reply::None.with_reaction("🕕");
    let mut sesi = muat_sesi(&ctx);
    let user = ctx.sender.clone();
    let sesi_user = teks_dari(sesi.get(&user).unwrap_or(&Value::Null));

    if teks.to_lowercase() == "reset" {
        let Some(id) = sesi_user else {
            return Reply::sequence(vec![
                jam,
                Reply::None.with_reaction("❌"),
                Reply::text(
                    "❌ Kamu belum memiliki sesi chat dengan AI ini. Mulailah ngobrol terlebih dahulu!",
                ),
            ]);
        };
        let _ = https_post(&ctx, "/api/ai/clear", &json!({ "sessionId": id }));
        if let Some(obj) = sesi.as_object_mut() {
            obj.remove(&user);
        }
        simpan_sesi(&ctx, &sesi);
        return Reply::sequence(vec![
            jam,
            Reply::None.with_reaction("✅"),
            Reply::text(
                "✅ *Sesi chat berhasil direset!* Ingatan AI tentang obrolan kita sudah dihapus sepenuhnya.",
            ),
        ]);
    }

    let id_awal = sesi_user.unwrap_or_else(sesi_baru);

    let hasil = match https_post(
        &ctx,
        "/api/ai/chat",
        &json!({ "message": teks, "sessionId": id_awal, "clientUser": Value::Null }),
    ) {
        Ok(v) => v,
        Err(_) => {
            return Reply::sequence(vec![
                jam,
                Reply::None.with_reaction("❌"),
                Reply::text(crate::common::error_template(&ctx)),
            ]);
        }
    };

    let sukses = crate::common::js_truthy(hasil.get("success"));
    let balasan = hasil.get("reply").and_then(Value::as_str);
    if !sukses && balasan.is_none() {
        let alasan = hasil
            .get("error")
            .and_then(Value::as_str)
            .or_else(|| hasil.get("raw").and_then(Value::as_str))
            .unwrap_or("Tidak ada balasan");
        return Reply::sequence(vec![
            jam,
            Reply::None.with_reaction("❌"),
            Reply::text(format!("❌ Error dari server AI: {alasan}")),
        ]);
    }

    let id_aktif = hasil
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or(&id_awal)
        .to_string();
    if sesi.get(&user).and_then(Value::as_str) != Some(id_aktif.as_str()) {
        if let Some(obj) = sesi.as_object_mut() {
            obj.insert(user.clone(), json!(id_aktif));
        }
        simpan_sesi(&ctx, &sesi);
    }

    let centang = Reply::None.with_reaction("✅");
    match balasan {
        // `m.reply(undefined)` di JS tidak mengirim apa pun, hanya reaksi.
        None => Reply::sequence(vec![jam, centang]),
        Some(isi) => Reply::sequence(vec![jam, centang, Reply::text(isi)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::uji::{HttpRangkaian, reaksi, teks};
    use oktz_plugin_api::berkas::{Berkas, BerkasPalsu};
    use std::sync::Arc;

    fn ctx_of(berkas: Arc<BerkasPalsu>, http: Arc<HttpRangkaian>, text: &str) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "dpsteai".into(),
            text: text.into(),
            berkas,
            http,
            ..Ctx::default()
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("dpsteai").unwrap();
        assert_eq!(p.meta.alias, ["dpste", "dipasteai"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.usage, ".dpsteai <pertanyaan> atau .dpsteai reset (untuk hapus sesi)");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_teks_membalas_kartu_pakai_tanpa_reaksi() {
        let r = dpsteai(ctx_of(Arc::new(BerkasPalsu::baru()), HttpRangkaian::offline(), ""));
        assert_eq!(teks(&r), help("."));
        assert!(reaksi(&r).is_empty());
    }

    #[test]
    fn reset_tanpa_sesi_lama_membalas_belum_punya_sesi() {
        let r = dpsteai(ctx_of(
            Arc::new(BerkasPalsu::baru()),
            HttpRangkaian::offline(),
            "reset",
        ));
        assert_eq!(reaksi(&r), ["🕕", "❌"]);
        assert!(teks(&r).starts_with("❌ Kamu belum memiliki sesi chat dengan AI ini."));
    }

    #[test]
    fn jawaban_server_disimpan_dan_sesi_baru_ditulis_ke_berkas() {
        let b = Arc::new(BerkasPalsu::baru());
        let http = HttpRangkaian::urutan(vec![
            r#"{"success":true,"sessionId":"dpste_ai_baru","reply":"Halo juga!"}"#.to_string(),
        ]);
        let r = dpsteai(ctx_of(b.clone(), http.clone(), "halo"));
        assert_eq!(reaksi(&r), ["🕕", "✅"]);
        assert_eq!(teks(&r), "Halo juga!");
        // Permintaan membawa pesan + sessionId 22 karakter.
        assert_eq!(http.terlihat().len(), 1);
        assert_eq!(http.terlihat()[0], "https://dipastebin.web.id/api/ai/chat");
        // Sesi tersimpan ke berkas yang sama dengan JS, pretty-print 2 spasi.
        let isi = b.baca_teks(FILE_SESI).expect("sesi harus tersimpan");
        assert!(isi.contains("\"628111@s.whatsapp.net\": \"dpste_ai_baru\""), "{isi}");
        assert!(isi.contains("\n  \""), "harus indentasi 2 spasi: {isi}");
    }

    #[test]
    fn galat_server_diteruskan_apa_adanya() {
        let r = dpsteai(ctx_of(
            Arc::new(BerkasPalsu::baru()),
            HttpRangkaian::urutan(vec![r#"{"error":"kuota habis"}"#.to_string()]),
            "halo",
        ));
        assert_eq!(reaksi(&r), ["🕕", "❌"]);
        assert_eq!(teks(&r), "❌ Error dari server AI: kuota habis");
    }

    #[test]
    fn body_non_json_dipakai_sebagai_alasan_mentah() {
        let r = dpsteai(ctx_of(
            Arc::new(BerkasPalsu::baru()),
            HttpRangkaian::urutan(vec!["502 Bad Gateway".to_string()]),
            "halo",
        ));
        assert_eq!(teks(&r), "❌ Error dari server AI: 502 Bad Gateway");
    }

    #[test]
    fn id_sesi_berpola_dpste_ai_22_karakter() {
        let id = sesi_baru();
        assert!(id.starts_with("dpste_ai_"));
        assert_eq!(id.len(), "dpste_ai_".len() + 22);
        assert!(
            id["dpste_ai_".len()..]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }
}
