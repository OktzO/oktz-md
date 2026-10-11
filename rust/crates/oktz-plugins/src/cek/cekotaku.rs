use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekotaku,
    "cekotaku",
    ["otaku"],
    "Cek tingkat otaku kamu",
    ".cekotaku <nama>",
    ".cekotaku Budi",
    "keotakuan",
    [
        "SUGOI! True otaku desu! 🎌✨",
        "Weeb level tinggi~ 🇯🇵",
        "Casual anime enjoyer 📺",
        "Tau anime dikit-dikit 🤔",
        "Normie detected! 😂"
    ]
);
