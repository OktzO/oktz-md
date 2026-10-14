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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "kapan".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("kapan").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("kapan").unwrap();
        assert_eq!(p.meta.name, "kapan");
        assert_eq!(p.meta.alias, ["when"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.description, "Tanya bot kapan sesuatu");
        assert_eq!(p.meta.usage, ".kapan <pertanyaan>");
        assert_eq!(p.meta.example, ".kapan aku nikah?");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("when").unwrap().meta.name, "kapan");
    }

    #[test]
    fn an_empty_question_shows_the_help_card() {
        for text in ["", "   ", "\t"] {
            assert_eq!(run(text).text_body(), Some(HELP_KAPAN), "input {text:?}");
        }
    }

    #[test]
    fn a_question_is_echoed_with_one_of_the_bundled_answers() {
        let t = run("aku nikah").text_body().unwrap().to_string();
        let (question, answer) = t.split_once("\n*").expect("harus `pertanyaan?\\n*jawaban*`");
        assert_eq!(question, "kapan aku nikah?");
        let answer = answer.strip_suffix('*').expect("penutup *");
        assert!(KAPAN.contains(&answer), "jawaban di luar daftar: {answer}");
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let t = run("aku nikah?").text_body().unwrap().to_string();
        assert!(t.starts_with("kapan aku nikah??\n*"), "{t}");
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
