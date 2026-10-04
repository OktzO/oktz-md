import { resolver, adaNormalisasiGagal } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "spotifydl",
  alias: ["spdl", "spotify-dl", "spotdl"],
  category: "download",
  description: "Unduh lagu favoritmu langsung dari Spotify tanpa ribet!",
  usage: ".spdl <link>",
  example: ".spdl https://open.spotify.com/track/...",
  cooldown: 15,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.text?.trim();

  if (!text || !/open\.spotify\.com\/track/i.test(text)) {
    return m.reply("❌ *Waduh, link Spotify-nya mana nih atau kurang tepat!*\n\nKamu harus memasukkan tautan (link) lagu dari Spotify yang valid. Pastikan itu adalah link ke track/lagu ya! \n\nContoh: `.spdl https://open.spotify.com/track/3RY0NyQQXxuAiyk5eAS4fC`");
  }

  await m.react("🕕");

  try {
    const { data } = await resolver.resolve("spotify", { url: text });
    // Pemeriksaan `!data?.url` yang dulu ada di sini sudah dihapus: `normalize`
    // menolak respons tanpa URL unduhan, jadi setelah resolve sukses `data.url` tidak
    // mungkin kosong dan cabang itu tidak pernah bisa bernilai true. Yang dicakupnya
    // kini ditangani di `catch` lewat `adaNormalisasiGagal`.

    const filename = `${data.artist || "Spotify"} - ${data.title || "Audio"}.mp3`;

    await sock.sendMessage(m.chat, {
      audio: { url: data.url },
      mimetype: "audio/mpeg",
      fileName: filename,
      ptt: false
    }, { quoted: m });

    await m.react("✅");

  } catch (error) {
    console.error("[Spotify DL Error]", error);
    // Semua backend menjawab tapi tidak ada yang bisa dikirim: track-nya tidak ada
    // di sana atau terkunci region. Pemisahan dilakukan dari `tried`, karena
    // `normalize` menutup jalan "data.url kosong" supaya plugin tidak pernah mengirim
    // tautan kosong. Bedakan dari kegagalan proses, yang harus dilaporkan ke owner.
    if (adaNormalisasiGagal(error)) {
      await m.react("❌");
      return m.reply("⚠️ *Gagal mengambil lagu!* \n\nServer tidak merespon dengan tautan unduhan yang valid.");
    }
    await m.react("❌");
    m.reply("😔 *Terjadi kesalahan sistem saat memproses tautan Spotify tersebut.* Mohon coba lagi nanti ya!");
  }
}

export { pluginConfig as config, handler };
