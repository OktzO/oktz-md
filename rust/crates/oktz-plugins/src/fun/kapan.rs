use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const KAPAN: [&str; 20] = [
    "Besok mungkin?",
    "Tahun depan kayaknya.",
    "3 hari lagi!",
    "Hmm, masih lama sih.",
    "Sebentar lagi kok!",
    "Kalau sudah waktunya, pasti terjadi.",
    "Bulan depan!",
    "Entah kapan, yang penting sabar.",
    "Dalam waktu dekat!",
    "10 tahun lagi mungkin?",
    "Nggak lama lagi!",
    "Kalau jodoh, pasti ketemu.",
    "Hmm, susah diprediksi.",
    "Minggu depan!",
    "Kalau usahanya lebih keras, lebih cepat!",
    "Pas waktunya tepat.",
    "Secepatnya, tenang aja.",
    "Ntar kalo udah siap.",
    "Dalam hitungan hari!",
    "Saat kamu sudah siap menerimanya.",
];

pub(crate) const HELP_KAPAN: &str =
    "⏰ *ᴋᴀᴘᴀɴ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .kapan aku nikah?";

tanya_plugin!(
    kapan,
    "kapan",
    ["when"],
    "Tanya bot kapan sesuatu",
    ".kapan <pertanyaan>",
    ".kapan aku nikah?",
    KAPAN,
    HELP_KAPAN
);
