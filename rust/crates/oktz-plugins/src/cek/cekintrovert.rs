use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekintrovert,
    "cekintrovert",
    ["introvert"],
    "Cek tingkat introvert kamu",
    ".cekintrovert <nama>",
    ".cekintrovert Budi",
    "keintrovertan",
    [
        "Rumah adalah surga! Stay home~ 🏠",
        "Social battery terbatas 🔋",
        "Ambivert, balance~ ⚖️",
        "Cukup social butterfly 🦋",
        "Extrovert mode ON! 🎉"
    ]
);
