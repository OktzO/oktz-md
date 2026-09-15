import { fluxImage } from "../../src/scraper/seaart.js";

const pluginConfig = {
  name: "bananagen2",
  alias: [],
  category: "ai",
  description: "Buat gambar dengan AI menggunakan prompt",
  usage: ".bananagen2 <prompt>",
  example: ".bananagen2 make it anime style",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 30,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const prompt = m.text;
  if (!prompt) {
    return m.reply(
      `🍌 *BANANA GEN 2*\n\n` +
        `> Buat gambar dengan AI\n\n` +
        `\`Contoh: ${m.prefix}bananagen2 make a cat\``,
    );
  }

  m.react("🕕");

  try {
    const result = await fluxImage(prompt, "1:1");
    const imageUrl = result.url;

    m.react("✅");

    await sock.sendMedia(m.chat, imageUrl, null, m, {
      type: "image",
    });
  } catch (error) {
    console.log(error);
    m.react("❌");
    const msg =
      error?.response?.data?.message ||
      error?.response?.data?.error ||
      error.message ||
      "Terjadi kesalahan";
    m.reply(`🍀 *Waduhh, sepertinya ini ada kendala*

${msg}

Silahkan coba lagi nanti, dimohon jangan spam`);
  }
}

export { pluginConfig as config, handler };
