use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekhoki,
    "cekhoki",
    ["hoki"],
    "Cek seberapa hoki kamu",
    ".cekhoki <nama>",
    ".cekhoki Budi",
    "kehokian",
    [
        "HOKI DEWA! Main gacha pasti menang! 🍀✨",
        "Hoki banget! 🎰",
        "Lumayan hoki 🍀",
        "Sedikit hoki 😊",
        "Sabar ya, lagi apes 😅"
    ]
);
