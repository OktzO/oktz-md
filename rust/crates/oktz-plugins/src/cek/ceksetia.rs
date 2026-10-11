use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    ceksetia,
    "ceksetia",
    ["setia", "loyal"],
    "Cek tingkat kesetiaan kamu",
    ".ceksetia <nama>",
    ".ceksetia Budi",
    "kesetiaan",
    [
        "Setia sampai mati! 💍💕",
        "Sangat setia dan tulus! ❤️",
        "Cukup setia~ 😊",
        "Hmm... kadang goyah 😅",
        "Playboy/Playgirl mode? 😏"
    ]
);
