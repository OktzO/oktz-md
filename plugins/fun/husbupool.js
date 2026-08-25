import { searchPool } from "../../data/husbu/index.js";

const pluginConfig = {
  name: ["husbupool", "poolhusbu"],
  alias: [],
  category: "fun",
  description: "Jelajahi pool husbu — cari nama/franchise/tier!",
  usage: ".husbupool [nama|tier|franchise]",
  example: ".husbupool legendary",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

const pages = new Map();

function trimPages(now = Date.now()) {
  if (pages.size > 500) {
    const cutoff = now - 30 * 60 * 1000;
    for (const [k, v] of pages) {
      if (v.ts < cutoff) pages.delete(k);
    }
  }
}

function tierEmoji(t) {
  return { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" }[t] || "";
}

async function handler(m, { sock }) {
  const args = m.args || [];
  const first = (args[0] || "").toLowerCase();
  const nav = (first === "prev" || first === "next") && args.length > 1 ? first : null;
  const query = (nav ? args.slice(1) : args).join(" ").trim();
  const key = `${m.sender}:${query.toLowerCase()}`;
  let page = Math.max(0, pages.get(key)?.page || 0);
  if (nav === "next") page++;
  if (nav === "prev") page--;
  page = Math.max(0, page);
  const PAGE_SIZE = 10;

  const pool = searchPool(query);
  if (!pool.length) {
    pages.set(key, { page: 0, ts: Date.now() });
    trimPages();
    return m.reply(`🔍 Tidak ada husbu cocok dengan *"${query}"*. Coba nama/tier/franchise lain.`);
  }

  const totalPages = Math.max(1, Math.ceil(pool.length / PAGE_SIZE));
  const cur = Math.min(page, totalPages - 1);
  const slice = pool.slice(cur * PAGE_SIZE, (cur + 1) * PAGE_SIZE);

  let text = `📖 *HUSBU POOL* 📖\n\n`;
  text += query ? `🔍 Pencarian: *"${query}"*\n` : "";
  text += `📊 Total: *${pool.length}* husbu | Halaman *${cur + 1}/${totalPages}*\n\n`;
  slice.forEach((h, i) => {
    text += `${cur * PAGE_SIZE + i + 1}. ${tierEmoji(h.tier)} *${h.name}* — ${h.tier}\n`;
    text += `   🏷️ ${h.series} | 🎭 ${h.personality}\n`;
  });

  const buttons = [];
  if (cur > 0) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "⬅️", id: `${m.prefix}husbupool prev ${query}` }) });
  if (cur < totalPages - 1) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "➡️", id: `${m.prefix}husbupool next ${query}` }) });
  if (buttons.length) text += `\nGunakan tombol untuk berpindah halaman.`;

  pages.set(key, { page: cur, ts: Date.now() });
  trimPages();
  return m.reply(text, null, buttons.length ? { buttons } : undefined);
}

export { pluginConfig as config, handler, pages };
