//! `plugins/main/tqto.js` — daftar kontributor bot.
//!
//! `ponytail:` `config.bot.name` tidak ada di `Ctx`, jadi nama bot
//! di-hardcode dari `config.js:39` — sama seperti `info/benefitpartner.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `config.bot.name` (`config.js:39`).
pub(crate) const BOT_NAME: &str = "Oktz Bot";

/// `(name, role, icon)` dari `tqto.js:20-81`, urutan persis.
pub(crate) const CREDITS: [(&str, &str, &str); 61] = [
    ("hyuuOkkotsuX", "Lead Staff", "👨‍💻"),
    ("Zann", "Creator", "👨‍💻"),
    ("SenzOkkotsu", "Developer", "👨‍💻"),
    ("Ell", "Developer", "👨‍💻"),
    ("Aqell", "Developer", "👨‍💻"),
    ("Mobbc", "Staff", "👨‍💻"),
    ("Raka", "Staff", "👨‍💻"),
    ("Sanxz", "Tangan Kanan", "👨‍💻"),
    ("Dinz", "Tangan Kanan", "👨‍💻"),
    ("Forone Store", "Tangan Kanan", "🛒"),
    ("Fahmi", "Tangan Kanan", "👨‍💻"),
    ("Sabila", "Tangan Kanan", "👩‍💻"),
    ("Syura Store", "Tangan Kanan", "👩‍💻"),
    ("Xero", "Tangan Kanan", "👩‍💻"),
    ("Aji", "Tangan Kanan", "👩‍💻"),
    ("Lyoraaa", "Owner", "👩‍💻"),
    ("Danzzz", "Owner", "👨‍💻"),
    ("Muzan", "Owner", "👨‍💻"),
    ("Gray", "Owner", "👨‍💻"),
    ("Baim", "Moderator", "👨‍💻"),
    ("Vadel", "Moderator", "👨‍💻"),
    ("Fikzz", "Moderator", "🛒"),
    ("Caca", "Moderator", "👨‍💻"),
    ("panceo", "Partner", "🛒"),
    ("KingSatzID", "Partner", "🛒"),
    ("Dashxz", "Partner", "🛒"),
    ("This JanzZ", "Partner", "🛒"),
    ("Ahmad", "Partner", "🛒"),
    ("nopal", "Partner", "🛒"),
    ("tuadit", "Partner", "🛒"),
    ("andry", "Partner", "🛒"),
    ("kingdanz", "Partner", "🛒"),
    ("patih", "Partner", "🛒"),
    ("Ryuu", "Partner", "🛒"),
    ("Pororo", "Partner", "🛒"),
    ("Janzz", "Partner", "🛒"),
    ("Morvic", "Partner", "🛒"),
    ("zylnzee", "Partner", "🛒"),
    ("Farhan", "Partner", "🛒"),
    ("Alizz", "Partner", "🛒"),
    ("Kiram", "Partner", "🛒"),
    ("Minerva", "Partner", "🛒"),
    ("HanzPiw", "Partner", "🛒"),
    ("Ryuzen", "Partner", "🛒"),
    ("Ahmad", "Partner", "🛒"),
    ("Riam", "Partner", "🛒"),
    ("Erren", "Partner", "🛒"),
    ("ranzen", "Partner", "🛒"),
    ("Febri", "Partner", "🛒"),
    ("Kuze", "Partner", "🛒"),
    ("Oscar Dani", "Partner", "🛒"),
    ("Udun", "Partner", "🛒"),
    ("Renn", "Partner", "🛒"),
    ("Taka", "Partner", "🛒"),
    ("Tatskuyy", "Partner", "🛒"),
    ("Yann", "Partner", "🛒"),
    ("Zanspiw", "Youtuber", "🌐"),
    ("Danzz Nano", "Youtuber", "🌐"),
    ("Youtuber Lain yang udah review", "Youtuber", "🌐"),
    ("Kalian Semua", "Best", "🌐"),
    ("Open Source Community", "Libraries & Tools", "🌐"),
];

/// `credits.map((c, i) => \`*${i + 1}*. *${c.name}* [ ${c.icon} ${c.role} ]\`)`.
pub(crate) fn credit_lines() -> Vec<String> {
    CREDITS
        .iter()
        .enumerate()
        .map(|(i, (name, role, icon))| format!("*{}. *{name}* [ {icon} {role} ]", i + 1))
        .collect()
}

#[plugin(
    name = "tqto",
    alias = ["thanksto", "credits", "kredit"],
    category = "main",
    description = "Menampilkan daftar kontributor bot",
    usage = ".tqto",
    example = ".tqto",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn tqto(_ctx: Ctx) -> Reply {
    Reply::text(format!(
        "🍟 *Berikut ini adalah orang orang yang sudah berkontribusi di bot {BOT_NAME}*\n        \n{}\n}}",
        credit_lines().join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> String {
        (oktz_plugin_api::find("tqto").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "tqto".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
        .text_body()
        .expect("harus membalas teks")
        .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tqto").unwrap();
        assert_eq!(p.meta.name, "tqto");
        assert_eq!(p.meta.alias, ["thanksto", "credits", "kredit"]);
        assert_eq!(p.meta.category, "main");
        assert_eq!(p.meta.description, "Menampilkan daftar kontributor bot");
        assert_eq!(p.meta.usage, ".tqto");
        assert_eq!(p.meta.example, ".tqto");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["tqto", "thanksto", "credits", "kredit"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("tqto"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_roster_is_copied_verbatim_from_the_js_list() {
        assert_eq!(CREDITS[0], ("hyuuOkkotsuX", "Lead Staff", "👨‍💻"));
        assert_eq!(CREDITS[1], ("Zann", "Creator", "👨‍💻"));
        assert_eq!(CREDITS[2], ("SenzOkkotsu", "Developer", "👨‍💻"));
        assert_eq!(
            CREDITS.last().unwrap(),
            &("Open Source Community", "Libraries & Tools", "🌐")
        );
        // "Ahmad" muncul dua kali di JS — duplikat itu disengaja, jadi ikut.
        assert_eq!(
            CREDITS.iter().filter(|(n, ..)| *n == "Ahmad").count(),
            2,
            "entri ganda di JS harus tetap dua"
        );
    }

    #[test]
    fn every_credit_is_numbered_from_one_in_source_order() {
        let lines = credit_lines();
        assert_eq!(lines.len(), CREDITS.len());
        assert_eq!(lines[0], "*1. *hyuuOkkotsuX* [ 👨‍💻 Lead Staff ]");
        assert_eq!(
            lines[1],
            "*2. *Zann* [ 👨‍💻 Creator ]",
            "nomor harus i + 1, bukan i"
        );
        assert_eq!(
            lines[lines.len() - 1],
            "*61. *Open Source Community* [ 🌐 Libraries & Tools ]"
        );
    }

    #[test]
    fn the_reply_carries_the_header_and_every_line() {
        let out = run();
        assert!(
            out.starts_with(&format!(
                "🍟 *Berikut ini adalah orang orang yang sudah berkontribusi di bot {BOT_NAME}*"
            )),
            "{out}"
        );
        assert!(out.ends_with('}'), "penutup kurung kurawal dari template JS: {out}");
        for (i, line) in credit_lines().iter().enumerate() {
            assert!(out.contains(line.as_str()), "baris hilang: {line}");
            if i == 0 {
                assert!(out.contains(&format!("
{line}
")), "{out}");
            }
        }
    }

    #[test]
    fn the_header_keeps_the_js_whitespace_between_the_title_and_the_list() {
        // `\n        \n` di JS — spasi ikut ke pengguna.
        let out = run();
        let idx = out.find("\n        \n").expect("jarak kosong JS hilang");
        assert!(out[idx + 10..].starts_with("*1. *hyuuOkkotsuX*"), "{out}");
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = Ctx::default();
        let r = (oktz_plugin_api::find("tqto").unwrap().handler)(ctx);
        assert!(r.mutations().is_empty());
    }
}
