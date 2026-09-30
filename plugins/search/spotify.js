import axios from "axios";
import { generateWAMessageFromContent } from "onigis";
import sharp from "sharp";
import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "spotify",
  alias: ["spotifysearch", "spsearch"],
  category: "search",
  description: "Mencari daftar lagu di Spotify berdasarkan judul atau artis",
  usage: ".spotify <query>",
  example: ".spotify neffex grateful",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock, text }) {
  if (!text) {
    return m.reply("❌ *Waduh, kata kuncinya mana nih?*\n\nKamu harus memasukkan judul lagu atau nama artis yang ingin dicari di Spotify. \n\nContoh penggunaan: `.spotify bruno mars`");
  }

  await m.react("🕕");

  try {
    const { data } = await resolver.resolve("spotify", { q: text });
    const hasil = (data?.tracks ?? []).slice(0, 5);

    if (hasil.length === 0) {
      await m.react("❌");
      return m.reply(`⚠️ *Maaf, lagu tidak ditemukan!* \n\nAku sudah mencari dengan kata kunci *${text}* tapi tidak ada hasil di Spotify. Coba gunakan judul yang lebih spesifik ya.`);
    }

    const firstResult = hasil[0];

    let contentText = `✨ *HASIL PENCARIAN SPOTIFY* ✨\n\nHalo! Aku berhasil menemukan beberapa lagu berdasarkan kata kunci *${text}*. Berikut adalah daftar teratasnya:\n\n`;

    hasil.forEach((t, i) => {
      contentText += `*${i + 1}. ${t.title}*\n`;
      contentText += `   🎤 Artis: ${t.artist}\n`;
      contentText += `   ⏱️ Durasi: ${t.duration}\n`;
      contentText += `   🔗 Link: ${t.url}\n\n`;
    });

    contentText += `*Catatan*: Kamu bisa menyalin link lagu di atas dan menggunakan perintah \`.spdl <link>\` untuk mengunduhnya secara langsung! Atau tekan tombol di bawah ini untuk lagu pertama. 🚀`;

    let thumbnailBuffer = null;
    try {
      if (firstResult.cover) {
        const imageResponse = await axios.get(firstResult.cover, { responseType: "arraybuffer" });
        thumbnailBuffer = await sharp(imageResponse.data).resize(300, 170).jpeg().toBuffer();
      }
    } catch (e) {
    }

    // Backend lokal mencari lewat YT Music, dan hasilnya tidak punya ID track
    // Spotify. Tombol unduh disembunyikan kalau begitu: `.spdl` hanya menerima
    // link open.spotify.com dan akan menolak tautan lain.
    const bisaUnduh = /^https?:\/\/open\.spotify\.com\/track\//i.test(firstResult.url);

    if (thumbnailBuffer) {
      const content = {
        buttonsMessage: {
          buttons: bisaUnduh ? [
            {
              buttonId: `.spdl ${firstResult.url}`,
              buttonText: { displayText: '🎵 Unduh Lagu Pertama' },
              type: 1,
            }
          ] : [],
          locationMessage: {
            jpegThumbnail: thumbnailBuffer,
            name: firstResult.title,
            address: `🎤 ${firstResult.artist} | ⏱️ ${firstResult.duration}`
          },
          contentText: contentText,
          footerText: '🚀 Spotify Search',
          headerType: 6,
        },
      };

      const msg = generateWAMessageFromContent(m.chat, content, { quoted: m });
      await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
    } else {
      await m.reply(contentText);
    }

    await m.react("✅");

  } catch (err) {
    console.error("[Spotify Search]", err.message);
    await m.react("☢");
    m.reply("😔 *Aduh, sepertinya API sedang bermasalah.* \n\nTerjadi kesalahan fatal saat mencoba memproses pencarian Spotify. Silakan coba lagi nanti ya!");
  }
}

export { pluginConfig as config, handler };
