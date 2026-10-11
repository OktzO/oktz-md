use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekcantik,
    "cekcantik",
    ["cantik", "beautiful"],
    "Cek seberapa cantik kamu",
    ".cekcantik <nama>",
    ".cekcantik Budi",
    "kecantikan",
    [
        "Cantik banget kayak bidadari! 👸✨",
        "Cantik banget! 💕",
        "Manis dan cantik~ 🌸",
        "Lumayan cantik 😊",
        "Tetep cantik kok! 💖"
    ]
);
