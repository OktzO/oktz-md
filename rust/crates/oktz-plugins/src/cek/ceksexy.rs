use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceksexy,
    "ceksexy",
    ["sexy", "hot"],
    "Cek seberapa sexy kamu",
    ".ceksexy <nama>",
    ".ceksexy Budi",
    "kesexyan",
    [
        "SEXY ABIS! 🔥🔥🔥",
        "Hot banget! 😏",
        "Lumayan menggoda~ 😊",
        "Biasa aja sih 🙂",
        "Mungkin cute bukan sexy 😅"
    ]
);
