use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const HARUSKAH: [&str; 20] = [
    "Ya, harus!",
    "Tidak usah.",
    "Hmm, terserah kamu sih.",
    "Harus banget! Jangan ragu!",
    "Nggak harus juga.",
    "Kalau menurutmu perlu, lakukan!",
    "Pikir dulu baik-baik.",
    "Harus! Sekarang!",
    "Jangan, mending tunggu dulu.",
    "Harus, tapi hati-hati.",
    "Nggak harus, tapi boleh.",
    "Wajib!",
    "Hmm, skip aja deh.",
    "Lakukan kalau sudah yakin.",
    "Harus, demi masa depanmu!",
    "Nggak harus, santai aja.",
    "Go for it!",
    "Jangan buru-buru, pikir lagi.",
    "Tentu harus!",
    "Lihat situasinya dulu.",
];

pub(crate) const HELP_HARUSKAH: &str =
    "⚖️ *ʜᴀʀᴜsᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .haruskah aku menyatakan cinta?";

tanya_plugin!(
    haruskah,
    "haruskah",
    ["harus", "should"],
    "Tanya bot haruskah sesuatu",
    ".haruskah <pertanyaan>",
    ".haruskah aku menyatakan cinta?",
    HARUSKAH,
    HELP_HARUSKAH
);
