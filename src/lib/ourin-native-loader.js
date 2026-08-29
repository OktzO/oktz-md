// Native module loader with JS fallback. Audit: docs/rust-migration-audit.md §3.1.
// Try prebuilt .node (gnu first, then musl); on failure fall back to the old JS
// implementation and log ONCE so a missing binary is visible in production logs.
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { logger } from "./ourin-logger.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const req = createRequire(import.meta.url);

let native = null;
let fallbackReason = null;

function tryLoad() {
  const candidates = [
    join(__dirname, "..", "..", "native", "index.cjs"), // napi-rs platform resolver
    join(__dirname, "..", "..", "native", "platforms", "x86_64-unknown-linux-gnu", "ourin_native.linux-x64-gnu.node"),
    join(__dirname, "..", "..", "native", "platforms", "x86_64-unknown-linux-musl", "ourin_native.linux-x64-musl.node"),
  ];
  for (const p of candidates) {
    if (!existsSync(p)) continue;
    try {
      native = req(p);
      return;
    } catch (e) {
      fallbackReason = `${p}: ${e.code || e.message}`;
    }
  }
  if (!native && !fallbackReason) fallbackReason = "no prebuilt binary found";
}
tryLoad();
if (!native) {
  // One-time visibility: a missing/incompatible prebuilt binary must be
  // discoverable in production logs without per-call spam.
  logger.warn("ourin-native", `fallback JS aktif (${fallbackReason})`);
}

const hasNative = () => !!native;

// --- OCR facade: identical signature either way -------------------------------
// tesseract.js worker-per-call (the old plugins/tools/ocr.js pattern), kept as
// the fallback so behavior is byte-identical when the native binary is absent.
async function ocrJsFallback(buffer) {
  const { createWorker } = await import("tesseract.js");
  const worker = await createWorker("eng");
  try {
    const { data } = await worker.recognize(buffer);
    return data.text ? data.text.trim() : "";
  } finally {
    await worker.terminate();
  }
}

let nativeInited = false;
let nativeBroken = false;
async function ocr(buffer) {
  if (!native || nativeBroken) return ocrJsFallback(buffer);
  if (!nativeInited) {
    try {
      await native.ocrInit(null, "eng");
      nativeInited = true;
    } catch (e) {
      // init failure (corrupt traineddata etc.) — degrade permanently to the
      // JS fallback instead of retrying (and failing) on every call.
      nativeBroken = true;
      logger.warn("ourin-native", `ocrInit gagal — fallback JS permanen: ${e.message}`);
      return ocrJsFallback(buffer);
    }
  }
  return native.ocrRecognize(buffer);
}

async function ocrShutdown() {
  if (native && nativeInited) {
    await native.ocrShutdown();
    nativeInited = false;
  }
}

export { hasNative, fallbackReason, ocr, ocrShutdown };
