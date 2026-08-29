#!/usr/bin/env node
// Fetch prebuilt ourin-native binary from GitHub Releases when missing locally.
// Only fetches the arch/libc that actually runs (panel is gnu today);
// non-fatal by design so tesseract.js fallback in src/lib/ourin-native-loader.js
// takes over if network is unavailable.
import { existsSync, mkdirSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = process.env.OURIN_NATIVE_VERSION || "v0.1.0";
const REPO = "OktzO/ourin-md";
const HERE = dirname(fileURLToPath(import.meta.url));
const PLATFORMS_DIR = join(HERE, "platforms");

const isMusl = existsSync("/etc/alpine-release");
const TARGET = isMusl
  ? { asset: "ourin_native.linux-x64-musl.node", sha256: "c9cecda5e9ad69c70ea77b37a40009a4808fed7c15fe73f63d8a58a6e4ee3d5e", dest: join(PLATFORMS_DIR, "x86_64-unknown-linux-musl", "ourin_native.linux-x64-musl.node") }
  : { asset: "ourin_native.linux-x64-gnu.node", sha256: "b173ecfff504f370dfed9177f98a81580a9dd9b4dd3c782175074d26a52bcae3", dest: join(PLATFORMS_DIR, "x86_64-unknown-linux-gnu", "ourin_native.linux-x64-gnu.node") };

async function fetchOne({ asset, dest, sha256 }) {
  if (existsSync(dest)) return { asset, cached: true };
  const url = `https://github.com/${REPO}/releases/download/${VERSION}/${asset}`;
  let lastErr;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      const res = await fetch(url, { redirect: "follow", signal: AbortSignal.timeout(120_000) });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = Buffer.from(await res.arrayBuffer());
      const actual = createHash("sha256").update(buf).digest("hex");
      if (actual !== sha256) throw new Error(`sha256 mismatch (${actual})`);
      mkdirSync(dirname(dest), { recursive: true });
      const tmp = dest + ".tmp";
      try { unlinkSync(tmp); } catch {}
      writeFileSync(tmp, buf);
      renameSync(tmp, dest);
      return { asset, size: buf.length };
    } catch (e) {
      lastErr = e;
      if (attempt < 3) await new Promise((r) => setTimeout(r, 2000 * attempt));
    }
  }
  throw lastErr;
}

try {
  const r = await fetchOne(TARGET);
  console.log(`[ourin-native] ${r.cached ? r.asset + " (cached)" : `fetched ${r.asset} ${(r.size / 1048576).toFixed(1)}MB`}`);
} catch (e) {
  console.warn(`[ourin-native] fetch gagal (${e.message}) — fallback JS aktif sampai binary tersedia`);
}
