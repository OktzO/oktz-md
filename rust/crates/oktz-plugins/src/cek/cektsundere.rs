use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cektsundere,
    "cektsundere",
    ["tsundere"],
    "Cek tingkat tsundere kamu",
    ".cektsundere <nama>",
    ".cektsundere Budi",
    "ketsunderean",
    [
        "BAKA! B-BUKAN BERARTI AKU SUKA! 😤💢",
        "Hmph! Jangan salah paham ya! 😳",
        "Y-yah terserah kamu deh... 👉👈",
        "Agak tsundere dikit~ 😊",
        "Bukan tsundere, jujur aja kok 💕"
    ]
);
