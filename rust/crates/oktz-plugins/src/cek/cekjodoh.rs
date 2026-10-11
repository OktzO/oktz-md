use crate::common::percent_101;
use oktz_plugin_api::{Ctx, Reply, plugin};

/// Tiga nilai yang dirender `cekjodoh` — dipisah dari `cek_reply` karena
/// input-nya bukan mention: `cekjodoh` menolak kalau teks tidak punya 2 nama.
pub(crate) fn jodoh_desc(percent: u32) -> &'static str {
    if percent >= 90 {
        "Jodoh banget! Langsung nikah aja! 💍"
    } else if percent >= 70 {
        "Cocok banget! 💕"
    } else if percent >= 50 {
        "Lumayan cocok~ 😊"
    } else if percent >= 30 {
        "Hmm, perlu usaha lebih 🤔"
    } else {
        "Mungkin cari yang lain? 😅"
    }
}

#[plugin(
    name = "cekjodoh",
    alias = [],
    category = "cek",
    description = "Cek kecocokan jodoh",
    usage = ".cekjodoh <nama1> & <nama2>",
    example = ".cekjodoh Budi & Ani",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn cekjodoh(ctx: Ctx) -> Reply {
    // JS: `m.text?.trim().split(/[&,]/).map(trim).filter(Boolean)` — nama yang
    // dipisah tidak pernah dipakai lagi, handler cuma butuh jumlahnya.
    let name_count = ctx
        .text
        .trim()
        .split(['&', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .count();

    if name_count < 2 {
        return Reply::text(format!(
            "💕 *ᴄᴇᴋ ᴊᴏᴅᴏʜ*\n\n> Masukkan 2 nama!\n\n> Contoh: {}cekjodoh Budi & Ani",
            ctx.prefix
        ));
    }

    let percent = percent_101();
    let mentioned = ctx.mention_or_sender().to_string();
    let number = mentioned.split('@').next().unwrap_or(&mentioned);
    let desc = jodoh_desc(percent);

    let txt = if mentioned == ctx.sender {
        format!("Hai @{number}\n    \nTingkat kejodohan kamu *{percent}%*\n```{desc}```")
    } else {
        format!(
            "Kamu ingin ngecek tingkat kejodohan @{number} yak? \n    \nTingkat kejodohan dia sebesar *{percent}%*\n```{desc}```"
        )
    };

    Reply::Text {
        text: txt,
        mentions: vec![mentioned],
    }
}
