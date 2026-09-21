import axios from "axios";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";
const pluginConfig = {
  name: "pixiv",
  alias: ["pixivsearch", "caripixiv"],
  category: "search",
  description: "Cari artwork di Pixiv",
  usage: ".pixiv <query>",
  example: ".pixiv rem",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 2,
  isEnabled: true,
};

async function handler(m, { sock }) {
  try {
    const query = m.args?.join(" ")?.trim();

    if (!query) {
      return m.reply(
        `❌ *Masukkan kata kunci pencarian!*\n\n> Contoh: .pixiv rem`,
      );
    }

    await m.react("🔍");

    const apikey = config.APIkey?.neoxr || '';
    const url = `https://api.neoxr.eu/api/pixiv-search?q=${encodeURIComponent(query)}&apikey=${apikey}`;

    const response = await axios.get(url, { timeout: 30000 });
    const data = response.data;

    if (!data.status || !data.data || data.data.length === 0) {
      await m.react("❌");
      return m.reply(`❌ *Tidak ditemukan hasil untuk:* ${query}`);
    }

    const results = data.data.slice(0, 10);

    const saluranId = config.saluran?.id || "120363400911374213@newsletter";
    const saluranName = config.saluran?.name || config.bot?.name || "Bot";

    let caption = `🎨 *ᴘɪxɪᴠ sᴇᴀʀᴄʜ*\n`;
    caption += `📝 *ᴋᴜᴇʀʏ:* ${query}\n`;
    caption += `📊 *ʜᴀsɪʟ:* ${results.length} artwork\n\n`;

    results.forEach((art, i) => {
      const aiLabel = art.aiType === 2 ? " 🤖" : "";
      const isNsfw = art.xRestrict > 0 ? " 🔞" : "";
      caption += `*${i + 1}.* ${art.title}${aiLabel}${isNsfw}\n`;
      caption += `   👤 ${art.userName}\n`;
      caption += `   📐 ${art.width}x${art.height} • 📄 ${art.pageCount} page\n`;
      caption += `   🔗 ${art.url}\n\n`;
    });

    caption += `> 🎨 Powered by Pixiv`;

    await m.react("🎨");

    await sock.sendMessage(
      m.chat,
      {
        text: caption,
        contextInfo: {
          forwardingScore: 9999,
          isForwarded: true,
          forwardedNewsletterMessageInfo: {
            newsletterJid: saluranId,
            newsletterName: saluranName,
            serverMessageId: 127,
          },
        },
      },
      { quoted: m },
    );
  } catch (error) {
    await m.react("☢");
    if (error.response?.status === 403) {
      return m.reply(`❌ *API Key tidak valid atau limit tercapai*`);
    }
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
