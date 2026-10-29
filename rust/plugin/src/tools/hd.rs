//! `plugins/tools/hd.js` — enhance gambar jadi HD (via ihancer.com API).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "*🪁 HD IMAGE*\n> Reply gambar yang mau dijernihkan\n\n`{prefix}remini`",
        prefix = prefix
    )
}

#[plugin(
    name = "remini",
    alias = ["hd", "enhance", "upscale"],
    category = "tools",
    description = "Enhance gambar jadi HD",
    usage = ".remini (reply gambar)",
    example = ".remini",
    cooldown = 15,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn hd(ctx: Ctx) -> Reply {
    // `hd.js` butuh:
    //   1. `m.isImage` / `m.quoted.type === "imageMessage"` — cek media, tersedia
    //      via `ctx.media` / `ctx.quoted`.
    //   2. `m.download()` / `m.quoted.download()` — byte gambar, tersedia via
    //      `ctx.media_fetch.download(...)`.
    //   3. `fetch('https://ihancer.com/api/enhance', { method: 'POST', body: FormData })`
    //      — bisa via `ctx.http.call(HttpCall::post(...).multipart(...))`.
    //   4. `sock.sendMessage({ image: enhancedBuffer })` — kirim gambar hasil,
    //      bisa `Reply::image(MediaSource::bytes(buf))`.
    //
    // Semua komponen HTTP ternyata tersedia di `Ctx`, sehingga plugin ini
    // mendekati bisa di-port utuh. Masalahnya: handler JS async (unduh dulu,
    // baru kirim), sementara `Fn(Ctx) -> Reply` sinkron dan `Reply` statis
    // tidak menunda pembentukan balasan sampai after I/O.
    //
    // `ponytail:` setelah handler bisa async / `Reply` menunda render,
    // pasangkan alur: unduh media → POST multipart ke ihancer → kirim byte
    // hasil sebagai image.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of() -> Ctx {
        Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "hd".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = hd(ctx_of());
        assert!(r.text_body().unwrap().contains("HD IMAGE"));
    }
}
