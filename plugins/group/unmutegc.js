import { getDatabase } from "../../src/lib/database.js";
import { saluranCtx } from "../../src/lib/context.js";

const pluginConfig = {
  name: "unmutegc",
  alias: ["unmutegrup", "unmutebot", "unblockbot", "unlockbot"],
  category: "group",
  description: "Buka blokir command bot untuk member di grup",
  usage: ".unmutegc",
  example: ".unmutegc",
  isOwner: false,
  isPremium: false,
  isGroup: true,
  isPrivate: false,
  isAdmin: true,
  isBotAdmin: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const db = getDatabase();
  const groupData = db.getGroup(m.chat) || {};

  if (!groupData.mutegc) {
    return m.reply(
      `🔊 *Mute GC Tidak Aktif*\n\n` +
        `> Member sudah bisa menggunakan command bot di grup ini`
    );
  }

  db.setGroup(m.chat, { mutegc: false });
  const ctx = saluranCtx();
  const groupName = m.groupMetadata?.subject || "grup ini";

  return m.reply(
    `🔊 *Mute GC Nonaktif*\n\n` +
      `> Grup: *${groupName}*\n` +
      `> Member sekarang bisa menggunakan command bot lagi\n\n` +
      `_Ketik *${m.prefix}mutegc* untuk memblokir kembali_`,
    { contextInfo: ctx }
  );
}

export { pluginConfig as config, handler };
