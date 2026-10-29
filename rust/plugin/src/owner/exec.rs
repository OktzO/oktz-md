//! `plugins/owner/exec.js` — jalankan kode JS dari pesan yang di-reply (Owner Only).
//!
//! `ponytail:` `eval()` kode JavaScript dan `config.isOwner(m.sender)` tidak
//! dapat di-replikasi di Rust (tidak ada mesin JS, dan `config.isOwner`
//! bukan bagian dari `Ctx`). Yang dipindah: deteksi kode dari quoted / teks,
//! dan teks penjelasan. Eksekusi kode sendiri tidak dapat dilakukan.
//!
//! Catatan: baris `pub mod exec;` perlu ditambah ke `owner/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "exec",
    alias = [">", "run", "execute"],
    category = "owner",
    description = "Jalankan kode JS dari pesan yang di-reply (Owner Only)",
    usage = ".> (reply pesan berisi kode)",
    example = ".> (reply)",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn exec(ctx: Ctx) -> Reply {
    let code = if let Some(q) = ctx.quoted() {
        if !q.text.is_empty() {
            q.text.clone()
        } else {
            String::new()
        }
    } else {
        ctx.text.trim().to_string()
    };

    if code.is_empty() {
        return Reply::text(
            "⚙️ *ᴇxᴇᴄ*\n\n\
             > Reply pesan berisi kode JavaScript!\n\n\
             *Atau:*\n\
             > .> <code>\n\n\
             *Contoh:*\n\
             > Reply pesan: `return m.chat`\n\
             > Lalu ketik: .>",
        );
    }

    Reply::text(
        "⚠️ *ᴇxᴇᴄ tidak tersedia di port Rust*\n\n\
         > `eval()` kode JavaScript tidak dapat dijalankan di Rust.\n\
         > Kode yang ingin dieksekusi:\n\
         `...`",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(quoted_text: Option<&str>, text: &str) -> Reply {
        let ctx = Ctx {
            command: ">".into(),
            text: text.to_string(),
            quoted: quoted_text.map(|t| oktz_plugin_api::QuotedMessage {
                id: "q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "628111@s.whatsapp.net".into(),
                text: t.to_string(),
                kind: oktz_plugin_api::QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            timestamp: 0,
        }),
            ..Default::default()
        };
        exec(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("exec").unwrap();
        assert_eq!(p.meta.name, "exec");
        assert_eq!(p.meta.alias, [">", "run", "execute"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn tanpa_kode_tampilkan_bantuan() {
        let r = run(None, "");
        assert!(body(&r).contains("ᴇxᴇᴄ"));
        assert!(body(&r).contains("Reply pesan berisi kode JavaScript"));
    }

    #[test]
    fn kode_dari_quoted_dipakai() {
        let r = run(Some("return 1 + 1"), "");
        assert!(!body(&r).contains("bantuan"));
        assert!(!r.is_none());
    }

    #[test]
    fn kode_dari_text_kalau_tidak_ada_quoted() {
        let r = run(None, "console.log('hi')");
        assert!(!body(&r).contains("bantuan"));
    }
}
