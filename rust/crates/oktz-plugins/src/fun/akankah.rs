use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const AKANKAH: [&str; 20] = [
    "Ya, pasti akan terjadi!",
    "Tidak, sepertinya tidak akan.",
    "Mungkin akan, mungkin tidak.",
    "InsyaAllah akan terjadi!",
    "Hmm, sulit diprediksi.",
    "Pasti! Yakin saja!",
    "Kayaknya nggak deh.",
    "Akan terjadi kalau kamu mau berusaha.",
    "Suatu saat nanti, pasti.",
    "Nggak akan, maaf.",
    "Tentu akan! Tunggu saja!",
    "Hmm, aku ragu.",
    "Akan! Percaya sama proses!",
    "Kemungkinannya kecil.",
    "Pasti akan, aku yakin!",
    "Nggak akan, cari yang lain aja.",
    "Akan, tapi butuh waktu.",
    "InsyaAllah!",
    "Kalau jodoh, pasti akan.",
    "Akan terjadi di saat yang tepat!",
];

pub(crate) const HELP_AKANKAH: &str =
    "🔮 *ᴀᴋᴀɴᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .akankah aku sukses?";

tanya_plugin!(
    akankah,
    "akankah",
    ["akan", "will"],
    "Tanya bot akankah sesuatu terjadi",
    ".akankah <pertanyaan>",
    ".akankah aku sukses?",
    AKANKAH,
    HELP_AKANKAH
);
