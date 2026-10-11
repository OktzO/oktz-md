use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceksabar,
    "ceksabar",
    ["sabar", "patience"],
    "Cek tingkat kesabaran kamu",
    ".ceksabar <nama>",
    ".ceksabar Budi",
    "kesabaran",
    [
        "Sabar level dewa! Zen master~ 🧘",
        "Sangat sabar! Terpuji 👏",
        "Cukup sabar 😊",
        "Kadang emosian dikit 😅",
        "Gampang marah nih... 😤"
    ]
);
