use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekimut,
    "cekimut",
    ["imut", "cute"],
    "Cek seberapa imut kamu",
    ".cekimut <nama>",
    ".cekimut Ani",
    "keimutan",
    [
        "IMUT BANGET! Kawaii~~ 🥺💕",
        "Imutnya kebangetan! 😍",
        "Lumayan imut~ 🌸",
        "Ada imutnya dikit 😊",
        "Mungkin cool bukan imut? 😎"
    ]
);
