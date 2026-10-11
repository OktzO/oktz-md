use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const BISAKAH: [&str; 20] = [
    "Bisa banget! Percaya diri aja!",
    "Hmm, kayaknya susah deh.",
    "Tentu bisa! Semangat!",
    "Nggak bisa, maaf.",
    "Mungkin bisa, kalau usaha keras.",
    "Pasti bisa! Jangan menyerah!",
    "Agak susah sih, tapi bisa dicoba.",
    "Bisa kok! Yakin deh!",
    "Kayaknya nggak deh.",
    "Bisa! Ayo buktikan!",
    "Hmm... aku ragu.",
    "Bisa banget! Gas terus!",
    "Nggak bisa, coba yang lain.",
    "Bisa! Percaya sama diri sendiri!",
    "Susah, tapi bukan berarti nggak mungkin.",
    "Absolutely! Kamu pasti bisa!",
    "Kayaknya perlu usaha ekstra nih.",
    "Bisa! Jangan ragukan dirimu!",
    "Hmm, coba lagi nanti deh.",
    "Bisa! Aku percaya kamu!",
];

pub(crate) const HELP_BISAKAH: &str =
    "💪 *ʙɪsᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bisakah aku lulus ujian?";

tanya_plugin!(
    bisakah,
    "bisakah",
    ["bisa"],
    "Tanya bot bisakah sesuatu",
    ".bisakah <pertanyaan>",
    ".bisakah aku lulus ujian?",
    BISAKAH,
    HELP_BISAKAH
);
