import { findSimilarCommands } from "./similarity.js";

/**
 * WhatsApp native_flow: tombol di grid 2 kolom, max 3 baris = 6 tombol.
 * Jumlah ganjil bikin satu kotak kosong di baris terakhir.
 */
export const NOT_FOUND_BUTTON_LIMIT = 6;

/**
 * Payload untuk kasus "command tidak ada".
 *
 * Dua hal yang penting:
 * 1. Bahasa Indonesia. String Inggris lama ("I'm sorry, but I don't have
 *    that command") tidak cocok dengan sisa bot yang berbahasa Indonesia.
 * 2. Tombol quick_reply yang isinya LANGSUNG command lengkap dengan prefix,
 *    jadi user tinggal ketuk — tidak perlu mengetik ulang. Seseorang yang
 *    salah ketik biasanya tidak ingin mengetik ulang.
 *
 * @param {string} input command yang diketik user
 * @param {string[]} commands daftar command yang tersedia
 * @param {{prefix?: string, pushName?: string, maxSuggestions?: number}} [opts]
 */
export function buildNotFoundPayload(input, commands, opts = {}) {
  const prefix = opts.prefix || ".";
  const pushName = String(opts.pushName || "").trim();
  const safeInput = String(input || "").replace(/[`*_]/g, "");

  const pool = Array.isArray(commands) ? commands : [];
  const suggestions = pool.length
    ? findSimilarCommands(String(input || ""), pool, {
        maxResults: opts.maxSuggestions ?? NOT_FOUND_BUTTON_LIMIT,
        minSimilarity: 0.2,
        maxDistance: 6,
      })
    : [];

  // Kalau ketemu sedikit (mis. satu typo yang jauh), tombolnya kekurangan
  // pilihan. Tambahkan command umum dari pool yang sama supaya user tetap
  // punya jalan keluar tanpa perlu mengetik.
  const FALLBACKS = ["menu", "help"];
  for (const name of FALLBACKS) {
    if (suggestions.length >= 2) break;
    if (!pool.includes(name)) continue;
    if (suggestions.some((s) => s.command === name)) continue;
    suggestions.push({
      command: name,
      similarity: 0.3,
      distance: name.length,
      type: "fallback",
      emoji: "💡",
      reason: "menu umum",
    });
  }

  // WA native_flow max 3 baris × 2 kolom. Bulatkan ke bawah supaya genap.
  const buttonCount = Math.min(
    suggestions.length,
    Math.floor(
      Math.min(opts.maxSuggestions ?? NOT_FOUND_BUTTON_LIMIT, NOT_FOUND_BUTTON_LIMIT) / 2,
    ) * 2,
  );

  const buttons = suggestions.slice(0, buttonCount).map((s) => ({
    name: "quick_reply",
    buttonParamsJson: JSON.stringify({
      display_text: `${prefix}${s.command}`,
      id: `${prefix}${s.command}`,
    }),
  }));

  const sapaan = pushName ? `Halo kak *${pushName}*! 👋` : "Halo kak! 👋";

  let text = `${sapaan}\n\n`;
  text += `Command \`${prefix}${safeInput}\` tidak ditemukan 🤔\n`;
  text += `Mungkin ada salah ketik atau command-nya memang belum tersedia di bot ini.\n\n`;

  if (buttons.length > 0) {
    text += `Mungkin yang kakak maksud:\n`;
    for (const b of buttons) {
      const id = JSON.parse(b.buttonParamsJson).id;
      text += `• \`${id}\`\n`;
    }
    text += `\n_Tap salah satu di bawah, kak — nggak perlu ketik ulang._ 👆\n\n`;
  }

  text += `Kalau memang belum ada, coba cek daftar lengkapnya:\n`;
  text += `• \`${prefix}menu\` — daftar semua menu\n`;
  text += `• \`${prefix}help\` — bantuan\n`;

  return { text, buttons, suggestions };
}
