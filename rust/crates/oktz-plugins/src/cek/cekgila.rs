use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekgila,
    "cekgila",
    ["gila", "crazy"],
    "Cek seberapa gila kamu",
    ".cekgila <nama>",
    ".cekgila Budi",
    "kegilaan",
    [
        "GILA BENERAN! Masuk RSJ! 🤪",
        "Hampir gila 😵",
        "Lumayan waras 😅",
        "Normal kok 🙂",
        "Waras banget! 😇"
    ]
);
