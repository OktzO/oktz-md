import yts from "yt-search";
import ytdl from "../../src/scraper/ytdl.js";
import axios from "axios";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "ytplay",
  alias: ["ytp", "playvideo"],
  category: "search",
  description: "Cari video YouTube lewat judul langsung putar/kirim videonya",
  usage: ".ytplay <judul>",
  example: ".ytplay melukis senja",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 20,
  energi: 3,
  isEnabled: true,
};

async function searchYoutube(query) {
  const r = await yts(query);
  const video = r.videos?.[0] || r.all?.find((x) => x.type === "video");
  if (!video?.url) throw new Error("Video tidak ditemukan.");
  return {
    url: video.url,
    title: video.title,
    author: video.author?.name || "YouTube",
    duration: video.seconds || 0,
    thumb: video.thumbnail,
  };
}

async function getYoutubeDirectUrl(videoUrl) {
  const result = await ytdl(videoUrl, "mp4");
  if (!result?.status || !result?.dl) {
    throw new Error(result?.mess || "Gagal mendapatkan direct link video.");
  }
  return { url: result.dl, title: result.title };
}

async function handler(m, { sock }) {
  const query = m.text?.trim();
  if (!query) {
    return m.reply(
      `🎬 *ʏᴛ ᴘʟᴀʏ*\n\n> Masukkan judul video\n\n\`Contoh: ${m.prefix}ytplay melukis senja\``,
    );
  }

  m.react("🕕");

  try {
    const yt = await searchYoutube(query);
    const info = await getYoutubeDirectUrl(yt.url);

    const dur = yt.duration
      ? `${Math.floor(yt.duration / 60)}:${String(yt.duration % 60).padStart(2, "0")}`
      : "-";

    const caption =
      `🎬 *${info.title || yt.title}*\n\n` +
      `> 📺 Channel: ${yt.author}\n` +
      `> ⏱️ Durasi: ${dur}\n` +
      `> 🌐 Sumber: YouTube\n\n` +
      `> 🎞️ Video 360p sedang dikirim...`;

    const thumbRes = await axios
      .get(yt.thumb, { responseType: "arraybuffer", timeout: 10000 })
      .catch(() => null);
    const jpegThumbnail = thumbRes
      ? Buffer.from(thumbRes.data)
      : undefined;

    await sock.sendMedia(m.chat, info.url, caption, m, {
      type: "video",
      mimetype: "video/mp4",
      fileName: `${(info.title || "video").replace(/[^\w\s-]/g, "").trim() || "video"}.mp4`,
      ...(jpegThumbnail ? { jpegThumbnail } : {}),
    });

    m.react("✅");
  } catch (err) {
    console.error("[ytplay error]", err.message || err);
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
