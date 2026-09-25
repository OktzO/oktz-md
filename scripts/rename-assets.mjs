import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ASSET_BASENAMES = [
  "ourin-daftar.jpeg",
  "ourin-demote.jpeg",
  "ourin-fishit.jpeg",
  "ourin-games.jpeg",
  "ourin-landscape.jpeg",
  "ourin-levelup.jpeg",
  "ourin-minecraft.jpeg",
  "ourin-promote.jpeg",
  "ourin-rpg.jpeg",
  "ourin-rules.jpeg",
  "ourin-store.jpeg",
  "ourin-v8.jpeg",
  "ourin-winner.jpeg",
  "ourin.jpeg",
  "ourin2.jpeg",
  "ourin3.jpeg",
  "ourin-mp4.mp4",
  "ourin-mp3.mp3",
  "ourin-font.ttf",
  "ourin-kertas.jpeg",
];

function newName(base) {
  if (base.startsWith("ourin-")) return base.slice("ourin-".length);
  return base.replace(/^ourin(?=\d*\.)/, "foto");
}

function buildAssetMap(assetBasenames = ASSET_BASENAMES) {
  const map = new Map();
  for (const base of assetBasenames) {
    if (map.has(base)) throw new Error(`Duplicate asset basename: ${base}`);
    map.set(base, newName(base));
  }
  return map;
}

function stripExtension(base) {
  return base.replace(/\.[a-z0-9]+$/i, "");
}

function buildReplacements(assetMap) {
  const replacements = [];
  for (const [oldBase, newBase] of assetMap) {
    replacements.push({ kind: "basename", oldValue: oldBase, newValue: newBase });
    replacements.push({
      kind: "stem",
      oldValue: stripExtension(oldBase),
      newValue: stripExtension(newBase),
    });
  }
  return replacements.sort((a, b) => b.oldValue.length - a.oldValue.length);
}

const ASSET_MAP = buildAssetMap();
const SCAN_EXT = new Set([".js", ".mjs", ".cjs"]);
const SKIP_FILES = new Set([
  "package.json",
  "package-lock.json",
  "package-lock.yml",
  "README.md",
  "src/data/family100.json",
  "tests/rename-invariants.test.mjs",
  "scripts/rename-assets.mjs",
]);
const SKIP_PREFIXES = [
  ".git/",
  "docs/",
  ".opencode/",
  ".superpowers/",
  "node_modules/",
  "native/",
];
const ASSET_KEY_FILES = new Set([
  "config.js",
  "src/lib/context.js",
  "plugins/user/daftar.js",
]);

function hasReplacementBoundary(source, index, length) {
  const before = source[index - 1] ?? "";
  const after = source[index + length] ?? "";
  return !/[\w.-]/.test(before) && !/[\w.-]/.test(after);
}

function replaceMappedBasenames(source, replacements) {
  const basenames = replacements.filter(({ kind }) => kind === "basename");
  let result = "";
  let cursor = 0;
  let index = 0;
  while (index < source.length) {
    const replacement = basenames.find(({ oldValue }) =>
      source.startsWith(oldValue, index) &&
      hasReplacementBoundary(source, index, oldValue.length)
    );
    if (!replacement) {
      index += 1;
      continue;
    }
    result += source.slice(cursor, index) + replacement.newValue;
    index += replacement.oldValue.length;
    cursor = index;
  }
  return result + source.slice(cursor);
}

function replaceQuotedMappedTokens(fragment, tokenMap) {
  return fragment.replace(/(["'`])ourin([^"'`]*)\1/g, (match, quote, rest) => {
    const oldValue = `ourin${rest}`;
    const newValue = tokenMap.get(oldValue);
    return newValue ? `${quote}${newValue}${quote}` : match;
  });
}

function rewriteCall(source, name, tokenMap) {
  const pattern = new RegExp(`${name}\\s*\\([^)]*\\)`, "g");
  return source.replace(pattern, (call) => replaceQuotedMappedTokens(call, tokenMap));
}

export function rewriteSource(source, file, assetMap = ASSET_MAP) {
  const replacements = buildReplacements(assetMap);
  const tokenMap = new Map(
    replacements
      .filter(({ kind }) => kind === "stem")
      .map(({ oldValue, newValue }) => [oldValue, newValue]),
  );
  let result = replaceMappedBasenames(source, replacements);
  if (ASSET_KEY_FILES.has(file)) {
    result = replaceQuotedMappedTokens(result, tokenMap);
  }
  result = rewriteCall(result, "getAssetBuffer", tokenMap);
  result = rewriteCall(result, "updateAssetUrl", tokenMap);
  result = result.replace(
    /(\.assets\s*\[\s*)(["'`])([^"'`]*)\2/g,
    (match, prefix, quote, value) => {
      const newValue = tokenMap.get(value);
      return newValue ? `${prefix}${quote}${newValue}${quote}` : match;
    },
  );
  return result;
}

function buildMoves(assetMap) {
  const assetFiles = execSync('git ls-files "assets"', { encoding: "utf8" })
    .trim()
    .split("\n")
    .filter(Boolean);
  const moves = new Map();
  for (const oldBase of assetMap.keys()) {
    const candidates = assetFiles.filter((file) => path.basename(file) === oldBase);
    if (candidates.length > 1) {
      throw new Error(`Ambiguous asset basename: ${oldBase}`);
    }
    if (candidates.length === 0) continue;
    const oldPath = candidates[0];
    const newPath = path.join(path.dirname(oldPath), assetMap.get(oldBase));
    if (fs.existsSync(newPath)) {
      throw new Error(`COLLISION: ${oldPath} -> ${newPath}`);
    }
    moves.set(oldPath, newPath);
  }
  return moves;
}

function collectTargets() {
  return execSync("git ls-files", { encoding: "utf8" })
    .trim()
    .split("\n")
    .filter((file) => {
      if (!file || !SCAN_EXT.has(path.extname(file))) return false;
      if (SKIP_FILES.has(file) || SKIP_FILES.has(path.basename(file))) return false;
      return !SKIP_PREFIXES.some((prefix) => file.startsWith(prefix));
    });
}

export function runMigration({ dryRun = false, root = process.cwd() } = {}) {
  const moves = buildMoves(ASSET_MAP);
  console.log(`peta aset: ${ASSET_MAP.size}`);
  let moved = 0;
  for (const [oldPath, newPath] of moves) {
    if (!dryRun) execFileSync("git", ["mv", oldPath, newPath], { cwd: root });
    moved += 1;
  }

  let rewritten = 0;
  for (const file of collectTargets()) {
    const source = fs.readFileSync(path.join(root, file), "utf8");
    const result = rewriteSource(source, file);
    if (result === source) continue;
    if (!dryRun) fs.writeFileSync(path.join(root, file), result);
    rewritten += 1;
  }

  const action = dryRun ? "akan di-rewrite" : "di-rewrite";
  console.log(`aset ${action} di ${rewritten} file`);
  console.log(`aset di-rename: ${dryRun ? "akan dijalankan" : "selesai"} (${moved})`);
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runMigration({ dryRun: process.argv.includes("--dry-run") });
}
