// C4 — aritmetika ceiling heap snapshot mengukur besaran yang salah.
//
// Konstanta lama menganggap biaya transien `v8.writeHeapSnapshot` ≈ 2×heapTotal
// → HEAP_SNAP_PROJECTED_CEILING = 800MB. Pengukuran nyata (Node 22, container):
// transien ≈ 6-10× heapTotal dan ≈ 5.5× ukuran file hasilnya, jadi proyeksi
// lama BAWAH JAUH dari kenyataan. Akibatnya ceiling 800MB jadi dead code dan
// hanya HEAP_SNAP_RSS_CEILING (400MB) yang mengikat — jalur yang dianggap
// "aman" justru yang bisa membuat kontainer 1GB OOM.
import { describe, it } from "node:test";
import assert from "node:assert";
import * as mod from "../src/lib/profiler.js";

const MB = 1024 * 1024;
const mem = (rssMb, heapTotalMb) => ({
  rss: rssMb * MB,
  heapTotal: heapTotalMb * MB,
});

const {
  heapSnapshotEligibility,
  hostMemoryBytes,
  projectedCeilingBytes,
  evaluateHeapSnapshot,
  HEAP_SNAP_RSS_CEILING,
  HEAP_SNAP_PROJECTED_CEILING,
  HEAP_SNAP_TRANSIENT_MULTIPLIER,
} = mod;

describe("proyeksi transient heap snapshot ikut pengukuran", () => {
  it("ekspor yang dibutuhkan tersedia (multiplier + evaluator murni)", () => {
    assert.equal(
      typeof HEAP_SNAP_TRANSIENT_MULTIPLIER,
      "number",
      "HEAP_SNAP_TRANSIENT_MULTIPLIER harus diekspor supaya bisa diaudit",
    );
    assert.equal(
      typeof evaluateHeapSnapshot,
      "function",
      "evaluateHeapSnapshot(mem) harus ada supaya aritmetikanya bisa diuji tanpa inflate RAM",
    );
    assert.equal(typeof hostMemoryBytes, "function");
    assert.equal(typeof projectedCeilingBytes, "function");
  });

  it("multiplier transient ikut hasil ukur, bukan asumsi 2×", () => {
    assert.ok(
      HEAP_SNAP_TRANSIENT_MULTIPLIER >= 9,
      `multiplier ${HEAP_SNAP_TRANSIENT_MULTIPLIER} terlalu kecil — hasil ukur di heap 150-240MB adalah 9.3-10.6× heapTotal, memakai nilai lebih kecil = fail-open`,
    );
    assert.ok(
      HEAP_SNAP_TRANSIENT_MULTIPLIER <= 12,
      `multiplier ${HEAP_SNAP_TRANSIENT_MULTIPLIER} tidak realistis`,
    );
  });

  it("proyeksi = rss + multiplier × heapTotal", () => {
    const e = evaluateHeapSnapshot(mem(100, 50));
    assert.strictEqual(
      e.projectedMb,
      +(100 + HEAP_SNAP_TRANSIENT_MULTIPLIER * 50).toFixed(1),
      "proyeksi harus memakai multiplier terukur",
    );
  });

  it("status proses nyata konsisten dengan rumus yang sama", () => {
    const e = heapSnapshotEligibility();
    const m = process.memoryUsage();
    const expected = +(
      (m.rss + HEAP_SNAP_TRANSIENT_MULTIPLIER * m.heapTotal) /
      MB
    ).toFixed(1);
    assert.strictEqual(
      e.projectedMb,
      expected,
      `proyeksi status harus ${expected}MB, dapat ${e.projectedMb}MB`,
    );
    assert.strictEqual(e.rssMb, +(m.rss / MB).toFixed(1));
  });
});

describe("ceiling menolak state yang akan OOM kontainer", () => {
  it("baseline idle panel (rss 330MB, heap 190MB) DITOLAK — proyeksinya ~2.2GB", () => {
    // Angka nyata dari memory monitor panel 1GB (lihat komentar di profiler.js).
    // Dengan rumus lama (2×) proyeksinya cuma 710MB → "aman" → OOM.
    const e = evaluateHeapSnapshot(mem(330, 190));
    assert.strictEqual(
      e.allowed,
      false,
      "snapshot pada baseline panel akan meledakkan RAM kontainer — harus abort",
    );
    assert.match(
      e.reason,
      /proyeksi/i,
      `alasan harus soal proyeksi, dapat: ${e.reason}`,
    );
    assert.ok(
      e.projectedMb > 2000,
      `proyeksi harus menunjukkan lonjakan nyata, dapat ${e.projectedMb}MB`,
    );
  });

  it("proses kecil (rss 100MB, heap 30MB) tetap boleh — snapshot murah dan aman", () => {
    const e = evaluateHeapSnapshot(mem(100, 30));
    assert.strictEqual(e.allowed, true, `harus boleh, alasan: ${e.reason}`);
  });

  it("RSS di atas ceiling RSS tetap ditolak dengan alasan RSS", () => {
    const e = evaluateHeapSnapshot(mem(HEAP_SNAP_RSS_CEILING / MB + 10, 10));
    assert.strictEqual(e.allowed, false);
    assert.match(e.reason, /RSS/i);
  });

  it("RSS di bawah ceiling tapi proyeksi lewat → DENY (ceiling lama sudah jadi dead code di sini)", () => {
    const rssMb = HEAP_SNAP_RSS_CEILING / MB - 20; // di bawah ceiling RSS
    const e = evaluateHeapSnapshot(mem(rssMb, 190));
    assert.ok(
      rssMb < HEAP_SNAP_RSS_CEILING / MB,
      "prekondisi: RSS harus di bawah ceiling RSS",
    );
    assert.strictEqual(
      e.allowed,
      false,
      "di sinilah ceiling 800MB lama tidak pernah mengikat — harus tetap bisa menolak",
    );
  });
});

describe("ceiling mengikuti RAM host (bukan angka mati yang salah di kedua arah)", () => {
  it("hostMemoryBytes mengembalikan angka waras", () => {
    const host = hostMemoryBytes();
    assert.ok(
      Number.isFinite(host) && host > 0,
      `host memory tidak valid: ${host}`,
    );
    assert.ok(
      host >= 256 * MB,
      `host memory implausible: ${(host / MB).toFixed(0)}MB`,
    );
  });

  it("ceiling di host kecil (1GB panel) tetap 800MB", () => {
    // 800MB = peak < 1GB dengan margin 200MB.
    assert.strictEqual(
      projectedCeilingBytes(1024 * MB),
      HEAP_SNAP_PROJECTED_CEILING,
    );
  });

  it("ceiling di host besar tidak lebih dari plafon keras 2GB", () => {
    const host8gb = 8 * 1024 * MB;
    const ceiling = projectedCeilingBytes(host8gb);
    assert.ok(
      ceiling > HEAP_SNAP_PROJECTED_CEILING,
      "host lega harus boleh lebih tinggi",
    );
    assert.ok(
      ceiling <= 2 * 1024 * MB,
      `plafon keras 2GB dilewati: ${ceiling / MB}MB`,
    );
    assert.ok(
      ceiling <= host8gb / 2,
      "selalu sisakan minimal 2× margin dari RAM host",
    );
  });

  it("default tanpa argumen memakai RAM host yang terdeteksi", () => {
    assert.strictEqual(
      projectedCeilingBytes(),
      projectedCeilingBytes(hostMemoryBytes()),
      "ceiling default harus diturunkan dari RAM host terdeteksi",
    );
    assert.ok(
      projectedCeilingBytes() >= HEAP_SNAP_PROJECTED_CEILING,
      "ceiling effective tidak boleh di bawah floor 800MB",
    );
  });
});
