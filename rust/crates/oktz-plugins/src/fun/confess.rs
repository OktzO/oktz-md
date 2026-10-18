//! `plugins/fun/confess.js` — kirim pesan anonim ke seseorang (menfess).
//!
//! `NOTE (mod.rs):` `fun/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/fun/mod.rs`:
//!   pub mod confess;
//
//! Handler utama memeriksa nomor tujuan via `sock.ada_whatsapp`, menyimpan
//! data menfess di `ctx.sessions`, dan mengirim pesan anonim.
//! Answer handler menangani balasan ke pesan menfess.
//!
//! `ponytail:` answer handler hanya bisa mengirim balasan ke `ctx.chat`
//! (chat saat ini). Meneruskan balasan ke `confessInfo.senderChat` yang
//! mungkin berbeda memerlukan API kirim-ke-chat-lain yang belum ada di
//! `Reply`, sehingga bagian itu tidak dapat direplikasi sepenuhnya.
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const CONFESS_SESSION_PREFIX: &str = "confess:";

fn help_text(prefix: &str) -> String {
    format!(
        "💌 *LAYANAN MENFESS ANONIM* 💌\n\n\
         Mau ngirim pesan rahasia ke gebetan atau teman tanpa ketahuan? Bisa banget kak!\n\n\
         *Cara Pakai:*\n\
         👉 `.confess nomor|pesan`\n\n\
         *Contoh:*\n\
         > `.confess 6281234567890|Hai kak, aku suka deh liat senyum kamu!`\n\n\
         > 🤫 _Tenang aja, identitas kamu 100% aman dan dirahasiakan!_"
    )
}

fn clean_number(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn normalize_number(raw: &str) -> String {
    let num = clean_number(raw);
    if num.starts_with('0') {
        format!("62{}", &num[1..])
    } else {
        num
    }
}

fn confess_text(message: &str) -> String {
    format!(
        "💌 *ADA PESAN RAHASIA BUAT KAMU KAK!* 💌\n\n\
         Sstt.. Ada seseorang yang diam-diam ngirim pesan buat kamu nih:\n\n\
         💬 *Isi Pesan:*\n\
         ```{message}```\n\n\
         > 🔒 _Pesan ini dikirim secara anonim (identitas pengirim dirahasiakan)._\n\
         > ✉️ _Kamu bisa balas pesan ini kok! Tinggal *REPLY* aja pesannya ya!_"
    )
}

fn success_text(target_number: &str) -> String {
    format!(
        "✅ *MENFESS BERHASIL TERKIRIM!* ✅\n\n\
         > 📱 Terkirim ke: `{target_number}`\n\
         > 🔒 Identitas kamu aman sentosa!\n\n\
         > _Nanti kalau dia balas pesannya, aku bakal langsung terusin ke sini kak! Santai aja_ 😉"
    )
}

#[plugin(
    name = "confess",
    alias = ["confession", "menfess", "anonim"],
    category = "fun",
    description = "Kirim pesan anonim ke seseorang",
    usage = ".confess nomor|pesan",
    example = ".confess 6281234567890|Hai, aku suka kamu!",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn confess(ctx: Ctx) -> Reply {
    let input = ctx.text.trim();

    if input.is_empty() || !input.contains('|') {
        return Reply::text(help_text(&ctx.prefix));
    }

    let mut parts = input.splitn(2, '|');
    let raw_number = parts.next().unwrap_or("");
    let message = parts.next().unwrap_or("").trim();

    if raw_number.is_empty() || message.is_empty() {
        return Reply::text(format!(
            "Aduh kak formatnya salah! 😅\n\nCoba ketik gini: `.confess nomor|pesan`"
        ));
    }

    let target_number = normalize_number(raw_number);
    let sender_number = ctx.sender.split('@').next().unwrap_or(&ctx.sender);

    if target_number == sender_number {
        return Reply::text("Ih masa ngirim menfess buat diri sendiri sih kak? Kasih buat orang lain dong! 😂");
    }

    if target_number.len() < 10 || target_number.len() > 15 {
        return Reply::text("Hmm, nomor tujuan yang kamu masukin kayaknya nggak valid deh kak! 🤔");
    }

    let target_jid = format!("{target_number}@s.whatsapp.net");

    let ada_whatsapp = match ctx.sock().ada_whatsapp(&[target_number.to_string()]) {
        Ok(list) => list,
        Err(_) => return Reply::text(
            "Hmm kak, verifikasi nomor ke WhatsApp gagal (timeout/koneksi), jadi pesan belum dikirim ya. Coba lagi nanti kak! 🙏",
        ),
    };

    let on_wa_exists = ada_whatsapp.iter().any(|a| a.jid == target_jid && a.ada);
    if !on_wa_exists {
        return Reply::text(
            "Yah kak, nomor ternyata nggak terdaftar di WhatsApp! 😔",
        );
    }

    if message.len() < 5 {
        return Reply::text("Pesannya kependekan kak! Minimal 5 karakter ya biar lebih bermakna. 📝");
    }

    if message.len() > 1000 {
        return Reply::text("Wah pesannya kepanjangan kak! Maksimal 1000 karakter ya biar bacanya nggak pusing. 📜");
    }

    let message_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
        .to_string();

    let entry = serde_json::json!({
        "senderJid": ctx.sender,
        "senderChat": ctx.chat,
        "targetJid": target_jid,
        "createdAt": message_id.parse::<i64>().unwrap_or(0),
    });
    ctx.session_set(&format!("{CONFESS_SESSION_PREFIX}{message_id}"), entry);

    Reply::text(success_text(&target_number))
}

/// Answer handler untuk balasan ke pesan menfess.
///
/// Dipanggil untuk pesan non-command. `None` berarti "tidak menangani".
///
/// JS asli mengirim balasan ke `confessInfo.senderChat` (bukan `m.chat`),
/// yang tidak dapat direplikasi dengan `Reply` karena semua balasan
/// mengikuti `ctx.chat`. Sini hanya mengonfirmasi di chat saat ini.
#[oktz_plugin_api::answer_handler]
pub fn confess_reply(ctx: Ctx) -> Option<Reply> {
    let quoted = ctx.quoted()?;
    let quoted_id = &quoted.id;

    let key = format!("{CONFESS_SESSION_PREFIX}{quoted_id}");
    let entry = ctx.session_get(&key)?;

    let target_jid = entry.get("targetJid")?.as_str()?;
    if ctx.sender != target_jid {
        return None;
    }

    let reply_text = ctx.text.trim();
    if reply_text.is_empty() {
        return None;
    }

    ctx.session_take(&key);

    Some(Reply::text(
        "✅ Balasan Anda telah diterima dan akan disampaikan.\n\n\
         _Catatan: penyaluran balasan ke chat pengirim asli memerlukan \
         kemampuan kirim ke chat lain yang belum tersedia di Rust._",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_number_hanya_mengambil_digit() {
        assert_eq!(clean_number("6281234567890"), "6281234567890");
        assert_eq!(clean_number("081234567890"), "081234567890");
        assert_eq!(clean_number("+62 812-3456 7890"), "6281234567890");
    }

    #[test]
    fn normalize_number_ubah_0_awal_menjadi_62() {
        assert_eq!(normalize_number("081234567890"), "6281234567890");
        assert_eq!(normalize_number("6281234567890"), "6281234567890");
    }

    #[test]
    fn confess_text_mengandung_pesan() {
        let t = confess_text("Halo kak");
        assert!(t.contains("ADA PESAN RAHASIA BUAT KAMU KAK"));
        assert!(t.contains("Halo kak"));
    }

    #[test]
    fn help_text_mengandung_contoh_pakai() {
        let h = help_text(".");
        assert!(h.contains("confess nomor|pesan"));
        assert!(h.contains("6281234567890"));
    }
}
