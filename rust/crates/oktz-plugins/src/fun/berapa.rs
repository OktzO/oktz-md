use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const BERAPA: [&str; 21] = [
    "1",
    "7",
    "12",
    "21",
    "99",
    "69",
    "100",
    "50",
    "25",
    "1000",
    "5",
    "17",
    "88",
    "33",
    "nothing (jawabannya selalu nothing)",
    "Banyak banget!",
    "Cuma sedikit.",
    "Tak terhitung!",
    "Hmm, sekitar 10-an.",
    "Lebih dari yang kamu kira!",
    "Gak tau ah, males",
];

pub(crate) const HELP_BERAPA: &str =
    "🔢 *ʙᴇʀᴀᴘᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .berapa umur jodohku?";

tanya_plugin!(
    berapa,
    "berapa",
    ["howmuch", "howmany"],
    "Tanya bot berapa sesuatu",
    ".berapa <pertanyaan>",
    ".berapa umur jodohku?",
    BERAPA,
    HELP_BERAPA
);
