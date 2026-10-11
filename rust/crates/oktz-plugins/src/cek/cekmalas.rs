use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekmalas,
    "cekmalas",
    ["malas", "lazy"],
    "Cek seberapa malas kamu",
    ".cekmalas <nama>",
    ".cekmalas Budi",
    "kemalasan",
    [
        "SUPER MALAS! Raja rebahan! 🛏️",
        "Malas banget! 😴",
        "Lumayan malas 🥱",
        "Sedikit malas 😊",
        "Rajin banget! 💪"
    ]
);
