import { getDatabase } from "../../src/lib/ourin-database.js";
import { albumStats } from "../../src/lib/ourin-husbu.js";

const pluginConfig = {
  name: ["husbualbum", "albumhusbu"],
  alias: [],
  category: "fun",
  description: "Lihat riwayat gacha husbu & statistik luck kamu!",
  usage: ".husbualbum",
  example: ".husbualbum",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const history = user.husbuHistory || [];
  if (!history.length) {
    return m.reply(`📭 *Album kosong!*\nKamu belum pernah gacha husbu. Ketik *${m.prefix}gachahusbu* untuk mulai!`);
  }

  const stats = albumStats(history, user.husbuStats || {});
  const tierEmoji = { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" };
  const bar = (count) => {
    const pct = stats.total ? Math.round((count / stats.total) * 20) : 0;
    return "█".repeat(pct) + "░".repeat(20 - pct);
  };

  let text = `📚 *ALBUM HUSBU & LUCK STATS* 📚\n\n` +
    `🎲 *Total Gacha:* ${stats.total}\n` +
    `🍀 *Luck Score:* ${stats.luck} ${stats.luck >= 1 ? "🔥" : "😅"}\n\n` +
    `*Distribusi Tier:*\n` +
    `🟢 Common     ${String(stats.byTier.Common || 0).padStart(3)} ${bar(stats.byTier.Common || 0)}\n` +
    `🔵 Rare       ${String(stats.byTier.Rare || 0).padStart(3)} ${bar(stats.byTier.Rare || 0)}\n` +
    `🟣 Epic       ${String(stats.byTier.Epic || 0).padStart(3)} ${bar(stats.byTier.Epic || 0)}\n` +
    `🟡 Legendary  ${String(stats.byTier.Legendary || 0).padStart(3)} ${bar(stats.byTier.Legendary || 0)}\n` +
    `🔴 Mythic     ${String(stats.byTier.Mythic || 0).padStart(3)} ${bar(stats.byTier.Mythic || 0)}\n\n`;

  if (stats.rarest) text += `🏆 *Pull Paling Langka:* ${tierEmoji[stats.rarest.tier] || ""} ${stats.rarest.name} (${stats.rarest.tier})\n`;
  if (stats.pityCounter > 0) text += `🎯 *Pity aktif:* ${stats.pityCounter}/20 roll tanpa Epic+\n`;
  text += `💍 *Pernikahan:* ${stats.marriedCount || 0}\n\n*10 Pull Terakhir:*\n`;

  stats.last10.forEach((h, i) => {
    text += `${i + 1}. ${tierEmoji[h.tier] || ""} *${h.name}* — ${h.tier}\n`;
  });

  return m.reply(text);
}

export { pluginConfig as config, handler };
