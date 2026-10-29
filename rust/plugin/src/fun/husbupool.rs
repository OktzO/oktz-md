//! `plugins/fun/husbupool.js` — jelajahi pool husbu — cari nama/franchise/tier.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📖 *HUSBU POOL* 📖\n\n> Cari nama/franchise/tier\n\n`{prefix}husbupool [nama|tier|franchise]`",
        prefix = prefix
    )
}

#[plugin(
    name = "husbupool",
    alias = ["poolhusbu"],
    category = "fun",
    description = "Jelajahi pool husbu — cari nama/franchise/tier!",
    usage = ".husbupool [nama|tier|franchise]",
    example = ".husbupool legendary",
    cooldown = 5,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn husbupool(ctx: Ctx) -> Reply {
    // `husbupool.js` butuh `searchPool(query)` dari `src/data/husbu/index.js`,
    // yang menggabungkan 36 file franchise dan membangun pool dengan rollWeight.
    // Data tersebut hidup di sisi JS (`data/husbu/`), bukan di crate Rust.
    // Tanpa data pool yang sama, hasil pencarian tidak akan cocok dengan JS.
    //
    // Juga butuh status halaman per (sender, query) (`pages: Map`) dan
    // `sock.sendButton` untuk navigasi prev/next — keduanya I/O yang tidak
    // ada di `Ctx` class A.
    //
    // `ponytail:` pindahkan data husbu ke Rust (atau sertakan sebagai embedded
    // data di crate ini), lalu implementasikan pencarian + paginasi dalam
    // `Reply::Text` / `Reply::Interactive`.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "husbupool".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = husbupool(ctx_of(""));
        let t = r.text_body().unwrap();
        assert!(t.contains("📖 *HUSBU POOL* 📖"));
        assert!(t.contains("`.husbupool [nama|tier|franchise]`"));
    }

    #[test]
    fn query_apa_pun_tetap_kartu_pakai_belum_ada_data_pool() {
        let r = husbupool(ctx_of("legendary"));
        assert!(r.text_body().unwrap().contains("HUSBU POOL"));
    }
}
