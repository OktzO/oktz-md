use crate::common::{cek_plugin, cek_reply, percent_101};
use oktz_plugin_api::{Ctx, Reply, plugin};

cek_plugin!(
    cekkece,
    "cekkece",
    ["kece", "cool"],
    "Cek seberapa kece kamu",
    ".cekkece <nama>",
    ".cekkece Budi",
    "kekecean",
    [
        "KECE BADAI! 😎🔥",
        "Kece banget! ✨",
        "Lumayan kece~ 👍",
        "Sedikit kece 😊",
        "Biasa aja, tapi tetep keren! 🙂"
    ]
);
