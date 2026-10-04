import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "douyindl",
  alias: ["douyin", "dydl"],
  category: "download",
  description: "Download video/audio dari Douyin (TikTok China)",
  usage: ".douyindl <url>",
  example: ".douyindl https://v.douyin.com/xxx",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

/**
 * Guard host, bukan `includes`. `douyin.com.evil.example` dan `notdouyin.com`
 * lolos pemeriksaan substring; kalau guard plugin hanya cek itu, guard
 * kapabilitas yang menolaknya dan `cobaBackend` mencatat kegagalan — tiga link
 * palsu dari satu user sudah cukup membuka breaker snapvideotools 30 detik untuk
 * semua orang, termasuk yang sedang mengunduh video Douyin yang sebenarnya bisa
 * diunduh.
 */
function hostDouyin(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return (
    host === "douyin.com" ||
    host.endsWith(".douyin.com") ||
    host === "iesdouyin.com" ||
    host.endsWith(".iesdouyin.com")
  );
}

async function handler(m, { sock }) {
  const text = m.text?.trim();
  if (!text) {
    m.react("❌");
    return m.reply(
      `🎵 *Douyin Downloader*\n\n` +
        `Download video atau audio dari Douyin (TikTok China).\n\n` +
        `*PENGGUNAAN:*\n` +
        `> *${m.prefix}douyindl <link>*\n\n` +
        `*CONTOH:*\n` +
        `> *${m.prefix}douyindl https://v.douyin.com/xxx*`,
    );
  }

  // Tanpa reaksi di sini, sama seperti guard plugin sfiledl/videy/pindl: input
  // salah bukan kegagalan proses, jadi tidak layak memakai centang ❌ yang di
  // command ini dipakai untuk "tidak ada argumen".
  if (!hostDouyin(text)) {
    return m.reply("❌ URL tidak valid. Gunakan link dari douyin.com atau v.douyin.com");
  }

  m.react("🕕");

  try {
    // Scraper lokal lebih dulu, aggregator jadi cadangan. `normalize` sudah
    // menolak respons tanpa URL video, jadi `video` di sini tidak mungkin
    // kosong — parser aggregator lama tidak lagi perlu hidup di plugin ini.
    const { data } = await resolver.resolve("douyin", { url: text });

    let caption = `🎵 *${data.platform || "Douyin"}*\n\n${data.title || ""}`;

    if (data.video) {
      await sock.sendMedia(m.chat, data.video, caption, m, {
        type: "video",
      });
    }

    if (data.audio) {
      await sock.sendMedia(m.chat, data.audio, null, m, {
        type: "audio",
      });
    }

    m.react("✅");
  } catch (e) {
    console.error(e);
    m.react("☢");
    m.reply("❌ Gagal mengambil data Douyin, coba lagi nanti");
  }
}

export { pluginConfig as config, handler };