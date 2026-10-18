// Penyimpanan pesan berbasis id (bukan per-chat FIFO) untuk keperluan:
//   - `getMessage` library (retry receipt → resend pesan kita, supaya peer
//     bisa decrypt ulang tanpa "Waiting for this message").
//   - `loadMessage` (quoted message pushName, plugin qwa.js).
//
// Desain lama: Map<remoteJid, Map<id, msg>> dengan cap 30 pesan/chat dan
// 300 chat (FIFO per chat) → pesan lama ter-evict walau ID-nya masih
// dibutuhkan untuk retry. Juga lookup gagal kalau bentuk JID berubah
// (LID vs PN) meskipun id-nya sama.
//
// Desain baru: satu indeks global berdasarkan id pesan (id WA unik per
// stanza), dengan batas total yang jelas dan eviction FIFO yang deterministik.
// Lookup tidak bergantung pada bentuk JID.

const DEFAULT_MAX_MESSAGES = 2000;
const DEFAULT_MAX_CONTACTS = 500;

function trimContacts(contacts) {
  const keys = Object.keys(contacts);
  if (keys.length <= DEFAULT_MAX_CONTACTS) return;
  const excess = keys.length - DEFAULT_MAX_CONTACTS;
  for (let i = 0; i < excess; i++) delete contacts[keys[i]];
}

import { cacheLidJid } from "./lid.js";

export function createMessageStore({
  maxMessages = DEFAULT_MAX_MESSAGES,
  maxContacts = DEFAULT_MAX_CONTACTS,
} = {}) {
  // id -> { jid, msg, storedAt }
  const byId = new Map();
  const contacts = {};
  const chats = new Map();

  function storeMessage(msg) {
    const id = msg?.key?.id;
    if (!id) return;
    // Map mempertahankan insertion order → FIFO eviction deterministik.
    if (byId.has(id)) byId.delete(id);
    byId.set(id, {
      jid: msg.key.remoteJid,
      msg,
      storedAt: Date.now(),
    });
    if (byId.size > maxMessages) {
      const firstKey = byId.keys().next().value;
      if (firstKey !== undefined) byId.delete(firstKey);
    }
  }

  function loadMessage(jid, id) {
    if (!id) return undefined;
    const entry = byId.get(id);
    if (!entry) return undefined;
    // id WA unik per stanza; toleran terhadap perbedaan bentuk JID.
    return entry.msg;
  }

  function getMessageContent(id) {
    const entry = byId.get(id);
    return entry?.msg?.message || undefined;
  }

  function bind(ev) {
    ev.on("messages.upsert", ({ messages }) => {
      for (const msg of messages || []) {
        storeMessage(msg);

        const from = msg.key?.remoteJid || "";
        if (from.endsWith("@s.whatsapp.net")) {
          const pushName = msg.pushName;
          if (pushName && pushName !== "undefined") {
            contacts[from] = { ...(contacts[from] || {}), notify: pushName };
            trimContacts(contacts);
          }
        }

        // Isi cache LID ↔ PN dari alt JID yang dibawa stanza.
        const lid = msg.key?.remoteJid;
        const alt = msg.key?.remoteJidAlt;
        if (lid && alt) cacheLidJid(lid, alt);
        const p = msg.key?.participant;
        const pa = msg.key?.participantAlt;
        if (p && pa) cacheLidJid(p, pa);
      }
    });

    ev.on("chats.upsert", (chatList) => {
      for (const chat of chatList || []) {
        if (chat?.id) {
          if (chats.size >= 400 && !chats.has(chat.id)) {
            const firstKey = chats.keys().next().value;
            if (firstKey) chats.delete(firstKey);
          }
          chats.set(chat.id, chat);
        }
      }
    });

    ev.on("contacts.upsert", (contacts) => {
      for (const contact of contacts || []) {
        if (contact?.id) {
          contacts[contact.id] = { ...contacts[contact.id], ...contact };
          trimContacts(contacts);
        }
      }
    });
  }

  return {
    bind,
    loadMessage,
    getMessageContent,
    contacts,
    chats,
    size: () => byId.size,
  };
}
