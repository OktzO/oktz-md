import { createBreaker } from "./circuit-breaker.js";
import { createCapabilityCache } from "./capability-cache.js";

const LOCAL_BUDGET_MS = 3000;
const TOTAL_BUDGET_MS = 8000;

// Registry kapabilitas. Fungsi panah, bukan import di module scope: kedelapan
// modul hanya dimuat saat namanya benar-benar dipanggil, jadi proses yang
// hanya butuh satu kapabilitas tidak membayar RAM untuk tujuh lainnya.
const CAPABILITIES = {
  spotify: () => import("../capabilities/spotify.js"),
  pinterest: () => import("../capabilities/pinterest.js"),
  douyin: () => import("../capabilities/douyin.js"),
  sfile: () => import("../capabilities/sfile.js"),
  videy: () => import("../capabilities/videy.js"),
  youtube: () => import("../capabilities/youtube.js"),
  ytmusic: () => import("../capabilities/ytmusic.js"),
  hd: () => import("../capabilities/hd.js"),
};

export class CapabilityError extends Error {
  constructor(message, { capability, tried = [], code = "unresolved", cause } = {}) {
    super(message);
    this.name = "CapabilityError";
    this.capability = capability;
    this.tried = tried;
    this.code = code;
    if (cause !== undefined) this.cause = cause;
  }
}

function timeoutError(ms) {
  const error = new Error(`batas waktu ${ms}ms habis`);
  error.reason = error.message;
  return error;
}

// Backend dijalankan di bawah AbortController supaya timeout benar-benar
// memutus kerja, bukan hanya melepaskannya. Scraper lokal yang sedang
// mengunduh file besar akan terus berjalan dan menahan socket jauh setelah
// pemanggil sudah pindah ke backend berikutnya.
function jalankan(backend, args, opts, ms) {
  const controller = new AbortController();
  let timer = null;

  // Promise.resolve().then(...) supaya backend yang melempar sinkron masuk ke
  // jalur gagal yang sama dengan yang melempar asinkron.
  const kerja = Promise.resolve().then(() =>
    backend.run(args, {
      signal: opts.signal
        ? AbortSignal.any([opts.signal, controller.signal])
        : controller.signal,
    }),
  );

  const batas = new Promise((_, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(timeoutError(ms));
    }, ms);
  });

  return Promise.race([kerja, batas]).finally(() => clearTimeout(timer));
}

async function cobaBackend(backend, cap, args, opts, ms) {
  let raw;
  try {
    raw = await jalankan(backend, args, opts, ms);
  } catch (error) {
    return { ok: false, reason: error?.reason ?? error?.message ?? "gagal" };
  }

  // null dihitung gagal, bukan data. Scraper lokal mengembalikan null saat
  // targetnya tidak ada, dan menyajikan { v: null } ke user lebih buruk
  // daripada mencoba backend berikutnya.
  if (raw === null || raw === undefined) return { ok: false, reason: "tanpa data" };

  try {
    return { ok: true, value: cap.normalize(raw) };
  } catch (error) {
    // Bentuk respons yang gagal dinormalisasi diperlakukan sebagai kegagalan
    // backend: bentuk dari backend lain mungkin masih bisa dipakai.
    return { ok: false, reason: `normalisasi gagal: ${error?.message ?? error}` };
  }
}

export function createResolver({
  capabilities = CAPABILITIES,
  breaker = createBreaker(),
  cache = createCapabilityCache(),
  now = Date.now,
  budget = { localMs: LOCAL_BUDGET_MS, totalMs: TOTAL_BUDGET_MS },
} = {}) {
  // Urutan tetap: semua local baru semua api. Agregator gratis adalah cadangan
  // terakhir yang paling sering mati, jadi posisi backend di array tidak boleh
  // menentukan backend mana yang dipakai lebih dulu.
  const bertingkat = (backends) => [
    ...backends.filter((b) => b.kind === "local"),
    ...backends.filter((b) => b.kind !== "local"),
  ];

  function antrean(backends) {
    const hidup = backends.filter((b) => !breaker.isOpen(b.name));
    if (hidup.length > 0) return bertingkat(hidup);

    // Semua backend OPEN. Gagal seketika di sini membuat kapabilitas mati
    // permanen selama proses hidup, jadi satu backend dengan openedAt terbaru
    // (paling dekat ke Closed) tetap dicoba sebagai percobaan pemulih.
    const states = new Map(
      (typeof breaker.snapshot === "function" ? breaker.snapshot() : [])
        .map((s) => [s.name, s]),
    );
    let pilihan = null;
    for (const backend of backends) {
      const openedAt = states.get(backend.name)?.openedAt ?? -Infinity;
      if (pilihan === null || openedAt > pilihan.openedAt) {
        pilihan = { backend, openedAt };
      }
    }
    return pilihan ? [pilihan.backend] : [];
  }

  // Backend boleh mendeklarasikan `applies(args)`: ia menjawab "apakah backend
  // ini melayani bentuk argumen ini", bukan "apakah hostnya hidup". Hasil
  // `false` berarti backend itu tidak pernah dihubungi, jadi tidak boleh
  // dihitung sebagai kegagalan — kalau tidak, satu kapabilitas dengan backend
  // bercabang ({ url } / { q }) akan mengeluarkan backend lokalnya sendiri dari
  // rotasi hanya karena lalu lintas memakai jalur yang memang tidak dilayaninya.
  // Penaringan dilakukan sebelum `antrean()` supaya urutan tier dan bailout
  // all-open tidak tersentuh.
  const berlaku = (backends, args) =>
    backends.filter(
      (b) => typeof b.applies !== "function" || b.applies(args) !== false,
    );

  async function muat(capability) {
    const loader = capabilities[capability];
    if (typeof loader !== "function") {
      throw new CapabilityError(`kapabilitas tidak dikenal: ${capability}`, {
        capability,
        code: "unknown-capability",
      });
    }

    let cap;
    try {
      const mod = await loader();
      cap = mod?.default ?? mod;
    } catch (error) {
      // Pesan ditulis ulang karena stack mentah dari import dinamis tidak
      // menyebut kapabilitas mana yang gagal dimuat.
      throw new CapabilityError(
        `modul kapabilitas ${capability} gagal dimuat (src/capabilities/${capability}.js)`,
        { capability, code: "load-failed", cause: error?.message },
      );
    }

    if (!cap || !Array.isArray(cap.backends) || cap.backends.length === 0) {
      throw new CapabilityError(`kapabilitas ${capability} tidak punya backend`, {
        capability,
        code: "no-backend",
      });
    }
    return cap;
  }

  async function resolve(capability, args = {}, opts = {}) {
    const mulai = now();
    const cap = await muat(capability);

    // Cache hanya untuk kapabilitas stable, dan hanya sebagai jalur cepat.
    // Tidak ada cabang di bawah yang mengembalikan entri cache lama ketika
    // semua backend gagal: jawaban basi lebih buruk daripada jawaban jujur
    // bahwa permintaan gagal.
    const cacheKey = cap.stable === true ? JSON.stringify(args ?? {}) : null;
    if (cacheKey !== null) {
      const cached = cache.get(capability, cacheKey);
      if (cached !== undefined) {
        return {
          ok: true,
          source: cached.source,
          data: cached.data,
          meta: { tookMs: now() - mulai, cached: true },
        };
      }
    }

    // Tidak ada satu pun backend yang mau melayani argumen ini. Keadaan ini
    // dibedakan dari "semua backend gagal": di sana hostnya sudah dicoba dan
    // menjawab, di sini tidak ada yang dihubungi sama sekali, jadi breaker
    // harus tetap kosong.
    const kandidat = berlaku(cap.backends, args);
    if (kandidat.length === 0) {
      throw new CapabilityError(
        `tidak ada backend ${capability} yang melayani argumen ini`,
        { capability, code: "no-applicable-backend" },
      );
    }

    const daftar = antrean(kandidat);
    const localCount = daftar.filter((b) => b.kind === "local").length;
    const apiCount = daftar.length - localCount;
    // Local berbagi budget localMs, api memakai sisa totalMs. Jumlahnya tidak
    // pernah melewati totalMs, jadi pemanggil selalu punya batas atas yang bisa
    // diandalkan meski semua backend lambat.
    const localMs = Math.max(1, Math.floor(budget.localMs / Math.max(1, localCount)));
    const sisaMs = Math.max(1, budget.totalMs - budget.localMs);
    const apiMs = Math.max(1, Math.floor(sisaMs / Math.max(1, apiCount)));
    const msUntuk = (backend) => (backend.kind === "local" ? localMs : apiMs);

    const tried = [];
    for (const backend of daftar) {
      const hasil = await cobaBackend(backend, cap, args, opts, msUntuk(backend));
      if (!hasil.ok) {
        breaker.recordFailure(backend.name);
        tried.push({ name: backend.name, reason: hasil.reason });
        continue;
      }
      breaker.recordSuccess(backend.name);
      if (cacheKey !== null) {
        cache.set(capability, cacheKey, { source: backend.name, data: hasil.value });
      }
      return {
        ok: true,
        source: backend.name,
        data: hasil.value,
        meta: { tookMs: now() - mulai, cached: false },
      };
    }

    throw new CapabilityError(`semua backend ${capability} gagal`, { capability, tried });
  }

  return { resolve, breaker, cache, capabilities };
}

export const resolver = createResolver();

export function resetResolver() {
  resolver.breaker.clear();
  resolver.cache.clear();
}

export { CAPABILITIES, LOCAL_BUDGET_MS, TOTAL_BUDGET_MS };