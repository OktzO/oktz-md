use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const AKANKAH: [&str; 20] = [
    "Ya, pasti akan terjadi!",
    "Tidak, sepertinya tidak akan.",
    "Mungkin akan, mungkin tidak.",
    "InsyaAllah akan terjadi!",
    "Hmm, sulit diprediksi.",
    "Pasti! Yakin saja!",
    "Kayaknya nggak deh.",
    "Akan terjadi kalau kamu mau berusaha.",
    "Suatu saat nanti, pasti.",
    "Nggak akan, maaf.",
    "Tentu akan! Tunggu saja!",
    "Hmm, aku ragu.",
    "Akan! Percaya sama proses!",
    "Kemungkinannya kecil.",
    "Pasti akan, aku yakin!",
    "Nggak akan, cari yang lain aja.",
    "Akan, tapi butuh waktu.",
    "InsyaAllah!",
    "Kalau jodoh, pasti akan.",
    "Akan terjadi di saat yang tepat!",
];

pub(crate) const HELP_AKANKAH: &str =
    "🔮 *ᴀᴋᴀɴᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .akankah aku sukses?";

tanya_plugin!(
    akankah,
    "akankah",
    ["akan", "will"],
    "Tanya bot akankah sesuatu terjadi",
    ".akankah <pertanyaan>",
    ".akankah aku sukses?",
    AKANKAH,
    HELP_AKANKAH
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "akankah".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("akankah").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("akankah").unwrap().meta;
        assert_eq!(m.name, "akankah");
        assert_eq!(m.alias, ["akan", "will"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot akankah sesuatu terjadi");
        assert_eq!(m.usage, ".akankah <pertanyaan>");
        assert_eq!(m.example, ".akankah aku sukses?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["akan", "will"] {
            assert_eq!(find(a).unwrap().meta.name, "akankah", "alias {a}");
        }
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "   ", "\t"] {
            assert_eq!(text_of(&run(empty)), HELP_AKANKAH, "input {empty:?}");
        }
    }

    #[test]
    fn the_question_is_echoed_from_the_body_without_the_prefix() {
        let __balasan = run("aku sukses");
        let t = text_of(&__balasan);
        assert!(t.starts_with("akankah aku sukses?\n*"), "{t}");
        assert!(t.ends_with('*'), "{t}");
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        assert!(
            text_of(&run("aku sukses?")).starts_with("akankah aku sukses??\n*"),
            "{}",
            text_of(&run("aku sukses?"))
        );
    }

    #[test]
    fn the_answer_is_one_of_the_twenty_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("besok lulus")).to_string();
            let answer = t.strip_prefix("akankah besok lulus?\n*").unwrap();
            let answer = answer.strip_suffix('*').unwrap();
            assert!(
                AKANKAH.contains(&answer),
                "jawaban di luar daftar JS: {answer}"
            );
        }
    }

    #[test]
    fn the_table_is_the_twenty_answers_from_the_js_file() {
        assert_eq!(AKANKAH.len(), 20);
        assert_eq!(AKANKAH[0], "Ya, pasti akan terjadi!");
        assert_eq!(AKANKAH[19], "Akan terjadi di saat yang tepat!");
    }
}
