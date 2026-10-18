//! `plugins/user/unreg.js` — hapus data pendaftaran dari store.
//!
//! `ponytail:` `getDatabase()`, `db.setUser()`, dan `db.save()` tidak diekspos
//! ke plugin di registry Rust. Tidak ada store user yang dapat dibaca atau
//! ditulis. Handler di bawah hanya melaporkan "belum terdaftar" (karena store
//! selalu kosong) dan jatuh ke error untuk cabang sukses yang seharusnya
//! menyimpan.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

/// Kartu "belum terdaftar" (store selalu kosong tanpa API database).
pub(crate) fn belum_terdaftar(prefix: &str) -> String {
    format!(
        "❌ Kamu belum terdaftar!\n\n> Daftar dengan `{prefix}daftar`"
    )
}

#[plugin(
    name = "unreg",
    alias = ["unregister", "hapusdaftar"],
    category = "user",
    description = "Hapus data pendaftaran kamu dari bot",
    usage = ".unreg",
    example = ".unreg",
    cooldown = 30,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn unreg(ctx: Ctx) -> Reply {
    // `ponytail:` `db.getUser(m.sender)` tidak tersedia; tanpa store, user
    // selalu dianggap belum terdaftar.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn selalu_error_tanpa_database_api() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "unreg".into(),
            ..Ctx::default()
        };
        let r = unreg(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "unreg")
        );
    }
}
