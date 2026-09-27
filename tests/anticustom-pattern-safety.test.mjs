import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  analyzeCustomPattern,
  matchCustomRule,
  CUSTOM_MAX_PATTERN_LEN,
  CUSTOM_MAX_SUBJECT_LEN,
  CUSTOM_MAX_RULES,
} from "../src/lib/group-protection.js";
import { parsePatternAnswer } from "../plugins/group/anticustom.js";

const regexRule = (pattern, extra = {}) => ({
  name: "test",
  groupName: "test",
  pattern,
  type: "regex",
  action: "remove",
  flags: "i",
  ...extra,
});

const BAD = {
  "nested quantifier": "(a+)+$",
  "nested quantifier non-capturing": "(?:a+)+",
  "nested quantifier deep": "((a+)+)+$",
  "nested quantifier optional inside": "([a-z]+)*$",
  "nested quantifier bounded inside": "(a{1,2})+$",
  "overlapping alternation identical": "(a|a)+$",
  "overlapping alternation prefix": "(a|ab)+c",
  "overlapping alternation long branch": "(anj|anjing)+",
  "empty alternation branch": "(a|)+b",
  "adjacent overlapping quantifiers": "a*a*b",
  "adjacent overlapping quantifiers class": "[a-z]+[a-z]+",
  "adjacent quantified group": "(a+)(a+)$",
  "quantified group that can match empty": "(a*|b)+",
  "too many unbounded quantifiers": "a*a*a*a*a*a*a*b",
  "backreference": "(a*)\\1",
};

const GOOD = {
  plainword: "anjing",
  alternation: "anjing|goblok|tolol",
  "quantified alternation, disjoint first chars": "(anjing|goblok)+",
  "documented help example": "(anj|anjing|a+n+j+)",
  "word-boundary": "\\banjing\\b",
  "digit range": "[0-9]{4,12}",
  "adjacent disjoint quantifiers": "\\w+\\s*\\w+\\s*\\w+",
  "non-space then space": "\\S+\\s*\\S+",
  "wildcard then multi-char literal": ".*anjing",
  "escaped dot": "free\\.fire",
  "alternation with quantifiers inside, group not repeated": "(anj|anjing|a+n+j+)\\b",
};

describe("AntiCustom — admin-supplied pattern (B4 ReDoS)", () => {
  it("menolak pola berbahaya saat instalasi (nested quantifier)", () => {
    for (const [label, pattern] of Object.entries(BAD)) {
      const result = analyzeCustomPattern(pattern);
      assert.equal(
        result.safe,
        false,
        `pola harus DITOLAK: ${pattern} (${label})`,
      );
      assert.ok(
        typeof result.reason === "string" && result.reason.length > 0,
        `harus ada alasan yang bisa dibaca admin: ${pattern}`,
      );
    }
  });

  it("menerima pola yang memang dipakai admin (termasuk contoh di help plugin)", () => {
    for (const [label, pattern] of Object.entries(GOOD)) {
      const result = analyzeCustomPattern(pattern);
      assert.equal(
        result.safe,
        true,
        `pola harus LOLOS: ${pattern} (${label}) — ${result.reason}`,
      );
    }
  });

  it("batas panjang pola, jumlah kuantor tak terbatas, dan backreference", () => {
    assert.equal(
      analyzeCustomPattern("a".repeat(CUSTOM_MAX_PATTERN_LEN + 1)).safe,
      false,
    );
    assert.equal(analyzeCustomPattern("a".repeat(CUSTOM_MAX_PATTERN_LEN)).safe, true);
    assert.equal(analyzeCustomPattern("(a*)\\1b").safe, false);
  });

  it("reply wizard menolak regex berbahaya dengan pesan jelas", () => {
    const bad = parsePatternAnswer("regex: (a+)+$");
    assert.ok(bad.error, "wizard harus menolak pola berbahaya");
    assert.match(bad.error, /pola|regex|aman/i);

    const good = parsePatternAnswer("regex: (anj|anjing|a+n+j+)");
    assert.equal(good.error, undefined);
    assert.equal(good.type, "regex");
    assert.deepEqual(good.patterns, ["(anj|anjing|a+n+j+)"]);

    const contains = parsePatternAnswer("anjing, goblok, tolol");
    assert.equal(contains.error, undefined);
    assert.equal(contains.type, "contains");
    assert.deepEqual(contains.patterns, ["anjing", "goblok", "tolol"]);
  });

  it("rule contains biasa tetap jalan", () => {
    const rule = { pattern: "Goblok", type: "contains", name: "kotor" };
    assert.equal(matchCustomRule("halo bro ini GOBLOK banget", [rule]), rule);
    assert.equal(matchCustomRule("halo bro ini bersih", [rule]), null);
  });

  it("regex aman tetap jalan, dan flag g tidak lagi membuat hasil bolong-balik", () => {
    const rule = regexRule("anjing|goblok");
    assert.equal(matchCustomRule("kamu anjing", [rule]), rule);
    assert.equal(matchCustomRule("kamu baik", [rule]), null);

    const sticky = regexRule("anjing", { flags: "gi" });
    assert.equal(matchCustomRule("anjing", [sticky]), sticky);
    assert.equal(matchCustomRule("anjing", [sticky]), sticky);
  });

  it("rule regex yang TERSIMPAN sebelum patch tidak lagi membekukan bot", () => {
    // Rule yang lolos validasi lama (hanya new RegExp) dan sudah menempel
    // di DB sebelum patch. Wajib inert, bukan stall.
    const stale = [
      regexRule("(a+)+$"),
      regexRule("(a|a)+$"),
      regexRule("a*a*a*a*a*b"),
    ];
    const text = `${"a".repeat(400)}X`;

    const started = process.hrtime.bigint();
    const hit = matchCustomRule(text, stale);
    const elapsedMs = Number(process.hrtime.bigint() - started) / 1e6;

    assert.equal(hit, null, "rule berbahaya tidak boleh match");
    assert.ok(
      elapsedMs < 50,
      `waktu eval harus tetap kecil, bukan stall: ${elapsedMs.toFixed(2)} ms`,
    );
  });

  it("pathological input tetap murah walau rule-nya seolah-olah sah", () => {
    // Pola "sah" tapi boros: subject dipotong + jumlah rule dibatasi.
    const heavy = Array.from({ length: CUSTOM_MAX_RULES + 200 }, (_, i) =>
      regexRule(`[a-z]+[0-9]+[a-z]+[0-9]+x${i}`),
    );
    const text = "a".repeat(CUSTOM_MAX_SUBJECT_LEN * 4);

    const started = process.hrtime.bigint();
    const hit = matchCustomRule(text, heavy);
    const elapsedMs = Number(process.hrtime.bigint() - started) / 1e6;

    assert.equal(hit, null);
    assert.ok(
      elapsedMs < 50,
      `waktu eval harus tetap kecil: ${elapsedMs.toFixed(2)} ms`,
    );
  });

  it("regex di-cache, tidak dikompilasi ulang tiap pesan", () => {
    const rule = regexRule("anjing|goblok");
    const text = "halo bro";

    const started = process.hrtime.bigint();
    for (let i = 0; i < 2000; i += 1) matchCustomRule(text, [rule]);
    const perCallMs = Number(process.hrtime.bigint() - started) / 1e6 / 2000;

    assert.ok(
      perCallMs < 0.05,
      `per-call harus submikrodetik setelah cache: ${perCallMs.toFixed(5)} ms`,
    );
  });

  it("jumlah rule per pesan dibatasi: rule setelah cap tidak dievaluasi", () => {
    // Padding dengan nol supaya tidak ada rule yang jadi substring rule lain.
    const token = (i) => `zz${String(i).padStart(3, "0")}z`;
    const rules = Array.from({ length: CUSTOM_MAX_RULES + 1 }, (_, i) => ({
      pattern: token(i),
      type: "contains",
      name: `r${i}`,
    }));

    assert.equal(matchCustomRule(`halo ${token(1)}`, rules), rules[1]);
    const beyondCap = rules[CUSTOM_MAX_RULES];
    assert.equal(
      matchCustomRule(`halo ${beyondCap.pattern}`, rules),
      null,
      "rule di luar cap tidak boleh dievaluasi",
    );
  });
});
