// Tujuh plugin to*.js (tomekah, tofigure, tofigurev2, tojapanese, tohijab,
// toghibli, tomoai) butuh API efek gambar yang menerima URL. Semuanya dulu
// menunjuk satu host: api-faa.my.id/faa/<efek>. Waktu itu Cloudflare WAF
// memblokir /faa/* per IP klien, jadi 7 command mati bersamaan tanpa fallback.
//
// WAF memblokir per IP, jadi dari VPS owner host itu bisa tetap jalan. Karena
// itu daftar provider di sini BISA DIKONFIGURASI, tidak dipindah permanen.
//
// Dua jebakan yang ditangani di sini:
//  1. Beberapa host membalas HTTP 200 dengan halaman HTML, bukan gambar.
//     Kalau diteruskan, bot mengirim HTML ke user seolah-olah gambar.
//  2. f() di src/lib/http.js me-return null untuk semua error dan membuang
//     status code, sehingga WAF 403 pernah dilaporkan "502 server error" dan
//     diagnosis mustahil. Karena itu request di sini sendiri, bukan lewat f().


import config from "../../config.js";

const PROVIDER_TIMEOUT_MS = 45000;
const UA = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";


// config.effectApi.providers adalah sumber kebenaran; array di bawah cuma
// fallback kalau config tidak mengaturnya. vynaa.web.id (suite botcahx-maker
// yang sama dengan /faa/*) ENOTFOUND per 2026-09-27, jadi tidak masuk default.
const FALLBACK_PROVIDERS = [
  { name: "faa", base: "https://api-faa.my.id/faa" },
];

export const DEFAULT_EFFECT_PROVIDERS =
  config.effectApi?.providers?.length ? config.effectApi.providers : FALLBACK_PROVIDERS;

function withTimeout(promise, ms) {
  let timer;
  return Promise.race([
    Promise.resolve(promise),
    new Promise((_, reject) => {
      timer = setTimeout(
        () => reject(Object.assign(new Error(`timeout ${ms}ms`), { status: 408 })),
        ms,
      );
    }),
  ]).finally(() => clearTimeout(timer));
}

export function describeProviderFailure({ status, message } = {}) {
  if (status === 403) return "403 (WAF memblokir IP ini)";
  if (status === 404) return "404 (endpoint tidak ada di host itu)";
  if (status === 429) return "429 (rate limit)";
  if (status === 408) return "408 (timeout)";
  if (status === 502) return "502 (respons tidak terbaca)";
  if (status >= 500) return `${status} (server error)`;
  return message || "tidak diketahui";
}

// signature PNG, JPEG, GIF, WEBP/RIFF, BMP
function looksLikeImage(buf) {
  if (!Buffer.isBuffer(buf) || buf.length < 12) return false;
  const hex = buf.subarray(0, 12).toString("hex");
  return (
    hex.startsWith("89504e47") ||
    hex.startsWith("ffd8ff") ||
    hex.startsWith("47494638") ||
    hex.startsWith("52494646") ||
    hex.startsWith("424d")
  );
}

export async function runEffectWA(provider, effect, imageUrl, opts = {}) {
  const doFetch = opts.fetchImpl ?? fetch;
  const key = typeof provider.apikey === "function" ? provider.apikey() : provider.apikey;
  const url =
    `${provider.base}/${encodeURIComponent(effect)}` +
    `?url=${encodeURIComponent(imageUrl)}` +
    (key ? `&apikey=${encodeURIComponent(key)}` : "");

  const res = await doFetch(url, {
    headers: { "user-agent": UA, accept: "image/*,*/*" },
    signal: opts.signal,
  });

  if (!res.ok) {
    throw Object.assign(new Error(`HTTP ${res.status}`), { status: res.status });
  }

  const buf = Buffer.from(await res.arrayBuffer());
  if (!looksLikeImage(buf)) {
    throw Object.assign(new Error("host membalas HTML, bukan gambar"), { status: 200 });
  }
  return buf;
}

/**
 * @param {string} effect nama efek, mis. "tomekah"
 * @param {string} imageUrl URL gambar yang sudah di-upload
 * @returns {Promise<Buffer>} buffer gambar hasil
 */
export async function applyImageEffect(effect, imageUrl, opts = {}) {
  const providers = opts.providers ?? DEFAULT_EFFECT_PROVIDERS;
  if (!providers.length) {
    throw new Error(`Efek ${effect} gagal: tidak ada provider yang dikonfigurasi`);
  }

  const failures = [];

  for (const provider of providers) {
    const run = provider.run ?? ((p, e, u) => runEffectWA(p, e, u));
    try {
      const buf = await withTimeout(
        run(provider, effect, imageUrl),
        opts.timeoutMs ?? config.effectApi?.timeoutMs ?? PROVIDER_TIMEOUT_MS,
      );
      if (!looksLikeImage(buf)) {
        throw Object.assign(new Error("host membalas HTML, bukan gambar"), { status: 200 });
      }
      return buf;
    } catch (e) {
      failures.push(`${provider.name}: ${describeProviderFailure(e)}`);
    }
  }

  throw new Error(`Efek ${effect} gagal di semua provider — ${failures.join(" | ")}`);
}

export { looksLikeImage };
