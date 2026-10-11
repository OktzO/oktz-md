use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekpelit,
    "cekpelit",
    ["pelit", "kikir"],
    "Cek seberapa pelit kamu",
    ".cekpelit <nama>",
    ".cekpelit Budi",
    "kepelitan",
    [
        "SUPER PELIT! Duit dijaga mati-matian! 💸",
        "Pelit banget! 🙊",
        "Lumayan pelit 😅",
        "Sedikit hemat 😊",
        "Dermawan banget! 🎁"
    ]
);
