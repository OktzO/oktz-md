use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const DIMANA: [&str; 20] = [
    "Di dekatmu!",
    "Jauh di sana.",
    "Di tempat yang tidak kamu duga.",
    "Di hatimu.",
    "Di sekitar sini.",
    "Hmm, coba cari di kamar.",
    "Di luar sana, menunggumu.",
    "Di tempat yang sama denganmu.",
    "Di suatu tempat yang indah.",
    "Di balik pintu.",
    "Di sebelah kirimu.",
    "Di depan matamu!",
    "Jauh banget, di luar negeri mungkin?",
    "Di tempat yang penuh kenangan.",
    "Di mana-mana!",
    "Di dunia maya.",
    "Di alam mimpi.",
    "Di tempat rahasia.",
    "Hmm, susah dijelaskan lokasinya.",
    "Di tempat yang akan membuatmu bahagia.",
];

pub(crate) const HELP_DIMANA: &str =
    "📍 *ᴅɪᴍᴀɴᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .dimana jodohku berada?";

tanya_plugin!(
    dimana,
    "dimana",
    ["where", "mana"],
    "Tanya bot dimana sesuatu",
    ".dimana <pertanyaan>",
    ".dimana jodohku berada?",
    DIMANA,
    HELP_DIMANA
);
