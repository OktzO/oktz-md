import config from "../../config.js";
import { httpAxios } from "./http.js";

// Peta aggregator ke key API. key: null berarti host ini benar-benar tidak
// butuh key dan header key tidak boleh dikirim sama sekali.
//
// Daftar ini sengaja pendek. Agregator gratis adalah sumber daya cadangan
// yang paling sering mati, jadi hanya host yang masih hidup dan punya endpoint
// yang benar-benar dipakai saja yang dicatat di sini.
const AGGREGATORS = {
  neoxr: { base: "https://api.neoxr.eu", key: "neoxr" },
  nexray: { base: "https://api.nexray.eu.cc", key: null },
  izuka: { base: "https://my.izuka-api.xyz", key: null },
  cuki: { base: "https://api.cuki.biz.id", key: "cuki" },
  siputzx: { base: "https://api.siputzx.my.id", key: null },
  azbry: { base: "https://api.azbry.com", key: null },
};

// Lebih pendek dari timeout instance httpAxios (15 detik di src/lib/http.js)
// karena pemanggil punya budget milidetik: menunggu 15 detik baru sadar hostnya
// sudah tidak ada jauh lebih lambat daripada tidak menunggu sama sekali.
const DEFAULT_TIMEOUT_MS = 5000;

// .status dibiarkan kosong kalau request tidak pernah terjadi (nama asing atau
// key kosong) dan diisi 0 kalau request sudah terjadi tapi tidak ada respons
// HTTP. Pola "status ||= 500" di src/lib/apimanager.js:1143 menghapus
// perbedaan itu, padahal konsumen seperti src/lib/serialize.js:561 justru
// memeriksa `typeof error.status === "number"` untuk membedakan upstream
// 4xx/5xx dari kegagalan koneksi atau DNS.
export class AggregatorError extends Error {
  constructor(message, { aggregator, status, cause, envName } = {}) {
    super(message);
    this.name = "AggregatorError";
    this.aggregator = aggregator;
    if (typeof status === "number") this.status = status;
    if (envName) this.envName = envName;
    if (cause !== undefined) this.cause = cause;
  }
}

function defaultKeyOf(name) {
  const entry = AGGREGATORS[name];
  return entry?.key ? config.APIkey[entry.key] : "";
}

// Nama env untuk satu aggregator, atau `null` kalau host itu tidak butuh key.
// Satu-satunya tempat di repo ini yang memetakan host ke nama env, supaya
// plugin tidak perlu menyebut nama host aggregator sama sekali hanya untuk
// menjelaskan key yang kurang.
export function envNameOf(name) {
  const key = AGGREGATORS[name]?.key;
  return key ? `APIKEY_${String(key).toUpperCase()}` : null;
}

/**
 * Terjemahkan kegagalan aggregator "API key … belum diisi" yang tercatat di
 * `tried` resolver menjadi nama env yang harus diisi user.
 *
 * Plugin butuh ini supaya bisa tetap jujur soal key yang kosong tanpa
 * mentioning host aggregators: pemanggilan aggregator hanya boleh lewat
 * `hit`, dan nama host tidak boleh bocor ke layer plugin.
 */
export function missingEnvNameOf(error) {
  for (const entry of error?.tried ?? []) {
    const cocok = /APIKEY_[A-Z0-9_]+/.exec(entry?.reason ?? "");
    if (cocok) return cocok[0];
  }
  return null;
}

export function createAggregatorClient({
  http = httpAxios,
  keyOf = defaultKeyOf,
} = {}) {
  // async, bukan fungsi biasa: pemanggil memakai .catch() dan tidak boleh
  // melempar sinkron di depan promise itu terbentuk.
  async function hit(
    name,
    path,
    { params, method = "GET", data, timeout = DEFAULT_TIMEOUT_MS, signal } = {},
  ) {
    const entry = AGGREGATORS[name];
    if (!entry) {
      throw new AggregatorError(`aggregator tidak dikenal: ${name}`, {
        aggregator: name,
      });
    }

// Key dicek sebelum ada request: .env di deployment sering masih template,
    // dan request tanpa key hanya membuang satu timeout untuk jawaban 401 yang
    // sudah bisa diprediksi di lokal. Nama env ikut ditulis ke pesan supaya
    // lapisan atas bisa menunjuk user ke baris .env yang tepat tanpa perlu tahu
    // host mana yang gagal — lihat `missingEnvNameOf`.
    const key = entry.key ? String(keyOf(name) ?? "") : "";
    if (entry.key && key === "") {
      const env = envNameOf(name);
      throw new AggregatorError(
        `API key ${entry.key} belum diisi, set ${env} di .env; ${name} dilewati`,
        { aggregator: name, envName: env },
      );
    }

    const target = new URL(entry.base + (path.startsWith("/") ? path : `/${path}`));
    for (const [nama, nilai] of Object.entries(params ?? {})) {
      if (nilai === undefined || nilai === null) continue;
      target.searchParams.set(nama, String(nilai));
    }

    const opts = {
      headers: entry.key ? { apikey: key } : {},
      timeout,
      // Tanpa ini, budget resolver hanya menghentikan resolver menunggu:
      // request ke tier api akan tetap jalan sampai timeout di atas, dan batas
      // 8 detik yang dijanjikan resolver tidak benar-benar ditegakkan.
      signal,
      ...(data === undefined ? {} : { data }),
    };

    try {
      const res = await http[method.toLowerCase()](target.toString(), opts);
      const status = res?.status ?? 0;
      if (status < 200 || status >= 300) {
        throw new AggregatorError(`${name} menjawab ${status}`, {
          aggregator: name,
          status,
        });
      }
      return res.data;
    } catch (error) {
      // AggregatorError yang kita lempar sendiri diteruskan apa adanya, supaya
      // status yang sudah benar tidak tertimpa `?? 0`.
      if (error instanceof AggregatorError) throw error;
      throw new AggregatorError(`${name}: ${error?.message ?? "request gagal"}`, {
        aggregator: name,
        status: error?.response?.status ?? 0,
        cause: error,
      });
    }
  }

  return { hit };
}

// Satu instance dipakai bersama seluruh kapabilitas. `hit` tidak menyimpan
// state per request, jadi klien per kapabilitas hanya mengulang pembacaan
// config tanpa ada gunanya.
export const aggregator = createAggregatorClient();