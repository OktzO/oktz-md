import axios from "axios";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "fakedana",
  alias: ["danafake", "fakedana"],
  category: "canvas",
  description: "Membuat gambar tampilan saldo atau transaksi Fake DANA",
  usage: ".fakedana <nominal>",
  example: ".fakedana 500000",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock, text }) {
  const nominal = (text || m.text || "").trim().replace(/[^0-9]/g, "");

  if (!nominal) {
    return m.reply(
      `⚠️ *PEMBUATAN FAKE DANA*\n\n` +
      `Fitur ini digunakan untuk membuat gambar saldo atau bukti transaksi DANA kustom secara otomatis.\n\n` +
      `*PENGGUNAAN:*\n` +
      `- *${m.prefix}fakedana <nominal>*\n\n` +
      `*CONTOH:*\n` +
      `- *${m.prefix}fakedana 500000*\n` +
      `- *${m.prefix}fakedana 1000000*\n\n` +
      `_Pastikan nominal berupa angka tanpa titik atau koma._`
    );
  }

  await m.react("🕕");

  try {
    const targetUrl = `https://api.nexray.eu.cc/maker/fakedana?nominal=${encodeURIComponent(nominal)}`;
    const res = await axios.get(targetUrl, {
      responseType: "arraybuffer",
    });

    const imageBuffer = Buffer.from(res.data);
    const nominalFormatted = Number(nominal).toLocaleString("id-ID");

    await sock.sendMessage(
      m.chat,
      {
        image: imageBuffer,
        caption: `✅ *BERHASIL*\n\nGambar *Fake DANA* dengan nominal *Rp ${nominalFormatted}* berhasil dibuat.`,
      },
      { quoted: m }
    );
    await m.react("✅");
  } catch (error) {
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };