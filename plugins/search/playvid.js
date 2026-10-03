import yts from "yt-search";
import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "playvid",
  alias: ["playmp4"],
  category: "search",
  description: "Cari dan putar video dari YouTube",
  usage: ".playvid <query>",
  example: ".playvid windah basudara",
  cooldown: 15,
  energi: 2,
  isEnabled: true,
};

function formatViews(n) {
  if (!n) return "0";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return n.toString();
}

async function getVideoDownloadUrl(url) {
  const { data } = await resolver.resolve("youtube", { url: String(url).trim(), format: "mp4" });
  return data.url;
}

async function handler(m, { sock, _ }) {
  const query = m.text?.trim();
  if (!query) {
    return m.reply(`Halo *${m.pushName}* 👋\n\nUntuk mencari dan memutar video dari YouTube, silakan gunakan format:\n- \`${m.prefix}playvid <judul video>\`\n\nContoh:\n- \`${m.prefix}playvid windah basudara\``);
  }

  m.react("🕕");

  try {
    const search = await yts(query);
    if (!search.videos.length) throw new Error("Video tidak ditemukan");
    const video = search.videos[0];

    let info = `Halo *${m.pushName}*, ini video yang kamu cari:\n\n`;
    info += `📌 *Judul:* ${video.title}\n`;
    info += `👤 *Channel:* ${video.author.name}\n`;
    info += `⏱️ *Durasi:* ${video.duration.timestamp}\n`;
    info += `👀 *Views:* ${formatViews(video.views)}\n`;
    info += `📅 *Upload:* ${video.ago}\n\n`;
    info += `_⏳ Sedang mengunduh video, harap tunggu sebentar ya..._`;

    await sock.sendPreview(
      m.chat,
      {
        caption: video.url + "\n" + info,
        url: video.url,
        title: video.title,
        description: "YouTube Video",
        image: video.thumbnail,
        previewType: 1,
      },
      {
        quoted: m,
      },
    );

    const downloadUrl = await getVideoDownloadUrl(video.url);

    await sock.sendMedia(m.chat, downloadUrl, null, m, {
      type: "video",
    });

    m.react("✅");
  } catch (err) {
    // `CapabilityError.message` hanya menyatakan "semua backend gagal", alasan
    // sebenarnya ada di `tried` — ikut dicetak supaya kegagalan aggregator
    // masih bisa didiagnosis dari log.
    const rincian = Array.isArray(err?.tried)
      ? err.tried.map((t) => `${t.name}: ${t.reason}`).join(" | ")
      : String(err?.message ?? err);
    console.error(`[PlayVid] gagal — ${rincian}`);
    m.react("❌");
    m.reply(
      `Maaf *${m.pushName}*, fitur putar videonya sedang ada kendala atau video tersebut terlalu besar. Silakan coba lagi nanti ya!`,
    );
  }
}

export { pluginConfig as config, handler };