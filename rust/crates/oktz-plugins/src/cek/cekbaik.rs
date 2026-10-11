use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekbaik,
    "cekbaik",
    ["baik", "kind"],
    "Cek seberapa baik kamu",
    ".cekbaik <nama>",
    ".cekbaik Budi",
    "kebaikan",
    [
        "Mantappp! kamu adalah orang paling Baik di dunia ini! 😇✨",
        "Baik hati dan tidak sombong! 💝",
        "Lumayan baik 😊",
        "Sedikit baik 🙂",
        "Hmm, perlu introspeksi ?? 🤔"
    ]
);
