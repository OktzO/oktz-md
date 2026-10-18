//! `plugins/main/donasi.js` — informasi donasi dengan QRIS (nama command:
//! `donasi`).
//!
//! `ponytail:` JS mengirim tombol interaktif (`sendButton`/`interactiveButtons`)
//! yang tidak ada dalam enum `Reply`; kirimannya disederhanakan jadi teks, atau
//! gambar QRIS dengan teks sebagai caption kalau `qrisUrl` terisi. Footer dan
//! `contextInfo` (newsletter) tidak dapat dicerminkan di API plugin.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use oktz_plugin_api::HttpError;

const DEFAULT_SALURAN_ID: &str = "120363400911374213@newsletter";

/// Huruf pertama di-kecilkan lalu hurur depan di-capitalize (`dana` → `Dana`).
fn nama_pay(pay: &serde_json::Value) -> String {
    let nama = pay.get("name").and_then(serde_json::Value::as_str).unwrap_or("");
    let kecil = nama.to_lowercase();
    let mut chars = kecil.chars();
    match chars.next() {
        Some(pertama) => pertama.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Ikon per nama link (`saweria` → ☕, `trakteer` → 🍵, `paypal` → 💰).
fn ikon_link(nama: &str) -> &'static str {
    match nama.to_lowercase().as_str() {
        "saweria" => "☕",
        "trakteer" => "🍵",
        "paypal" => "💰",
        _ => "🔗",
    }
}

#[plugin(
    name = "donasi",
    alias = ["donate", "donation", "support", "saweria", "trakteer"],
    category = "main",
    description = "Informasi donasi untuk mendukung bot dengan QRIS",
    usage = ".donasi",
    example = ".donasi",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn donasi(ctx: Ctx) -> Reply {
    let config = ctx.config();
    let bot_name = config.bot.name.as_deref().unwrap_or("Bot");
    let saluran_id = config.saluran.id.as_deref().unwrap_or(DEFAULT_SALURAN_ID);
    let saluran_name = config.saluran.name.as_deref().unwrap_or(bot_name);

    let donasi = &config.donasi;
    let payments = &donasi.payment;
    let links = &donasi.links;
    let qris_url = &donasi.qris;
    let benefits = if donasi.benefits.is_empty() {
        vec![
            "Mendukung development",
            "Server lebih stabil",
            "Fitur baru lebih cepat",
            "Priority support",
        ]
    } else {
        donasi.benefits.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
    };

    let mut text = format!("DONASI KE OWNER {bot_name} 🙏\n");

    if !payments.is_empty() || !links.is_empty() {
        text.push_str("Pembayaran\n");
        for pay in payments {
            let nama = nama_pay(pay);
            let nomor = pay.get("number").and_then(serde_json::Value::as_str).unwrap_or("");
            let holder = pay.get("holder").and_then(serde_json::Value::as_str).unwrap_or("");
            text.push_str(&format!("🏦 *{nama}*\n"));
            text.push_str(&format!("◦ {nomor} (a/n {holder})\n"));
        }
        for link in links {
            let nama = link.get("name").and_then(serde_json::Value::as_str).unwrap_or("");
            let url = link.get("url").and_then(serde_json::Value::as_str).unwrap_or("");
            text.push_str(&format!("{} *{nama}*\n", ikon_link(nama)));
            text.push_str(&format!("{url}\n"));
        }
    } else {
        text.push_str("╭┈┈⬡「 💳 *ᴘᴀʏᴍᴇɴᴛ* 」\n");
        text.push_str("┃\n");
        text.push_str("┃ > Belum dikonfigurasi\n");
        text.push_str("┃ > Edit config.donasi\n");
        text.push_str("┃\n");
        text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    }

    text.push_str("🎁 *ʙᴇɴᴇꜰɪᴛ*\n");
    for benefit in &benefits {
        text.push_str(&format!("◦ {benefit}\n"));
    }
    text.push('\n');

    text.push_str("_Donasi berapapun sangat berharga_\n");
    let owner_nomor = config.owner.number.first().map(String::as_str).unwrap_or("owner");
    text.push_str(&format!("Contact: @{owner_nomor}\n"));

    // QRIS image (bytes) sebagai media, teks sebagai caption.
    if !qris_url.is_empty() {
        match ctx.http.get(qris_url) {
            Ok(res) => {
                let data = res.body.bytes().to_vec();
                if !data.is_empty() {
                    return Reply::image(MediaSource::bytes(data), text.trim().to_string());
                }
            }
            Err(_) => {}
        }
    }

    Reply::text(text.trim().to_string())
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
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn bytes(b: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(b),
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

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
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

    fn config_ku() -> oktz_plugin_api::config::Runtime {
        oktz_plugin_api::config::Runtime {
            info: oktz_plugin_api::config::Info {
                website: String::new(),
                grupwa: String::new(),
            },
            owner: oktz_plugin_api::config::Owner {
                name: "Oktz".into(),
                number: vec!["6285608953677".into()],
            },
            bot: oktz_plugin_api::config::BotInfo {
                name: "OktzBot".into(),
                version: String::new(),
                developer: String::new(),
                website: String::new(),
            },
            assets: Default::default(),
            sticker: Default::default(),
            payment: Default::default(),
            donasi: oktz_plugin_api::config::Donasi {
                payment: vec![
                    oktz_plugin_api::config::PaymentMethod {
                        name: "Dana".into(),
                        number: "0812345678".into(),
                        holder: "Budi".into(),
                    },
                    oktz_plugin_api::config::PaymentMethod {
                        name: "GOPAY".into(),
                        number: "08999999".into(),
                        holder: "Ani".into(),
                    },
                ],
                links: vec![
                    oktz_plugin_api::config::DonasiLink {
                        name: "Saweria".into(),
                        url: "saweria.co/oktzz".into(),
                    },
                    oktz_plugin_api::config::DonasiLink {
                        name: "Trakteer".into(),
                        url: "trakteer.id/oktzz".into(),
                    },
                ],
                benefits: vec!["Mendukung development".into()],
                qris: "https://imgdrop.web.id/KodpV.webp".into(),
            },
            saluran: oktz_plugin_api::config::Saluran {
                id: "120363411617217441@newsletter".into(),
                name: "Join saluran resmi Oktz".into(),
                link: "https://whatsapp.com/channel/x".into(),
            },
            api_keys: Default::default(),
        }
    }

    fn run(http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "donasi".into(),
            http,
            config: Arc::new(config_ku()),
            ..Ctx::default()
        };
        donasi(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn config_digunakan_kosong_saat_tidak_terdaftar() {
        let r = run(Stub::offline());
        let t = text_of(&r);
        assert!(t.starts_with("DONASI KE OWNER OktzBot 🙏"), "{t}");
        assert!(t.contains("Pembayaran"), "{t}");
        assert!(t.contains("🏦 *Dana*\n"), "{t}");
        assert!(t.contains("◦ 0812345678 (a/n Budi)"), "{t}");
        assert!(t.contains("🏦 *Gopay*\n"), "{t}");
        assert!(t.contains("☕ *Saweria*\n"), "{t}");
        assert!(t.contains("saweria.co/oktzz"), "{t}");
        assert!(t.contains("🎁 *ʙᴇɴᴇꜰɪᴛ*\n"), "{t}");
        assert!(t.contains("◦ Mendukung development"), "{t}");
        assert!(t.contains("Contact: @6285608953677"), "{t}");
    }

    #[test]
    fn qris_url_digunakan_sebagai_media_bytes() {
        let r = run(Stub::bytes(b"QRIS-IMAGE-BYTES".to_vec()));
        let Reply::Image { media, caption, .. } = r.without_mutations() else {
            panic!("harus image, dapat {r:?}");
        };
        assert!(
            matches!(media, Some(oktz_plugin_api::MediaSource::Bytes { .. })),
            "harus bytes: {media:?}"
        );
        assert!(caption.contains("DONASI KE OWNER"));
        let seen = r; // tidak perlu
        let _ = seen;
    }

    #[test]
    fn qris_gagal_digagal_dan_teks_dikirim() {
        let r = run(Stub::offline());
        assert!(text_of(r).starts_with("DONASI KE OWNER"));
    }

    #[test]
    fn nama_pay_capitalize_huruf_pertama() {
        assert_eq!(nama_pay(&serde_json::json!({ "name": "dana" })), "Dana");
        assert_eq!(nama_pay(&serde_json::json!({ "name": "GOPAY" })), "Gopay");
        assert_eq!(nama_pay(&serde_json::json!({ "name": "OVO" })), "Ovo");
        assert_eq!(nama_pay(&serde_json::json!({})), "");
    }

    #[test]
    fn ikon_link_mirip_js() {
        assert_eq!(ikon_link("Saweria"), "☕");
        assert_eq!(ikon_link("TRAkteer"), "🍵");
        assert_eq!(ikon_link("PayPal"), "💰");
        assert_eq!(ikon_link("Lainnya"), "🔗");
    }

    #[test]
    fn benefits_default_dipakai_kosong() {
        // config_ku sudah punya benefits satu entri; coba kosongkan.
        let mut cfg = config_ku();
        cfg.donasi.benefits.clear();
        let http = Stub::offline();
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "donasi".into(),
            http,
            config: Arc::new(cfg),
            ..Ctx::default()
        };
        let r = donasi(ctx);
        let t = text_of(&r);
        assert!(t.contains("◦ Mendukung development"), "{t}");
        assert!(t.contains("◦ Server lebih stabil"), "{t}");
        assert!(t.contains("◦ Priority support"), "{t}");
    }

    #[test]
    fn pembayaran_kosong_dan_link_kosong_menampilkan_box() {
        let mut cfg = config_ku();
        cfg.donasi.payment.clear();
        cfg.donasi.links.clear();
        let http = Stub::offline();
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "donasi".into(),
            http,
            config: Arc::new(cfg),
            ..Ctx::default()
        };
        let r = donasi(ctx);
        let t = text_of(&r);
        assert!(t.contains("╭┈┈⬡「 💳 *ᴘᴀʏᴍᴇɴᴛ* 」"), "{t}");
        assert!(t.contains("> Belum dikonfigurasi"), "{t}");
    }
}
