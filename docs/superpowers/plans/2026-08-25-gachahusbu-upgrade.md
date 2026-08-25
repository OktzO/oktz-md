# GachaHusbu Upgrade + Complexity Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bawa gachahusbu setara gachawaifu (pool 300+ per franchise, personality, mood, event, pity, album, pool, jealousy) + tambah sistem kesulitan/emosi baru (diminishing, fatigue, neglect, anger) untuk dua gender + femboy.

**Architecture:** Subsistem data baru `data/husbu/` (mirror `data/waifu/`), engine shared `src/lib/ourin-romance.js` untuk sistem kesulitan/emosi, lib gender-specific `ourin-waifu.js` (diperluas) dan `ourin-husbu.js` (baru). Plugin `gachahusbu.js` ditulis ulang mirror `gachawaifu.js`; plugin waifu di-upgrade; plugin album/pool baru; `handler.js` beralih ke `jealousyCheckAll`. Test node:test untuk tiap lapisan.

**Tech Stack:** Node ≥22 (ESM), node:test, express-like plugin system (auto-load dari `plugins/fun/`), lowdb+Turso via `ourin-database.js`.

**Spec:** `docs/superpowers/specs/2026-08-25-gachahusbu-upgrade-design.md`

## Global Constraints

- Semua state hidup di objek user via `db.getUser(jid)` / `db.setUser(jid, user)`. Tidak ada schema DB baru.
- Economy memakai API existing: `db.updateKoin(jid, amt)`, `db.updateEnergi(jid, amt)` (LIMIT), `addExpWithLevelCheck(sock, m, db, user, expAmount)` (async, dari `src/lib/ourin-level.js`).
- Test waifu existing WAJIB tetap hijau: `tests/waifu-data.test.mjs`, `tests/waifu-lib.test.mjs`. `applyAction` di `ourin-waifu.js` TIDAK BOLEH mengubah nilai `change` untuk key yang sudah ada.
- Gaya kode: ESM, tanpa semicolon (kecuali di dalam statement yang butuh), `import` di atas, template literal untuk teks. Ikuti gaya `plugins/fun/gachawaifu.js`.
- Nama file/data: pool husbu wajib field `{ name, series, age, height, weight, tier, personality }`. Tier: `Common|Rare|Epic|Legendary|Mythic`.
- `npm test` (node --test tests/) dan `npm run lint` (eslint plugins/) harus bersih.

---

### Task 1: Shared romance engine — `src/lib/ourin-romance.js` + test

**Files:**
- Create: `src/lib/ourin-romance.js`
- Test: `tests/romance-lib.test.mjs`

**Interfaces:**
- Consumes: tidak ada (standalone, tanpa impor internal).
- Produces: konstanta + helper yang dipakai Task 4 (refactor waifu), Task 5 (husbu lib), Task 7-10 (plugin, handler):
  - `export const TIER_ORDER, TIER_VALUE, TIER_EXPECTED, PITY_THRESHOLD, EVENT_CHANCE, ANGER_THRESHOLD`
  - `export const DOWRY` — `{ Common:{limit,koin,exp}, Rare:{}, Epic:{}, Legendary:{}, Mythic:{} }`
  - `export const MOOD_MULT`
  - `export function todayStr()`
  - `export function moodLabel(mood)`
  - `export function getDailyMood(senderJid, dateStr)`
  - `export function diminish(change, current = 50)` → int
  - `export function fatigueMult(stats = {})` → number (stats.actionsToday)
  - `export function neglectDecay(partner = {}, now = Date.now())` → `{decay, hours}`
  - `export function angerEffMood(mood, partner = {})` → string
  - `export function angerUpdate(partner = {}, result)` → int (0..100)
  - `export function applyNeglect(partner = {}, now = Date.now())` → `{decay, hours}` (mutasi `partner.affection`, `partner.anger`, set `partner.lastInteractionAt`)
  - `export function finalGain(result, partner = {}, stats = {})` → `{change, drain, extra, angry}`
  - `export async function jealousyCheckAll({ m, sock, db, command })` → boolean

- [ ] **Step 1: Tulis failing test**

```js
// tests/romance-lib.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert";
import {
  diminish, fatigueMult, neglectDecay, angerEffMood, angerUpdate,
  applyNeglect, finalGain, getDailyMood, moodLabel, DOWRY, ANGER_THRESHOLD,
} from "../src/lib/ourin-romance.js";

describe("Romance shared engine", () => {
  it("diminish: gain mengecil seiring naiknya affection", () => {
    const lo = diminish(20, 50);
    const hi = diminish(20, 90);
    assert.ok(hi < lo, `${hi} should be < ${lo}`);
    assert.ok(hi >= 1);
  });

  it("diminish: floor 0.15 tidak pernah 0", () => {
    assert.ok(diminish(5, 100) >= 1);
  });

  it("fatigueMult: 0-3 aksi full, setelahnya menyusut", () => {
    assert.equal(fatigueMult({ actionsToday: 0 }), 1);
    assert.equal(fatigueMult({ actionsToday: 3 }), 1);
    assert.ok(fatigueMult({ actionsToday: 5 }) < 1);
    assert.ok(fatigueMult({ actionsToday: 50 }) >= 0.25);
  });

  it("neglectDecay: <24 jam = 0, per hari penuh -3, cap 20", () => {
    const now = Date.now();
    assert.deepEqual(neglectDecay({ lastInteractionAt: new Date(now - 1 * 3600000).toISOString() }, now), { decay: 0, hours: 1 });
    const d1 = neglectDecay({ lastInteractionAt: new Date(now - 25 * 3600000).toISOString() }, now);
    assert.equal(d1.decay, 3);
    const d7 = neglectDecay({ lastInteractionAt: new Date(now - 7 * 24 * 3600000).toISOString() }, now);
    assert.equal(d7.decay, 20);
  });

  it("anger: threshold paksa mood marah", () => {
    assert.equal(angerEffMood("ceria", { anger: ANGER_THRESHOLD }), "marah");
    assert.equal(angerEffMood("ceria", { anger: 0 }), "ceria");
    assert.equal(angerUpdate({ anger: 40 }, { dislike: true }), 52);
    assert.equal(angerUpdate({ anger: 40 }, { like: true }), 32);
    assert.equal(angerUpdate({ anger: 95 }, { dislike: true }), 100);
  });

  it("applyNeglect: decay affinity + decay anger + tandai waktu", () => {
    const now = Date.now();
    const p = { affection: 50, anger: 30, lastInteractionAt: new Date(now - 49 * 3600000).toISOString() };
    const r = applyNeglect(p, now);
    assert.equal(r.decay, 3);
    assert.equal(p.affection, 47);
    assert.equal(p.anger, 20);
    assert.ok(p.lastInteractionAt);
  });

  it("finalGain: diminishing + fatigue + drain anger", () => {
    const base = { change: 20, exp: 40 };
    const p = { affection: 90, anger: 50 };
    const r = finalGain(base, p, { actionsToday: 8 });
    assert.ok(r.change < 20);
    assert.equal(r.angry, true);
    assert.ok(r.extra.length >= 1);
  });

  it("DOWRY punya semua tier", () => {
    for (const t of ["Common", "Rare", "Epic", "Legendary", "Mythic"]) assert.ok(DOWRY[t]);
  });

  it("getDailyMood & moodLabel deterministik", () => {
    assert.equal(getDailyMood("628123", "2026-08-24"), getDailyMood("628123", "2026-08-24"));
    assert.ok(moodLabel("marah").includes("marah"));
  });
});
```

- [ ] **Step 2: Jalankan test, verifikasi FAIL**

Run: `node --test tests/romance-lib.test.mjs`
Expected: FAIL — `Cannot find module '../src/lib/ourin-romance.js'`

- [ ] **Step 3: Tulis implementasi**

```js
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
```

- [ ] **Step 4: Jalankan test, verifikasi PASS**

Run: `node --test tests/romance-lib.test.mjs`
Expected: PASS (10 tests)

- [ ] **Step 5: Commit**

```bash
git add src/lib/ourin-romance.js tests/romance-lib.test.mjs
git commit -m "feat(romance): shared difficulty engine + jealousyCheckAll"
```

---

### Task 2: Pool husbu — loader + template + batch 1 (~91 karakter)

**Files:**
- Create: `data/husbu/index.js`, `data/husbu/_template.js`, `data/husbu/naruto.js`, `data/husbu/one-piece.js`, `data/husbu/bleach.js`, `data/husbu/jujutsu-kaisen.js`, `data/husbu/kimetsu.js`, `data/husbu/my-hero-academia.js`, `data/husbu/attack-on-titan.js`, `data/husbu/dragon-ball.js`
- Test: `tests/husbu-data.test.mjs`

**Interfaces:**
- Consumes: `src/lib/ourin-logger.js` (`logger.warn`).
- Produces: `export const TIER_WEIGHTS`, `export const VALID_HUSB_PERSONALITIES`, `export function getPool()`, `export function searchPool(query = "")`. Format entry: `{ name, series, age, height, weight, tier, personality, keyword? }`. `getPool()` mengembalikan array entry yang sudah punya `rollWeight` (bobot tier agregat) dan `keyword` default `${name} ${series} anime`.

- [ ] **Step 1: Tulis failing test** (uji struktur; assertion `>= 300` baru aktif di Task 5 setelah semua file ada)

```js
// tests/husbu-data.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert";
import { getPool, searchPool, TIER_WEIGHTS, VALID_HUSB_PERSONALITIES } from "../data/husbu/index.js";

describe("Husbu data pool", () => {
  it("loads at least 300 entries", () => {
    assert.ok(getPool().length >= 300, `pool = ${getPool().length}`);
  });
  it("has no duplicate names", () => {
    const names = getPool().map(h => h.name.toLowerCase());
    assert.equal(new Set(names).size, names.length);
  });
  it("all entries have required fields", () => {
    for (const h of getPool()) {
      assert.ok(h.name && h.series && h.age && h.height && h.weight, `missing field: ${h.name}`);
    }
  });
  it("display weight preserved, roll probability on rollWeight", () => {
    for (const h of getPool()) {
      assert.match(String(h.weight), /kg/, `weight clobbered: ${h.name} = ${h.weight}`);
      assert.ok(h.rollWeight > 0, `rollWeight missing: ${h.name}`);
    }
  });
  it("all tiers valid", () => {
    for (const h of getPool()) assert.ok(TIER_WEIGHTS[h.tier], `bad tier: ${h.tier} ${h.name}`);
  });
  it("all personalities valid", () => {
    for (const h of getPool()) assert.ok(VALID_HUSB_PERSONALITIES.includes(h.personality), `bad personality: ${h.personality} ${h.name}`);
  });
  it("tier distribution near target", () => {
    const counts = {};
    for (const h of getPool()) counts[h.tier] = (counts[h.tier] || 0) + 1;
    assert.ok(counts.Common >= 20, `Common=${counts.Common}`);
    assert.ok(counts.Rare >= 40, `Rare=${counts.Rare}`);
    assert.ok(counts.Epic >= 40, `Epic=${counts.Epic}`);
    assert.ok(counts.Legendary >= 20, `Legendary=${counts.Legendary}`);
    assert.ok(counts.Mythic >= 4, `Mythic=${counts.Mythic}`);
  });
  it("searchPool filters by name/series/tier/personality", () => {
    const pool = getPool();
    const n = pool[0];
    assert.ok(searchPool(n.name).some(h => h.name === n.name));
    assert.ok(searchPool(n.tier).every(h => h.tier === n.tier));
  });
});
```

- [ ] **Step 2: Jalankan test, verifikasi FAIL**

Run: `node --test tests/husbu-data.test.mjs`
Expected: FAIL — `Cannot find module '../data/husbu/index.js'`

- [ ] **Step 3: Tulis loader + template + 8 file franchise batch 1**

```js
// data/husbu/index.js
// Loader pool husbu: merge file franchise, validasi entry, dedupe nama, hitung bobot tier.
import { logger } from "../../src/lib/ourin-logger.js";
import { husbus as naruto } from "./naruto.js";
import { husbus as onePiece } from "./one-piece.js";
import { husbus as bleach } from "./bleach.js";
import { husbus as jujutsu } from "./jujutsu-kaisen.js";
import { husbus as kimetsu } from "./kimetsu.js";
import { husbus as mha } from "./my-hero-academia.js";
import { husbus as aot } from "./attack-on-titan.js";
import { husbus as dragonBall } from "./dragon-ball.js";

export const TIER_WEIGHTS = { Common: 55, Rare: 25, Epic: 13, Legendary: 5.5, Mythic: 1.5 };
export const VALID_HUSB_PERSONALITIES = [
  "tsundere", "kuudere", "genki", "yandere", "dandere", "oji-san",
  "playboy", "prince", "badboy", "sunao", "femboy",
];

const FRANCHISES = [
  naruto, onePiece, bleach, jujutsu, kimetsu, mha, aot, dragonBall,
];

let cache = null;

function buildPool() {
  const seen = new Set();
  const all = [];
  for (const list of FRANCHISES) {
    for (const h of list || []) {
      const missing = !h || !h.name || !h.series || !h.age || !h.height || !h.weight || !h.tier || !h.personality;
      if (missing) { logger.warn("husbu", `skip invalid entry (${h?.name || "no-name"})`); continue; }
      if (!TIER_WEIGHTS[h.tier]) { logger.warn("husbu", `skip invalid tier ${h.tier} (${h.name})`); continue; }
      if (!VALID_HUSB_PERSONALITIES.includes(h.personality)) { logger.warn("husbu", `skip invalid personality ${h.personality} (${h.name})`); continue; }
      const key = h.name.trim().toLowerCase();
      if (seen.has(key)) { logger.warn("husbu", `duplicate name: ${h.name}`); continue; }
      seen.add(key);
      all.push({ ...h, name: h.name.trim(), series: h.series.trim(), keyword: h.keyword || `${h.name} ${h.series} anime` });
    }
  }
  const tierCount = {};
  for (const h of all) tierCount[h.tier] = (tierCount[h.tier] || 0) + 1;
  for (const h of all) h.rollWeight = TIER_WEIGHTS[h.tier] / tierCount[h.tier];
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
  return pool.filter(h =>
    h.name.toLowerCase().includes(q) ||
    h.series.toLowerCase().includes(q) ||
    h.tier.toLowerCase().includes(q) ||
    h.personality.toLowerCase().includes(q)
  );
}
```

```js
// data/husbu/_template.js
// Template untuk mengisi file franchise husbu. Wajib: name, series, age, height, weight, tier, personality.
// personality husbu: tsundere|kuudere|genki|yandere|dandere|oji-san|playboy|prince|badboy|sunao|femboy
export const husbus = [
  {
    name: "Gojo Satoru",              // required, unik lintas franchise
    series: "Jujutsu Kaisen",         // required
    age: 28, height: "190 cm", weight: "85 kg",  // required
    tier: "Mythic",                   // Common|Rare|Epic|Legendary|Mythic
    personality: "playboy",
    keyword: "",                      // optional override keyword Pinterest
  },
];
```

`data/husbu/naruto.js`:
```js
export const husbus = [
  { name: "Naruto Uzumaki", series: "Naruto", age: 17, height: "166 cm", weight: "51 kg", tier: "Epic", personality: "genki" },
  { name: "Sasuke Uchiha", series: "Naruto", age: 16, height: "168 cm", weight: "52 kg", tier: "Epic", personality: "tsundere" },
  { name: "Kakashi Hatake", series: "Naruto", age: 30, height: "181 cm", weight: "67 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Itachi Uchiha", series: "Naruto", age: 21, height: "178 cm", weight: "58 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Shikamaru Nara", series: "Naruto", age: 16, height: "170 cm", weight: "54 kg", tier: "Common", personality: "kuudere" },
  { name: "Rock Lee", series: "Naruto", age: 16, height: "163 cm", weight: "51 kg", tier: "Common", personality: "genki" },
  { name: "Gaara", series: "Naruto", age: 15, height: "166 cm", weight: "49 kg", tier: "Rare", personality: "dandere" },
  { name: "Minato Namikaze", series: "Naruto", age: 24, height: "179 cm", weight: "66 kg", tier: "Legendary", personality: "sunao" },
  { name: "Jiraiya", series: "Naruto", age: 54, height: "191 cm", weight: "88 kg", tier: "Epic", personality: "playboy" },
  { name: "Shino Aburame", series: "Naruto", age: 16, height: "175 cm", weight: "57 kg", tier: "Common", personality: "dandere" },
  { name: "Sai", series: "Naruto", age: 17, height: "172 cm", weight: "53 kg", tier: "Rare", personality: "dandere" },
  { name: "Obito Uchiha", series: "Naruto", age: 31, height: "175 cm", weight: "71 kg", tier: "Epic", personality: "badboy" },
  { name: "Madara Uchiha", series: "Naruto", age: 73, height: "179 cm", weight: "71 kg", tier: "Mythic", personality: "badboy" },
];
```

`data/husbu/one-piece.js`:
```js
export const husbus = [
  { name: "Monkey D. Luffy", series: "One Piece", age: 19, height: "174 cm", weight: "64 kg", tier: "Epic", personality: "genki" },
  { name: "Roronoa Zoro", series: "One Piece", age: 21, height: "181 cm", weight: "71 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Sanji", series: "One Piece", age: 21, height: "180 cm", weight: "70 kg", tier: "Legendary", personality: "playboy" },
  { name: "Trafalgar Law", series: "One Piece", age: 26, height: "191 cm", weight: "75 kg", tier: "Epic", personality: "kuudere" },
  { name: "Portgas D. Ace", series: "One Piece", age: 20, height: "185 cm", weight: "73 kg", tier: "Epic", personality: "sunao" },
  { name: "Sabo", series: "One Piece", age: 22, height: "187 cm", weight: "76 kg", tier: "Epic", personality: "sunao" },
  { name: "Shanks", series: "One Piece", age: 39, height: "199 cm", weight: "90 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Donquixote Doflamingo", series: "One Piece", age: 41, height: "305 cm", weight: "98 kg", tier: "Legendary", personality: "badboy" },
  { name: "Dracule Mihawk", series: "One Piece", age: 43, height: "198 cm", weight: "90 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Corazon", series: "One Piece", age: 26, height: "205 cm", weight: "98 kg", tier: "Rare", personality: "sunao" },
  { name: "Kuzan (Aokiji)", series: "One Piece", age: 49, height: "298 cm", weight: "92 kg", tier: "Epic", personality: "oji-san" },
  { name: "Marco", series: "One Piece", age: 45, height: "203 cm", weight: "92 kg", tier: "Epic", personality: "oji-san" },
];
```

`data/husbu/bleach.js`:
```js
export const husbus = [
  { name: "Ichigo Kurosaki", series: "Bleach", age: 17, height: "181 cm", weight: "66 kg", tier: "Epic", personality: "sunao" },
  { name: "Ulquiorra Cifer", series: "Bleach", age: 20, height: "169 cm", weight: "55 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Byakuya Kuchiki", series: "Bleach", age: 28, height: "180 cm", weight: "64 kg", tier: "Epic", personality: "prince" },
  { name: "Sosuke Aizen", series: "Bleach", age: 27, height: "186 cm", weight: "74 kg", tier: "Mythic", personality: "badboy" },
  { name: "Toshiro Hitsugaya", series: "Bleach", age: 15, height: "133 cm", weight: "28 kg", tier: "Epic", personality: "tsundere" },
  { name: "Kenpachi Zaraki", series: "Bleach", age: 32, height: "202 cm", weight: "90 kg", tier: "Legendary", personality: "badboy" },
  { name: "Kisuke Urahara", series: "Bleach", age: 37, height: "174 cm", weight: "60 kg", tier: "Epic", personality: "playboy" },
  { name: "Gin Ichimaru", series: "Bleach", age: 27, height: "185 cm", weight: "66 kg", tier: "Rare", personality: "playboy" },
  { name: "Renji Abarai", series: "Bleach", age: 23, height: "188 cm", weight: "78 kg", tier: "Rare", personality: "genki" },
  { name: "Grimmjow Jaegerjaquez", series: "Bleach", age: 24, height: "186 cm", weight: "80 kg", tier: "Rare", personality: "badboy" },
];
```

`data/husbu/jujutsu-kaisen.js`:
```js
export const husbus = [
  { name: "Gojo Satoru", series: "Jujutsu Kaisen", age: 28, height: "190 cm", weight: "85 kg", tier: "Mythic", personality: "playboy" },
  { name: "Ryomen Sukuna", series: "Jujutsu Kaisen", age: 1000, height: "173 cm", weight: "80 kg", tier: "Mythic", personality: "badboy" },
  { name: "Toji Fushiguro", series: "Jujutsu Kaisen", age: 30, height: "185 cm", weight: "90 kg", tier: "Legendary", personality: "badboy" },
  { name: "Megumi Fushiguro", series: "Jujutsu Kaisen", age: 15, height: "175 cm", weight: "60 kg", tier: "Epic", personality: "dandere" },
  { name: "Yuji Itadori", series: "Jujutsu Kaisen", age: 16, height: "173 cm", weight: "80 kg", tier: "Common", personality: "genki" },
  { name: "Yuta Okkotsu", series: "Jujutsu Kaisen", age: 17, height: "178 cm", weight: "75 kg", tier: "Epic", personality: "dandere" },
  { name: "Nanami Kento", series: "Jujutsu Kaisen", age: 28, height: "184 cm", weight: "78 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Suguru Geto", series: "Jujutsu Kaisen", age: 28, height: "186 cm", weight: "80 kg", tier: "Legendary", personality: "badboy" },
  { name: "Todo Aoi", series: "Jujutsu Kaisen", age: 18, height: "196 cm", weight: "105 kg", tier: "Rare", personality: "genki" },
  { name: "Choso", series: "Jujutsu Kaisen", age: 150, height: "182 cm", weight: "78 kg", tier: "Rare", personality: "oji-san" },
  { name: "Mahito", series: "Jujutsu Kaisen", age: 19, height: "176 cm", weight: "70 kg", tier: "Rare", personality: "playboy" },
  { name: "Panda", series: "Jujutsu Kaisen", age: 16, height: "180 cm", weight: "120 kg", tier: "Common", personality: "genki" },
];
```

`data/husbu/kimetsu.js`:
```js
export const husbus = [
  { name: "Tanjiro Kamado", series: "Kimetsu no Yaiba", age: 15, height: "165 cm", weight: "61 kg", tier: "Common", personality: "genki" },
  { name: "Zenitsu Agatsuma", series: "Kimetsu no Yaiba", age: 16, height: "164 cm", weight: "56 kg", tier: "Common", personality: "dandere" },
  { name: "Kyojuro Rengoku", series: "Kimetsu no Yaiba", age: 20, height: "177 cm", weight: "72 kg", tier: "Legendary", personality: "genki" },
  { name: "Tengen Uzui", series: "Kimetsu no Yaiba", age: 23, height: "198 cm", weight: "95 kg", tier: "Epic", personality: "playboy" },
  { name: "Giyuu Tomioka", series: "Kimetsu no Yaiba", age: 21, height: "176 cm", weight: "69 kg", tier: "Epic", personality: "kuudere" },
  { name: "Sanemi Shinazugawa", series: "Kimetsu no Yaiba", age: 21, height: "179 cm", weight: "75 kg", tier: "Epic", personality: "badboy" },
  { name: "Muichiro Tokito", series: "Kimetsu no Yaiba", age: 14, height: "167 cm", weight: "56 kg", tier: "Rare", personality: "kuudere" },
  { name: "Obanai Iguro", series: "Kimetsu no Yaiba", age: 21, height: "175 cm", weight: "60 kg", tier: "Rare", personality: "dandere" },
  { name: "Genya Shinazugawa", series: "Kimetsu no Yaiba", age: 16, height: "180 cm", weight: "80 kg", tier: "Rare", personality: "tsundere" },
  { name: "Gyomei Himejima", series: "Kimetsu no Yaiba", age: 27, height: "220 cm", weight: "130 kg", tier: "Epic", personality: "oji-san" },
  { name: "Akaza", series: "Kimetsu no Yaiba", age: 100, height: "173 cm", weight: "74 kg", tier: "Rare", personality: "badboy" },
  { name: "Kokushibo", series: "Kimetsu no Yaiba", age: 300, height: "190 cm", weight: "90 kg", tier: "Legendary", personality: "kuudere" },
];
```

`data/husbu/my-hero-academia.js`:
```js
export const husbus = [
  { name: "Izuku Midoriya", series: "My Hero Academia", age: 16, height: "166 cm", weight: "55 kg", tier: "Common", personality: "dandere" },
  { name: "Katsuki Bakugo", series: "My Hero Academia", age: 16, height: "172 cm", weight: "62 kg", tier: "Epic", personality: "tsundere" },
  { name: "Shoto Todoroki", series: "My Hero Academia", age: 16, height: "176 cm", weight: "60 kg", tier: "Epic", personality: "tsundere" },
  { name: "Tenya Iida", series: "My Hero Academia", age: 16, height: "179 cm", weight: "65 kg", tier: "Rare", personality: "sunao" },
  { name: "Eijiro Kirishima", series: "My Hero Academia", age: 16, height: "170 cm", weight: "63 kg", tier: "Rare", personality: "genki" },
  { name: "Shota Aizawa", series: "My Hero Academia", age: 30, height: "183 cm", weight: "70 kg", tier: "Legendary", personality: "oji-san" },
  { name: "All Might", series: "My Hero Academia", age: 55, height: "220 cm", weight: "130 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Tomura Shigaraki", series: "My Hero Academia", age: 21, height: "175 cm", weight: "55 kg", tier: "Epic", personality: "badboy" },
  { name: "Dabi", series: "My Hero Academia", age: 24, height: "176 cm", weight: "60 kg", tier: "Legendary", personality: "badboy" },
  { name: "Keigo Takami", series: "My Hero Academia", age: 22, height: "172 cm", weight: "63 kg", tier: "Epic", personality: "playboy" },
  { name: "Enji Todoroki", series: "My Hero Academia", age: 46, height: "195 cm", weight: "110 kg", tier: "Legendary", personality: "badboy" },
  { name: "Hitoshi Shinso", series: "My Hero Academia", age: 16, height: "173 cm", weight: "58 kg", tier: "Common", personality: "tsundere" },
];
```

`data/husbu/attack-on-titan.js`:
```js
export const husbus = [
  { name: "Levi Ackerman", series: "Attack on Titan", age: 30, height: "160 cm", weight: "65 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Eren Yeager", series: "Attack on Titan", age: 19, height: "183 cm", weight: "72 kg", tier: "Legendary", personality: "badboy" },
  { name: "Armin Arlert", series: "Attack on Titan", age: 19, height: "168 cm", weight: "60 kg", tier: "Epic", personality: "dandere" },
  { name: "Erwin Smith", series: "Attack on Titan", age: 34, height: "188 cm", weight: "84 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Jean Kirstein", series: "Attack on Titan", age: 19, height: "175 cm", weight: "65 kg", tier: "Rare", personality: "tsundere" },
  { name: "Connie Springer", series: "Attack on Titan", age: 19, height: "170 cm", weight: "66 kg", tier: "Common", personality: "genki" },
  { name: "Reiner Braun", series: "Attack on Titan", age: 19, height: "185 cm", weight: "100 kg", tier: "Epic", personality: "sunao" },
  { name: "Zeke Yeager", series: "Attack on Titan", age: 28, height: "183 cm", weight: "78 kg", tier: "Legendary", personality: "playboy" },
  { name: "Hannes", series: "Attack on Titan", age: 32, height: "175 cm", weight: "80 kg", tier: "Rare", personality: "oji-san" },
  { name: "Marco Bott", series: "Attack on Titan", age: 19, height: "179 cm", weight: "68 kg", tier: "Rare", personality: "sunao" },
];
```

`data/husbu/dragon-ball.js`:
```js
export const husbus = [
  { name: "Goku", series: "Dragon Ball", age: 41, height: "175 cm", weight: "62 kg", tier: "Legendary", personality: "genki" },
  { name: "Vegeta", series: "Dragon Ball", age: 47, height: "164 cm", weight: "72 kg", tier: "Legendary", personality: "tsundere" },
  { name: "Piccolo", series: "Dragon Ball", age: 58, height: "226 cm", weight: "116 kg", tier: "Epic", personality: "kuudere" },
  { name: "Gohan", series: "Dragon Ball", age: 27, height: "176 cm", weight: "65 kg", tier: "Rare", personality: "sunao" },
  { name: "Future Trunks", series: "Dragon Ball", age: 29, height: "178 cm", weight: "68 kg", tier: "Epic", personality: "sunao" },
  { name: "Yamcha", series: "Dragon Ball", age: 42, height: "183 cm", weight: "68 kg", tier: "Common", personality: "playboy" },
  { name: "Krillin", series: "Dragon Ball", age: 45, height: "153 cm", weight: "45 kg", tier: "Common", personality: "genki" },
  { name: "Beerus", series: "Dragon Ball", age: 1000, height: "189 cm", weight: "90 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Whis", series: "Dragon Ball", age: 1000, height: "190 cm", weight: "85 kg", tier: "Mythic", personality: "kuudere" },
  { name: "Future Gohan", series: "Dragon Ball", age: 23, height: "176 cm", weight: "60 kg", tier: "Rare", personality: "badboy" },
];
```

- [ ] **Step 4: Jalankan test, verifikasi PASS (kecuali >=300 yang gagal wajar)**

Run: `node --test tests/husbu-data.test.mjs`
Expected: PASS semua kecuali `loads at least 300 entries` (pool = 91 saat ini) — ini disengaja, assertion besar dituntaskan di Task 5.

- [ ] **Step 5: Commit**

```bash
git add data/husbu tests/husbu-data.test.mjs
git commit -m "feat(husbu): data pool loader + 8 franchise batch 1"
```

---

### Task 3: Pool husbu batch 2 (~90 karakter, + femboy Astolfo/Felix/Venti)

**Files:**
- Create: `data/husbu/hunter-x-hunter.js`, `data/husbu/black-clover.js`, `data/husbu/haikyuu.js`, `data/husbu/kuroko.js`, `data/husbu/blue-lock.js`, `data/husbu/fairy-tail.js`, `data/husbu/sword-art-online.js`, `data/husbu/rezero.js`, `data/husbu/fate.js`, `data/husbu/genshin-impact.js`

**Interfaces:**
- Consumes: `data/husbu/index.js` dari Task 2 (FRANCHISES array). **Wajib**: tambahkan setiap file baru ke array `FRANCHISES` di `data/husbu/index.js`.
- Produces: file franchise `export const husbus = [...]`.

- [ ] **Step 1: Tulis data + registrasi**

`data/husbu/hunter-x-hunter.js`:
```js
export const husbus = [
  { name: "Gon Freecss", series: "Hunter x Hunter", age: 14, height: "154 cm", weight: "42 kg", tier: "Common", personality: "genki" },
  { name: "Killua Zoldyck", series: "Hunter x Hunter", age: 14, height: "158 cm", weight: "45 kg", tier: "Rare", personality: "kuudere" },
  { name: "Kurapika", series: "Hunter x Hunter", age: 18, height: "171 cm", weight: "59 kg", tier: "Rare", personality: "sunao" },
  { name: "Leorio", series: "Hunter x Hunter", age: 19, height: "193 cm", weight: "85 kg", tier: "Rare", personality: "oji-san" },
  { name: "Hisoka Morow", series: "Hunter x Hunter", age: 28, height: "190 cm", weight: "79 kg", tier: "Legendary", personality: "playboy" },
  { name: "Chrollo Lucilfer", series: "Hunter x Hunter", age: 26, height: "177 cm", weight: "68 kg", tier: "Legendary", personality: "prince" },
  { name: "Illumi Zoldyck", series: "Hunter x Hunter", age: 25, height: "185 cm", weight: "72 kg", tier: "Epic", personality: "badboy" },
  { name: "Feitan", series: "Hunter x Hunter", age: 23, height: "155 cm", weight: "43 kg", tier: "Epic", personality: "badboy" },
  { name: "Meruem", series: "Hunter x Hunter", age: 21, height: "183 cm", weight: "75 kg", tier: "Mythic", personality: "prince" },
  { name: "Zeno Zoldyck", series: "Hunter x Hunter", age: 68, height: "165 cm", weight: "58 kg", tier: "Epic", personality: "oji-san" },
];
```

`data/husbu/black-clover.js`:
```js
export const husbus = [
  { name: "Asta", series: "Black Clover", age: 17, height: "155 cm", weight: "55 kg", tier: "Epic", personality: "genki" },
  { name: "Yuno", series: "Black Clover", age: 17, height: "172 cm", weight: "58 kg", tier: "Epic", personality: "kuudere" },
  { name: "Yami Sukehiro", series: "Black Clover", age: 31, height: "183 cm", weight: "78 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Finral Roulacase", series: "Black Clover", age: 21, height: "170 cm", weight: "65 kg", tier: "Common", personality: "playboy" },
  { name: "Luck Voltia", series: "Black Clover", age: 17, height: "165 cm", weight: "55 kg", tier: "Rare", personality: "genki" },
  { name: "Zora Ideale", series: "Black Clover", age: 24, height: "180 cm", weight: "76 kg", tier: "Rare", personality: "playboy" },
  { name: "Julius Novachrono", series: "Black Clover", age: 39, height: "182 cm", weight: "74 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Nozel Silva", series: "Black Clover", age: 25, height: "180 cm", weight: "71 kg", tier: "Epic", personality: "prince" },
  { name: "Vetto", series: "Black Clover", age: 37, height: "195 cm", weight: "88 kg", tier: "Rare", personality: "badboy" },
  { name: "Liebe", series: "Black Clover", age: 20, height: "183 cm", weight: "75 kg", tier: "Epic", personality: "badboy" },
];
```

`data/husbu/haikyuu.js`:
```js
export const husbus = [
  { name: "Kageyama Tobio", series: "Haikyuu!!", age: 16, height: "180 cm", weight: "66 kg", tier: "Epic", personality: "kuudere" },
  { name: "Hinata Shoyo", series: "Haikyuu!!", age: 16, height: "162 cm", weight: "52 kg", tier: "Epic", personality: "genki" },
  { name: "Ushijima Wakatoshi", series: "Haikyuu!!", age: 18, height: "190 cm", weight: "80 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Oikawa Tooru", series: "Haikyuu!!", age: 18, height: "184 cm", weight: "70 kg", tier: "Epic", personality: "playboy" },
  { name: "Bokuto Koutaro", series: "Haikyuu!!", age: 18, height: "185 cm", weight: "80 kg", tier: "Epic", personality: "genki" },
  { name: "Kuroo Tetsuro", series: "Haikyuu!!", age: 18, height: "188 cm", weight: "75 kg", tier: "Epic", personality: "playboy" },
  { name: "Nishinoya Yuu", series: "Haikyuu!!", age: 17, height: "160 cm", weight: "50 kg", tier: "Rare", personality: "genki" },
  { name: "Iwaizumi Hajime", series: "Haikyuu!!", age: 18, height: "180 cm", weight: "70 kg", tier: "Rare", personality: "tsundere" },
  { name: "Tendou Satori", series: "Haikyuu!!", age: 18, height: "188 cm", weight: "80 kg", tier: "Rare", personality: "playboy" },
  { name: "Tsukishima Kei", series: "Haikyuu!!", age: 16, height: "188 cm", weight: "68 kg", tier: "Rare", personality: "tsundere" },
];
```

`data/husbu/kuroko.js`:
```js
export const husbus = [
  { name: "Kuroko Tetsuya", series: "Kuroko no Basket", age: 16, height: "168 cm", weight: "57 kg", tier: "Epic", personality: "dandere" },
  { name: "Kagami Taiga", series: "Kuroko no Basket", age: 16, height: "190 cm", weight: "82 kg", tier: "Epic", personality: "genki" },
  { name: "Kise Ryota", series: "Kuroko no Basket", age: 17, height: "189 cm", weight: "77 kg", tier: "Legendary", personality: "playboy" },
  { name: "Midorima Shintaro", series: "Kuroko no Basket", age: 17, height: "195 cm", weight: "83 kg", tier: "Epic", personality: "kuudere" },
  { name: "Aomine Daiki", series: "Kuroko no Basket", age: 17, height: "192 cm", weight: "85 kg", tier: "Legendary", personality: "badboy" },
  { name: "Murasakibara Atsushi", series: "Kuroko no Basket", age: 17, height: "208 cm", weight: "99 kg", tier: "Epic", personality: "kuudere" },
  { name: "Akashi Seijuro", series: "Kuroko no Basket", age: 17, height: "173 cm", weight: "58 kg", tier: "Mythic", personality: "prince" },
  { name: "Hyuga Junpei", series: "Kuroko no Basket", age: 18, height: "178 cm", weight: "68 kg", tier: "Common", personality: "sunao" },
  { name: "Teppei Kiyoshi", series: "Kuroko no Basket", age: 19, height: "193 cm", weight: "88 kg", tier: "Rare", personality: "oji-san" },
  { name: "Takao Kazunari", series: "Kuroko no Basket", age: 17, height: "176 cm", weight: "65 kg", tier: "Common", personality: "genki" },
];
```

`data/husbu/blue-lock.js`:
```js
export const husbus = [
  { name: "Yoichi Isagi", series: "Blue Lock", age: 17, height: "175 cm", weight: "64 kg", tier: "Epic", personality: "sunao" },
  { name: "Rin Itoshi", series: "Blue Lock", age: 17, height: "177 cm", weight: "67 kg", tier: "Legendary", personality: "tsundere" },
  { name: "Sae Itoshi", series: "Blue Lock", age: 19, height: "178 cm", weight: "68 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Meguru Bachira", series: "Blue Lock", age: 17, height: "176 cm", weight: "62 kg", tier: "Rare", personality: "genki" },
  { name: "Hyoma Chigiri", series: "Blue Lock", age: 17, height: "176 cm", weight: "63 kg", tier: "Rare", personality: "dandere" },
  { name: "Nagi Seishiro", series: "Blue Lock", age: 17, height: "190 cm", weight: "76 kg", tier: "Epic", personality: "dandere" },
  { name: "Shouei Barou", series: "Blue Lock", age: 18, height: "187 cm", weight: "79 kg", tier: "Epic", personality: "badboy" },
  { name: "Reo Mikage", series: "Blue Lock", age: 17, height: "185 cm", weight: "75 kg", tier: "Rare", personality: "prince" },
  { name: "Oliver Aiku", series: "Blue Lock", age: 20, height: "187 cm", weight: "80 kg", tier: "Epic", personality: "badboy" },
  { name: "Tabito Karasu", series: "Blue Lock", age: 18, height: "176 cm", weight: "67 kg", tier: "Rare", personality: "kuudere" },
];
```

`data/husbu/fairy-tail.js`:
```js
export const husbus = [
  { name: "Natsu Dragneel", series: "Fairy Tail", age: 18, height: "175 cm", weight: "72 kg", tier: "Epic", personality: "genki" },
  { name: "Gray Fullbuster", series: "Fairy Tail", age: 18, height: "175 cm", weight: "70 kg", tier: "Epic", personality: "tsundere" },
  { name: "Jellal Fernandes", series: "Fairy Tail", age: 19, height: "178 cm", weight: "69 kg", tier: "Rare", personality: "badboy" },
  { name: "Laxus Dreyar", series: "Fairy Tail", age: 24, height: "193 cm", weight: "86 kg", tier: "Legendary", personality: "tsundere" },
  { name: "Gajeel Redfox", series: "Fairy Tail", age: 20, height: "182 cm", weight: "75 kg", tier: "Rare", personality: "badboy" },
  { name: "Gildarts Clive", series: "Fairy Tail", age: 45, height: "190 cm", weight: "88 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Mystogan", series: "Fairy Tail", age: 18, height: "176 cm", weight: "65 kg", tier: "Rare", personality: "kuudere" },
  { name: "Lyon Vastia", series: "Fairy Tail", age: 20, height: "178 cm", weight: "70 kg", tier: "Common", personality: "tsundere" },
  { name: "Bickslow", series: "Fairy Tail", age: 22, height: "175 cm", weight: "65 kg", tier: "Common", personality: "playboy" },
  { name: "Sting Eucliffe", series: "Fairy Tail", age: 18, height: "182 cm", weight: "76 kg", tier: "Epic", personality: "prince" },
];
```

`data/husbu/sword-art-online.js`:
```js
export const husbus = [
  { name: "Kirito", series: "Sword Art Online", age: 16, height: "172 cm", weight: "59 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Klein", series: "Sword Art Online", age: 25, height: "180 cm", weight: "75 kg", tier: "Common", personality: "genki" },
  { name: "Agil", series: "Sword Art Online", age: 27, height: "192 cm", weight: "98 kg", tier: "Rare", personality: "oji-san" },
  { name: "Eugeo", series: "Sword Art Online", age: 18, height: "178 cm", weight: "67 kg", tier: "Epic", personality: "sunao" },
  { name: "Kayaba Akihiko", series: "Sword Art Online", age: 30, height: "182 cm", weight: "78 kg", tier: "Mythic", personality: "prince" },
  { name: "PoH", series: "Sword Art Online", age: 24, height: "180 cm", weight: "72 kg", tier: "Legendary", personality: "badboy" },
  { name: "Sigurd", series: "Sword Art Online", age: 18, height: "175 cm", weight: "63 kg", tier: "Common", personality: "sunao" },
  { name: "XaXa", series: "Sword Art Online", age: 22, height: "178 cm", weight: "68 kg", tier: "Common", personality: "badboy" },
];
```

`data/husbu/rezero.js`:
```js
export const husbus = [
  { name: "Subaru Natsuki", series: "Re:Zero", age: 18, height: "173 cm", weight: "65 kg", tier: "Epic", personality: "genki" },
  { name: "Reinhard van Astrea", series: "Re:Zero", age: 18, height: "182 cm", weight: "78 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Julius Juukulius", series: "Re:Zero", age: 22, height: "183 cm", weight: "75 kg", tier: "Epic", personality: "prince" },
  { name: "Otto Suwen", series: "Re:Zero", age: 20, height: "170 cm", weight: "63 kg", tier: "Common", personality: "dandere" },
  { name: "Garfiel Tinsel", series: "Re:Zero", age: 17, height: "172 cm", weight: "77 kg", tier: "Rare", personality: "genki" },
  { name: "Wilhelm van Astrea", series: "Re:Zero", age: 65, height: "181 cm", weight: "77 kg", tier: "Epic", personality: "oji-san" },
  { name: "Roswaal L Mathers", series: "Re:Zero", age: 40, height: "186 cm", weight: "78 kg", tier: "Legendary", personality: "playboy" },
  { name: "Felix Argyle", series: "Re:Zero", age: 19, height: "161 cm", weight: "48 kg", tier: "Rare", personality: "femboy" },
  { name: "Regulus Corneas", series: "Re:Zero", age: 40, height: "175 cm", weight: "65 kg", tier: "Legendary", personality: "badboy" },
  { name: "Aldebaran", series: "Re:Zero", age: 25, height: "180 cm", weight: "75 kg", tier: "Rare", personality: "kuudere" },
];
```

`data/husbu/fate.js`:
```js
export const husbus = [
  { name: "Gilgamesh", series: "Fate", age: 3000, height: "182 cm", weight: "68 kg", tier: "Mythic", personality: "prince" },
  { name: "Archer (EMIYA)", series: "Fate", age: 25, height: "187 cm", weight: "78 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Lancer (Cu Chulainn)", series: "Fate", age: 28, height: "185 cm", weight: "70 kg", tier: "Epic", personality: "genki" },
  { name: "Kirei Kotomine", series: "Fate", age: 35, height: "185 cm", weight: "82 kg", tier: "Legendary", personality: "badboy" },
  { name: "Shirou Emiya", series: "Fate", age: 17, height: "167 cm", weight: "58 kg", tier: "Common", personality: "sunao" },
  { name: "Iskandar", series: "Fate", age: 30, height: "212 cm", weight: "130 kg", tier: "Epic", personality: "oji-san" },
  { name: "Astolfo", series: "Fate", age: 19, height: "164 cm", weight: "56 kg", tier: "Legendary", personality: "femboy" },
  { name: "Karna", series: "Fate", age: 25, height: "178 cm", weight: "65 kg", tier: "Legendary", personality: "prince" },
  { name: "Arthur Pendragon", series: "Fate", age: 35, height: "188 cm", weight: "82 kg", tier: "Epic", personality: "prince" },
  { name: "Emiya Kiritsugu", series: "Fate", age: 34, height: "175 cm", weight: "67 kg", tier: "Epic", personality: "oji-san" },
  { name: "Cu Chulainn (Proto)", series: "Fate", age: 20, height: "180 cm", weight: "70 kg", tier: "Rare", personality: "genki" },
  { name: "Ozymandias", series: "Fate", age: 3500, height: "178 cm", weight: "72 kg", tier: "Legendary", personality: "prince" },
];
```

`data/husbu/genshin-impact.js`:
```js
export const husbus = [
  { name: "Zhongli", series: "Genshin Impact", age: 6000, height: "190 cm", weight: "80 kg", tier: "Mythic", personality: "oji-san" },
  { name: "Kamisato Ayato", series: "Genshin Impact", age: 24, height: "188 cm", weight: "75 kg", tier: "Epic", personality: "prince" },
  { name: "Diluc Ragnvindr", series: "Genshin Impact", age: 22, height: "185 cm", weight: "72 kg", tier: "Epic", personality: "kuudere" },
  { name: "Kaeya Alberich", series: "Genshin Impact", age: 22, height: "186 cm", weight: "75 kg", tier: "Legendary", personality: "playboy" },
  { name: "Alhaitham", series: "Genshin Impact", age: 25, height: "188 cm", weight: "76 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Neuvillette", series: "Genshin Impact", age: 500, height: "188 cm", weight: "75 kg", tier: "Legendary", personality: "prince" },
  { name: "Xiao", series: "Genshin Impact", age: 2000, height: "158 cm", weight: "48 kg", tier: "Epic", personality: "kuudere" },
  { name: "Venti", series: "Genshin Impact", age: 2600, height: "164 cm", weight: "50 kg", tier: "Epic", personality: "femboy" },
  { name: "Kazuha", series: "Genshin Impact", age: 21, height: "168 cm", weight: "58 kg", tier: "Rare", personality: "dandere" },
  { name: "Wriothesley", series: "Genshin Impact", age: 28, height: "195 cm", weight: "95 kg", tier: "Legendary", personality: "badboy" },
  { name: "Heizou", series: "Genshin Impact", age: 21, height: "170 cm", weight: "62 kg", tier: "Rare", personality: "genki" },
  { name: "Baizhu", series: "Genshin Impact", age: 26, height: "181 cm", weight: "70 kg", tier: "Rare", personality: "oji-san" },
];
```

Tambahkan ke `data/husbu/index.js` (edit import block + FRANCHISES):
```js
import { husbus as hunterXHunter } from "./hunter-x-hunter.js";
import { husbus as blackClover } from "./black-clover.js";
import { husbus as haikyuu } from "./haikyuu.js";
import { husbus as kuroko } from "./kuroko.js";
import { husbus as blueLock } from "./blue-lock.js";
import { husbus as fairyTail } from "./fairy-tail.js";
import { husbus as sao } from "./sword-art-online.js";
import { husbus as rezero } from "./rezero.js";
import { husbus as fate } from "./fate.js";
import { husbus as genshin } from "./genshin-impact.js";
// ... dalam FRANCHISES:
  hunterXHunter, blackClover, haikyuu, kuroko, blueLock, fairyTail, sao, rezero, fate, genshin,
```

- [ ] **Step 2: Jalankan test**

Run: `node --test tests/husbu-data.test.mjs`
Expected: PASS (kecuali `>= 300`; pool sekarang = 91 + 90 = 181)

- [ ] **Step 3: Commit**

```bash
git add data/husbu
git commit -m "feat(husbu): 10 franchise batch 2 (incl femboy Astolfo/Felix/Venti)"
```

---

### Task 4: Pool husbu batch 3 (~73 karakter)

**Files:**
- Create: `data/husbu/honkai-star-rail.js`, `data/husbu/bungo-stray-dogs.js`, `data/husbu/spy-x-family.js`, `data/husbu/chainsaw-man.js`, `data/husbu/tokyo-revengers.js`, `data/husbu/one-punch-man.js`, `data/husbu/death-note.js`, `data/husbu/code-geass.js`

**Interfaces:**
- Consumes: `data/husbu/index.js` — tambahkan tiap file ke FRANCHISES.
- Produces: file franchise.

- [ ] **Step 1: Tulis data + registrasi**

`data/husbu/honkai-star-rail.js`:
```js
export const husbus = [
  { name: "Jing Yuan", series: "Honkai Star Rail", age: 700, height: "185 cm", weight: "75 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Dan Heng", series: "Honkai Star Rail", age: 25, height: "180 cm", weight: "70 kg", tier: "Epic", personality: "dandere" },
  { name: "Aventurine", series: "Honkai Star Rail", age: 22, height: "183 cm", weight: "73 kg", tier: "Legendary", personality: "playboy" },
  { name: "Blade", series: "Honkai Star Rail", age: 30, height: "185 cm", weight: "78 kg", tier: "Legendary", personality: "badboy" },
  { name: "Luocha", series: "Honkai Star Rail", age: 28, height: "182 cm", weight: "75 kg", tier: "Epic", personality: "playboy" },
  { name: "Welt Yang", series: "Honkai Star Rail", age: 45, height: "180 cm", weight: "78 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Gepard Landau", series: "Honkai Star Rail", age: 26, height: "188 cm", weight: "85 kg", tier: "Epic", personality: "prince" },
  { name: "Argenti", series: "Honkai Star Rail", age: 30, height: "188 cm", weight: "80 kg", tier: "Epic", personality: "prince" },
  { name: "Sunday", series: "Honkai Star Rail", age: 25, height: "180 cm", weight: "75 kg", tier: "Legendary", personality: "prince" },
  { name: "Dr. Ratio", series: "Honkai Star Rail", age: 28, height: "186 cm", weight: "78 kg", tier: "Epic", personality: "kuudere" },
];
```

`data/husbu/bungo-stray-dogs.js`:
```js
export const husbus = [
  { name: "Dazai Osamu", series: "Bungo Stray Dogs", age: 22, height: "181 cm", weight: "67 kg", tier: "Legendary", personality: "playboy" },
  { name: "Atsushi Nakajima", series: "Bungo Stray Dogs", age: 18, height: "170 cm", weight: "58 kg", tier: "Rare", personality: "sunao" },
  { name: "Chuuya Nakahara", series: "Bungo Stray Dogs", age: 22, height: "160 cm", weight: "58 kg", tier: "Epic", personality: "tsundere" },
  { name: "Ryunosuke Akutagawa", series: "Bungo Stray Dogs", age: 20, height: "174 cm", weight: "63 kg", tier: "Epic", personality: "badboy" },
  { name: "Yukichi Fukuzawa", series: "Bungo Stray Dogs", age: 45, height: "186 cm", weight: "80 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Oda Sakunosuke", series: "Bungo Stray Dogs", age: 28, height: "178 cm", weight: "70 kg", tier: "Rare", personality: "sunao" },
  { name: "Ranpo Edogawa", series: "Bungo Stray Dogs", age: 26, height: "168 cm", weight: "58 kg", tier: "Epic", personality: "playboy" },
  { name: "Shibusawa", series: "Bungo Stray Dogs", age: 22, height: "185 cm", weight: "75 kg", tier: "Rare", personality: "badboy" },
  { name: "Mori Ogai", series: "Bungo Stray Dogs", age: 43, height: "180 cm", weight: "75 kg", tier: "Rare", personality: "oji-san" },
  { name: "Doppo Kunikida", series: "Bungo Stray Dogs", age: 26, height: "188 cm", weight: "78 kg", tier: "Rare", personality: "sunao" },
];
```

`data/husbu/spy-x-family.js`:
```js
export const husbus = [
  { name: "Loid Forger", series: "Spy x Family", age: 28, height: "187 cm", weight: "75 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Yuri Briar", series: "Spy x Family", age: 21, height: "180 cm", weight: "70 kg", tier: "Rare", personality: "tsundere" },
  { name: "Franky Franklin", series: "Spy x Family", age: 30, height: "175 cm", weight: "70 kg", tier: "Common", personality: "genki" },
  { name: "Henry Henderson", series: "Spy x Family", age: 58, height: "180 cm", weight: "75 kg", tier: "Common", personality: "oji-san" },
  { name: "Daybreak", series: "Spy x Family", age: 27, height: "180 cm", weight: "72 kg", tier: "Common", personality: "playboy" },
];
```

`data/husbu/chainsaw-man.js`:
```js
export const husbus = [
  { name: "Denji", series: "Chainsaw Man", age: 17, height: "173 cm", weight: "65 kg", tier: "Epic", personality: "genki" },
  { name: "Aki Hayakawa", series: "Chainsaw Man", age: 18, height: "179 cm", weight: "68 kg", tier: "Epic", personality: "kuudere" },
  { name: "Kishibe", series: "Chainsaw Man", age: 40, height: "195 cm", weight: "90 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Yoshida", series: "Chainsaw Man", age: 18, height: "178 cm", weight: "65 kg", tier: "Rare", personality: "playboy" },
  { name: "Angel Devil", series: "Chainsaw Man", age: 20, height: "176 cm", weight: "60 kg", tier: "Rare", personality: "dandere" },
  { name: "Beam", series: "Chainsaw Man", age: 19, height: "180 cm", weight: "75 kg", tier: "Rare", personality: "genki" },
  { name: "Katana Man", series: "Chainsaw Man", age: 23, height: "180 cm", weight: "78 kg", tier: "Rare", personality: "badboy" },
  { name: "Gun Devil", series: "Chainsaw Man", age: 22, height: "185 cm", weight: "85 kg", tier: "Epic", personality: "badboy" },
];
```

`data/husbu/tokyo-revengers.js`:
```js
export const husbus = [
  { name: "Manjiro Sano", series: "Tokyo Revengers", age: 18, height: "162 cm", weight: "52 kg", tier: "Mythic", personality: "badboy" },
  { name: "Ken Ryuguji", series: "Tokyo Revengers", age: 21, height: "185 cm", weight: "78 kg", tier: "Epic", personality: "oji-san" },
  { name: "Takemichi Hanagaki", series: "Tokyo Revengers", age: 26, height: "165 cm", weight: "56 kg", tier: "Rare", personality: "dandere" },
  { name: "Chifuyu Matsuno", series: "Tokyo Revengers", age: 16, height: "167 cm", weight: "58 kg", tier: "Rare", personality: "sunao" },
  { name: "Keisuke Baji", series: "Tokyo Revengers", age: 18, height: "175 cm", weight: "72 kg", tier: "Legendary", personality: "badboy" },
  { name: "Shuji Hanma", series: "Tokyo Revengers", age: 22, height: "188 cm", weight: "77 kg", tier: "Epic", personality: "badboy" },
  { name: "Kazutora Hanemiya", series: "Tokyo Revengers", age: 18, height: "178 cm", weight: "72 kg", tier: "Epic", personality: "yandere" },
  { name: "Hajime Kokonoi", series: "Tokyo Revengers", age: 22, height: "172 cm", weight: "60 kg", tier: "Rare", personality: "kuudere" },
];
```

`data/husbu/one-punch-man.js`:
```js
export const husbus = [
  { name: "Saitama", series: "One Punch Man", age: 25, height: "175 cm", weight: "70 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Genos", series: "One Punch Man", age: 19, height: "178 cm", weight: "78 kg", tier: "Epic", personality: "sunao" },
  { name: "Garou", series: "One Punch Man", age: 21, height: "177 cm", weight: "71 kg", tier: "Legendary", personality: "badboy" },
  { name: "Sonic", series: "One Punch Man", age: 20, height: "175 cm", weight: "68 kg", tier: "Epic", personality: "tsundere" },
  { name: "King", series: "One Punch Man", age: 31, height: "187 cm", weight: "88 kg", tier: "Epic", personality: "dandere" },
  { name: "Mumen Rider", series: "One Punch Man", age: 24, height: "174 cm", weight: "72 kg", tier: "Rare", personality: "sunao" },
  { name: "Bang", series: "One Punch Man", age: 70, height: "166 cm", weight: "63 kg", tier: "Epic", personality: "oji-san" },
  { name: "Flashy Flash", series: "One Punch Man", age: 25, height: "180 cm", weight: "65 kg", tier: "Rare", personality: "kuudere" },
];
```

`data/husbu/death-note.js`:
```js
export const husbus = [
  { name: "Light Yagami", series: "Death Note", age: 18, height: "179 cm", weight: "54 kg", tier: "Mythic", personality: "prince" },
  { name: "L Lawliet", series: "Death Note", age: 25, height: "179 cm", weight: "50 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Near", series: "Death Note", age: 20, height: "159 cm", weight: "45 kg", tier: "Epic", personality: "dandere" },
  { name: "Mello", series: "Death Note", age: 20, height: "174 cm", weight: "62 kg", tier: "Epic", personality: "badboy" },
  { name: "Teru Mikami", series: "Death Note", age: 28, height: "175 cm", weight: "65 kg", tier: "Rare", personality: "oji-san" },
  { name: "Matsuda Todo", series: "Death Note", age: 31, height: "175 cm", weight: "65 kg", tier: "Common", personality: "genki" },
];
```

`data/husbu/code-geass.js`:
```js
export const husbus = [
  { name: "Lelouch Lamperouge", series: "Code Geass", age: 18, height: "178 cm", weight: "54 kg", tier: "Mythic", personality: "badboy" },
  { name: "Suzaku Kururugi", series: "Code Geass", age: 18, height: "176 cm", weight: "65 kg", tier: "Legendary", personality: "sunao" },
  { name: "Rolo Lamperouge", series: "Code Geass", age: 19, height: "175 cm", weight: "58 kg", tier: "Epic", personality: "yandere" },
  { name: "Jeremiah Gottwald", series: "Code Geass", age: 33, height: "188 cm", weight: "88 kg", tier: "Epic", personality: "oji-san" },
  { name: "Lloyd Asplund", series: "Code Geass", age: 30, height: "181 cm", weight: "70 kg", tier: "Rare", personality: "playboy" },
  { name: "Schneizel El Britannia", series: "Code Geass", age: 28, height: "178 cm", weight: "68 kg", tier: "Legendary", personality: "prince" },
];
```

Registrasi di `data/husbu/index.js` (import + FRANCHISES): `honkaiStarRail, bsd, spyFamily, chainsaw, tokyoRevengers, opm, deathNote, codeGeass`.

- [ ] **Step 2: Jalankan test**

Run: `node --test tests/husbu-data.test.mjs`
Expected: PASS (kecuali `>= 300`; pool = 181 + 73 = 254)

- [ ] **Step 3: Commit**

```bash
git add data/husbu
git commit -m "feat(husbu): 8 franchise batch 3"
```

---

### Task 5: Pool husbu batch 4 (~49 karakter) → total ≥300, aktifkan assertion

**Files:**
- Create: `data/husbu/fullmetal-alchemist.js`, `data/husbu/tokyo-ghoul.js`, `data/husbu/seven-deadly-sins.js`, `data/husbu/kaguya.js`, `data/husbu/cote.js`, `data/husbu/assassination-classroom.js`, `data/husbu/oregairu.js`, `data/husbu/baka-to-test.js`
- Modify: `tests/husbu-data.test.mjs` (tidak ada — assertion sudah aktif, akan PASS setelah semua file ada)

**Interfaces:**
- Consumes: `data/husbu/index.js` — registrasi 8 file.
- Produces: file franchise + pool akhir ≥300.

- [ ] **Step 1: Tulis data + registrasi**

`data/husbu/fullmetal-alchemist.js`:
```js
export const husbus = [
  { name: "Edward Elric", series: "Fullmetal Alchemist", age: 16, height: "165 cm", weight: "60 kg", tier: "Epic", personality: "tsundere" },
  { name: "Alphonse Elric", series: "Fullmetal Alchemist", age: 16, height: "220 cm", weight: "120 kg", tier: "Rare", personality: "sunao" },
  { name: "Roy Mustang", series: "Fullmetal Alchemist", age: 29, height: "173 cm", weight: "64 kg", tier: "Legendary", personality: "playboy" },
  { name: "Maes Hughes", series: "Fullmetal Alchemist", age: 27, height: "174 cm", weight: "67 kg", tier: "Epic", personality: "oji-san" },
  { name: "Scar", series: "Fullmetal Alchemist", age: 27, height: "182 cm", weight: "78 kg", tier: "Epic", personality: "badboy" },
  { name: "Greed", series: "Fullmetal Alchemist", age: 40, height: "182 cm", weight: "80 kg", tier: "Epic", personality: "playboy" },
  { name: "King Bradley", series: "Fullmetal Alchemist", age: 50, height: "183 cm", weight: "80 kg", tier: "Legendary", personality: "oji-san" },
  { name: "Van Hohenheim", series: "Fullmetal Alchemist", age: 400, height: "177 cm", weight: "72 kg", tier: "Rare", personality: "oji-san" },
];
```

`data/husbu/tokyo-ghoul.js`:
```js
export const husbus = [
  { name: "Kaneki Ken", series: "Tokyo Ghoul", age: 19, height: "170 cm", weight: "58 kg", tier: "Epic", personality: "dandere" },
  { name: "Shuu Tsukiyama", series: "Tokyo Ghoul", age: 20, height: "185 cm", weight: "75 kg", tier: "Epic", personality: "prince" },
  { name: "Uta", series: "Tokyo Ghoul", age: 28, height: "182 cm", weight: "70 kg", tier: "Epic", personality: "playboy" },
  { name: "Ayato Kirishima", series: "Tokyo Ghoul", age: 18, height: "171 cm", weight: "60 kg", tier: "Rare", personality: "badboy" },
  { name: "Hide Nagachika", series: "Tokyo Ghoul", age: 19, height: "172 cm", weight: "68 kg", tier: "Rare", personality: "genki" },
  { name: "Kishou Arima", series: "Tokyo Ghoul", age: 34, height: "185 cm", weight: "78 kg", tier: "Legendary", personality: "kuudere" },
];
```

`data/husbu/seven-deadly-sins.js`:
```js
export const husbus = [
  { name: "Meliodas", series: "Seven Deadly Sins", age: 3000, height: "152 cm", weight: "50 kg", tier: "Legendary", personality: "playboy" },
  { name: "Ban", series: "Seven Deadly Sins", age: 43, height: "190 cm", weight: "80 kg", tier: "Epic", personality: "playboy" },
  { name: "King", series: "Seven Deadly Sins", age: 1300, height: "162 cm", weight: "50 kg", tier: "Rare", personality: "dandere" },
  { name: "Escanor", series: "Seven Deadly Sins", age: 40, height: "220 cm", weight: "100 kg", tier: "Legendary", personality: "prince" },
  { name: "Gowther", series: "Seven Deadly Sins", age: 3000, height: "175 cm", weight: "60 kg", tier: "Rare", personality: "kuudere" },
  { name: "Estarossa", series: "Seven Deadly Sins", age: 3000, height: "180 cm", weight: "85 kg", tier: "Epic", personality: "badboy" },
  { name: "Arthur Pendragon", series: "Seven Deadly Sins", age: 19, height: "175 cm", weight: "65 kg", tier: "Epic", personality: "sunao" },
  { name: "Dreyfus", series: "Seven Deadly Sins", age: 45, height: "180 cm", weight: "80 kg", tier: "Rare", personality: "oji-san" },
];
```

`data/husbu/kaguya.js`:
```js
export const husbus = [
  { name: "Miyuki Shirogane", series: "Kaguya-sama", age: 17, height: "173 cm", weight: "65 kg", tier: "Epic", personality: "tsundere" },
  { name: "Yu Ishigami", series: "Kaguya-sama", age: 16, height: "168 cm", weight: "58 kg", tier: "Rare", personality: "dandere" },
  { name: "Tsubasa Tanuma", series: "Kaguya-sama", age: 17, height: "170 cm", weight: "60 kg", tier: "Common", personality: "sunao" },
  { name: "Shirogane Papa", series: "Kaguya-sama", age: 48, height: "178 cm", weight: "75 kg", tier: "Rare", personality: "oji-san" },
];
```

`data/husbu/cote.js`:
```js
export const husbus = [
  { name: "Kiyotaka Ayanokoji", series: "Classroom of the Elite", age: 17, height: "172 cm", weight: "62 kg", tier: "Legendary", personality: "kuudere" },
  { name: "Kakeru Ryuen", series: "Classroom of the Elite", age: 18, height: "175 cm", weight: "68 kg", tier: "Epic", personality: "badboy" },
  { name: "Sudo Ken", series: "Classroom of the Elite", age: 18, height: "180 cm", weight: "72 kg", tier: "Rare", personality: "genki" },
  { name: "Manabu Horikita", series: "Classroom of the Elite", age: 18, height: "178 cm", weight: "65 kg", tier: "Rare", personality: "tsundere" },
  { name: "Koenji Rokusuke", series: "Classroom of the Elite", age: 18, height: "182 cm", weight: "70 kg", tier: "Epic", personality: "prince" },
  { name: "Ike Kanji", series: "Classroom of the Elite", age: 17, height: "165 cm", weight: "60 kg", tier: "Common", personality: "genki" },
];
```

`data/husbu/assassination-classroom.js`:
```js
export const husbus = [
  { name: "Nagisa Shiota", series: "Assassination Classroom", age: 15, height: "158 cm", weight: "45 kg", tier: "Epic", personality: "femboy" },
  { name: "Karma Akabane", series: "Assassination Classroom", age: 15, height: "170 cm", weight: "55 kg", tier: "Epic", personality: "badboy" },
  { name: "Koro-sensei", series: "Assassination Classroom", age: 0, height: "200 cm", weight: "80 kg", tier: "Legendary", personality: "playboy" },
  { name: "Sugino Tomohito", series: "Assassination Classroom", age: 15, height: "170 cm", weight: "60 kg", tier: "Common", personality: "genki" },
  { name: "Terasaka Ryoma", series: "Assassination Classroom", age: 15, height: "178 cm", weight: "68 kg", tier: "Rare", personality: "badboy" },
  { name: "Karasuma Tadaomi", series: "Assassination Classroom", age: 37, height: "185 cm", weight: "80 kg", tier: "Epic", personality: "oji-san" },
  { name: "Asano Gakushu", series: "Assassination Classroom", age: 15, height: "172 cm", weight: "56 kg", tier: "Rare", personality: "prince" },
  { name: "Asano Gakuhou", series: "Assassination Classroom", age: 45, height: "180 cm", weight: "76 kg", tier: "Epic", personality: "oji-san" },
];
```

`data/husbu/oregairu.js`:
```js
export const husbus = [
  { name: "Hachiman Hikigaya", series: "Oregairu", age: 17, height: "173 cm", weight: "60 kg", tier: "Epic", personality: "kuudere" },
  { name: "Saika Totsuka", series: "Oregairu", age: 16, height: "160 cm", weight: "48 kg", tier: "Rare", personality: "femboy" },
  { name: "Hayato Hayama", series: "Oregairu", age: 17, height: "178 cm", weight: "68 kg", tier: "Rare", personality: "playboy" },
  { name: "Zaimokuza", series: "Oregairu", age: 17, height: "180 cm", weight: "80 kg", tier: "Common", personality: "prince" },
];
```

`data/husbu/baka-to-test.js`:
```js
export const husbus = [
  { name: "Hideyoshi Kinoshita", series: "Baka to Test", age: 16, height: "165 cm", weight: "50 kg", tier: "Epic", personality: "femboy" },
  { name: "Akihisa Yoshii", series: "Baka to Test", age: 16, height: "172 cm", weight: "60 kg", tier: "Rare", personality: "genki" },
  { name: "Yuuji Sakamoto", series: "Baka to Test", age: 16, height: "175 cm", weight: "65 kg", tier: "Rare", personality: "badboy" },
  { name: "Kouta Tsuchiya", series: "Baka to Test", age: 16, height: "175 cm", weight: "65 kg", tier: "Common", personality: "dandere" },
  { name: "Tsunemitsu Yuusei", series: "Baka to Test", age: 16, height: "178 cm", weight: "70 kg", tier: "Rare", personality: "kuudere" },
];
```

Registrasi di `data/husbu/index.js`: `fma, tokyoGhoul, sds, kaguya, cote, assassClass, oregairu, bakaTest`.

- [ ] **Step 2: Jalankan seluruh test suite**

Run: `node --test tests/`
Expected: PASS — termasuk `loads at least 300 entries` (total pool = 91+90+73+49 = **303**). Test waifu juga tetap hijau.

- [ ] **Step 3: Commit**

```bash
git add data/husbu tests/husbu-data.test.mjs
git commit -m "feat(husbu): final franchise batch - pool 300+"
```

---

### Task 6: Refactor `ourin-waifu.js` — shared imports + aksi/event baru + like/dislike

**Files:**
- Modify: `src/lib/ourin-waifu.js`
- Test: `tests/waifu-lib.test.mjs` (existing, harus tetap hijau)

**Interfaces:**
- Consumes: `src/lib/ourin-romance.js` (Task 1): `DOWRY, TIER_ORDER, TIER_VALUE, TIER_EXPECTED, PITY_THRESHOLD, EVENT_CHANCE, MOOD_MULT, getDailyMood`.
- Produces (ekspor tetap): `rollWaifu, applyAction, rollEvent, getDailyMood, jealousyCheck, albumStats, DOWRY` — signature `applyAction(key, waifu, moodType, rng, multOverride)` **TIDAK berubah**. `applyAction` dan `rollEvent` kini juga mengembalikan field tambahan `{ like, dislike }` (applyAction) dan `{ anger }` (rollEvent).

- [ ] **Step 1: Tulis test baru untuk perilaku tambahan** (append ke `tests/waifu-lib.test.mjs`)

```js
  it("applyAction returns like/dislike flags", () => {
    // jalan_taman likes tsundere → like=true
    const liked = applyAction("jalan_taman", { personality: "tsundere", name: "M" }, "biasa", () => 0);
    assert.equal(liked.like, true);
    // jalan_mall dislikes dandere → dislike=true
    const disliked = applyAction("jalan_mall", { personality: "dandere", name: "M" }, "biasa", () => 0);
    assert.equal(disliked.dislike, true);
    // kafe_matcha netral buat genki → keduanya false
    const neutral = applyAction("kafe_matcha", { personality: "genki", name: "M" }, "biasa", () => 0);
    assert.equal(neutral.like, false);
    assert.equal(neutral.dislike, false);
  });

  it("new actions exist (kuliner/olahraga/alam/seni/intim/married)", () => {
    for (const key of ["restoran_makan", "restoran_dimsum", "restoran_bbq",
      "olahraga_hiking", "olahraga_lari", "olahraga_panjat",
      "alam_camping", "alam_mancing", "alam_perahu",
      "seni_museum", "seni_melukis", "seni_konser",
      "pijat_bahu", "nontonrumah", "jalanpagi"]) {
      const r = applyAction(key, { personality: "deredere", name: "M" }, "biasa", () => 0);
      assert.ok(r, `missing action ${key}`);
    }
  });

  it("rollEvent passes anger through", () => {
    for (let i = 0; i < 200; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "M" }, mulberry32(i));
      if (!e) continue;
      assert.ok(typeof e.anger === "number");
    }
  });
```

- [ ] **Step 2: Jalankan test, verifikasi FAIL**

Run: `node --test tests/waifu-lib.test.mjs`
Expected: FAIL pada 3 test baru (aksi baru tidak ada, `like/dislike` undefined, `anger` undefined). Test lama masih PASS.

- [ ] **Step 3: Refactor implementasi**

Edit header `src/lib/ourin-waifu.js`: ganti deklarasi konstanta lokal dengan import dari romance + re-export untuk kompatibilitas:

```js
// src/lib/ourin-waifu.js
// Logika inti gacha waifu. Pure-ish, tanpa dependensi WA/Baileys.
import { getPool } from "../../data/waifu/index.js";
import { DOWRY, TIER_ORDER, TIER_VALUE, TIER_EXPECTED, PITY_THRESHOLD, EVENT_CHANCE, MOOD_MULT, getDailyMood } from "./ourin-romance.js";

export { DOWRY, PITY_THRESHOLD, EVENT_CHANCE, getDailyMood };
```

Hapus baris deklarasi: `const TIER_ORDER`, `export const PITY_THRESHOLD`, `export const EVENT_CHANCE`, `const MOOD_MULT`, `export const DOWRY`, `const TIER_VALUE`, `const TIER_EXPECTED`, dan blok `export function getDailyMood` (kini dari romance). Jaga `export function applyAction`, `rollEvent`, `albumStats` (import TIER_VALUE/TIER_EXPECTED sudah dari atas), `jealousyCheck` + `WAIFU_COMMANDS` + `JEALOUSY_COOLDOWN` (tetap lokal, tidak dihapus).

Tambahkan 15 aksi baru ke `ACTIONS` (tambahkan di akhir objek, sebelum `};`):

```js
  // ===== approach baru: kuliner =====
  restoran_makan:  { phase: "approach", base: [8, 16], exp: 35, likes: ["deredere", "onee-san", "amayadori"], dislikes: ["kuudere"], text: (n) => `🍽️ Makan malam di restoran romantis. *${n}* tersenyum menatapmu dari seberang meja.` },
  restoran_dimsum: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "deredere"], dislikes: ["ojou-sama"], text: (n) => `🥟 Cicip dimsum bareng! *${n}* rebutan denganmu sampai tawa.` },
  restoran_bbq:    { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "onee-san"], dislikes: ["himedere"], text: (n) => `🍖 Bakar-bakar di BBQ. *${n}* memangangkan daging untukmu.` },
  // ===== approach baru: olahraga =====
  olahraga_hiking: { phase: "approach", base: [6, 12], exp: 30, likes: ["genki", "kuudere"], dislikes: ["himedere", "ojou-sama"], text: (n) => `⛰️ Mendaki bukit. *${n}* terengah-engah tapi semangat!` },
  olahraga_lari:   { phase: "approach", base: [8, 14], exp: 35, likes: ["genki", "tsundere"], dislikes: ["dandere"], text: (n) => `🏃 Lari pagi keliling taman. *${n}* menantangmu balapan.` },
  olahraga_panjat: { phase: "approach", base: [5, 10], exp: 25, likes: ["genki"], dislikes: ["himedere", "ojou-sama"], text: (n) => `🧗 Panjat tebing indoor. *${n}* gemetaran tapi tak mau menyerah.` },
  // ===== approach baru: alam =====
  alam_camping:    { phase: "approach", base: [10, 18], exp: 40, likes: ["kuudere", "amayadori", "dandere"], dislikes: ["himedere"], text: (n) => `🏕️ Camping di bawah bintang. *${n}* bersandar santai menatap langit malam.` },
  alam_mancing:    { phase: "approach", base: [6, 12], exp: 30, likes: ["kuudere", "onee-san"], dislikes: ["genki"], text: (n) => `🎣 Memancing di danau tenang. *${n}* senang saat akhirnya dapat ikan.` },
  alam_perahu:     { phase: "approach", base: [8, 16], exp: 35, likes: ["genki", "amayadori"], dislikes: ["tsundere"], text: (n) => `⛵ Menyusuri sungai naik perahu. *${n}* menutup mata menikmati angin.` },
  // ===== approach baru: seni =====
  seni_museum:     { phase: "approach", base: [6, 12], exp: 30, likes: ["kuudere", "dandere"], dislikes: ["genki"], text: (n) => `🖼️ Menjelajahi museum. *${n}* takjub di depan lukisan tua.` },
  seni_melukis:    { phase: "approach", base: [8, 15], exp: 35, likes: ["deredere", "dandere", "onee-san"], dislikes: [], text: (n) => `🎨 Melukis bareng di studio. Karya *${n}* kacau balau tapi lucu.` },
  seni_konser:     { phase: "approach", base: [10, 18], exp: 40, likes: ["genki", "deredere", "amayadori"], dislikes: ["kuudere"], text: (n) => `🎸 Nonton konser. *${n}* bernyanyi keras tanpa malu.` },
  // ===== intim baru =====
  pijat_bahu:      { phase: "intim", base: [10, 18], exp: 45, likes: ["onee-san", "yandere", "deredere"], dislikes: [], text: (n) => `💆 Kamu memijat bahu *${n}* yang tegang. Ia mendesah rileks.` },
  // ===== married baru =====
  nontonrumah:     { phase: "married", base: [8, 14], exp: 40, likes: ["deredere", "dandere", "tsundere"], dislikes: [], text: (n) => `📺 Nonton maraton film di sofa. *${n}* nyender di bahumu sampai ketiduran.` },
  jalanpagi:       { phase: "married", base: [10, 16], exp: 45, likes: ["genki", "onee-san", "amayadori"], dislikes: [], text: (n) => `🌅 Jalan santai pagi hari sambil beli sarapan. *${n}* menggandengmu.` },
```

Tambahkan 8 event ke `EVENTS`:

```js
  { id: "foto", phase: "any", text: (n) => `📸 Foto bareng! *${n}* melotot saat kejepret momen memalukan.`, aff: 4 },
  { id: "lomba", phase: "any", text: () => `🏅 Kamu ikut lomba jalan santai dan menang hadiah receh!`, koin: () => 500 + Math.floor(Math.random() * 5001) },
  { id: "salahpaham", phase: "any", text: (n) => `📵 *${n}* melihat notif dari akun lain di ponselmu dan salah paham!`, aff: -5, anger: 12 },
  { id: "pujian", phase: "any", text: () => `🗣️ Seseorang memuji kalian: "cocok banget!" *${n}* tersipu.`, aff: 5 },
  { id: "sunset", phase: "any", text: () => `🌇 Kalian berhenti menikmati matahari terbenam. Momen tak terlupakan.`, aff: 6, mood: "romantis" },
  { id: "kenangan", phase: "any", marriedOnly: true, text: () => `📿 Kamu menemukan foto lama kalian. *${n}* terharu dan memelukmu erat.`, aff: 7 },
  { id: "badai", phase: "any", text: (n) => `⛈️ Hujan badai datang! *${n}* kehujanan dan mood-nya rusak.`, mood: "marah", aff: -4 },
  { id: "rezeki", phase: "any", text: () => `🍀 Rezeki nomplok! Kamu dapat uang tak terduga.`, koin: () => 3000 + Math.floor(Math.random() * 8001) },
```

Ubah `applyAction` agar menambahkan flag like/dislike (logika nilai `change` TIDAK berubah):

```js
export function applyAction(key, waifu, moodType = "biasa", rng = Math.random, multOverride = 1) {
  const a = ACTIONS[key];
  if (!a || !waifu || !waifu.personality) return null;
  const [min, max] = a.base;
  let base = min + rng() * (max - min);
  let mult = MOOD_MULT[moodType] || 1;
  mult *= multOverride;
  const tag = PERSONALITY_TAG[waifu.personality] || { like: [], dislike: [] };
  const liked = a.likes.includes(waifu.personality) || tag.like.includes(key);
  const disliked = a.dislikes.includes(waifu.personality) || tag.dislike.includes(key);
  if (liked) mult *= 1.2;
  else if (disliked) mult *= 0.7;
  const change = Math.max(1, Math.round(base * mult));
  return { key, phase: a.phase, change, exp: a.exp, text: a.text(waifu.name), like: liked, dislike: disliked };
}
```

Ubah `rollEvent` agar melewatkan `anger`:

```js
  return { id: e.id, text: typeof e.text === "function" ? e.text(name) : e.text, aff, koin: e.koin ? e.koin() : 0, mood: e.mood || null, nextMult: e.nextMult || 1, anger: e.anger || 0 };
```

- [ ] **Step 4: Jalankan test waifu**

Run: `node --test tests/waifu-lib.test.mjs`
Expected: PASS semua (lama + 3 baru)

- [ ] **Step 5: Commit**

```bash
git add src/lib/ourin-waifu.js tests/waifu-lib.test.mjs
git commit -m "feat(waifu): new actions/events + like/dislike flags + shared romance imports"
```

---

### Task 7: Husbu lib — `src/lib/ourin-husbu.js` + test

**Files:**
- Create: `src/lib/ourin-husbu.js`
- Test: `tests/husbu-lib.test.mjs`

**Interfaces:**
- Consumes: `src/lib/ourin-romance.js` (DOWRY, TIER_ORDER, TIER_VALUE, TIER_EXPECTED, PITY_THRESHOLD, EVENT_CHANCE, MOOD_MULT, getDailyMood), `data/husbu/index.js` (getPool).
- Produces:
  - `export function rollHusbu(pityCounter = 0, rng = Math.random)` → entry pool (pity ≥ PITY_THRESHOLD → hanya Epic+)
  - `export function applyAction(key, husbu, moodType = "biasa", rng = Math.random, multOverride = 1)` → `{ key, phase, change, exp, text, like, dislike }`
  - `export function rollEvent({ married, phase, personality, name } = {}, rng = Math.random)` → `{ id, text, aff, koin, mood, nextMult, anger }`
  - `export function albumStats(history = [], stats = {})` → sama dengan waifu
  - re-export: `getDailyMood, DOWRY, PITY_THRESHOLD, EVENT_CHANCE`

- [ ] **Step 1: Tulis failing test**

```js
// tests/husbu-lib.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert";
import {
  rollHusbu, applyAction, rollEvent, getDailyMood, albumStats, DOWRY,
} from "../src/lib/ourin-husbu.js";

function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

describe("Husbu lib", () => {
  it("rollHusbu with pity >= 20 returns Epic+", () => {
    const rng = mulberry32(7);
    for (let i = 0; i < 50; i++) {
      const h = rollHusbu(20, rng);
      assert.ok(["Epic", "Legendary", "Mythic"].includes(h.tier), h.tier);
    }
  });

  it("rollHusbu distribution sanity over 20000 rolls", () => {
    const rng = mulberry32(99);
    const counts = { Common: 0, Rare: 0, Epic: 0, Legendary: 0, Mythic: 0 };
    for (let i = 0; i < 20000; i++) counts[rollHusbu(0, rng).tier]++;
    const pct = {};
    for (const t of Object.keys(counts)) pct[t] = counts[t] / 20000;
    assert.ok(pct.Common > 0.45 && pct.Common < 0.65, pct.Common);
    assert.ok(pct.Rare > 0.18 && pct.Rare < 0.32, pct.Rare);
    assert.ok(pct.Epic > 0.08 && pct.Epic < 0.18, pct.Epic);
    assert.ok(pct.Legendary > 0.02 && pct.Legendary < 0.09, pct.Legendary);
    assert.ok(pct.Mythic > 0.002 && pct.Mythic < 0.04, pct.Mythic);
  });

  it("applyAction applies personality & mood multipliers", () => {
    const h = { personality: "tsundere", name: "Bakugo" };
    // jalan_taman likes tsundere (×1.2), mood biasa (×1.0), rng 0 → base 10 → 12
    const r = applyAction("jalan_taman", h, "biasa", () => 0);
    assert.equal(r.change, 12);
    assert.equal(r.like, true);
    // mood marah ×0.5: 10 * 1.2 * 0.5 = 6
    const r2 = applyAction("jalan_taman", h, "marah", () => 0);
    assert.equal(r2.change, 6);
  });

  it("applyAction unknown key returns null", () => {
    assert.equal(applyAction("nope", {}, "biasa", () => 0), null);
  });

  it("applyAction dislikes femboy at olahraga_panjat", () => {
    const r = applyAction("olahraga_panjat", { personality: "femboy", name: "Venti" }, "biasa", () => 0);
    assert.equal(r.dislike, true);
  });

  it("rollEvent respects 18% chance and gates", () => {
    let hit = 0;
    for (let i = 0; i < 1000; i++) if (rollEvent({ married: false, phase: "approach", personality: "genki", name: "Denji" }, mulberry32(i))) hit++;
    assert.ok(hit > 100 && hit < 260, `hit=${hit}`);
    for (let i = 0; i < 200; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "Denji" }, mulberry32(i));
      if (e) assert.ok(!["intimate", "confess"].includes(e.id));
    }
  });

  it("rollEvent text renders husbu name, no undefined/function", () => {
    for (let i = 0; i < 500; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "Denji" }, mulberry32(i));
      if (!e) continue;
      assert.strictEqual(typeof e.text, "string");
      assert.ok(!e.text.includes("undefined"), e.text);
      assert.ok(!e.text.includes("function"), e.text);
    }
  });

  it("rollEvent passes anger through", () => {
    for (let i = 0; i < 200; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "Denji" }, mulberry32(i));
      if (!e) continue;
      assert.ok(typeof e.anger === "number");
    }
  });

  it("getDailyMood deterministic", () => {
    assert.equal(getDailyMood("628123", "2026-08-24"), getDailyMood("628123", "2026-08-24"));
  });

  it("albumStats computes luck score", () => {
    const history = Array.from({ length: 100 }, () => ({ name: "C", series: "S", tier: "Common" }));
    const s = albumStats(history, { totalGacha: 100, byTier: { Common: 100 }, pityCounter: 3, rarest: { name: "C", tier: "Common" }, marriedCount: 0 });
    assert.ok(s.luck < 1, s.luck);
    assert.equal(s.last10.length, 10);
  });

  it("DOWRY has all tiers", () => {
    for (const t of ["Common", "Rare", "Epic", "Legendary", "Mythic"]) assert.ok(DOWRY[t]);
  });
});
```

- [ ] **Step 2: Jalankan test, verifikasi FAIL**

Run: `node --test tests/husbu-lib.test.mjs`
Expected: FAIL — `Cannot find module '../src/lib/ourin-husbu.js'`

- [ ] **Step 3: Tulis implementasi**

```js
// src/lib/ourin-husbu.js
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
  { id: "pujian", phase: "any", text: () => `🗣️ Seseorang memuji kalian: "cocok banget!" *${n}* tersipu.`, aff: 5 },
  { id: "sunset", phase: "any", text: () => `🌇 Kalian berhenti menikmati matahari terbenam. Momen tak terlupakan.`, aff: 6, mood: "romantis" },
  { id: "kenangan", phase: "any", marriedOnly: true, text: () => `📿 Kamu menemukan foto lama kalian. *${n}* terharu dan memelukmu erat.`, aff: 7 },
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
```

- [ ] **Step 4: Jalankan test**

Run: `node --test tests/husbu-lib.test.mjs`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/ourin-husbu.js tests/husbu-lib.test.mjs
git commit -m "feat(husbu): core lib - actions, events, roll, albumStats"
```

---

### Task 8: Plugin album & pool husbu

**Files:**
- Create: `plugins/fun/husbualbum.js`, `plugins/fun/husbupool.js`

**Interfaces:**
- Consumes: `albumStats` dari `../src/lib/ourin-husbu.js`, `searchPool` dari `../../data/husbu/index.js`. Data user: `user.husbuHistory`, `user.husbuStats`.
- Produces: command `husbualbum|albumhusbu` dan `husbupool|poolhusbu` (auto-terdaftar oleh loader plugin dari `plugins/fun/`).

- [ ] **Step 1: Tulis file**

`plugins/fun/husbualbum.js` (mirror `waifualbum.js`, ganti key waifu→husbu, nama command):
```js
import { getDatabase } from "../../src/lib/ourin-database.js";
import { albumStats } from "../../src/lib/ourin-husbu.js";

const pluginConfig = {
  name: ["husbualbum", "albumhusbu"],
  alias: [],
  category: "fun",
  description: "Lihat riwayat gacha husbu & statistik luck kamu!",
  usage: ".husbualbum",
  example: ".husbualbum",
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

  const history = user.husbuHistory || [];
  if (!history.length) {
    return m.reply(`📭 *Album kosong!*\nKamu belum pernah gacha husbu. Ketik *${m.prefix}gachahusbu* untuk mulai!`);
  }

  const stats = albumStats(history, user.husbuStats || {});
  const tierEmoji = { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" };
  const bar = (count) => {
    const pct = stats.total ? Math.round((count / stats.total) * 20) : 0;
    return "█".repeat(pct) + "░".repeat(20 - pct);
  };

  let text = `📚 *ALBUM HUSBU & LUCK STATS* 📚\n\n` +
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

`plugins/fun/husbupool.js` (mirror `waifupool.js`):
```js
import { searchPool } from "../../data/husbu/index.js";

const pluginConfig = {
  name: ["husbupool", "poolhusbu"],
  alias: [],
  category: "fun",
  description: "Jelajahi pool husbu — cari nama/franchise/tier!",
  usage: ".husbupool [nama|tier|franchise]",
  example: ".husbupool legendary",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 1,
  isEnabled: true,
};

const pages = new Map();

function trimPages(now = Date.now()) {
  if (pages.size > 500) {
    const cutoff = now - 30 * 60 * 1000;
    for (const [k, v] of pages) {
      if (v.ts < cutoff) pages.delete(k);
    }
  }
}

function tierEmoji(t) {
  return { Common: "🟢", Rare: "🔵", Epic: "🟣", Legendary: "🟡", Mythic: "🔴" }[t] || "";
}

async function handler(m, { sock }) {
  const args = m.args || [];
  const first = (args[0] || "").toLowerCase();
  const nav = (first === "prev" || first === "next") && args.length > 1 ? first : null;
  const query = (nav ? args.slice(1) : args).join(" ").trim();
  const key = `${m.sender}:${query.toLowerCase()}`;
  let page = Math.max(0, pages.get(key)?.page || 0);
  if (nav === "next") page++;
  if (nav === "prev") page--;
  page = Math.max(0, page);
  const PAGE_SIZE = 10;

  const pool = searchPool(query);
  if (!pool.length) {
    pages.set(key, { page: 0, ts: Date.now() });
    trimPages();
    return m.reply(`🔍 Tidak ada husbu cocok dengan *"${query}"*. Coba nama/tier/franchise lain.`);
  }

  const totalPages = Math.max(1, Math.ceil(pool.length / PAGE_SIZE));
  const cur = Math.min(page, totalPages - 1);
  const slice = pool.slice(cur * PAGE_SIZE, (cur + 1) * PAGE_SIZE);

  let text = `📖 *HUSBU POOL* 📖\n\n`;
  text += query ? `🔍 Pencarian: *"${query}"*\n` : "";
  text += `📊 Total: *${pool.length}* husbu | Halaman *${cur + 1}/${totalPages}*\n\n`;
  slice.forEach((h, i) => {
    text += `${cur * PAGE_SIZE + i + 1}. ${tierEmoji(h.tier)} *${h.name}* — ${h.tier}\n`;
    text += `   🏷️ ${h.series} | 🎭 ${h.personality}\n`;
  });

  const buttons = [];
  if (cur > 0) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "⬅️", id: `${m.prefix}husbupool prev ${query}` }) });
  if (cur < totalPages - 1) buttons.push({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "➡️", id: `${m.prefix}husbupool next ${query}` }) });
  if (buttons.length) text += `\nGunakan tombol untuk berpindah halaman.`;

  pages.set(key, { page: cur, ts: Date.now() });
  trimPages();
  return m.reply(text, null, buttons.length ? { buttons } : undefined);
}

export { pluginConfig as config, handler, pages };
```

- [ ] **Step 2: Lint & jalankan test**

Run: `npm run lint`
Expected: PASS (tidak ada error baru)

- [ ] **Step 3: Commit**

```bash
git add plugins/fun/husbualbum.js plugins/fun/husbupool.js
git commit -m "feat(husbu): album + pool browse plugins"
```

---

### Task 9: Rewrite `plugins/fun/gachahusbu.js`

**Files:**
- Modify: `plugins/fun/gachahusbu.js` (rewrite total)

**Interfaces:**
- Consumes: `rollHusbu, applyAction, rollEvent, getDailyMood, DOWRY` dari `../src/lib/ourin-husbu.js`; `angerEffMood, angerUpdate, applyNeglect, finalGain, moodLabel, todayStr` dari `../src/lib/ourin-romance.js`; `addExpWithLevelCheck` dari `../src/lib/ourin-level.js`; DB API existing.
- Produces: command `gachahusbu|gachasuami`, `husbuku|suamiku`, `husbuaction`, `tinggalinhusbu`. State: `user.husbu`, `user.husbuStats`, `user.husbuHistory`.

- [ ] **Step 1: Tulis file baru (ganti isi `plugins/fun/gachahusbu.js`)**

```js
import axios from "axios";
import { getDatabase } from "../../src/lib/ourin-database.js";
import te from "../../src/lib/ourin-error.js";
import { prepareWAMessageMedia, generateWAMessageFromContent } from "ourin";
import { addExpWithLevelCheck } from "../../src/lib/ourin-level.js";
import { rollHusbu, applyAction, rollEvent, getDailyMood, DOWRY } from "../../src/lib/ourin-husbu.js";
import { angerEffMood, angerUpdate, applyNeglect, finalGain, moodLabel, todayStr } from "../../src/lib/ourin-romance.js";

const pluginConfig = {
  name: ["gachahusbu", "husbuaction", "tinggalinhusbu", "husbuku", "suamiku"],
  alias: ["gachasuami"],
  category: "fun",
  description: "Gacha husbu impianmu, rebut hatinya, dan jadikan dia pasanganmu!",
  usage: ".gachahusbu | .husbuku | .tinggalinhusbu",
  example: ".gachahusbu",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 2,
  isEnabled: true,
};

async function getHusbuImage(keyword) {
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
    console.error("[GachaHusbu] Pinterest API error:", e.message);
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

function moodState(user, m) {
  const h = user.husbu;
  if (h.moodUntil && Date.now() < new Date(h.moodUntil).getTime() && h.mood?.type) return h.mood.type;
  return getDailyMood(m.sender, todayStr());
}

function angerMeter(h) {
  const a = h.anger || 0;
  const n = Math.round(a / 10);
  return "█".repeat(n) + "░".repeat(10 - n) + ` (${a}/100)`;
}

async function sendHusbuMessage(m, sock, husbu, textContent, customButtons = null) {
  let imgBuffer = null;
  if (husbu.imageUrl) imgBuffer = await getBuffer(husbu.imageUrl);
  if (!imgBuffer) {
    const newUrl = await getHusbuImage(husbu.keyword);
    husbu.imageUrl = newUrl;
    imgBuffer = await getBuffer(newUrl) || Buffer.alloc(0);
  }
  const media = await prepareWAMessageMedia({ image: imgBuffer }, { upload: sock.waUploadToServer });
  let buttons = customButtons;
  if (!buttons) {
    if (husbu.affection < 80) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🚶 Jalan-jalan", id: `${m.prefix}husbuaction menu_jalanjalan` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "☕ Kafe", id: `${m.prefix}husbuaction menu_kafe` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎬 Bioskop", id: `${m.prefix}husbuaction menu_bioskop` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛍️ Belanja", id: `${m.prefix}husbuaction menu_belanja` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎮 Lainnya", id: `${m.prefix}husbuaction menu_lainnya` }) },
      ];
    } else if (husbu.affection < 100) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🫂 Peluk", id: `${m.prefix}husbuaction menu_peluk` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💋 Cium", id: `${m.prefix}husbuaction menu_cium` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛏️ Tidur", id: `${m.prefix}husbuaction menu_tidur` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🛁 Mandi", id: `${m.prefix}husbuaction menu_mandi` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Lainnya", id: `${m.prefix}husbuaction menu_lainnya` }) },
      ];
    } else if (!husbu.married) {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💍 Terima Lamaran", id: `${m.prefix}husbuaction nikah` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💝 Hadiah", id: `${m.prefix}husbuaction hadiah` }) },
      ];
    } else {
      buttons = [
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "👩‍❤️‍👨 Mesra", id: `${m.prefix}husbuaction mesra` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "💌 Rayu", id: `${m.prefix}husbuaction rayu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🌴 Bulan Madu", id: `${m.prefix}husbuaction menu_bulanmadu` }) },
        { name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎁 Klaim Harian", id: `${m.prefix}husbuaction klaim` }) },
      ];
    }
  }
  let footerText = "❤️ Buat dia luluh dan jatuh cinta padamu!";
  if (customButtons) footerText = "💭 Dia menunggu jawabanmu...";
  else if (husbu.married) footerText = "❤️ Kamu adalah istrinya yang sah!";

  const msg = generateWAMessageFromContent(m.chat, {
    viewOnceMessage: {
      message: {
        messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
        interactiveMessage: {
          body: { text: textContent },
          footer: { text: footerText },
          header: {
            title: `🌟 *${husbu.tier.toUpperCase()} TIER HUSBANDO* 🌟`,
            subtitle: husbu.name,
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

function marrageDay(husbu) {
  if (!husbu.marriedDate) return 0;
  const a = new Date(husbu.marriedDate);
  const b = new Date();
  return Math.floor((b - a) / 86400000) + 1;
}

function initStats(user) {
  if (!user.husbuStats) user.husbuStats = { totalGacha: 0, byTier: {}, pityCounter: 0, rarest: null, marriedCount: 0 };
  return user.husbuStats;
}

function recordPull(user, husbu, stats) {
  stats.totalGacha++;
  stats.byTier[husbu.tier] = (stats.byTier[husbu.tier] || 0) + 1;
  const isEpicPlus = ["Epic", "Legendary", "Mythic"].includes(husbu.tier);
  stats.pityCounter = isEpicPlus ? 0 : (stats.pityCounter || 0) + 1;
  if (!stats.rarest || ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(husbu.tier) > ["Common", "Rare", "Epic", "Legendary", "Mythic"].indexOf(stats.rarest.tier)) {
    stats.rarest = { name: husbu.name, tier: husbu.tier };
  }
  if (!user.husbuHistory) user.husbuHistory = [];
  user.husbuHistory.push({ name: husbu.name, series: husbu.series, tier: husbu.tier, at: new Date().toISOString() });
  if (user.husbuHistory.length > 100) user.husbuHistory = user.husbuHistory.slice(-100);
}

const HUSB_PERSONALITIES = ["tsundere", "kuudere", "genki", "yandere", "dandere", "oji-san", "playboy", "prince", "badboy", "sunao", "femboy"];

async function handler(m, { sock }) {
  const db = getDatabase();
  const user = db.getUser(m.sender);
  if (!user) return;

  const cmd = m.command.toLowerCase();

  if (cmd === "husbuku" || cmd === "suamiku") {
    if (!user.husbu) return m.reply(`⚠️ *Kamu belum memiliki husbu!*\nSilakan ketik *${m.prefix}gachahusbu* untuk memulainya!`);
    m.react("🕕");
    const h = user.husbu;
    const mood = moodState(user, m);
    const { decay } = applyNeglect(h);
    let status = h.married ? "Telah Menikah 💍" : "Pendekatan 💖";
    const day = marrageDay(h);
    const moodLine = h.married
      ? `\n🗓️ *Hari ke-${day}*${day >= 7 ? ` ${day >= 100 ? "🏆" : day >= 30 ? "🎖️" : "🎉"}` : ""}`
      : "";
    let textContent = `📸 *STATUS HUSBU KAMU* 📸\n\n` +
      `💖 *Nama:* ${h.name}\n` +
      `💎 *Tier:* ${h.tier}\n` +
      `🎭 *Personality:* ${h.personality}\n` +
      `🌤️ *Mood hari ini:* ${moodLabel(mood)}\n` +
      `😡 *Anger:* ${angerMeter(h)}\n` +
      `💞 *Affection:* ${h.affection}/100\n` +
      `💍 *Status:* ${status}${moodLine}\n`;
    if (decay > 0) textContent += `📉 *Neglect:* Affection turun -${decay} karena kamu jarang interaksi!\n`;
    if ((h.anger || 0) >= 50) textContent += `⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n`;
    textContent += `\nLanjutkan interaksi dengan memilih aksi di bawah!`;
    user.husbu = h;
    db.setUser(m.sender, user);
    m.react("✅");
    return await sendHusbuMessage(m, sock, h, textContent, null);
  }

  if (cmd === "tinggalinhusbu") {
    if (!user.husbu) return m.reply(`⚠️ *Kamu bahkan belum punya husbu!* Cari dulu gih!`);
    const husbuName = user.husbu.name;
    const husbuJid = 'husbu_' + husbuName.replace(/\s+/g, '') + '@s.whatsapp.net';
    if (user.husbu.married) {
      if (user.fun && user.fun.pasangan === husbuJid) user.fun.pasangan = "";
      db.setUser(husbuJid, { fun: { pasangan: "" } });
    }
    delete user.husbu;
    db.setUser(m.sender, user);
    m.react("💔");
    return m.reply(`💔 *KAMU MENCAMPAKKAN ${husbuName.toUpperCase()}!*\n\nKamu mengembalikan barang-barangnya dan memintanya untuk pergi. Dia menatapmu dengan mata kecewa yang mendalam, berbalik tanpa sepatah kata pun, lalu menghilang di tengah hujan.\n\nKalian kini resmi berpisah.`);
  }

  if (cmd === "gachahusbu" || cmd === "gachasuami") {
    if (user.husbu) {
      m.react("😡");
      let pesanStatus = user.husbu.married ? "Dia sudah menjadi suamimu!" : "Dia sedang berusaha meluluhkan hatimu!";
      return m.reply(`⚠️ *Kamu sudah memiliki Husbu!*\n\nNama: *${user.husbu.name}*\nTier: *${user.husbu.tier}*\nAffection: *${user.husbu.affection}/100*\n\nJangan serakah! Jaga husbu yang kamu miliki sekarang. ${pesanStatus} Ketik *${m.prefix}husbuku* untuk berinteraksi dengannya.`);
    }
    const sub = (m.args[0] || "").toLowerCase();
    if (sub !== "start") {
      const panduan = `💕 *SISTEM GACHA HUSBU* 💕\n\n` +
        `Simulasi kencan virtual interaktif untuk mendapatkan laki-laki anime idamanmu! Tarik perhatiannya, buat dia jatuh cinta, dan nikahi dia!\n\n` +
        `*PENGGUNAAN:*\n` +
        `• *${m.prefix}gachahusbu* — Panduan ini\n` +
        `• *${m.prefix}husbuku* — Panel interaksi\n` +
        `• *${m.prefix}husbualbum* — Riwayat & statistik luck\n` +
        `• *${m.prefix}husbupool* — Jelajahi pool husbu\n` +
        `• *${m.prefix}tinggalinhusbu* — Putuskan hubungan\n\n` +
        `*ALUR:*\n1. Tekan tombol **Panggil Husbando**.\n2. 3 Fase: Pendekatan (<80) → Intim (80–99) → Menikah (100).\n3. Mood, personality & anger memengaruhi poin affection.\n4. Pity: 20 roll tanpa Epic+ dijamin dapat Epic+.\n5. Affection makin sulit ditambah makin tinggi + interaksi berulang menyusut.`;
      const msg = generateWAMessageFromContent(m.chat, {
        viewOnceMessage: {
          message: {
            messageContextInfo: { deviceListMetadata: {}, deviceListMetadataVersion: 2 },
            interactiveMessage: {
              body: { text: panduan },
              footer: { text: "Tekan tombol untuk mulai!" },
              nativeFlowMessage: { buttons: [{ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: "🎲 Panggil Husbando!", id: `${m.prefix}gachahusbu start` }) }] },
            },
          },
        },
      }, { quoted: m });
      return await sock.relayMessage(m.chat, msg.message, { messageId: msg.key.id });
    }

    m.react("🕕");
    try {
      const stats = initStats(user);
      const picked = rollHusbu(stats.pityCounter);
      const husbu = {
        ...picked,
        affection: 50,
        married: false,
        imageUrl: await getHusbuImage(picked.keyword),
      };
      recordPull(user, husbu, stats);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      const textContent = `🎉 *KYAAA! KAMU MENDAPATKAN HUSBANDO BARU!* 🎉\n\n` +
        `💖 *Nama:* ${husbu.name}\n🎂 *Usia:* ${husbu.age} tahun\n📏 *Tinggi:* ${husbu.height}\n⚖️ *Berat:* ${husbu.weight}\n` +
        `💎 *Tier:* ${husbu.tier}\n🎭 *Personality:* ${husbu.personality}\n💞 *Affection:* ${husbu.affection}/100\n\n` +
        `Pilih interaksi untuk mulai PDKT. Hati-hati jangan sampai affection habis!`;
      m.react("✅");
      await sendHusbuMessage(m, sock, husbu, textContent, null);
    } catch (err) {
      console.error(err);
      m.react("☢");
      return m.reply(te(m.prefix, m.command, m.pushName));
    }
    return;
  }

  if (cmd === "husbuaction") {
    if (!user.husbu) { m.react("❌"); return m.reply(`Kamu belum memiliki husbu! Ketik *${m.prefix}gachahusbu* untuk memulai.`); }

    const action = (m.args[0] || "").toLowerCase();
    const husbu = user.husbu;
    if (!husbu.personality) {
      let h = 0;
      for (const c of husbu.name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
      husbu.personality = HUSB_PERSONALITIES[h % HUSB_PERSONALITIES.length];
      user.husbu = husbu;
      db.setUser(m.sender, user);
    }
    const sendMenu = (title, options) => sendHusbuMessage(m, sock, husbu, title, options.map(([label, id]) => ({ name: "quick_reply", buttonParamsJson: JSON.stringify({ display_text: label, id: `${m.prefix}husbuaction ${id}` }) })));

    const MENUS = {
      menu_jalanjalan: ["🚶 Jalan-jalan ke...", [["🌳 Taman", "jalan_taman"], ["🏢 Mall", "jalan_mall"], ["🏖️ Pantai", "jalan_pantai"], ["🌆 Kota", "jalan_kota"]]],
      menu_kafe: ["☕ Kamu traktir dia...", [["☕ Kopi Pahit", "kafe_kopi"], ["🍵 Matcha Latte", "kafe_matcha"], ["🍰 Kue", "kafe_kue"], ["🥛 Susu Hangat", "kafe_susu"]]],
      menu_bioskop: ["🎬 Pilih genre film...", [["💥 Action", "bioskop_romantis"], ["👻 Horor", "bioskop_horor"], ["🎨 Animasi", "bioskop_animasi"]]],
      menu_belanja: ["🛍️ Minta dibeliin apa?..", [["👕 Baju Keren", "belanja_baju"], ["💍 Aksesoris", "belanja_perhiasan"], ["🧸 Boneka", "belanja_boneka"]]],
      menu_kuliner: ["🍽️ Makan di...", [["🍽️ Restoran", "restoran_makan"], ["🥟 Dimsum", "restoran_dimsum"], ["🍖 BBQ", "restoran_bbq"]]],
      menu_olahraga: ["⚽ Olahraga...", [["⛰️ Hiking", "olahraga_hiking"], ["🏃 Lari Pagi", "olahraga_lari"], ["🧗 Panjat", "olahraga_panjat"]]],
      menu_alam: ["🏕️ Petualangan alam...", [["🏕️ Camping", "alam_camping"], ["🎣 Mancing", "alam_mancing"], ["⛵ Perahu", "alam_perahu"]]],
      menu_seni: ["🎨 Seni & budaya...", [["🖼️ Museum", "seni_museum"], ["🎨 Melukis", "seni_melukis"], ["🎸 Konser", "seni_konser"]]],
      menu_karaoke: ["🎤 Karaoke...", [["🎶 Duet", "karaoke_duet"], ["🎵 Solo", "karaoke_solo"]]],
      menu_arcade: ["🕹️ Di arcade...", [["🎮 Adu Skor", "arcade_duo"], ["🎯 Main Boneka", "arcade_boneka"]]],
      menu_piknik: ["🧺 Piknik...", [["🌳 Di Taman", "piknik_taman"], ["🏝️ Di Pantai", "piknik_pantai"]]],
      menu_masak: ["🍳 Masak bareng...", [["🍳 Masakan Rumah", "masak_bareng"], ["🍰 Kue", "masak_kue"]]],
      menu_lainnya: ["🎮 Aksi lainnya...", husbu.affection < 80
        ? [["🎤 Karaoke", "menu_karaoke"], ["🕹️ Arcade", "menu_arcade"], ["🧺 Piknik", "menu_piknik"], ["🍳 Masak", "menu_masak"], ["🍽️ Kuliner", "menu_kuliner"], ["⚽ Olahraga", "menu_olahraga"], ["🏕️ Alam", "menu_alam"], ["🎨 Seni", "menu_seni"]]
        : husbu.affection < 100
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

    if (["peluk", "cium", "tidur", "mandi", "gendong_putri", "gendong_punggung", "tepuk_kepala", "intim_belakang", "intim_depan", "intim_kening", "intim_bibir", "intim_kelon", "intim_panas", "intim_punggung", "intim_bahu"].includes(action) && husbu.affection < 80) {
      const drop = Math.floor(Math.random() * 30) + 30;
      husbu.affection = Math.max(0, husbu.affection - drop);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      m.react("💢");
      return m.reply(`💢 *HENTIKAN!* Kamu bertindak terlalu agresif saat perasaannya masih samar! *${husbu.name}* menepis tanganmu dengan wajah marah dan kecewa.\n💞 *Affection -${drop}*`);
    }

    if (action === "nikah") {
      if (husbu.affection < 100) return m.reply(`⚠️ Affection belum 100! Jangan terburu-buru menerima lamarannya!`);
      if (husbu.married) return m.reply(`⚠️ Kalian kan sudah menikah!`);
      husbu.married = true;
      husbu.marriedDate = new Date().toISOString();
      const dowry = DOWRY[husbu.tier];
      db.updateEnergi(m.sender, dowry.limit);
      db.updateKoin(m.sender, dowry.koin);
      await addExpWithLevelCheck(sock, m, db, user, dowry.exp);
      const stats = initStats(user);
      stats.marriedCount = (stats.marriedCount || 0) + 1;
      if (!user.fun) user.fun = {};
      const husbuJid = 'husbu_' + husbu.name.replace(/\s+/g, '') + '@s.whatsapp.net';
      user.fun.pasangan = husbuJid;
      db.setUser(husbuJid, { fun: { pasangan: m.sender }, name: husbu.name });
      user.husbu = husbu;
      db.setUser(m.sender, user);
      m.react("💍");
      return m.reply(`💍 *KAMU RESMI MENERIMA LAMARAN ${husbu.name.toUpperCase()}!* 💍\n\nSebagai nafkah:\n- ⚡ ${dowry.limit.toLocaleString()} Limit/Energi\n- 💰 ${dowry.koin.toLocaleString()} Koin\n- ✨ ${dowry.exp.toLocaleString()} EXP\n\nStatus \`.cekpacar\` kini resmi berpasangan!`);
    }

    if (action === "hadiah") {
      if (husbu.affection < 100) return m.reply(`⚠️ Dia belum cukup mencintaimu untuk memberi hadiah!`);
      const g = 300 + Math.floor(Math.random() * 401);
      db.updateEnergi(m.sender, g);
      m.react("💝");
      return m.reply(`💝 *${husbu.name}* memberimu hadiah!\nKamu mendapat ⚡ ${g} Limit/Energi!`);
    }

    if (action === "klaim") {
      if (!husbu.married) return m.reply(`⚠️ Hanya suami sah yang bisa klaim hadiah harian!`);
      const last = husbu.lastClaimDate;
      if (last === todayStr()) return m.reply(`⚠️ Kamu sudah klaim hari ini! Coba lagi besok.`);
      const tierMult = { Common: 1, Rare: 1.2, Epic: 1.5, Legendary: 2, Mythic: 3 }[husbu.tier] || 1;
      const koin = Math.floor((5000 + Math.random() * 15001) * tierMult);
      const exp = Math.floor((200 + Math.random() * 601) * tierMult);
      const day = marrageDay(husbu);
      let bonus = "";
      if (day === 7 || day === 30 || day === 100) {
        const bKoin = day * 1000 * tierMult;
        db.updateKoin(m.sender, bKoin);
        bonus = `\n🎉 *MILESTONE HARI KE-${day}!* Bonus +${bKoin.toLocaleString()} Koin!`;
      }
      husbu.lastClaimDate = todayStr();
      user.husbu = husbu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, koin);
      await addExpWithLevelCheck(sock, m, db, user, exp);
      m.react("🎁");
      return m.reply(`🎁 *${husbu.name}* memberi hadiah harian!\n💰 +${koin.toLocaleString()} Koin\n✨ +${exp} EXP${bonus}`);
    }

    const { decay: negDecay } = applyNeglect(husbu);
    const mult = husbu.nextMultUntil && Date.now() < new Date(husbu.nextMultUntil).getTime() ? 0.8 : 1;
    const mood = angerEffMood(moodState(user, m), husbu);
    const result = applyAction(action, husbu, mood, undefined, mult);
    if (!result) { m.react("❓"); return m.reply(`Aksi tidak dikenali. Gunakan tombol husbu.`); }

    if (result.phase === "married" && !husbu.married) { m.react("⛔"); return m.reply(`Aksi ini hanya untuk pasangan suami istri!`); }
    if (result.phase === "intim" && !husbu.married && husbu.affection < 80) return;

    if (result.phase === "married" && action !== "nikah") {
      const { change } = finalGain(result, husbu, { actionsToday: 0 });
      husbu.affection = Math.min(100, husbu.affection + change);
      user.husbu = husbu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, Math.floor(change * 100));
      await addExpWithLevelCheck(sock, m, db, user, result.exp);
      m.react("❤️");
      return m.reply(`${result.text}\n\n💞 *Affection +${change}* (Total: ${husbu.affection}/100)\n💰 +${Math.floor(change * 100)} Koin\n✨ +${result.exp} EXP`);
    }

    if (husbu.lastActionDate !== todayStr()) {
      husbu.actionsToday = 0;
      husbu.lastActionDate = todayStr();
    }
    husbu.actionsToday = (husbu.actionsToday || 0) + 1;

    const eff = finalGain(result, husbu, { actionsToday: husbu.actionsToday });
    husbu.anger = angerUpdate(husbu, result);

    const affBefore = husbu.affection;
    let newAff = affBefore + eff.change;
    let eventBlock = "";
    let eventExp = 0;
    let eventKoin = 0;
    const ev = rollEvent({ married: husbu.married, phase: result.phase, personality: husbu.personality, name: husbu.name });
    if (ev) {
      newAff += ev.aff;
      eventKoin = ev.koin || 0;
      if (ev.anger) husbu.anger = Math.min(100, (husbu.anger || 0) + ev.anger);
      if (ev.mood) { husbu.mood = { type: ev.mood, since: new Date().toISOString() }; husbu.moodUntil = new Date(Date.now() + 12 * 3600000).toISOString(); }
      if (ev.nextMult < 1) husbu.nextMultUntil = new Date(Date.now() + 24 * 3600000).toISOString();
      eventBlock = `\n\n✨ *EVENT:* ${ev.text}${ev.aff ? ` (${ev.aff > 0 ? "+" : ""}${ev.aff} aff)` : ""}${eventKoin ? ` (+${eventKoin.toLocaleString()} koin)` : ""}`;
      eventExp = result.exp;
    }
    newAff = Math.max(0, Math.min(100, newAff));
    husbu.affection = newAff;
    user.husbu = husbu;
    db.setUser(m.sender, user);
    if (eventKoin) db.updateKoin(m.sender, eventKoin);
    await addExpWithLevelCheck(sock, m, db, user, result.exp + (eventExp ? 15 : 0));

    let affText = `💞 *Affection:* ${husbu.affection}/100`;
    if (eff.change !== 0) affText = `💞 *Affection berubah:* ${eff.change > 0 ? "+" : ""}${eff.change} (Total: ${husbu.affection}/100)`;
    if (husbu.affection === 100) affText = `💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Dia akan melamarmu!*`;
    if (negDecay > 0) affText += `\n📉 *Neglect: -${negDecay} aff* (jarang interaksi)`;
    if (eff.angry) affText += `\n😡 *Dia MARAH!* Drain -${eff.drain} aff`;
    if (eff.extra.length) affText += `\n${eff.extra.join("\n")}`;

    if (husbu.affection <= 0) {
      const husbuName = husbu.name;
      if (husbu.married) {
        const husbuJid = 'husbu_' + husbuName.replace(/\s+/g, '') + '@s.whatsapp.net';
        if (user.fun && user.fun.pasangan === husbuJid) user.fun.pasangan = "";
        db.setUser(husbuJid, { fun: { pasangan: "" } });
      }
      delete user.husbu;
      db.setUser(m.sender, user);
      m.react("💔");
      return m.reply(`💔 *${husbuName.toUpperCase()} MENINGGALKANMU!* 💔\n\nKarena kasih sayangnya habis (0), dia mengemasi seluruh barang-barangnya dan pergi. Kamu kehilangan husbumu! *(Ketik ${m.prefix}gachahusbu untuk memulai ulang)*`);
    }

    m.react(husbu.affection === 100 ? "💍" : "✨");
    const updated = `${result.text}${eventBlock}\n\n${affText}\n✨ +${result.exp} EXP${eventKoin ? `\n💰 +${eventKoin.toLocaleString()} Koin` : ""}`;
    await sendHusbuMessage(m, sock, husbu, updated, null);
  }
}

export { pluginConfig as config, handler };
```

- [ ] **Step 2: Lint**

Run: `npm run lint`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add plugins/fun/gachahusbu.js
git commit -m "feat(husbu): rewrite plugin mirroring gachawaifu + difficulty systems"
```

---

### Task 10: Upgrade `plugins/fun/gachawaifu.js` — menu baru + difficulty + panel anger

**Files:**
- Modify: `plugins/fun/gachawaifu.js`

**Interfaces:**
- Consumes: `angerEffMood, angerUpdate, applyNeglect, finalGain` dari `../src/lib/ourin-romance.js` (moodLabel/todayStr pakai fungsi lokal existing di file).
- Produces: menu kencan baru (kuliner/olahraga/alam/seni), panel anger + neglect, difficulty pada alur action.

- [ ] **Step 1: Edit — import**

Ubah baris import (`plugins/fun/gachawaifu.js` line 6):
```js
import { rollWaifu, applyAction, rollEvent, getDailyMood, DOWRY } from "../../src/lib/ourin-waifu.js";
import { angerEffMood, angerUpdate, applyNeglect, finalGain } from "../../src/lib/ourin-romance.js";
```

Catatan: jangan import `moodLabel`/`todayStr` dari romance — gachawaifu.js sudah punya fungsi lokal dengan nama sama (baris 50-56); import akan SyntaxError. Pakai yang lokal.

Tambahkan helper `angerMeter` setelah fungsi `moodState` (line ~62):
```js
function angerMeter(w) {
  const a = w.anger || 0;
  const n = Math.round(a / 10);
  return "█".repeat(n) + "░".repeat(10 - n) + ` (${a}/100)`;
}
```

- [ ] **Step 2: Edit — panel `waifuku` tampilkan anger + neglect**

Ubah blok `if (cmd === "waifuku" || cmd === "istriku")` (line 163-183). Ganti seluruh isi setelah `m.react("🕕");`:

```js
    m.react("🕕");
    const w = user.waifu;
    const mood = moodState(user, m);
    const { decay } = applyNeglect(w);
    let status = w.married ? "Telah Menikah 💍" : "Pendekatan 💖";
    const day = marrageDay(w);
    const moodLine = w.married
      ? `\n🗓️ *Hari ke-${day}*${day >= 7 ? ` ${day >= 100 ? "🏆" : day >= 30 ? "🎖️" : "🎉"}` : ""}`
      : "";
    let textContent = `📸 *STATUS WAIFU KAMU* 📸\n\n` +
      `💖 *Nama:* ${w.name}\n` +
      `💎 *Tier:* ${w.tier}\n` +
      `🎭 *Personality:* ${w.personality}\n` +
      `🌤️ *Mood hari ini:* ${moodLabel(mood)}\n` +
      `😡 *Anger:* ${angerMeter(w)}\n` +
      `💞 *Affection:* ${w.affection}/100\n` +
      `💍 *Status:* ${status}${moodLine}\n`;
    if (decay > 0) textContent += `📉 *Neglect:* Affection turun -${decay} karena kamu jarang interaksi!\n`;
    if ((w.anger || 0) >= 50) textContent += `⚠️ *Dia sedang MARAH!* Perbaiki hubungan sebelum affection habis!\n`;
    textContent += `\nLanjutkan interaksi dengan memilih aksi di bawah!`;
    user.waifu = w;
    db.setUser(m.sender, user);
    m.react("✅");
    return await sendWaifuMessage(m, sock, w, textContent, null);
```

- [ ] **Step 3: Edit — MENUS tambah 4 submenu**

Di objek `MENUS` (line 272-292), tambahkan 4 entry baru di akhir:
```js
      menu_kuliner: ["🍽️ Makan di...", [["🍽️ Restoran", "restoran_makan"], ["🥟 Dimsum", "restoran_dimsum"], ["🍖 BBQ", "restoran_bbq"]]],
      menu_olahraga: ["⚽ Olahraga...", [["⛰️ Hiking", "olahraga_hiking"], ["🏃 Lari Pagi", "olahraga_lari"], ["🧗 Panjat", "olahraga_panjat"]]],
      menu_alam: ["🏕️ Petualangan alam...", [["🏕️ Camping", "alam_camping"], ["🎣 Mancing", "alam_mancing"], ["⛵ Perahu", "alam_perahu"]]],
      menu_seni: ["🎨 Seni & budaya...", [["🖼️ Museum", "seni_museum"], ["🎨 Melukis", "seni_melukis"], ["🎸 Konser", "seni_konser"]]],
```

Ubah entry `menu_lainnya` pada cabang `< 80` agar menautkan submenu baru (ganti array 4 item):
```js
          ? [["🎤 Karaoke", "menu_karaoke"], ["🕹️ Arcade", "menu_arcade"], ["🧺 Piknik", "menu_piknik"], ["🍳 Masak", "menu_masak"], ["🍽️ Kuliner", "menu_kuliner"], ["⚽ Olahraga", "menu_olahraga"], ["🏕️ Alam", "menu_alam"], ["🎨 Seni", "menu_seni"]]
```

- [ ] **Step 4: Edit — alur `waifuaction` pakai difficulty**

Di bagian aksi (mulai `const mult = waifu.nextMultUntil...` line 360), ganti blok dari baris itu sampai `const result = applyAction(...)` dan proses berikut:

Ubah baris (line 360-361):
```js
    const { decay: negDecay } = applyNeglect(waifu);
    const mult = waifu.nextMultUntil && Date.now() < new Date(waifu.nextMultUntil).getTime() ? 0.8 : 1;
    const mood = angerEffMood(moodState(user, m), waifu);
    const result = applyAction(action, waifu, mood, undefined, mult);
```

Ubah blok `if (result.phase === "married" && action !== "nikah")` (line 367-375) — ganti `result.change` dengan hasil `finalGain`:
```js
    if (result.phase === "married" && action !== "nikah") {
      const { change } = finalGain(result, waifu, { actionsToday: 0 });
      waifu.affection = Math.min(100, waifu.affection + change);
      user.waifu = waifu;
      db.setUser(m.sender, user);
      db.updateKoin(m.sender, Math.floor(change * 100));
      await addExpWithLevelCheck(sock, m, db, user, result.exp);
      m.react("❤️");
      return m.reply(`${result.text}\n\n💞 *Affection +${change}* (Total: ${waifu.affection}/100)\n💰 +${Math.floor(change * 100)} Koin\n✨ +${result.exp} EXP`);
    }
```

Ubah blok "aksi approach/intim biasa" (line 377-397) — ganti perhitungan dengan fatigue + anger, dan sematkan info negDecay/anger/fatigue ke affText:
```js
    // aksi approach/intim biasa
    if (waifu.lastActionDate !== todayStr()) {
      waifu.actionsToday = 0;
      waifu.lastActionDate = todayStr();
    }
    waifu.actionsToday = (waifu.actionsToday || 0) + 1;

    const eff = finalGain(result, waifu, { actionsToday: waifu.actionsToday });
    waifu.anger = angerUpdate(waifu, result);

    const affBefore = waifu.affection;
    let newAff = affBefore + eff.change;
    let eventBlock = "";
    let eventExp = 0;
    let eventKoin = 0;
    const ev = rollEvent({ married: waifu.married, phase: result.phase, personality: waifu.personality, name: waifu.name });
    if (ev) {
      newAff += ev.aff;
      eventKoin = ev.koin || 0;
      if (ev.anger) waifu.anger = Math.min(100, (waifu.anger || 0) + ev.anger);
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
    if (eff.change !== 0) affText = `💞 *Affection berubah:* ${eff.change > 0 ? "+" : ""}${eff.change} (Total: ${waifu.affection}/100)`;
    if (waifu.affection === 100) affText = `💞 *Affection MAKSIMAL! (100/100)* 🎉\n💍 *Nikahi dia sekarang!*`;
    if (negDecay > 0) affText += `\n📉 *Neglect: -${negDecay} aff* (jarang interaksi)`;
    if (eff.angry) affText += `\n😡 *Dia MARAH!* Drain -${eff.drain} aff`;
    if (eff.extra.length) affText += `\n${eff.extra.join("\n")}`;
```

- [ ] **Step 5: Lint & jalankan seluruh test**

Run: `npm run lint && node --test tests/`
Expected: keduanya PASS (waifu-lib termasuk 3 test baru Task 6)

- [ ] **Step 6: Commit**

```bash
git add plugins/fun/gachawaifu.js
git commit -m "feat(waifu): new date menus + anger/neglect/fatigue systems"
```

---

### Task 11: Handler jealousy + verifikasi final

**Files:**
- Modify: `src/handler.js` (line 1805-1806)
- Run: `npm test`, `npm run lint`

**Interfaces:**
- Consumes: `jealousyCheckAll` dari `./lib/ourin-romance.js`.

- [ ] **Step 1: Edit handler**

Ubah (line 1805-1806):
```js
      const { jealousyCheckAll } = await import("./lib/ourin-romance.js");
      await jealousyCheckAll({ m, sock, db, command: m.command }).catch(() => {});
```

- [ ] **Step 2: Jalankan seluruh suite**

Run: `npm test`
Expected: PASS semua — termasuk `waifu-data`, `waifu-lib`, `husbu-data`, `husbu-lib`, `romance-lib`, dan test existing lain.

Run: `npm run lint`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/handler.js
git commit -m "feat: wire jealousyCheckAll for waifu + husbu"
```

---

## Self-Review Summary

- **Spec coverage:** pool ≥300 (Task 2-5) ✓, personality husbu 11 + femboy (Task 2 loader, Task 3-5 data) ✓, shared difficulty engine (Task 1) ✓, aksi/event baru (Task 6 waifu, Task 7 husbu) ✓, rewrite gachahusbu (Task 9) ✓, album/pool husbu (Task 8) ✓, waifu upgrade (Task 10) ✓, jealousy (Task 11) ✓, migration backfill (Task 9 personality hash) ✓, test waifu hijau (Task 6, verifikasi Task 10-11) ✓.
- **Type consistency:** `applyNeglect`, `finalGain`, `angerEffMood`, `angerUpdate` — signature konsisten antara Task 1 definisi, Task 9/10 pemakaian. `rollHusbu`, `applyAction`, `rollEvent`, `albumStats` konsisten Task 7/8/9. `jealousyCheckAll` Task 1/11.
- **Placeholder scan:** semua langkah berisi kode aktual; tidak ada TBD/TODO.
