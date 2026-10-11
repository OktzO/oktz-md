use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekrezeki,
    "cekrezeki",
    ["rezeki", "fortune"],
    "Cek tingkat rezeki kamu hari ini",
    ".cekrezeki <nama>",
    ".cekrezeki Budi",
    "kerezekian",
    [
        "Rezeki melimpah! Jackpot! 💰🎉",
        "Rezeki lancar hari ini~ 💵",
        "Rezeki cukup, bersyukurlah 🙏",
        "Rezeki pas-pasan 😅",
        "Sabar ya, rezeki akan datang~ 🫂"
    ]
);
