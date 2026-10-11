use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekgacha,
    "cekgacha",
    ["luck"],
    "Cek hoki gacha kamu",
    ".cekgacha <nama>",
    ".cekgacha Budi",
    "kegachaan",
    [
        "HOKI PARAH! SSR GUARANTEED! ✨💎",
        "Lucky! Pasti dapet SR keatas! 🍀",
        "Hoki-hoki dikit 😊",
        "Hmm... pray harder! 🙏",
        "SIAL! Nanti aja gachanya! 💔"
    ]
);
