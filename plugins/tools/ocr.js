import { ocr } from "../../src/lib/ourin-native-loader.js";
import te from "../../src/lib/ourin-error.js";
import { sendToolsPreview } from "../../src/lib/ourin-context.js";

const pluginConfig = {
  name: "ocr",
  alias: ["totext", "imagetotext", "readtext"],
  category: "tools",
  description: "Extract teks dari gambar (Offline/Local)",
  usage: ".ocr (reply gambar)",
  example: ".ocr",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};
async function handler(m, { sock }) {
  const isImage = m.isImage || (m.quoted && m.quoted.type === "imageMessage");
  if (!isImage) {
    return m.reply(
      `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
        `> Reply gambar dengan \`${m.prefix}ocr\`\n\n` +
        `> Media yang didukung:\n` +
        `> JPG, PNG, GIF, WEBP`,
    );
  }
  await m.react("🕕");
  await m.reply(`🕕 *ᴍᴇᴍᴘʀᴏsᴇs...*\n\n> Mengekstrak teks dari gambar...`);
  try {
    let buffer;
    if (m.quoted && m.quoted.isMedia) {
      buffer = await m.quoted.download();
    } else if (m.isMedia) {
      buffer = await m.download();
    }
    if (!buffer || buffer.length === 0) {
      await m.react("❌");
      return m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat download gambar`);
    }
    let extractedText = "";
    // Native ourin-native if the prebuilt .node loads; tesseract.js fallback otherwise.
    extractedText = (await ocr(buffer)).trim() || "";
    if (!extractedText || extractedText.length === 0) {
      await m.react("❌");
      return m.reply(
        `❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ᴛᴇᴋs*\n\n> Tidak ada teks yang terdeteksi di gambar`,
      );
    }
    await m.react("✅");
    const responseText =
      `📖 *ᴏᴄʀ ʀᴇsᴜʟᴛ*\n\n` +
      `╭┈┈⬡「 📝 *ᴛᴇᴋs* 」\n` +
      `${extractedText
        .split("\n")
        .map((l) => `┃ ${l}`)
        .join("\n")}\n` +
      `╰┈┈┈┈┈┈┈┈⬡\n\n` +
      `> Total: ${extractedText.length} karakter`;
    await sendToolsPreview(
      sock,
      m.chat,
      responseText,
      "📖 *ᴏᴄʀ*",
      `${extractedText.length} chars`,
      { quoted: m },
    );
  } catch (e) {
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}
export { pluginConfig as config, handler };
