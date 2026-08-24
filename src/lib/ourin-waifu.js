// src/lib/ourin-waifu.js
// Logika inti gacha waifu. Pure-ish, tanpa dependensi WA/Baileys.
import { getPool } from "../../data/waifu/index.js";

const TIER_ORDER = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
export const PITY_THRESHOLD = 20;
export const EVENT_CHANCE = 0.18;

const MOOD_MULT = { ceria: 1.3, romantis: 1.2, biasa: 1.0, sedih: 0.7, marah: 0.5 };

export const DOWRY = {
  Common:     { limit: 1000,  koin: 20000,  exp: 500 },
  Rare:       { limit: 3000,  koin: 60000,  exp: 1500 },
  Epic:       { limit: 8000,  koin: 200000, exp: 5000 },
  Legendary:  { limit: 15000, koin: 500000, exp: 12000 },
  Mythic:     { limit: 30000, koin: 1000000, exp: 30000 },
};

const TIER_VALUE = { Common: 1, Rare: 2, Epic: 4, Legendary: 8, Mythic: 16 };
const TIER_EXPECTED = 0.55 * 1 + 0.25 * 2 + 0.13 * 4 + 0.055 * 8 + 0.015 * 16;

export const ACTIONS = {
  // ===== approach (<80) =====
  jalan_taman:   { phase: "approach", base: [10, 18], exp: 40, likes: ["tsundere", "dandere", "kuudere", "genki", "amayadori"], dislikes: ["himedere", "ojou-sama"], text: (n) => `🌳 Suasana taman sejuk. *${n}* tersenyum riang sambil menggandeng lenganmu.` },
  jalan_mall:    { phase: "approach", base: [4, 10], exp: 25, likes: ["himedere"], dislikes: ["dandere", "kuudere", "genki"], text: (n) => `🏢 Mall ramai dan bising. *${n}* hanya berkeliling tanpa terlalu menikmatinya.` },
  jalan_pantai:  { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "amayadori", "kuudere"], dislikes: ["himedere"], text: (n) => `🏖️ Ombak pantai menenangkan. *${n}* menikmati angin laut dan menutup mata.` },
  jalan_kota:    { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "deredere"], dislikes: ["dandere", "amayadori"], text: (n) => `🌆 Kalian berjalan di tepi kota saat lampu mulai menyala. *${n}* terlihat kagum.` },
  kafe_kopi:     { phase: "approach", base: [4, 8], exp: 25, likes: ["kuudere"], dislikes: ["himedere"], text: (n) => `☕ *${n}* meringis meneguk kopi pahit. Sepertinya ia kurang suka.` },
  kafe_matcha:   { phase: "approach", base: [10, 18], exp: 40, likes: ["dandere", "deredere", "kuudere"], dislikes: [], text: (n) => `🍵 Matcha latte manis bikin *${n}* tersenyum manis padamu. Pilihan tepat!` },
  kafe_kue:      { phase: "approach", base: [8, 15], exp: 35, likes: ["deredere", "himedere", "genki"], dislikes: [], text: (n) => `🍰 Berbagi sepotong kue di kafe. *${n}* terlihat bahagia menikmatinya.` },
  kafe_susu:     { phase: "approach", base: [5, 10], exp: 25, likes: ["amayadori", "dandere"], dislikes: [], text: (n) => `🥛 Segelas susu hangat. *${n}* menyeruput pelan dan mengangguk puas.` },
  bioskop_romantis: { phase: "approach", base: [8, 16], exp: 35, likes: ["deredere", "kuudere", "onee-san"], dislikes: ["genki"], text: (n) => `🎬 Film romantis membuat *${n}* menyenderkan kepala di bahumu.` },
  bioskop_horor: { phase: "approach", base: [10, 20], exp: 40, likes: ["genki", "tsundere"], dislikes: ["dandere", "kuudere"], text: (n) => `👻 JUMPSCARE! *${n}* menjerit dan memeluk lenganmu erat sepanjang film.` },
  bioskop_animasi: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "amayadori"], dislikes: ["ojou-sama"], text: (n) => `🎨 Film animasi seru. *${n}* tertawa lepas di beberapa adegan.` },
  belanja_baju:  { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "himedere"], dislikes: ["amayadori"], text: (n) => `👗 Kamu membelikannya baju lucu. *${n}* memamerkannya sambil pipi merona.` },
  belanja_perhiasan: { phase: "approach", base: [10, 18], exp: 40, likes: ["himedere", "ojou-sama"], dislikes: ["kuudere"], text: (n) => `💎 Perhiasan mahal membuat mata *${n}* berbinar bahagia.` },
  belanja_boneka: { phase: "approach", base: [5, 10], exp: 25, likes: ["deredere", "dandere"], dislikes: ["ojou-sama"], text: (n) => `🧸 Boneka kecil mungil. *${n}* memeluknya gemas sepanjang perjalanan.` },
  karaoke_duet:  { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "deredere"], dislikes: ["kuudere", "himedere"], text: (n) => `🎤 Duet lagu favorit membuat *${n}* semangat tinggi. Kalian tertawa bersama.` },
  karaoke_solo:  { phase: "approach", base: [4, 10], exp: 25, likes: ["tsundere"], dislikes: ["genki"], text: (n) => `🎵 Kamu menyanyi sendirian sementara *${n}* menonton dengan tatapan datar.` },
  arcade_duo:    { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "tsundere"], dislikes: ["kuudere"], text: (n) => `🕹️ Adu skor di arcade. *${n}* menantangmu lagi dan lagi.` },
  arcade_boneka: { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "deredere"], dislikes: [], text: (n) => `🎯 Kamu memenangkan boneka untuk *${n}*. Wajahnya berseri-seri.` },
  piknik_taman:  { phase: "approach", base: [8, 15], exp: 35, likes: ["kuudere", "amayadori", "dandere"], dislikes: ["himedere"], text: (n) => `🧺 Piknik di atas tikar rumput. *${n}* bersandar santai dan menatap awan.` },
  piknik_pantai: { phase: "approach", base: [10, 18], exp: 40, likes: ["genki", "amayadori"], dislikes: ["ojou-sama"], text: (n) => `🏝️ Piknik pantai dengan bekal buatan sendiri. *${n}* sangat menikmatinya.` },
  masak_bareng:  { phase: "approach", base: [8, 16], exp: 35, likes: ["dandere", "onee-san", "deredere"], dislikes: [], text: (n) => `🍳 Kalian masak bareng di dapur. *${n}* antusias mengaduk sausnya.` },
  masak_kue:     { phase: "approach", base: [10, 18], exp: 40, likes: ["dandere", "onee-san", "himedere"], dislikes: [], text: (n) => `🍰 Memanggang kue bersama. *${n}* bangga dengan hasil kreasimu berdua.` },

  // ===== intim (80–99) =====
  peluk_belakang: { phase: "intim", base: [10, 18], exp: 45, likes: ["yandere", "deredere", "tsundere"], dislikes: [], text: (n) => `🤗 Kamu memeluk *${n}* dari belakang. Ia terkejut sesaat lalu tersenyum aman.` },
  peluk_depan:    { phase: "intim", base: [12, 20], exp: 50, likes: ["yandere", "deredere", "onee-san"], dislikes: [], text: (n) => `💑 Kalian saling berhadapan dan berpelukan erat. Detak jantung berdebar seirama.` },
  cium_kening:    { phase: "intim", base: [10, 18], exp: 45, likes: ["dandere", "tsundere", "yandere"], dislikes: [], text: (n) => `😚 Kecupan lembut di kening membuat *${n}* memejamkan mata bahagia.` },
  cium_bibir:     { phase: "intim", base: [14, 24], exp: 55, likes: ["yandere", "deredere"], dislikes: ["kuudere"], text: (n) => `💋 Ciuman lembut dan panjang. *${n}* membalas dengan desahan kecil.` },
  tidur_kelon:    { phase: "intim", base: [10, 16], exp: 45, likes: ["amayadori", "onee-san"], dislikes: [], text: (n) => `🛏️ *${n}* merebahkan kepalanya di dadamu dan tertidur pulas. Tenang sekali.` },
  tidur_serandu:  { phase: "intim", base: [12, 20], exp: 50, likes: ["onee-san", "amayadori", "yandere"], dislikes: [], text: (n) => `🌙 Kalian berpelukan erat di ranjang. *${n}* berbisik bahwa ia merasa aman.` },
  mandi_punggung: { phase: "intim", base: [10, 16], exp: 45, likes: ["onee-san", "kuudere"], dislikes: [], text: (n) => `🛁 Kamu menggosok punggung *${n}* dengan lembut. Ia mendesah rileks.` },
  mandi_bahu:     { phase: "intim", base: [12, 20], exp: 50, likes: ["deredere", "onee-san"], dislikes: [], text: (n) => `🧼 *${n}* menunduk malu saat kamu mengusap bahunya di air hangat.` },
  gendong_putri:  { phase: "intim", base: [10, 18], exp: 45, likes: ["deredere", "yandere", "tsundere"], dislikes: [], text: (n) => `👸 Kamu menggendong *${n}* dengan gaya putri. Ia memeluk lehermu erat.` },
  gendong_punggung: { phase: "intim", base: [8, 14], exp: 40, likes: ["genki", "amayadori"], dislikes: [], text: (n) => `🏃 *${n}* menaiki punggungmu tertawa-tawa sepanjang jalan pulang.` },
  tepuk_kepala:   { phase: "intim", base: [6, 12], exp: 35, likes: ["dandere", "tsundere"], dislikes: [], text: (n) => `🖐️ Kamu menepuk kepala *${n}* pelan. Ia mendengus tapi pipinya memerah.` },

  // ===== married =====
  mesra:          { phase: "married", base: [6, 12], exp: 35, likes: ["deredere", "onee-san"], dislikes: [], text: (n) => `👩‍❤️‍👨 Kalian menghabiskan malam dengan mesra sebagai pasangan suami istri.` },
  rayu:           { phase: "married", base: [8, 14], exp: 40, likes: ["tsundere", "deredere"], dislikes: [], text: (n) => `💌 Kata-kata rayuanmu membuat *${n}* tersipu dan memelukmu.` },
  hadiah:         { phase: "married", base: [10, 16], exp: 45, likes: ["himedere", "ojou-sama"], dislikes: [], text: (n) => `🎁 Hadiah kejutan membuat *${n}* melompat bahagia seperti anak kecil.` },
  bulanmadu_pantai: { phase: "married", base: [12, 20], exp: 50, likes: ["genki", "amayadori"], dislikes: [], text: (n) => `🌴 Bulan madu di pantai tropis. *${n}* menggandengmu menikmati sunset.` },
  bulanmadu_hotel:  { phase: "married", base: [14, 24], exp: 55, likes: ["onee-san", "yandere"], dislikes: [], text: (n) => `🏨 Suite hotel mewah. Malam penuh kehangatan berdua saja.` },
};

// personality like/dislike tag mapping: deredere menyukai semua aksi mesra, dst.
const PERSONALITY_TAG = {
  tsundere: { like: ["jalan_taman", "piknik_taman"], dislike: ["rayu"] },
  deredere: { like: ["peluk_depan", "cium_bibir", "mesra"], dislike: [] },
  kuudere:  { like: ["kafe_kopi", "bioskop_romantis", "piknik_taman"], dislike: ["arcade_duo"] },
  dandere:  { like: ["kafe_matcha", "masak_bareng"], dislike: ["jalan_mall"] },
  yandere:  { like: ["peluk_belakang", "cium_kening", "cium_bibir"], dislike: ["jalan_mall"] },
  "onee-san": { like: ["masak_bareng", "tidur_kelon"], dislike: [] },
  genki:    { like: ["karaoke_duet", "arcade_duo", "piknik_pantai"], dislike: ["kafe_kopi"] },
  himedere: { like: ["belanja_perhiasan", "hadiah"], dislike: ["jalan_taman"] },
  "ojou-sama": { like: ["belanja_perhiasan", "kafe_kue"], dislike: ["jalan_pantai"] },
  amayadori: { like: ["tidur_kelon", "piknik_taman"], dislike: ["karaoke_solo"] },
};

const EVENTS = [
  { id: "rain", phase: "any", text: (n) => `🌧️ Hujan deras turun! Kalian berbagi satu payung, bahu menempel bahu. *${n}* tersenyum malu.`, aff: 5 },
  { id: "wallet", phase: "any", text: () => `💸 Kamu menemukan dompet di jalan dan mengembalikannya. Rezeki mengalir!`, koin: () => 1000 + Math.floor(Math.random() * 24001) },
  { id: "rival", phase: "any", text: (n) => `😠 Seorang rival mendekati *${n}*! Ia gelisah dan mood-nya turun.`, aff: -4, yandereAff: 5 },
  { id: "idol", phase: "any", text: (n) => `🎤 Idola favoritnya lewat di jalanan! *${n}* kegirangan dan mood-nya naik.`, aff: 6, mood: "ceria" },
  { id: "cat", phase: "any", text: () => `🐱 Seekor kucing lucu tersangkut di pohon. Kalian menyelamatkannya!`, aff: (p) => (p === "deredere" || p === "genki" ? 9 : 4) },
  { id: "lottery", phase: "any", text: () => `🎰 Tiket lotre jatuh dari langit! Kamu coba keberuntunganmu...`, koin: () => (Math.random() < 0.5 ? 20000 + Math.floor(Math.random() * 40001) : 0) },
  { id: "late", phase: "any", text: (n) => `📉 Kamu hampir telat janji temu! *${n}* cemberut sepanjang hari.`, mood: "sedih", nextMult: 0.8 },
  { id: "anniv", phase: "any", marriedOnly: true, text: () => `💍 Kenangan hari pernikahan kalian teringat kembali. Kalian terharu bersama.`, aff: 6 },
  { id: "intimate", phase: "intim", text: (n) => `🔥 Momen kalian berlanjut. *${n}* berbisik pelan, "jangan berhenti..."`, aff: 10 },
  { id: "rainbow", phase: "approach", text: () => `🌈 Pelangi muncul setelah hujan reda. Kalian berhenti untuk mengabadikannya.`, aff: 5 },
  { id: "confess", phase: "intim", text: (n) => `💞 *${n}* mengaku bahwa ia mulai benar-benar mencintaimu.`, aff: 8 },
  { id: "gift", phase: "married", marriedOnly: true, text: () => `🎁 Hadiah kejutan kecil untukmu sebagai pasangan.`, koin: () => 2000 + Math.floor(Math.random() * 9001) },
];

export function rollWaifu(pityCounter = 0, rng = Math.random) {
  const pool = getPool();
  let eligible = pool;
  if (pityCounter >= PITY_THRESHOLD) {
    eligible = pool.filter(w => TIER_ORDER.indexOf(w.tier) >= 2);
  }
  const total = eligible.reduce((s, w) => s + w.weight, 0);
  let r = rng() * total;
  for (const w of eligible) { r -= w.weight; if (r <= 0) return w; }
  return eligible[eligible.length - 1];
}

export function applyAction(key, waifu, moodType = "biasa", rng = Math.random, multOverride = 1) {
  const a = ACTIONS[key];
  if (!a || !waifu || !waifu.personality) return null;
  const [min, max] = a.base;
  let base = min + rng() * (max - min);
  let mult = MOOD_MULT[moodType] || 1;
  mult *= multOverride;
  const tag = PERSONALITY_TAG[waifu.personality] || { like: [], dislike: [] };
  if (a.likes.includes(waifu.personality) || tag.like.includes(key)) mult *= 1.2;
  else if (a.dislikes.includes(waifu.personality) || tag.dislike.includes(key)) mult *= 0.7;
  const change = Math.max(1, Math.round(base * mult));
  return { key, phase: a.phase, change, exp: a.exp, text: a.text(waifu.name) };
}

export function rollEvent({ married, phase, personality, name } = {}, rng = Math.random) {
  if (rng() > EVENT_CHANCE) return null;
  const pool = EVENTS.filter(e => {
    if (e.marriedOnly && !married) return false;
    if (e.phase !== "any" && e.phase !== phase) return false;
    return true;
  });
  if (!pool.length) return null;
  const e = pool[Math.floor(rng() * pool.length)];
  let aff = typeof e.aff === "function" ? e.aff(personality) : (e.aff || 0);
  if (e.id === "rival" && personality === "yandere") aff = e.yandereAff;
  return { id: e.id, text: typeof e.text === "function" ? e.text(name) : e.text, aff, koin: e.koin ? e.koin() : 0, mood: e.mood || null, nextMult: e.nextMult || 1 };
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

const WAIFU_COMMANDS = new Set([
  "gachawaifu", "gachaistri", "waifuaction", "tinggalinwaifu", "waifuku",
  "istriku", "waifualbum", "albumwaifu", "waifupool", "poolwaifu",
]);
const JEALOUSY_COOLDOWN = 45 * 60 * 1000;

export async function jealousyCheck({ m, sock, db, command }) {
  const cmd = String(command || "").toLowerCase();
  if (WAIFU_COMMANDS.has(cmd)) return false;
  const user = db.getUser(m.sender);
  if (!user || !user.waifu) return false;
  const now = Date.now();
  if (now - (user.waifu.lastJealousAt || 0) < JEALOUSY_COOLDOWN) return false;
  const married = !!user.waifu.married;
  const chance = married ? 0.08 : 0.15;
  if (Math.random() > chance) return false;
  const drop = married
    ? Math.floor(Math.random() * 3) + 1
    : Math.floor(Math.random() * 6) + 3;
  user.waifu.affection = Math.max(0, (user.waifu.affection ?? 50) - drop);
  user.waifu.lastJealousAt = now;
  db.setUser(m.sender, user);
  const msg = married
    ? `💑 *${user.waifu.name}*: "Sayang, jangan lupa luangkan waktu untukku juga ya... 💕"\n💞 *Affection -${drop}*`
    : `😤 *${user.waifu.name}* cemburu! "Kamu main *${cmd}* terus, aku jadi tersisih... 😔"\n💞 *Affection -${drop}*`;
  await m.reply(msg).catch(() => {});
  return true;
}

export function albumStats(history = [], stats = {}) {
  const total = stats.totalGacha || 0;
  const byTier = stats.byTier || {};
  const actual = (byTier.Common || 0) * TIER_VALUE.Common + (byTier.Rare || 0) * TIER_VALUE.Rare + (byTier.Epic || 0) * TIER_VALUE.Epic + (byTier.Legendary || 0) * TIER_VALUE.Legendary + (byTier.Mythic || 0) * TIER_VALUE.Mythic;
  const luck = total > 0 ? actual / total / TIER_EXPECTED : 0;
  return {
    total,
    byTier: stats.byTier || {},
    pityCounter: stats.pityCounter || 0,
    rarest: stats.rarest || null,
    marriedCount: stats.marriedCount || 0,
    luck: Number(luck.toFixed(2)),
    last10: history.slice(-10).reverse(),
  };
}
