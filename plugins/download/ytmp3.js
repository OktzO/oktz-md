import { resolver } from "../../src/lib/resolve.js";

const pluginConfig = {
  name: "ytmp3",
  alias: ["youtubemp3", "ytaudio"],
  category: "download",
  description: "Download audio YouTube",
  usage: ".ytmp3 <url>",
  example: ".ytmp3 https://youtube.com/watch?v=xxx",
  cooldown: 20,
  energi: 2,
  isEnabled: true,
};

/**
 * Host dicek per label, bukan dengan `includes` seperti versi lama:
 * `youtube.com.evil.example` dan `notyoutube.com` lolos `includes` tapi bukan
 * YouTube. Guard plugin tetap ada supaya user dapat jawabannya sebelum
 * kapabilitas sempat mencatat kegagalan host; normalisasi short link ada di
 * kapabilitas, jadi `youtu.be/...` tetap diterima di sini.
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

/**
 * Satu-satunya jalan ke media: scraper lokal lebih dulu, aggregator jadi
 * cadangan. Jalur aggregator tidak lagi diperiksa di plugin ini karena `hit`
 * sudah menolak sendiri kalau key kosong, dan jalur lokal tidak butuh key
 * sama sekali.
 *
 * `deps.resolve` hanya untuk test —(testes/upstream-diagnostics.test.mjs)
 * menyuntikkan kegagalan aggregator tanpa jaringan.
 */
export async function getAudioDownload(url, deps = {}) {
  const run = deps.resolve ?? ((capability, args) => resolver.resolve(capability, args));
  const { data } = await run("youtube", { url: String(url).trim(), format: "mp3" });
  return data;
}

async function handler(m, { sock, resolve }) {
  const url = m.text?.trim();
  if (!url)
    return m.reply(`Contoh: ${m.prefix}ytmp3 https://youtube.com/watch?v=xxx`);
  if (!hostYoutube(url)) return m.reply("❌ URL harus YouTube");

  m.react("🕕");

  try {
    // `normalize` menolak respons tanpa URL unduhan, jadi `url` di sini tidak
    // mungkin kosong. Versi lama melakukan `fallbackToMp3Buffer` — mengunduh
    // seluruh audio ke memori hanya kalau aggregator gagal; sekarang scraper
    // lokal justru yang jalan pertama, jadi jalur buffer itu akan jadi jalur
    // utama dan menarik video 2 jam ke kotak 1GB setiap kali `.ytmp3` dipakai.
    const { title, url: download } = await getAudioDownload(url, { resolve });

    await sock.sendMedia(m.chat, download, null, m, {
      type: "audio",
      mimetype: "audio/mpeg",
      ptt: false,
      fileName: `${title || "audio"}.mp3`,
    });
    m.react("✅");
  } catch (err) {
    // `CapabilityError.message` hanya menyatakan "semua backend gagal";
    // alasannya ada di `tried`. Tanpa ikut dicetak, "aggregator menjawab 500"
    // hilang dari log dan diagnosis di hilir ikut buta.
    const rincian = Array.isArray(err?.tried)
      ? err.tried.map((t) => `${t.name}: ${t.reason}`).join(" | ")
      : String(err?.message ?? err);
    console.error(`[YTMP3] gagal — ${rincian}`);
    m.react("❌");
    m.reply("Gagal mengunduh audio.");
  }
}

export { pluginConfig as config, handler };