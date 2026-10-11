use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekbucin,
    "cekbucin",
    ["bucin"],
    "Cek seberapa bucin kamu",
    ".cekbucin <nama>",
    ".cekbucin Budi",
    "kebucinan",
    [
        "BUCIN AKUT! Udah gabisa diselamatkan 😭💔",
        "Bucin parah nih~ 🥺",
        "Lumayan bucin 💕",
        "Sedikit bucin 😊",
        "Santai aja, gak bucin 😎"
    ]
);
