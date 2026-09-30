import { LRUCache } from "lru-cache";

const DEFAULT_MAX = 200;
const DEFAULT_TTL_MS = 5 * 60 * 1000;

// Pemisah NUL: pasangan ("a b","c") dan ("a","b c") menghasilkan string gabungan
// yang sama tanpa karakter ini, jadi satu entri bisa menimpa entri lain untuk
// key yang tidak berhubungan.
const SEP = "\u0000";

// Box RAM 1GB (src/lib/memory-monitor.js) tidak bisa menahan cache respons yang
// tumbuh mengikuti jumlah request, jadi jumlah entri dibatasi di sini.
// Disalin dangkal hanya untuk objek polos dan array. structuredClone ditolak
// karena mengubah Buffer menjadi Uint8Array dan melempar pada nilai yang punya
// fungsi, sedangkan pemanggil di repo ini menyimpan kedua-duanya. Salinan
// dangkal sudah menutup kasus yang paling sering merusak cache: plugin yang
// menambah atau mengubah field level atas pada hasil yang diterimanya.
function salin(nilai) {
  if (Array.isArray(nilai)) return nilai.slice();
  if (nilai && typeof nilai === "object" && nilai.constructor === Object) {
    return { ...nilai };
  }
  return nilai;
}

export function createCapabilityCache({
  max = DEFAULT_MAX,
  ttlMs = DEFAULT_TTL_MS,
  now = Date.now,
} = {}) {
  const entries = new LRUCache({ max, ttl: ttlMs });

  const slot = (capability, key) => `${capability}${SEP}${key}`;

  // Jam internal LRUCache adalah Date.now dan tidak bisa di-inject, jadi umur
  // juga dicatat di sini memakai jam yang diberikan pemanggil. Tanpa itu
  // parameter `now` hanya jadi hiasan dan batas umur mustahil diuji tanpa
  // menunggu detik sungguhan.
  const basi = (entry) => now() - entry.at > ttlMs;

  function get(capability, key) {
    const entry = entries.get(slot(capability, key));
    if (entry === undefined) return undefined;
    if (basi(entry)) return undefined;
    return salin(entry.value);
  }

  function set(capability, key, value) {
    entries.set(slot(capability, key), { value: salin(value), at: now() });
  }

  // Dibutuhkan oleh resetResolver() supaya cache tidak bocor antar test.
  function clear() {
    entries.clear();
  }

  return { get, set, size: () => entries.size, clear };
}