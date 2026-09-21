import config from '../../config.js'
import { f } from '../../src/lib/ourin-http.js'
import te from '../../src/lib/ourin-error.js'
const NEOXR_APIKEY = config.APIkey?.neoxr || '';

const pluginConfig = {
  name: "pixeldraindl",
  alias: ["pddl", "pixeldrain", "pddownload"],
  category: "download",
  description: "Download file dari Pixeldrain",
  usage: ".pixeldraindl <url>",
  example: ".pixeldraindl https://pixeldrain.com/u/xxxxx",
  cooldown: 15,
  energi: 2,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const args = m.args || [];
  const url = args[0]?.trim();

  if (!url || !/^(https?:\/\/[\w.-]*\.?pixeldrain\.com\/)/.test(url)) {
    return m.reply(
      `📥 *ᴘɪxᴇʟᴅʀᴀɪɴ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n` +
        `> Download file dari Pixeldrain\n\n` +
        `*Format:*\n` +
        `> \`${m.prefix}pixeldraindl <url>\`\n\n` +
        `*Contoh:*\n` +
        `> \`${m.prefix}pixeldraindl https://pixeldrain.com/u/xxxxx\``,
    );
  }

  m.react("🕕");

  try {
    const apiUrl = `https://api.neoxr.eu/api/pixeldrain?url=${encodeURIComponent(url)}&apikey=${NEOXR_APIKEY}`;
    const data = await f(apiUrl)

    if (!data?.status || !data?.data) {
      m.react("❌");
      return m.reply(
        "❌ *ɢᴀɢᴀʟ*\n\n> File tidak ditemukan atau link tidak valid",
      );
    }

    const file = data.data;

    const sizeMatch = file.size?.match(/([\d.]+)\s*(MB|GB|KB)/i);
    let sizeInMB = 0;
    if (sizeMatch) {
      const value = parseFloat(sizeMatch[1]);
      const unit = sizeMatch[2].toUpperCase();
      if (unit === "GB") sizeInMB = value * 1024;
      else if (unit === "MB") sizeInMB = value;
      else if (unit === "KB") sizeInMB = value / 1024;
    }

    if (sizeInMB > 0 && sizeInMB <= 100) {

      await sock.sendMedia(m.chat, file.url, null, m, {
        type: 'document',
        fileName: file.filename,
        mimetype: 'application/octet-stream',
        contextInfo: {
          forwardingScore: 99,
          isForwarded: true
        }
      })
    } else if (sizeInMB > 100) {
      await m.reply(
        `⚠️ *ꜰɪʟᴇ ᴛᴇʀʟᴀʟᴜ ʙᴇsᴀʀ*\n\n> File ${file.size} terlalu besar untuk dikirim\n> Gunakan link download di atas`,
      );
    }

    m.react("✅");
  } catch (error) {
    m.react('☢');
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler }