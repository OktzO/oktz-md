import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const dryRun = process.argv.includes("--dry-run");
const assetFiles = execSync('git ls-files "assets"', { encoding: "utf8" })
  .trim()
  .split("\n")
  .filter((file) => file && path.basename(file).startsWith("ourin"));

function newName(base) {
  if (base.startsWith("ourin-")) return base.slice("ourin-".length);
  return base.replace(/^ourin(?=\d*\.)/, "foto");
}

const map = new Map();
const moves = new Map();
for (const file of assetFiles) {
  const base = path.basename(file);
  const next = newName(base);
  const target = path.join(path.dirname(file), next);
  if (fs.existsSync(path.join(ROOT, target))) {
    console.error(`COLLISION: ${file} -> ${target}`);
    process.exit(1);
  }
  map.set(base, next);
  moves.set(file, target);
}
console.log(`peta aset: ${map.size}`);

const SCAN_EXT = new Set([".js", ".mjs", ".cjs"]);
const SKIP_FILES = new Set([
  "package.json",
  "package-lock.json",
  "package-lock.yml",
  "README.md",
  "tests/rename-invariants.test.mjs",
]);
const SKIP_PREFIXES = [
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

const targets = execSync("git ls-files", { encoding: "utf8" })
  .trim()
  .split("\n")
  .filter((file) => {
    if (!file || !SCAN_EXT.has(path.extname(file))) return false;
    if (SKIP_FILES.has(file) || SKIP_FILES.has(path.basename(file))) return false;
    return !SKIP_PREFIXES.some((prefix) => file.startsWith(prefix));
  });

function renameStem(value) {
  if (value.startsWith("ourin-")) return value.slice("ourin-".length);
  return value.replace(/^ourin(?=\d*(?:\.|$))/, "foto");
}

function renameQuotedAssetStrings(fragment) {
  return fragment.replace(/(["'`])ourin([^"'`]*)\1/g, (match, quote, rest) => {
    const oldValue = `ourin${rest}`;
    const newValue = renameStem(oldValue);
    return newValue === oldValue ? match : `${quote}${newValue}${quote}`;
  });
}

function rewriteCall(source, name) {
  const pattern = new RegExp(`${name}\\s*\\([^)]*\\)`, "g");
  return source.replace(pattern, (call) => renameQuotedAssetStrings(call));
}

function rewriteSource(source, file) {
  let result = source;
  for (const [oldBase, newBase] of [...map].sort((a, b) => b[0].length - a[0].length)) {
    result = result.split(oldBase).join(newBase);
  }
  if (ASSET_KEY_FILES.has(file)) {
    for (const oldBase of map.keys()) {
      const oldStem = oldBase.replace(/\.[a-z0-9]+$/i, "");
      const newStem = newName(oldBase).replace(/\.[a-z0-9]+$/i, "");
      result = result.split(`"${oldStem}"`).join(`"${newStem}"`);
      result = result.split(`'${oldStem}'`).join(`'${newStem}'`);
    }
  }
  result = rewriteCall(result, "getAssetBuffer");
  result = rewriteCall(result, "updateAssetUrl");
  result = result.replace(
    /(\.assets\s*\[\s*)(["'`])ourin([^"'`]*)\2/g,
    (match, prefix, quote, rest) => `${prefix}${quote}${renameStem(`ourin${rest}`)}${quote}`,
  );
  result = result.replace(
    /(^|[^A-Za-z0-9_-])ourin-([A-Za-z0-9-]+\.jpeg)\b/g,
    (match, prefix, base) => `${prefix}${base}`,
  );
  result = result.replace(
    /(assets[\\/](?:[^\\/"'`\s()]*[\\/])?)ourin-/g,
    "$1",
  );
  result = result.replace(
    /(^|[^A-Za-z0-9_-])ourin(\d*\.(?:jpeg|jpg|png|gif|webp|mp3|mp4|ttf|woff2?))\b/g,
    (match, prefix, suffix) => `${prefix}foto${suffix}`,
  );
  return result;
}

let moved = 0;
for (const [oldPath, newPath] of moves) {
  if (!dryRun) execFileSync("git", ["mv", oldPath, newPath], { cwd: ROOT });
  moved += 1;
}

let rewritten = 0;
for (const file of targets) {
  const abs = path.join(ROOT, file);
  const source = fs.readFileSync(abs, "utf8");
  const result = rewriteSource(source, file);
  if (result === source) continue;
  if (!dryRun) fs.writeFileSync(abs, result);
  rewritten += 1;
}

const action = dryRun ? "akan di-rewrite" : "di-rewrite";
console.log(`aset ${action} di ${rewritten} file`);
console.log(`aset di-rename: ${dryRun ? "akan dijalankan" : "selesai"} (${moved})`);
