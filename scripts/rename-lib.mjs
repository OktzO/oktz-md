import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SCAN_DIRS = ["src", "plugins", "tests", "data", "case"];
const SCAN_EXT = new Set([".js", ".mjs", ".cjs"]);

const libFiles = execSync('git ls-files "src/lib/*.js"', { encoding: "utf8" })
  .trim()
  .split("\n")
  .filter(Boolean);

const map = new Map();
for (const file of libFiles) {
  const base = path.basename(file);
  if (!base.startsWith("ourin-")) continue;
  const next = base.slice("ourin-".length);
  const target = path.join(path.dirname(file), next);
  if (fs.existsSync(target)) {
    console.error(`COLLISION: ${file} -> ${target} sudah ada`);
    process.exit(1);
  }
  map.set(base, next);
}
console.log(`peta: ${map.size} file (collision: 0)`);

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(path.join(ROOT, dir), { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(file, out);
    else if (SCAN_EXT.has(path.extname(entry.name))) out.push(file);
  }
  return out;
}

function collectScanFiles() {
  return [
    ...SCAN_DIRS.flatMap((dir) =>
      fs.existsSync(path.join(ROOT, dir)) ? walk(dir) : [],
    ),
    ...["config.js", "index.js", "_test.mjs"].filter((file) =>
      fs.existsSync(path.join(ROOT, file)),
    ),
  ];
}

const dryRun = process.argv.includes("--dry-run");
let moved = 0;
let rewritten = 0;
for (const [oldBase, newBase] of map) {
  const oldPath = path.join("src/lib", oldBase);
  const newPath = path.join("src/lib", newBase);
  if (!fs.existsSync(path.join(ROOT, oldPath))) continue;
  if (!dryRun) {
    execFileSync("git", ["mv", oldPath, newPath], { cwd: ROOT });
  }
  moved += 1;
}

const files = collectScanFiles();
for (const file of files) {
  if (dryRun) continue;
  const abs = path.join(ROOT, file);
  const source = fs.readFileSync(abs, "utf8");
  let rewrittenSource = source;
  for (const [oldBase, newBase] of map) {
    if (rewrittenSource.includes(oldBase)) {
      rewrittenSource = rewrittenSource.split(oldBase).join(newBase);
    }
  }
  if (rewrittenSource !== source) {
    fs.writeFileSync(abs, rewrittenSource);
    rewritten += 1;
  }
}
console.log(`renamed: ${moved}, rewrite import: ${rewritten} file`);
