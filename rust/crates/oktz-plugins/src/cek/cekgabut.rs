use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekgabut,
    "cekgabut",
    ["gabut", "bored"],
    "Cek tingkat keGabutan kamu",
    ".cekgabut <nama>",
    ".cekgabut Budi",
    "kegabutan",
    [
        "GABUT LEVEL MAX! Main bot aja~ 🥱",
        "Gabut parah nih! 😴",
        "Lumayan gabut 😅",
        "Agak sibuk dikit 📝",
        "Sibuk banget! Produktif! 💼"
    ]
);
