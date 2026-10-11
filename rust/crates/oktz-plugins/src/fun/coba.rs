use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const COBA: [&str; 20] = [
    "Hmm, aku coba ya... Kamu lagi mikirin makanan!",
    "Aku teebak... Kamu lagi gabut!",
    "Coba ya... Kayaknya kamu lagi seneng!",
    "Hmm, aku rasa kamu lagi bingung.",
    "Aku coba nebak... Kamu lagi kangen seseorang?",
    "Kayaknya kamu lagi santai deh.",
    "Aku teebak kamu lagi scroll HP terus.",
    "Hmm, pasti lagi bosan ya?",
    "Coba diteebak... Kamu lagi pengen jalan-jalan!",
    "Aku rasa kamu lagi butuh hiburan.",
    "Hmm, kayaknya kamu lagi happy!",
    "Aku coba... Kamu pasti lagi penasaran!",
    "Tebakan aku: kamu lagi rebahan.",
    "Hmm, kamu mungkin lagi mikirin seseorang spesial.",
    "Aku coba: kamu lagi mau curhat?",
    "Kayaknya kamu lagi pengen main game!",
    "Hmm, aku teebak kamu lagi dengerin musik.",
    "Coba aku teebak... Kamu lagi di kamar!",
    "Aku rasa kamu lagi waiting for something.",
    "Hmm, tebakan aku: kamu butuh temen ngobrol!",
];

pub(crate) const HELP_COBA: &str =
    "🎯 *ᴄᴏʙᴀ*\n\n> Masukkan sesuatu!\n\n*Contoh:*\n> .coba tebak apa yang aku pikirkan";

tanya_plugin!(
    coba,
    "coba",
    ["try"],
    "Coba tanyakan sesuatu ke bot",
    ".coba <pertanyaan>",
    ".coba tebak apa yang aku pikirkan",
    COBA,
    HELP_COBA
);
