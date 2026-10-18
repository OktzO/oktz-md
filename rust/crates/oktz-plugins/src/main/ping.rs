//! `plugins/main/ping.js` — canvas "Epic System Diagnostics" dengan data OS,
//! CPU, memori, dan uptime.
//!
//! `ponytail:` Tidak ada API canvas (`@napi-rs/canvas`), `os`, `perf_hooks`,
//! atau `process.memoryUsage()` yang diekspos ke plugin di registry Rust.
//! Seluruh gambar dan data sistem tidak dapat dicerminkan. Handler di bawah
//! jatuh ke error template.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

#[plugin(
    name = "ping",
    alias = ["speed", "p", "sys"],
    category = "main",
    description = "Cek performa dan status sistem bot secara real-time (Canvas Epic)",
    usage = ".ping",
    example = ".ping",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ping(ctx: Ctx) -> Reply {
    // `ponytail:` Semua data sistem (canvas, os.cpus, os.totalmem, process.uptime,
    // dst.) dan render canvas tidak tersedia di registry Rust.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn selalu_error_tanpa_api_canvas_dan_os() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ping".into(),
            ..Ctx::default()
        };
        let r = ping(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "ping")
        );
    }
}
