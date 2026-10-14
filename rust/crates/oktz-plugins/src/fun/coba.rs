use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const COBA: [&str; 20] = [
    "Hmm, aku coba ya... Kamu lagi mikirin makanan!",
    "Aku tebak... Kamu lagi gabut!",
    "Coba ya... Kayaknya kamu lagi seneng!",
    "Hmm, aku rasa kamu lagi bingung.",
    "Aku coba nebak... Kamu lagi kangen seseorang?",
    "Kayaknya kamu lagi santai deh.",
    "Aku tebak kamu lagi scroll HP terus.",
    "Hmm, pasti lagi bosan ya?",
    "Coba ditebak... Kamu lagi pengen jalan-jalan!",
    "Aku rasa kamu lagi butuh hiburan.",
    "Hmm, kayaknya kamu lagi happy!",
    "Aku coba... Kamu pasti lagi penasaran!",
    "Tebakan aku: kamu lagi rebahan.",
    "Hmm, kamu mungkin lagi mikirin seseorang spesial.",
    "Aku coba: kamu lagi mau curhat?",
    "Kayaknya kamu lagi pengen main game!",
    "Hmm, aku tebak kamu lagi dengerin musik.",
    "Coba aku tebak... Kamu lagi di kamar!",
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

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "coba".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("coba").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("coba").unwrap().meta;
        assert_eq!(m.name, "coba");
        assert_eq!(m.alias, ["try"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Coba tanyakan sesuatu ke bot");
        assert_eq!(m.usage, ".coba <pertanyaan>");
        assert_eq!(m.example, ".coba tebak apa yang aku pikirkan");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert_eq!(find("try").unwrap().meta.name, "coba");
    }

    #[test]
    fn an_empty_or_whitespace_argument_returns_the_help_text() {
        for empty in ["", "  "] {
            assert_eq!(text_of(&run(empty)), HELP_COBA, "input {empty:?}");
        }
    }

    #[test]
    fn the_argument_is_echoed_and_the_guess_is_one_of_the_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("aku lagi mikirin")).to_string();
            let guess = t.strip_prefix("coba aku lagi mikirin?\n*").unwrap();
            let guess = guess.strip_suffix('*').unwrap();
            assert!(COBA.contains(&guess), "di luar daftar JS: {guess}");
        }
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(COBA.len(), 20);
        assert_eq!(COBA[0], "Hmm, aku coba ya... Kamu lagi mikirin makanan!");
        assert_eq!(COBA[19], "Hmm, tebakan aku: kamu butuh temen ngobrol!");
    }
}
