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
const ASSET_KEY_RENAMES = new Map([
  ["ourin-daftar", "daftar"],
  ["ourin-demote", "demote"],
  ["ourin-fishit", "fishit"],
  ["ourin-games", "games"],
  ["ourin-landscape", "landscape"],
  ["ourin-levelup", "levelup"],
  ["ourin-minecraft", "minecraft"],
  ["ourin-promote", "promote"],
  ["ourin-rpg", "rpg"],
  ["ourin-rules", "rules"],
  ["ourin-store", "store"],
  ["ourin-v8", "v8"],
  ["ourin-winner", "winner"],
  ["ourin", "foto"],
  ["ourin2", "foto2"],
  ["ourin3", "foto3"],
  ["pp-kosong", "pp-kosong"],
  ["ourin-mp4", "mp4"],
  ["ourin-mp3", "mp3"],
  ["ourin-font", "font"],
  ["ourin-kertas", "kertas"],
]);
const EXPECTED_ASSET_KEYS = new Set([
  "daftar",
  "demote",
  "fishit",
  "games",
  "kertas",
  "landscape",
  "levelup",
  "minecraft",
  "promote",
  "rpg",
  "rules",
  "store",
  "v8",
  "winner",
  "foto",
  "foto2",
  "foto3",
  "font",
  "mp4",
  "mp3",
  "pp-kosong",
]);
const EXPECTED_ASSET_KEY_LIST = [...EXPECTED_ASSET_KEYS].sort();

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

function stripComments(source) {
  let output = "";
  let state = "code";
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    const next = source[index + 1];
    if (state === "code") {
      if (character === "/" && next === "/") {
        output += "  ";
        index += 1;
        state = "line";
      } else if (character === "/" && next === "*") {
        output += "  ";
        index += 1;
        state = "block";
      } else {
        output += character;
        if (character === "\"" || character === "'" || character === "`") {
          state = character;
        }
      }
    } else if (state === "line") {
      if (character === "\n" || character === "\r") {
        output += character;
        state = "code";
      } else {
        output += " ";
      }
    } else if (state === "block") {
      if (character === "*" && next === "/") {
        output += "  ";
        index += 1;
        state = "code";
      } else if (character === "\n" || character === "\r") {
        output += character;
      } else {
        output += " ";
      }
    } else {
      output += character;
      if (character === "\\" && next !== undefined) {
        output += next;
        index += 1;
      } else if (character === state) {
        state = "code";
      }
    }
  }
  return output;
}

function skipTrivia(source, index) {
  while (index < source.length) {
    if (/\s/u.test(source[index])) {
      index += 1;
    } else if (source.startsWith("//", index)) {
      const end = source.indexOf("\n", index + 2);
      index = end === -1 ? source.length : end;
    } else if (source.startsWith("/*", index)) {
      const end = source.indexOf("*/", index + 2);
      index = end === -1 ? source.length : end + 2;
    } else {
      break;
    }
  }
  return index;
}

function readQuotedSpecifier(source, index) {
  const quote = source[index];
  let value = "";
  for (let cursor = index + 1; cursor < source.length; cursor += 1) {
    const character = source[cursor];
    if (character === "\\") {
      value += source[cursor + 1] ?? "";
      cursor += 1;
    } else if (character === quote) {
      return { value, end: cursor + 1 };
    } else {
      value += character;
    }
  }
  return null;
}

function isWordCharacter(character) {
  return character !== undefined && /[A-Za-z0-9_$]/.test(character);
}

function importSpecifiers(source) {
  const code = stripComments(source);
  const matches = [];
  for (let index = 0; index < code.length;) {
    const character = code[index];
    if (character === "\"" || character === "'" || character === "`") {
      const quoted = readQuotedSpecifier(code, index);
      index = quoted ? quoted.end : index + 1;
      continue;
    }
    if (!isWordCharacter(character)) {
      index += 1;
      continue;
    }
    let end = index + 1;
    while (end < code.length && isWordCharacter(code[end])) end += 1;
    const word = code.slice(index, end);
    if (word === "import" || word === "from") {
      let cursor = skipTrivia(code, end);
      if (word === "import" && code[cursor] === "(") {
        cursor = skipTrivia(code, cursor + 1);
      }
      if (code[cursor] === "\"" || code[cursor] === "'") {
        const specifier = readQuotedSpecifier(code, cursor);
        if (specifier) {
          matches.push(specifier.value);
          index = specifier.end;
          continue;
        }
      }
    }
    index = end;
  }
  return matches;
}

function isRegularFile(file) {
  try {
    return Boolean(file) && fs.existsSync(file) && fs.statSync(file).isFile();
  } catch {
    return false;
  }
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

function stripWhitelistedOurin(line) {
  return line
    .replace(/ourin-native/gi, "")
    .replace(/ourin_native(?:\.[a-z0-9_-]+)*/gi, "")
    .replace(/native[\\/][^\s"'`)]*ourin[^\s"'`)]*/gi, "")
    .replace(/ourin[\\/][^\s"'`)]*native[^\s"'`)]*/gi, "");
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
        if (!relativeImportCandidates(file, specifier).some((candidate) => isRegularFile(candidate))) {
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
    const assets = config.assets ?? {};
    const actualKeys = Object.keys(assets);
    assert.ok(actualKeys.includes("pp-kosong"), "kunci asset pp-kosong harus tetap ada");
    assert.equal(actualKeys.length, EXPECTED_ASSET_KEYS.size, "jumlah kunci config.assets harus tepat");
    const normalizedKeys = actualKeys
      .map((key) => ASSET_KEY_RENAMES.get(key) ?? key)
      .sort();
    assert.deepStrictEqual(
      normalizedKeys,
      EXPECTED_ASSET_KEY_LIST,
      "kunci config.assets harus sesuai peta rename",
    );
    const missing = [];
    for (const [key, value] of Object.entries(assets)) {
      if (typeof value === "string" && /^https?:\/\//i.test(value)) continue;
      const target = typeof value === "string" ? path.resolve(ROOT, value) : "";
      if (!isRegularFile(target)) missing.push(`${key} -> ${String(value)}`);
    }
    assert.deepStrictEqual(missing, [], `aset hilang: ${missing.join("\n")}`);
  });

  it("I5: tidak ada sisa referensi 'ourin' di luar whitelist", () => {
    const hits = [];
    for (const file of FILES) {
      if (file === TEST_FILE || isWhitelisted(file)) continue;
      const lines = read(file).split("\n");
      lines.forEach((line, index) => {
        const residual = stripWhitelistedOurin(line);
        if (/ourin/i.test(residual)) {
          hits.push(`${file}:${index + 1}: ${line.trim().slice(0, 90)}`);
        }
      });
    }
    assert.deepStrictEqual(hits, [], `sisa referensi ourin:\n${hits.slice(0, 40).join("\n")}`);
  });
});
