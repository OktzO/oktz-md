import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import { resolver } from "../../src/lib/resolve.js";
import config from "../../config.js";
import te from "../../src/lib/error.js";
import _sharp from 'sharp';

const pluginConfig = {
  name: "hd2",
  alias: ["enhance2", "upscale2", "aienhancer"],
  category: "tools",
  description: "Enhance gambar menjadi HD dengan AI (V3)",
  usage: ".hd2 (reply gambar)",
  example: ".hd2",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 30,
  energi: 2,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const isImage = m.isImage || (m.quoted && m.quoted.type === "imageMessage");

  if (!isImage) {
    let help = `✨ *FITUR HD ENHANCE V2*\n\n`
    help += `Tingkatkan resolusi gambar kamu menjadi jauh lebih HD dan tajam menggunakan AI!\n\n`
    help += `*Cara Penggunaan:*\n`
    help += `- Kirim gambar dan tambahkan pesan *${m.prefix}hd2*\n`
    help += `- Atau balas (reply) gambar yang sudah terkirim dengan perintah *${m.prefix}hd2*\n\n`
    help += `_Proses rendering mungkin memerlukan waktu beberapa detik hingga satu menit._`
    return m.reply(help);
  }

  await m.react("🕕");

  // Backend lokal dan aggregator sama-sama butuh path file di disk, sementara
  // `m.download()` hanya memberi Buffer. Berkanya ditulis sekali ke tmp dan
  // dihapus di `finally` — HD dipakai sering, jadi sisanya akan menumpuk.
  let media;
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

    media = path.join(os.tmpdir(), `hd-${crypto.randomUUID()}.jpg`);
    await fsp.writeFile(media, buffer);

    const { data } = await resolver.resolve("hd", { kind: "imglarger", media });

    if (!data?.url) {
      await m.react("❌");
      return m.reply(`Maaf, AI gagal memproses gambarmu kali ini. Silakan coba lagi dalam beberapa saat!`);
    }

    await m.react("✅");

    const thumbBuffer = await _sharp(buffer).resize(50, 50).jpeg({ quality: 30 }).toBuffer();

    await sock.sendMessage(
      m.chat,
      {
        document: { url: data.url },
        mimetype: "image/jpeg",
        jpegThumbnail: thumbBuffer,
        fileName: `HD_BY_${config.bot.name}.jpg`,
      },
      { quoted: m },
    );

  } catch (error) {
    console.error("[HD2 Plugin Error]", error);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  } finally {
    // Dibersihkan apa pun hasilnya: sukses, kegagalan, atau reply "gagal
    // memproses" yang keluar lebih dulu.
    if (media) await fsp.unlink(media).catch(() => {});
  }
}

export { pluginConfig as config, handler };
