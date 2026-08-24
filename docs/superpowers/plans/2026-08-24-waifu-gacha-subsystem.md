# Waifu Gacha Subsystem v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hardcoded 40-waifu `plugins/fun/gachawaifu.js` with modular subsystem: 300+ waifu data files, 5-tier pity gacha, random events, jealousy trigger, album/luck stats, mood+anniversary, expanded actions, economy integration, Turso-compatible.

**Architecture:** Data separated into `data/waifu/*.js` franchise files loaded by `data/waifu/index.js` (validates, dedupes, computes per-entry weight). Pure-ish logic in `src/lib/ourin-waifu.js` (roll/action/event/mood/jealousy/album). Three thin plugin files handle WA I/O + routing. One-line jealousy hook in `src/handler.js`. All state lives on user object via existing `getUser/setUser` dual-write — no new tables.

**Tech Stack:** Node ESM, Baileys interactiveMessage quick_reply, `node:test`, existing `ourin-database.js` / `ourin-level.js` / `ourin-error.js`.

**Spec:** `docs/superpowers/specs/2026-08-24-waifu-gacha-subsystem-design.md`

## Global Constraints

- ≥300 waifu entries total, no duplicate names.
- 5 tiers only: `Common`(55%) `Rare`(25%) `Epic`(13%) `Legendary`(5.5%) `Mythic`(1.5%).
- 10 personalities only: `tsundere deredere kuudere dandere yandere onee-san genki himedere ojou-sama amayadori`.
- Pity: 20 rolls without Epic+ → guaranteed Epic+ (then reset). Lifetime counter in `user.waifuStats.pityCounter`.
- Events: 18% chance after action (not after navigation/menu).
- Mood multiplier: ceria ×1.3, romantis ×1.2, biasa ×1.0, sedih ×0.7, marah ×0.5.
- Affection clamp 0–100. Actions: personality mult (like ×1.2 / dislike ×0.7) × mood mult.
- Jealousy hook: only for `fun`/`game` category, skip waifu commands, 45min cooldown, unmarried 15% (−3..−8), married 8% (−1..−3).
- All reward writes use `db.updateKoin`, `db.updateEnergi`, `await addExpWithLevelCheck(sock, m, db, user, exp)`.
- No new SQL tables. No NSFW content. Image fallback chain: `imageUrl` cache → Pinterest API → default URL.
- Tests run with `npm test` (`node --test tests/`). Plugin handlers not unit-tested.

---

### Task 1: Data loader + template + data validation test

**Files:**
- Create: `tests/waifu-data.test.mjs`
- Create: `data/waifu/_template.js`
- Create: `data/waifu/index.js`

**Interfaces:**
- Produces: `getPool()` → validated array of waifu entries (each with `weight`), `searchPool(query)`, exported consts `TIER_WEIGHTS`, `VALID_PERSONALITIES`.
- Consumes: `logger.warn` from `../../src/lib/ourin-logger.js`.

- [ ] **Step 1: Write the failing test**

```js
// tests/waifu-data.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert";
import { getPool, searchPool, TIER_WEIGHTS, VALID_PERSONALITIES } from "../data/waifu/index.js";

describe("Waifu data pool", () => {
  it("loads at least 300 entries", () => {
    assert.ok(getPool().length >= 300, `pool = ${getPool().length}`);
  });
  it("has no duplicate names", () => {
    const names = getPool().map(w => w.name.toLowerCase());
    assert.equal(new Set(names).size, names.length);
  });
  it("all entries have required fields", () => {
    for (const w of getPool()) {
      assert.ok(w.name && w.series && w.age && w.height && w.weight, `missing field: ${w.name}`);
    }
  });
  it("all tiers valid", () => {
    for (const w of getPool()) assert.ok(TIER_WEIGHTS[w.tier], `bad tier: ${w.tier} ${w.name}`);
  });
  it("all personalities valid", () => {
    for (const w of getPool()) assert.ok(VALID_PERSONALITIES.includes(w.personality), `bad personality: ${w.personality} ${w.name}`);
  });
  it("tier distribution near target", () => {
    const counts = {};
    for (const w of getPool()) counts[w.tier] = (counts[w.tier] || 0) + 1;
    assert.ok(counts.Common >= 90 && counts.Common <= 170, `Common=${counts.Common}`);
    assert.ok(counts.Rare >= 70 && counts.Rare <= 125, `Rare=${counts.Rare}`);
    assert.ok(counts.Epic >= 40 && counts.Epic <= 75, `Epic=${counts.Epic}`);
    assert.ok(counts.Legendary >= 15 && counts.Legendary <= 45, `Legendary=${counts.Legendary}`);
    assert.ok(counts.Mythic >= 5 && counts.Mythic <= 20, `Mythic=${counts.Mythic}`);
  });
  it("searchPool filters by name/series/tier/personality", () => {
    const pool = getPool();
    const n = pool[0];
    assert.ok(searchPool(n.name).some(w => w.name === n.name));
    assert.ok(searchPool(n.tier).every(w => w.tier === n.tier));
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/waifu-data.test.mjs`
Expected: FAIL — `data/waifu/index.js` module not found (throws ERR_MODULE_NOT_FOUND).

- [ ] **Step 3: Write `_template.js`**

```js
// data/waifu/_template.js
// Template untuk mengisi file franchise. Salin blok waifus ini ke file franchise baru,
// lalu isi sesuai data. Wajib: name, series, age, height, weight, tier, personality.
// keyword opsional; default auto: `${name} ${series} anime`
export const waifus = [
  {
    name: "Miku Nakano",            // required, unik lintas franchise
    series: "Gotoubun no Hanayome", // required
    age: 17, height: "165 cm", weight: "49 kg",   // required
    tier: "Rare",                   // Common|Rare|Epic|Legendary|Mythic
    personality: "dandere",         // wajib, lihat tabel personality di spec
    keyword: "",                    // optional override keyword Pinterest
  },
];
```

- [ ] **Step 4: Write `data/waifu/index.js`**

```js
// data/waifu/index.js
// Loader pool waifu: merge file franchise, validasi entry, dedupe nama, hitung bobot tier.
import { logger } from "../../src/lib/ourin-logger.js";
import { waifus as naruto } from "./naruto.js";
import { waifus as onePiece } from "./one-piece.js";
import { waifus as bleach } from "./bleach.js";
import { waifus as kimetsu } from "./kimetsu.js";
import { waifus as jujutsu } from "./jujutsu-kaisen.js";
import { waifus as mha } from "./my-hero-academia.js";
import { waifus as dragonBall } from "./dragon-ball.js";
import { waifus as sao } from "./sword-art-online.js";
import { waifus as rezero } from "./rezero.js";
import { waifus as fate } from "./fate.js";
import { waifus as genshin } from "./genshin-impact.js";
import { waifus as hsr } from "./honkai-star-rail.js";
import { waifus as kaguya } from "./kaguya-sama.js";
import { waifus as konosuba } from "./konosuba.js";
import { waifus as chainsaw } from "./chainsaw-man.js";
import { waifus as spyFamily } from "./spy-x-family.js";
import { waifus as aot } from "./attack-on-titan.js";
import { waifus as violet } from "./violet-evergarden.js";
import { waifus as franxx } from "./darling-in-franxx.js";
import { waifus as steins } from "./steins-gate.js";
import { waifus as kOn } from "./k-on.js";
import { waifus as oshi } from "./oshi-no-ko.js";
import { waifus as dateAlive } from "./date-a-live.js";
import { waifus as dxd } from "./highschool-dxd.js";
import { waifus as nier } from "./nier.js";
import { waifus as cote } from "./classroom-of-the-elite.js";
import { waifus as frieren } from "./frieren.js";
import { waifus as loveLive } from "./love-live.js";
import { waifus as fairyTail } from "./fairy-tail.js";
import { waifus as komi } from "./komi.js";
import { waifus as dressUp } from "./my-dress-up-darling.js";
import { waifus as lycoris } from "./lycoris-recoil.js";
import { waifus as bocchi } from "./bocchi-the-rock.js";
import { waifus as mushoku } from "./mushoku-tensei.js";
import { waifus as dandadan } from "./dandadan.js";
import { waifus as toradora } from "./toradora.js";
import { waifus as monogatari } from "./monogatari.js";
import { waifus as nikke } from "./nikke.js";
import { waifus as blueArchive } from "./blue-archive.js";

export const TIER_WEIGHTS = { Common: 55, Rare: 25, Epic: 13, Legendary: 5.5, Mythic: 1.5 };
export const VALID_PERSONALITIES = [
  "tsundere", "deredere", "kuudere", "dandere", "yandere",
  "onee-san", "genki", "himedere", "ojou-sama", "amayadori",
];

const FRANCHISES = [
  naruto, onePiece, bleach, kimetsu, jujutsu, mha, dragonBall, sao, rezero,
  fate, genshin, hsr, kaguya, konosuba, chainsaw, spyFamily, aot, violet,
  franxx, steins, kOn, oshi, dateAlive, dxd, nier, cote, frieren, loveLive,
  fairyTail, komi, dressUp, lycoris, bocchi, mushoku, dandadan, toradora,
  monogatari, nikke, blueArchive,
];

let cache = null;

function buildPool() {
  const seen = new Set();
  const all = [];
  for (const list of FRANCHISES) {
    for (const w of list || []) {
      const missing = !w || !w.name || !w.series || !w.age || !w.height || !w.weight || !w.tier || !w.personality;
      if (missing) { logger.warn("waifu", `skip invalid entry (${w?.name || "no-name"})`); continue; }
      if (!TIER_WEIGHTS[w.tier]) { logger.warn("waifu", `skip invalid tier ${w.tier} (${w.name})`); continue; }
      if (!VALID_PERSONALITIES.includes(w.personality)) { logger.warn("waifu", `skip invalid personality ${w.personality} (${w.name})`); continue; }
      const key = w.name.trim().toLowerCase();
      if (seen.has(key)) { logger.warn("waifu", `duplicate name: ${w.name}`); continue; }
      seen.add(key);
      all.push({ ...w, name: w.name.trim(), series: w.series.trim(), keyword: w.keyword || `${w.name} ${w.series} anime` });
    }
  }
  const tierCount = {};
  for (const w of all) tierCount[w.tier] = (tierCount[w.tier] || 0) + 1;
  for (const w of all) w.weight = TIER_WEIGHTS[w.tier] / tierCount[w.tier];
  return all;
}

export function getPool() {
  if (!cache) cache = buildPool();
  return cache;
}

export function searchPool(query = "") {
  const q = String(query).toLowerCase();
  const pool = getPool();
  if (!q) return pool;
  return pool.filter(w =>
    w.name.toLowerCase().includes(q) ||
    w.series.toLowerCase().includes(q) ||
    w.tier.toLowerCase().includes(q) ||
    w.personality.toLowerCase().includes(q)
  );
}
```

- [ ] **Step 5: Run test to verify partial pass**

Run: `npm test -- tests/waifu-data.test.mjs`
Expected: FAIL on `≥300` (pool will be 0 until data files exist in Tasks 3–5). Field/validation tests pass only once data present. This is expected — documented intermediate state.

- [ ] **Step 6: Commit**

```bash
git add tests/waifu-data.test.mjs data/waifu/_template.js data/waifu/index.js
git commit -m "feat(waifu): data pool loader + validation test"
```

---

### Task 2: Core logic lib + lib tests

**Files:**
- Create: `tests/waifu-lib.test.mjs`
- Create: `src/lib/ourin-waifu.js`

**Interfaces:**
- Consumes: `getPool()` from `../../data/waifu/index.js`.
- Produces: `rollWaifu(pityCounter, rng)`, `applyAction(key, waifu, moodType, rng)`, `rollEvent({married, phase, personality}, rng)`, `getDailyMood(senderJid, dateStr)`, `jealousyCheck({m, sock, db, command})`, `albumStats(history, stats)`, `DOWRY`, `ACTIONS`.

- [ ] **Step 1: Write the failing test**

```js
// tests/waifu-lib.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert";
import {
  rollWaifu, applyAction, rollEvent, getDailyMood, jealousyCheck, albumStats, DOWRY,
} from "../src/lib/ourin-waifu.js";

function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

describe("Waifu lib", () => {
  it("rollWaifu with pity >= 20 returns Epic+", () => {
    const rng = mulberry32(7);
    for (let i = 0; i < 50; i++) {
      const w = rollWaifu(20, rng);
      assert.ok(["Epic", "Legendary", "Mythic"].includes(w.tier), w.tier);
    }
  });

  it("rollWaifu distribution sanity over 20000 rolls", () => {
    const rng = mulberry32(1234);
    const counts = { Common: 0, Rare: 0, Epic: 0, Legendary: 0, Mythic: 0 };
    for (let i = 0; i < 20000; i++) counts[rollWaifu(0, rng).tier]++;
    const pct = {};
    for (const t of Object.keys(counts)) pct[t] = counts[t] / 20000;
    assert.ok(pct.Common > 0.45 && pct.Common < 0.65, pct.Common);
    assert.ok(pct.Rare > 0.18 && pct.Rare < 0.32, pct.Rare);
    assert.ok(pct.Epic > 0.08 && pct.Epic < 0.18, pct.Epic);
    assert.ok(pct.Legendary > 0.02 && pct.Legendary < 0.09, pct.Legendary);
    assert.ok(pct.Mythic > 0.002 && pct.Mythic < 0.04, pct.Mythic);
  });

  it("applyAction applies personality & mood multipliers", () => {
    const waifu = { personality: "tsundere", name: "Miku" };
    // jalan_taman likes tsundere (×1.2), mood biasa (×1.0), rng 0 → base 10 → 12
    const r = applyAction("jalan_taman", waifu, "biasa", () => 0);
    assert.equal(r.change, 12);
    // mood marah ×0.5: 10 * 1.2 * 0.5 = 6
    const r2 = applyAction("jalan_taman", waifu, "marah", () => 0);
    assert.equal(r2.change, 6);
  });

  it("applyAction unknown key returns null", () => {
    assert.equal(applyAction("nope", {}, "biasa", () => 0), null);
  });

  it("rollEvent respects 18% chance and gates", () => {
    let hit = 0;
    for (let i = 0; i < 1000; i++) if (rollEvent({ married: false, phase: "approach", personality: "genki" }, mulberry32(i))) hit++;
    assert.ok(hit > 100 && hit < 260, `hit=${hit}`);
    // event intim phase never fires in approach
    for (let i = 0; i < 200; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki" }, mulberry32(i));
      if (e) assert.notEqual(e.phase, "intim");
    }
  });

  it("getDailyMood deterministic per day+jid", () => {
    assert.equal(getDailyMood("628123", "2026-08-24"), getDailyMood("628123", "2026-08-24"));
  });

  it("jealousyCheck respects cooldown and married chance", () => {
    let affection = 50;
    const user = { waifu: { name: "Miku", affection, married: false, lastJealousAt: 0 } };
    const db = {
      getUser: () => user,
      setUser: (jid, d) => { Object.assign(user, d); },
    };
    let replied = "";
    const m = { sender: "628123", reply: async (t) => { replied = t; } };
    const origRandom = Math.random;
    Math.random = () => 0.01; // force trigger
    // first call triggers
    jealousyCheck({ m, sock: {}, db, command: "menu" });
    assert.notEqual(replied, "");
    assert.ok(user.waifu.affection < 50);
    // second call within cooldown does nothing
    const before = user.waifu.affection;
    replied = "";
    jealousyCheck({ m, sock: {}, db, command: "menu" });
    assert.equal(replied, "");
    assert.equal(user.waifu.affection, before);
    // waifu commands skipped
    replied = "";
    user.waifu.lastJealousAt = 0;
    jealousyCheck({ m, sock: {}, db, command: "gachawaifu" });
    assert.equal(replied, "");
    Math.random = origRandom;
  });

  it("albumStats computes luck score", () => {
    const history = Array.from({ length: 100 }, () => ({ name: "C", series: "S", tier: "Common" }));
    const s = albumStats(history, { totalGacha: 100, byTier: { Common: 100 }, pityCounter: 3, rarest: { name: "C", tier: "Common" }, marriedCount: 0 });
    assert.ok(s.luck < 1, s.luck);
    const myth = albumStats([{ name: "M", series: "S", tier: "Mythic" }], { totalGacha: 1, byTier: { Mythic: 1 }, rarest: { name: "M", tier: "Mythic" }, marriedCount: 0 });
    assert.ok(myth.luck > 1, myth.luck);
  });

  it("DOWRY has all tiers", () => {
    for (const t of ["Common", "Rare", "Epic", "Legendary", "Mythic"]) assert.ok(DOWRY[t]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/waifu-lib.test.mjs`
Expected: FAIL — module `../src/lib/ourin-waifu.js` not found.

- [ ] **Step 3: Write `src/lib/ourin-waifu.js`**

```js
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
  { id: "idol", phase: "any", text: () => `🎤 Idola favoritnya lewat di jalanan! *${n}* kegirangan dan mood-nya naik.`, aff: 6, mood: "ceria" },
  { id: "cat", phase: "any", text: () => `🐱 Seekor kucing lucu tersangkut di pohon. Kalian menyelamatkannya!`, aff: (p) => (p === "deredere" || p === "genki" ? 9 : 4) },
  { id: "lottery", phase: "any", text: () => `🎰 Tiket lotre jatuh dari langit! Kamu coba keberuntunganmu...`, koin: () => (Math.random() < 0.5 ? 20000 + Math.floor(Math.random() * 40001) : 0) },
  { id: "late", phase: "any", text: () => `📉 Kamu hampir telat janji temu! *${n}* cemberut sepanjang hari.`, mood: "sedih", nextMult: 0.8 },
  { id: "anniv", phase: "any", marriedOnly: true, text: () => `💍 Kenangan hari pernikahan kalian teringat kembali. *${n}* terharu.`, aff: 6 },
  { id: "intimate", phase: "intim", text: (n) => `🔥 Momen kalian berlanjut. *${n}* berbisik pelan, "jangan berhenti..."`, aff: 10 },
  { id: "rainbow", phase: "approach", text: () => `🌈 Pelangi muncul setelah hujan reda. Kalian berhenti untuk mengabadikannya.`, aff: 5 },
  { id: "confess", phase: "intim", text: (n) => `💞 *${n}* mengaku bahwa ia mulai benar-benar mencintaimu.`, aff: 8 },
  { id: "gift", phase: "married", marriedOnly: true, text: () => `🎁 Hadiah kejutan kecil dari *${n}* untukmu.`, koin: () => 2000 + Math.floor(Math.random() * 9001) },
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

export function applyAction(key, waifu, moodType = "biasa", rng = Math.random) {
  const a = ACTIONS[key];
  if (!a || !waifu || !waifu.personality) return null;
  const [min, max] = a.base;
  let base = min + rng() * (max - min);
  let mult = MOOD_MULT[moodType] || 1;
  const tag = PERSONALITY_TAG[waifu.personality] || { like: [], dislike: [] };
  if (a.likes.includes(waifu.personality) || tag.like.includes(key)) mult *= 1.2;
  else if (a.dislikes.includes(waifu.personality) || tag.dislike.includes(key)) mult *= 0.7;
  const change = Math.max(1, Math.round(base * mult));
  return { key, phase: a.phase, change, exp: a.exp, text: a.text(waifu.name) };
}

export function rollEvent({ married, phase, personality } = {}, rng = Math.random) {
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
  return { id: e.id, text: e.text, aff, koin: e.koin ? e.koin() : 0, mood: e.mood || null, nextMult: e.nextMult || 1 };
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
  user.waifu.affection = Math.max(0, (user.waifu.affection || 50) - drop);
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
  const actual = history.reduce((s, h) => s + (TIER_VALUE[h.tier] || 1), 0);
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- tests/waifu-lib.test.mjs`
Expected: PASS (rollWaifu needs data pool — Tasks 3–5 must exist first; if data files missing, distribution test may fail on empty pool. Order note: lib code is written here, data batches follow. If pool empty, `getPool()` returns `[]`, rollWaifu crashes — run data tasks first if lib test must pass standalone.)

- [ ] **Step 5: Commit**

```bash
git add tests/waifu-lib.test.mjs src/lib/ourin-waifu.js
git commit -m "feat(waifu): core logic lib + tests"
```

---

### Task 3: Franchise data batch A (naruto → kaguya)

**Files:**
- Create: `data/waifu/naruto.js`, `data/waifu/one-piece.js`, `data/waifu/bleach.js`, `data/waifu/kimetsu.js`, `data/waifu/jujutsu-kaisen.js`, `data/waifu/my-hero-academia.js`, `data/waifu/dragon-ball.js`, `data/waifu/sword-art-online.js`, `data/waifu/rezero.js`, `data/waifu/fate.js`, `data/waifu/genshin-impact.js`, `data/waifu/honkai-star-rail.js`, `data/waifu/kaguya-sama.js`

**Interfaces:**
- Consumes: file names must match imports in `data/waifu/index.js` (already wired).
- Produces: each file `export const waifus = [...]` with valid fields.

- [ ] **Step 1: Write `data/waifu/naruto.js`**

```js
export const waifus = [
  { name: "Hinata Hyuga", series: "Naruto", age: 16, height: "160 cm", weight: "45 kg", tier: "Rare", personality: "dandere" },
  { name: "Sakura Haruno", series: "Naruto", age: 16, height: "161 cm", weight: "46 kg", tier: "Common", personality: "tsundere" },
  { name: "Ino Yamanaka", series: "Naruto", age: 16, height: "162 cm", weight: "48 kg", tier: "Common", personality: "deredere" },
  { name: "Tenten", series: "Naruto", age: 17, height: "164 cm", weight: "47 kg", tier: "Common", personality: "genki" },
  { name: "Temari", series: "Naruto", age: 18, height: "165 cm", weight: "48 kg", tier: "Common", personality: "onee-san" },
  { name: "Konan", series: "Naruto", age: 35, height: "169 cm", weight: "47 kg", tier: "Common", personality: "kuudere" },
  { name: "Anko Mitarashi", series: "Naruto", age: 24, height: "167 cm", weight: "50 kg", tier: "Common", personality: "tsundere" },
  { name: "Kushina Uzumaki", series: "Naruto", age: 24, height: "165 cm", weight: "48 kg", tier: "Common", personality: "genki" },
  { name: "Karin", series: "Naruto", age: 18, height: "163 cm", weight: "47 kg", tier: "Common", personality: "yandere" },
  { name: "Sarada Uchiha", series: "Naruto", age: 12, height: "148 cm", weight: "39 kg", tier: "Rare", personality: "tsundere" },
  { name: "Kurenai Yuhi", series: "Naruto", age: 28, height: "169 cm", weight: "50 kg", tier: "Rare", personality: "kuudere" },
  { name: "Tsunade", series: "Naruto", age: 50, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "onee-san" },
  { name: "Mei Terumi", series: "Naruto", age: 31, height: "174 cm", weight: "52 kg", tier: "Rare", personality: "onee-san" },
  { name: "Samui", series: "Naruto", age: 26, height: "168 cm", weight: "50 kg", tier: "Rare", personality: "kuudere" },
  { name: "Hanabi Hyuga", series: "Naruto", age: 14, height: "156 cm", weight: "42 kg", tier: "Epic", personality: "genki" },
  { name: "Mikoto Uchiha", series: "Naruto", age: 40, height: "163 cm", weight: "48 kg", tier: "Epic", personality: "onee-san" },
  { name: "Kaguya Otsutsuki", series: "Naruto", age: 1000, height: "171 cm", weight: "55 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 2: Write `data/waifu/one-piece.js`**

```js
export const waifus = [
  { name: "Nami", series: "One Piece", age: 20, height: "170 cm", weight: "50 kg", tier: "Common", personality: "tsundere" },
  { name: "Nico Robin", series: "One Piece", age: 30, height: "188 cm", weight: "60 kg", tier: "Common", personality: "kuudere" },
  { name: "Tashigi", series: "One Piece", age: 24, height: "170 cm", weight: "50 kg", tier: "Common", personality: "dandere" },
  { name: "Perona", series: "One Piece", age: 25, height: "160 cm", weight: "45 kg", tier: "Common", personality: "himedere" },
  { name: "Kalifa", series: "One Piece", age: 29, height: "185 cm", weight: "58 kg", tier: "Common", personality: "ojou-sama" },
  { name: "Carrot", series: "One Piece", age: 15, height: "151 cm", weight: "42 kg", tier: "Common", personality: "genki" },
  { name: "Vivi", series: "One Piece", age: 18, height: "169 cm", weight: "47 kg", tier: "Rare", personality: "deredere" },
  { name: "Koala", series: "One Piece", age: 22, height: "166 cm", weight: "49 kg", tier: "Rare", personality: "genki" },
  { name: "Rebecca", series: "One Piece", age: 16, height: "171 cm", weight: "52 kg", tier: "Rare", personality: "dandere" },
  { name: "Reiju Vinsmoke", series: "One Piece", age: 24, height: "180 cm", weight: "58 kg", tier: "Rare", personality: "kuudere" },
  { name: "Jewelry Bonney", series: "One Piece", age: 24, height: "174 cm", weight: "55 kg", tier: "Epic", personality: "genki" },
  { name: "Hiyori Kozuki", series: "One Piece", age: 26, height: "170 cm", weight: "52 kg", tier: "Epic", personality: "dandere" },
  { name: "Shirahoshi", series: "One Piece", age: 17, height: "1187 cm", weight: "650 kg", tier: "Epic", personality: "dandere" },
  { name: "Boa Hancock", series: "One Piece", age: 31, height: "191 cm", weight: "60 kg", tier: "Legendary", personality: "tsundere" },
];
```

- [ ] **Step 3: Write `data/waifu/bleach.js`**

```js
export const waifus = [
  { name: "Orihime Inoue", series: "Bleach", age: 16, height: "157 cm", weight: "45 kg", tier: "Common", personality: "deredere" },
  { name: "Rukia Kuchiki", series: "Bleach", age: 15, height: "144 cm", weight: "33 kg", tier: "Common", personality: "kuudere" },
  { name: "Rangiku Matsumoto", series: "Bleach", age: 23, height: "172 cm", weight: "56 kg", tier: "Common", personality: "onee-san" },
  { name: "Soi Fon", series: "Bleach", age: 22, height: "150 cm", weight: "38 kg", tier: "Common", personality: "tsundere" },
  { name: "Isane Kotetsu", series: "Bleach", age: 24, height: "166 cm", weight: "52 kg", tier: "Common", personality: "dandere" },
  { name: "Tatsuki Arisawa", series: "Bleach", age: 16, height: "166 cm", weight: "51 kg", tier: "Common", personality: "genki" },
  { name: "Yachiru Kusajishi", series: "Bleach", age: 10, height: "109 cm", weight: "23 kg", tier: "Rare", personality: "genki" },
  { name: "Nanao Ise", series: "Bleach", age: 20, height: "166 cm", weight: "52 kg", tier: "Rare", personality: "kuudere" },
  { name: "Kukaku Shiba", series: "Bleach", age: 27, height: "170 cm", weight: "55 kg", tier: "Rare", personality: "onee-san" },
  { name: "Senna", series: "Bleach", age: 16, height: "155 cm", weight: "44 kg", tier: "Rare", personality: "dandere" },
  { name: "Nelliel Tu", series: "Bleach", age: 20, height: "175 cm", weight: "57 kg", tier: "Epic", personality: "deredere" },
  { name: "Tier Harribel", series: "Bleach", age: 25, height: "178 cm", weight: "60 kg", tier: "Epic", personality: "kuudere" },
  { name: "Retsu Unohana", series: "Bleach", age: 100, height: "170 cm", weight: "52 kg", tier: "Legendary", personality: "onee-san" },
  { name: "Yoruichi Shihoin", series: "Bleach", age: 100, height: "156 cm", weight: "42 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 4: Write `data/waifu/kimetsu.js`**

```js
export const waifus = [
  { name: "Nezuko Kamado", series: "Kimetsu no Yaiba", age: 14, height: "153 cm", weight: "45 kg", tier: "Common", personality: "dandere" },
  { name: "Kanao Tsuyuri", series: "Kimetsu no Yaiba", age: 16, height: "155 cm", weight: "44 kg", tier: "Common", personality: "kuudere" },
  { name: "Aoi Kanzaki", series: "Kimetsu no Yaiba", age: 15, height: "154 cm", weight: "44 kg", tier: "Common", personality: "tsundere" },
  { name: "Makomo", series: "Kimetsu no Yaiba", age: 11, height: "140 cm", weight: "36 kg", tier: "Common", personality: "dandere" },
  { name: "Tamayo", series: "Kimetsu no Yaiba", age: 500, height: "162 cm", weight: "47 kg", tier: "Common", personality: "onee-san" },
  { name: "Shinobu Kocho", series: "Kimetsu no Yaiba", age: 18, height: "151 cm", weight: "37 kg", tier: "Rare", personality: "tsundere" },
  { name: "Mitsuri Kanroji", series: "Kimetsu no Yaiba", age: 19, height: "167 cm", weight: "56 kg", tier: "Rare", personality: "deredere" },
  { name: "Naho Takada", series: "Kimetsu no Yaiba", age: 14, height: "152 cm", weight: "43 kg", tier: "Rare", personality: "genki" },
  { name: "Amane Ubuyashiki", series: "Kimetsu no Yaiba", age: 27, height: "160 cm", weight: "46 kg", tier: "Rare", personality: "kuudere" },
  { name: "Makio", series: "Kimetsu no Yaiba", age: 21, height: "170 cm", weight: "54 kg", tier: "Epic", personality: "genki" },
  { name: "Hinatsuru", series: "Kimetsu no Yaiba", age: 22, height: "167 cm", weight: "50 kg", tier: "Epic", personality: "kuudere" },
  { name: "Suma", series: "Kimetsu no Yaiba", age: 19, height: "164 cm", weight: "48 kg", tier: "Epic", personality: "himedere" },
  { name: "Daki", series: "Kimetsu no Yaiba", age: 100, height: "168 cm", weight: "52 kg", tier: "Legendary", personality: "himedere" },
];
```

- [ ] **Step 5: Write `data/waifu/jujutsu-kaisen.js`**

```js
export const waifus = [
  { name: "Nobara Kugisaki", series: "Jujutsu Kaisen", age: 16, height: "160 cm", weight: "45 kg", tier: "Common", personality: "tsundere" },
  { name: "Maki Zenin", series: "Jujutsu Kaisen", age: 17, height: "170 cm", weight: "55 kg", tier: "Common", personality: "kuudere" },
  { name: "Miwa Kasumi", series: "Jujutsu Kaisen", age: 16, height: "158 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Mei Mei", series: "Jujutsu Kaisen", age: 28, height: "168 cm", weight: "53 kg", tier: "Common", personality: "himedere" },
  { name: "Shoko Ieiri", series: "Jujutsu Kaisen", age: 28, height: "166 cm", weight: "50 kg", tier: "Rare", personality: "onee-san" },
  { name: "Utahime Iori", series: "Jujutsu Kaisen", age: 30, height: "165 cm", weight: "49 kg", tier: "Rare", personality: "tsundere" },
  { name: "Mai Zenin", series: "Jujutsu Kaisen", age: 17, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "tsundere" },
  { name: "Rika Orimoto", series: "Jujutsu Kaisen", age: 17, height: "158 cm", weight: "45 kg", tier: "Epic", personality: "yandere" },
  { name: "Hana Kurusu", series: "Jujutsu Kaisen", age: 16, height: "162 cm", weight: "48 kg", tier: "Epic", personality: "deredere" },
  { name: "Kirara Hoshi", series: "Jujutsu Kaisen", age: 17, height: "164 cm", weight: "49 kg", tier: "Epic", personality: "genki" },
  { name: "Yuki Tsukumo", series: "Jujutsu Kaisen", age: 29, height: "172 cm", weight: "58 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 6: Write `data/waifu/my-hero-academia.js`**

```js
export const waifus = [
  { name: "Ochaco Uraraka", series: "My Hero Academia", age: 16, height: "156 cm", weight: "43 kg", tier: "Common", personality: "deredere" },
  { name: "Momo Yaoyorozu", series: "My Hero Academia", age: 16, height: "173 cm", weight: "54 kg", tier: "Common", personality: "ojou-sama" },
  { name: "Tsuyu Asui", series: "My Hero Academia", age: 16, height: "150 cm", weight: "40 kg", tier: "Common", personality: "kuudere" },
  { name: "Mina Ashido", series: "My Hero Academia", age: 16, height: "159 cm", weight: "47 kg", tier: "Common", personality: "genki" },
  { name: "Kyoka Jiro", series: "My Hero Academia", age: 16, height: "154 cm", weight: "44 kg", tier: "Common", personality: "tsundere" },
  { name: "Toru Hagakure", series: "My Hero Academia", age: 16, height: "152 cm", weight: "43 kg", tier: "Common", personality: "genki" },
  { name: "Eri", series: "My Hero Academia", age: 7, height: "115 cm", weight: "20 kg", tier: "Rare", personality: "dandere" },
  { name: "Camie Utsushimi", series: "My Hero Academia", age: 16, height: "162 cm", weight: "48 kg", tier: "Rare", personality: "himedere" },
  { name: "Mei Hatsume", series: "My Hero Academia", age: 16, height: "162 cm", weight: "49 kg", tier: "Rare", personality: "genki" },
  { name: "Midnight", series: "My Hero Academia", age: 31, height: "175 cm", weight: "55 kg", tier: "Rare", personality: "himedere" },
  { name: "Mt. Lady", series: "My Hero Academia", age: 23, height: "180 cm", weight: "60 kg", tier: "Rare", personality: "himedere" },
  { name: "Mirko", series: "My Hero Academia", age: 28, height: "175 cm", weight: "58 kg", tier: "Epic", personality: "genki" },
  { name: "Himiko Toga", series: "My Hero Academia", age: 17, height: "157 cm", weight: "45 kg", tier: "Epic", personality: "yandere" },
  { name: "Lady Nagant", series: "My Hero Academia", age: 32, height: "172 cm", weight: "55 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 7: Write `data/waifu/dragon-ball.js`**

```js
export const waifus = [
  { name: "Bulma", series: "Dragon Ball", age: 28, height: "165 cm", weight: "49 kg", tier: "Common", personality: "tsundere" },
  { name: "Chi-Chi", series: "Dragon Ball", age: 24, height: "160 cm", weight: "47 kg", tier: "Common", personality: "tsundere" },
  { name: "Videl", series: "Dragon Ball", age: 18, height: "157 cm", weight: "48 kg", tier: "Common", personality: "genki" },
  { name: "Launch", series: "Dragon Ball", age: 24, height: "162 cm", weight: "48 kg", tier: "Common", personality: "kuudere" },
  { name: "Cheelai", series: "Dragon Ball", age: 22, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "genki" },
  { name: "Pan", series: "Dragon Ball", age: 8, height: "120 cm", weight: "22 kg", tier: "Rare", personality: "genki" },
  { name: "Android 18", series: "Dragon Ball", age: 25, height: "165 cm", weight: "50 kg", tier: "Rare", personality: "tsundere" },
  { name: "Vados", series: "Dragon Ball", age: 20, height: "168 cm", weight: "52 kg", tier: "Rare", personality: "ojou-sama" },
  { name: "Kefla", series: "Dragon Ball", age: 20, height: "165 cm", weight: "51 kg", tier: "Epic", personality: "genki" },
  { name: "Android 21", series: "Dragon Ball", age: 30, height: "168 cm", weight: "52 kg", tier: "Epic", personality: "deredere" },
  { name: "Caulifla", series: "Dragon Ball", age: 20, height: "162 cm", weight: "49 kg", tier: "Legendary", personality: "genki" },
  { name: "Kale", series: "Dragon Ball", age: 20, height: "163 cm", weight: "50 kg", tier: "Legendary", personality: "dandere" },
];
```

- [ ] **Step 8: Write `data/waifu/sword-art-online.js`**

```js
export const waifus = [
  { name: "Asuna Yuuki", series: "Sword Art Online", age: 17, height: "168 cm", weight: "55 kg", tier: "Common", personality: "tsundere" },
  { name: "Silica", series: "Sword Art Online", age: 14, height: "153 cm", weight: "42 kg", tier: "Common", personality: "deredere" },
  { name: "Lisbeth", series: "Sword Art Online", age: 17, height: "158 cm", weight: "46 kg", tier: "Common", personality: "genki" },
  { name: "Sinon", series: "Sword Art Online", age: 16, height: "165 cm", weight: "50 kg", tier: "Rare", personality: "kuudere" },
  { name: "Leafa", series: "Sword Art Online", age: 15, height: "168 cm", weight: "50 kg", tier: "Rare", personality: "genki" },
  { name: "Sachi", series: "Sword Art Online", age: 16, height: "158 cm", weight: "46 kg", tier: "Rare", personality: "dandere" },
  { name: "Yui", series: "Sword Art Online", age: 7, height: "120 cm", weight: "20 kg", tier: "Rare", personality: "deredere" },
  { name: "Ronye Arabel", series: "Sword Art Online", age: 17, height: "160 cm", weight: "48 kg", tier: "Epic", personality: "dandere" },
  { name: "Alice Zuberg", series: "Sword Art Online", age: 17, height: "168 cm", weight: "53 kg", tier: "Epic", personality: "kuudere" },
  { name: "Yuuki Konno", series: "Sword Art Online", age: 15, height: "153 cm", weight: "42 kg", tier: "Legendary", personality: "genki" },
  { name: "Quinella", series: "Sword Art Online", age: 300, height: "175 cm", weight: "57 kg", tier: "Legendary", personality: "himedere" },
];
```

- [ ] **Step 9: Write `data/waifu/rezero.js`**

```js
export const waifus = [
  { name: "Emilia", series: "Re:Zero", age: 19, height: "164 cm", weight: "45 kg", tier: "Rare", personality: "deredere" },
  { name: "Ram", series: "Re:Zero", age: 17, height: "154 cm", weight: "44 kg", tier: "Common", personality: "tsundere" },
  { name: "Felt", series: "Re:Zero", age: 15, height: "153 cm", weight: "43 kg", tier: "Common", personality: "genki" },
  { name: "Rem", series: "Re:Zero", age: 17, height: "154 cm", weight: "44 kg", tier: "Rare", personality: "dandere" },
  { name: "Priscilla Barielle", series: "Re:Zero", age: 19, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "ojou-sama" },
  { name: "Crusch Karsten", series: "Re:Zero", age: 22, height: "170 cm", weight: "55 kg", tier: "Rare", personality: "onee-san" },
  { name: "Beatrice", series: "Re:Zero", age: 400, height: "125 cm", weight: "24 kg", tier: "Rare", personality: "tsundere" },
  { name: "Frederica Baumann", series: "Re:Zero", age: 23, height: "170 cm", weight: "55 kg", tier: "Epic", personality: "onee-san" },
  { name: "Anastasia Hoshin", series: "Re:Zero", age: 20, height: "156 cm", weight: "45 kg", tier: "Epic", personality: "himedere" },
  { name: "Elsa Granhiert", series: "Re:Zero", age: 25, height: "168 cm", weight: "52 kg", tier: "Legendary", personality: "yandere" },
  { name: "Echidna", series: "Re:Zero", age: 400, height: "160 cm", weight: "45 kg", tier: "Mythic", personality: "kuudere" },
];
```

- [ ] **Step 10: Write `data/waifu/fate.js`**

```js
export const waifus = [
  { name: "Rin Tohsaka", series: "Fate", age: 17, height: "158 cm", weight: "47 kg", tier: "Common", personality: "tsundere" },
  { name: "Sakura Matou", series: "Fate", age: 16, height: "156 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Illyasviel", series: "Fate", age: 18, height: "133 cm", weight: "34 kg", tier: "Common", personality: "himedere" },
  { name: "Mash Kyrielight", series: "Fate", age: 17, height: "157 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Kiyohime", series: "Fate", age: 20, height: "156 cm", weight: "45 kg", tier: "Common", personality: "yandere" },
  { name: "Mordred", series: "Fate", age: 19, height: "154 cm", weight: "42 kg", tier: "Rare", personality: "tsundere" },
  { name: "Jeanne d'Arc", series: "Fate", age: 19, height: "159 cm", weight: "47 kg", tier: "Rare", personality: "deredere" },
  { name: "Nero Claudius", series: "Fate", age: 19, height: "150 cm", weight: "42 kg", tier: "Rare", personality: "himedere" },
  { name: "Tamamo no Mae", series: "Fate", age: 20, height: "160 cm", weight: "45 kg", tier: "Rare", personality: "deredere" },
  { name: "Rider (Medusa)", series: "Fate", age: 25, height: "172 cm", weight: "57 kg", tier: "Rare", personality: "kuudere" },
  { name: "Lancer (Scathach)", series: "Fate", age: 30, height: "168 cm", weight: "55 kg", tier: "Epic", personality: "onee-san" },
  { name: "Artoria Pendragon", series: "Fate", age: 24, height: "154 cm", weight: "42 kg", tier: "Mythic", personality: "kuudere" },
  { name: "Ishtar", series: "Fate", age: 17, height: "158 cm", weight: "47 kg", tier: "Epic", personality: "himedere" },
  { name: "Ereshkigal", series: "Fate", age: 17, height: "157 cm", weight: "47 kg", tier: "Legendary", personality: "dandere" },
  { name: "Queen Medb", series: "Fate", age: 20, height: "168 cm", weight: "53 kg", tier: "Legendary", personality: "himedere" },
];
```

- [ ] **Step 11: Write `data/waifu/genshin-impact.js`**

```js
export const waifus = [
  { name: "Lumine", series: "Genshin Impact", age: 17, height: "163 cm", weight: "49 kg", tier: "Common", personality: "deredere" },
  { name: "Barbara", series: "Genshin Impact", age: 16, height: "156 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Amber", series: "Genshin Impact", age: 18, height: "156 cm", weight: "46 kg", tier: "Common", personality: "genki" },
  { name: "Noelle", series: "Genshin Impact", age: 15, height: "165 cm", weight: "50 kg", tier: "Common", personality: "dandere" },
  { name: "Xiangling", series: "Genshin Impact", age: 14, height: "156 cm", weight: "45 kg", tier: "Common", personality: "genki" },
  { name: "Fischl", series: "Genshin Impact", age: 15, height: "158 cm", weight: "46 kg", tier: "Common", personality: "himedere" },
  { name: "Sucrose", series: "Genshin Impact", age: 16, height: "158 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Kokomi Sangonomiya", series: "Genshin Impact", age: 18, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Beidou", series: "Genshin Impact", age: 26, height: "178 cm", weight: "62 kg", tier: "Rare", personality: "onee-san" },
  { name: "Ningguang", series: "Genshin Impact", age: 25, height: "173 cm", weight: "55 kg", tier: "Rare", personality: "ojou-sama" },
  { name: "Keqing", series: "Genshin Impact", age: 17, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "tsundere" },
  { name: "Mona", series: "Genshin Impact", age: 18, height: "162 cm", weight: "48 kg", tier: "Rare", personality: "himedere" },
  { name: "Rosaria", series: "Genshin Impact", age: 22, height: "169 cm", weight: "52 kg", tier: "Rare", personality: "kuudere" },
  { name: "Nahida", series: "Genshin Impact", age: 500, height: "152 cm", weight: "42 kg", tier: "Epic", personality: "deredere" },
  { name: "Nilou", series: "Genshin Impact", age: 18, height: "162 cm", weight: "48 kg", tier: "Epic", personality: "deredere" },
  { name: "Ganyu", series: "Genshin Impact", age: 3000, height: "158 cm", weight: "48 kg", tier: "Epic", personality: "dandere" },
  { name: "Eula", series: "Genshin Impact", age: 20, height: "172 cm", weight: "55 kg", tier: "Epic", personality: "kuudere" },
  { name: "Raiden Shogun", series: "Genshin Impact", age: 500, height: "170 cm", weight: "55 kg", tier: "Epic", personality: "tsundere" },
  { name: "Kamisato Ayaka", series: "Genshin Impact", age: 18, height: "162 cm", weight: "48 kg", tier: "Legendary", personality: "dandere" },
  { name: "Yae Miko", series: "Genshin Impact", age: 500, height: "166 cm", weight: "50 kg", tier: "Legendary", personality: "onee-san" },
  { name: "Shenhe", series: "Genshin Impact", age: 25, height: "173 cm", weight: "56 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Furina", series: "Genshin Impact", age: 500, height: "155 cm", weight: "45 kg", tier: "Mythic", personality: "himedere" },
];
```

- [ ] **Step 12: Write `data/waifu/honkai-star-rail.js`**

```js
export const waifus = [
  { name: "March 7th", series: "Honkai Star Rail", age: 16, height: "160 cm", weight: "47 kg", tier: "Common", personality: "genki" },
  { name: "Stelle", series: "Honkai Star Rail", age: 18, height: "168 cm", weight: "52 kg", tier: "Common", personality: "deredere" },
  { name: "Herta", series: "Honkai Star Rail", age: 10, height: "140 cm", weight: "35 kg", tier: "Common", personality: "himedere" },
  { name: "Asta", series: "Honkai Star Rail", age: 18, height: "163 cm", weight: "49 kg", tier: "Common", personality: "genki" },
  { name: "Pela", series: "Honkai Star Rail", age: 15, height: "150 cm", weight: "40 kg", tier: "Common", personality: "kuudere" },
  { name: "Natasha", series: "Honkai Star Rail", age: 25, height: "168 cm", weight: "52 kg", tier: "Common", personality: "onee-san" },
  { name: "Tingyun", series: "Honkai Star Rail", age: 19, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "deredere" },
  { name: "Bronya", series: "Honkai Star Rail", age: 19, height: "165 cm", weight: "51 kg", tier: "Rare", personality: "tsundere" },
  { name: "Seele", series: "Honkai Star Rail", age: 18, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "tsundere" },
  { name: "Silver Wolf", series: "Honkai Star Rail", age: 17, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "himedere" },
  { name: "Himeko", series: "Honkai Star Rail", age: 28, height: "170 cm", weight: "54 kg", tier: "Epic", personality: "onee-san" },
  { name: "Kafka", series: "Honkai Star Rail", age: 28, height: "170 cm", weight: "55 kg", tier: "Epic", personality: "onee-san" },
  { name: "Jingliu", series: "Honkai Star Rail", age: 30, height: "168 cm", weight: "53 kg", tier: "Epic", personality: "kuudere" },
  { name: "Fu Xuan", series: "Honkai Star Rail", age: 25, height: "160 cm", weight: "47 kg", tier: "Epic", personality: "tsundere" },
  { name: "Ruan Mei", series: "Honkai Star Rail", age: 27, height: "165 cm", weight: "50 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Black Swan", series: "Honkai Star Rail", age: 27, height: "168 cm", weight: "52 kg", tier: "Legendary", personality: "onee-san" },
  { name: "Firefly", series: "Honkai Star Rail", age: 19, height: "160 cm", weight: "47 kg", tier: "Mythic", personality: "dandere" },
  { name: "Acheron", series: "Honkai Star Rail", age: 25, height: "170 cm", weight: "55 kg", tier: "Mythic", personality: "kuudere" },
];
```

- [ ] **Step 13: Write `data/waifu/kaguya-sama.js`**

```js
export const waifus = [
  { name: "Kaguya Shinomiya", series: "Kaguya-sama", age: 17, height: "158 cm", weight: "46 kg", tier: "Common", personality: "tsundere" },
  { name: "Chika Fujiwara", series: "Kaguya-sama", age: 16, height: "154 cm", weight: "44 kg", tier: "Common", personality: "genki" },
  { name: "Ai Hayasaka", series: "Kaguya-sama", age: 17, height: "161 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Miko Iino", series: "Kaguya-sama", age: 16, height: "158 cm", weight: "45 kg", tier: "Rare", personality: "tsundere" },
  { name: "Tsubame Koyasu", series: "Kaguya-sama", age: 18, height: "162 cm", weight: "49 kg", tier: "Rare", personality: "deredere" },
  { name: "Kei Shirogane", series: "Kaguya-sama", age: 15, height: "155 cm", weight: "44 kg", tier: "Epic", personality: "deredere" },
  { name: "Karen Kino", series: "Kaguya-sama", age: 17, height: "158 cm", weight: "46 kg", tier: "Epic", personality: "genki" },
  { name: "Nagisa Kashiwagi", series: "Kaguya-sama", age: 17, height: "159 cm", weight: "47 kg", tier: "Legendary", personality: "deredere" },
];
```

- [ ] **Step 14: Run data test (partial)**

Run: `npm test -- tests/waifu-data.test.mjs`
Expected: FAIL on `≥300` (pool ≈ 120). No duplicate/validation failures.

- [ ] **Step 15: Commit**

```bash
git add data/waifu/
git commit -m "data(waifu): franchise batch A (13 file, ~173 waifu)"
```

---

### Task 4: Franchise data batch B (konosuba → dxd)

**Files:**
- Create: `data/waifu/konosuba.js`, `data/waifu/chainsaw-man.js`, `data/waifu/spy-x-family.js`, `data/waifu/attack-on-titan.js`, `data/waifu/violet-evergarden.js`, `data/waifu/darling-in-franxx.js`, `data/waifu/steins-gate.js`, `data/waifu/k-on.js`, `data/waifu/oshi-no-ko.js`, `data/waifu/date-a-live.js`, `data/waifu/highschool-dxd.js`

- [ ] **Step 1: Write `data/waifu/konosuba.js`**

```js
export const waifus = [
  { name: "Megumin", series: "KonoSuba", age: 13, height: "145 cm", weight: "38 kg", tier: "Rare", personality: "himedere" },
  { name: "Aqua", series: "KonoSuba", age: 17, height: "161 cm", weight: "49 kg", tier: "Common", personality: "himedere" },
  { name: "Darkness", series: "KonoSuba", age: 18, height: "172 cm", weight: "58 kg", tier: "Common", personality: "kuudere" },
  { name: "Yunyun", series: "KonoSuba", age: 13, height: "147 cm", weight: "40 kg", tier: "Common", personality: "dandere" },
  { name: "Wiz", series: "KonoSuba", age: 20, height: "165 cm", weight: "50 kg", tier: "Common", personality: "deredere" },
  { name: "Eris", series: "KonoSuba", age: 17, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "deredere" },
  { name: "Chris", series: "KonoSuba", age: 17, height: "162 cm", weight: "50 kg", tier: "Rare", personality: "genki" },
  { name: "Komekko", series: "KonoSuba", age: 6, height: "110 cm", weight: "19 kg", tier: "Epic", personality: "deredere" },
];
```

- [ ] **Step 2: Write `data/waifu/chainsaw-man.js`**

```js
export const waifus = [
  { name: "Power", series: "Chainsaw Man", age: 17, height: "170 cm", weight: "52 kg", tier: "Rare", personality: "genki" },
  { name: "Makima", series: "Chainsaw Man", age: 25, height: "173 cm", weight: "58 kg", tier: "Mythic", personality: "yandere" },
  { name: "Kobeni", series: "Chainsaw Man", age: 19, height: "158 cm", weight: "46 kg", tier: "Common", personality: "dandere" },
  { name: "Himeno", series: "Chainsaw Man", age: 24, height: "165 cm", weight: "50 kg", tier: "Common", personality: "onee-san" },
  { name: "Reze", series: "Chainsaw Man", age: 18, height: "162 cm", weight: "49 kg", tier: "Rare", personality: "genki" },
  { name: "Asa Mitaka", series: "Chainsaw Man", age: 17, height: "160 cm", weight: "47 kg", tier: "Epic", personality: "dandere" },
  { name: "Quanxi", series: "Chainsaw Man", age: 22, height: "170 cm", weight: "55 kg", tier: "Epic", personality: "kuudere" },
  { name: "Yoru", series: "Chainsaw Man", age: 17, height: "160 cm", weight: "47 kg", tier: "Legendary", personality: "tsundere" },
];
```

- [ ] **Step 3: Write `data/waifu/spy-x-family.js`**

```js
export const waifus = [
  { name: "Yor Forger", series: "Spy x Family", age: 27, height: "170 cm", weight: "55 kg", tier: "Rare", personality: "dandere" },
  { name: "Anya Forger", series: "Spy x Family", age: 6, height: "110 cm", weight: "19 kg", tier: "Common", personality: "genki" },
  { name: "Becky Blackbell", series: "Spy x Family", age: 12, height: "148 cm", weight: "40 kg", tier: "Common", personality: "himedere" },
  { name: "Fiona Frost", series: "Spy x Family", age: 25, height: "167 cm", weight: "52 kg", tier: "Rare", personality: "kuudere" },
  { name: "Sylvia Sherwood", series: "Spy x Family", age: 30, height: "172 cm", weight: "56 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 4: Write `data/waifu/attack-on-titan.js`**

```js
export const waifus = [
  { name: "Mikasa Ackerman", series: "Attack on Titan", age: 15, height: "170 cm", weight: "59 kg", tier: "Rare", personality: "kuudere" },
  { name: "Sasha Blouse", series: "Attack on Titan", age: 15, height: "168 cm", weight: "55 kg", tier: "Common", personality: "genki" },
  { name: "Hange Zoe", series: "Attack on Titan", age: 29, height: "170 cm", weight: "55 kg", tier: "Common", personality: "genki" },
  { name: "Pieck Finger", series: "Attack on Titan", age: 20, height: "160 cm", weight: "50 kg", tier: "Common", personality: "dandere" },
  { name: "Petra Ral", series: "Attack on Titan", age: 19, height: "162 cm", weight: "52 kg", tier: "Common", personality: "deredere" },
  { name: "Historia Reiss", series: "Attack on Titan", age: 15, height: "145 cm", weight: "45 kg", tier: "Rare", personality: "deredere" },
  { name: "Annie Leonhart", series: "Attack on Titan", age: 16, height: "153 cm", weight: "54 kg", tier: "Rare", personality: "kuudere" },
  { name: "Ymir", series: "Attack on Titan", age: 19, height: "172 cm", weight: "63 kg", tier: "Rare", personality: "genki" },
  { name: "Frieda Reiss", series: "Attack on Titan", age: 19, height: "160 cm", weight: "50 kg", tier: "Epic", personality: "ojou-sama" },
  { name: "Gabi Braun", series: "Attack on Titan", age: 12, height: "155 cm", weight: "48 kg", tier: "Epic", personality: "tsundere" },
  { name: "Hitch Dreyse", series: "Attack on Titan", age: 22, height: "165 cm", weight: "52 kg", tier: "Epic", personality: "himedere" },
  { name: "Yelena", series: "Attack on Titan", age: 28, height: "175 cm", weight: "58 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 5: Write `data/waifu/violet-evergarden.js`**

```js
export const waifus = [
  { name: "Violet Evergarden", series: "Violet Evergarden", age: 14, height: "161 cm", weight: "48 kg", tier: "Rare", personality: "kuudere" },
  { name: "Cattleya Baudelaire", series: "Violet Evergarden", age: 22, height: "168 cm", weight: "53 kg", tier: "Common", personality: "onee-san" },
  { name: "Iris Cannary", series: "Violet Evergarden", age: 18, height: "162 cm", weight: "50 kg", tier: "Common", personality: "tsundere" },
  { name: "Erica Brown", series: "Violet Evergarden", age: 19, height: "160 cm", weight: "48 kg", tier: "Common", personality: "dandere" },
  { name: "Luculia Marlborough", series: "Violet Evergarden", age: 22, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "onee-san" },
  { name: "Amy Bartlett", series: "Violet Evergarden", age: 18, height: "160 cm", weight: "47 kg", tier: "Epic", personality: "deredere" },
  { name: "Lux Sibyl", series: "Violet Evergarden", age: 16, height: "158 cm", weight: "47 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 6: Write `data/waifu/darling-in-franxx.js`**

```js
export const waifus = [
  { name: "Zero Two", series: "Darling in the Franxx", age: 16, height: "170 cm", weight: "55 kg", tier: "Mythic", personality: "tsundere" },
  { name: "Ichigo", series: "Darling in the Franxx", age: 16, height: "165 cm", weight: "50 kg", tier: "Common", personality: "tsundere" },
  { name: "Kokoro", series: "Darling in the Franxx", age: 16, height: "160 cm", weight: "48 kg", tier: "Common", personality: "deredere" },
  { name: "Miku", series: "Darling in the Franxx", age: 15, height: "157 cm", weight: "46 kg", tier: "Common", personality: "genki" },
  { name: "Ikuno", series: "Darling in the Franxx", age: 16, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "kuudere" },
  { name: "Naomi", series: "Darling in the Franxx", age: 15, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "deredere" },
  { name: "9'α", series: "Darling in the Franxx", age: 17, height: "167 cm", weight: "52 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 7: Write `data/waifu/steins-gate.js`**

```js
export const waifus = [
  { name: "Kurisu Makise", series: "Steins;Gate", age: 18, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "tsundere" },
  { name: "Mayuri Shiina", series: "Steins;Gate", age: 16, height: "152 cm", weight: "43 kg", tier: "Common", personality: "deredere" },
  { name: "Faris Nyannyan", series: "Steins;Gate", age: 17, height: "156 cm", weight: "45 kg", tier: "Common", personality: "himedere" },
  { name: "Moeka Kiryu", series: "Steins;Gate", age: 20, height: "168 cm", weight: "52 kg", tier: "Common", personality: "dandere" },
  { name: "Suzuha Amane", series: "Steins;Gate", age: 18, height: "164 cm", weight: "50 kg", tier: "Epic", personality: "genki" },
  { name: "Mahi Amane", series: "Steins;Gate", age: 24, height: "166 cm", weight: "51 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 8: Write `data/waifu/k-on.js`**

```js
export const waifus = [
  { name: "Yui Hirasawa", series: "K-On!", age: 16, height: "156 cm", weight: "44 kg", tier: "Common", personality: "genki" },
  { name: "Mio Akiyama", series: "K-On!", age: 16, height: "160 cm", weight: "46 kg", tier: "Rare", personality: "dandere" },
  { name: "Ritsu Tainaka", series: "K-On!", age: 16, height: "157 cm", weight: "45 kg", tier: "Common", personality: "genki" },
  { name: "Tsumugi Kotobuki", series: "K-On!", age: 16, height: "157 cm", weight: "46 kg", tier: "Rare", personality: "ojou-sama" },
  { name: "Azusa Nakano", series: "K-On!", age: 15, height: "150 cm", weight: "41 kg", tier: "Epic", personality: "tsundere" },
  { name: "Ui Hirasawa", series: "K-On!", age: 15, height: "154 cm", weight: "43 kg", tier: "Rare", personality: "deredere" },
  { name: "Sawako Yamanaka", series: "K-On!", age: 27, height: "168 cm", weight: "52 kg", tier: "Legendary", personality: "onee-san" },
];
```

- [ ] **Step 9: Write `data/waifu/oshi-no-ko.js`**

```js
export const waifus = [
  { name: "Ruby Hoshino", series: "Oshi no Ko", age: 16, height: "158 cm", weight: "46 kg", tier: "Common", personality: "genki" },
  { name: "Kana Arima", series: "Oshi no Ko", age: 17, height: "153 cm", weight: "43 kg", tier: "Rare", personality: "tsundere" },
  { name: "Akane Kurokawa", series: "Oshi no Ko", age: 17, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Ai Hoshino", series: "Oshi no Ko", age: 16, height: "151 cm", weight: "42 kg", tier: "Epic", personality: "deredere" },
  { name: "Memcho", series: "Oshi no Ko", age: 25, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "genki" },
  { name: "Miyako Saito", series: "Oshi no Ko", age: 32, height: "164 cm", weight: "50 kg", tier: "Legendary", personality: "onee-san" },
  { name: "Frill Shiranui", series: "Oshi no Ko", age: 18, height: "162 cm", weight: "49 kg", tier: "Epic", personality: "himedere" },
];
```

- [ ] **Step 10: Write `data/waifu/date-a-live.js`**

```js
export const waifus = [
  { name: "Kurumi Tokisaki", series: "Date A Live", age: 17, height: "157 cm", weight: "46 kg", tier: "Epic", personality: "yandere" },
  { name: "Tohka Yatogami", series: "Date A Live", age: 17, height: "157 cm", weight: "46 kg", tier: "Rare", personality: "deredere" },
  { name: "Yoshino", series: "Date A Live", age: 13, height: "144 cm", weight: "38 kg", tier: "Common", personality: "dandere" },
  { name: "Kotori Itsuka", series: "Date A Live", age: 15, height: "155 cm", weight: "44 kg", tier: "Common", personality: "tsundere" },
  { name: "Origami Tobiichi", series: "Date A Live", age: 16, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Miku Izayoi", series: "Date A Live", age: 17, height: "163 cm", weight: "50 kg", tier: "Common", personality: "himedere" },
  { name: "Natsumi", series: "Date A Live", age: 16, height: "145 cm", weight: "39 kg", tier: "Rare", personality: "himedere" },
  { name: "Mukuro Hoshimiya", series: "Date A Live", age: 15, height: "156 cm", weight: "45 kg", tier: "Legendary", personality: "dandere" },
];
```

- [ ] **Step 11: Write `data/waifu/highschool-dxd.js`**

```js
export const waifus = [
  { name: "Rias Gremory", series: "High School DxD", age: 18, height: "172 cm", weight: "58 kg", tier: "Mythic", personality: "onee-san" },
  { name: "Akeno Himejima", series: "High School DxD", age: 18, height: "168 cm", weight: "54 kg", tier: "Rare", personality: "himedere" },
  { name: "Koneko Toujou", series: "High School DxD", age: 15, height: "149 cm", weight: "40 kg", tier: "Common", personality: "kuudere" },
  { name: "Asia Argento", series: "High School DxD", age: 17, height: "162 cm", weight: "48 kg", tier: "Common", personality: "dandere" },
  { name: "Xenovia Quarta", series: "High School DxD", age: 18, height: "171 cm", weight: "56 kg", tier: "Common", personality: "tsundere" },
  { name: "Irina Shidou", series: "High School DxD", age: 17, height: "164 cm", weight: "50 kg", tier: "Rare", personality: "genki" },
  { name: "Rossweisse", series: "High School DxD", age: 24, height: "168 cm", weight: "52 kg", tier: "Epic", personality: "dandere" },
  { name: "Kuroka", series: "High School DxD", age: 20, height: "165 cm", weight: "51 kg", tier: "Epic", personality: "onee-san" },
  { name: "Grayfia Lucifuge", series: "High School DxD", age: 30, height: "170 cm", weight: "54 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 12: Run data test (partial)**

Run: `npm test -- tests/waifu-data.test.mjs`
Expected: FAIL on `≥300` (pool ≈ 250). No other failures.

- [ ] **Step 13: Commit**

```bash
git add data/waifu/
git commit -m "data(waifu): franchise batch B (11 file, ~84 waifu)"
```

---

### Task 5: Franchise data batch C (nier → blue-archive) + data test green

**Files:**
- Create: `data/waifu/nier.js`, `data/waifu/classroom-of-the-elite.js`, `data/waifu/frieren.js`, `data/waifu/love-live.js`, `data/waifu/fairy-tail.js`, `data/waifu/komi.js`, `data/waifu/my-dress-up-darling.js`, `data/waifu/lycoris-recoil.js`, `data/waifu/bocchi-the-rock.js`, `data/waifu/mushoku-tensei.js`, `data/waifu/dandadan.js`, `data/waifu/toradora.js`, `data/waifu/monogatari.js`, `data/waifu/nikke.js`, `data/waifu/blue-archive.js`

- [ ] **Step 1: Write `data/waifu/nier.js`**

```js
export const waifus = [
  { name: "2B", series: "NieR", age: 20, height: "168 cm", weight: "52 kg", tier: "Mythic", personality: "kuudere" },
  { name: "A2", series: "NieR", age: 24, height: "169 cm", weight: "53 kg", tier: "Common", personality: "tsundere" },
  { name: "Devola", series: "NieR", age: 23, height: "164 cm", weight: "49 kg", tier: "Common", personality: "deredere" },
  { name: "Popola", series: "NieR", age: 23, height: "164 cm", weight: "49 kg", tier: "Common", personality: "onee-san" },
  { name: "Kainé", series: "NieR", age: 20, height: "165 cm", weight: "50 kg", tier: "Epic", personality: "yandere" },
];
```

- [ ] **Step 2: Write `data/waifu/classroom-of-the-elite.js`**

```js
export const waifus = [
  { name: "Kei Karuizawa", series: "Classroom of the Elite", age: 16, height: "159 cm", weight: "47 kg", tier: "Common", personality: "himedere" },
  { name: "Suzune Horikita", series: "Classroom of the Elite", age: 16, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "tsundere" },
  { name: "Sakura Airi", series: "Classroom of the Elite", age: 16, height: "162 cm", weight: "48 kg", tier: "Common", personality: "dandere" },
  { name: "Kushida Kikyo", series: "Classroom of the Elite", age: 16, height: "160 cm", weight: "46 kg", tier: "Rare", personality: "deredere" },
  { name: "Honami Ichinose", series: "Classroom of the Elite", age: 16, height: "162 cm", weight: "49 kg", tier: "Epic", personality: "deredere" },
  { name: "Arisu Sakayanagi", series: "Classroom of the Elite", age: 16, height: "148 cm", weight: "40 kg", tier: "Epic", personality: "himedere" },
  { name: "Hiyori Shiina", series: "Classroom of the Elite", age: 16, height: "158 cm", weight: "46 kg", tier: "Rare", personality: "kuudere" },
  { name: "Satsuki Shinohara", series: "Classroom of the Elite", age: 16, height: "160 cm", weight: "48 kg", tier: "Legendary", personality: "genki" },
];
```

- [ ] **Step 3: Write `data/waifu/frieren.js`**

```js
export const waifus = [
  { name: "Frieren", series: "Frieren", age: 1000, height: "163 cm", weight: "48 kg", tier: "Mythic", personality: "kuudere" },
  { name: "Fern", series: "Frieren", age: 18, height: "162 cm", weight: "49 kg", tier: "Rare", personality: "tsundere" },
  { name: "Ubel", series: "Frieren", age: 20, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "yandere" },
  { name: "Sense", series: "Frieren", age: 30, height: "166 cm", weight: "51 kg", tier: "Rare", personality: "onee-san" },
  { name: "Serie", series: "Frieren", age: 1000, height: "158 cm", weight: "45 kg", tier: "Epic", personality: "tsundere" },
  { name: "Aura", series: "Frieren", age: 500, height: "170 cm", weight: "54 kg", tier: "Legendary", personality: "ojou-sama" },
];
```

- [ ] **Step 4: Write `data/waifu/love-live.js`**

```js
export const waifus = [
  { name: "Honoka Kosaka", series: "Love Live!", age: 16, height: "157 cm", weight: "45 kg", tier: "Common", personality: "genki" },
  { name: "Umi Sonoda", series: "Love Live!", age: 16, height: "159 cm", weight: "46 kg", tier: "Common", personality: "kuudere" },
  { name: "Kotori Minami", series: "Love Live!", age: 16, height: "157 cm", weight: "45 kg", tier: "Common", personality: "dandere" },
  { name: "Eli Ayase", series: "Love Live!", age: 17, height: "162 cm", weight: "49 kg", tier: "Rare", personality: "onee-san" },
  { name: "Nozomi Tojo", series: "Love Live!", age: 17, height: "159 cm", weight: "48 kg", tier: "Rare", personality: "onee-san" },
  { name: "Maki Nishikino", series: "Love Live!", age: 15, height: "158 cm", weight: "46 kg", tier: "Rare", personality: "tsundere" },
  { name: "Rin Hoshizora", series: "Love Live!", age: 15, height: "155 cm", weight: "44 kg", tier: "Common", personality: "genki" },
  { name: "Hanayo Koizumi", series: "Love Live!", age: 15, height: "156 cm", weight: "45 kg", tier: "Common", personality: "dandere" },
  { name: "Nico Yazawa", series: "Love Live!", age: 17, height: "154 cm", weight: "43 kg", tier: "Epic", personality: "himedere" },
  { name: "Dia Kurosawa", series: "Love Live!", age: 17, height: "162 cm", weight: "49 kg", tier: "Legendary", personality: "ojou-sama" },
];
```

- [ ] **Step 5: Write `data/waifu/fairy-tail.js`**

```js
export const waifus = [
  { name: "Erza Scarlet", series: "Fairy Tail", age: 19, height: "169 cm", weight: "50 kg", tier: "Epic", personality: "tsundere" },
  { name: "Lucy Heartfilia", series: "Fairy Tail", age: 17, height: "165 cm", weight: "47 kg", tier: "Rare", personality: "deredere" },
  { name: "Wendy Marvell", series: "Fairy Tail", age: 14, height: "145 cm", weight: "38 kg", tier: "Common", personality: "dandere" },
  { name: "Juvia Lockser", series: "Fairy Tail", age: 19, height: "168 cm", weight: "52 kg", tier: "Rare", personality: "dandere" },
  { name: "Mirajane Strauss", series: "Fairy Tail", age: 21, height: "162 cm", weight: "48 kg", tier: "Rare", personality: "onee-san" },
  { name: "Levy McGarden", series: "Fairy Tail", age: 18, height: "160 cm", weight: "47 kg", tier: "Common", personality: "dandere" },
  { name: "Lisanna Strauss", series: "Fairy Tail", age: 18, height: "157 cm", weight: "46 kg", tier: "Common", personality: "deredere" },
  { name: "Cana Alberona", series: "Fairy Tail", age: 20, height: "168 cm", weight: "52 kg", tier: "Common", personality: "genki" },
  { name: "Ultear Milkovich", series: "Fairy Tail", age: 30, height: "167 cm", weight: "51 kg", tier: "Epic", personality: "kuudere" },
  { name: "Mavis Vermillion", series: "Fairy Tail", age: 20, height: "147 cm", weight: "40 kg", tier: "Legendary", personality: "deredere" },
  { name: "Minerva Orland", series: "Fairy Tail", age: 22, height: "165 cm", weight: "50 kg", tier: "Epic", personality: "tsundere" },
  { name: "Kagura Mikazuchi", series: "Fairy Tail", age: 18, height: "170 cm", weight: "53 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 6: Write `data/waifu/komi.js`**

```js
export const waifus = [
  { name: "Shoko Komi", series: "Komi Can't Communicate", age: 16, height: "168 cm", weight: "50 kg", tier: "Rare", personality: "dandere" },
  { name: "Najimi Osana", series: "Komi Can't Communicate", age: 16, height: "158 cm", weight: "46 kg", tier: "Rare", personality: "genki" },
  { name: "Himiko Agari", series: "Komi Can't Communicate", age: 16, height: "155 cm", weight: "44 kg", tier: "Common", personality: "dandere" },
  { name: "Ren Yamai", series: "Komi Can't Communicate", age: 16, height: "156 cm", weight: "45 kg", tier: "Common", personality: "yandere" },
  { name: "Nakanaka Omoharu", series: "Komi Can't Communicate", age: 16, height: "160 cm", weight: "47 kg", tier: "Common", personality: "genki" },
  { name: "Emoyama Nene", series: "Komi Can't Communicate", age: 16, height: "162 cm", weight: "48 kg", tier: "Epic", personality: "himedere" },
];
```

- [ ] **Step 7: Write `data/waifu/my-dress-up-darling.js`**

```js
export const waifus = [
  { name: "Marin Kitagawa", series: "My Dress-Up Darling", age: 16, height: "164 cm", weight: "50 kg", tier: "Rare", personality: "deredere" },
  { name: "Shizuku Kuroe", series: "My Dress-Up Darling", age: 17, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "kuudere" },
  { name: "Sajuna Inui", series: "My Dress-Up Darling", age: 18, height: "166 cm", weight: "51 kg", tier: "Common", personality: "tsundere" },
  { name: "Shinju Inui", series: "My Dress-Up Darling", age: 14, height: "152 cm", weight: "43 kg", tier: "Common", personality: "dandere" },
  { name: "Nowa Sumi", series: "My Dress-Up Darling", age: 17, height: "160 cm", weight: "47 kg", tier: "Epic", personality: "genki" },
];
```

- [ ] **Step 8: Write `data/waifu/lycoris-recoil.js`**

```js
export const waifus = [
  { name: "Chisato Nishikigi", series: "Lycoris Recoil", age: 17, height: "161 cm", weight: "48 kg", tier: "Rare", personality: "genki" },
  { name: "Takina Inoue", series: "Lycoris Recoil", age: 16, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Mizuki Nakahara", series: "Lycoris Recoil", age: 25, height: "167 cm", weight: "52 kg", tier: "Common", personality: "onee-san" },
  { name: "Fuki Harukawa", series: "Lycoris Recoil", age: 18, height: "163 cm", weight: "49 kg", tier: "Common", personality: "tsundere" },
  { name: "Sakura Otome", series: "Lycoris Recoil", age: 17, height: "159 cm", weight: "46 kg", tier: "Epic", personality: "dandere" },
];
```

- [ ] **Step 9: Write `data/waifu/bocchi-the-rock.js`**

```js
export const waifus = [
  { name: "Hitori Gotoh", series: "Bocchi the Rock!", age: 15, height: "156 cm", weight: "45 kg", tier: "Rare", personality: "dandere" },
  { name: "Nijika Ijichi", series: "Bocchi the Rock!", age: 16, height: "159 cm", weight: "47 kg", tier: "Rare", personality: "genki" },
  { name: "Ryo Yamada", series: "Bocchi the Rock!", age: 16, height: "164 cm", weight: "50 kg", tier: "Common", personality: "kuudere" },
  { name: "Ikuyo Kita", series: "Bocchi the Rock!", age: 16, height: "157 cm", weight: "46 kg", tier: "Common", personality: "deredere" },
  { name: "Kikuri Hiroi", series: "Bocchi the Rock!", age: 27, height: "165 cm", weight: "51 kg", tier: "Epic", personality: "genki" },
];
```

- [ ] **Step 10: Write `data/waifu/mushoku-tensei.js`**

```js
export const waifus = [
  { name: "Roxy Migurdia", series: "Mushoku Tensei", age: 40, height: "147 cm", weight: "40 kg", tier: "Rare", personality: "tsundere" },
  { name: "Sylphiette", series: "Mushoku Tensei", age: 15, height: "155 cm", weight: "44 kg", tier: "Rare", personality: "dandere" },
  { name: "Eris Boreas Greyrat", series: "Mushoku Tensei", age: 15, height: "165 cm", weight: "50 kg", tier: "Rare", personality: "tsundere" },
  { name: "Ghislaine Dedoldia", series: "Mushoku Tensei", age: 30, height: "175 cm", weight: "62 kg", tier: "Common", personality: "onee-san" },
  { name: "Elinalise Dragonroad", series: "Mushoku Tensei", age: 300, height: "168 cm", weight: "52 kg", tier: "Common", personality: "himedere" },
  { name: "Aisha Greyrat", series: "Mushoku Tensei", age: 15, height: "152 cm", weight: "43 kg", tier: "Common", personality: "genki" },
  { name: "Zenith Greyrat", series: "Mushoku Tensei", age: 35, height: "164 cm", weight: "49 kg", tier: "Epic", personality: "onee-san" },
  { name: "Nanahoshi Shizuka", series: "Mushoku Tensei", age: 17, height: "158 cm", weight: "46 kg", tier: "Legendary", personality: "kuudere" },
];
```

- [ ] **Step 11: Write `data/waifu/dandadan.js`**

```js
export const waifus = [
  { name: "Momo Ayase", series: "Dandadan", age: 16, height: "164 cm", weight: "49 kg", tier: "Rare", personality: "genki" },
  { name: "Aira Shiratori", series: "Dandadan", age: 16, height: "162 cm", weight: "48 kg", tier: "Rare", personality: "himedere" },
  { name: "Seiko Ayase", series: "Dandadan", age: 45, height: "168 cm", weight: "52 kg", tier: "Epic", personality: "onee-san" },
  { name: "Rin Sawaki", series: "Dandadan", age: 17, height: "160 cm", weight: "47 kg", tier: "Common", personality: "deredere" },
  { name: "Turbo Granny", series: "Dandadan", age: 100, height: "150 cm", weight: "42 kg", tier: "Legendary", personality: "tsundere" },
];
```

- [ ] **Step 12: Write `data/waifu/toradora.js`**

```js
export const waifus = [
  { name: "Taiga Aisaka", series: "Toradora!", age: 16, height: "145 cm", weight: "39 kg", tier: "Rare", personality: "tsundere" },
  { name: "Minori Kushieda", series: "Toradora!", age: 16, height: "160 cm", weight: "48 kg", tier: "Common", personality: "genki" },
  { name: "Ami Kawashima", series: "Toradora!", age: 16, height: "161 cm", weight: "47 kg", tier: "Common", personality: "himedere" },
  { name: "Yasuko Takasu", series: "Toradora!", age: 35, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "deredere" },
  { name: "Kano Sumire", series: "Toradora!", age: 18, height: "165 cm", weight: "50 kg", tier: "Epic", personality: "onee-san" },
];
```

- [ ] **Step 13: Write `data/waifu/monogatari.js`**

```js
export const waifus = [
  { name: "Hitagi Senjougahara", series: "Monogatari", age: 18, height: "165 cm", weight: "49 kg", tier: "Rare", personality: "tsundere" },
  { name: "Tsubasa Hanekawa", series: "Monogatari", age: 17, height: "162 cm", weight: "48 kg", tier: "Rare", personality: "dandere" },
  { name: "Shinobu Oshino", series: "Monogatari", age: 500, height: "156 cm", weight: "44 kg", tier: "Epic", personality: "kuudere" },
  { name: "Mayoi Hachikuji", series: "Monogatari", age: 11, height: "140 cm", weight: "36 kg", tier: "Common", personality: "deredere" },
  { name: "Suruga Kanbaru", series: "Monogatari", age: 17, height: "170 cm", weight: "54 kg", tier: "Common", personality: "genki" },
  { name: "Nadeko Sengoku", series: "Monogatari", age: 14, height: "156 cm", weight: "45 kg", tier: "Common", personality: "dandere" },
  { name: "Karen Araragi", series: "Monogatari", age: 15, height: "162 cm", weight: "49 kg", tier: "Epic", personality: "genki" },
  { name: "Tsukihi Araragi", series: "Monogatari", age: 14, height: "160 cm", weight: "48 kg", tier: "Legendary", personality: "himedere" },
];
```

- [ ] **Step 14: Write `data/waifu/nikke.js`**

```js
export const waifus = [
  { name: "Rapi", series: "Nikke", age: 22, height: "168 cm", weight: "52 kg", tier: "Rare", personality: "kuudere" },
  { name: "Anis", series: "Nikke", age: 21, height: "165 cm", weight: "50 kg", tier: "Common", personality: "genki" },
  { name: "Modernia", series: "Nikke", age: 20, height: "166 cm", weight: "51 kg", tier: "Epic", personality: "dandere" },
  { name: "Neon", series: "Nikke", age: 19, height: "160 cm", weight: "47 kg", tier: "Common", personality: "genki" },
  { name: "Scarlet", series: "Nikke", age: 25, height: "172 cm", weight: "56 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Alice", series: "Nikke", age: 20, height: "163 cm", weight: "49 kg", tier: "Rare", personality: "deredere" },
];
```

- [ ] **Step 15: Write `data/waifu/blue-archive.js`**

```js
export const waifus = [
  { name: "Hoshino", series: "Blue Archive", age: 17, height: "150 cm", weight: "40 kg", tier: "Rare", personality: "amayadori" },
  { name: "Sorasaki Hina", series: "Blue Archive", age: 17, height: "160 cm", weight: "47 kg", tier: "Rare", personality: "kuudere" },
  { name: "Shiroko", series: "Blue Archive", age: 16, height: "158 cm", weight: "46 kg", tier: "Common", personality: "kuudere" },
  { name: "Aru", series: "Blue Archive", age: 17, height: "158 cm", weight: "46 kg", tier: "Common", personality: "himedere" },
  { name: "Yuuka", series: "Blue Archive", age: 16, height: "156 cm", weight: "45 kg", tier: "Common", personality: "tsundere" },
  { name: "Saori", series: "Blue Archive", age: 17, height: "162 cm", weight: "49 kg", tier: "Epic", personality: "dandere" },
  { name: "Noa", series: "Blue Archive", age: 17, height: "159 cm", weight: "47 kg", tier: "Epic", personality: "deredere" },
  { name: "Mika", series: "Blue Archive", age: 16, height: "160 cm", weight: "48 kg", tier: "Legendary", personality: "ojou-sama" },
];
```

- [ ] **Step 16: Run data test — should pass**

Run: `npm test -- tests/waifu-data.test.mjs`
Expected: PASS. Pool ≥ 300, no dupes, valid tiers/personalities, distribution in range. Verify count: `node -e "import('./data/waifu/index.js').then(m => console.log(m.getPool().length))"`.

- [ ] **Step 17: Run lib test**

Run: `npm test -- tests/waifu-lib.test.mjs`
Expected: PASS (pool now populated).

- [ ] **Step 18: Commit**

```bash
git add data/waifu/
git commit -m "data(waifu): franchise batch C (15 file) — 300+ total"
```

---

### Task 6: Refactor plugin gachawaifu

**Files:**
- Modify: `plugins/fun/gachawaifu.js` (full rewrite)

**Interfaces:**
- Consumes: `rollWaifu, applyAction, rollEvent, getDailyMood, albumStats, DOWRY, ACTIONS` from `../../src/lib/ourin-waifu.js`; `addExpWithLevelCheck` from `../../src/lib/ourin-level.js`; existing image/WA helpers pattern.
- Produces: same command names + expanded actions + economy integration.

- [ ] **Step 1: Rewrite `plugins/fun/gachawaifu.js`**

```js
import axios from "axios";
import { getDatabase } from "../../src/lib/ourin-database.js";
import te from "../../src/lib/ourin-error.js";
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin";
import { addExpWithLevelCheck } from "../../src/lib/ourin-level.js";
import { rollWaifu, applyAction, rollEvent, getDailyMood, albumStats, DOWRY } from "../../src/lib/ourin-waifu.js";

const pluginConfig = {
  name: ["gachawaifu", "waifuaction", "tinggalinwaifu", "waifuku", "istriku"],
  alias: ["gachaistri"],
  category: "fun",
  description: "Gacha waifu impianmu, jaga perasaannya, dan jadikan dia pasanganmu!",
  usage: ".gachawaifu | .waifuku | .tinggalinwaifu",
  example: ".gachawaifu",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 2,
  isEnabled: true,
};

async function getWaifuImage(keyword) {
  try {
    const res = await axios.get(`https://api.cuki.biz.id/api/search/pinterest?apikey=cuki-x&query=${encodeURIComponent(keyword)}&type=image`);
    const results = res.data?.data?.results;
    if (results && results.length > 0) {
      const validImages = results.filter((item) => item.image_url);
      if (validImages.length > 0) {
        const limit = Math.min(15, validImages.length);
        return validImages[Math.floor(Math.random() * limit)].image_url;
      }
    }
  } catch (e) {
    console.error("[GachaWaifu] Pinterest API error:", e.message);
  }
  return "https://i.pinimg.com/736x/8f/3e/2a/8f3e2a77ec65cdbcfad4ff3bc17e825f.jpg";
}

async function getBuffer(url) {
  try {
    const res = await axios.get(url, { responseType: "arraybuffer", timeout: 15000 });
    return Buffer.from(res.data);
  } catch {
    return null;
  }
}

function todayStr() {
  return new Date().toLocaleDateString("id-ID", { timeZone: "Asia/Jakarta" });
}

function moodLabel(mood) {
  return { ceria: "ceria 😄", romantis: "romantis 💘", biasa: "biasa 🙂", sedih: "sedih 😢", marah: "marah 😡" }[mood] || mood;
}

function moodState(user, m) {
  const w = user.waifu;
  if (w.moodUntil && Date.now() < new Date(w.moodUntil).getTime() && w.mood?.type) return w.mood.type;
  return getDailyMood(m.sender, todayStr());
}

async function sendWaifuMessage(m, sock, waifu, textContent, customButtons = null) {
  let imgBuffer = null;
  if (waifu.imageUrl) imgBuffer = await getBuffer(waifu.imageUrl);
  if (!imgBuffer) {
    const newUrl = await getWaifuImage(waifu.keyword);
    waifu.imageUrl = newUrl;
    imgBuffer = await getBuffer(newUrl) || Buffer.alloc(0);
  }
  const media = await prepareWAMessageMedia({ image: imgBuffer }, { upload: sock.waUploadToServer });
  let buttons = customButtons;
  if (!buttons) {
    if (waifu.affection < 80) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🚶 Jalan-jalan", id: `${m.prefix}waifuaction menu_jalanjalan` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "☕ Kafe", id: `${m.prefix}waifuaction menu_kafe` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎬 Bioskop", id: `${m.prefix}waifuaction menu_bioskop` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛍️ Belanja", id: `${m.prefix}waifuaction menu_belanja` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎮 Lainnya", id: `${m.prefix}waifuaction menu_lainnya` }) },
      ];
    } else if (waifu.affection < 100) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🫂 Peluk", id: `${m.prefix}waifuaction menu_peluk` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💋 Cium", id: `${m.prefix}waifuaction menu_cium` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛏️ Tidur", id: `${m.prefix}waifuaction menu_tidur` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛁 Mandi", id: `${m.prefix}waifuaction menu_mandi` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Lainnya", id: `${m.prefix}waifuaction menu_lainnya` }) },
      ];
    } else if (!waifu.married) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💍 Nikahi", id: `${m.prefix}waifuaction nikah` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💝 Hadiah", id: `${m.prefix}waifuaction hadiah` }) },
      ];
    } else {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "👩‍❤️‍👨 Mesra", id: `${m.prefix}waifuaction mesra` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💌 Rayu", id: `${m.prefix}waifuaction rayu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🌴 Bulan Madu", id: `${m.prefix}waifuaction menu_bulanmadu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Klaim Harian", id: `${m.prefix}waifuaction klaim` }) },
      ];
    }
  }
  let footerText = "❤️ Jaga terus perasaannya ya!";
  if (customButtons) footerText = "💭 Dia menunggu jawabanmu...";
  else if (waifu.married) footerText = "❤️ Kamu sudah menikahinya!";

  const msg = generateWAMessageFromContent(m.chat, {
    viewOnceMessage: {
      message: {
        messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
        interactiveMessage: {
          body: { text: textContent },
          footer: { text: footerText },
          header: {
            title: `🌟 *${waifu.tier.toUpperCase()} TIER WAIFU* 🌟`,
            subtitle: waifu.name,
            hasMediaAttachment: true,
            imageMessage: media.imageMessage,
          },
          nativeFlowMessage: { buttons },
        },
      },
    },
  }, { quoted: m });

  await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
}

function marrageDay(waifu) {
  if (!waifu.marriedDate) return 0;
  const a = new Date(waifu.marriedDate);
  const b = new Date();
  return Math.floor((b - a) / 86400000) + 1;
}

function initStats(user) {
  if (!user.waifuStats) user.waifuStats = { totalGacha: 0, byTier: {}, pityCounter: 0, rarest: null, marriedCount: 0 };
  return user.waifuStats;
}

function recordPull(user, waifu, stats) {
  stats.totalGacha++;
  stats.byTier[waifu.tier] = (stats.byTier[waifu.tier] || 0) + 1;
  const isEpicPlus = ["Epic", "Legendary", "Mythic"].includes(waifu.tier);
  stats.pityCounter = isEpicPlus ? 0 : (stats.pityCounter || 0) + 1;
  if (!stats.rarest || ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(waifu.tier) > ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(stats.rarest.tier)) {
    stats.rarest = { name: waifu.name, tier: waifu.tier };
  }
  if (!user.waifuHistory) user.waifuHistory = [];
  user.waifuHistory.push({ name: waifu.name, series: waifu.series, tier: waifu.tier, at: new Date().toISOString() });
  if (user.waifuHistory.length > 100) user.waifuHistory = user.waifuHistory.slice(-100);
}

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const cmd = m.command.toLowerCase();

  if (cmd === "waifuku" || cmd === "istriku") {
    if (!user.waifu) return m.reply(`⚠️ *Kamu belum memiliki waifu!*\nSilakan ketik *${m.prefix}gachawaifu* untuk memulainya!`);
    m.react("🕕");
    const w = user.waifu;
    const mood = moodState(user, m);
    let status = w.married ? "Telah Menikah 💍" : "Pendekatan 💖";
    const day = marrageDay(w);
    const moodLine = w.married
      ? `\n🗓️ *Hari ke-${day}*${day >= 7 ? ` ${day >= 100 ? "🏆" : day >= 30 ? "🎖️" : "🎉"}` : ""}`
      : "";
    const textContent = `📸 *STATUS WAIFU KAMU* 📸\n\n` +
      `💖 *Nama:* ${w.name}\n` +
      `💎 *Tier:* ${w.tier}\n` +
      `🎭 *Personality:* ${w.personality}\n` +
      `🌤️ *Mood hari ini:* ${moodLabel(mood)}\n` +
      `💞 *Affection:* ${w.affection}/100\n` +
      `💍 *Status:* ${status}${moodLine}\n\n` +
      `Lanjutkan interaksi dengan memilih aksi di bawah!`;
    m.react("✅");
    return await sendWaifuMessage(m, sock, w, textContent, null);
  }

  if (cmd === "tinggalinwaifu") {
    if (!user.waifu) return m.reply(`⚠️ *Kamu bahkan belum punya waifu!* Halu ya?`);
    const waifuName = user.waifu.name;
    const waifuJid = 'waifu_' + waifuName.replace(/\s+/g, '') + '@s.whatsapp.net';
    if (user.waifu.married) {
      if (user.fun && user.fun.pasangan === waifuJid) user.fun.pasangan = "";
      db.setUser(waifuJid, { fun: { pasangan: "" } });
    }
    delete user.waifu;
    db.setUser(m.sender, user);
    m.react("💔");
    return m.reply(`💔 *KAMU MEMUTUSKAN UNTUK MENINGGALKAN ${waifuName.toUpperCase()}!*\n\nKamu mengemas barang-barangmu dan pergi. Dia menangis tersedu-sedu. Kalian kini resmi berpisah.`);
  }

  if (cmd === "gachawaifu" || cmd === "gachaistri") {
    if (user.waifu) {
      m.react("😡");
      return m.reply(`⚠️ *Kamu sudah memiliki waifu!*\nNama: *${user.waifu.name}*\nTier: *${user.waifu.tier}*\nAffection: *${user.waifu.affection}/100*\n\nJangan serakah! Ketik *${m.prefix}waifuku* untuk berinteraksi dengannya.`);
    }
    const sub = (m.args[0] || "").toLowerCase();
    if (sub !== "start") {
      const panduan = `💕 *SISTEM GACHA WAIFU* 💕\n\n` +
        `Simulasi kencan virtual interaktif. Dapatkan waifu impianmu, dekati hatinya, dan nikahi dia!\n\n` +
        `*PENGGUNAAN:*\n` +
        `• *${m.prefix}gachawaifu* — Panduan ini\n` +
        `• *${m.prefix}waifuku* — Panel interaksi\n` +
        `• *${m.prefix}waifualbum* — Riwayat & statistik luck\n` +
        `• *${m.prefix}waifupool* — Jelajahi pool waifu\n` +
        `• *${m.prefix}tinggalinwaifu* — Putuskan hubungan\n\n` +
        `*ALUR:*\n1. Tekan tombol **Mulai Gacha**.\n2. 3 Fase: Pendekatan (<80) → Intim (80–99) → Menikah (100).\n3. Mood & personality memengaruhi poin affection.\n4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+.\n5. Aksi memberi EXP + reward ekonomi.`;
      const msg = generateWAMessageFromContent(m.chat, {
        viewOnceMessage: {
          message: {
            messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
            interactiveMessage: {
              body: { text: panduan },
              footer: { text: "Tekan tombol untuk mulai!" },
              nativeFlowMessage: { buttons: [{ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎲 Mulai Gacha!", id: `${m.prefix}gachawaifu start` }) }] },
            },
          },
        },
      }, { quoted: m });
      return await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
    }

    m.react("🕕");
    try {
      const stats = initStats(user);
      const picked = rollWaifu(stats.pityCounter);
      const waifu = {
        ...picked,
        affection: 50,
        married: false,
        imageUrl: await getWaifuImage(picked.keyword),
      };
      recordPull(user, waifu, stats);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      const textContent = `🎉 *SELAMAT! KAMU MENDAPATKAN WAIFU BARU!* 🎉\n\n` +
        `💖 *Nama:* ${waifu.name}\n🎂 *Usia:* ${waifu.age} tahun\n📏 *Tinggi:* ${waifu.height}\n⚖️ *Berat:* ${waifu.weight}\n` +
        `💎 *Tier:* ${waifu.tier}\n🎭 *Personality:* ${waifu.personality}\n💞 *Affection:* ${waifu.affection}/100\n\n` +
        `Pilih interaksi untuk mulai PDKT. Hati-hati jangan sampai affection habis!`;
      m.react("✅");
      await sendWaifuMessage(m, sock, waifu, textContent, null);
    } catch (err) {
      console.error(err);
      m.react("☢");
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
    return;
  }

  if (cmd === "waifuaction") {
    if (!user.waifu) { m.react("❌"); return m.reply(`Kamu belum memiliki waifu! Ketik *${m.prefix}gachawaifu* untuk memulai.`); }

    const action = (m.args[0] || "").toLowerCase();
    const waifu = user.waifu;
    const mood = moodState(user, m);
    const sendMenu = (title, options) => sendWaifuMessage(m, sock, waifu, title, options.map(([label, id]) => ({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: label, id: `${m.prefix}waifuaction ${id}` }) })));

    const MENUS = {
      menu_jalanjalan: ["🚶 Jalan-jalan ke...", [["🌳 Taman", "jalan_taman"], ["🏢 Mall", "jalan_mall"], ["🏖️ Pantai", "jalan_pantai"], ["🌆 Kota", "jalan_kota"]]],
      menu_kafe: ["☕ Pesan untuknya...", [["☕ Kopi Pahit", "kafe_kopi"], ["🍵 Matcha Latte", "kafe_matcha"], ["🍰 Kue", "kafe_kue"], ["🥛 Susu Hangat", "kafe_susu"]]],
      menu_bioskop: ["🎬 Pilih genre film...", [["💞 Romantis", "bioskop_romantis"], ["👻 Horor", "bioskop_horor"], ["🎨 Animasi", "bioskop_animasi"]]],
      menu_belanja: ["🛍️ Belikan dia apa?..", [["👗 Baju Lucu", "belanja_baju"], ["💎 Perhiasan", "belanja_perhiasan"], ["🧸 Boneka", "belanja_boneka"]]],
      menu_karaoke: ["🎤 Karaoke...", [["🎶 Duet", "karaoke_duet"], ["🎵 Solo", "karaoke_solo"]]],
      menu_arcade: ["🕹️ Di arcade...", [["🎮 Adu Skor", "arcade_duo"], ["🎯 Main Boneka", "arcade_boneka"]]],
      menu_piknik: ["🧺 Piknik...", [["🌳 Di Taman", "piknik_taman"], ["🏝️ Di Pantai", "piknik_pantai"]]],
      menu_masak: ["🍳 Masak bareng...", [["🍳 Masakan Rumah", "masak_bareng"], ["🍰 Kue", "masak_kue"]]],
      menu_lainnya: ["🎮 Aksi lainnya...", waifu.affection < 80
        ? [["🎤 Karaoke", "menu_karaoke"], ["🕹️ Arcade", "menu_arcade"], ["🧺 Piknik", "menu_piknik"], ["🍳 Masak", "menu_masak"]]
        : waifu.affection < 100
          ? [["🤗 Gendong", "menu_gendong"], ["🖐️ Tepuk Kepala", "tepuk_kepala"]]
          : [["🌴 Bulan Madu", "menu_bulanmadu"], ["💝 Hadiah", "hadiah"]]],
      menu_peluk: ["🫂 Cara memeluk...", [["🤗 Dari Belakang", "peluk_belakang"], ["💑 Berhadapan", "peluk_depan"]]],
      menu_cium: ["💋 Cium di...", [["😚 Kening", "cium_kening"], ["💋 Bibir", "cium_bibir"]]],
      menu_tidur: ["🛏️ Tidur bareng...", [["🫂 Pulas", "tidur_kelon"], ["🌙 Serandu", "tidur_serandu"]]],
      menu_mandi: ["🛁 Mandi bareng...", [["🛁 Gosok Punggung", "mandi_punggung"], ["🧼 Usap Bahu", "mandi_bahu"]]],
      menu_gendong: ["🤗 Gendong...", [["👸 Gaya Putri", "gendong_putri"], ["🏃 Punggung", "gendong_punggung"]]],
      menu_bulanmadu: ["🌴 Bulan madu...", [["🏝️ Pantai", "bulanmadu_pantai"], ["🏨 Hotel", "bulanmadu_hotel"]]],
    };

    if (MENUS[action]) {
      const [title, options] = MENUS[action];
      return sendMenu(title, options);
    }

    if (["peluk", "cium", "tidur", "mandi", "gendong_putri", "gendong_punggung", "tepuk_kepala", "intim_belakang", "intim_depan", "intim_kening", "intim_bibir", "intim_kelon", "intim_panas", "intim_punggung", "intim_bahu"].includes(action) && waifu.affection < 80) {
      const drop = Math.floor(Math.random() * 30) + 30;
      waifu.affection = Math.max(0, waifu.affection - drop);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      m.react("💢");
      return m.reply(`💢 *PLAKK!!* Kamu mencoba bersikap mesum kepada *${waifu.name}*, tapi kalian belum sedekat itu! Dia menamparmu keras.\n💞 *Affection -${drop}*`);
    }

    const result = applyAction(action, waifu, mood);
    if (!result) { m.react("❓"); return m.reply(`Aksi tidak dikenali. Gunakan tombol waifu.`); }

    if (result.phase === "married" && !waifu.married) { m.react("⛔"); return m.reply(`Aksi ini hanya untuk pasangan suami istri!`); }
    if (result.phase === "intim" && !waifu.married && waifu.affection < 80) return;

    if (action === "nikah") {
      if (waifu.affection < 100) return m.reply(`⚠️ Affection belum 100! Jangan terburu-buru melamar!`);
      if (waifu.married) return m.reply(`⚠️ Kalian kan sudah menikah!`);
      waifu.married = true;
      waifu.marriedDate = new Date().toISOString();
      const dowry = DOWRY[waifu.tier];
      db.updateEnergi(m.sender, dowry.limit);
      db.updateKoin(m.sender, dowry.koin);
      await addExpWithLevelCheck(sock, m, db, user, dowry.exp);
      const stats = initStats(user);
      stats.marriedCount = (stats.marriedCount || 0) + 1;
      if (!user.fun) user.fun = {};
      const waifuJid = 'waifu_' + waifu.name.replace(/\s+/g, '') + '@s.whatsapp.net';
      user.fun.pasangan = waifuJid;
      db.setUser(waifuJid, { fun: { pasangan: m.sender }, name: waifu.name });
      user.waifu = waifu;
      db.setUser(m.sender, user);
      m.react("💍");
      return m.reply(`💍 *KAMU RESMI MENIKAH DENGAN ${waifu.name.toUpperCase()}!* 💍\n\nSebagai dowry:\n- ⚡ ${dowry.limit} Limit/Energi\n- 💰 ${dowry.koin.toLocaleString()} Koin\n- ✨ ${dowry.exp.toLocaleString()} EXP\n\nStatus \`.cekpacar\` kini resmi berpasangan!`);
    }

    if (action === "hadiah") {
      if (waifu.affection < 100) return m.reply(`⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!`);
      const g = 300 + Math.floor(Math.random() * 401);
      db.updateEnergi(m.sender, g);
      m.react("💝");
      return m.reply(`💝 *${waifu.name}* memberimu hadiah!\nKamu mendapat ⚡ ${g} Limit/Energi!`);
    }

    if (action === "klaim") {
      if (!waifu.married) return m.reply(`⚠️ Hanya istri sah yang bisa klaim hadiah harian!`);
      const last = waifu.lastClaimDate;
      if (last === todayStr()) return m.reply(`⚠️ Kamu sudah klaim hari ini! Coba lagi besok.`);
      const tierMult = { Common: 1, Rare: 1.2, Epic: 1.5, Legendary: 2, Mythic: 3 }[waifu.tier] || 1;
      const koin = Math.floor((5000 + Math.random() * 15001) * tierMult);
      const exp = Math.floor((200 + Math.random() * 601) * tierMult);
      const day = marrageDay(waifu);
      let bonus = "";
      if (day === 7 || day === 30 || day === 100) {
        const bKoin = day * 1000 * tierMult;
        db.updateKoin(m.sender, bKoin);
        bonus = `\n🎉 *MILESTONE HARI KE-${day}!* Bonus +${bKoin.toLocaleString()} Koin!`;
      }
      waifu.lastClaimDate = todayStr();
      user.waifu = waifu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, koin);
      await addExpWithLevelCheck(sock, m, db, user, exp);
      m.react("🎁");
      return m.reply(`🎁 *${waifu.name}* memberi hadiah harian!\n💰 +${koin.toLocaleString()} Koin\n✨ +${exp} EXP${bonus}`);
    }

    if (result.phase === "married" && action !== "nikah") {
      waifu.affection = Math.min(100, waifu.affection + result.change);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, Math.floor(result.change * 100));
      await addExpWithLevelCheck(sock, m, db, user, result.exp);
      m.react("❤️");
      return m.reply(`${result.text}\n\n💞 *Affection +${result.change}* (Total: ${waifu.affection}/100)\n💰 +${Math.floor(result.change * 100)} Koin\n✨ +${result.exp} EXP`);
    }

    // aksi approach/intim biasa
    const affBefore = waifu.affection;
    let newAff = affBefore + result.change;
    let eventBlock = "";
    let eventExp = 0;
    let eventKoin = 0;
    const ev = rollEvent({ married: waifu.married, phase: result.phase, personality: waifu.personality });
    if (ev) {
      newAff += ev.aff;
      eventKoin = ev.koin || 0;
      if (ev.mood) { waifu.mood = { type: ev.mood, since: new Date().toISOString() }; waifu.moodUntil = new Date(Date.now() + 12 * 3600000).toISOString(); }
      if (ev.nextMult < 1) waifu.nextMultUntil = new Date(Date.now() + 24 * 3600000).toISOString();
      eventBlock = `\n\n✨ *EVENT:* ${ev.text}${ev.aff ? ` (${ev.aff > 0 ? "+" : ""}${ev.aff} aff)` : ""}${eventKoin ? ` (+${eventKoin.toLocaleString()} koin)` : ""}`;
      eventExp = result.exp;
    }
    newAff = Math.max(0, Math.min(100, newAff));
    waifu.affection = newAff;
    user.waifu = waifu;
    db.setUser(m.sender, user);
    if (eventKoin) db.updateKoin(m.sender, eventKoin);
    await addExpWithLevelCheck(sock, m, db, user, result.exp + (eventExp ? 15 : 0));

    let affText = `💞 *Affection:* ${waifu.affection}/100`;
    if (result.change !== 0) affText = `💞 *Affection berubah:* ${result.change > 0 ? "+" : ""}${result.change} (Total: ${waifu.affection}/100)`;
    if (waifu.affection === 100) affText = `💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Nikahi dia sekarang!*`;

    if (waifu.affection <= 0) {
      const waifuName = waifu.name;
      if (waifu.married) {
        const waifuJid = 'waifu_' + waifuName.replace(/\s+/g, '') + '@s.whatsapp.net';
        if (user.fun && user.fun.pasangan === waifuJid) user.fun.pasangan = "";
        db.setUser(waifuJid, { fun: { pasangan: "" } });
      }
      delete user.waifu;
      db.setUser(m.sender, user);
      m.react("💔");
      return m.reply(`💔 *${waifuName.toUpperCase()} MENINGGALKANMU!* 💔\n\nKarena kasih sayangnya habis (0), dia pergi meninggalkan surat basah air mata. Kamu kehilangan waifumu! *(Ketik ${m.prefix}gachawaifu untuk memulai ulang)*`);
    }

    m.react(waifu.affection === 100 ? "💍" : "✨");
    const updated = `${result.text}${eventBlock}\n\n${affText}\n✨ +${result.exp} EXP${eventKoin ? `\n💰 +${eventKoin.toLocaleString()} Koin` : ""}`;
    await sendWaifuMessage(m, sock, waifu, updated, null);
  }
}

export { pluginConfig as config, handler };
```

- [ ] **Step 2: Run syntax check**

Run: `node --check plugins/fun/gachawaifu.js`
Expected: no output (syntax OK).

- [ ] **Step 3: Smoke test data/lib wiring**

Run: `node -e "import('./src/lib/ourin-waifu.js').then(m => { const w = m.rollWaifu(0); console.log('roll:', w.name, w.tier); console.log('action:', JSON.stringify(m.applyAction('kafe_matcha', {personality:'dandere', name:'Miku'}, 'ceria', ()=>0))); })"`
Expected: prints a waifu + action result object.

- [ ] **Step 4: Commit**

```bash
git add plugins/fun/gachawaifu.js
git commit -m "feat(waifu): refactor gachawaifu plugin — pity, event, mood, ekonomi, aksi diperluas"
```

---

### Task 7: waifualbum plugin

**Files:**
- Create: `plugins/fun/waifualbum.js`

**Interfaces:**
- Consumes: `albumStats` from `../../src/lib/ourin-waifu.js`; `getDatabase`.

- [ ] **Step 1: Write `plugins/fun/waifualbum.js`**

```js
import { getDatabase } from "../../src/lib/ourin-database.js";
import { albumStats } from "../../src/lib/ourin-waifu.js";

const pluginConfig = {
  name: ["waifualbum", "albumwaifu"],
  alias: [],
  category: "fun",
  description: "Lihat riwayat gacha waifu & statistik luck kamu!",
  usage: ".waifualbum",
  example: ".waifualbum",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const history = user.waifuHistory || [];
  if (!history.length) {
    return m.reply(`📭 *Album kosong!*\nKamu belum pernah gacha waifu. Ketik *${m.prefix}gachawaifu* untuk mulai!`);
  }

  const stats = albumStats(history, user.waifuStats || {});
  const tierEmoji = { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" };
  const bar = (count) => {
    const pct = stats.total ? Math.round((count / stats.total) * 20) : 0;
    return "█".repeat(pct) + "░".repeat(20 - pct);
  };

  let text = `📚 *ALBUM WAIFU & LUCK STATS* 📚\n\n` +
    `🎲 *Total Gacha:* ${stats.total}\n` +
    `🍀 *Luck Score:* ${stats.luck} ${stats.luck >= 1 ? "🔥" : "😅"}\n\n` +
    `*Distribusi Tier:*\n` +
    `🟢 Common     ${String(stats.byTier.Common || 0).padStart(3)} ${bar(stats.byTier.Common || 0)}\n` +
    `🔵 Rare       ${String(stats.byTier.Rare || 0).padStart(3)} ${bar(stats.byTier.Rare || 0)}\n` +
    `🟣 Epic       ${String(stats.byTier.Epic || 0).padStart(3)} ${bar(stats.byTier.Epic || 0)}\n` +
    `🟡 Legendary  ${String(stats.byTier.Legendary || 0).padStart(3)} ${bar(stats.byTier.Legendary || 0)}\n` +
    `🔴 Mythic     ${String(stats.byTier.Mythic || 0).padStart(3)} ${bar(stats.byTier.Mythic || 0)}\n\n`;

  if (stats.rarest) text += `🏆 *Pull Paling Langka:* ${tierEmoji[stats.rarest.tier] || ""} ${stats.rarest.name} (${stats.rarest.tier})\n`;
  if (stats.pityCounter > 0) text += `🎯 *Pity aktif:* ${stats.pityCounter}/20 roll tanpa Epic+\n`;
  text += `💍 *Pernikahan:* ${stats.marriedCount || 0}\n\n*10 Pull Terakhir:*\n`;

  stats.last10.forEach((h, i) => {
    text += `${i + 1}. ${tierEmoji[h.tier] || ""} *${h.name}* — ${h.tier}\n`;
  });

  return m.reply(text);
}

export { pluginConfig as config, handler };
```

- [ ] **Step 2: Syntax check**

Run: `node --check plugins/fun/waifualbum.js`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add plugins/fun/waifualbum.js
git commit -m "feat(waifu): plugin waifualbum — riwayat & luck stats"
```

---

### Task 8: waifupool plugin

**Files:**
- Create: `plugins/fun/waifupool.js`

**Interfaces:**
- Consumes: `searchPool` from `../../data/waifu/index.js`.

- [ ] **Step 1: Write `plugins/fun/waifupool.js`**

```js
import { getDatabase } from "../../src/lib/ourin-database.js";
import { searchPool } from "../../data/waifu/index.js";

const pluginConfig = {
  name: ["waifupool", "poolwaifu"],
  alias: [],
  category: "fun",
  description: "Jelajahi pool waifu — cari nama/franchise/tier!",
  usage: ".waifupool [nama|tier|franchise]",
  example: ".waifupool rare",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

const pages = new Map(); // `${jid}:${query}` -> page index

function tierEmoji(t) {
  return { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" }[t] || "";
}

async function handler(m, { sock }) {
  const db = getDatabase();
  const query = (m.args.join(" ") || "").trim();
  const key = `${m.sender}:${query.toLowerCase()}`;
  const page = Math.max(0, pages.get(key) || 0);
  const PAGE_SIZE = 10;

  const pool = searchPool(query);
  if (!pool.length) {
    pages.set(key, 0);
    return m.reply(`🔍 Tidak ada waifu cocok dengan *"${query}"*. Coba nama/tier/franchise lain.`);
  }

  const totalPages = Math.max(1, Math.ceil(pool.length / PAGE_SIZE));
  const cur = Math.min(page, totalPages - 1);
  const slice = pool.slice(cur * PAGE_SIZE, (cur + 1) * PAGE_SIZE);

  let text = `📖 *WAIFU POOL* 📖\n\n`;
  text += query ? `🔍 Pencarian: *"${query}"*\n` : "";
  text += `📊 Total: *${pool.length}* waifu | Halaman *${cur + 1}/${totalPages}*\n\n`;
  slice.forEach((w, i) => {
    text += `${cur * PAGE_SIZE + i + 1}. ${tierEmoji(w.tier)} *${w.name}* — ${w.tier}\n`;
    text += `   🏷️ ${w.series} | 🎭 ${w.personality}\n`;
  });

  const buttons = [];
  if (cur > 0) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "⬅️", id: `${m.prefix}waifupool prev ${query}` }) });
  if (cur < totalPages - 1) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "➡️", id: `${m.prefix}waifupool next ${query}` }) });
  if (buttons.length) text += `\nGunakan tombol untuk berpindah halaman.`;

  pages.set(key, cur);
  return m.reply(text, null, buttons.length ? { buttons } : undefined);
}

export { pluginConfig as config, handler };
```

- [ ] **Step 2: Syntax check**

Run: `node --check plugins/fun/waifupool.js`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add plugins/fun/waifupool.js
git commit -m "feat(waifu): plugin waifupool — jelajahi pool gacha"
```

---

### Task 9: Jealousy hook di handler

**Files:**
- Modify: `src/handler.js` (insert after line 1802 `await plugin.handler(m, context);`)

**Interfaces:**
- Consumes: `jealousyCheck` from `./lib/ourin-waifu.js`.

- [ ] **Step 1: Insert hook**

After `await plugin.handler(m, context);` (handler.js:1802), add:

```js
    if (["fun", "game"].includes(plugin.config.category)) {
      const { jealousyCheck } = await import("./lib/ourin-waifu.js");
      await jealousyCheck({ m, sock, db, command: m.command }).catch(() => {});
    }
```

- [ ] **Step 2: Syntax check**

Run: `node --check src/handler.js`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add src/handler.js
git commit -m "feat(waifu): jealousy trigger hook di handler"
```

---

### Task 10: Verifikasi akhir

- [ ] **Step 1: Run all tests**

Run: `npm test`
Expected: all `.test.mjs` pass (turso + waifu-data + waifu-lib + existing).

- [ ] **Step 2: Syntax check semua file baru**

Run: `for f in data/waifu/*.js src/lib/ourin-waifu.js plugins/fun/gachawaifu.js plugins/fun/waifualbum.js plugins/fun/waifupool.js; do node --check "$f" || echo "FAIL $f"; done`
Expected: no FAIL lines.

- [ ] **Step 3: Boot smoke test (optional, manual)**

Run: `node .` (or `npm start`) — verify bot boots, plugin table shows gachawaifu/waifualbum/waifupool loaded, no loader warnings for waifu data.

- [ ] **Step 4: Commit final if any cleanup**

```bash
git add -A
git commit -m "chore(waifu): verifikasi akhir subsystem gacha"
```

---

## Self-Review

**Spec coverage:**
- §1 tujuan/modular → Task 1–5 (data) + Task 6 (plugin) ✓
- §2 arsitektur file → semua file dibuat ✓
- §3 schema entry + personality → template + loader validation + lib tags ✓
- §4 tier/bobot/pity/dowry → `rollWaifu` + `PITY_THRESHOLD` + `DOWRY` + nikah handler ✓
- §5 random event → `rollEvent` (18%, gates, marriedOnly, phase) ✓
- §6 mood + anniversary → `getDailyMood`, `moodState`, `marrageDay`, `klaim` milestone ✓
- §7 ekonomi → `db.updateKoin/updateEnergi`, `addExpWithLevelCheck` di aksi/nikah/klaim ✓
- §8 jealousy → Task 9 hook + `jealousyCheck` ✓
- §9 album & luck → Task 7 + `albumStats` ✓; waifupool → Task 8 ✓
- §10 aksi diperluas → MENUS + 22+ aksi katalog ✓ (anti-mesum retain)
- §11 Turso compat → state di user object, `getUser/setUser` existing; `data/waifu` static ✓
- §12 error handling → loader skip+warn, jealousy try/catch di hook, plugin try-catch existing ✓
- §13 testing → `tests/waifu-data.test.mjs`, `tests/waifu-lib.test.mjs` ✓
- §14 non-tujuan → tidak ada tabel baru, NSFW tidak ditambah ✓

**Placeholder scan:** semua step punya konten nyata; tidak ada TBD/TODO.

**Type consistency:** `rollWaifu(pityCounter, rng)`, `applyAction(key, waifu, moodType, rng)`, `rollEvent({married, phase, personality}, rng)`, `getDailyMood(senderJid, dateStr)`, `jealousyCheck({m, sock, db, command})`, `albumStats(history, stats)`, `DOWRY` — konsisten antar task (test lib memakai signature yang sama dengan implementasi). `searchPool` di loader vs waifupool ✓.

**Catatan order eksekusi:** Task 2 (lib) bergantung pada data pool untuk lulus tes. Jalankan Task 3–5 sebelum mengecek lib test, atau terima kegagalan sementara. Urutan commit tetap aman karena tes tidak menghalangi commit.
