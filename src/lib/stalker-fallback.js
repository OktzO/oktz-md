import axios from "axios";
import config from "../../config.js";

const NEXRAY_BASES = [
  "https://api.nexray.eu.cc",
  "https://api.nexray.web.id",
];

const FIREFLY = "https://firefly.maiku.my.id";

/**
 * Error khusus saat API key provider belum di-set di .env. Dipakai supaya
 * plugin bisa bilang key mana yang kurang, bukan pesan error generik.
 */
export class MissingApiKeyError extends Error {
  constructor(provider, envName) {
    super(`${provider}: API key belum di-set (set ${envName} di .env)`);
    this.name = "MissingApiKeyError";
    this.envName = envName;
    this.provider = provider;
  }
}

/**
 * Pesan yang bisa dibaca user, bukan template generik. Kalau penyebabnya
 * API key yang belum di-set, sebut nama env-nya — itu satu-satunya
 * informasi yang berguna untuk owner.
 */
export function explainFailure(error, command) {
  if (error instanceof MissingApiKeyError) {
    return (
      `❌ *API Key belum diset!*\n\n` +
      `> Command \`${command}\` butuh \`${error.envName}\` di \`.env\`\n` +
      `> Saat ini belum ada, jadi tidak bisa dipanggil.`
    );
  }
  if (error?.code === "ENOTFOUND" || error?.code === "ECONNREFUSED") {
    return (
      `❌ *Server tidak bisa dihubungi*\n\n` +
      `> Host \`${error.host || error.config?.url || "?"}\` tidak merespons.\n` +
      `> Coba lagi nanti, kak.`
    );
  }
  return null;
}

/**
 * Jalankan provider berurutan sampai ada yang berhasil. Setiap provider
 * harus mengembalikan object yang sudah ternormalisasi — caller tidak boleh
 * tahu datanya datang dari host mana.
 *
 * @param {Array<{name: string, run: () => Promise<any>}>} providers
 * @returns {Promise<{value: any, provider: string}>}
 */
export async function withFallback(providers) {
  const errors = [];
  let missingKey = null;

  for (const p of providers) {
    try {
      const value = await p.run();
      if (value === null || value === undefined) {
        throw new Error(`${p.name} mengembalikan data kosong`);
      }
      return { value, provider: p.name };
    } catch (e) {
      if (e instanceof MissingApiKeyError && !missingKey) missingKey = e;
      const msg = e?.response?.status ? `${p.name} HTTP ${e.response.status}` : e.message;
      errors.push(msg);
      console.warn(`[withFallback] ${msg}`);
    }
  }

  // Kalau semua provider gagal dan setidaknya satu gagal karena key kosong,
  // lebih berguna memberitahu key yang mana daripada dump error HTTP.
  if (missingKey && errors.every((e) => /belum di-set/.test(e))) {
    throw missingKey;
  }

  throw new Error(errors.join(" | ") || "Semua provider gagal");
}

function fireflyKey() {
  return config.APIkey.firefly || "";
}

/** Stalk GitHub. Firefly dulu (butuh key), nexray sebagai cadangan (tanpa key). */
export async function fetchGithubProfile(username) {
  return withFallback([
    {
      name: "firefly",
      run: async () => {
        if (!fireflyKey()) throw new MissingApiKeyError("firefly", "APIKEY_FIREFLY");
        const res = await axios.get(
          `${FIREFLY}/api/stalk-github?apikey=${fireflyKey()}&username=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        if (!res.data?.status || !res.data?.data) {
          throw new Error(res.data?.message || "tidak ditemukan");
        }
        const d = res.data.data;
        return {
          username: d.username,
          name: d.name,
          company: d.company,
          location: d.location,
          bio: d.bio,
          url: d.url,
          avatar: d.avatar,
          publicRepos: d.public_repos,
          followers: d.followers,
          following: d.following,
        };
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}/stalker/github?username=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        const r = res.data?.result;
        if (!res.data?.status || !r) {
          throw new Error(res.data?.error || "tidak ditemukan");
        }
        // nexray pakai nama field berbeda: nickname/profile_pic/public_repo.
        return {
          username: r.username,
          name: r.nickname,
          company: r.company,
          location: r.location,
          bio: r.bio,
          url: r.url,
          avatar: r.profile_pic,
          publicRepos: r.public_repo,
          followers: r.followers,
          following: r.following,
        };
      },
    },
  ]);
}

/** Stalk TikTok. Field nexray: stats.likes (firefly: stats.hearts). */
export async function fetchTiktokProfile(username) {
  return withFallback([
    {
      name: "firefly",
      run: async () => {
        if (!fireflyKey()) throw new MissingApiKeyError("firefly", "APIKEY_FIREFLY");
        const res = await axios.get(
          `${FIREFLY}/api/stalk-tiktok?apikey=${fireflyKey()}&username=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        if (!res.data?.status || !res.data?.data) {
          throw new Error(res.data?.message || "tidak ditemukan");
        }
        const d = res.data.data;
        const s = d.stats || {};
        return {
          username: d.username,
          name: d.nickname,
          avatar: d.avatar,
          bio: d.signature,
          verified: !!d.verified,
          private: !!d.private,
          followers: s.followers,
          following: s.following,
          likes: s.hearts,
          videos: s.videos,
          link: `https://tiktok.com/@${d.username}`,
        };
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}/stalker/tiktok?username=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        const r = res.data?.result;
        if (!res.data?.status || !r) {
          throw new Error(res.data?.error || "tidak ditemukan");
        }
        const s = r.stats || {};
        return {
          username: r.username,
          name: r.name,
          avatar: r.avatar,
          bio: r.bio,
          // nexray kirim string "Unverified"/"No", bukan boolean.
          verified: r.verified === true || r.verified === "Verified",
          private: r.private === true || r.private === "Yes",
          followers: s.raw_followers ?? s.followers,
          following: s.raw_following ?? s.following,
          likes: s.raw_likes ?? s.likes,
          videos: s.raw_videos ?? s.videos,
          link: r.link,
        };
      },
    },
  ]);
}

/**
 * Stalk NPM. Nexray menyediakannya di /stalker/npmjs dengan parameter `name`
 * (bukan `username` — kalau `username` dijawab 400 "Name is required").
 */
export async function fetchNpmProfile(username) {
  return withFallback([
    {
      name: "firefly",
      run: async () => {
        if (!fireflyKey()) throw new MissingApiKeyError("firefly", "APIKEY_FIREFLY");
        const res = await axios.get(
          `${FIREFLY}/api/stalk-npm?apikey=${fireflyKey()}&username=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        if (!res.data?.status || !res.data?.data) {
          throw new Error(res.data?.message || "tidak ditemukan");
        }
        const d = res.data.data;
        return {
          username: d.username,
          name: d.name,
          email: d.email,
          description: d.name,
          profile: d.profile,
          avatar: d.avatar,
          packages: d.packages || [],
        };
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}/stalker/npmjs?name=${encodeURIComponent(username)}`,
          { timeout: 30000 },
        );
        const r = res.data?.result;
        if (!res.data?.status || !r) {
          throw new Error(res.data?.error || "tidak ditemukan");
        }
        return {
          username: r.name,
          name: r.author?.name || r.name,
          email: r.author?.email || "",
          description: r.description,
          profile: r.homepage || `https://www.npmjs.com/~${r.name}`,
          avatar: "",
          packages: [],
        };
      },
    },
  ]);
}

/**
 * AI text. Cuki dulu (butuh APIKEY_CUKI), nexray sebagai cadangan (tanpa key).
 * Balasan cuki dan nexray dibungkus shape berbeda, jadi dinormalisasi ke string.
 */
export async function fetchAiText(kind, text) {
  const cukiKey = config.APIkey.cuki || "";
  const cukiPaths = { wormgpt: "/api/ai/wormgpt", gita: "/api/ai/gita" };
  const nexrayPaths = { wormgpt: "/ai/chatgpt", gita: "/ai/gitagpt" };

  const cukiPath = cukiPaths[kind];
  const nexrayPath = nexrayPaths[kind];
  if (!cukiPath) throw new Error(`jenis AI tidak dikenal: ${kind}`);

  return withFallback([
    {
      name: "cuki",
      run: async () => {
        if (!cukiKey) throw new MissingApiKeyError("cuki", "APIKEY_CUKI");
        const res = await axios.get(
          `https://api.cuki.biz.id${cukiPath}?apikey=${cukiKey}&${
            kind === "gita" ? "q" : "question"
          }=${encodeURIComponent(text)}`,
          { timeout: 30000 },
        );
        const out =
          res.data?.data?.response || res.data?.results || res.data?.data?.text;
        if (!res.data?.status && !res.data?.success) {
          throw new Error(res.data?.error || "respons kosong");
        }
        if (!out) throw new Error("respons kosong");
        return String(out).trim();
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}${nexrayPath}?text=${encodeURIComponent(text)}`,
          { timeout: 45000 },
        );
        if (!res.data?.status || !res.data?.result) {
          throw new Error(res.data?.error || "respons kosong");
        }
        return String(res.data.result).trim();
      },
    },
  ]);
}

/**
 * Spam NGL. Cuki dulu (butuh key), nexray /tools/spamngl sebagai cadangan.
 * Nexray memakai param: url + jumlah + pesan (bukan link/text).
 * Endpoint mengembalikan JSON { status, result } — BUKAN binary.
 */
export async function sendNgl(link, text, jumlah = 1) {
  const cukiKey = config.APIkey.cuki || "";

  return withFallback([
    {
      name: "cuki",
      run: async () => {
        if (!cukiKey) throw new MissingApiKeyError("cuki", "APIKEY_CUKI");
        // cuki balaszannya { success: true } kalau sukses, { success: false,
        // error } kalau tidak — jadi harus dicek, bukan cuma status HTTP.
        const res = await axios.post(
          "https://api.cuki.biz.id/api/ephoto/ngl",
          { link, text },
          {
            params: { apikey: cukiKey },
            timeout: 30000,
          },
        );
        if (res.data && res.data.success === false) {
          throw new Error(res.data.error || "cuki menolak request");
        }
        return { link, text, jumlah: 1 };
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}/tools/spamngl?url=${encodeURIComponent(link)}` +
            `&jumlah=${encodeURIComponent(jumlah)}&pesan=${encodeURIComponent(text)}`,
          { timeout: 30000 },
        );
        if (!res.data?.status) {
          throw new Error(res.data?.error || "gagal mengirim");
        }
        return { link, text, jumlah, note: res.data.result };
      },
    },
  ]);
}

/**
 * NIK parser. Obscura dulu (APIKEY_OBSCURA), nexray /tools/nikparse sebagai
 * cadangan. Dua host ini punya nama field berbeda, jadi dinormalisasi ke
 * bentuk yang dipakai plugins/tools/nikparser.js.
 */
export async function parseNik(nik) {
  const obscuraKey = config.APIkey.obscura || "";

  return withFallback([
    {
      name: "obscura",
      run: async () => {
        if (!obscuraKey) throw new MissingApiKeyError("obscura", "APIKEY_OBSCURA");
        const res = await axios.get(
          `https://api.obscuraworks.org/api/v2/tools/nik?nik=${encodeURIComponent(nik)}`,
          {
            timeout: 30000,
            headers: { Authorization: `Bearer ${obscuraKey}` },
          },
        );
        if (!res.data?.valid) {
          throw new Error("NIK tidak valid");
        }
        const d = res.data;
        return {
          valid: true,
          raw: d.raw,
          birthISO: d.birthISO,
          gender: d.gender,
          provinceId: d.provinceId,
          province: d.province,
          kabupatenKotaId: d.kabupatenKotaId,
          kecamatanId: d.kecamatanId,
          uniqcode: d.uniqcode,
        };
      },
    },
    {
      name: "nexray",
      run: async () => {
        const res = await axios.get(
          `${NEXRAY_BASES[0]}/tools/nikparse?nik=${encodeURIComponent(nik)}`,
          { timeout: 30000 },
        );
        const r = res.data?.result;
        if (!res.data?.status || !r) {
          throw new Error(res.data?.error || "gagal parse");
        }
        // Nexray: kelamin "PEREMPUAN"/"LAKI-LAKI", lahir "DD/MM/YYYY".
        // Kadang bulan dikirim > 12 (mis. "05/67/1989") karena NIK encode
        // bulan sebagai (bulan + 12) untuk perempuan. Normalisasi ke bulan
        // asli; tahun ikut digeser karena itu rentang 2000-an.
        const lahir = String(r.lahir || "");
        const [dd, mm, yyyy] = lahir.split("/");
        let birthISO = null;
        if (dd && mm && yyyy) {
          const rawMonth = Number(mm);
          const month = rawMonth > 12 ? rawMonth - 12 : rawMonth;
          const year =
            rawMonth > 12 ? Number(yyyy) + 2000 : Number(yyyy);
          const parsed = new Date(year, month - 1, Number(dd));
          if (!Number.isNaN(parsed.getTime())) {
            birthISO = parsed.toISOString();
          }
        }
        // Nexray kirim "LAKI-LAKI"/"PEREMPUAN", plugin memakai
        // "pria"/"wanita" (lihat plugins/tools/nikparser.js).
        const kelamin = String(r.kelamin || "").toUpperCase();
        return {
          valid: true,
          raw: r.nik,
          birthISO,
          gender: kelamin.includes("PEREMPUAN") ? "wanita" : "pria",
          provinceId: Number(r.provinsi?.kode) || 0,
          province: r.provinsi?.nama || "",
          kabupatenKotaId: r.kotakab?.kode || "",
          kecamatanId: r.kecamatan?.kode || "",
          uniqcode: r.nomor_urut || "",
        };
      },
    },
  ]);
}

export { NEXRAY_BASES, FIREFLY };
