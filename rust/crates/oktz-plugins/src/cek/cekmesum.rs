use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekmesum,
    "cekmesum",
    ["mesum"],
    "Cek seberapa mesum kamu",
    ".cekmesum <nama>",
    ".cekmesum Budi",
    "kemesuman",
    [
        "MESUM AKUT! Tobat mas! 😳🔞",
        "Mesum banget! 👀",
        "Lumayan mesum 😏",
        "Sedikit mesum 🙈",
        "Polos dan suci! 😇"
    ]
);
