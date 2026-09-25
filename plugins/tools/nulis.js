import * as _canvas from "@napi-rs/canvas";
import * as timeHelper from "../../src/lib/time.js";
import te from "../../src/lib/error.js";
import { saluranCtx } from "../../src/lib/context.js";
import { getAssetBuffer } from "../../src/lib/asset-manager.js";
const pluginConfig = {
  name: "nulis",
  alias: ["tulis", "write"],
  category: "tools",
  description: "Generate tulisan tangan di kertas",
  usage: ".nulis <teks>",
  example: ".nulis Aku cinta kamu selamanya",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};
getAssetBuffer("font");
let _fontRegistered = false;
function wrapText(ctx, text, maxWidth) {
  const words = text.split(" ");
  const lines = [];
  let currentLine = "";
  for (const word of words) {
    const testLine = currentLine + (currentLine ? " " : "") + word;
    const metrics = ctx.measureText(testLine);
    if (metrics.width > maxWidth && currentLine) {
      lines.push(currentLine);
      currentLine = word;
    } else {
      currentLine = testLine;
    }
  }
  if (currentLine) lines.push(currentLine);
  return lines;
}
async function handler(m, { sock }) {
  const text = m.args?.join(" ");
  if (!text) {
    return m.reply(
      `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
        `> \`${m.prefix}nulis <teks>\`\n\n` +
        `> Contoh:\n` +
        `> \`${m.prefix}nulis Aku cinta kamu selamanya\``,
    );
  }
  if (text.length > 500) {
    return m.reply(`❌ *ᴛᴇᴋs ᴛᴇʀʟᴀʟᴜ ᴘᴀɴᴊᴀɴɢ*\n\n> Maksimal 500 karakter`);
  }
  const inputUrl = getAssetBuffer("kertas");
  if (!inputUrl) {
    return m.reply(
      `❌ *ᴛᴇᴍᴘʟᴀᴛᴇ ᴛɪᴅᴀᴋ ᴀᴅᴀ*\n\n> File template kertas tidak ditemukan di config.assets`,
    );
  }
  await m.react("🕕");
  await m.reply(`🕕 *ᴍᴇᴍᴘʀᴏsᴇs...*\n\n> Membuat tulisan tangan...`);
  try {
    const { createCanvas, loadImage, GlobalFonts } = _canvas;
    if (!_fontRegistered) {
      try {
        const fontBuf = getAssetBuffer("font");
        if (fontBuf) {
          GlobalFonts.register(fontBuf, "Zahraaa");
        }
      } catch (err) {
        console.error("Gagal load font:", err);
      }
      _fontRegistered = true;
    }
    const bgBuf = getAssetBuffer("kertas");
    const bgImage = await loadImage(bgBuf);
    const canvas = createCanvas(bgImage.width, bgImage.height);
    const ctx = canvas.getContext("2d");
    ctx.drawImage(bgImage, 0, 0);
    const tgl = timeHelper.formatDate("DD/MM/YYYY");
    const hari = timeHelper.formatFull("dddd");
    ctx.font = "20px Zahraaa, Arial";
    ctx.fillStyle = "#1a1a2e";
    ctx.fillText(hari, 806, 78);
    ctx.font = "18px Zahraaa, Arial";
    ctx.fillText(tgl, 806, 102);
    ctx.font = "20px Zahraaa, Arial";
    const maxWidth = 600;
    const lineHeight = 28;
    const startX = 344;
    const startY = 142;
    const lines = wrapText(ctx, text, maxWidth);
    lines.forEach((line, i) => {
      ctx.fillText(line, startX, startY + i * lineHeight);
    });
    const buffer = canvas.toBuffer("image/jpeg");
    await m.react("✅");
    await sock.sendMedia(
      m.chat,
      buffer,
      `✅ *ʟᴜʟɪsᴀɴ ᴛᴀɴɢᴀɴ*\n\n> Hatihati ketahuan! 📖`,
      m,
      { type: "image", contextInfo: saluranCtx() },
    );
  } catch (error) {
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}
export { pluginConfig as config, handler };
