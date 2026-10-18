// Pengambilan metadata grup yang dibatasi waktu tunggu, agar pesan pertama di
// grup baru (atau grup yang cachenya kosong) tidak memblokir handler sampai
// timeout query default (60s) dan menahan seluruh pool pesan.

import { sweepGroupMetadataCache } from "./lid.js";
const DEFAULT_WAIT_MS = 3000;
const FETCH_TIMEOUT_MS = 15000;

const inflight = new Map();

export async function getGroupMetadata(sock, jid, { waitMs = DEFAULT_WAIT_MS } = {}) {
  if (!jid || !sock) return null;

  // Cache global (dipakai serialize.js dan chats.upsert prefetch).
  const cache = global.groupMetadataCache;
  const cached = cache?.get?.(jid);
  if (cached?.data) return cached.data;

  let promise = inflight.get(jid);
  if (!promise) {
    promise = (async () => {
      try {
        const fresh = await Promise.race([
          sock.groupMetadata(jid),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("groupMetadata timeout")), FETCH_TIMEOUT_MS),
          ),
        ]);
        if (fresh && cache?.set) {
          sweepGroupMetadataCache(cache);
          cache.set(jid, { data: fresh, timestamp: Date.now() });
        }
        return fresh;
      } catch {
        return null;
      } finally {
        // biarkan inflight hidup sampai promise selesai, lalu hapus
      }
    })();
    const tracked = promise.finally(() => {
      if (inflight.get(jid) === tracked) inflight.delete(jid);
    });
    inflight.set(jid, tracked);
    promise = tracked;
  }

  return Promise.race([
    promise,
    new Promise((resolve) => {
      const t = setTimeout(() => resolve(null), waitMs);
      if (t && typeof t.unref === "function") t.unref();
      promise.then(() => clearTimeout(t), () => clearTimeout(t));
    }),
  ]);
}

export const GROUP_META_WAIT_MS = DEFAULT_WAIT_MS;
