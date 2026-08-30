import inspector from "node:inspector";
import v8 from "node:v8";
import fs from "node:fs";
import path from "node:path";
import { logger } from "./ourin-logger.js";

// ─────────────────────────────────────────────────────────────
// Harness profiling aman produksi — CPU (node:inspector) + heap
// (v8.writeHeapSnapshot). Default OFF; hanya nyala via trigger
// eksplisit (command owner-only). Lihat docs/profiling-harness.md.
// ─────────────────────────────────────────────────────────────

const OUT_DIR = path.join(process.cwd(), "storage", "profiling");

// Safety thresholds (host Pterodactyl 512MB heap / 1GB total RAM).
// Nilai disesuaikan dengan pengukuran NYATA di panel (2026-08-30):
// baseline RSS idle 290-375MB (bukan 150-230MB seperti mesin audit),
// heapTotal ~185-195MB. Proyeksi puncak idle = rss + 2*heapTotal
// ≈ 330 + 380 = ~710MB.
// - HEAP_SNAP_RSS_CEILING: 400MB. Baseline idle 330MB → margin 70MB
//   untuk kerja normal; abort kalau RSS mendekati zona bahaya.
// - HEAP_SNAP_PROJECTED_CEILING: 800MB. Proyeksi puncak
//   rss + 2*heapTotal saat serialisasi. 800MB < 1GB total RAM kontainer
//   (margin 200MB). Dua kondisi, abort kalau salah satu dilanggar.
const HEAP_SNAP_RSS_CEILING = 400 * 1024 * 1024;
const HEAP_SNAP_PROJECTED_CEILING = 800 * 1024 * 1024;

// Auto-stop safety net: CPU profiler gak boleh nyala lebih dari
// durasi maksimum — kalau lupa dimatiin, berhenti sendiri.
const CPU_MAX_DURATION_MS = 10 * 60 * 1000;
const CPU_DEFAULT_DURATION_MS = 5 * 60 * 1000;

let cpuSession = null;
let cpuStartTime = 0;
let cpuDurationMs = 0;
let cpuName = "";
let cpuTrigger = "";
let cpuTimer = null;

const activeHeapSnapshots = new Set();

function formatMB(bytes) {
  return (bytes / 1024 / 1024).toFixed(1) + "MB";
}

function ensureOutDir() {
  fs.mkdirSync(OUT_DIR, { recursive: true });
}

function timestamp() {
  const d = new Date();
  const pad = (n) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-` +
    `${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
  );
}

function isCpuProfiling() {
  return cpuSession !== null;
}

// ── CPU profiling (node:inspector Session) ───────────────────

async function startCpuProfile({ name = "window", durationMs, trigger = "command" } = {}) {
  if (cpuSession) {
    const remaining = Math.round((cpuDurationMs - (Date.now() - cpuStartTime)) / 1000);
    return {
      ok: false,
      error: `CPU profiler sudah jalan (${cpuName}), sisa ${remaining}s. Stop dulu: .cpuprofile stop`,
    };
  }

  const safeDuration = Math.min(
    durationMs || CPU_DEFAULT_DURATION_MS,
    CPU_MAX_DURATION_MS,
  );

  ensureOutDir();

  const session = new inspector.Session();
  try {
    session.connect();
    await new Promise((resolve, reject) => {
      session.post("Profiler.enable", (err) => (err ? reject(err) : resolve()));
    });
    await new Promise((resolve, reject) => {
      session.post("Profiler.start", (err) => (err ? reject(err) : resolve()));
    });
  } catch (e) {
    try { session.disconnect(); } catch {}
    logger.error("PROFILER", `CPU start gagal: ${e.message}`);
    return { ok: false, error: `Gagal start CPU profiler: ${e.message}` };
  }

  cpuSession = session;
  cpuStartTime = Date.now();
  cpuDurationMs = safeDuration;
  cpuName = String(name).replace(/[^a-zA-Z0-9_-]/g, "_") || "window";
  cpuTrigger = trigger;

  cpuTimer = setTimeout(() => {
    logger.warn(
      "PROFILER",
      `CPU auto-stop (durasi max tercapai, trigger ${cpuTrigger})`,
    );
    stopCpuProfile().catch(() => {});
  }, safeDuration);
  if (cpuTimer.unref) cpuTimer.unref();

  logger.system(
    "PROFILER",
    `CPU profiling START · ${cpuName} · ${Math.round(safeDuration / 1000)}s · trigger ${trigger} · rss ${formatMB(process.memoryUsage().rss)}`,
  );
  return { ok: true, durationMs: safeDuration, name: cpuName };
}

async function stopCpuProfile() {
  if (!cpuSession) {
    return { ok: false, error: "CPU profiler tidak sedang jalan" };
  }

  if (cpuTimer) clearTimeout(cpuTimer);
  cpuTimer = null;

  const session = cpuSession;
  const name = cpuName;
  const trigger = cpuTrigger;
  const elapsedMs = Date.now() - cpuStartTime;
  cpuSession = null;

  try {
    const { profile } = await new Promise((resolve, reject) => {
      session.post("Profiler.stop", (err, result) => (err ? reject(err) : resolve(result)));
    });
    session.disconnect();

    const filePath = path.join(OUT_DIR, `cpu-${timestamp()}-${name}.cpuprofile`);
    fs.writeFileSync(filePath, JSON.stringify(profile));
    logger.system(
      "PROFILER",
      `CPU profiling STOP · ${name} · ${(elapsedMs / 1000).toFixed(1)}s · trigger ${trigger} · ${filePath}`,
    );
    return { ok: true, filePath, elapsedMs, durationMs: cpuDurationMs };
  } catch (e) {
    try { session.disconnect(); } catch {}
    logger.error("PROFILER", `CPU stop gagal: ${e.message}`);
    return { ok: false, error: `Gagal stop CPU profiler: ${e.message}` };
  }
}

function cpuStatus() {
  if (!cpuSession) return { active: false };
  const elapsedMs = Date.now() - cpuStartTime;
  return {
    active: true,
    name: cpuName,
    elapsedS: Math.round(elapsedMs / 1000),
    remainingS: Math.round((cpuDurationMs - elapsedMs) / 1000),
    durationS: Math.round(cpuDurationMs / 1000),
  };
}

// ── Heap snapshot (v8.writeHeapSnapshot) dengan guard RSS ─────

function heapSnapshotEligibility() {
  const mem = process.memoryUsage();
  const rssMb = mem.rss / 1024 / 1024;
  const heapTotalMb = mem.heapTotal / 1024 / 1024;
  const projectedMb = (mem.rss + 2 * mem.heapTotal) / 1024 / 1024;

  let allowed = true;
  let reason = "";
  if (mem.rss > HEAP_SNAP_RSS_CEILING) {
    allowed = false;
    reason = `RSS ${formatMB(mem.rss)} > ceiling ${formatMB(HEAP_SNAP_RSS_CEILING)}`;
  } else if (mem.rss + 2 * mem.heapTotal > HEAP_SNAP_PROJECTED_CEILING) {
    allowed = false;
    reason = `proyeksi puncak ${projectedMb.toFixed(1)}MB (rss + 2×heapTotal) > ${formatMB(HEAP_SNAP_PROJECTED_CEILING)}`;
  }

  return {
    allowed,
    reason,
    rssMb: +rssMb.toFixed(1),
    heapTotalMb: +heapTotalMb.toFixed(1),
    projectedMb: +projectedMb.toFixed(1),
  };
}

function takeHeapSnapshot({ label = "snap", trigger = "command" } = {}) {
  const elig = heapSnapshotEligibility();

  if (activeHeapSnapshots.has(label)) {
    return { ok: false, error: `Snapshot label "${label}" masih diproses (serialisasi berjalan)`, ...elig };
  }

  if (!elig.allowed) {
    logger.warn("PROFILER", `Heap snapshot ABORT · ${elig.reason} · trigger ${trigger}`);
    return { ok: false, abort: true, reason: elig.reason, ...elig };
  }

  ensureOutDir();
  activeHeapSnapshots.add(label);
  const cleanLabel = String(label).replace(/[^a-zA-Z0-9_-]/g, "_") || "snap";

  logger.system(
    "PROFILER",
    `Heap snapshot START · ${cleanLabel} · rss ${formatMB(process.memoryUsage().rss)} · trigger ${trigger}`,
  );

  try {
    // writeHeapSnapshot() sinkron — diblok di luar try tidak boleh;
    // data hasilnya tidak direferensi (hanya path file di-return).
    const filePath = v8.writeHeapSnapshot(
      path.join(OUT_DIR, `heap-${timestamp()}-${cleanLabel}.heapsnapshot`),
    );
    const size = fs.statSync(filePath).size;
    logger.system(
      "PROFILER",
      `Heap snapshot DONE · ${cleanLabel} · ${formatMB(size)} · ${filePath} · trigger ${trigger}`,
    );
    return { ok: true, filePath, sizeBytes: size, ...elig };
  } catch (e) {
    logger.error("PROFILER", `Heap snapshot gagal: ${e.message}`);
    return { ok: false, error: e.message, ...elig };
  } finally {
    activeHeapSnapshots.delete(label);
  }
}

function heapStatus() {
  return {
    ...heapSnapshotEligibility(),
    activeSnapshots: [...activeHeapSnapshots],
  };
}

function profilerStatus() {
  return { cpu: cpuStatus(), heap: heapStatus() };
}

export {
  startCpuProfile,
  stopCpuProfile,
  takeHeapSnapshot,
  profilerStatus,
  heapSnapshotEligibility,
  isCpuProfiling,
  OUT_DIR,
  HEAP_SNAP_RSS_CEILING,
  HEAP_SNAP_PROJECTED_CEILING,
  CPU_MAX_DURATION_MS,
};
