//! `plugins/tools/tempmail.js` — buat email sementara & cek inbox.
//!
//! `ponytail:` `TempMailCreate()` / `TempMailInbox()` dipanggil lewat crate
//! `oktz_scraper::tempmail` (bukan menyalin logika scraper). Simpan email ke
//! store user lewat `Mutation::SetUser`. `m.react("🕕")` / `✅` / `❌` /
//! `☢` / `📭` — satu reaksi saja yang dibawa (`✅` atau error).
//!
//! Catatan: baris `pub mod tempmail;` perlu ditambah ke `tools/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, HttpCall, HttpError, Mutation, Reply, plugin};
use oktz_scraper::tempmail::{temp_mail_create, temp_mail_inbox};
use serde_json::{json, Value};
use oktz_plugin_api::HttpGet;

#[plugin(
    name = "tempmail",
    alias = ["tmpmail", "tmp", "trashmail"],
    category = "tools",
    description = "Buat email sementara & cek inbox",
    usage = ".tempmail create/inbox",
    example = ".tempmail create",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tempmail(ctx: Ctx) -> Reply {
    let option = ctx.text.trim().to_lowercase();

    if option.is_empty() {
        let saved = ctx.user_field("tempmail").and_then(Value::as_str);
        let aktif = match saved {
            Some(e) => format!("> Email aktif: *{}*\n", e),
            None => "> Belum punya email, ketik *.tempmail create* dulu\n".to_string(),
        };
        return Reply::text(format!(
            "📧 *Temp Mail*\n\n\
             Buat email sementara yang bisa terima pesan — cocok buat daftar akun tanpa kasih email asli.\n\n\
             *PENGGUNAAN:*\n\
             > *.tempmail create* — Bikin email baru\n\
             > *.tempmail inbox* — Cek pesan masuk\n\n{aktif}\
             _Email ini bersifat sementara, bisa hilang kapan saja_"
        ));
    }

    if option == "create" {
        match temp_mail_create(ctx.http.as_ref()) {
            Ok(res) => {
                if !res.get("status").and_then(Value::as_bool).unwrap_or(false) {
                    let error = res
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("kesalahan tidak diketahui");
                    return Reply::text(format!("❌ *Gagal Bikin Email*\n\n> {error}"))
                        .with_reaction("☢");
                }
                let email = res
                    .get("email")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                let mut user_data = ctx.user_data.clone();
                user_data.insert("tempmail".to_string(), serde_json::json!(email.clone()));

                return Reply::text(format!(
                    "📧 *Email Sementara Dibuat!*\n\n\
                     > 📬 Email: *{}*\n\n\
                     Sekarang kamu bisa pakai email ini buat daftar apa aja.\n\
                     Cek pesan masuk dengan *.tempmail inbox*\n\n\
                     _Email ini sementara, jangan dipakai buat hal penting_",
                    email
                ))
                .with_reaction("✅")
                .with_mutation(Mutation::SetUser {
                    jid: ctx.sender.clone(),
                    data: user_data,
                });
            }
            Err(e) => {
                return Reply::text(format!("❌ *Gagal Bikin Email*\n\n> {}", e.message))
                    .with_reaction("☢");
            }
        }
    }

    if option == "inbox" {
        let saved = ctx.user_field("tempmail").and_then(Value::as_str);
        let Some(email) = saved else {
            return Reply::text(
                "❌ *Belum Ada Email*\n\n\
                 Kamu belum bikin email sementara.\n\
                 Ketik *.tempmail create* dulu.",
            )
            .with_reaction("❌");
        };

        match temp_mail_inbox(ctx.http.as_ref(), &email) {
            Ok(res) => {
                if !res.get("status").and_then(Value::as_bool).unwrap_or(false) {
                    let error = res
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("kesalahan tidak diketahui");
                    return Reply::text(format!("❌ *Gagal Cek Inbox*\n\n> {error}"))
                        .with_reaction("☢");
                }

                let count = res.get("count").and_then(Value::as_u64).unwrap_or(0);
                if count == 0 {
                    return Reply::text(format!(
                        "📭 *Inbox Kosong*\n\n\
                         > Email: *{}*\n\n\
                         Belum ada pesan masuk. Coba cek lagi nanti.",
                        email
                    ))
                    .with_reaction("📭");
                }

                let messages = res.get("messages").and_then(Value::as_array).unwrap_or(&Vec::new());
                let mut txt = format!(
                    "📬 *Inbox — {} Pesan*\n\n\
                     > Email: *{}*\n\n",
                    count, email
                );
                for msg in messages {
                    let from = msg.get("from").and_then(Value::as_str).unwrap_or("-");
                    let subject = msg.get("subject").and_then(Value::as_str).unwrap_or("-");
                    let created_at = msg
                        .get("created_at")
                        .and_then(Value::as_str)
                        .unwrap_or("-");
                    let body_text = msg
                        .get("body_text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .chars()
                        .take(500)
                        .collect::<String>();
                    let body_display = if body_text.is_empty() {
                        "(tidak ada isi)".to_string()
                    } else {
                        body_text
                    };
                    txt.push_str(&format!(
                        "*━━━━━━━━━━━━━━━━━━━━*\n\
                         > 📧 Dari: *{}*\n\
                         > 📌 Subjek: *{}*\n\
                         > 🕐 {}\n\
                         > 📝 {}\n\n",
                        from, subject, created_at, body_display
                    ));
                }
                return Reply::text(txt.trim_end().to_string()).with_reaction("✅");
            }
            Err(e) => {
                return Reply::text(format!("❌ *Gagal Cek Inbox*\n\n> {}", e.message))
                    .with_reaction("☢");
            }
        }
    }

    Reply::text(format!(
        "❌ *Opsi Tidak Valid*\n\n> Gunakan *.tempmail create* atau *.tempmail inbox*"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        responses: Mutex<Vec<(String, String)>>, // (url contains, body)
        seen: Mutex<Vec<String>>,
        hit: Mutex<usize>,
    }

    impl Stub {
        fn with_responses(responses: Vec<(String, String)>) -> Arc<Stub> {
            Arc::new(Stub {
                responses: Mutex::new(responses),
                seen: Mutex::new(Vec::new()),
                hit: Mutex::new(0),
            })
        }
    }

    impl HttpGet for Stub {
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            let mut hit = self.hit.lock().unwrap();
            let i = *hit;
            *hit += 1;
            let responses = self.responses.lock().unwrap();
            let (_, body) = responses
                .get(i)
                .cloned()
                .unwrap_or_else(|| (String::new(), String::new()));
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
    }

    fn run(http: Arc<Stub>, text: &str, user_data: Value) -> Reply {
        let ctx = Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(|s| s.to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            command: "tempmail".into(),
            http,
            user_data,
            ..Default::default()
        };
        tempmail(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn stored_user(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("tempmail").unwrap();
        assert_eq!(p.meta.name, "tempmail");
        assert_eq!(p.meta.alias, ["tmpmail", "tmp", "trashmail"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
    }

    #[test]
    fn tanpa_opsi_tampilkan_bantuan_dan_email_aktif_jika_ada() {
        let http = Stub::with_responses(vec![]);
        let r = run(http.clone(), "", serde_json::json!({ "tempmail": "abc@temp-mail.io" }));
        assert!(body(&r).contains("Temp Mail"));
        assert!(body(&r).contains("Email aktif: *abc@temp-mail.io*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn create_menyimpan_email_ke_user_dan_balasan_sukses() {
        let http = Stub::with_responses(vec![
            (
                "email/new".to_string(),
                r#"{"email":"abc1234567@temp-mail.io"}"#.to_string(),
            ),
        ]);
        let r = run(http.clone(), "create", serde_json::json!({}));
        assert_eq!(r.mutations().len(), 1);
        let data = stored_user(&r);
        assert_eq!(data.get("tempmail").and_then(Value::as_str), Some("abc1234567@temp-mail.io"));
        assert!(body(&r).contains("Email Sementara Dibuat!"));
        assert!(body(&r).contains("abc1234567@temp-mail.io"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("email/new"));
    }

    #[test]
    fn create_gagal_tanpa_status() {
        let http = Stub::with_responses(vec![(
            "email/new".to_string(),
            r#"{"status":false,"error":"server busy"}"#.to_string(),
        )]);
        let r = run(http, "create", serde_json::json!({}));
        assert!(body(&r).contains("Gagal Bikin Email"));
        assert!(body(&r).contains("server busy"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn inbox_tanpa_email_disimpan() {
        let http = Stub::with_responses(vec![]);
        let r = run(http, "inbox", serde_json::json!({}));
        assert!(body(&r).contains("Belum Ada Email"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn inbox_menampilkan_pesan() {
        let http = Stub::with_responses(vec![
            (
                "email/abc@temp-mail.io/messages".to_string(),
                r#"[{"id":"1","from":"a@b.c","subject":"halo","created_at":"2026-01-01","body_text":"isi pesan"}]"#.to_string(),
            ),
        ]);
        let r = run(http, "inbox", serde_json::json!({ "tempmail": "abc@temp-mail.io" }));
        assert!(body(&r).contains("1 Pesan"));
        assert!(body(&r).contains("halo"));
        assert!(body(&r).contains("isi pesan"));
    }
}
