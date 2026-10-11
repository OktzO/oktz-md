use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

cek_plugin!(
    cekkpopers,
    "cekkpopers",
    ["kpopers", "kpop"],
    "Cek tingkat kpopers kamu",
    ".cekkpopers <nama>",
    ".cekkpopers Budi",
    "kekpopersan",
    [
        "ARMY/BLINK level max! 💜💗",
        "Stan berat nih! 🎤",
        "Casual listener~ 🎵",
        "Tau dikit-dikit aja 😅",
        "Bukan kpopers 🤷"
    ]
);
