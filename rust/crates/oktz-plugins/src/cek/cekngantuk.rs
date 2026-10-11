use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekngantuk,
    "cekngantuk",
    ["ngantuk", "sleepy"],
    "Cek tingkat ngantuk kamu",
    ".cekngantuk <nama>",
    ".cekngantuk Budi",
    "kengantukan",
    [
        "ZZZZZ... Tidur sana! 😴💤",
        "Mata 5 watt nih~ 😪",
        "Agak ngantuk dikit 🥱",
        "Masih fresh! ☕",
        "Melek banget! Insomnia? 👀"
    ]
);
