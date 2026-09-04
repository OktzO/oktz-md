import { createCanvas } from "@napi-rs/canvas";
import te from "../../src/lib/ourin-error.js";
import config from "../../config.js";

const pluginConfig = {
  name: "ttp",
  alias: ["texttopicture"],
  category: "maker",
  description: "Membuat stiker keren dari teks",
  usage: ".ttp <teks>",
  example: ".ttp Hai Cantik",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.args.join(" ") || m.text?.trim();

  if (!text) {
    return m.reply("❌ *Waduh, teksnya mana nih?*\n\nKamu harus memasukkan teks yang ingin dijadikan stiker.\n\nContoh: `.ttp Hai Cantik`");
  }

  await m.react("🕕");

  try {
    const canvas = createCanvas(512, 512);
    const ctx = canvas.getContext("2d");

    ctx.fillStyle = "#FFFFFF";
    ctx.fillRect(0, 0, 512, 512);

    let fontSize = 60;
    ctx.font = `bold ${fontSize}px sans-serif`;
    let lines = [];
    let words = text.split(" ");
    let currentLine = "";

    for (const word of words) {
      const test = currentLine ? currentLine + " " + word : word;
      if (ctx.measureText(test).width > 480) {
        lines.push(currentLine);
        currentLine = word;
      } else {
        currentLine = test;
      }
    }
    if (currentLine) lines.push(currentLine);

    const totalH = lines.length * fontSize * 1.3;
    let startY = (512 - totalH) / 2 + fontSize;

    ctx.fillStyle = "#000000";
    ctx.textAlign = "center";
    ctx.textBaseline = "alphabetic";

    for (const line of lines) {
      ctx.font = `bold ${fontSize}px sans-serif`;
      if (ctx.measureText(line).width > 480) {
        fontSize = Math.max(20, fontSize - 4);
        ctx.font = `bold ${fontSize}px sans-serif`;
      }
      ctx.fillText(line, 256, startY);
      startY += fontSize * 1.3;
    }

    const buffer = await canvas.encode("png");

    await sock.sendImageAsSticker(m.chat, buffer, m, {
      packname: config.sticker.packname,
      author: config.sticker.author,
    });

    await m.react("✅");

  } catch (err) {
    console.error("[TTP Maker]", err.message);
    await m.react("☢");
    m.reply("😔 *Terjadi masalah di sistem kami.* \n\nSistem gagal menghubungi server pembuat stiker. Silakan coba beberapa saat lagi ya.");
  }
}

export { pluginConfig as config, handler };
