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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "berapa".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("berapa").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("berapa").unwrap();
        assert_eq!(p.meta.name, "berapa");
        assert_eq!(p.meta.alias, ["howmuch", "howmany"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Tanya bot berapa sesuatu");
        assert_eq!(p.meta.usage, ".berapa <pertanyaan>");
        assert_eq!(p.meta.example, ".berapa umur jodohku?");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("howmuch").unwrap().meta.name, "berapa");
        assert_eq!(oktz_plugin_api::find("howmany").unwrap().meta.name, "berapa");
    }

    #[test]
    fn an_empty_question_shows_the_help_card() {
        for text in ["", "   ", "\t "] {
            assert_eq!(run(text.clone()).text_body(), Some(HELP_BERAPA), "input {text:?}");
        }
    }

    #[test]
    fn a_question_is_echoed_with_one_of_the_bundled_answers() {
        // `m.body.slice(1)` membuang prefix, jadi "berapa" ikut echoed.
        let t = run("umur jodohku").text_body().unwrap().to_string();
        let (question, answer) = t.split_once("\n*").expect("harus `pertanyaan?\\n*jawaban*`");
        assert_eq!(question, "berapa umur jodohku?");
        let answer = answer.strip_suffix('*').expect("penutup *");
        assert!(BERAPA.contains(&answer), "jawaban di luar daftar: {answer}");
    }

    #[test]
    fn every_answer_can_be_reached_and_none_is_empty() {
        assert!(!BERAPA.is_empty());
        for a in BERAPA {
            assert!(!a.is_empty(), "jawaban kosong tidak mungkin di JS");
        }
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let t = run("x").text_body().unwrap().to_string();
            let a = t.split_once("\n*").unwrap().1.strip_suffix('*').unwrap().to_string();
            seen.insert(a);
        }
        assert!(seen.len() > 1, "jawaban selalu sama: {seen:?}");
    }
}
