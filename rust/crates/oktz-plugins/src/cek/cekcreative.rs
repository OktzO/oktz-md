use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekcreative,
    "cekcreative",
    ["creative", "kreatif"],
    "Cek tingkat kreativitas kamu",
    ".cekcreative <nama>",
    ".cekcreative Budi",
    "kecreativean",
    [
        "SUPER KREATIF! Artis sejati! 🎨✨",
        "Imajinatif banget! 💡",
        "Cukup kreatif 😊",
        "Biasa aja sih 🤔",
        "Kurang imajinasi nih 😅"
    ]
);
