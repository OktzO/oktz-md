import axios from "axios";
import te from "../../src/lib/error.js";
import config from "../../config.js";
const pluginConfig = {
  name: "text2img",
  alias: [],
  category: "ai",
  description: "Buat gambar dari teks",
  usage: ".text2img <teks>",
  example: ".text2img Buat gambar dari teks",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.args.join(" ");
  if (!text) {
    return m.reply(
      `📿 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}text2img Buat gambar dari teks\``,
    );
  }

  m.react("🕕");

  if (!config.APIkey.firefly) {
    return m.reply(`❌ *API Key belum diset!*\n\n> Owner harus set \`APIKEY_FIREFLY\` di \`.env\``)
  }

  try {
    const url = `https://firefly.maiku.my.id/api/deepai?apikey=${config.APIkey.firefly}&prompt=${encodeURIComponent(text)}`;
    const data = await axios.get(url, { timeout: 30000 });

    const content = data?.data?.data?.output_url;
    if (!content) throw new Error('Output kosong');

    m.react("✅");
    await sock.sendMedia(m.chat, content, text, m, {
      type: "image",
    });
  } catch (error) {
    console.error(error);
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
