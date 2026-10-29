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

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mengapa".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("mengapa").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("mengapa").unwrap().meta;
        assert_eq!(m.name, "mengapa");
        assert_eq!(m.alias, ["kenapa", "why"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot mengapa sesuatu");
        assert_eq!(m.usage, ".mengapa <pertanyaan>");
        assert_eq!(m.example, ".mengapa langit biru?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["kenapa", "why"] {
            assert_eq!(find(a).unwrap().meta.name, "mengapa", "alias {a}");
        }
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "\t\t"] {
            assert_eq!(text_of(&run(empty)), HELP_MENGAPA, "input {empty:?}");
        }
    }

    #[test]
    fn the_question_is_echoed_and_the_reason_is_one_of_the_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("langit biru")).to_string();
            let reason = t.strip_prefix("mengapa langit biru?\n*").unwrap();
            let reason = reason.strip_suffix('*').unwrap();
            assert!(MENGAPA.contains(&reason), "di luar daftar JS: {reason}");
        }
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let __balasan = run("langit biru?");
        let t = text_of(&__balasan);
        assert!(t.starts_with("mengapa langit biru??\n*"), "{t}");
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(MENGAPA.len(), 20);
        assert_eq!(MENGAPA[0], "Karena memang sudah takdirnya begitu.");
        assert_eq!(MENGAPA[19], "Nah itu aku juga mikir!");
    }
}
