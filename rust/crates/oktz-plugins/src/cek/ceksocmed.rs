use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceksocmed,
    "ceksocmed",
    ["sosmed", "medsos"],
    "Cek tingkat kecanduan sosmed",
    ".ceksocmed <nama>",
    ".ceksocmed Budi",
    "kesocmedan",
    [
        "Kecanduan parah! Detox needed! 📱💀",
        "Scroll terus tanpa henti~ 📲",
        "Normal usage 👍",
        "Cukup sehat nih 🌿",
        "Digital detox master! 🧘"
    ]
);
