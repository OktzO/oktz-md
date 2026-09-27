import { BufferJSON } from 'onigis';
import fs from 'fs';
import path from 'path';
import { getTursoClient } from './turso.js';

const keysCache = new Map();
const KEYS_CACHE_CAP = 1000;

// Berapa file rusak yang boleh menumpuk per tipe sebelum yang paling lama
// dibuang. Tanpa ini, karantina berulang akan mengisi folder sesi tanpa batas
// pada bot yang corruption-nya terus-menerus. Isinya hanya dibuang
// SETELAH operator punya kesempatan mengambilnya; tidak ada yang hilang diam.
const CORRUPT_KEY_KEEP = 5;

// Jumlah karantina creds yang disimpan sebelum yang tertua dibuang. Bot yang
// boot gagal berulang tidak boleh menabrak batas inode, tapi file lama
// dibuang hanya SETELAH operator sempat mengambilnya.
const BROKEN_CREDS_KEEP = 5;

function pruneBrokenCredsBackups(storageRoot, credsPath, report) {
  const dir = path.dirname(credsPath);
  const base = path.basename(credsPath);
  try {
    const backups = fs
      .readdirSync(dir)
      .filter((f) => f.startsWith(`${base}.broken-`))
      .sort();
    while (backups.length > BROKEN_CREDS_KEEP) {
      const victim = backups.shift();
      try {
        fs.rmSync(path.join(dir, victim), { force: true });
        report?.(`karantina creds lama dibuang: ${victim}`);
      } catch { }
    }
  } catch { }
  void storageRoot;
}

// Pindahkan creds.json yang tidak bisa dipakai ke <file>.broken-<ts>.
// RENAME, bukan unlink: isi asli utuh untuk dipulihkan operator dan tidak
// ada satu pun file yang dihapus. Identitas di dalam file itu tidak berlaku
// lagi, tapi membawanya ke log lebih berguna daripada diam-diam hilang.
function quarantineBrokenCreds(credsPath, report) {
  const dest = `${credsPath}.broken-${Date.now()}`;
  try {
    fs.renameSync(credsPath, dest);
  } catch (e) {
    report?.(`gagal memindahkan creds.json ke karantina: ${e.message}`);
    return null;
  }
  report?.(
    `creds.json tidak bisa dipakai -> dipindah ke ${path.basename(dest)} ` +
      `(isi file TIDAK dihapus, bisa dipulihkan dari sana). ` +
      `Bot mulai dengan sesi baru: pair ulang / scan QR lagi. ` +
      `Kalau ini terjadi berulang, cek izin tulis folder storage.`,
  );
  return dest;
}

// Klasifikasi kegagalan load auth-state.
//
// "corrupt" hanya diberikan kalau kita BISA membuktikan file-nya ada dan
// errornya memang tentang file itu. Semua else -- EACCES, folder bukan
// direktori, ENOSPC, ENOENT yang aneh -- dikembalikan sebagai "other" supaya
// pemanggil TIDAK mengarang pemulihan. Mengarantaina file karena masalah
// yang lain seperti ini justru menghapus bukti masalah aslinya.
function classifyAuthStateFailure(error, { credsExists, credsPath } = {}) {
  const message = String(error?.message || error || "");
  if (!credsExists) return { kind: "other", reason: "creds.json tidak ada" };
  if (message.includes(credsPath) || message.includes("creds.json")) {
    return { kind: "corrupt", reason: message };
  }
  return { kind: "other", reason: message };
}

/**
 * Muat auth state, dan pulihkan sendiri kalau creds.json ada tapi rusak.
 *
 * =onigis menolak creds.json yang HADIR tapi tidak bisa dipakai (`me: null`,
 * array di level atas, `"hello"`, JSON terpotong) alih-alih diam-diam
 * memalsukan identitas baru. Itu fail-closed yang benar. Tapi startConnection
 * dulu dipanggil tanpa guard di dalam main(), dan main().catch melakukan
 * process.exit(1) -- jadi setiap boot gagal sampai operator menghapus
 * storage/session dengan tangan. Repo ini sudah punya storage/session.broken-*,
 * jadi itu benar-benar pernah terjadi.
 *
 * Karantina di level FILE, bukan folder: pre-key, app-state, dan session-*
 * milik identitas itu, tapi membuang folder berarti identifier baru DAN
 * memicu rate-overlimit, sedangkan mengganti identitas tanpa menghapus apa
 * pun tidak meninggalkan jejak yang tidak bisa dipulihkan. Yang dibuang
 * hanya file creds.json itu sendiri, yang memang sudah tidak berlaku.
 *
 * Creds RUSAK dan creds BELUM PAIR sengaja tidak dicampur:
 * - rusak: file ada tapi JSON-nya tidak bisa dipakai -> karantina + sesi baru
 * - belum pair: file sah, `me` tidak ada, `registered: false` -> kondisi
 *   NORMAL hasil pairing yang belum selesai, dibiarkan apa adanya tanpa report
 *   karantina, karena inilah yang terjadi pada setiap pairing baru.
 * Klasifikasi hanya melaporkan "corrupt" kalau file-nya terbukti ADA dan
 * errornya memang tentang file itu.
 *
 * @returns {Promise<{state: Object, saveCreds: Function, recovered: boolean}>}
 */
async function loadAuthStateWithRecovery(sessionPath, report) {
  const credsPath = path.join(sessionPath, "creds.json");
  const load = async () => {
    const { useMultiFileAuthState } = await import("onigis");
    return useMultiFileAuthState(sessionPath);
  };

  try {
    const res = await load();
    return { state: res.state, saveCreds: res.saveCreds, recovered: false };
  } catch (error) {
    const credsExists = (() => {
      try {
        return fs.existsSync(credsPath);
      } catch {
        return false;
      }
    })();

    const kind = classifyAuthStateFailure(error, { credsExists, credsPath });
    if (kind.kind !== "corrupt") {
      // Bukan masalah creds.json: mengarantaina apa pun di sini akan
      // menghapus bukti masalah yang sebenarnya. Biarkan error asli naik
      // supaya pesan yang muncul jujur.
      throw error;
    }

    report?.(
      `gagal memuat sesi dari ${path.basename(sessionPath)}: ${kind.reason}`,
    );
    const dest = quarantineBrokenCreds(credsPath, report);
    if (!dest) {
      // Tidak bisa dipindah: jangan ulangi tanpa batas, biarkan error asli.
      throw error;
    }
    pruneBrokenCredsBackups(path.dirname(sessionPath), credsPath, report);

    const res = await load();
    return { state: res.state, saveCreds: res.saveCreds, recovered: true };
  }
}

function trimLocalCache(local, cap) {
  if (local.size <= cap) return;
  const excess = local.size - cap;
  for (let i = 0; i < excess; i++) {
    local.delete(local.keys().next().value);
  }
}

// File auth-state yang rusak tidak bisa "diperbaiki" di tempat: isinya hilang
// dan tidak ada salinan. Yang bisa dilakukan hanya memindahkannya supaya
// baca berikutnya jadi miss -- dan miss pre-key/sender-key adalah jalur
// protokol NORMAL (sesi di-re-negotiate), bukan error. Fail-loud sendirian
// tidak cukup karena file rusak tidak pernah hilang dari disk, jadi hasilnya
// outage permanen yang hanya bisa dilihat dari log.
//
// RENAME, bukan unlink: isi asli utuh di file .broken-<ts> untuk dipulihkan
// operator, dan tidak ada satu pun file yang dihapus.
//
// Corrupt != absent. File yang memang tidak ada (ENOENT) adalah "belum ada
// nilai" yang wajar dan TIDAK boleh dilaporkan; hanya file yang ADA tapi tidak
// bisa di-parse yang dikarantina.
function quarantineCorruptKeyFiles(sessionPath, type, report) {
  let files;
  try {
    files = fs.readdirSync(sessionPath);
  } catch {
    return [];
  }
  const prefix = `${String(type).replace(/\//g, '__').replace(/:/g, '-')}-`;
  const moved = [];
  for (const name of files) {
    if (!name.startsWith(prefix) || !name.endsWith('.json')) continue;
    if (name.includes('.broken-')) continue;
    const full = path.join(sessionPath, name);
    let body;
    try {
      body = fs.readFileSync(full, "utf8");
    } catch {
      continue;
    }
    try {
      JSON.parse(body);
    } catch {
      const dest = `${full}.broken-${Date.now()}`;
      try {
        fs.renameSync(full, dest);
        moved.push({ name, dest });
      } catch (e) {
        report?.(`gagal memindahkan ${name} ke karantina: ${e.message}`);
      }
    }
  }
  if (moved.length) {
    pruneCorruptKeyBackups(sessionPath, prefix, report);
    for (const m of moved) {
      report?.(
        `file key rusak dikarantina: ${m.name} -> ${path.basename(m.dest)} ` +
          `(isi tidak bisa di-parse; key dianggap hilang dan sesi akan ` +
          `di-re-negotiate, file aslinya tetap ada untuk dipulihkan)`,
      );
    }
  }
  return moved;
}

function pruneCorruptKeyBackups(sessionPath, prefix, report) {
  try {
    const backups = fs
      .readdirSync(sessionPath)
      .filter((f) => f.startsWith(prefix) && f.includes(".broken-"))
      .sort();
    while (backups.length > CORRUPT_KEY_KEEP) {
      const victim = backups.shift();
      try {
        fs.rmSync(path.join(sessionPath, victim), { force: true });
        report?.(`karantina key lama dibuang: ${victim}`);
      } catch { }
    }
  } catch { }
}

// Bungkus keys store lokal dengan pemulihan koruppsi.
//
// Read dilakukan PER ID, bukan satu batch: keys.get di library memakai
// Promise.all, jadi satu file rusak melempar untuk SELURUH batch dan
// menenggelamkan key yang sehat ikut yang salah baca. Per-id memakai
// allSettled, jadi key sehat tetap kembali dan hanya id yang rusak yang
// jadi miss.
//
// Bentuk objek store diteruskan apa adanya (get/set/getMany) supaya wrapper
// ini transparan untuk semua konsumennya.
function withCorruptKeyRecovery(store, { sessionPath, report } = {}) {
  if (!store || typeof store.get !== "function") return store;
  const wrapGet = (fetch) =>
    async function get(type, ids) {
      if (!Array.isArray(ids) || ids.length === 0) return {};
      let settled;
      try {
        settled = await Promise.allSettled(ids.map((id) => fetch(type, [id])));
      } catch {
        // library store tidak bisa dibaca sama sekali (mis. folder hilang)
        report?.(`keys.get(${type}) gagal total: tidak ada key yang bisa dibaca`);
        return {};
      }
      const out = {};
      let corrupt = 0;
      for (let i = 0; i < ids.length; i++) {
        const r = settled[i];
        if (r.status === "fulfilled" && r.value && r.value[ids[i]]) {
          out[ids[i]] = r.value[ids[i]];
        } else if (r.status === "rejected") {
          corrupt++;
        }
      }
      if (corrupt > 0 && sessionPath) {
        quarantineCorruptKeyFiles(sessionPath, type, report);
      }
      return out;
    };

  return {
    ...store,
    get: wrapGet((type, ids) => store.get(type, ids)),
    set: store.set ? store.set.bind(store) : undefined,
  };
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
  const { initAuthCreds } = await import('onigis');
  if (!folder) throw new Error('useDurableAuthState butuh folder lokal');

  // Jalur INI yang hidup di produksi (config.turso.enabled default true), dan
  // useMultiFileAuthState di dalamnya yang melempar kalau creds.json ada tapi
  // tidak bisa dipakai. Tanpa recovery di sini, tiap boot gagal sampai
  // operator menghapus storage/session dengan tangan.
  const loaded = await loadAuthStateWithRecovery(folder, warnRemote);
  const local = { state: loaded.state, saveCreds: loaded.saveCreds };
  let creds = local.state.creds;
  // Dibungkus DI SINI, bukan nanti di keys.get: ini satu-satunya titik di mana
  // store lokal dibuat, dan useMultiFileAuthState juga dipakai langsung dari
  // connection.js pada jalur tanpa Turso.
  const localKeys = withCorruptKeyRecovery(local.state.keys, {
    sessionPath: folder,
    report: warnRemote,
  });

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
      // lokal dulu (durable), Turso hanya untuk key yang belum ada di disk.
      // localKeys sudah dibungkus withCorruptKeyRecovery: file rusak jadi
      // miss + karantina, tidak pernah lempar ke sini. Jaring pengaman
      // tambahan tetap ada karena store lokal bisa gagal dengan cara lain
      // (EACCES, I/O), dan kegagalannya di sini berarti bot diam untuk peer
      // itu selamanya.
      let localGot = {};
      try {
        localGot = await localKeys.get(type, ids);
      } catch (e) {
        warnRemote(
          `keys.get(${type}) gagal total: ${e.message} (lokal) -- key dianggap hilang, sesi mungkin re-negotiate`,
        );
      }
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
      try {
        await localKeys.set(data);
      } catch (e) {
        // key yang gagal ditulis = sesi tidak bisa di-re-negotiate nanti.
        // Reporter, bukan dibiarkan hilang.
        warnRemote(`keys.set(${Object.keys(data || {}).join(",")}) gagal: ${e.message}`);
        return;
      }
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
  withCorruptKeyRecovery,
  quarantineCorruptKeyFiles,
  loadAuthStateWithRecovery,
  classifyAuthStateFailure,
  quarantineBrokenCreds,
};
