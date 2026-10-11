use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekprocastinator,
    "cekprocastinator",
    ["procrastinator", "nunda"],
    "Cek tingkat suka menunda",
    ".cekprocastinator <nama>",
    ".cekprocastinator Budi",
    "keprocastinatoran",
    [
        "Deadline? Besok aja deh~ 😴",
        "Master procrastination! 🦥",
        "Kadang nunda, kadang rajin 😅",
        "Cukup produktif! 💪",
        "Disiplin tinggi! Salut! 🏆"
    ]
);
