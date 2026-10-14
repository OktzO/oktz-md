use crate::common::{tanya_plugin, tanya_reply};
use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const HARUSKAH: [&str; 20] = [
    "Ya, harus!",
    "Tidak usah.",
    "Hmm, terserah kamu sih.",
    "Harus banget! Jangan ragu!",
    "Nggak harus juga.",
    "Kalau menurutmu perlu, lakukan!",
    "Pikir dulu baik-baik.",
    "Harus! Sekarang!",
    "Jangan, mending tunggu dulu.",
    "Harus, tapi hati-hati.",
    "Nggak harus, tapi boleh.",
    "Wajib!",
    "Hmm, skip aja deh.",
    "Lakukan kalau sudah yakin.",
    "Harus, demi masa depanmu!",
    "Nggak harus, santai aja.",
    "Go for it!",
    "Jangan buru-buru, pikir lagi.",
    "Tentu harus!",
    "Lihat situasinya dulu.",
];

pub(crate) const HELP_HARUSKAH: &str =
    "⚖️ *ʜᴀʀᴜsᴋᴀʜ*\n\n> Masukkan pertanyaan!\n\n*Contoh:*\n> .haruskah aku menyatakan cinta?";

tanya_plugin!(
    haruskah,
    "haruskah",
    ["harus", "should"],
    "Tanya bot haruskah sesuatu",
    ".haruskah <pertanyaan>",
    ".haruskah aku menyatakan cinta?",
    HARUSKAH,
    HELP_HARUSKAH
);

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "haruskah".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("haruskah").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("haruskah").unwrap().meta;
        assert_eq!(m.name, "haruskah");
        assert_eq!(m.alias, ["harus", "should"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Tanya bot haruskah sesuatu");
        assert_eq!(m.usage, ".haruskah <pertanyaan>");
        assert_eq!(m.example, ".haruskah aku menyatakan cinta?");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["harus", "should"] {
            assert_eq!(find(a).unwrap().meta.name, "haruskah", "alias {a}");
        }
    }

    #[test]
    fn an_empty_or_whitespace_question_returns_the_help_text() {
        for empty in ["", "   "] {
            assert_eq!(text_of(&run(empty.clone())), HELP_HARUSKAH, "input {empty:?}");
        }
    }

    #[test]
    fn the_question_is_echoed_and_the_verdict_is_one_of_the_js_entries() {
        for _ in 0..50 {
            let t = text_of(&run("aku nyanyi")).to_string();
            let verdict = t.strip_prefix("haruskah aku nyanyi?\n*").unwrap();
            let verdict = verdict.strip_suffix('*').unwrap();
            assert!(HARUSKAH.contains(&verdict), "di luar daftar JS: {verdict}");
        }
    }

    #[test]
    fn the_question_mark_is_appended_even_when_the_input_already_has_one() {
        // JS: `${m.body.slice(1)}?` — `?` ditambahkan tanpa syarat, jadi
        // input yang sudah berakhir `?` jadi `??`. Bukan salah port.
        let __balasan = run("aku nyanyi?");
        let t = text_of(&__balasan);
        assert!(t.starts_with("haruskah aku nyanyi??\n*"), "{t}");
    }

    #[test]
    fn the_table_matches_the_js_answer_list() {
        assert_eq!(HARUSKAH.len(), 20);
        assert_eq!(HARUSKAH[0], "Ya, harus!");
        assert_eq!(HARUSKAH[19], "Lihat situasinya dulu.");
    }
}
