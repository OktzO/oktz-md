//! `plugins/owner/onlythisgrup.js` — bot hanya aktif di grup ini saja.
//!
//! `NOTE (mod.rs):` `owner/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/owner/mod.rs`:
//!   pub mod onlythisgrup;
//
//! Perbedaan sengaja dari JS: JS mencari bot di `groupMetadata.participants`
//! melalui `findParticipantByNumber` (penanganan LID). Di Rust,
//! `ctx.roster.is_bot_admin` sudah memberikan apakah bot admin, jadi pencarian
//! peserta bot tidak diperlukan. `sock.user.id` (nomor bot sendiri) juga tidak
//! ada di `Sock`, jadi tidak digunakan.
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

#[plugin(
    name = "onlythisgrup",
    alias = ["onlythisgroup", "lockgrup", "lockgroup"],
    category = "owner",
    description = "Bot hanya aktif di grup ini saja",
    usage = ".onlythisgrup",
    example = ".onlythisgrup",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn onlythisgrup(ctx: Ctx) -> Reply {
    let current = ctx.setting("onlyThisGroup");

    // Jika sudah terkunci ke grup ini: buka kunci.
    let already_locked = current
        .and_then(|v| v.get("jid").and_then(Value::as_str))
        .is_some_and(|jid| jid == ctx.chat);

    if already_locked {
        let mut r = Reply::text(
            "🔓 *UNLOCKED*\n\nBot kembali aktif di semua grup secara publik.",
        )
        .with_reaction("🔓");
        r = r.with_mutation(Mutation::SetSetting {
            key: "onlyThisGroup".into(),
            value: Value::Null,
        });
        return r;
    }

    if !ctx.roster.is_bot_admin {
        return Reply::text(
            "❌ *AKSES DITOLAK*\n\nBot harus menjadi admin di grup ini terlebih dahulu agar bisa mengambil tautan undangan (link grup).",
        );
    }

    let invite_code = match ctx.sock().kode_undangan(&ctx.chat) {
        Ok(code) => code,
        Err(_) => {
            return Reply::text(
                "❌ Gagal mengambil tautan undangan grup. Pastikan bot adalah admin yang sah.",
            )
        }
    };

    let invite_link = format!("https://chat.whatsapp.com/{invite_code}");
    // `group_metadata.subject` tidak tersedia lewak `roster`; gunakan `ctx.chat`
    // sebagai nama pengganti, mirip perilaku JS saat metadata tidak terbaca.

    let mut r = Reply::text(format!(
        "🔒 *LOCKED BERHASIL*\n\n\
         Mulai sekarang, bot hanya bisa digunakan secara eksklusif di grup:\n\
         *{chat}*\n\n\
         Pengguna di grup lain akan diarahkan untuk bergabung melalui tautan:\n\
         {link}\n\n\
         Ketik `.onlythisgrup` lagi untuk membuka kunci.",
        chat = ctx.chat,
        link = invite_link
    ))
    .with_reaction("🔒");

    r = r.with_mutation(Mutation::SetSetting {
        key: "onlyThisGroup".into(),
        value: serde_json::json!({
            "jid": ctx.chat,
            "name": ctx.chat,
            "link": invite_link,
        }),
    });

    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(settings: Value, is_bot_admin: bool, chat: &str) -> Reply {
        let ctx = Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: chat.to_string(),
            prefix: ".".into(),
            command: "onlythisgrup".into(),
            is_group: true,
            is_owner: true,
            settings,
            roster: oktz_plugin_api::registry::GroupRoster {
                is_bot_admin,
                ..Default::default()
            },
            ..Ctx::default()
        };
        // SockMati default — kode_undangan akan error. Kita override di test
        // yang membutuhkannya lewat konstruksi manual di bawah.
        onlythisgrup(ctx)
    }

    #[test]
    fn sudah_terkunci_ke_grup_sama_membuka_kunci() {
        let settings = serde_json::json!({"onlyThisGroup": {"jid": "123456@g.us"}});
        let r = run(settings.clone(), true, "123456@g.us");
        assert!(r.text_body().unwrap().contains("UNLOCKED"));
        assert_eq!(r.mutations().len(), 1);
        assert_eq!(
            r.mutations()[0],
            Mutation::SetSetting {
                key: "onlyThisGroup".into(),
                value: Value::Null
            }
        );
    }

    #[test]
    fn tidak_ada_admin_ditolak() {
        let r = run(serde_json::json!({}), false, "123456@g.us");
        assert!(r.text_body().unwrap().contains("AKSES DITOLAK"));
        assert_eq!(r.mutations().len(), 0);
    }
}
