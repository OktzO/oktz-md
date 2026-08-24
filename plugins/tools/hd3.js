import axios from "axios";
import FormData from "form-data";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";
import _sharp from 'sharp';

const pluginConfig = {
  name: "hd3",
  alias: ["enhance3", "upscale3", "unblur"],
  category: "tools",
  description: "Memperjelas gambar blur menjadi tajam dengan AI (Unblur)",
  usage: ".hd3 (reply gambar)",
  example: ".hd3",
  cooldown: 20,
  energi: 2,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const isImage = m.isImage || (m.quoted && m.quoted.type === "imageMessage");

  if (!isImage) {
    let help = `✨ *FITUR HD ENHANCE V3 (UNBLUR)*\n\n`
    help += `Fitur canggih untuk memperbaiki gambar yang buram (blur) menjadi jelas dan tajam kembali menggunakan kecerdasan buatan!\n\n`
    help += `*Cara Penggunaan:*\n`
    help += `- Kirim gambar dan tambahkan pesan *${m.prefix}hd3*\n`
    help += `- Atau balas (reply) gambar yang sudah terkirim dengan perintah *${m.prefix}hd3*\n\n`
    help += `_Proses rendering mungkin memerlukan waktu beberapa saat._`
    return m.reply(help);
  }

  await m.react("🕕");

  try {
    let buffer;
    if (m.quoted && m.quoted.isMedia) {
      buffer = await m.quoted.download();
    } else if (m.isMedia) {
      buffer = await m.download();
    }

    if (!buffer) {
      await m.react("❌");
      return m.reply(`Maaf, sistem gagal mengunduh gambar yang kamu berikan. Silakan coba kirim ulang gambarnya!`);
    }

    const form = new FormData();
    form.append("image", buffer, { filename: "image.jpg", contentType: "image/jpeg" });

    const response = await axios.post("https://my.izuka-api.xyz/api/tools/unblur", form, {
      headers: form.getHeaders(),
      timeout: 60000
    });

    const data = response.data;
    if (!data || !data.status || !data.result || !data.result.output_url || !data.result.output_url[0]) {
      await m.react("❌");
      return m.reply(`Maaf, AI gagal memproses gambarmu kali ini. Silakan coba lagi dalam beberapa saat!`);
    }

    await m.react("✅");

    const resultUrl = data.result.output_url[0];
    const thumbBuffer = await _sharp(buffer).resize(50, 50).jpeg({ quality: 30 }).toBuffer();

    await sock.sendMessage(
      m.chat,
      {
        document: { url: resultUrl },
        mimetype: "image/jpeg",
        jpegThumbnail: thumbBuffer,
        fileName: `UNBLUR_BY_${config.bot.name}.jpg`,
      },
      { quoted: m },
    );

  } catch (error) {
    console.error("[HD3 Plugin Error]", error);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
