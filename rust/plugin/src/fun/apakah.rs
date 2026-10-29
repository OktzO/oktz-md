use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const APAKAH: [&str; 20] = [
    "Ya, tentu saja!",
    "Tidak, sepertinya tidak.",
    "Mungkin saja, coba lagi nanti.",
    "Hmm... aku rasa iya.",
    "Aku ragu, tapi bisa jadi.",
    "Pasti! 100%!",
    "Tidak mungkin.",
    "Bisa jadi, siapa yang tau?",
    "Menurutku sih iya.",
    "Wah, kayaknya nggak deh.",
    "Tentu, kenapa tidak?",
    "Aku nggak tau, coba tanya yang lain.",
    "Ya ampun, pasti lah!",
    "Hmm... sepertinya tidak.",
    "Aku yakin iya!",
    "Nggak mungkin banget.",
    "Mungkin, tapi jangan berharap terlalu tinggi.",
    "Iya dong!",
    "Nggak, maaf ya.",
    "Bisa! Semangat!",
];

pub(crate) const HELP_APAKAH: &str =
    "❓ *ᴀᴘᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .apakah aku bisa jadi kaya?";

tanya_plugin!(
    apakah,
    "apakah",
    ["apa"],
    "Tanya bot apakah sesuatu",
    ".apakah <pertanyaan>",
    ".apakah aku bisa kaya?",
    APAKAH,
    HELP_APAKAH
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "apakah".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("apakah").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("apakah").unwrap().meta;
        assert_eq!(m.name, "apakah");
        assert_eq!(m.alias, ["apa"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot apakah sesuatu");
        assert_eq!(m.usage, ".apakah <pertanyaan>");
        assert_eq!(m.example, ".apakah aku bisa kaya?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert_eq!(find("apa").unwrap().meta.name, "apakah");
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "  ", "\n"] {
            assert_eq!(text_of(&run(empty)), HELP_APAKAH, "input {empty:?}");
        }
    }

    #[test]
    fn the_question_is_echoed_from_the_body_without_the_prefix() {
        let __balasan = run("aku bisa kaya");
        let t = text_of(&__balasan);
        assert!(t.starts_with("apakah aku bisa kaya?\n*"), "{t}");
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let __balasan = run("aku bisa kaya?");
        let t = text_of(&__balasan);
        assert!(t.starts_with("apakah aku bisa kaya??\n*"), "{t}");
    }

    #[test]
    fn the_answer_is_one_of_the_twenty_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("hari ini uang")).to_string();
            let answer = t.strip_prefix("apakah hari ini uang?\n*").unwrap();
            let answer = answer.strip_suffix('*').unwrap();
            assert!(APAKAH.contains(&answer), "di luar daftar JS: {answer}");
        }
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(APAKAH.len(), 20);
        assert_eq!(APAKAH[0], "Ya, tentu saja!");
        assert_eq!(APAKAH[19], "Bisa! Semangat!");
    }
}
