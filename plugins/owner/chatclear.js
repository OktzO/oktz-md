import {
  isAutoClearEnabled,
  setAutoClearEnabled,
} from "../../src/lib/chat-cleaner.js";

const pluginConfig = {
  name: ["autoclear"],
  alias: [],
  category: "owner",
  description: "Auto-hapus semua chat grup & private tiap tengah malam",
  usage: ".autoclear on|off|status",
  example: ".autoclear on",
  isOwner: true,
  cooldown: 3,
  energi: 0,
  isEnabled: true,
};

function statusLine() {
  return isAutoClearEnabled()
    ? "✅ *HIDUP* — semua chat (grup & private) akan dihapus tiap tengah malam (00:00 WIB)"
    : "❌ *MATI* — bot tidak menghapus chat otomatis";
}

async function handler(m) {
  const action = (m.args[0] || "status").toLowerCase();

  if (action === "on") {
    setAutoClearEnabled(true);
    await m.react("✅");
    return m.reply(
      `⚡ *AUTO CLEAR CHAT*\n\n> Auto-clear sekarang *AKTIF*\n> Semua chat grup & private akan dihapus otomatis tiap tengah malam (*00:00 WIB*)\n\nTo matikan: \`${m.prefix}autoclear off\``
    );
  }

  if (action === "off") {
    setAutoClearEnabled(false);
    await m.react("✅");
    return m.reply(
      `⚡ *AUTO CLEAR CHAT*\n\n> Auto-clear sekarang *MATI*\n> Chat grup & private tidak akan dihapus otomatis\n\nTo aktifkan: \`${m.prefix}autoclear on\``
    );
  }

  if (action === "status") {
    return m.reply(
      `⚡ *AUTO CLEAR CHAT*\n\n> Status: ${statusLine()}\n\n> \`${m.prefix}autoclear\` — lihat status\n> \`${m.prefix}autoclear on\` — aktifkan\n> \`${m.prefix}autoclear off\` — matikan`
    );
  }

  return m.reply(
    `⚡ *AUTO CLEAR CHAT*\n\n> Perintah tidak dikenal: \`${action}\`\n\n> \`${m.prefix}autoclear\` — lihat status\n> \`${m.prefix}autoclear on\` — aktifkan\n> \`${m.prefix}autoclear off\` — matikan\n\n_Owner only_`
  );
}

export { pluginConfig as config, handler };