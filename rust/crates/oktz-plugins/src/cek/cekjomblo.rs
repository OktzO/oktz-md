use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekjomblo,
    "cekjomblo",
    ["jomblo", "single"],
    "Cek tingkat kejombloan kamu",
    ".cekjomblo <nama>",
    ".cekjomblo Budi",
    "kejombloan",
    [
        "Jomblo abadi! Single is happiness~ 💔😎",
        "Strong independent person! 💪",
        "MasihPDKT mode ON 😍",
        "Ada yang naksir kayaknya~ 👀",
        "Soon taken! 💕"
    ]
);
