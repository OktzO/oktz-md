use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekkaya,
    "cekkaya",
    ["kaya", "rich"],
    "Cek seberapa kaya kamu",
    ".cekkaya <nama>",
    ".cekkaya Budi",
    "kekayaan",
    [
        "Sultan! Crazy rich! 💎",
        "Tajir melintir! 💰",
        "Lumayan berada 💵",
        "Cukup lah buat hidup 😊",
        "Semangat nabung! 🙏"
    ]
);
