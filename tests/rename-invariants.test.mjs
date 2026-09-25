import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const ROOT = process.cwd();
const SCAN_DIRS = ["src", "plugins", "tests", "data", "case"];
const SCAN_EXT = new Set([".js", ".mjs", ".cjs"]);
const TEST_FILE = "tests/rename-invariants.test.mjs";
const WHITELIST = new Set([
  "src/data/family100.json",
  "native/fetch-prebuilt.mjs",
  "native/index.cjs",
  "native/Cargo.toml",
  "native/build.rs",
  "native/build-prebuilt.sh",
]);
const GUARDED_MISSING_IMPORTS = new Set([
  "src/handler.js -> ./lib/ourin-sticker-reply.js",
  "src/handler.js -> ./lib/sticker-reply.js",
]);

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(path.join(ROOT, dir), { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(file, out);
    } else if (SCAN_EXT.has(path.extname(entry.name))) {
      out.push(file);
    }
  }
  return out;
}

const FILES = [
  ...SCAN_DIRS.flatMap((dir) =>
    fs.existsSync(path.join(ROOT, dir)) ? walk(dir) : [],
  ),
  ...["index.js", "_test.mjs"].filter((file) =>
    fs.existsSync(path.join(ROOT, file)),
  ),
];

function read(file) {
  return fs.readFileSync(path.join(ROOT, file), "utf8");
}

function importSpecifiers(source) {
  const matches = [];
  const re = /\b(?:from|import)\s*(?:\(\s*)?(["'])([^"']+)\1/g;
  let match;
  while ((match = re.exec(source))) matches.push(match[2]);
  return matches;
}

function relativeImportCandidates(file, specifier) {
  const base = path.resolve(ROOT, path.dirname(file), specifier);
  return [
    base,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.cjs`,
    path.join(base, "index.js"),
  ];
}

function isWhitelisted(file) {
  return WHITELIST.has(file) || file.startsWith("native/");
}

describe("rename invariants", () => {
  it("I1: semua import relatif resolve ke file yang ada", () => {
    const bad = [];
    for (const file of FILES) {
      for (const specifier of importSpecifiers(read(file))) {
        if (!specifier.startsWith(".")) continue;
        const key = `${file} -> ${specifier}`;
        if (GUARDED_MISSING_IMPORTS.has(key)) continue;
        if (!relativeImportCandidates(file, specifier).some((candidate) => fs.existsSync(candidate))) {
          bad.push(key);
        }
      }
    }
    assert.deepStrictEqual(bad, [], `import rusak:\n${bad.join("\n")}`);
  });

  it("I2: tidak ada lagi src/lib/ourin-*.js", () => {
    const left = fs
      .readdirSync(path.join(ROOT, "src/lib"), { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.startsWith("ourin-"))
      .map((entry) => entry.name)
      .sort();
    assert.deepStrictEqual(left, [], `masih ada: ${left.join(", ")}`);
  });

  it("I3: tidak ada import dari specifier 'ourin'", () => {
    const bad = [];
    for (const file of FILES) {
      if (file === TEST_FILE) continue;
      if (importSpecifiers(read(file)).some((specifier) => specifier === "ourin")) {
        bad.push(file);
      }
    }
    assert.deepStrictEqual(bad, [], `masih import 'ourin': ${bad.join(", ")}`);
  });

  it("I4: setiap kunci config.assets punya file aset yang ada", async () => {
    const config = (await import(pathToFileURL(path.join(ROOT, "config.js")).href)).default;
    const missing = [];
    for (const [key, value] of Object.entries(config.assets ?? {})) {
      if (typeof value === "string" && /^https?:\/\//i.test(value)) continue;
      const target = typeof value === "string" ? path.resolve(ROOT, value) : "";
      if (!target || !fs.existsSync(target)) missing.push(`${key} -> ${String(value)}`);
    }
    assert.deepStrictEqual(missing, [], `aset hilang: ${missing.join("\n")}`);
  });

  it("I5: tidak ada sisa referensi 'ourin' di luar whitelist", () => {
    const hits = [];
    for (const file of FILES) {
      if (file === TEST_FILE || isWhitelisted(file)) continue;
      const lines = read(file).split("\n");
      lines.forEach((line, index) => {
        if (/ourin/i.test(line) && !/ourin-native/i.test(line)) {
          hits.push(`${file}:${index + 1}: ${line.trim().slice(0, 90)}`);
        }
      });
    }
    assert.deepStrictEqual(hits, [], `sisa referensi ourin:\n${hits.slice(0, 40).join("\n")}`);
  });
});
