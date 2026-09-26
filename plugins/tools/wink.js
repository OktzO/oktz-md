import winkEnhance from "../../src/scraper/wink.js";

const MAX_BYTES = 50 * 1024 * 1024;

const pluginConfig = {
  name: "wink",
  alias: ["winkenhance", "winkhd", "wenhance"],
  category: "tools",
  description: "Meningkatkan kualitas video menjadi Ultra HD dengan Wink AI",
  usage: ".wink (reply video)",
  example: ".wink",
  isOwner: false,
  isPremium: true,
  isGroup: false,
  isPrivate: false,
  cooldown: 120,
  energi: 3,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const isVideoMessage = m.isVideo || m.quoted?.type === "videoMessage";
  const isDocumentMessage =
    (m.type === "documentMessage" &&
      m.message?.documentMessage?.mimetype?.startsWith("video")) ||
    (m.quoted?.type === "documentMessage" &&
      m.quoted?.message?.documentMessage?.mimetype?.startsWith("video"));

  if (!isVideoMessage && !isDocumentMessage) {
    return m.reply(
      `✨ *ᴡɪɴᴋ ᴠɪᴅᴇᴏ ᴇɴʜᴀɴᴄᴇʀ*\n\n` +
        `> Bikin video buram jadi *Ultra HD* pakai AI Wink!\n\n` +
        `*Cara pakai:*\n` +
        `> Kirim/reply video lalu caption \`${m.prefix}wink\`\n\n` +
        `⚠️ _Fitur Premium, proses estimasi 1-4 menit. Kalau lewat 4 menit ` +
        `prosesnya dibatalkan — bukan dikasih video asal._`,
    );
  }

  await m.react("🕕");

  try {
    // Kalau video-nya di-reply, unduh dari quoted. Kalau tidak, dari pesan ini.
    // Urutan ini penting: m.quoted bisa ada tapi bukan video (mis. user reply
    // teks sambil command), dan m.download() yang tidak ada disitu akan
    // mengembalikan null sehingga video gagal terbaca.
    const fromQuoted =
      m.quoted?.isMedia === true && typeof m.quoted.download === "function";
    const videoBuffer = fromQuoted
      ? await m.quoted.download()
      : await m.download();

    if (!videoBuffer || videoBuffer.length === 0) {
      await m.react("❌");
      return m.reply(`❌ *GAGAL*\n\nVideonya gagal diunduh, coba kirim ulang ya!`);
    }

    if (videoBuffer.length > MAX_BYTES) {
      await m.react("❌");
      return m.reply(
        `❌ *FILE TERLALU BESAR*\n\nMaksimal *50MB*, video kamu *${(videoBuffer.length / 1024 / 1024).toFixed(1)}MB*.`,
      );
    }

    await m.reply(
      `🎬 *ᴘʀᴏsᴇs ᴡɪɴᴋ ᴇɴʜᴀɴᴄᴇ ᴅɪᴍᴜʟᴀɪ*\n\n` +
        `> Video lagi diproses AI Wink biar jadi *Ultra HD* ✨\n` +
        `> Maksimal *4 menit*. Kalau prosesnya tidak selesai,command ini ` +
        `batal — tidak akan dikasih video setengah jalan.`,
    );

    const result = await winkEnhance(videoBuffer, {
      filename: `wink-${Date.now()}.mp4`,
    });

    if (!result?.resultUrl) {
      throw new Error("Wink selesai tanpa URL hasil");
    }

    await sock.sendMedia(
      m.chat,
      result.resultUrl,
      `✨ *ᴡɪɴᴋ ᴇɴʜᴀɴᴄᴇ sᴇʟᴇsᴀɪ!*\n\n> Ini dia hasilnya, udah jadi *Ultra HD* kan? 😍`,
      m,
      {
        type: "video",
        mimetype: "video/mp4",
        fileName: `WINK-HD-${Date.now()}.mp4`,
      },
    );

    await m.react("✅");
  } catch (err) {
    console.error(`[wink] gagal: ${err?.message || err}`);
    await m.react("❌");
    // Bedakan "proses tidak selesai" dari "server error" — user perlu tau
    // apakah harus coba lagi atau videonya memang tidak bisa diproses.
    const timedOut = /tidak selesai/i.test(err?.message || "");
    await m.reply(
      timedOut
        ? `⏱️ *ᴘʀᴏsᴇs ᴋᴇʜᴀʟᴜᴜ ᴛɪᴍᴇ ɴᴏᴜᴛ*\n\n` +
          `> Wink tidak selesai dalam batas waktu, jadi tidak ada hasil yang bisa dikirim.\n` +
          `> Energi kakak tidak terbuang percuma. Coba lagi dengan video yang lebih pendek ya.`
        : `❌ *ᴡɪɴᴋ ᴇɴʜᴀɴᴄᴇ ɢᴀɢᴀʟ*\n\n> \`${String(err?.message || err).slice(0, 160)}\`\n> Coba lagi nanti ya.`,
    );
  }
}

export { pluginConfig as config, handler };
