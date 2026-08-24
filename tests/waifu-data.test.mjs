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
