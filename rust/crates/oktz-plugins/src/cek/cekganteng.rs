use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekganteng,
    "cekganteng",
    ["ganteng", "handsome"],
    "Cek seberapa ganteng kamu",
    ".cekganteng <nama>",
    ".cekganteng Budi",
    "kegantengan",
    [
        "Ganteng maksimal! 😍🔥",
        "Ganteng banget! 😎",
        "Lumayan ganteng~ 👍",
        "Biasa aja sih 😅",
        "Mungkin inner beauty? 🤭"
    ]
);
