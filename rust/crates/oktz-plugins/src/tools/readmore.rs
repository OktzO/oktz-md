use oktz_plugin_api::{Ctx, Reply, plugin};

/// `String.fromCharCode(8206)` = U+200E LEFT-TO-RIGHT MARK.
pub(crate) const LRM: &str = "\u{200e}";

pub(crate) const LRM_COUNT: usize = 4001;

pub(crate) const HELP_READMORE: &str = "⚠️ Masukan text!";

/// `l + LRM*4001 + r` dengan `l`/`r` = dua segmen pertama hasil `text.split('|')`.
pub(crate) fn readmore_body(text: &str) -> String {
    let mut parts = text.split('|');
    let l = parts.next().unwrap_or("");
    let r = parts.next().unwrap_or("");

    let mut out = String::with_capacity(l.len() + r.len() + LRM_COUNT * LRM.len());
    out.push_str(l);
    for _ in 0..LRM_COUNT {
        out.push_str(LRM);
    }
    out.push_str(r);
    out
}

#[plugin(
    name = "readmore",
    alias = ["selengkapnya", "spoiler"],
    category = "tools",
    description = "Membuat teks baca selengkapnya (spoiler)",
    usage = ".readmore <text_awal>|<text_akhir>",
    example = ".readmore Hai|Ini adalah pesan rahasia",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn readmore(ctx: Ctx) -> Reply {
    if ctx.text.is_empty() {
        return Reply::text(format!(
            "{HELP_READMORE}\nContoh: `{}{} Halo|Ini teks tersembunyi`",
            ctx.prefix, ctx.command
        ));
    }
    Reply::text(readmore_body(&ctx.text))
}
