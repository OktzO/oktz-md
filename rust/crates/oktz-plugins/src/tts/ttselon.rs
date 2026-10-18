//! `plugins/tts/ttselon.js` — TTS suara Elon Musk lewat API Emilia.
//!
//! `ponytail:` `m.react('🚀')` lalu `✅`/`❌`/`☢` — satu reaksi terakhir.
//! `ctx.config()` tidak dibaca (API tidak butuh kunci).
//! `sock.sendMessage({ audio, contextInfo: saluranCtx() })` →
//! `Reply::voice_note` (contextInfo/saluran diabaikan, `Reply` tidak bawa
//! contextInfo).
//! `fs` temp + ffmpeg → `ctx.berkas()` + `oktz_plugin_api::ffmpeg`.
//! `te(...)` → `crate::common::error_template(&ctx.prefix, &ctx.command)`.

use oktz_plugin_api::{
    Ctx, HttpCall, HttpError, MediaSource, Reply, plugin,
};
use oktz_plugin_api::ffmpeg;
use serde_json::Value;

use crate::common::error_template;

/// `https://api.emiliabot.my.id/tools/text-to-speech?text=<teks>` (`ttselon.js:58`).
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.emiliabot.my.id/tools/text-to-speech?text={}",
        crate::tools::phisingataubukan::encode_uri_component(text)
    )
}

/// Bantuan saat teks kosong (`ttselon.js:50-52`).
pub(crate) fn bantuan(prefix: &str) -> String {
    format!(
        "🚀 *ᴇʟᴏɴ ᴍᴜsᴋ ᴛᴛs*\n\n> Gunakan: `{prefix}ttselon <text>`"
    )
}

/// Suara Elon tidak ditemukan (`ttselon.js:63-65`).
pub(crate) const SUARA_ERROR: &str = "❌ Elon voice error. Coba TTS lain.";

/// Nama folder temp (`ttselon.js:68`).
const TEMP_DIR: &str = "temp";

/// Prefix berkas temp (`ttselon.js:70-71`).
const TEMP_PREFIX: &str = "tts_";

/// `ctx.berkas()` + ffmpeg: tulis WAV, konversi ke Opus/OGG, baca, bersihkan.
fn konversi_ke_opus(berkas: &dyn oktz_plugin_api::berkas::Berkas, wav: &[u8]) -> Result<Vec<u8>, String> {
    let nama = format!("{TEMP_PREFIX}{}.wav", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis()));
    let wav_path = format!("{TEMP_DIR}/{nama}");
    let opus_path = format!("{TEMP_DIR}/{}.ogg", nama.trim_end_matches(".wav"));

    berkas.buat_folder(TEMP_DIR)?;
    berkas.tulis(&wav_path, wav)?;

    let hasil = ffmpeg::jalankan_ke_berkas(&[
        "-y",
        "-i",
        &wav_path,
        "-c:a",
        "libopus",
        "-b:a",
        "64k",
        "-vbr",
        "on",
        "-compression_level",
        "10",
        "-y",
        &opus_path,
    ]);

    // Bersihkan berkas temp baik sukses maupun gagal.
    let _ = berkas.hapus(&wav_path);
    let _ = berkas.hapus(&opus_path);

    hasil.and_then(|_| berkas.baca(&opus_path))
}

/// Cari suara Elon Musk dari array `result` (`ttselon.js:61`).
fn cari_suara_elon(data: &Value) -> Option<String> {
    let daftar = data.get("result").and_then(Value::as_array)?;
    for item in daftar {
        let elon_ada = item
            .get("elon_musk")
            .is_some_and(|v| crate::common::js_truthy_value(v));
        let error_salah = item
            .get("error")
            .map_or(true, |e| !crate::common::js_truthy_value(e));
        if elon_ada && error_salah {
            return item.get("elon_musk").and_then(Value::as_str).map(|s| s.to_string());
        }
    }
    None
}

#[plugin(
    name = "ttselon",
    alias = ["elontts", "ttselonmusk"],
    category = "tts",
    description = "Text to Speech dengan suara Elon Musk",
    usage = ".ttselon <text>",
    example = ".ttselon To Mars!",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ttselon(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(bantuan(&ctx.prefix));
    }

    let Ok(res) = ctx.http.call(&HttpCall::get(&api_url(text))) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);

    let Some(url) = cari_suara_elon(&data) else {
        return Reply::text(SUARA_ERROR).with_reaction("❌");
    };

    let Ok(audio_res) = ctx.http.call(&HttpCall::get(&url)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let wav = audio_res.body.bytes().to_vec();

    let opus = match konversi_ke_opus(ctx.berkas(), &wav) {
        Ok(b) => b,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
        }
    };

    Reply::voice_note(MediaSource::bytes(opus)).with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        panggilan: Mutex<Vec<String>>,
        balasan: Option<Vec<u8>>,
        gagal: bool,
    }

    impl Stub {
        fn body(bytes: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(bytes),
                ..Default::default()
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                gagal: true,
                ..Default::default()
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            self.panggilan.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.clone().unwrap_or_default()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
    }

    fn berkas_palsu() -> oktz_plugin_api::berkas::BerkasPalsu {
        oktz_plugin_api::berkas::BerkasPalsu::baru()
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ttselon".into(),
            http,
            berkas: Arc::new(berkas_palsu()),
            ..Ctx::default()
        };
        ttselon(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn teks_kosong_menghasilkan_bantuan() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), bantuan("."));
        assert_eq!(reaksi(&r), None);
        assert!(http.panggilan.lock().unwrap().is_empty());
    }

    #[test]
    fn api_url_mengencode_teks() {
        assert_eq!(
            api_url("To Mars!"),
            "https://api.emiliabot.my.id/tools/text-to-speech?text=To%20Mars%21"
        );
    }

    #[test]
    fn api_tanpa_suara_elon_hasilkan_teks_error() {
        let http = Stub::body(
            serde_json::json!({ "result": [{ "elon_musk": null, "error": true }] }).to_string().into_bytes(),
        );
        let r = run(http.clone(), "halo");
        assert_eq!(text_of(&r), SUARA_ERROR);
        assert_eq!(reaksi(&r), Some("❌"));
        let panggilan = http.panggilan.lock().unwrap().clone();
        assert_eq!(panggilan.len(), 1);
        assert!(panggilan[0].starts_with("https://api.emiliabot.my.id/tools/text-to-speech?"));
    }

    #[test]
    fn cari_suara_elon_pilih_item_benar() {
        let data = serde_json::json!({
            "result": [
                { "error": true, "elon_musk": null },
                { "elon_musk": "https://cdn/elon.mp3", "error": false },
            ]
        });
        assert_eq!(
            cari_suara_elon(&data),
            Some("https://cdn/elon.mp3".to_string())
        );
    }

    #[test]
    fn gagal_jaringan_api_menggunakan_error_template() {
        let http = Stub::offline();
        let r = run(http.clone(), "halo");
        assert_eq!(text_of(&r), crate::common::error_template(".", "ttselon"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn konversi_ke_opus_tanpa_ffmpeg_hasilkan_err() {
        let berkas = berkas_palsu();
        let err = konversi_ke_opus(&berkas, b"wav-data").unwrap_err();
        assert!(err.contains("ffmpeg"), "harus menyebut ffmpeg: {err}");
    }
}
