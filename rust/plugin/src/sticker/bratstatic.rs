//! Static brat variants.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

fn run(ctx: Ctx, tema: oktz_plugin_api::BratTema) -> Reply {
    if ctx.text.is_empty() {
        return Reply::text(format!(
            "🖼️ *ʙʀᴀᴛ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n`Contoh: {}bratimg Hai semua`",
            ctx.prefix
        ));
    }
    let (parsed, _) = super::brat::parse_brat_args(&ctx.text);
    let bytes = oktz_plugin_api::render_brat(&parsed, tema, 0).unwrap_or_default();
    Reply::sticker_opsi(MediaSource::bytes(bytes), oktz_plugin_api::StickerOpsi::gambar()).with_reaction("✅")
}

#[plugin(
    name = "bratgreen",
    alias = ["brat2"],
    category = "sticker",
    description = "Variant brat warna hijau",
    usage = ".bratgreen <text>",
    example = ".bratgreen Hai",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn bratgreen(ctx: Ctx) -> Reply {
    run(ctx, oktz_plugin_api::BratTema::Hijau)
}

#[plugin(
    name = "bratanime",
    alias = ["animebrat"],
    category = "sticker",
    description = "Variant brat anime",
    usage = ".bratanime <text>",
    example = ".bratanime Hai",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn bratanime(ctx: Ctx) -> Reply {
    run(ctx, oktz_plugin_api::BratTema::Anime)
}

#[plugin(
    name = "bratcewek",
    alias = ["cewekbrat", "bratperempuan", "bratgirl"],
    category = "sticker",
    description = "Variant brat cewek",
    usage = ".bratcewek <text>",
    example = ".bratcewek Hai",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn bratcewek(ctx: Ctx) -> Reply {
    run(ctx, oktz_plugin_api::BratTema::Cewek)
}
