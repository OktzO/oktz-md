use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const MENGAPA: [&str; 20] = [
    "Karena memang sudah takdirnya begitu.",
    "Hmm, pertanyaan bagus! Aku juga bingung.",
    "Karena itulah cara kerjanya.",
    "Karena Tuhan berkehendak demikian.",
    "Aku nggak tau, cari di Google aja.",
    "Karena ya gitu aja.",
    "Mungkin karena kebetulan?",
    "Karena dunia memang penuh misteri.",
    "Hmm, sulit dijelaskan sih.",
    "Karena alam semesta bekerja dengan cara yang misterius.",
    "Aku juga penasaran, kenapa ya?",
    "Karena hal tersebut memang seharusnya terjadi.",
    "Pertanyaan yang bagus! Sayangnya aku nggak punya jawabannya.",
    "Karena itulah keunikan hidup.",
    "Karena setiap hal punya alasannya masing-masing.",
    "Hmm... aku butuh waktu untuk memikirkannya.",
    "Karena begitulah logikanya.",
    "Aku rasa karena memang harus begitu.",
    "Karena segala sesuatu saling berhubungan.",
    "Nah itu aku juga mikir!",
];

pub(crate) const HELP_MENGAPA: &str =
    "🤔 *ᴍᴇɴɢᴀᴘᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .mengapa langit biru?";

tanya_plugin!(
    mengapa,
    "mengapa",
    ["kenapa", "why"],
    "Tanya bot mengapa sesuatu",
    ".mengapa <pertanyaan>",
    ".mengapa langit biru?",
    MENGAPA,
    HELP_MENGAPA
);
