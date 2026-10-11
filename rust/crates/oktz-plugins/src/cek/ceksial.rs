use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceksial,
    "ceksial",
    ["sial", "apes"],
    "Cek seberapa sial kamu",
    ".ceksial <nama>",
    ".ceksial Budi",
    "kesialan",
    [
        "SIAL BANGET! Mending di rumah aja! 😭",
        "Lagi apes nih~ 😢",
        "Lumayan sial 😓",
        "Sedikit sial 😕",
        "Gak sial, hoki dong! 🍀"
    ]
);
