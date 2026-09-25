import config from "../../config.js";
import te from "../../src/lib/error.js";
import { generateBrat, parseBratArgs } from "../../src/lib/brat.js";

const pluginConfig = {
  name: "bratgreen",
  alias: ["brat2"],
  category: "sticker",
  description: "Membuat sticker brat ijo",
  usage: ".brat2 <text>",
  example: ".brat2 Hai semua",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.args.join(" ").trim();
  if (!text) {
    return m.reply(`🖼️ *BRAT GREEN*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}bratgreen Hai semua\``);
  }

  m.react("🕕");

  try {
    const { text: parsed, blur } = parseBratArgs(text);
    const buffer = await generateBrat({ text: parsed, theme: "green", blur });
    await sock.sendImageAsSticker(m.chat, buffer, m, {
      packname: config.sticker.packname,
      author: config.sticker.author,
    });
    m.react("✅");
  } catch (error) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };