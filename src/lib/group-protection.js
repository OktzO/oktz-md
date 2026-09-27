import { downloadMediaMessage } from "onigis";
import {
  isLid,
  lidToJid,
  lidToJidSafe,
  isSameParticipant,
  normalizeComparableJid,
} from "./lid.js";
import { adminFlagsFor } from "./serialize.js";
import { evictOldestOverCap } from "./cache-cap.js";
import config from "../../config.js";
const messageCache = new Map();
const CACHE_EXPIRY = 10 * 60 * 1000;
const CACHE_MAX_SIZE = 500;

// Precompiled placeholder regex per replacement key — dulu new RegExp per
// placeholder per call di hot path proteksi pesan.
const _gpReplaceCache = new Map();
const _gpRegex = (k) => {
  let r = _gpReplaceCache.get(k);
  if (!r) {
    r = new RegExp(`%${k}%`, "g");
    _gpReplaceCache.set(k, r);
  }
  return r;
};
function gpMsg(key, replacements = {}) {
  const defaults = {
    antilink: "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.",
    antilinkKick: "⚠ *Antilink* — @%user% di-kick karena mengirim link.",
    antilinkGc: "⚠ *Antilink WA* — @%user% mengirim link WA.\nPesan dihapus.",
    antilinkGcKick:
      "⚠ *Antilink WA* — @%user% di-kick karena mengirim link WA.",
    antilinkAll: "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.",
    antilinkAllKick: "⚠ *Antilink* — @%user% di-kick karena mengirim link.",
    antitagsw: "⚠ *AntiTagSW* — Tag status dari @%user% dihapus.",
    antiswgc: "⚠ *AntiSWGC* — SW group type *%type%* dari @%user% dihapus.",
    antijudol:
      "⚠ *AntiJudol* — @%user% terdeteksi kirim konten judol.\nPesan dihapus.",
    antijudolKick: "⚠ *AntiJudol* — @%user% di-kick karena kirim konten judol.",
    antiphising:
      "⚠ *AntiPhising* — @%user% terdeteksi kirim konten phising.\nPesan dihapus.",
    antiphisingKick:
      "⚠ *AntiPhising* — @%user% di-kick karena kirim konten phising.",
    anticustom:
      "⚠ *AntiCustom* — @%user% melanggar rule custom *%rule%*.\nPesan dihapus.",
    anticustomKick:
      "⚠ *AntiCustom* — @%user% di-kick karena melanggar rule custom *%rule%*.",
    antiviewonce: "👁️ *ViewOnce* — Dari @%user%",
    antiremove: "🗑️ *AntiDelete* — @%user% menghapus pesan:",
    antihidetag: "⚠ *AntiHidetag* — Hidetag dari @%user% dihapus.",
    notAdmin: "⚠ Bot bukan admin, tidak bisa menghapus pesan.",
  };
  let text = config.groupProtection?.[key] || defaults[key] || "";
  for (const [k, v] of Object.entries(replacements)) {
    text = text.replace(_gpRegex(k), v);
  }
  return text;
}

function cacheMessage(key, message, content) {
  messageCache.set(key, {
    message,
    content,
    timestamp: Date.now(),
  });

  if (messageCache.size > CACHE_MAX_SIZE) {
    const entries = [...messageCache.entries()].sort(
      (a, b) => a[1].timestamp - b[1].timestamp,
    );
    const toDelete = entries.slice(0, entries.length - CACHE_MAX_SIZE);
    for (const [k] of toDelete) messageCache.delete(k);
  }
}

function getCachedMessage(key) {
  const cached = messageCache.get(key);
  if (!cached) return null;
  if (Date.now() - cached.timestamp > CACHE_EXPIRY) {
    messageCache.delete(key);
    return null;
  }
  return cached;
}

function deleteCachedMessage(key) {
  messageCache.delete(key);
}

const LINK_REGEX =
  /(?:https?:\/\/)?(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9()@:%_\+.~#?&//=]*/gi;
const WA_LINK_REGEX =
  /(?:https?:\/\/)?(?:www\.)?(?:chat\.whatsapp\.com|wa\.me|api\.whatsapp\.com|whatsapp\.com\/channel|whatsapp\.com\/catalog|call\.whatsapp\.com)\/[A-Za-z0-9+_\-]{4,}/gi;
const LEET_CHAR_MAP = Object.freeze({
  0: "o",
  1: "i",
  2: "z",
  3: "e",
  4: "a",
  5: "s",
  6: "g",
  7: "t",
  8: "b",
  9: "g",
  "@": "a",
  $: "s",
  "!": "i",
  "+": "t",
});
const JUDOL_PATTERNS = [
  { pattern: /\bjud[iol]{1,4}\b/i, score: 3, label: "judi" },
  { pattern: /\bslot\b/i, score: 3, label: "slot" },
  { pattern: /\bgacor\b/i, score: 3, label: "gacor" },
  { pattern: /\bslot\s+gacor\b/i, score: 4, label: "slot gacor" },
  { pattern: /\bmax\s?win\b/i, score: 3, label: "maxwin" },
  { pattern: /\bscatter\b/i, score: 2, label: "scatter" },
  { pattern: /\btogel\b/i, score: 3, label: "togel" },
  { pattern: /\bcasino\b/i, score: 3, label: "casino" },
  { pattern: /\brtp\b/i, score: 2, label: "rtp" },
  { pattern: /\bpragmatic\b/i, score: 2, label: "pragmatic" },
  { pattern: /\bpg\s?soft\b/i, score: 2, label: "pgsoft" },
  {
    pattern: /\b(habanero|joker|spadegaming|microgaming)\b/i,
    score: 2,
    label: "provider",
  },
  { pattern: /\bjackpot\b/i, score: 2, label: "jackpot" },
  { pattern: /\bfree\s?spin\b/i, score: 2, label: "free spin" },
  { pattern: /\bspin\s?gratis\b/i, score: 2, label: "spin gratis" },
  { pattern: /\bpola\s?slot\b/i, score: 2, label: "pola slot" },
  { pattern: /\bdeposit\b/i, score: 1, label: "deposit" },
  { pattern: /\bdepo\b/i, score: 1, label: "depo" },
  { pattern: /\bwithdraw\b/i, score: 1, label: "withdraw" },
  { pattern: /\bwd\b/i, score: 1, label: "wd" },
  { pattern: /bonus\s+new\s+member/i, score: 2, label: "bonus new member" },
  {
    pattern: /bonus\s+(member|new\s*member)/i,
    score: 2,
    label: "bonus member",
  },
  { pattern: /link\s+alternatif/i, score: 2, label: "link alternatif" },
  { pattern: /bandar\s+(slot|togel|judi)/i, score: 2, label: "bandar" },
  { pattern: /\bbet\s?(kecil|besar)?\b/i, score: 1, label: "bet" },
  { pattern: /\btaruhan\b/i, score: 2, label: "taruhan" },
];
const JUDOL_COMPACT_KEYWORDS = [
  { keyword: "judi", score: 3, label: "judi" },
  { keyword: "judol", score: 3, label: "judol" },
  { keyword: "slot", score: 3, label: "slot" },
  { keyword: "gacor", score: 3, label: "gacor" },
  { keyword: "slotgacor", score: 4, label: "slot gacor" },
  { keyword: "maxwin", score: 3, label: "maxwin" },
  { keyword: "scatter", score: 2, label: "scatter" },
  { keyword: "togel", score: 3, label: "togel" },
  { keyword: "casino", score: 3, label: "casino" },
  { keyword: "rtp", score: 2, label: "rtp" },
  { keyword: "pragmatic", score: 2, label: "pragmatic" },
  { keyword: "pgsoft", score: 2, label: "pgsoft" },
  { keyword: "jackpot", score: 2, label: "jackpot" },
  { keyword: "freespin", score: 2, label: "free spin" },
  { keyword: "spingratis", score: 2, label: "spin gratis" },
  { keyword: "polaslot", score: 2, label: "pola slot" },
  { keyword: "bonusmember", score: 2, label: "bonus member" },
  { keyword: "bonusnewmember", score: 2, label: "bonus new member" },
  { keyword: "linkalternatif", score: 2, label: "link alternatif" },
  { keyword: "bandarslot", score: 2, label: "bandar slot" },
  { keyword: "bandartogel", score: 2, label: "bandar togel" },
  { keyword: "bandarjudi", score: 2, label: "bandar judi" },
];
const PHISHING_PATTERNS = [
  { pattern: /klik\s+(link|tautan)/i, score: 2, label: "klik link" },
  { pattern: /(tap|tekan)\s+(link|tautan)/i, score: 2, label: "tap link" },
  {
    pattern: /verifikasi\s+(akun|nomor|rekening|wallet|email|data)/i,
    score: 3,
    label: "verifikasi",
  },
  {
    pattern:
      /(login|log\s?in|signin|sign\s?in)\s+(disini|di\s+sini|sekarang|ulang)/i,
    score: 3,
    label: "login",
  },
  { pattern: /masuk\s+(disini|di\s+sini)/i, score: 2, label: "masuk" },
  {
    pattern: /\b(otp|pin|password|passcode|kode\s?otp|kode\s?verifikasi)\b/i,
    score: 2,
    label: "credential",
  },
  {
    pattern: /akun\s+(dibatasi|terblokir|bermasalah|expired|kedaluwarsa)/i,
    score: 2,
    label: "akun",
  },
  {
    pattern: /klaim\s+(hadiah|saldo|voucher|bonus)/i,
    score: 2,
    label: "klaim",
  },
  {
    pattern: /(saldo|dana|ovo|gopay|shopeepay)\s+(gratis|kaget|masuk)/i,
    score: 2,
    label: "saldo",
  },
  { pattern: /hadiah\s+(gratis|menarik|langsung)/i, score: 1, label: "hadiah" },
  { pattern: /cek\s+(resi|paket|rekening)/i, score: 1, label: "cek" },
  { pattern: /reset\s+(password|akun)/i, score: 3, label: "reset" },
  { pattern: /pulihkan\s+akun/i, score: 3, label: "pulihkan akun" },
  {
    pattern: /update\s+(akun|rekening|wallet|data)/i,
    score: 2,
    label: "update akun",
  },
  {
    pattern: /konfirmasi\s+(akun|identitas|rekening|data)/i,
    score: 2,
    label: "konfirmasi",
  },
  { pattern: /isi\s+form(ulir)?/i, score: 2, label: "form" },
  { pattern: /akses\s+(sekarang|link)/i, score: 1, label: "akses" },
  { pattern: /hadiah\s+menanti/i, score: 1, label: "hadiah menanti" },
];
const PHISHING_COMPACT_KEYWORDS = [
  { keyword: "kliklink", score: 2, label: "klik link" },
  { keyword: "kliktautan", score: 2, label: "klik tautan" },
  { keyword: "taplink", score: 2, label: "tap link" },
  { keyword: "tekanlink", score: 2, label: "tekan link" },
  { keyword: "verifikasiakun", score: 3, label: "verifikasi akun" },
  { keyword: "verifikasinomor", score: 3, label: "verifikasi nomor" },
  { keyword: "verifikasirekening", score: 3, label: "verifikasi rekening" },
  { keyword: "verifikasiwallet", score: 3, label: "verifikasi wallet" },
  { keyword: "loginsekarang", score: 3, label: "login sekarang" },
  { keyword: "logindisini", score: 3, label: "login disini" },
  { keyword: "masukdisini", score: 2, label: "masuk disini" },
  { keyword: "kodeotp", score: 2, label: "kode otp" },
  { keyword: "kodeverifikasi", score: 2, label: "kode verifikasi" },
  { keyword: "resetpassword", score: 3, label: "reset password" },
  { keyword: "resetakun", score: 3, label: "reset akun" },
  { keyword: "pulihkanakun", score: 3, label: "pulihkan akun" },
  { keyword: "klaimhadiah", score: 2, label: "klaim hadiah" },
  { keyword: "klaimsaldo", score: 2, label: "klaim saldo" },
  { keyword: "klaimbonus", score: 2, label: "klaim bonus" },
  { keyword: "saldogratis", score: 2, label: "saldo gratis" },
  { keyword: "saldokaget", score: 2, label: "saldo kaget" },
  { keyword: "updateakun", score: 2, label: "update akun" },
  { keyword: "updaterekening", score: 2, label: "update rekening" },
  { keyword: "konfirmasiakun", score: 2, label: "konfirmasi akun" },
  { keyword: "konfirmasiidentitas", score: 2, label: "konfirmasi identitas" },
  { keyword: "isiform", score: 2, label: "isi form" },
];
const SHORTENER_PATTERN =
  /(?:bit\.ly|tinyurl\.com|cutt\.ly|s\.id|t\.co|goo\.gl|is\.gd|rebrand\.ly|linktr\.ee)\//i;
const IP_URL_PATTERN =
  /(?:https?:\/\/)?(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?(?:\/|\b)/i;
const AT_IN_URL_PATTERN = /https?:\/\/[^\s/]+@[^\s]+/i;
const PUNYCODE_PATTERN = /xn--[a-z0-9-]+/i;

/**
 * Satu sumber kebenaran untuk cek admin, dipakai juga oleh serialize.
 *
 * Bug lama: helper ini hanya membaca `p.jid || p.id` (+ `p.lid`). library
 * mengirim participant berbeda per addressing_mode: grup LID memberi
 * { id: LID, phoneNumber: PN }, jadi (a) bot admin tidak pernah terdeteksi
 * (semua proteksi diam / balas "notAdmin") dan (b) pesan admin ikut
 * dihapus. Sekarang semua varian nomor dibandingkan dan pemanggil wajib
 * mengirim KANDIDAT JID (raw addressing + versi resolved), bukan digits.
 *
 * @param {Object[]} participants
 * @param {string|string[]} senderJid  - mis. [m.key?.participant, m.sender]
 */
function isAdminCheck(participants, senderJid) {
  return adminFlagsFor(participants, senderJid, []).isAdmin;
}

/** @param {string|string[]} botJids - mis. [sock.user.id, sock.user.lid] */
function isBotAdminCheck(participants, botJids) {
  return adminFlagsFor(participants, [], botJids).isBotAdmin;
}

/**
 * Key revoke untuk pesan yang BARANG DATANG: pakai key asli dari server.
 * Di grup addressing_mode=lid pesan disimpan dengan participant LID, jadi
 * participant versi PN (hasil resolve m.sender) tidak dicocokkan server —
 * pesan tampak tidak terhapus tanpa error.
 */
function revokeKey(m) {
  return m.key || {
    remoteJid: m.chat,
    fromMe: false,
    id: m.id,
    participant: m.sender,
  };
}

function normalizeProtectionMode(mode, fallback = "remove") {
  const value = String(mode || fallback).toLowerCase();
  if (["kick", "remove", "delete"].includes(value)) {
    return value === "delete" ? "remove" : value;
  }
  return fallback;
}

function getProtectionText(m) {
  return String(m.body || m.text || "").trim();
}

function resolveMessageSenderJid(key, sock) {
  const fallback = key?.fromMe ? sock.user?.id : key?.remoteJid;
  return normalizeComparableJid(key?.participant || fallback);
}

function normalizeProtectionText(text) {
  const value = String(text || "")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();

  const mapped = [...value]
    .map((char) => LEET_CHAR_MAP[char] || char)
    .join("")
    .replace(/[^a-z0-9]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();

  return mapped.replace(/\b(?:[a-z0-9]\s+){2,}[a-z0-9]\b/g, (fragment) =>
    fragment.replace(/\s+/g, ""),
  );
}

function buildProtectionTextVariants(text) {
  const raw = String(text || "")
    .toLowerCase()
    .trim();
  const normalized = normalizeProtectionText(text);
  const compact = normalized.replace(/[^a-z0-9]+/g, "");
  return [...new Set([raw, normalized, compact].filter(Boolean))];
}

function summarizeScoreHits(hits = []) {
  const uniqueHits = new Map();

  for (const hit of hits) {
    if (!hit?.label) continue;
    const current = uniqueHits.get(hit.label);
    if (!current || current.score < hit.score) {
      uniqueHits.set(hit.label, hit);
    }
  }

  const items = [...uniqueHits.values()];
  return {
    score: items.reduce((total, item) => total + (item.score || 0), 0),
    matches: items.map((item) => item.label),
  };
}

function scorePatterns(text, patterns = []) {
  const variants = Array.isArray(text)
    ? text.filter(Boolean)
    : [text].filter(Boolean);
  const hits = [];

  for (const item of patterns) {
    for (const variant of variants) {
      item.pattern.lastIndex = 0;
      if (item.pattern.test(variant)) {
        hits.push({
          label: item.label || item.pattern.source,
          score: item.score || 1,
        });
        break;
      }
    }
  }

  return summarizeScoreHits(hits);
}

function scoreKeywords(text, keywords = []) {
  const source = String(text || "");
  const hits = [];

  for (const item of keywords) {
    if (source.includes(item.keyword)) {
      hits.push({ label: item.label || item.keyword, score: item.score || 1 });
    }
  }

  return summarizeScoreHits(hits);
}

function detectJudol(text) {
  if (!text) return { matched: false, score: 0, matches: [] };
  const variants = buildProtectionTextVariants(text);
  const compact = variants[variants.length - 1] || "";
  const { hasLink } = containsAnyLink(text);
  const patternResult = scorePatterns(variants, JUDOL_PATTERNS);
  const compactResult = scoreKeywords(compact, JUDOL_COMPACT_KEYWORDS);
  const result = summarizeScoreHits([
    ...patternResult.matches.map((label) => ({ label, score: 0 })),
    ...compactResult.matches.map((label) => ({ label, score: 0 })),
  ]);
  const totalScore = patternResult.score + compactResult.score;
  const matched = totalScore >= 3 || (hasLink && totalScore >= 2);
  return { matched, hasLink, score: totalScore, matches: result.matches };
}

function detectPhishing(text) {
  if (!text) return { matched: false, score: 0, matches: [] };
  const variants = buildProtectionTextVariants(text);
  const compact = variants[variants.length - 1] || "";
  const { hasLink } = containsAnyLink(text);
  const patternResult = scorePatterns(variants, PHISHING_PATTERNS);
  const compactResult = scoreKeywords(compact, PHISHING_COMPACT_KEYWORDS);
  let score = patternResult.score + compactResult.score;
  const matches = [
    ...new Set([...patternResult.matches, ...compactResult.matches]),
  ];

  if (SHORTENER_PATTERN.test(text)) {
    score += 2;
    matches.push("shortener");
  }
  if (IP_URL_PATTERN.test(text)) {
    score += 2;
    matches.push("ip-url");
  }
  if (AT_IN_URL_PATTERN.test(text)) {
    score += 2;
    matches.push("url-userinfo");
  }
  if (PUNYCODE_PATTERN.test(text)) {
    score += 2;
    matches.push("punycode");
  }

  const matched = (hasLink && score >= 2) || score >= 3;
  return { matched, hasLink, score, matches: [...new Set(matches)] };
}

/**
 * Ambil contextInfo dari node. Semua msgType WA punya contextInfo, tapi
 * posisinya bisa satu lapis dalam, jadi dibaca langsung dari node.
 */
function contextInfoOf(node) {
  if (!node || typeof node !== "object") return null;
  if (node.contextInfo && typeof node.contextInfo === "object") {
    return node.contextInfo;
  }
  return null;
}

/**
 * SWGC = "orang nge-share status ke grup", bukan pesan biasa.
 *
 * Dua aturan yang WAJIB dijaga:
 *
 * 1. JANGAN rekursif tanpa batas. `quotedMessage` sering berisi SWGC
 *    (user mengutip status lalu membalas). Kalau ikut dicek, balasan biasa
 *    ikut terhapus — persis keluhan "yang bukan SWGC malah kena".
 *
 * 2. Ikuti wrapper yang benar-benar ada di wire. onigis tidak meng-unwrap
 *    `groupStatusMessageV2Extension`, jadi bentuk itu wajib dicek langsung;
 *    sebelumnya tidak ada yang memeriksanya sehingga "kadang lolos".
 */
function findSwGcType(node, path = "message", seen = new WeakSet(), depth = 0) {
  // Batas kedalaman: pesan WA nyata paling dalam ~4 lapis (viewOnce →
  // extendedText → contextInfo). Sisanya bukan mention, hanya pemborosan.
  if (!node || typeof node !== "object" || depth > 6) return null;
  if (seen.has(node)) return null;
  seen.add(node);

  if (node.groupStatusMessage) {
    return `${path}.groupStatusMessage`;
  }
  if (node.groupStatusMessageV2) {
    return `${path}.groupStatusMessageV2`;
  }
  if (node.groupStatusMessageV2Extension) {
    return `${path}.groupStatusMessageV2Extension`;
  }
  if (node.groupStatusMentionMessage) {
    return `${path}.groupStatusMentionMessage`;
  }
  if (node.groupMentionedMessage) {
    return `${path}.groupMentionedMessage`;
  }
  if (node.statusMentionMessage) {
    return `${path}.statusMentionMessage`;
  }

  const ci = contextInfoOf(node);
  if (ci && Array.isArray(ci.groupMentions) && ci.groupMentions.length > 0) {
    return `${path}.contextInfo.groupMentions`;
  }

  for (const [key, value] of Object.entries(node)) {
    if (!value || typeof value !== "object") continue;
    // Jangan pernah masuk ke isi pesan yang sedang dikutip.
    if (key === "quotedMessage") continue;
    const found = findSwGcType(value, `${path}.${key}`, seen, depth + 1);
    if (found) return found;
  }

  return null;
}

function detectSwGcType(rawMsg) {
  const msg = rawMsg?.message;
  if (!msg) return null;

  return findSwGcType(msg, "message");
}

/**
 * Gate tunggal untuk semua pemanggil (connection.js + handler.js).
 * Sebelumnya connection.js punya daftar flat sendiri yang tidak sinkron
 * dengan detectSwGcType — dua sumber kebenaran yang bisa berbeda.
 */
function isSwGcCandidate(rawMsg) {
  return Boolean(detectSwGcType(rawMsg));
}

// ---------------------------------------------------------------------------
// AntiCustom: pola datang dari admin grup (`regex:` di plugins/group/anticustom.js).
//
// Seluruh blok ini bounding: `matchCustomRule()` jalan sinkron di event loop
// untuk SETIAP pesan, jadi pola yang bisa backtrack eksponensial membekukan
// seluruh bot (bukan cuma grup itu). Dua lapis pertahanan:
//
//   1. Analisis struktural (analyzeCustomPattern) menolak pola dengan
//      kuantifier bersarang / alternasi overlap di bawah kuantifier /
//      kuantifier bertumpuk yang bisa overlap. Dipakai juga saat instalasi.
//   2. Batas kerja per pesan: panjang subjek dipotong, jumlah rule dibatasi,
//      budget waktu total, dan RegExp di-precompile + di-cache (dulu
//      `new RegExp` ulang di tiap pesan).
//
// Aturan tidak aman yang sudah menempel di DB sebelum patch tetap inert:
// performanya dicek ulang tiap kali pola pertama kali dipakai, bukan hanya
// saat instalasi.
// ---------------------------------------------------------------------------

const CUSTOM_MAX_PATTERN_LEN = 120;
const CUSTOM_MAX_SUBJECT_LEN = 512;
const CUSTOM_MAX_RULES = 50;
const CUSTOM_MAX_UNBOUNDED_QUANTIFIERS = 6;
const CUSTOM_MATCH_BUDGET_MS = 20;
const CUSTOM_REGEX_CACHE_CAP = 200;

const CHAR_SET_ANY = Object.freeze({ any: true, chars: null, ws: "maybe" });
const CHAR_SET_NONE = Object.freeze({ any: false, chars: new Set(), ws: "never" });
// `\S` = "bukan whitespace": satu-satunya kelas yang tidak akan pernah
// overlap dengan `\s`.
const CHAR_SET_ANY_WITHOUT_SPACE = Object.freeze({
  any: true,
  chars: null,
  ws: "never",
});

const WHITESPACE_CHARS = new Set(
  " \t\n\r\f\v\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000\ufeff".split(
    "",
  ),
);

// `ws`: apakah set ini bisa kena karakter whitespace.
//   "only"  = hanya whitespace (contoh: `\s`)
//   "never" = tidak pernah whitespace (contoh: `\S`)
//   "maybe" = tidak diketahui / semua karakter (contoh: `.`, `[^a]`, `\D`)
function wsModeOf(chars) {
  let hasSpace = false;
  let hasOther = false;
  for (const char of chars) {
    if (WHITESPACE_CHARS.has(char)) hasSpace = true;
    else hasOther = true;
  }
  if (!hasSpace) return "never";
  if (!hasOther) return "only";
  return "maybe";
}

function charSetOf(chars) {
  const set = new Set(chars);
  return { any: false, chars: set, ws: wsModeOf(set) };
}

// `i` aktif: satu literal bisa cocok dengan huruf besar/kecilnya.
function literalSet(char) {
  const lower = char.toLowerCase();
  const upper = char.toUpperCase();
  return charSetOf(lower === upper ? [char] : [lower, upper]);
}

function setUnion(target, other) {
  if (other.any) {
    target.any = true;
    target.ws = "maybe";
    return target;
  }
  for (const char of other.chars) target.chars.add(char);
  target.ws = wsModeOf(target.chars);
  return target;
}

// Dua set dianggap overlap kalau ada teks yang bisa dicocokkan keduanya.
function setsOverlap(a, b) {
  if (a.any && b.any) return true;
  if (a.any) {
    if (a.ws === "maybe") return true;
    if (a.ws === "only") return b.ws !== "never";
    return b.ws !== "only";
  }
  if (b.any) {
    if (b.ws === "maybe") return true;
    if (b.ws === "only") return a.ws !== "never";
    return a.ws !== "only";
  }
  for (const char of a.chars) {
    if (b.chars.has(char)) return true;
  }
  return false;
}

function setIsEmpty(set) {
  return !set.any && set.chars.size === 0;
}

const SHORTHAND_SETS = {
  d: charSetOf("0123456789"),
  w: charSetOf("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_"),
  s: charSetOf(WHITESPACE_CHARS),
  S: CHAR_SET_ANY_WITHOUT_SPACE,
};

const CONTROL_ESCAPES = {
  n: "\n",
  r: "\r",
  t: "\t",
  f: "\f",
  v: "\v",
  0: "\0",
};

function readEscape(src, start) {
  const char = src[start + 1];
  if (char === undefined) {
    return { kind: "char", set: charSetOf([]), next: start + 1, broken: true };
  }
  if (char >= "1" && char <= "9") {
    return { kind: "backref", set: CHAR_SET_ANY, next: start + 2 };
  }
  if (SHORTHAND_SETS[char]) {
    return { kind: "char", set: SHORTHAND_SETS[char], next: start + 2 };
  }
  if (char === "D" || char === "W" || char === "p" || char === "P") {
    // `\D`/`\W` masih bisa cocok dengan whitespace, jadi tetap "maybe".
    return { kind: "char", set: CHAR_SET_ANY, next: start + 2 };
  }
  if (char === "b" || char === "B") {
    // Assertion, bukan karakter: `\w+\b` itu idiom aman dan harus tetap lolos.
    return { kind: "assert", set: CHAR_SET_NONE, next: start + 2 };
  }
  if (char === "x" && /^[0-9a-fA-F]{2}/.test(src.slice(start + 2, start + 4))) {
    return {
      kind: "char",
      set: literalSet(String.fromCharCode(parseInt(src.slice(start + 2, start + 4), 16))),
      next: start + 4,
    };
  }
  if (char === "u" && src[start + 2] === "{") {
    const close = src.indexOf("}", start + 3);
    const body = close === -1 ? "" : src.slice(start + 3, close);
    if (close !== -1 && /^[0-9a-fA-F]+$/.test(body)) {
      return {
        kind: "char",
        set: literalSet(String.fromCodePoint(parseInt(body, 16))),
        next: close + 1,
      };
    }
  }
  if (char === "u" && /^[0-9a-fA-F]{4}/.test(src.slice(start + 2, start + 6))) {
    return {
      kind: "char",
      set: literalSet(String.fromCharCode(parseInt(src.slice(start + 2, start + 6), 16))),
      next: start + 6,
    };
  }
  return { kind: "char", set: literalSet(CONTROL_ESCAPES[char] ?? char), next: start + 2 };
}

function readCharClass(src, start) {
  let index = start + 1;
  let negated = false;
  if (src[index] === "^") {
    negated = true;
    index += 1;
  }

  const set = charSetOf([]);
  let first = true;
  let closed = false;

  while (index < src.length) {
    const char = src[index];

    if (char === "]" && !first) {
      closed = true;
      index += 1;
      break;
    }
    first = false;

    if (char === "\\") {
      const escape = readEscape(src, index);
      if (escape.kind === "backref") {
        return { set: CHAR_SET_ANY, next: index + 2, broken: true };
      }
      setUnion(set, escape.set);
      index = escape.next;
      continue;
    }

    // POSIX class gaya `[[:alpha:]]` -> Perl regex, tidak didukung RegExp JS.
    if (char === "[" && src[index + 1] === ":") {
      const close = src.indexOf("]", index);
      index = close === -1 ? src.length : close + 1;
      set.any = true;
      continue;
    }

    const next = src[index + 1];
    if (next === "-" && src[index + 2] !== undefined && src[index + 2] !== "]") {
      const tail = src[index + 2];
      if (tail === "\\") {
        setUnion(set, literalSet(char));
        setUnion(set, literalSet("-"));
        index += 2;
        continue;
      }
      const from = char.codePointAt(0);
      const to = tail.codePointAt(0);
      if (to < from) {
        set.any = true;
        index += 3;
        continue;
      }
      for (let code = from; code <= to; code += 1) {
        set.chars.add(String.fromCodePoint(code));
      }
      index += 3;
      continue;
    }

    set.chars.add(char);
    if (char.toLowerCase() !== char.toUpperCase()) {
      set.chars.add(char.toLowerCase());
      set.chars.add(char.toUpperCase());
    }
    index += 1;
  }

  // `[abc` tanpa penutup: `new RegExp` akan throw, rule jadi inert. analysed
  // sebagai "apa saja" supaya tidak ada jalur aman yang lolos.
  if (!closed) return { set: CHAR_SET_ANY, next: index, broken: true };
  return { set: negated ? CHAR_SET_ANY : set, next: index, broken: false };
}

function readQuantifier(src, start) {
  const char = src[start];
  if (char === "*" || char === "+") {
    return { min: char === "*" ? 0 : 1, max: Infinity, next: start + 1 };
  }
  if (char === "?") {
    return { min: 0, max: 1, next: start + 1 };
  }
  if (char === "{") {
    const match = /^\{(\d+)(,(\d*))?\}/.exec(src.slice(start));
    if (match) {
      const min = Number(match[1]);
      const max = match[2] === undefined ? min : match[3] === "" ? Infinity : Number(match[3]);
      return { min, max, next: start + match[0].length };
    }
  }
  return null;
}

function readAtom(src, start) {
  const char = src[start];

  if (char === "\\") return { ...readEscape(src, start), kind: "char" };
  if (char === "[") {
    const parsed = readCharClass(src, start);
    return { kind: "char", set: parsed.set, next: parsed.next, broken: parsed.broken };
  }
  if (char === ".") {
    return { kind: "char", set: CHAR_SET_ANY, next: start + 1 };
  }
  if (char === "^" || char === "$") {
    return { kind: "assert", set: CHAR_SET_NONE, next: start + 1 };
  }
  if (char === "(") return readGroupAtom(src, start);
  if (char === ")" || char === "|" || char === "]") {
    return { kind: "broken", set: CHAR_SET_NONE, next: start + 1, broken: true };
  }
  return { kind: "char", set: literalSet(char), next: start + 1 };
}

function findGroupEnd(src, start) {
  let depth = 0;
  let index = start;
  let inClass = false;

  while (index < src.length) {
    const char = src[index];
    if (inClass) {
      if (char === "\\") index += 1;
      else if (char === "]") inClass = false;
      index += 1;
      continue;
    }
    if (char === "\\") {
      index += 2;
      continue;
    }
    if (char === "[") {
      inClass = true;
      index += 1;
      continue;
    }
    if (char === "(") depth += 1;
    else if (char === ")") {
      depth -= 1;
      if (depth === 0) return index;
    }
    index += 1;
  }
  return -1;
}

function readGroupAtom(src, start) {
  let inner = start + 1;
  if (src[inner] === "?") {
    // (?:...) (?=...) (?!...) (?<=...) (?<!...) (?<name>...) (?ims-...)
    if (src[inner + 1] === "<" && src[inner + 2] !== "=" && src[inner + 2] !== "!") {
      const close = src.indexOf(">", inner + 2);
      inner = close === -1 ? inner + 1 : close + 1;
    } else {
      inner += 2;
    }
  }

  const end = findGroupEnd(src, start);
  if (end === -1) {
    return {
      kind: "broken",
      set: CHAR_SET_ANY,
      next: src.length,
      broken: true,
      analysis: EMPTY_GROUP_ANALYSIS,
    };
  }

  const analysis = analyzeAlternation(src.slice(inner, end));
  return {
    kind: "group",
    set: analysis.first,
    next: end + 1,
    broken: analysis.broken,
    analysis,
  };
}

const EMPTY_GROUP_ANALYSIS = {
  first: CHAR_SET_ANY,
  broken: true,
  reason: null,
  hasQuantifier: false,
  canMatchEmpty: true,
  overlappingBranches: false,
  hasUnboundedQuantifier: false,
  singleChar: false,
  unbounded: 0,
};

// Panjang run literal berurutan yang dimulai di `start`. Dipakai supaya
// `.*anjing` (literal panjang setelah kuantifier) tidak dianggap ambigu,
// sementara `.*a` (satu karakter) tetap dianggap ambigu.
function literalRunLength(src, start) {
  if (src[start] === "\\") return 1;
  let index = start;
  while (index < src.length) {
    const char = src[index];
    if (
      char === "[" ||
      char === "(" ||
      char === ")" ||
      char === "|" ||
      char === "." ||
      char === "^" ||
      char === "$" ||
      char === "*" ||
      char === "+" ||
      char === "?"
    ) {
      break;
    }
    if (readQuantifier(src, index + 1)) break;
    index += 1;
  }
  return index - start;
}

/**
 * Analisis satu cabang (tanpa `|` di level teratas) menjadi daftar item.
 * Item = satu atom + kuantifier-nya.
 */
function analyzeBranch(src, state) {
  const items = [];
  let index = 0;
  let reason = null;

  const reject = (message) => {
    if (!reason) reason = message;
  };

  while (index < src.length) {
    const char = src[index];
    if (char === "|" || char === ")") return { items, reason };

    const atom = readAtom(src, index);
    if (atom.broken) state.broken = true;

    let next = atom.next;
    const quantifier = readQuantifier(src, next);
    if (quantifier) next = quantifier.next;

    if (atom.kind === "assert") {
      index = next;
      continue;
    }

    const quantified = Boolean(quantifier);
    const unbounded = quantified && (quantifier.max === Infinity || quantifier.max === 1);
    const runLength =
      atom.kind === "char" && !quantified && src[index] !== "\\" && src[index] !== "["
        ? literalRunLength(src, index)
        : 0;

    const isGroup = atom.kind === "group";
    if (isGroup) {
      state.unbounded += atom.analysis.unbounded;
      if (quantified) {
        // Kelompok yang diulang + isi yang bisa diulang/dikosongkan =ibi
        // backtracking eksponensial.
        if (atom.analysis.hasQuantifier) {
          reject("ada kuantifier di dalam kelompok yang juga diulang (mis. `(a+)+`)");
        }
        if (atom.analysis.canMatchEmpty) {
          reject("isi kelompok yang diulang bisa kosong (mis. `(a*|b)+`)");
        }
        if (atom.analysis.overlappingBranches) {
          reject("dua cabang alternasi di bawah kuantifier bisa cocok dengan teks yang sama (mis. `(a|ab)+`)");
        }
      }
    }

    const item = {
      set: atom.set,
      quantified,
      unbounded,
      // `unbounded` hanya untuk atom ini. `expansive` juga mencakup kelompok
      // yang isinya bisa mezclar beberapa panjang (mis. `(a+)`), karena
      // `(a+)(a+)$`Tetap quadratic walau `(a+)` itu sendiri tidak diulang.
      expansive:
        unbounded || (isGroup && atom.analysis.hasUnboundedQuantifier),
      singleChar: isGroup
        ? atom.analysis.singleChar
        : runLength > 0
          ? runLength <= 1
          : true,
      canMatchEmpty: quantified
        ? quantifier.min === 0 || (isGroup && atom.analysis.canMatchEmpty)
        : isGroup
          ? atom.analysis.canMatchEmpty
          : setIsEmpty(atom.set),
    };

    index = runLength > 0 ? index + runLength : next;

    const previous = items[items.length - 1];
    if (previous && previous.expansive && setsOverlap(previous.set, item.set)) {
      if (item.quantified) {
        reject("dua kuantifier bertumpuk menebak karakter yang sama (mis. `a*a*`)");
      } else if (item.expansive || item.singleChar) {
        reject("karakter setelah kuantifier bisa ikut ditelan kuantifier itu (mis. `.*a`)");
      }
    }

    if (unbounded) state.unbounded += 1;
    items.push(item);
  }

  return { items, reason };
}

function splitAlternation(src) {
  const branches = [];
  let depth = 0;
  let start = 0;
  let inClass = false;

  for (let index = 0; index < src.length; index += 1) {
    const char = src[index];
    if (inClass) {
      if (char === "\\") index += 1;
      else if (char === "]") inClass = false;
      continue;
    }
    if (char === "\\") {
      index += 1;
      continue;
    }
    if (char === "[") {
      inClass = true;
      continue;
    }
    if (char === "(") depth += 1;
    else if (char === ")") depth -= 1;
    else if (char === "|" && depth === 0) {
      branches.push(src.slice(start, index));
      start = index + 1;
    }
  }

  branches.push(src.slice(start));
  return branches;
}

function analyzeAlternation(src) {
  const state = { unbounded: 0, broken: false };
  const branches = splitAlternation(src);
  const first = charSetOf([]);
  let allSingleChar = true;
  let canMatchEmpty = false;
  let hasQuantifier = false;
  let hasUnboundedQuantifier = false;
  let overlappingBranches = false;
  let reason = null;
  let previousFirst = null;

  for (const branch of branches) {
    const result = analyzeBranch(branch, state);
    if (result.reason && !reason) reason = result.reason;
    if (state.broken && !reason) {
      reason = "pola tidak bisa diurai (kurung kurawal tidak seimbang)";
    }

    if (result.items.length === 0) {
      canMatchEmpty = true;
      allSingleChar = false;
      continue;
    }

    const head = result.items[0];
    if (head.quantified) allSingleChar = false;
    if (!head.singleChar) allSingleChar = false;
    if (head.canMatchEmpty) canMatchEmpty = true;
    if (head.quantified) hasQuantifier = true;
    if (result.items.some((item) => item.quantified)) hasQuantifier = true;
    if (result.items.some((item) => item.unbounded)) hasUnboundedQuantifier = true;
    if (result.items.every((item) => item.canMatchEmpty)) canMatchEmpty = true;
    if (previousFirst && setsOverlap(previousFirst, head.set)) overlappingBranches = true;

    setUnion(first, head.set);
    previousFirst = head.set;
  }

  return {
    first,
    broken: state.broken,
    reason,
    hasQuantifier,
    canMatchEmpty,
    overlappingBranches,
    hasUnboundedQuantifier,
    singleChar: allSingleChar,
    unbounded: state.unbounded,
  };
}

/**
 * Gerbang install-time untuk pola dari admin grup.
 *
 * Menolak (fail-closed) pola yang bisa membuat `.test()` backtracking
 * eksponensial/polomial: kuantifier bersarang, isi kelompok yang diulang
 * masih bisa kosong, alternasi overlap di bawah kuantifier, kuantifier
 * bertumpuk yang overlap, backreference, dan pola terlalu panjang.
 *
 * @returns {{safe: boolean, reason: string|null}}
 */
function analyzeCustomPattern(pattern) {
  const source = String(pattern ?? "");

  if (!source.trim()) return { safe: false, reason: "pola kosong" };
  if (source.length > CUSTOM_MAX_PATTERN_LEN) {
    return {
      safe: false,
      reason: `pola terlalu panjang (maks ${CUSTOM_MAX_PATTERN_LEN} karakter)`,
    };
  }

  let analysis;
  try {
    analysis = analyzeAlternation(source);
  } catch {
    return { safe: false, reason: "pola tidak bisa diurai" };
  }

  if (analysis.broken) {
    return { safe: false, reason: "pola tidak bisa diurai (kurung kurawal tidak seimbang)" };
  }
  if (analysis.reason) return { safe: false, reason: analysis.reason };
  if (analysis.unbounded > CUSTOM_MAX_UNBOUNDED_QUANTIFIERS) {
    return {
      safe: false,
      reason: `terlalu banyak kuantifier tak terbatas (maks ${CUSTOM_MAX_UNBOUNDED_QUANTIFIERS})`,
    };
  }

  try {
    new RegExp(source, "i");
  } catch {
    return { safe: false, reason: "regex tidak valid" };
  }

  return { safe: true, reason: null };
}
// `g`/`y` bikin `.test()` stateful (lastIndex) sehingga hasilbergantian antar
// pesan untuk rule yang sama. Buang supaya hasil per pesan deterministik.
function sanitizeCustomFlags(flags) {
  const keep = new Set();
  for (const flag of String(flags ?? "i")) {
    if (flag === "i" || flag === "m" || flag === "s" || flag === "u" || flag === "v") {
      keep.add(flag);
    }
  }
  return keep.size ? [...keep].join("") : "i";
}

const _customRegexCache = new Map();

/**
 * Precompile + cache. Cache berisi `null` juga, jadi pola yang sudah
 * dinyatakan tidak aman tidak dianalisis ulang tiap pesan.
 */
function getCustomRegex(rule) {
  const pattern = String(rule.pattern);
  const flags = sanitizeCustomFlags(rule.flags);
  const key = flags + "|" + pattern;

  if (_customRegexCache.has(key)) return _customRegexCache.get(key);

  let compiled = null;
  if (analyzeCustomPattern(pattern).safe) {
    try {
      compiled = new RegExp(pattern, flags);
    } catch {
      compiled = null;
    }
  }
  _customRegexCache.set(key, compiled);
  evictOldestOverCap(_customRegexCache, CUSTOM_REGEX_CACHE_CAP);
  return compiled;
}

/**
 * Cocokkan satu pesan dengan rule AntiCustom milik grup.
 * Batas kerja per pesan: subjek dipotong ke CUSTOM_MAX_SUBJECT_LEN, maksimal
 * CUSTOM_MAX_RULES rule, budget CUSTOM_MATCH_BUDGET_MS total, dan setiap
 * regex di-precompile sekali (cache) bukan tiap pesan.
 */
function matchCustomRule(text, rules = []) {
  if (!text || !Array.isArray(rules) || rules.length === 0) return null;

  const subject = String(text).slice(0, CUSTOM_MAX_SUBJECT_LEN);
  const lower = subject.toLowerCase();
  const deadline = Date.now() + CUSTOM_MATCH_BUDGET_MS;
  const limit = Math.min(rules.length, CUSTOM_MAX_RULES);

  for (let index = 0; index < limit; index += 1) {
    if (index > 0 && Date.now() > deadline) break;

    const rule = rules[index];
    if (!rule || !rule.pattern) continue;

    if (rule.type === "regex") {
      const regex = getCustomRegex(rule);
      if (regex && regex.test(subject)) return rule;
      continue;
    }

    if (lower.includes(String(rule.pattern).toLowerCase())) return rule;
  }

  return null;
}

/**
 * @param {string} sender - addressing ASLI server (key.participant), dipakai
 *   untuk revoke, kick, dan mention; bukan m.sender yang sudah di-resolve ke PN.
 */
async function executeProtectionAction({
  sock,
  chatId,
  keyId,
  sender,
  senderTag,
  mode,
  removeMessageKey,
  kickMessageKey = null,
  replacements = {},
}) {
  await sock.sendMessage(chatId, {
    delete: {
      remoteJid: chatId,
      fromMe: false,
      id: keyId,
      participant: sender,
    },
  });

  if (mode === "kick") {
    try {
      await sock.groupParticipantsUpdate(chatId, [sender], "remove");
      if (kickMessageKey) {
        await sock.sendMessage(chatId, {
          text: gpMsg(kickMessageKey, { user: senderTag, ...replacements }),
          mentions: [sender],
        });
      }
      return true;
    } catch {}
  }

  await sock.sendMessage(chatId, {
    text: gpMsg(removeMessageKey, { user: senderTag, ...replacements }),
    mentions: [sender],
  });
  return true;
}

async function handleTextProtection({
  m,
  sock,
  db,
  toggleKey,
  modeKey,
  removeMessageKey,
  kickMessageKey,
  detector,
  defaultMode = "remove",
  buildReplacements = () => ({}),
}) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group[toggleKey] !== "on") return false;

  const text = getProtectionText(m);
  const detected = detector(text, group, m);
  if (!detected?.matched) return false;

  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  const selfMode = db.setting("selfMode") === true;
  if (!selfMode && isSameParticipant(m.key?.participant || m.sender, botNumber)) return false;

  try {
    const groupMeta = await sock.groupMetadata(m.chat);
    const senderCand = [m.key?.participant, m.sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = m.sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;

    if (!isBotAdminCheck(groupMeta.participants, botCand)) {
      await sock.sendMessage(m.chat, {
        text: gpMsg("notAdmin"),
        mentions: [m.sender],
      });
      return true;
    }

    const mode = normalizeProtectionMode(group[modeKey], defaultMode);
    return executeProtectionAction({
      sock,
      chatId: m.chat,
      keyId: m.key.id || m.id,
      sender: m.key?.participant || m.sender,
      senderTag,
      mode,
      removeMessageKey,
      kickMessageKey,
      replacements: buildReplacements(detected, m),
    });
  } catch {
    return false;
  }
}

async function handleAntilink(m, sock, db) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group.antilink !== "on") return false;

  const text = m.body || "";
  LINK_REGEX.lastIndex = 0;
  WA_LINK_REGEX.lastIndex = 0;
  const hasLink =
    LINK_REGEX.test(text) ||
    ((WA_LINK_REGEX.lastIndex = 0), WA_LINK_REGEX.test(text));
  LINK_REGEX.lastIndex = 0;
  WA_LINK_REGEX.lastIndex = 0;
  if (!hasLink) return false;

  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  if (isSameParticipant(m.key?.participant || m.sender, botNumber)) return false;

  try {
    const groupMeta = await sock.groupMetadata(m.chat);
    const senderCand = [m.key?.participant, m.sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = m.sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;

    if (!isBotAdminCheck(groupMeta.participants, botCand)) {
      await sock.sendMessage(m.chat, {
        text: gpMsg("notAdmin"),
        mentions: [m.sender],
      });
      return true;
    }

    await sock.sendMessage(m.chat, { delete: revokeKey(m) });
    const mode = group.antilinkMode || "remove";

    if (mode === "kick") {
      await sock.groupParticipantsUpdate(m.chat, [m.key?.participant || m.sender], "remove");
      await sock.sendMessage(m.chat, {
        text: gpMsg("antilinkKick", { user: senderTag }),
        mentions: [m.sender],
      });
    } else {
      await sock.sendMessage(m.chat, {
        text: gpMsg("antilink", { user: senderTag }),
        mentions: [m.sender],
      });
    }

    return true;
  } catch (error) {
    return false;
  }
}

/**
 * JID bot dalam bentuk PN yang bisa dibandingkan.
 *
 * `sock.user.id` bisa "62899:12@s.whatsapp.net" (dengan device) atau
 * "62899@s.whatsapp.net" (tanpa). Pola lama `id.split(":")[0] + "@s.whatsapp.net"`
 * di kasus kedua jadi "62899@s.whatsapp.net@s.whatsapp.net" — semua cek
 * "apakah ini bot?" lalu gagal, dan bot mulai revoke pesan sendiri.
 */
function botJidOf(sock) {
  const number = String(sock?.user?.id || "").split(":")[0].split("@")[0];
  return number ? `${number}@s.whatsapp.net` : "";
}

async function handleAntiTagSW(rawMsg, sock, db) {
  const key = rawMsg.key;
  if (!key?.remoteJid) return false;

  const chatId = key.remoteJid;
  if (!chatId.endsWith("@g.us")) return false;

  const group = db.getGroup(chatId) || {};
  if (group.antitagsw !== "on") return false;

  const msg = rawMsg.message;
  if (!msg) return false;

  const hasStatusTag =
    msg.groupStatusMentionMessage ||
    msg.statusMentionMessage ||
    msg.groupMentionedMessage;
  if (!hasStatusTag) return false;

  const sender = key.participant || key.remoteJid;
  const botNumber = botJidOf(sock);
  // Bandingkan ke PN DAN ke LID: di grup addressing_mode=lid, participant
  // pesan bot sendiri adalah sock.user.lid, bukan sock.user.id.
  if (
    key.fromMe === true ||
    isSameParticipant(sender, botNumber) ||
    (sock.user?.lid && isSameParticipant(key.participant, sock.user.lid))
  ) {
    return false;
  }

  try {
    const groupMeta = await sock.groupMetadata(chatId);
    const senderCand = [sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;
    if (!isBotAdminCheck(groupMeta.participants, botCand)) {
      await sock.sendMessage(chatId, {
        text: gpMsg("notAdmin"),
        mentions: [sender],
      });
      return true;
    }

    await sock.sendMessage(chatId, {
      delete: {
        remoteJid: chatId,
        fromMe: false,
        id: key.id,
        participant: sender,
      },
    });
    await sock.sendMessage(chatId, {
      text: gpMsg("antitagsw", { user: senderTag }),
      mentions: [sender],
    });

    return true;
  } catch (error) {
    return false;
  }
}

async function handleAntiViewOnce(rawMsg, sock, db) {
  const key = rawMsg.key;
  if (!key?.remoteJid) return false;

  const chatId = key.remoteJid;
  if (!chatId.endsWith("@g.us")) return false;

  const group = db.getGroup(chatId) || {};
  if (group.antiviewonce !== "on") return false;

  const msg = rawMsg.message;
  if (!msg) return false;

  let innerMsg = null;
  if (msg.viewOnceMessage?.message) innerMsg = msg.viewOnceMessage.message;
  else if (msg.viewOnceMessageV2?.message)
    innerMsg = msg.viewOnceMessageV2.message;
  else if (msg.viewOnceMessageV2Extension?.message)
    innerMsg = msg.viewOnceMessageV2Extension.message;
  if (!innerMsg) return false;

  const sender = key.participant || key.remoteJid;
  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  if (isSameParticipant(sender, botNumber)) return false;

  try {
    let mediaType = null;
    let mediaMsg = null;

    if (innerMsg.imageMessage) {
      mediaType = "image";
      mediaMsg = innerMsg.imageMessage;
    } else if (innerMsg.videoMessage) {
      mediaType = "video";
      mediaMsg = innerMsg.videoMessage;
    } else if (innerMsg.audioMessage) {
      mediaType = "audio";
      mediaMsg = innerMsg.audioMessage;
    }

    if (!mediaType || !mediaMsg) return false;

    const fakeMsg = { key, message: innerMsg };
    const mediaBuffer = await downloadMediaMessage(fakeMsg, "buffer", {});
    if (!mediaBuffer || mediaBuffer.length < 100) return false;

    const caption = mediaMsg.caption || "";
    const senderTag = sender.split("@")[0];
    const headerText = gpMsg("antiviewonce", { user: senderTag });
    const fullCaption = caption ? `${headerText}\n${caption}` : headerText;

    const msgContent = { mentions: [sender] };

    if (mediaType === "image") {
      msgContent.image = mediaBuffer;
      msgContent.caption = fullCaption;
    } else if (mediaType === "video") {
      msgContent.video = mediaBuffer;
      msgContent.caption = fullCaption;
    } else if (mediaType === "audio") {
      msgContent.audio = mediaBuffer;
      msgContent.mimetype = "audio/mpeg";
    }

    await sock.sendMessage(chatId, msgContent);
    return true;
  } catch (error) {
    return false;
  }
}

async function handleAntiRemove(messageUpdate, sock, db) {
  try {
    const { key, update } = messageUpdate;
    if (!key?.remoteJid) return false;

    const chatId = key.remoteJid;
    if (!chatId.endsWith("@g.us")) return false;

    const group = db.getGroup(chatId) || {};
    if (group.antiremove !== "on") return false;

    const messageStubType = update?.messageStubType;
    if (messageStubType !== 1 && messageStubType !== 132) return false;

    const deletedMsgId = key.id;
    const cached = getCachedMessage(deletedMsgId);
    if (!cached) {
      const storeMsg = sock.store?.messages?.get?.(chatId)?.get?.(deletedMsgId);
      if (storeMsg?.message) {
        const storeData = {
          key: { ...storeMsg.key },
          message: JSON.parse(JSON.stringify(storeMsg.message)),
          messageTimestamp: storeMsg.messageTimestamp,
          pushName: storeMsg.pushName,
        };
        cacheMessage(deletedMsgId, storeData, null);
        return handleAntiRemove(messageUpdate, sock, db);
      }
      return false;
    }
    let originalSender =
      update?.key?.participant ||
      key.participant ||
      (key.fromMe ? sock.user?.id : chatId);
    if (isLid(originalSender)) {
      const safe = lidToJidSafe(originalSender);
      originalSender = safe || lidToJid(originalSender);
    }
    const senderTag = originalSender?.split("@")[0] || "Unknown";

    const headerText = gpMsg("antiremove", { user: senderTag });

    const headerMsg = await sock.sendMessage(chatId, {
      text: headerText,
      mentions: originalSender?.includes("@") ? [originalSender] : [],
    });

    try {
      const msgContent = cached.message?.message || cached.message;
      if (msgContent) {
        const contentKeys = Object.keys(msgContent).filter(
          (k) => k !== "messageContextInfo",
        );
        const msgType = contentKeys[0];

        if (
          msgType === "conversation" &&
          typeof msgContent.conversation === "string"
        ) {
          await sock.sendMessage(
            chatId,
            { text: msgContent.conversation },
            { quoted: headerMsg },
          );
        } else if (
          msgType === "extendedTextMessage" &&
          msgContent.extendedTextMessage?.text
        ) {
          const ext = msgContent.extendedTextMessage;
          await sock.sendMessage(
            chatId,
            { text: ext.text, contextInfo: ext.contextInfo },
            { quoted: headerMsg },
          );
        } else if (
          msgType === "imageMessage" ||
          msgType === "videoMessage" ||
          msgType === "documentMessage" ||
          msgType === "audioMessage" ||
          msgType === "stickerMessage"
        ) {
          const { downloadMediaMessage } = await import("onigis");
          const fakeMsg = {
            key: cached.key,
            message: { [msgType]: structuredClone(msgContent[msgType]) },
          };
          const buffer = await downloadMediaMessage(
            fakeMsg,
            "buffer",
            {},
            { logger: console, reuploadRequest: sock.updateMediaMessage },
          );
          if (buffer) {
            const sendContent = {};
            if (msgType === "imageMessage")
              Object.assign(sendContent, {
                image: buffer,
                caption: msgContent[msgType].caption || "",
              });
            else if (msgType === "videoMessage")
              Object.assign(sendContent, {
                video: buffer,
                caption: msgContent[msgType].caption || "",
              });
            else if (msgType === "documentMessage")
              Object.assign(sendContent, {
                document: buffer,
                fileName: msgContent[msgType].fileName || "file",
                mimetype: msgContent[msgType].mimetype,
              });
            else if (msgType === "audioMessage")
              Object.assign(sendContent, {
                audio: buffer,
                mimetype: msgContent[msgType].mimetype || "audio/mp3",
              });
            else if (msgType === "stickerMessage")
              Object.assign(sendContent, { sticker: buffer });
            await sock.sendMessage(chatId, sendContent, { quoted: headerMsg });
          }
        } else {
          const { generateWAMessageFromContent } = await import("onigis");
          const cleanContent = {};
          for (const k of contentKeys) {
            if (typeof msgContent[k] === "object" && msgContent[k] !== null) {
              cleanContent[k] = msgContent[k];
            }
          }
          if (Object.keys(cleanContent).length > 0) {
            const forwarded = generateWAMessageFromContent(
              chatId,
              cleanContent,
              {
                userJid: sock.user?.id,
              },
            );
            await sock.relayMessage(chatId, forwarded.message, {
              messageId: forwarded.key.id,
            });
          }
        }
      }
    } catch (e) {
      try {
        const msgContent = cached.message?.message || cached.message;
        const msgType = Object.keys(msgContent || {}).filter(
          (k) => k !== "messageContextInfo",
        )[0];
        if (msgType === "conversation") {
          await sock.sendMessage(
            chatId,
            { text: msgContent.conversation },
            { quoted: headerMsg },
          );
        } else if (msgType === "extendedTextMessage") {
          await sock.sendMessage(
            chatId,
            {
              text: msgContent.extendedTextMessage?.text || "[deleted message]",
            },
            { quoted: headerMsg },
          );
        } else {
          await sock.sendMessage(
            chatId,
            { text: "[AntiDelete] Pesan dihapus (tipe: " + msgType + ")" },
            { quoted: headerMsg },
          );
        }
      } catch (e2) {}
    }

    deleteCachedMessage(deletedMsgId);
    return true;
  } catch (error) {
    return false;
  }
}

async function handleAntiRemoveFromUpsert(msg, sock, db) {
  try {
    const chatId = msg.key?.remoteJid;
    if (!chatId?.endsWith("@g.us")) return false;

    const group = db.getGroup(chatId) || {};
    if (group.antiremove !== "on") return false;

    if (msg.messageStubType !== 1 && msg.messageStubType !== 132) return false;

    const deletedMsgId = msg.key?.id;
    if (!deletedMsgId) return false;

    const cached = getCachedMessage(deletedMsgId);
    if (!cached) return false;

    let deleterJid =
      msg.key?.participant || (msg.key?.fromMe ? sock.user?.id : chatId);
    if (isLid(deleterJid)) {
      const safe = lidToJidSafe(deleterJid);
      deleterJid = safe || lidToJid(deleterJid);
    }
    const deleterTag = deleterJid?.split("@")[0] || "Unknown";

    const headerText = gpMsg("antiremove", { user: deleterTag });

    const headerMsg = await sock.sendMessage(chatId, {
      text: headerText,
      mentions: deleterJid?.includes("@") ? [deleterJid] : [],
    });

    try {
      const msgContent = cached.message?.message || cached.message;
      if (msgContent) {
        const contentKeys = Object.keys(msgContent).filter(
          (k) => k !== "messageContextInfo",
        );
        const msgType = contentKeys[0];

        if (
          msgType === "conversation" &&
          typeof msgContent.conversation === "string"
        ) {
          await sock.sendMessage(
            chatId,
            { text: msgContent.conversation },
            { quoted: headerMsg },
          );
        } else if (
          msgType === "extendedTextMessage" &&
          msgContent.extendedTextMessage?.text
        ) {
          const ext = msgContent.extendedTextMessage;
          await sock.sendMessage(
            chatId,
            { text: ext.text, contextInfo: ext.contextInfo },
            { quoted: headerMsg },
          );
        } else if (
          msgType === "imageMessage" ||
          msgType === "videoMessage" ||
          msgType === "documentMessage" ||
          msgType === "audioMessage" ||
          msgType === "stickerMessage"
        ) {
          const { downloadMediaMessage } = await import("onigis");
          const fakeMsg = {
            key: cached.key,
            message: { [msgType]: structuredClone(msgContent[msgType]) },
          };
          const buffer = await downloadMediaMessage(
            fakeMsg,
            "buffer",
            {},
            { logger: console, reuploadRequest: sock.updateMediaMessage },
          );
          if (buffer) {
            const sendContent = {};
            if (msgType === "imageMessage")
              Object.assign(sendContent, {
                image: buffer,
                caption: msgContent[msgType].caption || "",
              });
            else if (msgType === "videoMessage")
              Object.assign(sendContent, {
                video: buffer,
                caption: msgContent[msgType].caption || "",
              });
            else if (msgType === "documentMessage")
              Object.assign(sendContent, {
                document: buffer,
                fileName: msgContent[msgType].fileName || "file",
                mimetype: msgContent[msgType].mimetype,
              });
            else if (msgType === "audioMessage")
              Object.assign(sendContent, {
                audio: buffer,
                mimetype: msgContent[msgType].mimetype || "audio/mp3",
              });
            else if (msgType === "stickerMessage")
              Object.assign(sendContent, { sticker: buffer });
            await sock.sendMessage(chatId, sendContent, { quoted: headerMsg });
          }
        } else {
          const { generateWAMessageFromContent } = await import("onigis");
          const cleanContent = {};
          for (const k of contentKeys) {
            if (typeof msgContent[k] === "object" && msgContent[k] !== null) {
              cleanContent[k] = msgContent[k];
            }
          }
          if (Object.keys(cleanContent).length > 0) {
            const forwarded = generateWAMessageFromContent(
              chatId,
              cleanContent,
              { userJid: sock.user?.id },
            );
            await sock.relayMessage(chatId, forwarded.message, {
              messageId: forwarded.key.id,
            });
          }
        }
      }
    } catch (e) {}

    deleteCachedMessage(deletedMsgId);
    return true;
  } catch (error) {
    return false;
  }
}

async function cacheMessageForAntiRemove(m, sock, db) {
  if (!m.isGroup) return;

  const group = db.getGroup(m.chat) || {};
  if (group.antiremove !== "on") return;

  try {
    const msgType = m.type;
    if (!msgType) return;
    if (
      msgType.includes("protocolMessage") ||
      msgType.includes("senderKeyDistribution") ||
      msgType.includes("reactionMessage")
    )
      return;

    const msgId = m.key?.id;
    if (!msgId) return;

    const rawMsg = {
      key: { ...m.key },
      message: m.message || {},
      messageTimestamp: m.messageTimestamp,
      pushName: m.pushName,
    };

    cacheMessage(msgId, rawMsg, null);
  } catch {}
}

const WA_SPECIFIC_PATTERNS = [
  /chat\.whatsapp\.com\/[A-Za-z0-9]+/gi,
  /wa\.me\/[0-9+]+/gi,
  /whatsapp\.com\/channel\/[A-Za-z0-9]+/gi,
  /whatsapp\.com\/c\/[A-Za-z0-9]+/gi,
  /api\.whatsapp\.com\/send/gi,
];

const ALL_LINK_PATTERN =
  /(?:(?:https?:\/\/)|(?:www\.))?[^\s<>"{}|\\^`[\]]+\.(?:com|net|org|io|id|co|cc|me|ly|gg|gl|it|tv|ru|de|fr|uk|us|info|biz|xyz|top|site|online|store|app|dev|ai|link|click|fun|space|live|world|tech|digital|cloud|pro|vip|pw|tk|ml|ga|cf|gq|club|mobi|name|asia|tel|wang|win|bid|loan|ren|ink|art|shop|life|game|news|blog|design|studio|photo|press|media|social|group|team|community|domain)(?:\/[^\s<>"{}|\\^`[\]]*)?/gi;

function containsWaSpecificLink(text) {
  if (!text) return { hasLink: false, link: null };
  for (const pattern of WA_SPECIFIC_PATTERNS) {
    pattern.lastIndex = 0;
    const match = text.match(pattern);
    if (match) return { hasLink: true, link: match[0] };
  }
  return { hasLink: false, link: null };
}

function containsAnyLink(text) {
  if (!text) return { hasLink: false, link: null, links: [] };
  ALL_LINK_PATTERN.lastIndex = 0;
  const matches = text.match(ALL_LINK_PATTERN);
  if (matches && matches.length > 0) {
    return { hasLink: true, link: matches[0], links: matches };
  }
  return { hasLink: false, link: null, links: [] };
}

async function handleAntilinkGc(m, sock, db) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group.antilinkgc !== "on") return false;

  const text = m.body || "";
  const { hasLink, link } = containsWaSpecificLink(text);
  if (!hasLink) return false;

  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  const selfMode = db.setting("selfMode") === true;
  if (!selfMode && isSameParticipant(m.key?.participant || m.sender, botNumber)) return false;

  try {
    const groupMeta = await sock.groupMetadata(m.chat);
    const senderCand = [m.key?.participant, m.sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = m.sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;
    if (!isBotAdminCheck(groupMeta.participants, botCand)) return false;

    await sock.sendMessage(m.chat, { delete: revokeKey(m) });
    const mode = group.antilinkgcMode || "remove";

    if (mode === "kick") {
      try {
        await sock.groupParticipantsUpdate(m.chat, [m.key?.participant || m.sender], "remove");
        await sock.sendMessage(m.chat, {
          text: gpMsg("antilinkGcKick", { user: senderTag }),
          mentions: [m.sender],
        });
      } catch (kickErr) {
        await sock.sendMessage(m.chat, {
          text: gpMsg("antilinkGc", { user: senderTag }),
          mentions: [m.sender],
        });
      }
    } else {
      await sock.sendMessage(m.chat, {
        text: gpMsg("antilinkGc", { user: senderTag }),
        mentions: [m.sender],
      });
    }

    return true;
  } catch (error) {
    console.error(
      `[ANTILINK GC ERROR] Failed to perform action: ${error.message}`,
    );
    return false;
  }
}

async function handleAntilinkAll(m, sock, db) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group.antilinkall !== "on") return false;

  const text = m.body || "";
  const { hasLink, link } = containsAnyLink(text);
  if (!hasLink) return false;

  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  const selfMode = db.setting("selfMode") === true;
  if (!selfMode && isSameParticipant(m.key?.participant || m.sender, botNumber)) return false;

  try {
    const groupMeta = await sock.groupMetadata(m.chat);
    const senderCand = [m.key?.participant, m.sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = m.sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;
    if (!isBotAdminCheck(groupMeta.participants, botCand)) return false;

    await sock.sendMessage(m.chat, { delete: revokeKey(m) });
    const mode = group.antilinkallMode || "remove";

    if (mode === "kick") {
      try {
        await sock.groupParticipantsUpdate(m.chat, [m.key?.participant || m.sender], "remove");
        await sock.sendMessage(m.chat, {
          text: gpMsg("antilinkAllKick", { user: senderTag }),
          mentions: [m.sender],
        });
      } catch (kickErr) {
        await sock.sendMessage(m.chat, {
          text: gpMsg("antilinkAll", { user: senderTag }),
          mentions: [m.sender],
        });
      }
    } else {
      await sock.sendMessage(m.chat, {
        text: gpMsg("antilinkAll", { user: senderTag }),
        mentions: [m.sender],
      });
    }

    return true;
  } catch (error) {
    console.error(
      `[ANTILINK ALL ERROR] Failed to perform action: ${error.message}`,
    );
    return false;
  }
}

async function handleAntiJudol(m, sock, db) {
  return handleTextProtection({
    m,
    sock,
    db,
    toggleKey: "antijudol",
    modeKey: "antijudolMode",
    removeMessageKey: "antijudol",
    kickMessageKey: "antijudolKick",
    detector: detectJudol,
  });
}

async function handleAntiPhising(m, sock, db) {
  return handleTextProtection({
    m,
    sock,
    db,
    toggleKey: "antiphising",
    modeKey: "antiphisingMode",
    removeMessageKey: "antiphising",
    kickMessageKey: "antiphisingKick",
    detector: detectPhishing,
  });
}

async function handleAntiCustom(m, sock, db) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group.anticustom !== "on") return false;

  const text = getProtectionText(m);
  const rule = matchCustomRule(text, group.anticustomRules || []);
  if (!rule) return false;

  const botNumber = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
  const selfMode = db.setting("selfMode") === true;
  if (!selfMode && isSameParticipant(m.key?.participant || m.sender, botNumber)) return false;

  try {
    const groupMeta = await sock.groupMetadata(m.chat);
    const senderCand = [m.key?.participant, m.sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = m.sender.split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;
    if (!isBotAdminCheck(groupMeta.participants, botCand)) {
      await sock.sendMessage(m.chat, {
        text: gpMsg("notAdmin"),
        mentions: [m.sender],
      });
      return true;
    }

    const mode = normalizeProtectionMode(
      rule.action || group.anticustomMode,
      "remove",
    );
    return executeProtectionAction({
      sock,
      chatId: m.chat,
      keyId: m.key.id || m.id,
      sender: m.key?.participant || m.sender,
      senderTag,
      mode,
      removeMessageKey: "anticustom",
      kickMessageKey: "anticustomKick",
      replacements: {
        rule: rule.name || rule.pattern || "custom-rule",
      },
    });
  } catch {
    return false;
  }
}

async function handleAntiSwGc(rawMsg, sock, db) {
  const key = rawMsg?.key;
  if (!key?.remoteJid) return false;

  const chatId = key.remoteJid;
  if (!chatId.endsWith("@g.us")) return false;

  const group = db.getGroup(chatId) || {};
  if (group.antiswgc !== "on") return false;

  const detectedType = detectSwGcType(rawMsg);
  if (!detectedType) return false;

  const sender = resolveMessageSenderJid(key, sock);
  const botNumber = botJidOf(sock);
  // Pesan bot sendiri tidak boleh pernah di-revoke. Sebelumnya isSelfSender
  // hanya dipakai untuk MENELEWATI cek admin, lalu tetap dihapus — bot
  // memproses outputnya sendiri dan menghapus pesannya sendiri.
  //
  // Bandingkan ke PN DAN ke LID: di grup addressing_mode=lid, participant
  // pesan bot sendiri adalah sock.user.lid, bukan sock.user.id.
  if (
    key.fromMe === true ||
    isSameParticipant(sender, botNumber) ||
    (sock.user?.lid && isSameParticipant(key.participant, sock.user.lid))
  ) {
    return false;
  }

  try {
    const groupMeta = await sock.groupMetadata(chatId);
    const senderCand = [sender];
    const botCand = [botNumber, sock.user?.lid];
    const senderTag = (sender || botNumber || "Unknown").split("@")[0];

    if (isAdminCheck(groupMeta.participants, senderCand)) return false;
    if (!isBotAdminCheck(groupMeta.participants, botCand)) {
      await sock.sendMessage(chatId, {
        text: gpMsg("notAdmin"),
        mentions: sender?.includes("@") ? [sender] : [],
      });
      return true;
    }

    try {
      await sock.sendMessage(chatId, {
        delete: key,
      });
    } catch {
      await sock.sendMessage(chatId, {
        delete: {
          remoteJid: chatId,
          fromMe: Boolean(key.fromMe || isSelfSender),
          id: key.id,
          participant: key.participant || sender,
        },
      });
    }

    await sock.sendMessage(chatId, {
      text: gpMsg("antiswgc", { user: senderTag, type: detectedType }),
      mentions: sender?.includes("@") ? [sender] : [],
    });

    return true;
  } catch {
    return false;
  }
}

async function handleAntiHidetag(m, sock, db) {
  if (!m.isGroup) return false;

  const group = db.getGroup(m.chat) || {};
  if (group.antihidetag !== "on") return false;

  if (!m.mentionedJid || m.mentionedJid.length === 0) return false;

  try {
    const groupMetadata = await sock.groupMetadata(m.chat);
    const participants = groupMetadata.participants || [];

    if (m.mentionedJid.length < participants.length) return false;
    if (m.isAdmin || m.isOwner || m.fromMe) return false;
    if (!m.isBotAdmin) return false;

    await sock.sendMessage(m.chat, { delete: revokeKey(m) });

    const senderTag = m.sender.split("@")[0];
    await sock.sendMessage(m.chat, {
      text: gpMsg("antihidetag", { user: senderTag }),
      mentions: [m.sender],
    });

    return true;
  } catch (e) {
    return false;
  }
}

export {
  handleAntilink,
  handleAntiTagSW,
  handleAntiSwGc,
  detectSwGcType,
  isSwGcCandidate,
  handleAntiJudol,
  handleAntiPhising,
  handleAntiCustom,
  handleAntiViewOnce,
  handleAntiRemove,
  handleAntiRemoveFromUpsert,
  cacheMessageForAntiRemove,
  handleAntilinkGc,
  handleAntilinkAll,
  handleAntiHidetag,
  analyzeCustomPattern,
  matchCustomRule,
  CUSTOM_MAX_PATTERN_LEN,
  CUSTOM_MAX_SUBJECT_LEN,
  CUSTOM_MAX_RULES,
  CUSTOM_MAX_UNBOUNDED_QUANTIFIERS,
  CUSTOM_MATCH_BUDGET_MS,
};
