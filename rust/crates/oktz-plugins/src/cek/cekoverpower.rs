use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekoverpower,
    "cekoverpower",
    ["overpower", "op"],
    "Cek tingkat overpower kamu",
    ".cekoverpower <nama>",
    ".cekoverpower Budi",
    "keoverpoweran",
    [
        "OVERPOWER BANGET! LEGEND! 👑🔥",
        "Kuat banget nih! 💪",
        "Lumayan strong~ 😎",
        "Biasa aja sih 🤔",
        "Masih perlu latihan 📝"
    ]
);
