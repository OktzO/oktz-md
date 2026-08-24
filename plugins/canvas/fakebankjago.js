import axios from "axios";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "fakebankjago",
  alias: ["fakebank-jago", "fakejago", "bankjago"],
  category: "canvas",
  description: "Membuat gambar tampilan rekening atau saldo Fake Bank Jago",
  usage: ".fakebankjago <nama>|<saldo>",
  example: ".fakebankjago Furina|4342342",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock, text }) {
  const rawText = (text || m.text || "").trim();
  let nama = "";
  let saldo = "";

  if (rawText.includes("|")) {
    const parts = rawText.split("|").map((v) => v.trim());
    nama = parts[0] || "";
    saldo = parts[1] || "";
  } else if (rawText.includes(",")) {
    const parts = rawText.split(",").map((v) => v.trim());
    nama = parts[0] || "";
    saldo = parts[1] || "";
  }

  const saldoClean = saldo.replace(/[^0-9]/g, "");

  if (!nama || !saldoClean) {
    return m.reply(
      `⚠️ *PEMBUATAN FAKE BANK JAGO*\n\n` +
      `Fitur ini digunakan untuk membuat gambar tampilan akun Bank Jago kustom dengan nama dan nominal saldo yang ditentukan.\n\n` +
      `*PENGGUNAAN:*\n` +
      `- *${m.prefix}fakebankjago <nama>|<saldo>*\n` +
      `- *${m.prefix}fakebankjago <nama>,<saldo>*\n\n` +
      `*CONTOH:*\n` +
      `- *${m.prefix}fakebankjago Furina|4342342*\n` +
      `- *${m.prefix}fakebankjago Zann,5000000*\n\n` +
      `_Gunakan pemisah tanda vertikal | atau koma antara nama dan saldo._`
    );
  }

  await m.react("🕕");

  try {
    const targetUrl = `https://api.nexray.eu.cc/maker/fakebank-jago?nama=${encodeURIComponent(nama)}&saldo=${encodeURIComponent(saldoClean)}`;
    const res = await axios.get(targetUrl, {
      responseType: "arraybuffer",
    });

    const imageBuffer = Buffer.from(res.data);
    const saldoFormatted = Number(saldoClean).toLocaleString("id-ID");

    await sock.sendMessage(
      m.chat,
      {
        image: imageBuffer,
        caption: `✅ *BERHASIL*\n\nGambar *Fake Bank Jago* atas nama *${nama}* dengan saldo *Rp ${saldoFormatted}* berhasil dibuat.`,
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