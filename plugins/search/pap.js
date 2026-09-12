import {
  jidNormalizedUser,
  generateWAMessageFromContent,
  prepareWAMessageMedia,
} from "ourin";
import axios from "axios";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";
import { f } from "../../src/lib/ourin-http.js";

const pluginConfig = {
  name: "pap",
  alias: ["papcewe", "papcowo", "papfemboy"],
  category: "search",
  description: "Minta pap cewe, cowo, atau femboy dari Pinterest",
  usage: ".pap <cewe/cowo/femboy>",
  example: ".pap cewe",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

/**
 * Kata kunci pencarian Pinterest per tipe.
 * Dipisah dari `arg` supaya hasilnya relevan — query mentah ("cewe")
 * mengembalikan campuran yang tidak selalu sesuai.
 */
const QUERIES = {
  cewe: ["cewe cantik indonesia", "gadis cantik", "wanita indonesia cantik"],
  cowo: ["cowo ganteng indonesia", "pria tampan", "cowok ganteng"],
  femboy: ["femboy", "femboy anime", "femboy aesthetic"],
};

const VALID_TYPES = Object.keys(QUERIES);

/**
 * Hook injeksi dependensi untuk pengujian. Dipanggil hanya dari test;
 * tidak mengubah perilaku produksi (default = implementasi nyata).
 * @internal
 */
export function __setDeps(overrides = {}) {
  if (overrides.fetchJson) fetchProviders.fetchJson = overrides.fetchJson;
  if (overrides.fetchImageBuffer) fetchImageBufferImpl = overrides.fetchImageBuffer;
}

// Indirection supaya test bisa mengganti fetch tanpa menyentuh jaringan.
const fetchProviders = { fetchJson: (url) => f(url) };
let fetchImageBufferImpl = null;

/**
 * Deteksi buffer gambar lewat magic number; menolak halaman error HTML
 * yang dikembalikan CDN dengan status 200.
 */
function isImage(buf) {
  if (!Buffer.isBuffer(buf) || buf.length < 4) return false;
  // JPEG: FF D8 FF
  if (buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) return true;
  // PNG: 89 50 4E 47
  if (
    buf[0] === 0x89 &&
    buf[1] === 0x50 &&
    buf[2] === 0x4e &&
    buf[3] === 0x47
  )
    return true;
  // WebP: "RIFF" .... "WEBP"
  if (
    buf.toString("ascii", 0, 4) === "RIFF" &&
    buf.toString("ascii", 8, 12) === "WEBP"
  )
    return true;
  return false;
}

/** Ambil buffer gambar; Pinterest butuh Referer supaya tidak 403. */
async function fetchImageBuffer(url) {
  if (fetchImageBufferImpl) return fetchImageBufferImpl(url);
  const res = await axios.get(url, {
    responseType: "arraybuffer",
    timeout: 15000,
    headers: { Referer: "https://www.pinterest.com/" },
  });
  return Buffer.from(res.data);
}

/**
 * Provider Pinterest.
 * - cuki.biz.id: MATI — apikey "cuki-x" selalu 401 (Invalid API key).
 * - azbry.com:   provider utama, tapi sesekali 502 (tanpa peringatan).
 * Karena itu dipakai daftar provider + fallback berantai, dan setiap
 * provider dibungkus try/catch supaya satu provider down tidak
 * menggagalkan seluruh command.
 */
const PROVIDERS = [
  {
    name: "azbry",
    build: (q) =>
      `https://api.azbry.com/api/search/pinterest?q=${encodeURIComponent(q)}`,
    // azbry: { status, result: [{ image, ... }] }
    extract: (data) =>
      (data?.result || [])
        .map((it) => it?.image || it?.images_url)
        .filter((u) => typeof u === "string" && u.startsWith("http")),
  },
  {
    name: "cuki",
    build: (q, apiKey) =>
      apiKey
        ? `https://api.cuki.biz.id/api/search/pinterest?apikey=${encodeURIComponent(
            apiKey,
          )}&query=${encodeURIComponent(q)}&type=image`
        : null,
    // cuki: { data: { results: [{ image_url }] } }
    extract: (data) =>
      (data?.data?.results || [])
        .map((it) => it?.image_url)
        .filter((u) => typeof u === "string" && u.startsWith("http")),
  },
];

/** Ambil daftar URL gambar: coba tiap provider × tiap query sampai dapat. */
async function fetchImageUrls(type) {
  const queries = QUERIES[type] || [];
  const apiKey = config.APIkey?.cuki || "";

  for (const provider of PROVIDERS) {
    for (const q of queries) {
      let url = null;
      try {
        url = provider.build(q, apiKey);
      } catch {
        url = null;
      }
      if (!url) continue;
      try {
        const data = await fetchProviders.fetchJson(url);
        const results = provider.extract(data);
        if (results.length > 0) return results;
      } catch {
        continue; // provider/query ini gagal, lanjut ke berikutnya
      }
    }
  }
  return [];
}

/**
 * Bungkus react/reply agar tidak pernah melempar keluar handler.
 * Kalau koneksi putus saat mengirim react, handler yang tidak
 * membungkusnya akan jadi unhandledRejection (bisa mematikan proses
 * di Node 20). Helper ini membuat notifikasi bersifat best-effort.
 */
const safeReact = async (m, emoji) => {
  try {
    if (typeof m?.react === "function") await m.react(emoji);
  } catch {
    /* abaikan: notifikasi bersifat opsional */
  }
};

const safeReply = async (m, text) => {
  try {
    if (typeof m?.reply === "function") await m.reply(text);
  } catch {
    /* abaikan */
  }
};

async function handler(m, { sock }) {
  // m.args bisa berisi non-string dari beberapa parser; paksa ke string aman.
  const raw = m.args?.[0];
  const arg = typeof raw === "string" ? raw.trim().toLowerCase() : "";

  if (!arg || !VALID_TYPES.includes(arg)) {
    await safeReply(m,
      `❌ Pilih salah satu tipe pap yang tersedia: \`cewe\`, \`cowo\`, atau \`femboy\`.\n\nContoh: \`.pap cewe\``
    );
    return;
  }

  await safeReact(m, "🕕");

  try {
    const urls = await fetchImageUrls(arg);
    if (urls.length === 0) {
      await safeReact(m, "❌");
      await safeReply(m,
        `❌ Waduh, pap ${arg} lagi kosong nih. Coba lagi nanti.`
      );
      return;
    }

    // Acak urutan lalu coba satu-satu: URL Pinterest bisa mati/403.
    // Ambil yang pertama kali berhasil di-download.
    const shuffled = [...urls].sort(() => Math.random() - 0.5);
    let buffer = null;
    for (const url of shuffled) {
      try {
        const buf = await fetchImageBuffer(url);
        // Validasi di SINI (bukan di dalam fetchImageBuffer) supaya tetap
        // berlaku berapa pun sumber buffer-nya: tolak buffer kosong,
        // terlalu kecil (placeholder/error page CDN), atau bukan JPEG/PNG.
        if (Buffer.isBuffer(buf) && buf.length > 1000 && isImage(buf)) {
          buffer = buf;
          break;
        }
      } catch {
        continue;
      }
    }

    if (!buffer) {
      await safeReact(m, "❌");
      await safeReply(m, "⚠️ Gambar tidak tersedia. Coba lagi nanti.");
      return;
    }

    const media = await prepareWAMessageMedia(
      { image: buffer },
      { upload: sock.waUploadToServer },
    );

    if (!media?.imageMessage) {
      await safeReact(m, "❌");
      await safeReply(m, "⚠️ Gagal menyiapkan gambar. Coba lagi nanti.");
      return;
    }

    const prefix = m.prefix || ".";
    const msg = generateWAMessageFromContent(
      m.chat,
      {
        viewOnceMessage: {
          message: {
            messageContextInfo: {
              deviceListMetadata: {},
              deviceListMetadataVersion: 2,
            },
            interactiveMessage: {
              body: { text: `📸 *PAP ${arg.toUpperCase()}*` },
              footer: { text: "Pilih menu pap lainnya di bawah ini 👇" },
              header: {
                hasMediaAttachment: true,
                imageMessage: media.imageMessage,
              },
              nativeFlowMessage: {
                buttons: [
                  {
                    name: "quick_reply",
                    buttonParamsJson: JSON.stringify({
                      display_text: "🔁 Next",
                      id: `${prefix}pap ${arg}`,
                    }),
                  },
                  {
                    name: "quick_reply",
                    buttonParamsJson: JSON.stringify({
                      display_text: "👧 Cewe",
                      id: `${prefix}pap cewe`,
                    }),
                  },
                  {
                    name: "quick_reply",
                    buttonParamsJson: JSON.stringify({
                      display_text: "👦 Cowo",
                      id: `${prefix}pap cowo`,
                    }),
                  },
                  {
                    name: "quick_reply",
                    buttonParamsJson: JSON.stringify({
                      display_text: "⚧ Femboy",
                      id: `${prefix}pap femboy`,
                    }),
                  },
                ],
              },
            },
          },
        },
      },
      { quoted: m, userJid: jidNormalizedUser(sock.user?.id) },
    );

    await sock.relayMessage(m.chat, msg.message, {
      messageId: msg.key.id,
    });

    await safeReact(m, "✅");
  } catch (error) {
    console.error("[PAP Search]", error);
    await safeReact(m, "☢");
    await safeReply(m, te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
