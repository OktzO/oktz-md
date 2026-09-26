import te from "../../src/lib/error.js";
import { sendNgl } from "../../src/lib/stalker-fallback.js";
const pluginConfig = {
  name: "sendngl",
  alias: [],
  category: "tools",
  description: "Send NGL",
  usage: ".sendngl <url> | <text>",
  example: ".sendngl https://ngl.link/xxxx | hai",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};

async function handler(m, { sock }) {
  // m.text = bagian setelah nama command, tapi user tetap mengetik spasi
  // di sekitar "|" — trim biar URL/pesan tidak ikut spaces.
  const [rawLink, rawKata] = (m.text ?? "").split("|");
  const link = rawLink?.trim();
  const kata = rawKata?.trim();
  if (!link)
    return m.reply(
      `*LINK NGL NYA MANA ??*\nContoh: \`${m?.prefix}sendngl https://ngl.link/xxxx | hai`,
    );
  if (!kata)
    return m.reply(
      `*KATA KATA NYA MANA ??*\n\nContoh: \`${m?.prefix}sendngl https://ngl.link/xxxx | hai`,
    );
  m.react("🎴");

  try {
    await sendNgl(link, kata);

    m.react("✅");

    await sock.sendMessage(
      m.chat,
      {
        text: `✅ *DONE*\n\nBerhasil mengirim pesan!\nTarget: ${link}\nPesan: ${kata}`,
      },
      { quoted: m },
    );
  } catch (error) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
