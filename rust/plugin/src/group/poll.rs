//! `plugins/group/poll.js` — validasi format polling.
//!
//! `ponytail:` `sock.sendMessage(m.chat, { poll: {...} })` tidak ada di API
//! plugin (poll native WA), jadi validasi dipindah penuh tetapi pembuatan
//! poll-nya tidak. Plugin diam setelah validasi lolos — bukan menulis
//! "✅ Success membuat poll" untuk poll yang tidak pernah dikirim.
//!
//! `question.length` di JS menghitung unit UTF-16; di sini `chars().count()`,
//! yang sama untuk semua teks BMP dan hanya beda untuk emoji di luar BMP.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const MAX_OPTIONS: usize = 12;
pub(crate) const MAX_QUESTION_CHARS: usize = 255;

pub(crate) fn usage() -> &'static str {
    "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
     > Format tidak valid!\n\n\
     *Format:*\n\
     > `.poll pertanyaan | opsi1, opsi2`\n\n\
     *Contoh:*\n\
     > `.poll Makan siang apa? | Nasi Goreng, Mie Ayam`\n\n\
     *Opsi tambahan:*\n\
     > `.poll multi | pertanyaan | opsi1, opsi2, opsi3, dst`\n\
     > (untuk pilihan ganda)"
}

pub(crate) const BAD_FORMAT: &str =
    "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n> Format: `pertanyaan | opsi1, opsi2, ...`";
pub(crate) const TOO_FEW: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n> Minimal 2 opsi pilihan!";
pub(crate) const TOO_MANY: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n> Maksimal 12 opsi pilihan!";
pub(crate) const QUESTION_TOO_LONG: &str =
    "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n> Pertanyaan terlalu panjang!\n> Maksimal 255 karakter.";

/// Hasil parsing `poll.js:35-52`.
pub(crate) struct Parsed {
    /// `isMultiple` hanya dipakai untuk `selectableCount` di `sendMessage`
    /// yang tidak ada di `Ctx` — jadi hanya tes parser yang membacanya.
    #[cfg(test)]
    pub is_multiple: bool,
    pub question: String,
    pub options: Vec<String>,
}

/// `text.split('|').map(trim)` + prefiks `multi` + `parts[1].split(',')`
/// dengan `.filter(o => o)`.
pub(crate) fn parse(text: &str) -> Option<Parsed> {
    let mut parts: Vec<&str> = text.split('|').map(str::trim).collect();
    let is_multiple = parts.first().is_some_and(|p| p.to_lowercase() == "multi");
    if is_multiple {
        parts.remove(0);
    }
    if parts.len() < 2 {
        return None;
    }
    let question = parts[0].to_string();
    let options: Vec<String> = parts[1]
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .map(str::to_string)
        .collect();
    Some(Parsed {
        #[cfg(test)]
        is_multiple,
        question,
        options,
    })
}

#[plugin(
    name = "poll",
    alias = ["voting", "vote", "survei"],
    category = "group",
    description = "Buat polling/voting di grup",
    usage = ".poll <pertanyaan> | <opsi1>, <opsi2>, ...",
    example = ".poll Makan apa? | Nasi Goreng, Mie Ayam, Bakso",
    cooldown = 30,
    energi = 1,
    is_group = true,
    is_private = false
)]
pub fn poll(ctx: Ctx) -> Reply {
    if ctx.text.trim().is_empty() {
        return Reply::text(usage());
    }

    let Some(parsed) = parse(&ctx.text) else {
        return Reply::text(BAD_FORMAT);
    };

    if parsed.options.len() < 2 {
        return Reply::text(TOO_FEW);
    }
    if parsed.options.len() > MAX_OPTIONS {
        return Reply::text(TOO_MANY);
    }
    if parsed.question.chars().count() > MAX_QUESTION_CHARS {
        return Reply::text(QUESTION_TOO_LONG);
    }

    // `sock.sendMessage({ poll })` tidak ada di `Ctx` (lihat `ponytail:`).
    let _ = parsed;
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "poll".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            is_group: true,
            ..Default::default()
        };
        (oktz_plugin_api::find("poll").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("poll").unwrap();
        assert_eq!(p.meta.name, "poll");
        assert_eq!(p.meta.alias, ["voting", "vote", "survei"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Buat polling/voting di grup");
        assert_eq!(p.meta.usage, ".poll <pertanyaan> | <opsi1>, <opsi2>, ...");
        assert_eq!(p.meta.example, ".poll Makan apa? | Nasi Goreng, Mie Ayam, Bakso");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1, "satu-satunya plugin ini yang ambil energi");
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private && !p.meta.is_admin);
        assert!(!p.meta.is_bot_admin && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["voting", "vote", "survei"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("poll")
            );
        }
    }

    #[test]
    fn empty_text_prints_the_full_usage() {
        for t in ["", "   ", "\t"] {
            let r = run(t);
            assert!(body(&r).starts_with("⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n> Format tidak valid!"), "{t:?}");
            assert!(body(&r).contains("> `.poll multi | pertanyaan | opsi1, opsi2, opsi3, dst`"), "{t:?}");
        }
    }

    #[test]
    fn a_single_segment_is_a_format_error() {
        // Hanya `parts.length < 2` yang kena BAD_FORMAT; satu opsi selalu TOO_FEW.
        assert_eq!(body(&run("makan apa")), BAD_FORMAT);
        for t in ["makan|apa", "a|b|c"] {
            assert_eq!(body(&run(t)), TOO_FEW, "input {t:?}");
        }
        assert!(run("makan apa").mutations().is_empty());
    }

    #[test]
    fn the_multi_prefix_is_case_insensitive_and_stripped() {
        // JS hanya membaca `parts[1]` sebagai sumber opsi, jadi segmen ketiga
        // ke bawah diabaikan meski teks usage menyebutnya.
        let p = parse("MULTI |mobil| ya, tidak | aba").unwrap();
        assert!(p.is_multiple);
        assert_eq!(p.question, "mobil");
        assert_eq!(p.options, ["ya", "tidak"]);

        // Tanpa `multi`, "multi" jadi pertanyaannya.
        let p = parse("makan | ya, tidak").unwrap();
        assert!(!p.is_multiple);
        assert_eq!(p.question, "makan");
        // Tapi `multi | ...` tanpa segmen kedua tetap BAD_FORMAT.
        assert!(parse("multi | ya, tidak").is_none());
    }

    #[test]
    fn empty_option_slots_are_dropped_like_the_js_filter() {
        let p = parse("makan | nasi ,, , Goreng , ").unwrap();
        assert_eq!(p.options, ["nasi", "Goreng"]);
        // Setelah filter tersisa dua → lolos validasi, poll-nya yang tidak portable.
        assert!(run("makan | nasi ,, Goreng").is_none());
        // Satu slot kosong saja → tersisa satu opsi.
        assert_eq!(body(&run("makan | ,, Goreng ,,")), TOO_FEW);
    }

    #[test]
    fn the_option_bounds_are_two_and_twelve_inclusive() {
        let two = "a | x, y";
        assert_eq!(parse(two).unwrap().options.len(), 2);
        assert!(run(two).is_none(), "tepat 2 lolos validasi");

        let twelve = format!("a | {}", (1..=12).map(|i| format!("o{i}")).collect::<Vec<_>>().join(", "));
        let p = parse(&twelve).unwrap();
        assert_eq!(p.options.len(), MAX_OPTIONS);
        assert!(run(&twelve).is_none(), "tepat 12 lolos validasi");

        let thirteen = format!("a | {}", (1..=13).map(|i| format!("o{i}")).collect::<Vec<_>>().join(", "));
        assert_eq!(parse(&thirteen).unwrap().options.len(), 13);
        assert_eq!(body(&run(&thirteen)), TOO_MANY);
    }

    #[test]
    fn the_question_bound_is_255_characters_inclusive() {
        let q = "q".repeat(MAX_QUESTION_CHARS);
        assert!(run(&format!("{q} | a, b")).is_none(), "tepat 255 lolos");
        let long = "q".repeat(MAX_QUESTION_CHARS + 1);
        assert_eq!(body(&run(&format!("{long} | a, b"))), QUESTION_TOO_LONG);
    }

    #[test]
    fn the_too_many_check_runs_before_the_question_length_check() {
        // JS: amount > 12 dicek sebelum panjang pertanyaan.
        let long = "q".repeat(MAX_QUESTION_CHARS + 1);
        let many = (1..=13).map(|i| format!("o{i}")).collect::<Vec<_>>().join(", ");
        assert_eq!(body(&run(&format!("{long} | {many}"))), TOO_MANY);
    }

    #[test]
    fn a_valid_poll_never_claims_success_it_cannot_deliver() {
        let r = run("Makan apa? | Nasi Goreng, Mie Ayam");
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
        assert!(!r.has_media());
    }
}