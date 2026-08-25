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
