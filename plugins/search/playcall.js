import axios from "axios";
import yts from "yt-search";
import fs from "fs";
import path from "path";
import crypto from "crypto";
import te from "../../src/lib/ourin-error.js";
import ytdl from "../../src/scraper/ytdl.js";

const pluginConfig = {
  name: "playcall",
  alias: ["telepon", "call", "playvn"],
  category: "search",
  description: "Memutar musik dari YouTube lewat panggilan suara WhatsApp",
  usage: ".playcall <judul lagu>",
  example: ".playcall komang",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 15,
  energi: 2,
  isEnabled: false,
};

async function downloadAudio(videoUrl) {
  try {
    const { data } = await axios.get(
      `https://my.izuka-api.xyz/api/downloader/ytmp3?url=${encodeURIComponent(videoUrl)}`,
      { timeout: 60000 }
    );
    const download = data?.result?.download_url;
    if (download) return download;
  } catch { }

  const fallback = await ytdl(videoUrl, "mp3");
  if (fallback?.status && fallback?.dl) return fallback.dl;

  throw new Error("Gagal mendapatkan URL audio");
}


async function handler(m, { sock, text }) {
  const query = (text || m.text || "").trim();

  if (!query) {
    return m.reply(
      `⚠️ *PANGGILAN MUSIK (PLAYCALL)*\n\n` +
      `Fitur ini digunakan untuk memutar musik YouTube langsung melalui panggilan telepon suara WhatsApp ke nomor kamu.\n\n` +
      `*PENGGUNAAN:*\n` +
      `- *${m.prefix}playcall <judul lagu>*\n\n` +
      `*CONTOH:*\n` +
      `- *${m.prefix}playcall surat cinta untuk starla*\n` +
      `- *${m.prefix}playcall komang*\n\n` +
      `_Pastikan nomormu dapat menerima panggilan suara WhatsApp._`
    );
  }

  if (!global.voipClient) {
    return m.reply(
      `⚠️ *LAYANAN BELUM SIAP*\n\n` +
      `Modul panggilan suara VoIP belum aktif atau sedang dalam proses inisialisasi pada server.`
    );
  }

  await m.react("🕕");

  const tmpDir = path.join(process.cwd(), "tmp");
  if (!fs.existsSync(tmpDir)) fs.mkdirSync(tmpDir, { recursive: true });
  const tmpFile = path.join(tmpDir, `call_${crypto.randomBytes(4).toString("hex")}.mp3`);

  try {
    const search = await yts(query);
    if (!search.videos.length) {
      await m.react("❌");
      return m.reply(`❌ Musik dengan judul *${query}* tidak ditemukan.`);
    }

    const video = search.videos[0];
    const audioUrl = await downloadAudio(video.url);

    const audioRes = await axios.get(audioUrl, {
      responseType: "arraybuffer",
      timeout: 60000,
    });
    fs.writeFileSync(tmpFile, Buffer.from(audioRes.data));

    const targetNumber = m.sender.split("@")[0].replace(/\D/g, "");

    await m.react("📞");
    await m.reply(
      `📞 *MEMULAI PANGGILAN*\n\n` +
      `- Judul: *${video.title}*\n` +
      `- Durasi: *${video.timestamp}*\n` +
      `- Tujuan: *+${targetNumber}*\n\n` +
      `_Panggilan sedang dialihkan ke WhatsApp kamu, silakan angkat telepon untuk mendengarkan lagu._`
    );

    const call = await global.voipClient.call(targetNumber, {
      audioSource: tmpFile,
      durationMs: 300000,
    });

    call.on("connected", () => {
      m.reply(`✅ *TERHUBUNG*\n\nPanggilan berhasil tersambung! Lagu *${video.title}* sedang diputar di telepon.`);
    });

    call.on("ended", (reason) => {
      try {
        if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
      } catch { }
      m.reply(`📵 *PANGGILAN BERAKHIR*\n\nPanggilan telepon telah selesai (${reason || "selesai"}).`);
    });

    call.on("error", () => {
      try {
        if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
      } catch { }
    });
  } catch (err) {
    try {
      if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
    } catch { }
    await m.react("☢");
    console.log(err)
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
