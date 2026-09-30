import { LRUCache } from "lru-cache";

// Batas slot dan cooldown default. Host yang gagal terus-menerus harus
// dilupakan supaya tidak menambah beban request ke host yang sudah jelas mati
// (beberapa aggregator gratis di repo ini sudah lama 404/500).
const DEFAULT_MAX = 64;
const DEFAULT_THRESHOLD = 3;
const DEFAULT_COOLDOWN_MS = 30_000;

// State per nama: { failures, openedAt }. openedAt = kapan breaker terakhir
// dibuka; null berarti belum pernah OPEN atau sudah di-reset.
//
// Map polos ditolak di sini: nama host berasal dari daftar kapabilitas dan
// endpoint, jadi tanpa batas ia tumbuh terus dan tidak pernah dibersihkan.
// Box RAM 1GB dengan GC di 380MB (src/lib/memory-monitor.js) tidak menyisakan
// kelonggaran untuk struktur terbuka seperti itu.
export function createBreaker({
  max = DEFAULT_MAX,
  threshold = DEFAULT_THRESHOLD,
  cooldownMs = DEFAULT_COOLDOWN_MS,
  now = Date.now,
} = {}) {
  // ttl sengaja tidak dipakai: yang membatasi adalah jumlah slot, bukan umur.
  // Host aktif sering di-get sehingga urutan LRU-nya naik dan tidak tersingkir
  // oleh host yang kebetulan dipakai lebih sering lebih dulu.
  const slots = new LRUCache({ max, ttl: undefined });

  // Baca tidak boleh membuat slot: isOpen() dipanggil untuk setiap backend di
  // setiap resolve, jadi membuat slot di sana akan exhausting capacity breaker
  // hanya dengan pembacaan.
  const stateOf = (name) =>
    slots.get(name) ?? { failures: 0, openedAt: null };

  function isOpen(name) {
    const { openedAt } = stateOf(name);
    if (openedAt === null) return false;
    // Lewat cooldown = Half-Open: percobaan berikutnya boleh masuk supaya host
    // yang sudah pulih tidak ditolak selamanya.
    return now() - openedAt < cooldownMs;
  }

  function recordFailure(name) {
    const state = stateOf(name);
    const failures = state.failures + 1;
    slots.set(name, {
      failures,
      openedAt: failures >= threshold ? now() : state.openedAt,
    });
  }

  function recordSuccess(name) {
    slots.set(name, { failures: 0, openedAt: null });
  }

  function snapshot() {
    return [...slots.entries()].map(([name, state]) => ({
      name,
      failures: state.failures,
      openedAt: state.openedAt,
    }));
  }

  // Dibutuhkan oleh resetResolver() supaya state breaker tidak bocor antar test.
  function clear() {
    slots.clear();
  }

  return { isOpen, recordFailure, recordSuccess, snapshot, clear };
}