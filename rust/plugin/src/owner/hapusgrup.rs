//! `plugins/owner/hapusgrup.js` — keluar dari grup / hapus grup.
//!
//! `NOTE (mod.rs):` `owner/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/owner/mod.rs`:
//!   pub mod hapusgrup;
use oktz_plugin_api::{Ctx, Reply, plugin};

/// Kartu pakai saat tidak ada argumen dan bukan di grup, atau JID tidak valid.
fn help() -> String {
    "🗑️ *ʜᴀᴘᴜs ɢʀᴜᴘ*\n\n\
     > `.hapusgrup` (di dalam grup) — Keluar dari grup ini\n\
     > `.hapusgrup <id_grup>` — Keluar dari grup tertentu\n\n\
     ⚠️ Bot akan keluar dari grup, bukan menghapus grup secara permanen"
        .to_string()
}

/// Membersihkan JID: hanya menyimpan digit, `@`, dan titik (mirip `/[^0-9@.]/g` di JS).
fn bersihkan_jid(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_digit() || *c == '@' || *c == '.')
        .collect()
}

/// `m.args[0].replace(/[^0-9@.]/g, '')` lalu `if (!x.endsWith('@g.us')) x += '@g.us'`.
/// Argumen yang tersaring habis jadi `"@g.us"` — JS tidak menolaknya, jadi
/// penolakan hanya terjadi saat guard `!targetJid || !endsWith('@g.us')`.
fn target_jid_dari_arg(raw: &str) -> String {
    let cleaned = bersihkan_jid(raw);
    if cleaned.ends_with("@g.us") {
        cleaned
    } else {
        cleaned + "@g.us"
    }
}

#[plugin(
    name = "hapusgrup",
    alias = ["deletegrup", "delgrup"],
    category = "owner",
    description = "Keluar dari grup / hapus grup",
    usage = ".hapusgrup (di dalam grup) atau .hapusgrup <jid>",
    example = ".hapusgrup",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn hapusgrup(ctx: Ctx) -> Reply {
    let target_jid = match ctx.args.first() {
        Some(raw) => target_jid_dari_arg(raw),
        None => {
            if ctx.is_group {
                ctx.chat.clone()
            } else {
                return Reply::text(help());
            }
        }
    };

    if !target_jid.ends_with("@g.us") {
        return Reply::text(help());
    }

    let group_name = match ctx.sock().metadata_grup(&target_jid) {
        Ok(meta) => meta
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or(&target_jid)
            .to_string(),
        Err(_) => target_jid.clone(),
    };

    match ctx.sock().keluar_grup(&target_jid) {
        Ok(()) => Reply::text(format!(
            "🗑️ *ʙᴏᴛ ᴋᴇʟᴜᴀʀ ᴅᴀʀɪ ɢʀᴜᴘ*\n\n\
             > Grup: {group_name}\n\
             > ID: {target_jid}"
        ))
        .with_reaction("✅"),
        Err(err) => Reply::text(format!("❌ Gagal keluar dari grup: {}", err.pesan)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_sock_ok(args: Vec<String>, is_group: bool, chat: &str) -> Ctx {
        Ctx {
            text: args.join(" "),
            args,
            sender: "628111@s.whatsapp.net".into(),
            chat: chat.to_string(),
            prefix: ".".into(),
            command: "hapusgrup".into(),
            is_group,
            is_owner: true,
            ..Ctx::default()
        }
    }

    #[test]
    fn tanpa_argumen_di_bukan_grup_mengembalikan_kontoh_pakai() {
        let ctx = ctx_sock_ok(vec![], false, "628111@s.whatsapp.net");
        let r = hapusgrup(ctx);
        assert!(r.text_body().unwrap().contains("ʜᴀᴘᴜs ɢʀᴜᴘ"));
    }

    #[test]
    fn argumen_tanpa_gus_ditambahi_gus() {
        let ctx = ctx_sock_ok(vec!["1234567890".into()], false, "x");
        let r = hapusgrup(ctx);
        // SockMati: `groupMetadata` gagal → `groupName = targetJid`, lalu
        // `groupLeave` gagal → cabang catch JS `❌ Gagal keluar dari grup: ...`
        assert_eq!(
            r.text_body().unwrap(),
            "❌ Gagal keluar dari grup: sock tidak tersedia"
        );
    }

    #[test]
    fn argumen_dibersihkan_dan_ditambahi_gus() {
        // `replace(/[^0-9@.]/g, '')` membuang huruf, jadi `123-456@g.us` jadi
        // `123456@.` lalu ditambahi `@g.us` — kuirk JS, bukan bug port.
        assert_eq!(target_jid_dari_arg("123-456@g.us"), "123456@.@g.us");
        assert_eq!(target_jid_dari_arg("1234567890"), "1234567890@g.us");
        // Argumen yang tersaring habis jadi "@g.us" (mirip JS), bukan kartu bantuan.
        assert_eq!(target_jid_dari_arg("abc"), "@g.us");
    }

    #[test]
    fn argumen_tak_layak_tetap_masuk_cabang_catch() {
        let ctx = ctx_sock_ok(vec!["abc".into()], false, "x");
        let r = hapusgrup(ctx);
        assert_eq!(
            r.text_body().unwrap(),
            "❌ Gagal keluar dari grup: sock tidak tersedia"
        );
    }

    #[test]
    fn tanpa_argumen_di_grup_memakai_chat_sebagai_target() {
        let ctx = ctx_sock_ok(vec![], true, "123456@g.us");
        let r = hapusgrup(ctx);
        assert_eq!(
            r.text_body().unwrap(),
            "❌ Gagal keluar dari grup: sock tidak tersedia"
        );
    }
}
