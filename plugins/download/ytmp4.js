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
 *
 * Teks user juga sering berupa tautan tanpa skema (`youtu.be/...`) atau tautan
 * di dalam kalimat ("putar https://youtu.be/... dong"). Keduanya harus lolos
 * guard: versi lama menerimanya, dan versi yang lebih ketat hanya memindahkan
 * penolakan ke kapabilitas tanpa memperbaiki apa pun. Kandidat diambil dengan
 * urutan yang sama seperti `kandidatTautan` di src/capabilities/youtube.js.
 */
function hostYoutube(url) {
  const mentah = String(url ?? "").trim();
  const lengkap = /\bhttps?:\/\/[^\s<>"']+/i.exec(mentah)?.[0];
  const dasar = lengkap ?? mentah.split(/\s+/).find((token) => /youtu/i.test(token)) ?? mentah;
  let parsed;
  try {
    parsed = new URL(/^[a-z][a-z0-9+.-]*:/i.test(dasar) ? dasar : `https://${dasar}`);
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return host === "youtu.be" || host === "youtube.com" || host.endsWith(".youtube.com");
}

async function getVideoDownloadUrl(url) {
  const { data } = await resolver.resolve("youtube", { url: String(url).trim(), format: "mp4" });
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