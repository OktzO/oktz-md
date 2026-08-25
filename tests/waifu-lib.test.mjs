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
    for (let i = 0; i < 1000; i++) if (rollEvent({ married: false, phase: "approach", personality: "genki", name: "Miku" }, mulberry32(i))) hit++;
    assert.ok(hit > 100 && hit < 260, `hit=${hit}`);
    // event intim phase never fires in approach
    for (let i = 0; i < 200; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "Miku" }, mulberry32(i));
      if (e) assert.ok(!["intimate", "confess"].includes(e.id));
    }
  });

  it("rollEvent text renders waifu name as string, no undefined/function", () => {
    for (let i = 0; i < 500; i++) {
      const e = rollEvent({ married: false, phase: "approach", personality: "genki", name: "Miku" }, mulberry32(i));
      if (!e) continue;
      assert.strictEqual(typeof e.text, "string");
      assert.ok(!e.text.includes("undefined"), e.text);
      assert.ok(!e.text.includes("function"), e.text);
    }
    for (let i = 0; i < 500; i++) {
      const e = rollEvent({ married: true, phase: "married", personality: "deredere", name: "Miku" }, mulberry32(i));
      if (!e) continue;
      assert.strictEqual(typeof e.text, "string");
      assert.ok(!e.text.includes("undefined"), e.text);
      assert.ok(!e.text.includes("function"), e.text);
    }
  });

  it("applyAction honors multOverride (nextMult 0.8)", () => {
    const waifu = { personality: "tsundere", name: "Miku" };
    // jalan_taman likes tsundere ×1.2, biasa ×1.0, rng 0 → base 10, mult 0.8 → 10*0.96 = 9.6 → 10
    const r = applyAction("jalan_taman", waifu, "biasa", () => 0, 0.8);
    assert.equal(r.change, 10);
    assert.equal(applyAction("jalan_taman", waifu, "biasa", () => 0, 1).change, 12);
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
    assert.equal(s.last10.length, 10);
    const myth = albumStats([{ name: "M", series: "S", tier: "Mythic" }], { totalGacha: 1, byTier: { Mythic: 1 }, rarest: { name: "M", tier: "Mythic" }, marriedCount: 0 });
    assert.ok(myth.luck > 1, myth.luck);
  });

  it("albumStats luck uses all-time byTier, not capped last-100 history", () => {
    // 300 pulls, all Common (capped history only holds last 100)
    const history = Array.from({ length: 100 }, () => ({ name: "C", series: "S", tier: "Common" }));
    const s = albumStats(history, { totalGacha: 300, byTier: { Common: 300 }, pityCounter: 3, rarest: { name: "C", tier: "Common" }, marriedCount: 0 });
    assert.equal(s.luck, 0.44); // 300/300/2.25
    assert.equal(s.last10.length, 10);
  });

  it("DOWRY has all tiers", () => {
    for (const t of ["Common", "Rare", "Epic", "Legendary", "Mythic"]) assert.ok(DOWRY[t]);
  });

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
});
