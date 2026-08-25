// Logika inti gacha husbu. Pure-ish, tanpa dependensi WA/Baileys.
import { getPool } from "../../data/husbu/index.js";
import { DOWRY, TIER_ORDER, TIER_VALUE, TIER_EXPECTED, PITY_THRESHOLD, EVENT_CHANCE, MOOD_MULT, getDailyMood } from "./ourin-romance.js";

export { DOWRY, PITY_THRESHOLD, EVENT_CHANCE, getDailyMood };

const HUSB_PERSONALITY_TAG = {
  tsundere: { like: ["jalan_taman", "piknik_taman"], dislike: ["rayu"] },
  playboy: { like: ["belanja_perhiasan", "cium_bibir", "mesra"], dislike: ["olahraga_panjat"] },
  kuudere: { like: ["kafe_kopi", "bioskop_romantis", "piknik_taman"], dislike: ["arcade_duo"] },
  dandere: { like: ["kafe_matcha", "masak_bareng"], dislike: ["jalan_mall"] },
  yandere: { like: ["peluk_belakang", "cium_kening", "cium_bibir"], dislike: ["jalan_mall"] },
  "oji-san": { like: ["masak_bareng", "tidur_kelon", "alam_mancing"], dislike: [] },
  genki: { like: ["karaoke_duet", "arcade_duo", "piknik_pantai"], dislike: ["kafe_kopi"] },
  prince: { like: ["belanja_perhiasan", "hadiah"], dislike: ["jalan_pantai"] },
  badboy: { like: ["olahraga_panjat", "bioskop_horor"], dislike: ["piknik_taman"] },
  sunao: { like: ["jalan_pantai", "piknik_pantai", "masak_bareng"], dislike: ["karaoke_solo"] },
  femboy: { like: ["kafe_kue", "kafe_susu", "piknik_taman", "belanja_boneka", "tidur_kelon"], dislike: ["olahraga_panjat", "bioskop_horor"] },
};

const HUSB_ACTIONS = {
  // ===== approach (<80) =====
  jalan_taman:   { phase: "approach", base: [10, 18], exp: 40, likes: ["sunao", "dandere", "kuudere", "genki", "femboy"], dislikes: ["badboy", "prince"], text: (n) => `🌳 Kamu ajak *${n}* jalan di taman. Angin sejuk, dia menggenggam tanganmu sambil tersenyum.` },
  jalan_mall:    { phase: "approach", base: [4, 10], exp: 25, likes: ["playboy"], dislikes: ["dandere", "kuudere", "genki"], text: (n) => `🏢 Mall ramai. *${n}* cuma mengekor pelan, terlihat bosan.` },
  jalan_pantai:  { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "sunao", "kuudere"], dislikes: ["prince"], text: (n) => `🏖️ Ombak pantai. *${n}* berlari-lari di pasir dan mengajakmu main air.` },
  jalan_kota:    { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "playboy"], dislikes: ["dandere", "femboy"], text: (n) => `🌆 Jalan kaki di tepi kota saat lampu mulai menyala. *${n}* menatapmu kagum.` },
  kafe_kopi:     { phase: "approach", base: [4, 8], exp: 25, likes: ["kuudere"], dislikes: ["prince"], text: (n) => `☕ *${n}* meringis meneguk kopi pahit. Ternyata dia lebih suka manis.` },
  kafe_matcha:   { phase: "approach", base: [10, 18], exp: 40, likes: ["dandere", "playboy", "kuudere"], dislikes: [], text: (n) => `🍵 Kamu traktir matcha latte. *${n}* tersenyum manis padamu.` },
  kafe_kue:      { phase: "approach", base: [8, 15], exp: 35, likes: ["playboy", "femboy", "genki"], dislikes: [], text: (n) => `🍰 Berbagi sepotong kue. *${n}* tak mau kalah rebutan minta gigitan.` },
  kafe_susu:     { phase: "approach", base: [5, 10], exp: 25, likes: ["femboy", "sunao"], dislikes: [], text: (n) => `🥛 Segelas susu hangat. *${n}* menyeruput pelan dan tampak nyaman.` },
  bioskop_romantis: { phase: "approach", base: [8, 16], exp: 35, likes: ["playboy", "kuudere"], dislikes: ["genki"], text: (n) => `🎬 Film romantis. *${n}* merangkulmu makin erat saat adegan ciuman.` },
  bioskop_horor: { phase: "approach", base: [10, 20], exp: 40, likes: ["genki", "tsundere"], dislikes: ["dandere", "kuudere"], text: (n) => `👻 JUMPSCARE! *${n}* pura-pura tenang padahal tangannya mencengkerammu.` },
  bioskop_animasi: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "sunao"], dislikes: ["prince"], text: (n) => `🎨 Film animasi. *${n}* tertawa lepas di beberapa adegan.` },
  belanja_baju:  { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "playboy"], dislikes: ["femboy"], text: (n) => `👕 Kamu pilihkan baju keren buat *${n}*. Dia memamerkannya sambil pipi merona.` },
  belanja_perhiasan: { phase: "approach", base: [10, 18], exp: 40, likes: ["playboy", "prince"], dislikes: ["kuudere"], text: (n) => `💍 Dia membelikanmu aksesoris manis. Matanya berbinar saat kamu memakainya.` },
  belanja_boneka: { phase: "approach", base: [5, 10], exp: 25, likes: ["femboy", "genki"], dislikes: ["prince"], text: (n) => `🧸 Boneka kecil mungil. *${n}* malu-malu tapi tetap memeluknya.` },
  karaoke_duet:  { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "playboy"], dislikes: ["kuudere", "tsundere"], text: (n) => `🎤 Duet lagu favorit. *${n}* semangat sampai fals nada tapi tertawa.` },
  karaoke_solo:  { phase: "approach", base: [4, 10], exp: 25, likes: ["tsundere"], dislikes: ["genki"], text: (n) => `🎵 *${n}* menyanyi sendirian dengan suara merdu. Kamu terpukau.` },
  arcade_duo:    { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "tsundere"], dislikes: ["kuudere"], text: (n) => `🕹️ Adu skor di arcade. *${n}* menantangmu lagi dan lagi, tak mau kalah.` },
  arcade_boneka: { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "playboy"], dislikes: [], text: (n) => `🎯 *${n}* memenangkan boneka untukmu. Wajahnya berseri-seri.` },
  piknik_taman:  { phase: "approach", base: [8, 15], exp: 35, likes: ["kuudere", "sunao", "femboy"], dislikes: ["badboy"], text: (n) => `🧺 Piknik di atas tikar rumput. *${n}* bersandar santai menatap awan.` },
  piknik_pantai: { phase: "approach", base: [10, 18], exp: 40, likes: ["genki", "sunao"], dislikes: ["prince"], text: (n) => `🏝️ Piknik pantai dengan bekal buatanmu. *${n}* sangat menikmatinya.` },
  masak_bareng:  { phase: "approach", base: [8, 16], exp: 35, likes: ["sunao", "oji-san", "playboy"], dislikes: [], text: (n) => `🍳 Masak bareng di dapur. *${n}* antusias mengaduk sausnya.` },
  masak_kue:     { phase: "approach", base: [10, 18], exp: 40, likes: ["sunao", "oji-san", "femboy"], dislikes: [], text: (n) => `🍰 Memanggang kue bersama. *${n}* bangga dengan hasil kreasimu berdua.` },
  restoran_makan:  { phase: "approach", base: [8, 16], exp: 35, likes: ["oji-san", "playboy", "femboy"], dislikes: ["kuudere"], text: (n) => `🍽️ Makan malam di restoran romantis. *${n}* menatapmu lembut dari seberang meja.` },
  restoran_dimsum: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "playboy"], dislikes: ["prince"], text: (n) => `🥟 Cicip dimsum bareng! *${n}* berebutan denganmu sampai tawa.` },
  restoran_bbq:    { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "oji-san"], dislikes: ["prince"], text: (n) => `🍖 Bakar-bakar di BBQ. *${n}* memangangkan daging untukmu dengan telaten.` },
  olahraga_hiking: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "kuudere"], dislikes: ["playboy", "prince"], text: (n) => `⛰️ Mendaki bukit. *${n}* terengah-engah tapi semangat dan menjaga langkahmu.` },
  olahraga_lari:   { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "tsundere"], dislikes: ["femboy"], text: (n) => `🏃 Lari pagi keliling taman. *${n}* menyesuaikan kecepatannya untukmu.` },
  olahraga_panjat: { phase: "approach", base: [5, 10], exp: 25, likes: ["genki", "badboy"], dislikes: ["prince", "femboy"], text: (n) => `🧗 Panjat tebing indoor. *${n}* merayap pelan demi mengamankanmu.` },
  alam_camping:    { phase: "approach", base: [10, 18], exp: 40, likes: ["kuudere", "sunao", "dandere"], dislikes: ["playboy"], text: (n) => `🏕️ Camping di bawah bintang. *${n}* bersandar santai menatap langit malam.` },
  alam_mancing:    { phase: "approach", base: [6, 12], exp: 30, likes: ["kuudere", "oji-san"], dislikes: ["genki"], text: (n) => `🎣 Memancing di danau tenang. *${n}* senang saat akhirnya dapat ikan.` },
  alam_perahu:     { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "sunao"], dislikes: ["tsundere"], text: (n) => `⛵ Menyusuri sungai naik perahu. *${n}* menutup mata menikmati angin.` },
  seni_museum:     { phase: "approach", base: [6, 12], exp: 30, likes: ["kuudere", "dandere"], dislikes: ["genki"], text: (n) => `🖼️ Menjelajahi museum. *${n}* takjub di depan lukisan tua.` },
  seni_melukis:    { phase: "approach", base: [8, 15], exp: 35, likes: ["playboy", "sunao", "oji-san"], dislikes: [], text: (n) => `🎨 Melukis bareng di studio. Karya *${n}* kacau balau tapi lucu.` },
  seni_konser:     { phase: "approach", base: [10, 18], exp: 40, likes: ["genki", "playboy", "sunao"], dislikes: ["kuudere"], text: (n) => `🎸 Nonton konser. *${n}* bernyanyi keras tanpa malu.` },

  // ===== intim (80–99) =====
  peluk_belakang: { phase: "intim", base: [10, 18], exp: 45, likes: ["yandere", "playboy", "tsundere"], dislikes: [], text: (n) => `🤗 Kamu memeluk *${n}* dari belakang. Dia terkejut sesaat lalu tersenyum aman.` },
  peluk_depan:    { phase: "intim", base: [12, 20], exp: 50, likes: ["yandere", "playboy", "oji-san"], dislikes: [], text: (n) => `💑 Kalian berpelukan erat. Detak jantung berdebar seirama.` },
  cium_kening:    { phase: "intim", base: [10, 18], exp: 45, likes: ["dandere", "tsundere", "yandere"], dislikes: [], text: (n) => `😚 Kamu mengecup keningnya. *${n}* memejamkan mata bahagia.` },
  cium_bibir:     { phase: "intim", base: [14, 24], exp: 55, likes: ["yandere", "playboy"], dislikes: ["kuudere"], text: (n) => `💋 Ciuman lembut dan panjang. *${n}* membalas dengan desahan kecil.` },
  tidur_kelon:    { phase: "intim", base: [10, 16], exp: 45, likes: ["femboy", "oji-san"], dislikes: [], text: (n) => `🛏️ Kamu merebahkan kepala di dada *${n}* yang bidang. Dia mengusap rambutmu pelan.` },
  tidur_serandu:  { phase: "intim", base: [12, 20], exp: 50, likes: ["oji-san", "femboy", "yandere"], dislikes: [], text: (n) => `🌙 Kalian berpelukan erat di ranjang. *${n}* berbisik bahwa dia merasa aman.` },
  mandi_punggung: { phase: "intim", base: [10, 16], exp: 45, likes: ["oji-san", "kuudere"], dislikes: [], text: (n) => `🛁 Kamu membasuh punggung *${n}* dengan lembut. Dia mendesah rileks.` },
  mandi_bahu:     { phase: "intim", base: [12, 20], exp: 50, likes: ["playboy", "oji-san"], dislikes: [], text: (n) => `🧼 *${n}* menunduk malu saat kamu mengusap bahunya di air hangat.` },
  gendong_putri:  { phase: "intim", base: [10, 18], exp: 45, likes: ["playboy", "yandere", "tsundere"], dislikes: [], text: (n) => `👸 *${n}* menggendongmu dengan gaya putri. Kamu memeluk lehernya erat.` },
  gendong_punggung: { phase: "intim", base: [8, 14], exp: 40, likes: ["genki", "sunao"], dislikes: [], text: (n) => `🏃 *${n}* menggendongmu di punggung sambil tertawa-tawa.` },
  tepuk_kepala:   { phase: "intim", base: [6, 12], exp: 35, likes: ["dandere", "tsundere"], dislikes: [], text: (n) => `🖐️ Kamu menepuk kepala *${n}* pelan. Dia mendengus tapi pipinya memerah.` },
  pijat_bahu:     { phase: "intim", base: [10, 18], exp: 45, likes: ["oji-san", "yandere", "playboy"], dislikes: [], text: (n) => `💆 Kamu memijat bahu *${n}* yang tegang. Dia mendesah rileks.` },

  // ===== married =====
  mesra:          { phase: "married", base: [6, 12], exp: 35, likes: ["playboy", "oji-san"], dislikes: [], text: (n) => `👩‍❤️‍👨 Kalian menghabiskan malam dengan mesra sebagai pasangan suami istri.` },
  rayu:           { phase: "married", base: [8, 14], exp: 40, likes: ["tsundere", "playboy"], dislikes: [], text: (n) => `💌 Kata-kata rayuanmu membuat *${n}* tersipu dan memelukmu.` },
  hadiah:         { phase: "married", base: [10, 16], exp: 45, likes: ["playboy", "prince"], dislikes: [], text: (n) => `🎁 Hadiah kejutan untuk suami tercinta. *${n}* tersenyum bangga.` },
  bulanmadu_pantai: { phase: "married", base: [12, 20], exp: 50, likes: ["genki", "sunao"], dislikes: [], text: (n) => `🌴 Bulan madu di pantai tropis. *${n}* menggandengmu menikmati sunset.` },
  bulanmadu_hotel:  { phase: "married", base: [14, 24], exp: 55, likes: ["oji-san", "yandere"], dislikes: [], text: (n) => `🏨 Suite hotel mewah. Malam penuh kehangatan berdua saja.` },
  nontonrumah:    { phase: "married", base: [8, 14], exp: 40, likes: ["playboy", "dandere", "tsundere"], dislikes: [], text: (n) => `📺 Nonton maraton film di sofa. *${n}* nyender di bahumu sampai ketiduran.` },
  jalanpagi:      { phase: "married", base: [10, 16], exp: 45, likes: ["genki", "oji-san", "sunao"], dislikes: [], text: (n) => `🌅 Jalan santai pagi hari sambil beli sarapan. *${n}* menggandengmu.` },
};

const HUSB_EVENTS = [
  { id: "rain", phase: "any", text: (n) => `🌧️ Hujan deras turun! *${n}* berbagi jaket denganmu, bahu menempel bahu.`, aff: 5 },
  { id: "wallet", phase: "any", text: () => `💸 Kamu menemukan dompet di jalan dan mengembalikannya. Rezeki mengalir!`, koin: () => 1000 + Math.floor(Math.random() * 24001) },
  { id: "rival", phase: "any", text: (n) => `😠 Seseorang mendekatimu! *${n}* gelisah dan mood-nya turun.`, aff: -4, yandereAff: 5 },
  { id: "idol", phase: "any", text: (n) => `🎮 Toko game favoritnya buka! *${n}* semangat dan mood-nya naik.`, aff: 6, mood: "ceria" },
  { id: "cat", phase: "any", text: () => `🐱 Seekor kucing lucu tersangkut di pohon. Kalian menyelamatkannya!`, aff: (p) => (p === "playboy" || p === "genki" ? 9 : 4) },
  { id: "lottery", phase: "any", text: () => `🎰 Tiket lotre jatuh dari langit! Kamu coba keberuntunganmu...`, koin: () => (Math.random() < 0.5 ? 20000 + Math.floor(Math.random() * 40001) : 0) },
  { id: "late", phase: "any", text: (n) => `📉 Kamu hampir telat janji temu! *${n}* cemberut sepanjang hari.`, mood: "sedih", nextMult: 0.8 },
  { id: "anniv", phase: "any", marriedOnly: true, text: () => `💍 Kenangan hari pernikahan kalian teringat kembali. Kalian terharu bersama.`, aff: 6 },
  { id: "intimate", phase: "intim", text: (n) => `🔥 Momen kalian berlanjut. *${n}* berbisik pelan, "jangan berhenti..."`, aff: 10 },
  { id: "rainbow", phase: "approach", text: () => `🌈 Pelangi muncul setelah hujan reda. Kalian berhenti untuk mengabadikannya.`, aff: 5 },
  { id: "confess", phase: "intim", text: (n) => `💞 *${n}* mengaku bahwa dia mulai benar-benar mencintaimu.`, aff: 8 },
  { id: "gift", phase: "married", marriedOnly: true, text: () => `🎁 Hadiah kejutan kecil untukmu sebagai pasangan.`, koin: () => 2000 + Math.floor(Math.random() * 9001) },
  { id: "foto", phase: "any", text: (n) => `📸 Foto bareng! *${n}* melotot saat kejepret momen memalukan.`, aff: 4 },
  { id: "lomba", phase: "any", text: () => `🏅 Kamu ikut lomba jalan santai dan menang hadiah receh!`, koin: () => 500 + Math.floor(Math.random() * 5001) },
  { id: "salahpaham", phase: "any", text: (n) => `📵 *${n}* melihat notif dari akun lain di ponselmu dan salah paham!`, aff: -5, anger: 12 },
  { id: "pujian", phase: "any", text: (n) => `🗣️ Seseorang memuji kalian: "cocok banget!" *${n}* tersipu.`, aff: 5 },
  { id: "sunset", phase: "any", text: () => `🌇 Kalian berhenti menikmati matahari terbenam. Momen tak terlupakan.`, aff: 6, mood: "romantis" },
  { id: "kenangan", phase: "any", marriedOnly: true, text: (n) => `📿 Kamu menemukan foto lama kalian. *${n}* terharu dan memelukmu erat.`, aff: 7 },
  { id: "badai", phase: "any", text: (n) => `⛈️ Hujan badai datang! *${n}* kehujanan dan mood-nya rusak.`, mood: "marah", aff: -4 },
  { id: "rezeki", phase: "any", text: () => `🍀 Rezeki nomplok! Kamu dapat uang tak terduga.`, koin: () => 3000 + Math.floor(Math.random() * 8001) },
];

export function rollHusbu(pityCounter = 0, rng = Math.random) {
  const pool = getPool();
  let eligible = pool;
  if (pityCounter >= PITY_THRESHOLD) {
    eligible = pool.filter(h => TIER_ORDER.indexOf(h.tier) >= 2);
  }
  const total = eligible.reduce((s, h) => s + h.rollWeight, 0);
  let r = rng() * total;
  for (const h of eligible) { r -= h.rollWeight; if (r <= 0) return h; }
  return eligible[eligible.length - 1];
}

export function applyAction(key, husbu, moodType = "biasa", rng = Math.random, multOverride = 1) {
  const a = HUSB_ACTIONS[key];
  if (!a || !husbu || !husbu.personality) return null;
  const [min, max] = a.base;
  let base = min + rng() * (max - min);
  let mult = MOOD_MULT[moodType] || 1;
  mult *= multOverride;
  const tag = HUSB_PERSONALITY_TAG[husbu.personality] || { like: [], dislike: [] };
  const liked = a.likes.includes(husbu.personality) || tag.like.includes(key);
  const disliked = a.dislikes.includes(husbu.personality) || tag.dislike.includes(key);
  if (liked) mult *= 1.2;
  else if (disliked) mult *= 0.7;
  const change = Math.max(1, Math.round(base * mult));
  return { key, phase: a.phase, change, exp: a.exp, text: a.text(husbu.name), like: liked, dislike: disliked };
}

export function rollEvent({ married, phase, personality, name } = {}, rng = Math.random) {
  if (rng() > EVENT_CHANCE) return null;
  const pool = HUSB_EVENTS.filter(e => {
    if (e.marriedOnly && !married) return false;
    if (e.phase !== "any" && e.phase !== phase) return false;
    return true;
  });
  if (!pool.length) return null;
  const e = pool[Math.floor(rng() * pool.length)];
  let aff = typeof e.aff === "function" ? e.aff(personality) : (e.aff || 0);
  if (e.id === "rival" && personality === "yandere") aff = e.yandereAff;
  return { id: e.id, text: typeof e.text === "function" ? e.text(name) : e.text, aff, koin: e.koin ? e.koin() : 0, mood: e.mood || null, nextMult: e.nextMult || 1, anger: e.anger || 0 };
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
