use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekjahat,
    "cekjahat",
    ["evil"],
    "Cek seberapa jahat kamu",
    ".cekjahat <nama>",
    ".cekjahat Budi",
    "kejahatan",
    [
        "VILLAIN LEVEL! 😈👿",
        "Jahat banget! 💀",
        "Lumayan jahat 😏",
        "Sedikit nakal 😊",
        "Baik kok, gak jahat! 😇"
    ]
);
