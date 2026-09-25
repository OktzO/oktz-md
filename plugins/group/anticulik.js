import { getDatabase } from "../../src/lib/database.js";
import { saluranCtx } from "../../src/lib/context.js";
import { isSameParticipant } from "../../src/lib/lid.js";

const pluginConfig = {
  name: "anticulik",
  alias: ["antikidnap", "antiileng", "anticulikgc"],
  category: "group",
  description: "Bot otomatis keluar grup jika ditambah tanpa izin",
  usage: ".anticulik on/off",
  example: ".anticulik on",
  isOwner: true,
  isPremium: false,
  isGroup: true,
  isPrivate: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const db = getDatabase();
  const option = m.text?.toLowerCase()?.trim();

  if (!option) {
    const status = db.setting("anticulik") || "off";

    return m.reply(
      `🛡️ *Anti Culik*\n\n` +
        `Bot akan otomatis keluar dari grup jika ditambah oleh orang yang tidak dikenal tanpa izin.\n\n` +
        `*STATUS:*\n` +
        `> Mode: *${status === "on" ? "Aktif ✅" : "Nonaktif ❌"}*\n\n` +
        `*PENGGUNAAN:*\n` +
        `> *${m.prefix}anticulik on* — Aktifkan\n` +
        `> *${m.prefix}anticulik off* — Nonaktifkan\n\n` +
        `_Jika aktif, bot hanya bisa join via *${m.prefix}join* atau ditambah oleh owner_`
    );
  }

  if (option === "on") {
    db.setting("anticulik", "on");
    const ctx = saluranCtx();
    return m.reply(
      `🛡️ *Anti Culik Aktif*\n\n` +
        `> Bot akan keluar otomatis jika ditambah tanpa izin\n` +
        `> Satu-satunya cara bot bisa join: *${m.prefix}join* oleh owner\n\n` +
        `_Member yang menambah bot akan diberi peringatan_`,
      { contextInfo: ctx }
    );
  }

  if (option === "off") {
    db.setting("anticulik", "off");
    return m.reply(
      `🛡️ *Anti Culik Nonaktif*\n\n` +
        `> Bot tidak akan keluar otomatis jika ditambah ke grup\n` +
        `> Siapapun bisa menambahkan bot ke grup`
    );
  }

  return m.reply(
    `❌ *Opsi Tidak Valid*\n\n> Gunakan *${m.prefix}anticulik on* atau *${m.prefix}anticulik off*`
  );
}

async function handleAntiCulik(event, sock, db) {
  if (event.action !== "add") return false;

  const botNumber =
    sock.user?.id?.split(":")[0] || sock.user?.id?.split("@")[0];

  const isBotAdded = (event.participants || []).some((p) => {
    const rJid = typeof p === "object" && p !== null ? p.phoneNumber || p.id : p;
    if (typeof rJid !== "string") return false;
    const pNum = rJid.split("@")[0].split(":")[0];
    return (
      pNum === botNumber ||
      isSameParticipant(rJid, sock.user?.id) ||
      isSameParticipant(rJid, sock.user?.lid) ||
      rJid.includes(botNumber)
    );
  });

  if (!isBotAdded) return false;

  const anticulikStatus = db.setting("anticulik") || "off";
  if (anticulikStatus !== "on") return false;

  const inviter = event.author || "";
  const ownerNumbers = (global.owner || []).map((o) =>
    typeof o === "string" ? o.split("@")[0] : o
  );

  // event.author di grup LID berupa LID, jadi perbandingan digits/ string
  // saja membuat owner yang mengundang bot dianggap orang asing -> bot keluar
  // sendiri dari grupnya pemiliknya.
  const isOwnerInviter =
    isSameParticipant(inviter, sock.user?.id) ||
    isSameParticipant(inviter, sock.user?.lid) ||
    ownerNumbers.some((o) => isSameParticipant(String(o), inviter));

  if (isOwnerInviter) return false;

  const inviterMention = inviter
    ? `@${inviter.split("@")[0]}`
    : "seseorang";

  await sock.sendMessage(event.id, {
    text:
      `🛡️ *Anti Culik*\n\n` +
      `Minimal izin dulu ya bang, jangan asal culik 🗿\n\n` +
      `> Bot ditambah oleh ${inviterMention} tanpa izin\n` +
      `> Bot akan keluar dari grup ini\n\n` +
      `_Hubungi owner untuk menambahkan bot dengan cara yang benar_`,
    contextInfo: saluranCtx(),
    mentionedJid: inviter ? [inviter] : [],
  });

  await new Promise((r) => setTimeout(r, 2000));
  await sock.groupLeave(event.id);
  return true;
}

export { pluginConfig as config, handler, handleAntiCulik };
