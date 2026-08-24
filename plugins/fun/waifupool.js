import { getDatabase } from "../../src/lib/ourin-database.js";
import { searchPool } from "../../data/waifu/index.js";

const pluginConfig = {
  name: ["waifupool", "poolwaifu"],
  alias: [],
  category: "fun",
  description: "Jelajahi pool waifu — cari nama/franchise/tier!",
  usage: ".waifupool [nama|tier|franchise]",
  example: ".waifupool rare",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

const pages = new Map(); // `${jid}:${query}` -> page index

function tierEmoji(t) {
  return { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" }[t] || "";
}

async function handler(m, { sock }) {
  const db = getDatabase();
  const args = m.args || [];
  const nav = ["prev", "next"].includes((args[0] || "").toLowerCase()) ? args[0].toLowerCase() : null;
  const query = (nav ? args.slice(1) : args).join(" ").trim();
  const key = `${m.sender}:${query.toLowerCase()}`;
  let page = Math.max(0, pages.get(key) || 0);
  if (nav === "next") page++;
  if (nav === "prev") page--;
  page = Math.max(0, page);
  const PAGE_SIZE = 10;

  const pool = searchPool(query);
  if (!pool.length) {
    pages.set(key, 0);
    return m.reply(`🔍 Tidak ada waifu cocok dengan *"${query}"*. Coba nama/tier/franchise lain.`);
  }

  const totalPages = Math.max(1, Math.ceil(pool.length / PAGE_SIZE));
  const cur = Math.min(page, totalPages - 1);
  const slice = pool.slice(cur * PAGE_SIZE, (cur + 1) * PAGE_SIZE);

  let text = `📖 *WAIFU POOL* 📖\n\n`;
  text += query ? `🔍 Pencarian: *"${query}"*\n` : "";
  text += `📊 Total: *${pool.length}* waifu | Halaman *${cur + 1}/${totalPages}*\n\n`;
  slice.forEach((w, i) => {
    text += `${cur * PAGE_SIZE + i + 1}. ${tierEmoji(w.tier)} *${w.name}* — ${w.tier}\n`;
    text += `   🏷️ ${w.series} | 🎭 ${w.personality}\n`;
  });

  const buttons = [];
  if (cur > 0) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "⬅️", id: `${m.prefix}waifupool prev ${query}` }) });
  if (cur < totalPages - 1) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "➡️", id: `${m.prefix}waifupool next ${query}` }) });
  if (buttons.length) text += `\nGunakan tombol untuk berpindah halaman.`;

  pages.set(key, cur);
  return m.reply(text, null, buttons.length ? { buttons } : undefined);
}

export { pluginConfig as config, handler };
