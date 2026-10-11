use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceklapar,
    "ceklapar",
    ["lapar", "hungry"],
    "Cek tingkat kelaparan kamu",
    ".ceklapar <nama>",
    ".ceklapar Budi",
    "kelaparan",
    [
        "LAPARRR! Makan sekarang! 🍔🍕🍜",
        "Perut keroncongan~ 😋",
        "Bisa lah ngemil 🍿",
        "Masih kenyang 😊",
        "Kekenyangan! 🤰"
    ]
);
