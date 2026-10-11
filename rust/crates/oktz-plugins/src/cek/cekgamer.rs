use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekgamer,
    "cekgamer",
    ["gamer", "pro"],
    "Cek seberapa pro gamer kamu",
    ".cekgamer <nama>",
    ".cekgamer Budi",
    "kegameran",
    [
        "PRO PLAYER! Esports level! 🏆",
        "Jago banget! 🎮",
        "Lumayan pro 👍",
        "Masih noob nih 😅",
        "Mending main masak-makanan 🍳"
    ]
);
