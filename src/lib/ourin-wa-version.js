const DEFAULT_VERSION = [2, 3000, 1043857760];
const CACHE_TTL_MS = 6 * 3600e3;

function race(promise, ms) {
  let timer;
  const guard = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error("timeout")), ms);
  });
  // timer SENGAJA tidak di-unref: kalau tidak ada handle lain, event loop
  // bisa habis sebelum guard menyala dan promise menggantung. `finally` di
  // bawah sudah clearTimeout, jadi tidak menahan proses.
  return Promise.race([Promise.resolve(promise), guard]).finally(() => {
    clearTimeout(timer);
  });
}

/**
 * Kedua helper versi library TIDAK pernah throw — di network fail mereka
 * return { version: <fallback>, isLatest: false, error }. Jadi `.isLatest`
 * penentu keberhasilan, bukan try/catch, dan semua sumber harus dicoba
 * sebelum menerima fallback.
 *
 * Selalu mengembalikan array versi konkret: `makeWASocket` melakukan spread
 * config, jadi `version: undefined` menimpa default library lalu handshake
 * jatuh saat dereference `config.version[0]`.
 */
export async function resolveWaVersion({
  fetchers = [],
  cache = null,
  timeoutMs = 5000,
  defaultVersion = DEFAULT_VERSION,
} = {}) {
  if (cache?.v && Date.now() - cache.t < CACHE_TTL_MS) {
    return { version: cache.v, isLatest: true };
  }

  for (const fetcher of fetchers) {
    let result;
    try {
      result = await race(fetcher(), timeoutMs);
    } catch {
      continue;
    }
    if (result?.version && result.isLatest) {
      return { version: result.version, isLatest: true };
    }
  }

  // semua sumber gagal/terblokir: pakai cache basahi bila ada, lalu default
  return {
    version: cache?.v || defaultVersion,
    isLatest: false,
  };
}

export { DEFAULT_VERSION, CACHE_TTL_MS };
