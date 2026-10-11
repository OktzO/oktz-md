use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const BAGAIMANA: [&str; 20] = [
    "Caranya gampang, ya tinggal dilakuin aja!",
    "Hmm, susah dijelasin sih. Coba aja dulu!",
    "Dengan usaha dan doa pastinya.",
    "Ya begitulah caranya.",
    "Aku kurang tau sih, coba cari referensi lain.",
    "Pelan-pelan aja, nanti juga bisa.",
    "Dengan kerja keras dan pantang menyerah!",
    "Pertama, percaya sama diri sendiri dulu.",
    "Hmm, tiap orang beda-beda sih caranya.",
    "Ikutin kata hatimu aja.",
    "Belajar dari yang sudah berpengalaman.",
    "Step by step, jangan terburu-buru.",
    "Dengan tekad yang kuat!",
    "Mulai dari yang kecil dulu.",
    "Konsisten aja, nanti juga bisa.",
    "Jangan overthinking, langsung action!",
    "Gampang! Tinggal mulai aja!",
    "Caranya? Ya dicoba dulu!",
    "Dengan strategi yang tepat.",
    "Hmm, aku juga masih belajar sih.",
];

pub(crate) const HELP_BAGAIMANA: &str =
    "📋 *ʙᴀɢᴀɪᴍᴀɴᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bagaimana cara jadi sukses?";

tanya_plugin!(
    bagaimana,
    "bagaimana",
    ["gimana", "how"],
    "Tanya bot bagaimana sesuatu",
    ".bagaimana <pertanyaan>",
    ".bagaimana cara jadi sukses?",
    BAGAIMANA,
    HELP_BAGAIMANA
);
