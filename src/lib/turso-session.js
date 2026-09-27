import { BufferJSON } from 'onigis';
import { getTursoClient } from './turso.js';

const keysCache = new Map();
const KEYS_CACHE_CAP = 1000;

function trimLocalCache(local, cap) {
  if (local.size <= cap) return;
  const excess = local.size - cap;
  for (let i = 0; i < excess; i++) {
    local.delete(local.keys().next().value);
  }
}

// reports remote problems to the bot logger instead of a bare console.warn,
// which index.js filters/noises differently from colors.logger
let remoteWarn = null;
export function setRemoteSessionLogger(fn) {
  remoteWarn = typeof fn === 'function' ? fn : null;
}
function warnRemote(msg) {
  if (remoteWarn) remoteWarn(msg);
  else console.warn(`[turso-session] ${msg}`);
}

async function loadState(scope) {
  try {
  const client = getTursoClient();
  if (!client) return null;
  // load creds
  const credsRs = await client.execute({
    sql: 'SELECT creds FROM session_creds WHERE scope = ?',
    args: [scope],
  });
  if (!credsRs.rows || credsRs.rows.length === 0) {
    // WAJIB null, bukan initAuthCreds(): state fabricated bikin connection.js
    // skip fallback file lokal, jadi sesi WA hanya hidup di Turso.
    return null;
  }
  const creds = JSON.parse(credsRs.rows[0].creds, BufferJSON.reviver);
  // keys are loaded lazily on demand
  return {
    creds,
    keys: {
      get: async (type, ids) => {
        if (!Array.isArray(ids) || ids.length === 0) return {};
        const cacheKey = `${scope}:${type}`;
        if (!keysCache.has(cacheKey)) keysCache.set(cacheKey, new Map());
        const local = keysCache.get(cacheKey);
        const missing = ids.filter(id => !local.has(id));
        if (missing.length > 0) {
          const placeholders = missing.map(() => '?').join(',');
          const rs = await client.execute({
            sql: `SELECT id, data FROM session_keys WHERE scope = ? AND category = ? AND id IN (${placeholders})`,
            args: [scope, type, ...missing],
          });
          for (const row of rs.rows) {
            local.set(row.id, JSON.parse(row.data, BufferJSON.reviver));
          }
          trimLocalCache(local, KEYS_CACHE_CAP);
        }
        const result = {};
        for (const id of ids) {
          if (local.has(id)) result[id] = local.get(id);
        }
        return result;
      },
      set: async (data) => {
        const client = getTursoClient();
        if (!client) return;
        const statements = [];
        for (const [type, entries] of Object.entries(data)) {
          for (const [id, value] of Object.entries(entries)) {
            const cacheKey = `${scope}:${type}`;
            if (!keysCache.has(cacheKey)) keysCache.set(cacheKey, new Map());
            const local = keysCache.get(cacheKey);
            local.delete(id);
            local.set(id, value);
            trimLocalCache(local, KEYS_CACHE_CAP);
            statements.push({
              sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(scope, category, id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at',
              args: [scope, type, id, JSON.stringify(value, BufferJSON.replacer), Date.now()],
            });
          }
        }
        if (statements.length > 0) {
          try {
            await client.batch(statements, "write");
          } catch (e) {
            console.warn('[turso-session] batch failed, falling back to sequential:', e.message);
            for (const stmt of statements) {
              await client.execute(stmt);
            }
          }
        }
      },
      getMany: async (type) => {
        const rs = await client.execute({
          sql: 'SELECT id, data FROM session_keys WHERE scope = ? AND category = ?',
          args: [scope, type],
        });
        const result = {};
        for (const row of rs.rows) {
          result[row.id] = JSON.parse(row.data, BufferJSON.reviver);
        }
        return result;
      },
    },
  };
  } catch (e) {
    warnRemote(`load failed, falling back: ${e.message}`);
    return null;
  }
}

// Turso tidak punya mutex bawaan: dua UPSERT yang tumpang tindih bisa selesai
// terbalik, dan UPSERT yang masih in-flight bisa	create ulang creds yang
// baru saja dihapus. Serialisasi per-scope + delete menunggu antrean selesai.
const scopeQueues = new Map();

function enqueue(scope, task) {
  const prev = scopeQueues.get(scope) || Promise.resolve();
  const next = prev.then(task, task).catch(() => {});
  scopeQueues.set(scope, next);
  return next;
}

async function saveCreds(scope, creds) {
  const snapshot = JSON.stringify(creds, BufferJSON.replacer);
  return enqueue(scope, async () => {
    try {
      const client = getTursoClient();
      if (!client) return;
      await client.execute({
        sql: 'INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?) ON CONFLICT(scope) DO UPDATE SET creds = excluded.creds, updated_at = excluded.updated_at',
        args: [scope, snapshot, Date.now()],
      });
    } catch (e) {
      warnRemote(`save failed: ${e.message}`);
    }
  });
}

async function deleteTursoSession(scope) {
  for (const k of keysCache.keys()) {
    if (k.startsWith(scope + ':')) keysCache.delete(k);
  }
  // tunggu antrean write scope ini selesai dulu, kalau tidak UPSERT yang
  // masih in-flight bisa create ulang creds yang baru saja dihapus
  return enqueue(scope, async () => {
    const client = getTursoClient();
    if (!client) return;
    try {
      await client.execute({ sql: 'DELETE FROM session_creds WHERE scope = ?', args: [scope] });
      await client.execute({ sql: 'DELETE FROM session_keys WHERE scope = ?', args: [scope] });
    } catch (e) { console.warn('[turso-session] delete failed:', e.message); }
  });
}

async function useTursoAuthState(scope = 'main') {
  const state = await loadState(scope);
  if (!state) {
    // Tidak ada sesi remote (tabel kosong / token mati / client null).
    // Caller wajib fallback ke file lokal.
    return { state: null, saveCreds: () => {} };
  }
  return {
    state,
    saveCreds: () => saveCreds(scope, state.creds),
  };
}

// File lokal = source of truth yang durable, Turso hanya mirror best-effort.
// Token Turso mati = warning, bukan logout.
async function useDurableAuthState(scope, folder) {
  const { useMultiFileAuthState, initAuthCreds } = await import('onigis');
  if (!folder) throw new Error('useDurableAuthState butuh folder lokal');

  const local = await useMultiFileAuthState(folder);
  let creds = local.state.creds;
  const localKeys = local.state.keys;

  // sisa pairing gagal (registered:false tapi me/pairingCode terisi) bukan sesi sah
  const looksPaired = creds.registered === true || Boolean(creds.account);

  // belum pernah paired di disk -> coba pulihkan dari mirror Turso
  let remote = null;
  if (!looksPaired) {
    remote = await loadState(scope);
    if (remote?.creds) {
      const r = remote.creds;
      if (r.registered === true || Boolean(r.account)) {
        creds = r;
        warnRemote('sesi dipulihkan dari Turso ke storage lokal');
      }
    }
  }

  const remoteKeys = remote?.state?.keys ?? remote?.keys ?? null;

  const keys = {
    get: async (type, ids) => {
      const out = {};
      if (!Array.isArray(ids) || ids.length === 0) return out;
      // lokal dulu (durable), Turso hanya untuk key yang belum ada di disk
      const localGot = await localKeys.get(type, ids);
      const missing = ids.filter((id) => !localGot[id]);
      if (missing.length && remoteKeys) {
        try {
          const remoteGot = await remoteKeys.get(type, missing);
          for (const id of missing) if (remoteGot[id]) out[id] = remoteGot[id];
        } catch (e) {
          warnRemote(`keys.get(${type}) gagal: ${e.message}`);
        }
      }
      for (const id of ids) if (localGot[id]) out[id] = localGot[id];
      return out;
    },
    set: async (data) => {
      await localKeys.set(data);
      if (!remoteKeys) return;
      try {
        await remoteKeys.set(data);
      } catch (e) {
        warnRemote(`keys.set mirror gagal: ${e.message}`);
      }
    },
    getMany: async (type) => {
      const localAll = await localKeys.getMany(type);
      if (!remoteKeys) return localAll;
      try {
        return { ...(await remoteKeys.getMany(type)), ...localAll };
      } catch {
        return localAll;
      }
    },
  };

  let lastLocalError = null;
  const saveCredsHybrid = async () => {
    try {
      await local.saveCreds();
      lastLocalError = null;
    } catch (e) {
      // ini yang fatal kalau sampai sini: tanpa file lokal, sesi hilang
      lastLocalError = e;
      warnRemote(`PENYIMPANAN LOKAL GAGAL: ${e.message}`);
    }
    await saveCreds(scope, creds);
    if (lastLocalError) throw lastLocalError;
  };

  return {
    state: { creds, keys },
    saveCreds: saveCredsHybrid,
    initAuthCreds,
  };
}

export {
  useTursoAuthState,
  useDurableAuthState,
  loadState,
  saveCreds,
  deleteTursoSession,
  trimLocalCache,
};
