use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekpsikopat,
    "cekpsikopat",
    ["psikopat", "psycho"],
    "Cek seberapa psikopat kamu",
    ".cekpsikopat <nama>",
    ".cekpsikopat Budi",
    "kepsikopatan",
    [
        "PSIKOPAT AKUT! Jauhi! 😈",
        "Hati-hati sama orang ini 👀",
        "Ada sisi gelapnya 🌑",
        "Sedikit misterius 🤔",
        "Normal dan baik hati 😇"
    ]
);
