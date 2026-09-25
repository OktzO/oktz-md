import { getAssetBuffer } from "../../src/lib/asset-manager.js";
import axios from "axios";
import config from "../../config.js";
import te from "../../src/lib/error.js";
const NEOXR_APIKEY = config.APIkey?.neoxr || '';

const pluginConfig = {
  name: "film",
  alias: ["nonton", "lk21"],
  category: "search",
  description: "Cari film dan nonton online",
  usage: ".film <judul>",
  example: ".film civil war",
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const args = m.args || [];
  const query = args.join(" ").trim();

  if (!query) {
    return m.reply(
      `🎬 *ꜰɪʟᴍ sᴇᴀʀᴄʜ*\n\n` +
        `> Cari dan nonton film online\n\n` +
        `*Format:*\n` +
        `> \`${m.prefix}film <judul>\`\n\n` +
        `*Contoh:*\n` +
        `> \`${m.prefix}film civil war\``,
    );
  }

  m.react("🎬");

  try {
    const apiUrl = `https://api.neoxr.eu/api/film?q=${encodeURIComponent(query)}&apikey=${NEOXR_APIKEY}`;
    const { data } = await axios.get(apiUrl, { timeout: 30000 });

    if (!data?.status || !data?.data?.length) {
      m.react("❌");
      return m.reply(
        `❌ *ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> Film "${query}" tidak ditemukan`,
      );
    }

    const films = data.data.slice(0, 10);

    let text = `🎬 *ʜᴀsɪʟ ᴘᴇɴᴄᴀʀɪᴀɴ*\n\n`;
    text += `> Ditemukan *${films.length}* film untuk "${query}"\n\n`;

    films.forEach((f, i) => {
      text += `*${i + 1}. ${f.title}*\n`;
      text += `> ⭐ ${f.rating} | 📺 ${f.quality} | 📅 ${f.release}\n\n`;
    });

    text += `> _Pilih film dari list di bawah_`;

    const listItems = films.map((f, _) => ({
      header: "",
      title: f.title,
      description: `⭐ ${f.rating} | ${f.quality} | ${f.release}`,
      id: `${m.prefix}filmget ${f.url}`,
    }));

    await sock.sendButton(
      m.chat,
      getAssetBuffer("foto"),
      text,
      m,
      {
        buttons: [
          {
            name: "single_select",
            buttonParamsJson: JSON.stringify({
              title: "🎬 Pilih Film",
              sections: [
                {
                  title: "Hasil Pencarian",
                  rows: listItems,
                },
              ],
            }),
          },
        ],
        footer: "🎬 Film Search",
      },
    );

    m.react("✅");
  } catch (error) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
