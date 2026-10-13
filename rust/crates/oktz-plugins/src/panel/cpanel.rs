//! `plugins/panel/cpanel.js` — menu kumpulan command panel (v1-v5).
//!
//! `ponytail:` status server, token DigitalOcean, dan website diambil dari
//! `config.js` (`config.pterodactyl.server1..5`, `config.digitalocean.token`,
//! `config.info.website`). `Ctx` tidak membawa config, jadi nilainya ikut
//! `config.js` yang di-hardcode — persis seperti `info/benefitpartner.rs`
//! melakukan untuk `config.bot.name`. Saat port ini semua `domain`/`apikey`
//! masih kosong, jadi kelima status `❌` dan token DO `❌`; begitu `config.js`
//! diisi, baris ini ikut stale sampai router menyediakannya lewat `Ctx`.
//!
//! `ponytail:` `getAccessibleServers` (roles-cpanel.js) membaca file
//! `database/cpanel/*.json` lewat `fs` — tidak ada padanannya di `Ctx`, jadi
//! role user non-owner selalu tampil "Tidak ada". Lihat laporan batch.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const VALID_SERVERS: [&str; 5] = ["v1", "v2", "v3", "v4", "v5"];

/// `config.info.website` (`config.js:12`).
pub(crate) const WEBSITE: &str = "https://oktzz.xyz";

/// `config.pterodactyl.server1..5` — semua `domain` dan `apikey` kosong di
/// `config.js`, jadi `getStatus` mengembalikan `❌` untuk kelimanya.
pub(crate) const SERVER_OK: [bool; 5] = [false, false, false, false, false];

/// `config.digitalocean.token` kosong di `config.js`.
pub(crate) const DO_HAS_TOKEN: bool = false;

fn mark(ok: bool) -> &'static str {
    if ok { "✅" } else { "❌" }
}

/// Satu blok menu: judul + satu baris per versi server.
fn block(txt: &mut String, title: &str, suffix: &str, prefix: &str) {
    txt.push_str(&format!("╭─「 {title} 」\n"));
    for ver in VALID_SERVERS {
        txt.push_str(&format!("┃ `{prefix}1gb{ver}` - `{prefix}10gb{ver}` | `{prefix}{suffix}{ver}`\n"));
    }
    txt.push_str("╰───────────────\n\n");
}

/// Blok owner/ceo/reseller/admin/server: tiga (atau tiga+dua) perintah per versi.
fn tri_block(txt: &mut String, title: &str, prefix: &str, a: &str, b: &str, c: &str) {
    txt.push_str(&format!("╭─「 {title} 」\n"));
    for ver in VALID_SERVERS {
        txt.push_str(&format!("┃ `{prefix}{a}{ver}` | `{prefix}{b}{ver}` | `{prefix}{c}{ver}`\n"));
    }
    txt.push_str("╰───────────────\n\n");
}

/// `userRoleList` — `SERVER:role` dipisah koma, atau "Tidak ada".
fn role_list(ctx: &Ctx) -> String {
    if ctx.is_owner {
        return "Bot Owner".to_string();
    }
    "Tidak ada".to_string()
}

pub(crate) fn build_menu(prefix: &str, roles: &str) -> String {
    let mut txt = String::from("🖥️ *ᴄᴘᴀɴᴇʟ ᴍᴇɴᴜ v2.0*\n\n");
    txt.push_str(&format!(
        "> V1: {} | V2: {} | V3: {} | V4: {} | V5: {}\n",
        mark(SERVER_OK[0]),
        mark(SERVER_OK[1]),
        mark(SERVER_OK[2]),
        mark(SERVER_OK[3]),
        mark(SERVER_OK[4]),
    ));
    txt.push_str(&format!("> Role kamu: *{roles}*\n\n"));

    block(&mut txt, "📦 *ᴄʀᴇᴀᴛᴇ sᴇʀᴠᴇʀ*", "unli", prefix);
    tri_block(&mut txt, "👑 *ᴏᴡɴᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ*", prefix, "addowner", "delowner", "listowner");
    tri_block(&mut txt, "🎯 *ᴄᴇᴏ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ*", prefix, "addceo", "delceo", "listceo");
    tri_block(&mut txt, "👥 *ʀᴇsᴇʟʟᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ*", prefix, "addreseller", "delreseller", "listreseller");
    tri_block(&mut txt, "🔐 *ᴀᴅᴍɪɴ ᴘᴀɴᴇʟ*", prefix, "cadmin", "deladmin", "listadmin");
    tri_block(&mut txt, "🖥️ *sᴇʀᴠᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ*", prefix, "listserver", "delserver", "serverinfo");

    txt.push_str("╭─「 👤 *ᴜsᴇʀ ᴍᴀɴᴀɢᴇᴍᴇɴᴛ* 」\n");
    for ver in VALID_SERVERS {
        txt.push_str(&format!("┃ `{prefix}listuser{ver}`\n"));
    }
    txt.push_str("╰───────────────\n\n");

    txt.push_str("╭─「 🏪 *ɢᴄ sᴇʟʟᴇʀ ᴘᴀɴᴇʟ* 」\n");
    for ver in VALID_SERVERS {
        txt.push_str(&format!("┃ `{prefix}addgcseller{ver}` | `{prefix}resetgcseller{ver}`\n"));
    }
    txt.push_str("╰───────────────\n\n");

    txt.push_str("╭─「 🌊 *ᴅɪɢɪᴛᴀʟᴏᴄᴇᴀɴ ᴠᴘs* 」\n");
    txt.push_str(&format!("┃ Status: {} Token\n", mark(DO_HAS_TOKEN)));
    txt.push_str("┃\n");
    txt.push_str("┃ 📦 *ᴄʀᴇᴀᴛᴇ ᴠᴘs:*\n");
    txt.push_str(&format!("┃ `{prefix}vps1g1c` - 1GB/1CPU\n"));
    txt.push_str(&format!("┃ `{prefix}vps2g1c` - 2GB/1CPU\n"));
    txt.push_str(&format!("┃ `{prefix}vps4g2c` - 4GB/2CPU\n"));
    txt.push_str(&format!("┃ `{prefix}vps8g4c` - 8GB/4CPU\n"));
    txt.push_str("┃\n");
    txt.push_str("┃ 🔧 *ᴍᴀɴᴀɢᴇ:*\n");
    txt.push_str(&format!("┃ `{prefix}listvps` | `{prefix}cekvps` | `{prefix}delvps` | `{prefix}sisavps`\n"));
    txt.push_str("┃\n");
    txt.push_str("┃ ⚡ *ᴋᴏɴᴛʀᴏʟ:*\n");
    txt.push_str(&format!("┃ `{prefix}turnon` | `{prefix}turnoff` | `{prefix}restartvps`\n"));
    txt.push_str("╰───────────────\n\n");

    txt.push_str(&format!("> _Powered by {WEBSITE}_"));
    txt
}

#[plugin(
    name = "cpanel",
    alias = ["panelmenu", "menupanel"],
    category = "panel",
    description = "Menu panel pterodactyl (v1-v5)",
    usage = ".cpanel",
    example = ".cpanel",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cpanel(ctx: Ctx) -> Reply {
    // JS: `const prefix = m.prefix || '.'` — mode noprefix tetap pakai titik.
    let prefix = if ctx.prefix.is_empty() { "." } else { ctx.prefix.as_str() };
    Reply::text(build_menu(prefix, &role_list(&ctx)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(prefix: &str, is_owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "cpanel".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_owner,
            ..Default::default()
        };
        (find("cpanel").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("cpanel").unwrap().meta;
        assert_eq!(m.name, "cpanel");
        assert_eq!(m.alias, ["panelmenu", "menupanel"]);
        assert_eq!(m.category, "panel");
        assert_eq!(m.description, "Menu panel pterodactyl (v1-v5)");
        assert_eq!(m.usage, ".cpanel");
        assert_eq!(m.example, ".cpanel");
        assert_eq!(m.cooldown, 5);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["panelmenu", "menupanel"] {
            assert_eq!(find(a).unwrap().meta.name, "cpanel", "alias {a}");
        }
    }

    #[test]
    fn the_menu_lists_every_command_for_every_server_version() {
        let t = text_of(&run(".", false));
        for ver in VALID_SERVERS {
            for cmd in [
                format!("1gb{ver}"),
                format!("10gb{ver}"),
                format!("unli{ver}"),
                format!("addowner{ver}"),
                format!("delceo{ver}"),
                format!("listreseller{ver}"),
                format!("cadmin{ver}"),
                format!("serverinfo{ver}"),
                format!("listuser{ver}"),
                format!("resetgcseller{ver}"),
            ] {
                assert!(t.contains(&format!("`.{cmd}`")), "command hilang: {cmd}");
            }
        }
        assert!(t.starts_with("🖥️ *ᴄᴘᴀɴᴇʟ ᴍᴇɴᴜ v2.0*"), "{t}");
        assert!(t.ends_with(&format!("> _Powered by {WEBSITE}_")), "{t}");
    }

    #[test]
    fn the_live_prefix_is_used_instead_of_a_hardcoded_dot() {
        let t = text_of(&run("!", false));
        assert!(t.contains("`!1gbv1`"), "{t}");
        assert!(!t.contains("`.1gbv1`"), "prefix lama masih bocor: {t}");
        // `m.prefix || '.'` — prefix kosong berarti noprefix, tetap pakai titik.
        let t = text_of(&run("", false));
        assert!(t.contains("`.vps1g1c`"), "{t}");
    }

    #[test]
    fn the_owner_sees_its_role_and_everyone_else_sees_the_empty_list() {
        let owner = text_of(&run(".", true));
        assert!(owner.contains("> Role kamu: *Bot Owner*"), "{owner}");
        let user = text_of(&run(".", false));
        assert!(user.contains("> Role kamu: *Tidak ada*"), "{user}");
    }

    #[test]
    fn the_config_backed_status_marks_are_reported_per_server() {
        // `config.js` masih kosong untuk kelima server dan token DigitalOcean.
        let t = text_of(&run(".", false));
        assert!(
            t.contains("> V1: ❌ | V2: ❌ | V3: ❌ | V4: ❌ | V5: ❌"),
            "{t}"
        );
        assert!(t.contains("┃ Status: ❌ Token"), "{t}");
        assert!(t.contains("🌊 *ᴅɪɢɪᴛᴀʟᴏᴄᴇᴀɴ ᴠᴘs*"), "{t}");
    }

    #[test]
    fn the_menu_writes_nothing_to_the_store() {
        assert!(run(".", true).mutations().is_empty());
    }
}