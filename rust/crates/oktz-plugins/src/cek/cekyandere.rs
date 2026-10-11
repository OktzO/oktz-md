use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekyandere,
    "cekyandere",
    ["yandere"],
    "Cek tingkat yandere kamu",
    ".cekyandere <nama>",
    ".cekyandere Budi",
    "keyanderean",
    [
        "Kamu milikku selamanya~ 🔪💕",
        "Jangan dekati dia ya... 👁️",
        "Overprotective sedikit~ 🫂",
        "Agak posesif 😅",
        "Normal kok, santai~ 😊"
    ]
);
