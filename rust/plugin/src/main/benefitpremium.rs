use crate::common::{CONFIG_PREFIX, registry_command_lines};
use oktz_plugin_api::{Ctx, Reply, plugin};

// `config.limits` tidak ada di `config.js`, jadi `|| 25` dan `|| 100` yang
// selalu dipakai — angka di bawah bukan tebakan, itu hasil runtime JS.
pub(crate) const LIMIT_DEFAULT: u32 = 25;

pub(crate) const LIMIT_PREMIUM: u32 = 100;

// `ponytail:` `config.owner.number` di-hardcode dari `config.js`.
pub(crate) const OWNER_NUMBERS: [&str; 1] = ["6285608953677"];

/// Badan pesan `benefitpremium` — cabang `total > 0`vs`total == 0` di sini,
/// bukan di handler, supaya kedua-duanya bisa diuji.
pub(crate) fn premium_message(command_list: &[String]) -> String {
    let total_commands = command_list.len();

    let mut message = String::new();
    message.push_str("⭐ *ᴀᴘᴀ ɪᴛᴜ ᴘʀᴇᴍɪᴜᴍ?*\n\n");
    message.push_str(
        "Premium adalah *user berbayar* yang mendapatkan akses ke fitur eksklusif dan keuntungan lebih.\n\n",
    );
    message.push_str("╭┈┈⬡「 💎 *ᴋᴇᴜɴᴛᴜɴɢᴀɴ ᴘʀᴇᴍɪᴜᴍ* 」\n");
    message.push_str(&format!(
        "┃ ✦ ```Limit harian: {LIMIT_PREMIUM}x (vs {LIMIT_DEFAULT}x user biasa)```\n"
    ));
    message.push_str("┃ ✦ ```Cooldown lebih rendah```\n");
    message.push_str("┃ ✦ ```Akses fitur eksklusif```\n");
    message.push_str("┃ ✦ ```Prioritas response```\n");
    message.push_str("┃ ✦ ```No watermark di beberapa fitur```\n");
    message.push_str("┃ ✦ ```Support prioritas```\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 ⚙️ *ᴄᴀʀᴀ ᴍᴇɴᴅᴀᴘᴀᴛᴋᴀɴ* 」\n");
    message.push_str("┃ `Premium didapatkan melalui:`\n");
    message.push_str("┃ • Hubungi owner bot\n");
    message.push_str(&format!(
        "┃ • ```{CONFIG_PREFIX}addprem <nomor> <durasi>```\n"
    ));
    message.push_str("┃ • Contoh: .addprem 628xxx 30d\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴄᴏᴍᴍᴀɴᴅ ᴘʀᴇᴍɪᴜᴍ* 」\n");
    message.push_str(&format!("┃ `Total: {total_commands} command`\n"));
    message.push_str("┃\n");
    if total_commands > 0 {
        // Sama seperti benefitowner: `┃ ` per baris.
        let listed: Vec<String> = command_list.iter().map(|c| format!("┃ {c}")).collect();
        message.push_str(&listed.join("\n"));
    } else {
        message.push_str("┃ Semua command bisa diakses user biasa");
    }
    message.push_str("\n╰┈┈┈┈┈┈┈┈⬡\n\n");
    // Spasi penutup milik `join()` di dalam literal JS, bukan bagian output —
    // balasannya TIDAK berakhir dengan spasi.
    let wa_links: Vec<String> = OWNER_NUMBERS
        .iter()
        .map(|n| format!("- wa.me/{n}"))
        .collect();
    message.push_str(&format!(
        "Mau Upgrade? silahkan hubungi owner bot\n{}",
        wa_links.join("\n")
    ));
    message
}

#[plugin(
    name = "benefitpremium",
    alias = ["premiumbenefits", "premiumfitur", "benefitprem"],
    category = "main",
    description = "Lihat penjelasan dan daftar fitur khusus Premium",
    usage = ".benefitpremium",
    example = "",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn benefitpremium(_ctx: Ctx) -> Reply {
    Reply::text(premium_message(&registry_command_lines(|m| {
        m.is_premium && m.is_enabled
    })))
}
