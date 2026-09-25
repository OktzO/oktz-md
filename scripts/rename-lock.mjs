import fs from "node:fs";
import { fileURLToPath } from "node:url";

const lockPath = fileURLToPath(new URL("../package-lock.json", import.meta.url));
const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
const rootDependencies = lock.packages[""].dependencies;
const nextRootDependencies = {};

for (const [name, range] of Object.entries(rootDependencies)) {
  if (name === "ourin") {
    nextRootDependencies.onigis = "^10.1.0-rc.6";
  } else {
    nextRootDependencies[name] = range;
  }
}
lock.packages[""].dependencies = nextRootDependencies;

function renamePackageKey(key) {
  return key.replace(/^node_modules\/ourin(?=\/|$)/, "node_modules/onigis");
}

function insertPackageEntry(entries, entry) {
  const index = entries.findIndex(([key]) => key > entry[0]);
  if (index === -1) entries.push(entry);
  else entries.splice(index, 0, entry);
}

const nextPackages = [];
let renamedKeys = 0;
for (const [key, value] of Object.entries(lock.packages)) {
  const nextKey = renamePackageKey(key);
  if (nextKey === key) {
    nextPackages.push([key, value]);
    continue;
  }
  if (nextPackages.some(([existingKey]) => existingKey === nextKey)) {
    throw new Error(`Duplicate lockfile key after rename: ${nextKey}`);
  }
  insertPackageEntry(nextPackages, [nextKey, value]);
  renamedKeys += 1;
}
lock.packages = Object.fromEntries(nextPackages);
fs.writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
console.log(`renamed ${renamedKeys} package keys`);
