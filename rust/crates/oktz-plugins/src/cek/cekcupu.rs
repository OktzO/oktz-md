use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekcupu,
    "cekcupu",
    ["cupu", "noob"],
    "Cek tingkat kecupuan kamu",
    ".cekcupu <nama>",
    ".cekcupu Budi",
    "kecupuan",
    [
        "CUPU PARAH! NOOB DETECTED! 🤡",
        "Masih newbie nih~ 😅",
        "Biasa aja lah 🤔",
        "Cukup jago! 💪",
        "PRO PLAYER! GG! 🏆"
    ]
);
