/**
 * Fan-out upload ke banyak host.
 *
 * Dua masalah yang harus dipecahkan di sini (bukan di plugin):
 *
 * 1. Sequential = user nunggu jumlah-host × timeout. 9 host dengan timeout
 *    30-120 detik bisa jadi 10 menit. Semua item dijalankan bareng.
 *
 * 2. Satu host yang menggantung tidak boleh menahan host yang sudah selesai.
 *    Setiap item punya batas sendiri (perHostMs), dan seluruh proses punya
 *    batas total (deadlineMs). Item yang belum selesai saat salah satu batas
 *    tercapai dilaporkan sebagai timeout — bukan dianggap gagal, karena kita
 *    memang tidak tahu hasilnya.
 *
 * Semua timer dibersihkan supaya tidak menahan event loop.
 */

const DEFAULT_DEADLINE_MS = 45000;
const DEFAULT_PER_HOST_MS = 30000;

/**
 * @param {Array<{name: string, run: () => Promise<any>}>} items
 * @param {{deadlineMs?: number, perHostMs?: number, onSettled?: (n: number) => void}} [options]
 * @returns {Promise<{done: any[], failed: string[], timedOut: string[]}>}
 */
export async function runUploadFanout(items, options = {}) {
  const deadlineMs = options.deadlineMs ?? DEFAULT_DEADLINE_MS;
  const perHostMs = options.perHostMs ?? DEFAULT_PER_HOST_MS;

  if (!Array.isArray(items) || items.length === 0) {
    return { done: [], failed: [], timedOut: [] };
  }

  const done = [];
  const failed = [];
  const timedOut = [];
  /** @type {Map<string, string>} name -> 'done' | 'failed' | 'timedOut' */
  const state = new Map();

  const timers = [];
  let remaining = items.length;

  const finish = (name, kind, value) => {
    if (state.has(name)) return;
    state.set(name, kind);

    if (kind === "done") {
      if (value && typeof value.url === "string" && value.url) {
        done.push(value);
      } else {
        failed.push(name);
        state.set(name, "failed");
      }
    } else if (kind === "failed") {
      failed.push(name);
    } else {
      timedOut.push(name);
    }

    remaining -= 1;
    options.onSettled?.(remaining);
  };

  for (const item of items) {
    const hostTimer = setTimeout(() => finish(item.name, "timedOut"), perHostMs);
    timers.push(hostTimer);

    Promise.resolve()
      .then(() => item.run())
      .then(
        (value) => finish(item.name, "done", value),
        (err) => {
          if (process.env.NODE_ENV !== "test") {
            console.warn(`[fanout] ${item.name}: ${err?.message || err}`);
          }
          finish(item.name, "failed", err);
        },
      );
  }

  const overallTimer = setTimeout(() => {
    // Item yang masih jalan = timeout.
    for (const item of items) {
      finish(item.name, "timedOut");
    }
  }, deadlineMs);
  timers.push(overallTimer);

  await new Promise((resolve) => {
    if (remaining === 0) {
      resolve();
      return;
    }
    const poll = setInterval(() => {
      if (remaining === 0) {
        clearInterval(poll);
        resolve();
      }
    }, 20);
    timers.push(poll);
  });

  for (const t of timers) clearTimeout(t);
  clearInterval(timers[timers.length - 1]);

  return { done, failed, timedOut };
}

export { DEFAULT_DEADLINE_MS, DEFAULT_PER_HOST_MS };
