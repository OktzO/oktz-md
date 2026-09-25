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

const REGEX_PREFIX_KEYWORDS = new Set([
  "return",
  "typeof",
  "case",
  "in",
  "of",
  "new",
  "delete",
  "void",
  "instanceof",
  "do",
  "else",
  "yield",
  "await",
  "throw",
  "default",
  "extends",
]);

function isIdentifierCharacter(character) {
  return character !== undefined && /[A-Za-z0-9_$]/.test(character);
}

function scanJavaScriptTokens(source) {
  const tokens = [];
  let index = 0;
  let lastToken = null;

  function addToken(type, value) {
    const token = { type, value };
    tokens.push(token);
    lastToken = token;
  }

  function rememberValue() {
    lastToken = { type: "value", value: "" };
  }

  function skipLineComment() {
    index += 2;
    while (index < source.length && source[index] !== "\n" && source[index] !== "\r") {
      index += 1;
    }
  }

  function skipBlockComment() {
    index += 2;
    while (index < source.length) {
      if (source[index] === "*" && source[index + 1] === "/") {
        index += 2;
        return;
      }
      index += 1;
    }
  }

  function scanQuotedString(quote) {
    index += 1;
    let value = "";
    while (index < source.length) {
      const character = source[index];
      if (character === "\\") {
        value += source[index + 1] ?? "";
        index += 2;
      } else if (character === quote) {
        index += 1;
        addToken("string", value);
        return;
      } else {
        value += character;
        index += 1;
      }
    }
    addToken("string", value);
  }

  function canStartRegex() {
    if (!lastToken) return true;
    if (lastToken.type === "identifier") return REGEX_PREFIX_KEYWORDS.has(lastToken.value);
    if (lastToken.type === "value") return false;
    return ![")", "]", "}"].includes(lastToken.value);
  }

  function scanRegexLiteral() {
    const start = index;
    index += 1;
    let inCharacterClass = false;
    while (index < source.length) {
      const character = source[index];
      if (character === "\\") {
        index += 2;
        continue;
      }
      if (character === "\n" || character === "\r") break;
      if (character === "[") {
        inCharacterClass = true;
      } else if (character === "]") {
        inCharacterClass = false;
      } else if (character === "/" && !inCharacterClass) {
        index += 1;
        while (index < source.length && isIdentifierCharacter(source[index])) index += 1;
        rememberValue();
        return true;
      }
      index += 1;
    }
    index = start;
    return false;
  }

  function scanTemplate() {
    index += 1;
    while (index < source.length) {
      const character = source[index];
      if (character === "\\") {
        index += 2;
        continue;
      }
      if (character === "`") {
        index += 1;
        rememberValue();
        return;
      }
      if (character === "$" && source[index + 1] === "{") {
        index += 2;
        scanCode("}");
        if (source[index] === "}") index += 1;
        continue;
      }
      index += 1;
    }
    rememberValue();
  }

  function scanCode(endCharacter = null) {
    let braceDepth = 0;
    while (index < source.length) {
      const character = source[index];
      const next = source[index + 1];
      if (endCharacter === "}" && character === "}" && braceDepth === 0) return;
      if (/\s/u.test(character)) {
        index += 1;
        continue;
      }
      if (character === "/" && next === "/") {
        skipLineComment();
        continue;
      }
      if (character === "/" && next === "*") {
        skipBlockComment();
        continue;
      }
      if (character === "\"" || character === "'") {
        scanQuotedString(character);
        continue;
      }
      if (character === "`") {
        scanTemplate();
        continue;
      }
      if (character === "/" && canStartRegex() && scanRegexLiteral()) continue;
      if (isIdentifierCharacter(character)) {
        const start = index;
        index += 1;
        while (index < source.length && isIdentifierCharacter(source[index])) index += 1;
        addToken("identifier", source.slice(start, index));
        continue;
      }
      if (character === "{") braceDepth += 1;
      if (character === "}" && braceDepth > 0) braceDepth -= 1;
      addToken("punctuation", character);
      index += 1;
    }
  }

  scanCode();
  return tokens;
}

function importSpecifiers(source) {
  const tokens = scanJavaScriptTokens(source);
  const matches = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.type !== "identifier") continue;
    if (token.value !== "import" && token.value !== "from") continue;
    let next = index + 1;
    if (token.value === "import" && tokens[next]?.value === "(") next += 1;
    if (tokens[next]?.type === "string") matches.push(tokens[next].value);
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
