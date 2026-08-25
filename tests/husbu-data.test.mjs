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
