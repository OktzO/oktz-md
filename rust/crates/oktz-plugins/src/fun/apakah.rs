use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const APAKAH: [&str; 20] = [
    "Ya, tentu saja!",
    "Tidak, sepertinya tidak.",
    "Mungkin saja, coba lagi nanti.",
    "Hmm... aku rasa iya.",
    "Aku ragu, tapi bisa jadi.",
    "Pasti! 100%!",
    "Tidak mungkin.",
    "Bisa jadi, siapa yang tau?",
    "Menurutku sih iya.",
    "Wah, kayaknya nggak deh.",
    "Tentu, kenapa tidak?",
    "Aku nggak tau, coba tanya yang lain.",
    "Ya ampun, pasti lah!",
    "Hmm... sepertinya tidak.",
    "Aku yakin iya!",
    "Nggak mungkin banget.",
    "Mungkin, tapi jangan berharap terlalu tinggi.",
    "Iya dong!",
    "Nggak, maaf ya.",
    "Bisa! Semangat!",
];

pub(crate) const HELP_APAKAH: &str =
    "❓ *ᴀᴘᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .apakah aku bisa jadi kaya?";

tanya_plugin!(
    apakah,
    "apakah",
    ["apa"],
    "Tanya bot apakah sesuatu",
    ".apakah <pertanyaan>",
    ".apakah aku bisa kaya?",
    APAKAH,
    HELP_APAKAH
);
