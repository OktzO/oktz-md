import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "ytmp4",
  alias: ["youtubemp4", "ytvideo"],
  category: "download",
  description: "Download video YouTube",
  usage: ".ytmp4 <url>",
  example: ".ytmp4 https://youtube.com/watch?v=xxx",
  cooldown: 20,
  energi: 2,
  isEnabled: true,
};

/**
 * Host dicek per label, bukan dengan `includes` seperti versi lama:
 * `youtube.com.evil.example` dan `notyoutube.com` lolos `includes` tapi bukan
 * YouTube. Normalisasi short link ada di kapabilitas, jadi `youtu.be/...`,
 * `m.youtube.com`, dan `/shorts/...` tetap diterima.
 */
function hostYoutube(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return host === "youtu.be" || host === "youtube.com" || host.endsWith(".youtube.com");
}

async function getVideoDownloadUrl(url, deps = {}) {
  const run = deps.resolve ?? ((capability, args) => resolver.resolve(capability, args));
  const { data } = await run("youtube", { url: String(url).trim(), format: "mp4" });
  return data.url;
}

async function handler(m, { sock }) {
  const url = m.text?.trim();
  if (!url)
    return m.reply(`Contoh: ${m.prefix}ytmp4 https://youtube.com/watch?v=xxx`);
  if (!hostYoutube(url)) return m.reply("❌ URL harus YouTube");

  m.react("🕕");

  try {
    // `normalize` menolak respons tanpa URL unduhan, jadi hasil di sini tidak
    // mungkin kosong — versi lama bisa mengembalikan `undefined` lalu mengirim
    // `sendMedia` dengan tidak ada file.
    const downloadUrl = await getVideoDownloadUrl(url);

    await sock.sendMedia(m.chat, downloadUrl, null, m, {
      type: "video",
    });
    m.react("✅");
  } catch (err) {
    const rincian = Array.isArray(err?.tried)
      ? err.tried.map((t) => `${t.name}: ${t.reason}`).join(" | ")
      : String(err?.message ?? err);
    console.error(`[YTMP4] gagal — ${rincian}`);
    m.react("❌");
    m.reply("Gagal mengunduh video.");
  }
}

export { pluginConfig as config, handler };