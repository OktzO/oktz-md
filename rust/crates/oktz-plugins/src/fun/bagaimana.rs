use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const BAGAIMANA: [&str; 20] = [
    "Caranya gampang, ya tinggal dilakuin aja!",
    "Hmm, susah dijelasin sih. Coba aja dulu!",
    "Dengan usaha dan doa pastinya.",
    "Ya begitulah caranya.",
    "Aku kurang tau sih, coba cari referensi lain.",
    "Pelan-pelan aja, nanti juga bisa.",
    "Dengan kerja keras dan pantang menyerah!",
    "Pertama, percaya sama diri sendiri dulu.",
    "Hmm, tiap orang beda-beda sih caranya.",
    "Ikutin kata hatimu aja.",
    "Belajar dari yang sudah berpengalaman.",
    "Step by step, jangan terburu-buru.",
    "Dengan tekad yang kuat!",
    "Mulai dari yang kecil dulu.",
    "Konsisten aja, nanti juga bisa.",
    "Jangan overthinking, langsung action!",
    "Gampang! Tinggal mulai aja!",
    "Caranya? Ya dicoba dulu!",
    "Dengan strategi yang tepat.",
    "Hmm, aku juga masih belajar sih.",
];

pub(crate) const HELP_BAGAIMANA: &str =
    "📋 *ʙᴀɢᴀɪᴍᴀɴᴀ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .bagaimana cara jadi sukses?";

tanya_plugin!(
    bagaimana,
    "bagaimana",
    ["gimana", "how"],
    "Tanya bot bagaimana sesuatu",
    ".bagaimana <pertanyaan>",
    ".bagaimana cara jadi sukses?",
    BAGAIMANA,
    HELP_BAGAIMANA
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(command: &str, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("bagaimana").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("bagaimana").unwrap().meta;
        assert_eq!(m.name, "bagaimana");
        assert_eq!(m.alias, ["gimana", "how"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot bagaimana sesuatu");
        assert_eq!(m.usage, ".bagaimana <pertanyaan>");
        assert_eq!(m.example, ".bagaimana cara jadi sukses?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["gimana", "how"] {
            assert_eq!(find(a).unwrap().meta.name, "bagaimana", "alias {a}");
        }
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "   "] {
            assert_eq!(text_of(&run("bagaimana", empty)), HELP_BAGAIMANA);
        }
    }

    #[test]
    fn the_echo_uses_the_invoked_command_word_not_the_canonical_one() {
        // JS `m.body.slice(1)` memakai kata yang diketik user.
        let t = text_of(&run("gimana", "cara belajar"));
        assert!(t.starts_with("gimana cara belajar?\n*"), "{t}");
        let t = text_of(&run("bagaimana", "cara belajar"));
        assert!(t.starts_with("bagaimana cara belajar?\n*"), "{t}");
    }

    #[test]
    fn the_answer_is_one_of_the_twenty_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("bagaimana", "bisu")).to_string();
            let answer = t.strip_prefix("bagaimana bisu?\n*").unwrap();
            let answer = answer.strip_suffix('*').unwrap();
            assert!(BAGAIMANA.contains(&answer), "di luar daftar JS: {answer}");
        }
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(BAGAIMANA.len(), 20);
        assert_eq!(BAGAIMANA[0], "Caranya gampang, ya tinggal dilakuin aja!");
        assert_eq!(BAGAIMANA[19], "Hmm, aku juga masih belajar sih.");
    }
}
