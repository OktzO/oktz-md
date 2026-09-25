import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

let espree = null;
let espreeLoadError = null;
try {
  espree = await import("espree");
} catch (error) {
  espreeLoadError = error;
}
if (espree && typeof espree.parse !== "function") {
  espreeLoadError = new Error("espree does not export parse");
}
const ESPEE_UNAVAILABLE_MESSAGE =
  'espree tidak termuat → 5 invariant rename GAGAL (tidak dilewati): parser adalah syarat mutlak, ' +
  'jadi safety net aktif tetapi hasilnya tidak bisa dipercaya. "espree" adalah devDependency yang ' +
  'dideklarasikan di package.json, jadi instalasi node_modules tidak lengkap — jalankan ' +
  '"npm install" (atau "npm ci") lalu ulangi "npm test". ' +
  `Detail: ${espreeLoadError ? espreeLoadError.message : "espree tidak mengekspor parse"}`;

function assertEspreeAvailable() {
  if (espreeLoadError) throw new Error(ESPEE_UNAVAILABLE_MESSAGE);
}

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

const PARSE_OPTIONS = {
  ecmaVersion: "latest",
  sourceType: "module",
  loc: false,
  range: false,
  comment: false,
  ecmaFeatures: { jsx: false },
};
const TEMPLATE_OURIN_IMPORT_PATTERN =
  /\b(?:import\s*\(\s*(?:"ourin"|'ourin')|from\s+(?:"ourin"|'ourin'))/;
const TEMPLATE_EXPRESSION_PLACEHOLDER = "__RENAME_INVARIANT_EXPR__";

function importSourceValue(node) {
  if (!node || typeof node !== "object") return null;
  if (node.type === "Literal") return typeof node.value === "string" ? node.value : null;
  if (node.type !== "TemplateLiteral") return null;
  if (node.expressions.length !== 0 || node.quasis.length !== 1) return null;
  const cooked = node.quasis[0]?.value?.cooked;
  return typeof cooked === "string" ? cooked : null;
}

function collectModuleSpecifiers(ast) {
  const matches = [];
  const seen = new WeakSet();
  function addSource(node) {
    const value = importSourceValue(node);
    if (value !== null) matches.push(value);
  }
  const pending = [ast];
  while (pending.length > 0) {
    const node = pending.pop();
    if (!node || typeof node !== "object" || seen.has(node)) continue;
    seen.add(node);
    if (node.type === "ImportDeclaration") addSource(node.source);
    if (
      (node.type === "ExportNamedDeclaration" || node.type === "ExportAllDeclaration") &&
      node.source
    ) {
      addSource(node.source);
    }
    if (node.type === "ImportExpression") addSource(node.source);
    const children = [];
    for (const value of Object.values(node)) {
      if (Array.isArray(value)) {
        for (const child of value) {
          if (child && typeof child === "object") children.push(child);
        }
      } else if (value && typeof value === "object") {
        children.push(value);
      }
    }
    for (let index = children.length - 1; index >= 0; index -= 1) {
      pending.push(children[index]);
    }
  }
  return matches;
}

function templateLiteralHasOurinImport(ast) {
  const pending = [ast];
  const seen = new WeakSet();
  while (pending.length > 0) {
    const node = pending.pop();
    if (!node || typeof node !== "object" || seen.has(node)) continue;
    seen.add(node);
    if (node.type === "TemplateLiteral") {
      const source = node.quasis
        .map((quasi) => quasi.value.raw)
        .join(TEMPLATE_EXPRESSION_PLACEHOLDER);
      if (TEMPLATE_OURIN_IMPORT_PATTERN.test(source)) {
        const candidates = [
          source,
          `async function __renameInvariantTemplate() {\n${source}\n}`,
        ];
        for (const candidate of candidates) {
          try {
            const parsed = parseSource(candidate);
            if (collectModuleSpecifiers(parsed).includes("ourin")) return true;
          } catch {
            continue;
          }
        }
      }
    }
    const children = [];
    for (const value of Object.values(node)) {
      if (Array.isArray(value)) {
        for (const child of value) {
          if (child && typeof child === "object") children.push(child);
        }
      } else if (value && typeof value === "object") {
        children.push(value);
      }
    }
    for (let index = children.length - 1; index >= 0; index -= 1) {
      pending.push(children[index]);
    }
  }
  return false;
}

function parseSource(source) {
  return espree.parse(source, PARSE_OPTIONS);
}

const MODULE_SPECIFIERS = new Map();
const TEMPLATE_OURIN_IMPORT_FILES = new Set();
const PARSE_ERRORS = [];
if (espree) {
  for (const file of FILES) {
    try {
      const source = read(file);
      const ast = parseSource(source);
      MODULE_SPECIFIERS.set(file, collectModuleSpecifiers(ast));
      if (templateLiteralHasOurinImport(ast)) {
        TEMPLATE_OURIN_IMPORT_FILES.add(file);
      }
    } catch (error) {
      PARSE_ERRORS.push({ file, error });
    }
  }
}

function importSpecifiers(file) {
  return MODULE_SPECIFIERS.get(file) ?? [];
}

function parseErrorText({ file, error }) {
  return `${file}: ${error instanceof Error ? error.message : String(error)}`;
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
    .replace(/(^|[^a-z0-9.-])api\.ourin\.my\.id(?=$|[^a-z0-9.-])/gi, "$1")
    .replace(/(^|[^a-z0-9.-])ourin\.my\.id(?=$|[^a-z0-9.-])/gi, "$1")
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
    assertEspreeAvailable();
    assert.deepStrictEqual(
      PARSE_ERRORS.map(parseErrorText),
      [],
      `file gagal diparse:\n${PARSE_ERRORS.map(parseErrorText).join("\n")}`,
    );
    const bad = [];
    for (const file of FILES) {
      for (const specifier of importSpecifiers(file)) {
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
    assertEspreeAvailable();
    const left = fs
      .readdirSync(path.join(ROOT, "src/lib"), { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.startsWith("ourin-"))
      .map((entry) => entry.name)
      .sort();
    assert.deepStrictEqual(left, [], `masih ada: ${left.join(", ")}`);
  });

  it("I3: tidak ada import dari specifier 'ourin'", () => {
    assertEspreeAvailable();
    const bad = [];
    for (const file of FILES) {
      if (file === TEST_FILE) continue;
      if (
        importSpecifiers(file).some((specifier) => specifier === "ourin") ||
        TEMPLATE_OURIN_IMPORT_FILES.has(file)
      ) {
        bad.push(file);
      }
    }
    assert.deepStrictEqual(bad, [], `masih import 'ourin': ${bad.join(", ")}`);
  });

  it("I4: setiap kunci config.assets punya file aset yang ada", async () => {
    assertEspreeAvailable();
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
    assertEspreeAvailable();
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
