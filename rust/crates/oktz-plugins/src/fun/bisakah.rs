use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const BISAKAH: [&str; 20] = [
    "Bisa banget! Percaya diri aja!",
    "Hmm, kayaknya susah deh.",
    "Tentu bisa! Semangat!",
    "Nggak bisa, maaf.",
    "Mungkin bisa, kalau usaha keras.",
    "Pasti bisa! Jangan menyerah!",
    "Agak susah sih, tapi bisa dicoba.",
    "Bisa kok! Yakin deh!",
    "Kayaknya nggak deh.",
    "Bisa! Ayo buktikan!",
    "Hmm... aku ragu.",
    "Bisa banget! Gas terus!",
    "Nggak bisa, coba yang lain.",
    "Bisa! Percaya sama diri sendiri!",
    "Susah, tapi bukan berarti nggak mungkin.",
    "Absolutely! Kamu pasti bisa!",
    "Kayaknya perlu usaha ekstra nih.",
    "Bisa! Jangan ragukan dirimu!",
    "Hmm, coba lagi nanti deh.",
    "Bisa! Aku percaya kamu!",
];

pub(crate) const HELP_BISAKAH: &str =
    "💪 *ʙɪsᴀᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bisakah aku lulus ujian?";

tanya_plugin!(
    bisakah,
    "bisakah",
    ["bisa"],
    "Tanya bot bisakah sesuatu",
    ".bisakah <pertanyaan>",
    ".bisakah aku lulus ujian?",
    BISAKAH,
    HELP_BISAKAH
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bisakah".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("bisakah").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("bisakah").unwrap().meta;
        assert_eq!(m.name, "bisakah");
        assert_eq!(m.alias, ["bisa"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot bisakah sesuatu");
        assert_eq!(m.usage, ".bisakah <pertanyaan>");
        assert_eq!(m.example, ".bisakah aku lulus ujian?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert_eq!(find("bisa").unwrap().meta.name, "bisakah");
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "  "] {
            assert_eq!(text_of(&run(empty.clone())), HELP_BISAKAH, "input {empty:?}");
        }
    }

    #[test]
    fn the_question_is_echoed_and_the_answer_is_one_of_the_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("aku lulus ujian")).to_string();
            let answer = t.strip_prefix("bisakah aku lulus ujian?\n*").unwrap();
            let answer = answer.strip_suffix('*').unwrap();
            assert!(BISAKAH.contains(&answer), "di luar daftar JS: {answer}");
        }
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let __balasan = run("aku lulus ujian?");
        let t = text_of(&__balasan);
        assert!(t.starts_with("bisakah aku lulus ujian??\n*"), "{t}");
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(BISAKAH.len(), 20);
        assert_eq!(BISAKAH[0], "Bisa banget! Percaya diri aja!");
        assert_eq!(BISAKAH[19], "Bisa! Aku percaya kamu!");
    }
}
