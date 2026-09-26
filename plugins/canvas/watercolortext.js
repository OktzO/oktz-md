import te from "../../src/lib/error.js";
import config from "../../config.js";
import axios from "axios";
import { explainFailure, MissingApiKeyError } from "../../src/lib/stalker-fallback.js";

const pluginConfig = {
  name: "watercolorfx",
  alias: ["watercolor", "wctext"],
  category: "canvas",
  description: "Buat gambar teks dengan efek watercolor",
  usage: ".watercolorfx <teks>",
  example: ".watercolorfx Halo",
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
      `🎨 *ᴡᴀᴛᴇʀᴄᴏʟᴏʀ ᴛᴇxᴛ*\n\n> Masukkan teks yang ingin dijadikan gambar\n\n\`Contoh: ${m.prefix}watercolortext Halo\``,
    );
  }

  m.react("🕕");

  // Nexray tidak punya padanan watercolortext (dicek live: semua /ephoto/* 500),
  // jadi tidak ada fallback. Kalau key kosong, sebut saja — user bisa isi.
  if (!config.APIkey.cuki) {
    m.react("❌");
    return m.reply(
      explainFailure(
        new MissingApiKeyError("cuki", "APIKEY_CUKI"),
        `${m.prefix}watercolortext`,
      ),
    );
  }

  try {
    const url = `https://api.cuki.biz.id/api/ephoto/watercolortext?apikey=${config.APIkey.cuki}&query=${encodeURIComponent(text)}`;
    
    // Mengunduh gambar langsung (response type image)
    const { data } = await axios.get(url, {
      responseType: 'arraybuffer',
      timeout: 30000
    });

    const imageBuffer = Buffer.from(data, 'binary');

    m.react("✅");
    await sock.sendMessage(m.chat, { 
      image: imageBuffer, 
      caption: `🎨 *Water Color Text*\n\nTeks: ${text}` 
    }, { quoted: m });
  } catch (error) {
    console.log(error);
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
