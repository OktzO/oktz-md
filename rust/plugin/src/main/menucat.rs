//! `plugins/main/menucat.js` — daftar command per kategori.
//!
//! `ponytail:` versi JS mengirim kartu interactive dengan simbol permission
//! (`ⓄⓟⓁⒶⒼⓅ`) dan "case" dari `case/foto.js`; `Reply::Interactive` tidak
//! punya header/tombol dan registry Rust tidak punya case. Yang dikirim teks:
//! nama plugin per kategori, urut nama. `.menu` menautkan ke command ini.

use oktz_plugin_api::{Ctx, Reply, all_plugins, plugin};

#[plugin(
    name = "menucat",
    alias = ["menucategory", "categorymenu"],
    category = "main",
    description = "Menampilkan daftar command per kategori",
    usage = ".menucat <kategori>",
    example = ".menucat main",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn menucat(ctx: Ctx) -> Reply {
    let kategori = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    let prefix = if ctx.prefix.is_empty() {
        "."
    } else {
        ctx.prefix.as_str()
    };

    if kategori.is_empty() {
        return Reply::text(format!(
            "📁 *ᴋᴀᴛᴇɢᴏʀɪ*\n\n> `{prefix}menucat <kategori>`\n\n{}",
            super::menu::kategori(&ctx)
                .into_iter()
                .map(|(cat, n)| format!("> {prefix}menucat {cat} ({n})"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    let mut nama: Vec<&str> = all_plugins()
        .iter()
        .filter(|p| p.meta.category == kategori)
        .filter(|_| kategori != "owner" || ctx.is_owner)
        .map(|p| p.meta.name.as_str())
        .collect();
    nama.sort_unstable();

    if nama.is_empty() {
        return Reply::text(format!(
            "❌ Kategori `{kategori}` tidak ada atau kosong.\n\n> Coba `{prefix}menucat` tanpa argumen."
        ));
    }

    let mut t = format!(
        "╭─☰ {}\n",
        kategori.replace('-', " ").to_uppercase()
    );
    for n in &nama {
        t.push_str(&format!("> {prefix}{n}\n"));
    }
    t.push_str(&format!("╰─⬣\n\n> Total: {} command", nama.len()));
    Reply::text(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(kategori: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            args: if kategori.is_empty() {
                Vec::new()
            } else {
                vec![kategori.into()]
            },
            command: "menucat".into(),
            is_owner: true,
            ..Ctx::default()
        }
    }

    fn teks(r: Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn mendaftar_command_kategori_main() {
        let t = teks(menucat(ctx("main")));
        assert!(t.contains(".menu"), "{t}");
        assert!(t.contains("Total:"), "{t}");
    }

    #[test]
    fn kategori_tidak_dikenal_memberi_saran() {
        let t = teks(menucat(ctx("tidakada")));
        assert!(t.contains("tidak ada"), "{t}");
    }

    #[test]
    fn non_owner_tidak_melihat_kategori_owner() {
        let mut c = ctx("owner");
        c.is_owner = false;
        let t = teks(menucat(c));
        assert!(t.contains("tidak ada"), "{t}");
    }

    #[test]
    fn tanpa_argumen_mendaftar_kategori() {
        let t = teks(menucat(ctx("")));
        assert!(t.contains(".menucat main"), "{t}");
    }
}
