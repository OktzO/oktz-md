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
    const p = { affection: 50, anger: 30, lastInteractionAt: new Date(now - 25 * 3600000).toISOString() };
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
