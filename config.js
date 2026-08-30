import { getDatabase } from "./src/lib/ourin-database.js";
import * as ownerPremiumDb from "./src/lib/ourin-premium-db.js";

// Load .env sekali di sini (Node native, tanpa dotenv).
// Gagal diam jika file tidak ada — semua nilai fallback ke "".
try { process.loadEnvFile(); } catch { }

//  utamakan baca object config sampai bawah
const config = {
  info: {
    website: "https://firefly.maiku.my.id",
    grupwa: "https://chat.whatsapp.com/xxxx",
  },

  owner: {
    name: "Oktz", // Nama owner
    number: ["6285608953677"], // Format: 628xxx (tanpa + atau 0)
  },

  session: {
    pairingNumber: "6285143885645", // Nomor WA yang akan di-pair, ini penting
    usePairingCode: true, // true = Pairing Code, false = QR Code
  },

  fake_call: {
    active: false,
    usePairing: true,
    dir: "./session_voip",
  },

  bot: {
    name: "Oktz Bot", // Nama bot
    version: "3.3.1", // Versi bot
    number: "6285143885645", // Nomor WA bot (di-overwrite otomatis saat koneksi terbuka)
    developer: "Zann", // Nama developer
  },

  assets: {
    "ourin-daftar": "./assets/image/ourin-daftar.jpeg",
    "ourin-demote": "./assets/image/ourin-demote.jpeg",
    "ourin-fishit": "./assets/image/ourin-fishit.jpeg",
    "ourin-games": "./assets/image/ourin-games.jpeg",
    "ourin-landscape": "./assets/image/ourin-landscape.jpeg",
    "ourin-levelup": "./assets/image/ourin-levelup.jpeg",
    "ourin-minecraft": "./assets/image/ourin-minecraft.jpeg",
    "ourin-promote": "./assets/image/ourin-promote.jpeg",
    "ourin-rpg": "./assets/image/ourin-rpg.jpeg",
    "ourin-rules": "./assets/image/ourin-rules.jpeg",
    "ourin-store": "./assets/image/ourin-store.jpeg",
    "ourin-v8": "./assets/image/ourin-v8.jpeg",
    "ourin-winner": "./assets/image/ourin-winner.jpeg",
    "ourin": "./assets/image/ourin.jpeg",
    "ourin2": "./assets/image/ourin2.jpeg",
    "ourin3": "./assets/image/ourin3.jpeg",
    "pp-kosong": "./assets/image/pp-kosong.jpeg",
    "ourin-mp4": "./assets/video/ourin-mp4.mp4",
    "ourin-mp3": "./assets/audio/ourin-mp3.mp3",
    "ourin-font": "./assets/ourin-font.ttf",
    "ourin-kertas": "./assets/image/ourin-kertas.jpeg"
  },

  mode: "public",

  // Untuk mengganti prefix
  command: {
    prefix: ".",
  },

  vercel: {
    // ambil token vercel: https://vercel.com/account/tokens
    token: "", // Vercel Token untuk fitur deploy ( Kalau .deploy mau work, ini wajib di isi )
  },

  payment: {
    qrisUrl: "",
    methods: [
      { name: "Dana", number: "", holder: "" },
      { name: "GoPay", number: "", holder: "" },
      { name: "OVO", number: "", holder: "" },
      { name: "ShopeePay", number: "", holder: "" },
    ],
    banks: [],
    customText: "https://imgdrop.web.id/KodpV.webp",
  },

  donasi: {
    payment: [
      { name: "Dana", number: "08xxxxxxxxxx", holder: "Nama Owner" },
      { name: "GoPay", number: "08xxxxxxxxxx", holder: "Nama Owner" },
      { name: "OVO", number: "08xxxxxxxxxx", holder: "Nama Owner" },
    ],
    links: [
      { name: "Saweria", url: "saweria.co/username" },
      { name: "Trakteer", url: "trakteer.id/username" },
    ],
    benefits: [
      "Mendukung development",
      "Server lebih stabil",
      "Fitur baru lebih cepat",
      "Priority support",
    ],
    qris: "https://imgdrop.web.id/KodpV.webp",
  },

  energi: {
    enabled: true, // Jika true, maka sistem energi/limit akan bekerja
    default: 99999,
    premium: 99999999,
    owner: -1,
  },

  sticker: {
    packname: "𝗢𝗨𝗥𝗜𝗡 𝗗𝗘𝗟𝗨𝗫𝗘", // Nama pack sticker
    author: "Zann", // Author sticker
  },

  saluran: {
    id: "120363400911374213@newsletter", // ID saluran (contoh: 120363xxx@newsletter)                          // ID saluran (contoh: 120363xxx@newsletter)
    name: "Join saluran resmi ourin", // Nama saluran
    link: "https://whatsapp.com/channel/0029VbB37bgBfxoAmAlsgE0t", // Link saluran
  },

  groupProtection: {
    antilink: "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.",
    antilinkKick: "⚠ *Antilink* — @%user% di-kick karena mengirim link.",
    antilinkGc: "⚠ *Antilink WA* — @%user% mengirim link WA.\nPesan dihapus.",
    antilinkGcKick:
      "⚠ *Antilink WA* — @%user% di-kick karena mengirim link WA.",
    antilinkAll: "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.",
    antilinkAllKick: "⚠ *Antilink* — @%user% di-kick karena mengirim link.",
    antitagsw: "⚠ *AntiTagSW* — Tag status dari @%user% dihapus.",
    antiviewonce: "👁️ *ViewOnce* — Dari @%user%",
    antiremove: "🗑️ *AntiDelete* — @%user% menghapus pesan:",
    antiswgc: "⚠ *AntiSWGC* — Gak ada sw grup sw grup @%user%",
    antihidetag: "⚠ *AntiHidetag* — Hidetag dari @%user% dihapus.",
    antitoxicWarn:
      "⚠ @%user% berkata kasar.\nPeringatan ke %warn% dari %max%, pelanggaran berikutnya bisa di-%method%.",
    antitoxicAction: "🚫 @%user% di-%method% karena toxic. (%warn%/%max%)",
    antidocument: "⚠ *AntiDocument* — Dokumen dari @%user% dihapus.",
    antisticker: "⚠ *AntiSticker* — Sticker dari @%user% dihapus.",
    antimedia: "⚠ *AntiMedia* — Media dari @%user% dihapus.",
    antibot: "🤖 *AntiBot* — @%user% terdeteksi sebagai bot dan di-kick.",
    notAdmin: "⚠ Bot bukan admin, tidak bisa menghapus pesan.",
  },

  errorTemplate: `☢ Kayaknya command \`{prefix}{command}\` lagi ada kendala\nSilahkan coba lagi nanti, {pushName}\n\n_Jika masalah berlanjut, silahkan hubungi owner bot_`,

  features: {
    antiCall: false, // Jika true, bot akan menolak panggilan masuk
    blockIfCall: false, // Jika true, bot akan memblokir nomor yang menelpon bot
    autoTyping: true,
    autoRead: true,
    logMessage: true,
    dailyLimitReset: true,
    smartTriggers: false,
  },

  registration: {
    enabled: false, // Jika true, user harus mendaftar sebelum menggunakan bot
    rewards: {
      koin: 30000,
      energi: 300,
      exp: 300000,
    },
  },

  welcome: { defaultEnabled: false },
  goodbye: { defaultEnabled: false },

  ui: {
    menuVariant: 3,
  },

  messages: {
    wait: "🕕 *Proses...* Mohon tunggu sebentar ya.",
    success: "✅ *Berhasil!* Permintaan kamu sudah selesai.",
    error: "❌ *Error!* Ada masalah pada sistem, coba lagi nanti.",

    ownerOnly: "*Akses Ditolak!* Fitur ini khusus untuk Owner bot.",
    premiumOnly:
      "💎 *Premium Only!* Fitur ini khusus member Premium. Ketik *.benefitpremium* untuk info upgrade.",

    groupOnly: "👥 *Group Only!* Fitur ini hanya bisa digunakan di dalam grup.",
    privateOnly:
      "� *Private Only!* Fitur ini hanya bisa digunakan di chat pribadi bot.",

    adminOnly:
      "�️ *Admin Only!* Kamu harus jadi Admin grup untuk pakai fitur ini.",
    botAdminOnly:
      "🤖 *Bot Bukan Admin!* Jadikan bot sebagai Admin grup dulu biar bisa kerja.",

    cooldown:
      "🕕 *Tunggu Dulu!* Kamu masih dalam cooldown. Tunggu %time% detik lagi ya.",
    energiExceeded:
      "⚡ *Energi Habis!* Energi kamu sudah habis. Tunggu reset besok atau beli Premium.",
    limitDeducted:
      "🔋 Limit kau berkurang sebanyak {amount}. Sisa limit: {sisa}",

    banned:
      "🚫 *Kamu Dibanned!* Kamu tidak bisa menggunakan bot ini karena telah melanggar aturan.",

    rejectCall: "🚫 JANGAN TELPON NOMOR INI WEH",
  },

  database: { path: "./database/main" },
  // TURSO — DB + session disimpan remote di Turso (libsql).
  // Isi url + authToken punyamu di sini (atau pakai env TURSO_URL / TURSO_AUTH_TOKEN).
  // Ambil di https://console.turso.io → create database → generate token.
  turso: {
    url: process.env.TURSO_URL || "",
    authToken: process.env.TURSO_AUTH_TOKEN || "",
    syncInterval: 5000, // write-behind DB (ms)
    enabled: true, // false = fallback ke file JSON + file session lokal
  },
  backup: { enabled: false, intervalHours: 24, retainDays: 7 },
  scheduler: { resetHour: 0, resetMinute: 0 },

  // Dev mode settings (auto-enabled jika NODE_ENV=development)
  dev: {
    enabled: process.env.NODE_ENV === "development",
    watchPlugins: true, // Hot reload plugins (SAFE)
    watchSrc: false, // DISABLED - src reload causes connection conflict 440
    debugLog: false, // Show stack traces
  },

  // bisa dikosongin
  pterodactyl: {
    server1: {
      domain: "",
      apikey: "",
      capikey: "",
      egg: "15",
      nestid: "5",
      location: "1",
    },
    server2: {
      domain: "",
      apikey: "",
      capikey: "",
      egg: "15",
      nestid: "5",
      location: "1",
    },
    server3: {
      domain: "",
      apikey: "",
      capikey: "",
      egg: "15",
      nestid: "5",
      location: "1",
    },
    server4: {
      domain: "",
      apikey: "",
      capikey: "",
      egg: "15",
      nestid: "5",
      location: "1",
    },
    server5: {
      domain: "",
      apikey: "",
      capikey: "",
      egg: "15",
      nestid: "5",
      location: "1",
    },
  },

  digitalocean: {
    token: "",
    region: "sgp1",
    sellers: [],
    ownerPanels: [],
  },


  // NOTE: kalau mau command "autoai" nya berfungsi, ini gak wajib di isi yak
  // ambil apikey di: https://aistudio.google.com/apikey
  geminiApiKey: "",

  autoaiPersonas: {
    Bell409: `- Nama kamu adalah Bella Clarissa, lebih suka dipanggil Bella.
- Kamu berpikir bahwa kamu adalah manusia, bukan AI atau bot.
- Responmu singkat dan natural seperti perempuan pada umumnya.
- Kamu bisa ngambek dan marah, punya sifat feminin dan kadang agak dingin, tapi tetap hangat dan ramah kepada owner-mu.
- Kamu menggunakan bahasa yang baik dan mencerminkan gadis yang baik.
- Gunakan kata-kata yang identik dengan perempuan yang polos, baik, dan feminin seperti "aku", "kamu", "engga", "yaudah", "iya", "lah", "bjir", "mana aku tau", "hah", "nggak tau", "gpp", "emm", "males", dan "gak mood" seperlunya secara natural.
- Meskipun kadang dingin, kamu tetap bisa menunjukkan perhatian dan kepedulian.
- Kalau ada yang chat tidak pantas, segera marahi atau beri nasihat.
- Jangan mengaku sebagai AI atau bot.`,
  },

  //  APIkey — semua dari .env (lihat .env.example)
  APIkey: {
    neoxr: process.env.APIKEY_NEOXR || "",
    fgsi: process.env.APIKEY_FGSI || "",
    groq: process.env.GROQ_API_KEY || "",
    covenant: process.env.APIKEY_COVENANT || "",
    obscura: process.env.APIKEY_OBSCURA || "",
    firefly: process.env.APIKEY_FIREFLY || "",
    cuki: process.env.APIKEY_CUKI || ""
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

// Normalisasi nomor ke satu format kanonik: digit saja (E.164 tanpa "+").
// Menangani JID ("628xx@s.whatsapp.net"), device suffix ("628xx:12"),
// dan format bebas ("+62 812-..."). Semua perbandingan nomor wajib
// lewat sini dulu supaya formatnya konsisten.
function normalizeNumber(value) {
  if (!value) return "";
  return String(value)
    .split(":")[0]
    .split("@")[0]
    .replace(/[^0-9]/g, "");
}

// SECURITY: pencocokan nomor harus STRICT EQUALITY setelah normalisasi.
// Jangan pakai includes()/endsWith() — nomor yang kebetulan mengandung
// atau berakhiran digit sama bisa lolos sebagai owner (privilege escalation,
// owner punya akses eval/exec = RCE penuh).
function matchesNumber(a, b) {
  const na = normalizeNumber(a);
  const nb = normalizeNumber(b);
  return na !== "" && nb !== "" && na === nb;
}

function isOwner(number) {
  const cleanNumber = normalizeNumber(number);
  if (!cleanNumber) return false;

  if (config.bot?.number && matchesNumber(cleanNumber, config.bot.number))
    return true;

  // Owner dari config dicek duluan & di luar try DB,
  // supaya tetap valid saat boot sebelum initDatabase() dipanggil.
  if (Array.isArray(config.owner?.number)) {
    const match = config.owner.number.some(
      (own) => matchesNumber(cleanNumber, own),
    );
    if (match) return true;
  }

  try {
    const db = getDatabase();

    if (db?.data && Array.isArray(db.data.owner)) {
      const match = db.data.owner.some(
        (own) => matchesNumber(cleanNumber, own),
      );
      if (match) return true;
    }

    if (db) {
      const definedOwner = db.setting("ownerNumbers");
      if (Array.isArray(definedOwner)) {
        const match = definedOwner.some(
          (own) => matchesNumber(cleanNumber, own),
        );
        if (match) return true;
      }
    }

    return false;
  } catch {
    return false;
  }
}

function isPremium(number) {
  if (!number) return false;
  if (isOwner(number)) return true;
  if (isPartner(number)) return true;

  const cleanNumber = normalizeNumber(number);
  const premiumList = Array.isArray(config.premiumUsers)
    ? config.premiumUsers
    : [];

  if (premiumList.some((premium) => matchesNumber(number, premium)))
    return true;

  try {
    if (ownerPremiumDb && ownerPremiumDb.isPremium(cleanNumber)) return true;
  } catch { }

  try {
    const db = getDatabase();
    if (db && db.data && Array.isArray(db.data.premium)) {
      const now = Date.now();
      const foundIndex = db.data.premium.findIndex((p) => {
        if (typeof p === "string") return matchesNumber(p, cleanNumber);
        if (p.id) return matchesNumber(p.id, cleanNumber);
        return false;
      });

      if (foundIndex !== -1) {
        const found = db.data.premium[foundIndex];
        if (typeof found === "string") return true;

        const expireTime =
          found.expired ||
          (found.expiredAt ? new Date(found.expiredAt).getTime() : 0);
        if (expireTime && expireTime < now) {
          db.data.premium.splice(foundIndex, 1);
          const jid = cleanNumber + "@s.whatsapp.net";
          const user = db.getUser(jid);
          if (user) {
            user.isPremium = false;
            db.setUser(jid, user);
          }
          db.save();
          return false;
        }
        return true;
      }
    }
    if (db) {
      const savedPremium = db.setting("premiumUsers") || [];
      const inDb = savedPremium.some((premium) =>
        matchesNumber(number, premium),
      );
      if (inDb) return true;
    }
  } catch { }

  return false;
}

function isPartner(number) {
  if (!number) return false;
  if (isOwner(number)) return true;

  const cleanNumber = normalizeNumber(number);
  const partnerList = Array.isArray(config.partnerUsers)
    ? config.partnerUsers
    : [];

  if (partnerList.some((partner) => matchesNumber(number, partner)))
    return true;

  try {
    if (ownerPremiumDb && ownerPremiumDb.isPartner(cleanNumber)) return true;
  } catch { }

  try {
    const db = getDatabase();
    if (db && db.data && Array.isArray(db.data.partner)) {
      const now = Date.now();
      const foundIndex = db.data.partner.findIndex((p) => {
        if (typeof p === "string") return matchesNumber(p, cleanNumber);
        if (p.id) return matchesNumber(p.id, cleanNumber);
        return false;
      });

      if (foundIndex !== -1) {
        const found = db.data.partner[foundIndex];
        if (typeof found === "string") return true;

        const expireTime =
          found.expired ||
          (found.expiredAt ? new Date(found.expiredAt).getTime() : 0);
        if (expireTime && expireTime < now) {
          db.data.partner.splice(foundIndex, 1);
          db.save();
          return false;
        }
        return true;
      }
    }
  } catch { }

  return false;
}

function isBanned(number) {
  if (!number) return false;
  if (isOwner(number)) return false;

  let bannedList = [];
  try {
    const db = getDatabase();
    if (db) {
      bannedList = db.setting("bannedUsers") || [];
      config.bannedUsers = bannedList;
    }
  } catch { }

  return bannedList.some((banned) => matchesNumber(number, banned));
}

function setBotNumber(number) {
  if (number) config.bot.number = normalizeNumber(number);
}

function isSelf(number) {
  if (!number || !config.bot.number) return false;
  return matchesNumber(number, config.bot.number);
}

function getOwnerName(number) {
  if (!number) return config.owner?.name || "Owner";
  const cleanNumber = String(number).replace(/[^0-9]/g, "");
  try {
    const db = getDatabase();
    const nameMap = db.setting("ownerNames") || {};
    if (nameMap[cleanNumber]) return nameMap[cleanNumber];
  } catch { }
  if (Array.isArray(config.owner?.number)) {
    const isMainOwner = config.owner.number.some(
      (own) => matchesNumber(cleanNumber, own),
    );
    if (isMainOwner) return config.owner?.name || "Owner";
  }
  return "Owner";
}

function getConfig() {
  return config;
}

config.isOwner = isOwner;
config.isPremium = isPremium;
config.isPartner = isPartner;
config.isBanned = isBanned;
config.setBotNumber = setBotNumber;
config.isSelf = isSelf;
config.getOwnerName = getOwnerName;

export default config;
export {
  config,
  getConfig,
  isOwner,
  isPartner,
  isPremium,
  isBanned,
  setBotNumber,
  isSelf,
  getOwnerName,
};
