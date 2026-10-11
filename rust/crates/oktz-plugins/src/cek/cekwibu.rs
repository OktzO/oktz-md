use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekwibu,
    "cekwibu",
    ["wibu", "weeb"],
    "Cek seberapa wibu kamu",
    ".cekwibu <nama>",
    ".cekwibu Budi",
    "kewibuan",
    [
        "WIBU SEJATI! Ara ara~ 🎌",
        "Wibu parah! Kimochi~ 😍",
        "Lumayan wibu 🌸",
        "Sedikit wibu 😊",
        "Bukan wibu, normal! 😎"
    ]
);
