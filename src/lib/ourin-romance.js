// src/lib/ourin-romance.js
// Shared romance engine for gacha waifu & gacha husbu:
// difficulty systems (diminishing, fatigue, neglect, anger), moods, constants, jealousy.

export const TIER_ORDER = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
export const TIER_VALUE = { Common: 1, Rare: 2, Epic: 4, Legendary: 8, Mythic: 16 };
export const TIER_EXPECTED = 0.55 * 1 + 0.25 * 2 + 0.13 * 4 + 0.055 * 8 + 0.015 * 16;
export const PITY_THRESHOLD = 20;
export const EVENT_CHANCE = 0.18;
export const ANGER_THRESHOLD = 50;

export const DOWRY = {
  Common:     { limit: 1000,   koin: 20000,   exp: 500 },
  Rare:       { limit: 3000,   koin: 60000,   exp: 1500 },
  Epic:       { limit: 8000,   koin: 200000,  exp: 5000 },
  Legendary:  { limit: 15000,  koin: 500000,  exp: 12000 },
  Mythic:     { limit: 30000,  koin: 1000000, exp: 30000 },
};

export const MOOD_MULT = { ceria: 1.3, romantis: 1.2, biasa: 1.0, sedih: 0.7, marah: 0.5 };

export function todayStr() {
  return new Date().toLocaleDateString("id-ID", { timeZone: "Asia/Jakarta" });
}

export function moodLabel(mood) {
  return { ceria: "ceria 😄", romantis: "romantis 💘", biasa: "biasa 🙂", sedih: "sedih 😢", marah: "marah 😡" }[mood] || mood;
}

export function getDailyMood(senderJid, dateStr) {
  const seed = `${dateStr}_${senderJid}`;
  let h = 0;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  const r = (h % 10000) / 10000;
  if (r < 0.3) return "ceria";
  if (r < 0.55) return "romantis";
  if (r < 0.8) return "biasa";
  if (r < 0.92) return "sedih";
  return "marah";
}

// 1) Diminishing gain: makin tinggi affection, makin kecil gain.
export function diminish(change, current = 50) {
  const f = Math.max(0.15, 1.05 - (current || 50) / 100);
  return Math.max(1, Math.round(change * f));
}

// 2) Fatigue harian: interaksi > 3 per hari menyusut.
export function fatigueMult(stats = {}) {
  const n = stats.actionsToday ?? 0;
  if (n <= 3) return 1;
  return Math.max(0.25, 1 - (n - 3) * 0.12);
}

// 3) Neglect decay: tidak diinteraksi > 24 jam → aff turun -3/hari, cap 20.
export function neglectDecay(partner = {}, now = Date.now()) {
  const last = partner.lastInteractionAt;
  if (!last) return { decay: 0, hours: 0 };
  const hours = (now - new Date(last).getTime()) / 3600000;
  if (hours < 24) return { decay: 0, hours };
  return { decay: Math.min(20, Math.floor(hours / 24) * 3), hours };
}

// 4) Anger: >= ANGER_THRESHOLD paksa mood marah.
export function angerEffMood(mood, partner = {}) {
  return (partner.anger || 0) >= ANGER_THRESHOLD ? "marah" : mood;
}

export function angerUpdate(partner = {}, result) {
  let a = partner.anger || 0;
  if (result?.dislike) a += 12;
  else if (result?.like) a -= 8;
  return Math.max(0, Math.min(100, a));
}

// Terapkan neglect decay + anger decay, tandai terakhir interaksi. Mutasi partner.
export function applyNeglect(partner = {}, now = Date.now()) {
  const { decay, hours } = neglectDecay(partner, now);
  if (decay > 0) {
    partner.affection = Math.max(0, (partner.affection ?? 50) - decay);
    partner.anger = Math.max(0, (partner.anger || 0) - 10);
  }
  partner.lastInteractionAt = now;
  return { decay, hours };
}

// Terapkan diminishing + fatigue + drain anger ke change mentah.
export function finalGain(result, partner = {}, stats = {}) {
  const angry = (partner.anger || 0) >= ANGER_THRESHOLD;
  const extra = [];
  let change = diminish(result.change, partner.affection ?? 50);
  const fm = fatigueMult(stats);
  if (fm < 1) extra.push(`😮‍💨 Dia mulai lelah karena interaksi terus-menerus... (×${fm.toFixed(2)})`);
  change = Math.round(change * fm);
  let drain = 0;
  if (angry) {
    drain = Math.floor((partner.anger || 0) / 25);
    change -= drain;
  }
  return { change: Math.max(-50, change), drain, extra, angry };
}

const WAIFU_CMDS = new Set([
  "gachawaifu", "gachaistri", "waifuaction", "tinggalinwaifu", "waifuku",
  "istriku", "waifualbum", "albumwaifu", "waifupool", "poolwaifu",
]);
const HUSB_CMDS = new Set([
  "gachahusbu", "gachasuami", "husbuaction", "tinggalinhusbu", "husbuku",
  "suamiku", "husbualbum", "albumhusbu", "husbupool", "poolhusbu",
]);
const JEALOUSY_COOLDOWN = 45 * 60 * 1000;

function jealousyDrop(partner, now) {
  if (!partner) return null;
  if (now - (partner.lastJealousAt || 0) < JEALOUSY_COOLDOWN) return null;
  const married = !!partner.married;
  const chance = married ? 0.08 : 0.15;
  if (Math.random() > chance) return null;
  const drop = married
    ? Math.floor(Math.random() * 3) + 1
    : Math.floor(Math.random() * 6) + 3;
  partner.affection = Math.max(0, (partner.affection ?? 50) - drop);
  partner.lastJealousAt = now;
  return { drop, married };
}

export async function jealousyCheckAll({ m, sock, db, command }) {
  const cmd = String(command || "").toLowerCase();
  if (WAIFU_CMDS.has(cmd) || HUSB_CMDS.has(cmd)) return false;
  const user = db.getUser(m.sender);
  if (!user) return false;
  const now = Date.now();
  const fired = [];
  const wd = jealousyDrop(user.waifu, now);
  if (wd) fired.push({ name: user.waifu.name, ...wd });
  const hd = jealousyDrop(user.husbu, now);
  if (hd) fired.push({ name: user.husbu.name, ...hd });
  if (!fired.length) return false;
  db.setUser(m.sender, user);
  const lines = fired.map(f =>
    f.married
      ? `💑 *${f.name}*: "Sayang, jangan lupa luangkan waktu untukku juga ya... 💕"\n💞 *Affection -${f.drop}*`
      : `😤 *${f.name}* cemburu! "Kamu main *${cmd}* terus, aku jadi tersisih... 😔"\n💞 *Affection -${f.drop}*`
  );
  await m.reply(lines.join("\n")).catch(() => {});
  return true;
}
