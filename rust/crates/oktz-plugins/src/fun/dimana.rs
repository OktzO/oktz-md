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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "dimana".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("dimana").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("dimana").unwrap();
        assert_eq!(p.meta.name, "dimana");
        assert_eq!(p.meta.alias, ["where", "mana"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Tanya bot dimana sesuatu");
        assert_eq!(p.meta.usage, ".dimana <pertanyaan>");
        assert_eq!(p.meta.example, ".dimana jodohku berada?");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("where").unwrap().meta.name, "dimana");
        assert_eq!(oktz_plugin_api::find("mana").unwrap().meta.name, "dimana");
    }

    #[test]
    fn an_empty_question_shows_the_help_card() {
        for text in ["", "  ", "\n\t"] {
            assert_eq!(run(text).text_body(), Some(HELP_DIMANA), "input {text:?}");
        }
    }

    #[test]
    fn a_question_is_echoed_with_one_of_the_bundled_answers() {
        let t = run("jodohku berada").text_body().unwrap().to_string();
        let (question, answer) = t.split_once("\n*").expect("harus `pertanyaan?\\n*jawaban*`");
        assert_eq!(question, "dimana jodohku berada?");
        let answer = answer.strip_suffix('*').expect("penutup *");
        assert!(DIMANA.contains(&answer), "jawaban di luar daftar: {answer}");
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let t = run("jodohku berada?").text_body().unwrap().to_string();
        assert!(t.starts_with("dimana jodohku berada??\n*"), "{t}");
    }

    #[test]
    fn the_answer_pool_is_not_a_single_constant() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let t = run("x").text_body().unwrap().to_string();
            seen.insert(t.split_once("\n*").unwrap().1.to_string());
        }
        assert!(seen.len() > 1, "jawaban selalu sama: {seen:?}");
    }
}
