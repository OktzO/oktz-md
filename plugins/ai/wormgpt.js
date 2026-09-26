import te from "../../src/lib/error.js";
import { fetchAiText } from "../../src/lib/stalker-fallback.js";

const pluginConfig = {
  name: "wormgpt",
  alias: ["worm"],
  category: "ai",
  description: "Chat dengan WormGPT (uncensored AI)",
  usage: ".wormgpt <pertanyaan>",
  example: ".wormgpt perkenalkan dirimu",
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
      `🐛 *ᴡᴏʀᴍ ɢᴘᴛ*\n\n> Masukkan pertanyaan\n\n\`Contoh: ${m.prefix}wormgpt perkenalkan dirimu\``,
    );
  }

  m.react("🕕");

  try {
    const { value: answer } = await fetchAiText("wormgpt", text);

    m.react("✅");
    await m.reply(answer);
  } catch (error) {
    console.log(error);
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
