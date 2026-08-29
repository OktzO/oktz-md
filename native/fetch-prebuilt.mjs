#!/usr/bin/env node
// Fetch prebuilt ourin-native binaries from GitHub Releases when they are
// missing locally (production panels never compile Rust).
// Non-fatal by design: if anything fails, the JS fallback in
// src/lib/ourin-native-loader.js still works.
import { existsSync, mkdirSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = process.env.OURIN_NATIVE_VERSION || "v0.1.0";
const REPO = "OktzO/ourin-md";
const HERE = dirname(fileURLToPath(import.meta.url));
const PLATFORMS_DIR = join(HERE, "platforms");

const FILES = [
  {
    asset: "ourin_native.linux-x64-gnu.node",
    dest: join(PLATFORMS_DIR, "x86_64-unknown-linux-gnu", "ourin_native.linux-x64-gnu.node"),
  },
  {
    asset: "ourin_native.linux-x64-musl.node",
    dest: join(PLATFORMS_DIR, "x86_64-unknown-linux-musl", "ourin_native.linux-x64-musl.node"),
  },
];

async function fetchOne({ asset, dest }) {
  if (existsSync(dest)) return { asset, skipped: true };
  const url = `https://github.com/${REPO}/releases/download/${VERSION}/${asset}`;
  const res = await fetch(url, { redirect: "follow", signal: AbortSignal.timeout(120_000) });
  if (!res.ok) throw new Error(`${asset}: HTTP ${res.status}`);
  const tmp = dest + ".tmp";
  mkdirSync(dirname(dest), { recursive: true });
  const buf = Buffer.from(await res.arrayBuffer());
  try { unlinkSync(tmp); } catch {}
  writeFileSync(tmp, buf);
  renameSync(tmp, dest);
  return { asset, size: buf.length };
}

const results = [];
for (const f of FILES) {
  try {
    results.push(await fetchOne(f));
  } catch (e) {
    // Non-fatal: fallback JS tetap jalan jika binary tidak tersedia.
    console.warn(`[ourin-native] fetch gagal (${e.message}) — fallback JS aktif sampai binary tersedia`);
    process.exit(0);
  }
}
const parts = results.map((r) => (r.skipped ? `${r.asset} (cached)` : `${r.asset} ${(r.size / 1048576).toFixed(1)}MB`));
console.log(`[ourin-native] fetched: ${parts.join(", ")}`);
