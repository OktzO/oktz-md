use oktz_plugin_api::{Ctx, Reply, plugin};

// `ponytail:` `config.bot.name` / `config.owner.name` di-hardcode dari
// `config.js` karena `Ctx` tidak membawa config. Ganti dengan config runtime
// kalau router someday menyediakannya.
pub(crate) const BOT_NAME: &str = "Oktz Bot";

pub(crate) const OWNER_NAME: &str = "Oktz";

#[plugin(
    name = "benefitpartner",
    alias = ["partnerbenefits", "keuntunganpartner"],
    category = "info",
    description = "Lihat keuntungan menjadi partner bot",
    usage = ".benefitpartner",
    example = ".benefitpartner",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn benefitpartner(ctx: Ctx) -> Reply {
    let prefix = ctx.prefix.as_str();

    let mut txt = String::from("🤝 *BENEFIT PARTNER*\n\n");
    txt.push_str(&format!("Keuntungan menjadi partner {BOT_NAME}:\n\n"));

    txt.push_str("🔓 *Akses Fitur*\n");
    txt.push_str("├ Semua fitur premium terbuka\n");
    txt.push_str("├ Energi & koin unlimited\n");
    txt.push_str("├ Akses command owner tertentu\n");
    txt.push_str("└ Prioritas support\n\n");

    txt.push_str("📦 *Panel Pterodactyl*\n");
    txt.push_str("├ Bisa create server sendiri\n");
    txt.push_str("├ Akses panel management\n");
    txt.push_str("└ Bisa jualan panel (reseller)\n\n");

    txt.push_str("💎 *Bonus*\n");
    txt.push_str("├ +200.000 EXP saat aktivasi\n");
    txt.push_str("├ +20.000 Koin saat aktivasi\n");
    txt.push_str("├ Badge partner di profil\n");
    txt.push_str("└ Akses early feature\n\n");

    txt.push_str("💰 *Cara Jadi Partner*\n");
    txt.push_str(&format!("├ Hubungi owner: {OWNER_NAME}\n"));
    txt.push_str("├ Durasi: 30/60/90 hari\n");
    txt.push_str(&format!("└ Command: `{prefix}addpartner` (owner only)\n\n"));

    txt.push_str("📋 *Command Partner*\n");
    txt.push_str(&format!("├ `{prefix}cekpartner` — Cek status partner\n"));
    txt.push_str(&format!("├ `{prefix}cekprem` — Cek status premium\n"));
    txt.push_str(&format!("├ `{prefix}cekowner` — Cek role user\n"));
    txt.push_str(&format!("└ `{prefix}listpartner` — Daftar partner\n\n"));

    txt.push_str("> _Hubungi owner untuk info lebih lanjut_");

    Reply::text(txt)
}
