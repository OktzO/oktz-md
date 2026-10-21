//! `plugins/main/sc.js` — link script bot (kartu tombol `cta_url`).

use oktz_plugin_api::{Ctx, Reply, Tombol, plugin};

/// `sc.js:24` — satu-satunya URL.
pub(crate) const REPO_URL: &str = "https://github.com/OktzO/oktz-md";

/// Body kartu (`sc.js:33-45`).
pub(crate) fn body(nama_pengirim: &str, bot: &oktz_plugin_api::config::BotInfo) -> String {
    format!(
        "🌾 Halo kak *{nama_pengirim}*\n\n\
         Script bot ini open source, kamu bisa dapatkan langsung di repo *Oktz-MD*\n\n\
         > 🤖 *Name*: {}\n\
         > ⚙️ *Version*: {}\n\
         > 👨‍💻 *Developer*: {}\n\
         > 🧩 *Repo*: OktzO/oktz-md\n\n\
         Klik tombol di bawah untuk mendapatkan scriptnya ya!",
        bot.name, bot.version, bot.developer
    )
}

#[plugin(
    name = "sc",
    alias = ["script"],
    category = "main",
    description = "Link script bot wa terbaru",
    usage = ".sc",
    example = ".sc",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn sc(ctx: Ctx) -> Reply {
    let bot = &ctx.config().bot;
    Reply::interactive(body(&ctx.push_name, bot))
        .with_footer(format!("{} — Oktz-MD", bot.name))
        .with_button(Tombol::buka_url("🥐 Kunjungi GitHub OktzO", REPO_URL))
        .with_quote()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot() -> oktz_plugin_api::config::BotInfo {
        oktz_plugin_api::config::BotInfo {
            name: "Oktz Bot".into(),
            version: "3.3.1".into(),
            developer: "Zann".into(),
            website: "https://oktzz.xyz".into(),
        }
    }

    #[test]
    fn body_mengikuti_baris_javascript() {
        assert_eq!(
            body("Zann", &bot()),
            "🌾 Halo kak *Zann*\n\n\
             Script bot ini open source, kamu bisa dapatkan langsung di repo *Oktz-MD*\n\n\
             > 🤖 *Name*: Oktz Bot\n\
             > ⚙️ *Version*: 3.3.1\n\
             > 👨‍💻 *Developer*: Zann\n\
             > 🧩 *Repo*: OktzO/oktz-md\n\n\
             Klik tombol di bawah untuk mendapatkan scriptnya ya!"
        );
    }

    #[test]
    fn kartu_membawa_footer_dan_tombol_cta_url() {
        let ctx = Ctx {
            push_name: "Zann".into(),
            config: std::sync::Arc::new(oktz_plugin_api::config::Runtime {
                bot: bot(),
                ..Default::default()
            }),
            ..Ctx::default()
        };

        let r = sc(ctx);
        let Reply::Interactive { kartu } = r.without_mutations() else {
            panic!("harus kartu interaktif");
        };
        assert!(kartu.body.starts_with("🌾 Halo kak *Zann*"));
        assert_eq!(kartu.footer.as_deref(), Some("Oktz Bot — Oktz-MD"));
        assert_eq!(kartu.buttons[0].aksi.as_str(), "cta_url");
        assert_eq!(kartu.buttons[0].params["display_text"], "🥐 Kunjungi GitHub OktzO");
        assert_eq!(kartu.buttons[0].params["url"], REPO_URL);
        assert_eq!(kartu.buttons[0].params["merchant_url"], REPO_URL);
        assert!(kartu.quote, "kartu sc mengutip pemicu seperti JS");
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("sc").unwrap();
        assert_eq!(p.meta.name, "sc");
        assert_eq!(p.meta.alias, ["script"]);
        assert_eq!(p.meta.category, "main");
        assert_eq!(p.meta.description, "Link script bot wa terbaru");
        assert_eq!(p.meta.usage, ".sc");
        assert_eq!(p.meta.example, ".sc");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_group);
    }
}
