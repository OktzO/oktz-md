use crate::common::{CONFIG_PREFIX, registry_command_lines};
use oktz_plugin_api::{Ctx, Reply, plugin};

/// Badan pesan `benefitowner` — dipisah dari handler supaya cabang daftar
/// kosong / terisi bisa diuji tanpa mengubah registry.
pub(crate) fn owner_message(command_list: &[String]) -> String {
    let total_commands = command_list.len();

    let mut message = String::new();
    message.push_str("👑 *ᴀᴘᴀ ɪᴛᴜ ᴏᴡɴᴇʀ?*\n\n");
    message.push_str(
        "Owner adalah *pemilik bot* yang memiliki akses penuh ke semua fitur dan kontrol sistem.\n\n",
    );
    message.push_str("╭┈┈⬡「 🔐 *ᴋᴇɪꜱᴛɪᴍᴇᴡᴀᴀɴ ᴏᴡɴᴇʀ* 」\n");
    message.push_str("┃ ✦ ```Akses semua command tanpa batasan```\n");
    message.push_str("┃ ✦ ```Limit tidak terbatas (-1)```\n");
    message.push_str("┃ ✦ ```Bypass semua cooldown```\n");
    message.push_str("┃ ✦ ```Kontrol penuh sistem bot```\n");
    message.push_str("┃ ✦ ```Manajemen user & group```\n");
    message.push_str("┃ ✦ ```Akses panel & server```\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 ⚙️ *ᴄᴀʀᴀ ᴋᴇʀᴊᴀ* 」\n");
    message.push_str("┃ `Owner ditambahkan melalui:`\n");
    message.push_str(&format!("┃ • ```{CONFIG_PREFIX}addowner <nomor>```\n"));
    message.push_str("┃ • Atau langsung di config.js\n");
    message.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ ᴄᴏᴍᴍᴀɴᴅ ᴏᴡɴᴇʀ* 」\n");
    message.push_str(&format!("┃ `Total: {total_commands} command`\n"));
    message.push_str("┃\n");
    // JS: `commandList.map(cmd => `┃ ${cmd}`).join('\n')` — tiap baris
    // dapat awalan "┃ ", bukan cuma dipisah newline.
    let listed: Vec<String> = command_list.iter().map(|c| format!("┃ {c}")).collect();
    message.push_str(&listed.join("\n"));
    message.push_str("\n╰┈┈┈┈┈┈┈┈⬡\n\n");
    message.push_str("> Hubungi owner untuk mendapatkan akses!");
    message
}

#[plugin(
    name = "benefitowner",
    alias = ["ownerbenefits", "ownerfitur"],
    category = "main",
    description = "Lihat penjelasan dan daftar fitur khusus Owner",
    usage = ".benefitowner",
    example = "",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn benefitowner(_ctx: Ctx) -> Reply {
    Reply::text(owner_message(&registry_command_lines(|m| {
        m.is_owner && m.is_enabled
    })))
}
