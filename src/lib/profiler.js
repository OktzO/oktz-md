import inspector from "node:inspector";
import v8 from "node:v8";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { logger } from "./logger.js";

// ─────────────────────────────────────────────────────────────
// Harness profiling aman produksi — CPU (node:inspector) + heap
// (v8.writeHeapSnapshot). Default OFF; hanya nyala via trigger
// eksplisit (command owner-only). Lihat docs/profiling-harness.md.
// ─────────────────────────────────────────────────────────────

const OUT_DIR = path.join(process.cwd(), "storage", "profiling");

// Safety thresholds (host Pterodactyl 512MB heap / 1GB total RAM).
// Nilai idle di panel (2026-08-30, memory monitor): rss 290-375MB,
// heapTotal 185-195MB.
//
// HEAP_SNAP_TRANSIENT_MULTIPLIER — rasio transient v8.writeHeapSnapshot
// terhadap heapTotal. Angka LAMA (2×) salah jauh terlalu kecil: snapshot
// serialisasi SELURUH heap ke string lalu menulisnya, jadi biayanya
// jauh lebih besar dari asumsi "2× heapTotal". Pengukuran nyata di
// container 8GB (Node 22, heap terisi penuh, peak RSS disampel tiap 5ms):
//
//   heapTotal 79.6MB → file  81.5MB → transient  475.8MB ( 6.0× heap, 5.8× file)
//   heapTotal 142.3MB → file 199.9MB → transient 1079.1MB ( 7.6× heap, 5.4× file)
//   heapTotal 205.2MB → file 318.4MB → transient 1754.3MB ( 8.6× heap, 5.5× file)
//   heapTotal 309.3MB → file 519.4MB → transient 2889.3MB ( 9.3× heap, 5.6× file)
//
// Rasio NAIK seiring membesarnya heap (bukan konstan ~5.5× file karena file
// sendiri hanya ~1.5× heapTotal saat heap berisi objek kecil). Angka 10
// diambil dari ujung atas ukuran yang relevan untuk produksi (150-240MB
// heapTotal → 9.3-10.6×) supaya guard tidak UNDERESTIMATE — memakai angka
// yang lebih kecil berarti fail-open, persis bug yang sedang diperbaiki.
//
// Kesimpulan untuk panel 1GB: dengan rss 330MB + heap 190MB, transient nyata
// ~1.9GB → peak ~2.2GB. Snapshot di baseline panel itu OOM, jadi WAJIB
// ditolak. Honesty > "fitur selalu bisa jalan": jalur yang pernah dianggap
// aman inilah yang meledakkan kontainer.
//
// - HEAP_SNAP_RSS_CEILING: 400MB. Pre-check murah: RSS dekat batas = proses
//   sudah tidak sehat, jangan sentuh V8 sama sekali.
// - HEAP_SNAP_PROJECTED_CEILING: 800MB. Lantai ceiling untuk host kecil:
//   kontainer 1GB → peak < 1GB dengan margin 200MB. Untuk host lebih besar
//   ceiling mengikuti RAM host (setengahnya, plafon keras 2GB) — di mesin dev
//   8GB, snapshot pada 300MB RSS bukan perilaku OOM.
// - RSS ceiling sendiri TIDAK cukup jadi satu-satunya guard: ia tidak melihat
//   lonjakan 1.5-2.9GB yang menyusul. Ceiling proyeksi yang mengikat.
const HEAP_SNAP_RSS_CEILING = 400 * 1024 * 1024;
const HEAP_SNAP_PROJECTED_CEILING = 800 * 1024 * 1024;
const HEAP_SNAP_PROJECTED_CEILING_MAX = 2 * 1024 * 1024 * 1024;
const HEAP_SNAP_TRANSIENT_MULTIPLIER = 10;

// Auto-stop safety net: CPU profiler gak boleh nyala lebih dari
// durasi maksimum — kalau lupa dimatiin, berhenti sendiri.
const CPU_MAX_DURATION_MS = 10 * 60 * 1000;
const CPU_DEFAULT_DURATION_MS = 5 * 60 * 1000;

const CGROUP_LIMITS = [
  "/sys/fs/cgroup/memory.max", // cgroup v2
  "/sys/fs/cgroup/memory/memory.limit_in_bytes", // cgroup v1
];

/**
 * RAM yang boleh dipakai proses: limit cgroup kalau ada (Pterodactyl/Docker),
 * fallback ke RAM host. Nilai ngawur ("max", 2^63) diperlakukan sebagai
 * tidak terbatas.
 * @returns {number} byte
 */
function hostMemoryBytes() {
  for (const file of CGROUP_LIMITS) {
    try {
      const raw = fs.readFileSync(file, "utf8").trim();
      if (!raw || raw === "max") continue;
      const value = Number(raw);
      if (!Number.isFinite(value) || value <= 0) continue;
      if (value >= Number.MAX_SAFE_INTEGER / 2) continue; // ~"unlimited"
      if (value >= 256 * 1024 * 1024) return value;
    } catch { }
  }
  return os.totalmem();
}

/**
 * Ceiling proyeksi efektif: minimal 800MB (aman untuk kontainer 1GB), maksimal
 * separuh RAM host dengan plafon keras 2GB.
 * @param {number} [hostBytes] RAM host — argumen opsional untuk uji.
 * @returns {number} byte
 */
function projectedCeilingBytes(hostBytes) {
  const host = hostBytes ?? hostMemoryBytes();
  const half = Math.floor(host / 2);
  if (half <= HEAP_SNAP_PROJECTED_CEILING) return HEAP_SNAP_PROJECTED_CEILING;
  return Math.min(half, HEAP_SNAP_PROJECTED_CEILING_MAX);
}

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

// ── Heap snapshot (v8.writeHeapSnapshot) dengan guard RSS + proyeksi ─────

/**
 * Pure evaluator — logika guard dipisah dari process.memoryUsage() supaya
 * aritmetikanya bisa diuji tanpa harus inflate RAM sungguhan.
 * @param {{rss:number, heapTotal:number}} mem
 * @param {number} [projectedCeiling] default: projectedCeilingBytes()
 * @returns {{allowed:boolean, reason:string, rssMb:number, heapTotalMb:number,
 *            projectedMb:number, ceilingMb:number}}
 */
function evaluateHeapSnapshot(mem, projectedCeiling) {
  const ceiling = projectedCeiling ?? projectedCeilingBytes();
  const rssMb = mem.rss / 1024 / 1024;
  const heapTotalMb = mem.heapTotal / 1024 / 1024;
  const projectedBytes = mem.rss + HEAP_SNAP_TRANSIENT_MULTIPLIER * mem.heapTotal;
  const projectedMb = projectedBytes / 1024 / 1024;

  let allowed = true;
  let reason = "";
  if (mem.rss > HEAP_SNAP_RSS_CEILING) {
    allowed = false;
    reason = `RSS ${formatMB(mem.rss)} > ceiling ${formatMB(HEAP_SNAP_RSS_CEILING)}`;
  } else if (projectedBytes > ceiling) {
    allowed = false;
    reason = `proyeksi puncak ${projectedMb.toFixed(1)}MB (rss + ${HEAP_SNAP_TRANSIENT_MULTIPLIER}×heapTotal) > ${formatMB(ceiling)}`;
  }

  return {
    allowed,
    reason,
    rssMb: +rssMb.toFixed(1),
    heapTotalMb: +heapTotalMb.toFixed(1),
    projectedMb: +projectedMb.toFixed(1),
    ceilingMb: +(ceiling / 1024 / 1024).toFixed(1),
  };
}

function heapSnapshotEligibility() {
  return evaluateHeapSnapshot(process.memoryUsage());
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
  evaluateHeapSnapshot,
  hostMemoryBytes,
  projectedCeilingBytes,
  isCpuProfiling,
  OUT_DIR,
  HEAP_SNAP_RSS_CEILING,
  HEAP_SNAP_PROJECTED_CEILING,
  HEAP_SNAP_PROJECTED_CEILING_MAX,
  HEAP_SNAP_TRANSIENT_MULTIPLIER,
  CPU_MAX_DURATION_MS,
};
