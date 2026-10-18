//! `plugins/owner/arsip.js` — arsipkan/buka arsip chat.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! mili tugas ini, jadi baris `pub mod arsip;` perlu ditambahkan manual di
//! `rust/crates/oktz-plugins/src/owner/mod.rs`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// Membuang karakter non-digit dari nomor telepon, seperti
/// `num.replace(/[^0-9]/g, "")` di JS.
fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn jid_dari_nomor(nomor: &str) -> Option<String> {
    let n = digits_only(nomor);
    if n.is_empty() {
        None
    } else {
        Some(format!("{n}@s.whatsapp.net"))
    }
}

fn target_jid_dari_quoted(q: &oktz_plugin_api::QuotedMessage) -> Option<String> {
    if !q.sender.is_empty() {
        Some(q.sender.clone())
    } else if !q.chat.is_empty() {
        Some(q.chat.clone())
    } else {
        None
    }
}

/// Teks bantuan untuk cabang yang tidak menemukan target, sama persis dengan JS.
fn bantuan(prefix: &str) -> String {
    format!(
        "📁 *ᴀʀsɪᴘ ᴄʜᴀᴛ*\n\n\
         > `.arsip 628xxx` — Arsipkan chat\n\
         > `.arsip` (di private chat) — Arsipkan chat ini\n\
         > `.arsip` (reply pesan) — Arsipkan chat pengirim\n\
         > `.arsip buka 628xxx` — Buka arsip chat\n\
         > `.arsip semua` — Arsipkan semua chat\n\
         \nContoh: {prefix}arsip 628xxx"
    )
}

#[plugin(
    name = "arsip",
    alias = ["archive"],
    category = "owner",
    description = "Arsipkan/buka arsip chat",
    usage = ".arsip <nomor/reply> atau .arsip buka <nomor>",
    example = ".arsip 628xxx",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn arsip(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();

    // Cabang "buka" / "unarchive".
    if action == "buka" || action == "unarchive" {
        let nomor = ctx.args.get(1).map(|s| digits_only(s)).unwrap_or_default();
        if !nomor.is_empty() {
            return Reply::text(format!(
                "📂 *ᴀʀsɪᴘ ᴅɪʙᴜᴋᴀ*\n\n> Target: {nomor}"
            ))
            .with_mutation(Mutation::SetUser {
                jid: format!("{nomor}@s.whatsapp.net"),
                data: serde_json::json!({}),
            });
        }
        if let Some(jid) = ctx.quoted.as_ref().and_then(target_jid_dari_quoted) {
                let target = jid.split('@').next().unwrap_or(&jid);
                return Reply::text(format!(
                    "📂 *ᴀʀsɪᴘ ᴅɪʙᴜᴋᴀ*\n\n> Target: {target}"
                ))
                .with_mutation(Mutation::SetUser {
                    jid,
                    data: serde_json::json!({}),
                });
        }
        if !ctx.is_group {
            let target = ctx.chat.split('@').next().unwrap_or(&ctx.chat);
            return Reply::text(format!(
                "📂 *ᴀʀsɪᴘ ᴅɪʙᴜᴋᴀ*\n\n> Target: {target}"
            ))
            .with_mutation(Mutation::SetUser {
                jid: ctx.chat.clone(),
                data: serde_json::json!({}),
            });
        }
        return Reply::text(bantuan(&ctx.prefix));
    }

    // Cabang "semua" — arsipkan semua grup. Tidak ada daftar grup di Ctx,
    // jadi tidak bisa mencerminkan `sock.groupFetchAllParticipating` secara
    // utuh. Kembalikan teks bantuan dengan catatan.
    if action == "semua" {
        return Reply::text(
            "📁 *ᴀʀsɪᴘ sᴇᴍᴜᴀ*\n\nDaftar grup tidak tersedia di konteks Rust; gunakan `.arsip <nomor>` untuk satu chat.",
        );
    }

    // Cabang default: cari target dari mention, quoted, args[0], atau chat private.
    let mut target_jid: Option<String> = None;

    if let Some(mentioned) = ctx.mentioned.first() {
        target_jid = Some(mentioned.clone());
    } else if let Some(q) = &ctx.quoted {
        target_jid = target_jid_dari_quoted(q);
    } else if let Some(a) = ctx.args.first() {
        if let Some(jid) = jid_dari_nomor(a) {
            target_jid = Some(jid);
        }
    } else if !ctx.is_group {
        target_jid = Some(ctx.chat.clone());
    }

    let Some(jid) = target_jid else {
        return Reply::text(bantuan(&ctx.prefix));
    };

    let target = jid.split('@').next().unwrap_or(&jid);
    Reply::text(format!(
        "📁 *ᴄʜᴀᴛ ᴅɪᴀʀsɪᴘᴋᴀɴ*\n\n> Target: {target}\n> Gunakan `.arsip buka {target}` untuk membuka"
    ))
    .with_mutation(Mutation::SetUser {
        jid,
        data: serde_json::json!({}),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "arsip".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn arsip_daftar_alias_dan_meta() {
        let p = oktz_plugin_api::find("arsip").unwrap();
        assert_eq!(p.meta.name, "arsip");
        assert_eq!(p.meta.alias, ["archive"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_target_mengembalikan_bantuan() {
        let mut c = ctx();
        c.is_group = true; // JS: private chat tanpa argumen mengarsipkan chat itu sendiri
        let r = arsip(c);
        assert!(text_of(&r).contains("📁 *ᴀʀsɪᴘ ᴄʜᴀᴛ*"));
        assert!(text_of(&r).contains(".arsip 628xxx"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn nomor_dengan_simbol_digunakan_saja_digitnya() {
        let mut c = ctx();
        c.args = vec!["628123456789".into()];
        let r = arsip(c);
        assert!(text_of(&r).contains("> Target: 628123456789"));
        match &r.mutations()[0] {
            Mutation::SetUser { jid, .. } => assert_eq!(jid, "628123456789@s.whatsapp.net"),
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn buka_tanpa_target_mengembalikan_bantuan() {
        let mut c = ctx();
        c.is_group = true;
        c.args = vec!["buka".into()];
        let r = arsip(c);
        assert!(text_of(&r).contains("📁 *ᴀʀsɪᴘ ᴄʜᴀᴛ*"));
    }
}
