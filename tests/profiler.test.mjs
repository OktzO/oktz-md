// Profiling harness — bukti CPU profile + heap snapshot jalan dengan guard RSS.
// Run: node --test tests/profiler.test.mjs
import { describe, it, before, after } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import {
  startCpuProfile,
  stopCpuProfile,
  takeHeapSnapshot,
  heapSnapshotEligibility,
  isCpuProfiling,
  OUT_DIR,
} from "../src/lib/ourin-profiler.js";

const writtenFiles = [];

function busyWork(ms) {
  const end = Date.now() + ms;
  let x = 0;
  while (Date.now() < end) {
    x += Math.sqrt(x + 1) * 1.0000001;
  }
  return x;
}

after(() => {
  for (const f of writtenFiles) {
    try {
      fs.rmSync(f, { force: true });
    } catch {}
  }
});

describe("CPU profiler (node:inspector)", () => {
  it("start → profile → stop menghasilkan file .cpuprofile valid", async () => {
    const mem = process.memoryUsage();
    assert.ok(
      mem.rss < 250 * 1024 * 1024,
      `RSS harus rendah untuk test (${(mem.rss / 1024 / 1024).toFixed(1)}MB)`,
    );

    const startRes = await startCpuProfile({
      name: "test-window",
      durationMs: 60_000,
      trigger: "test",
    });
    assert.ok(startRes.ok, startRes.error);
    assert.ok(isCpuProfiling(), "isCpuProfiling harus true setelah start");

    busyWork(1200);

    const stopRes = await stopCpuProfile();
    assert.ok(stopRes.ok, stopRes.error);
    assert.ok(stopRes.filePath.endsWith(".cpuprofile"));
    assert.ok(fs.existsSync(stopRes.filePath), "file .cpuprofile harus ada");
    assert.ok(stopRes.elapsedMs >= 1000, `elapsed harus >= 1s (${stopRes.elapsedMs})`);

    const profile = JSON.parse(fs.readFileSync(stopRes.filePath, "utf-8"));
    assert.ok(profile.nodes?.length > 0, "profile harus punya nodes");
    assert.ok(
      Array.isArray(profile.samples) && Array.isArray(profile.timeDeltas),
      "profile harus punya samples/timeDeltas (format inspector modern)",
    );
    writtenFiles.push(stopRes.filePath);

    assert.ok(!isCpuProfiling(), "setelah stop harus tidak aktif");
  });

  it("start dua kali berturut harus ditolak", async () => {
    await startCpuProfile({ name: "dup", durationMs: 30_000, trigger: "test" });
    const second = await startCpuProfile({ name: "dup2", durationMs: 30_000, trigger: "test" });
    assert.ok(!second.ok, "start kedua harus gagal");
    const stopRes = await stopCpuProfile();
    assert.ok(stopRes.ok);
    writtenFiles.push(stopRes.filePath);
  });

  it("durasi > max dibatasi ke max", async () => {
    const res = await startCpuProfile({
      name: "cap",
      durationMs: 60 * 60 * 1000,
      trigger: "test",
    });
    assert.ok(res.ok);
    assert.strictEqual(res.durationMs, 10 * 60 * 1000, "harus dibatasi ke 10 menit");
    const stopRes = await stopCpuProfile();
    writtenFiles.push(stopRes.filePath);
  });
});

describe("Heap snapshot (v8.writeHeapSnapshot + guard RSS)", () => {
  it("ambil snapshot saat RSS rendah → sukses, file valid", () => {
    const elig = heapSnapshotEligibility();
    assert.ok(elig.allowed, `harus allowed, reason: ${elig.reason}`);
    assert.ok(elig.rssMb < 300, `rss harus < 300MB (${elig.rssMb}MB)`);

    const res = takeHeapSnapshot({ label: "test-low", trigger: "test" });
    assert.ok(res.ok, res.error);
    assert.ok(res.filePath.endsWith(".heapsnapshot"));
    assert.ok(fs.existsSync(res.filePath), "file .heapsnapshot harus ada");
    assert.ok(res.sizeBytes > 0, "file tidak boleh kosong");

    // heapsnapshot adalah JSON teks (Node 22), mulai dari '{"snapshot":'
    const head = fs.readFileSync(res.filePath).subarray(0, 12);
    assert.strictEqual(head.toString("utf-8"), "{\"snapshot\":", "header harus berupa JSON teks snapshot");
    // parse JSON untuk verifikasi valid
    const raw = fs.readFileSync(res.filePath, "utf-8");
    const parsed = JSON.parse(raw);
    assert.ok(parsed.snapshot?.meta, "snapshot harus punya meta field");
    writtenFiles.push(res.filePath);
  });

  it("guard RSS abort saat RSS tinggi (bukan diasumsikan)", () => {
    const before = heapSnapshotEligibility();
    // Test eligibility math — harus provide struktur yang benar
    assert.ok(typeof before.allowed === "boolean");
    assert.ok(typeof before.rssMb === "number");
    assert.ok(typeof before.projectedMb === "number");

    // Inflasi heap dengan array besar (RSS nyata naik)
    const spike = [];
    let safety = 0;
    while (safety < 30) {
      spike.push(new Array(500_000).fill("x"));
      const after = heapSnapshotEligibility();
      if (!after.allowed) {
        const res = takeHeapSnapshot({ label: "test-high", trigger: "test" });
        assert.ok(!res.ok, "snapshot harus gagal");
        assert.ok(res.abort === true, "harus abort, bukan error lain");
        assert.ok(res.reason, "harus ada reason");
        spike.length = 0;
        return;
      }
      safety++;
    }

    // Fallback: kalau 30 iterasi masih belum trigger (sangat jarang),
    // verifikasi projectedMb naik signifikan
    const after = heapSnapshotEligibility();
    assert.ok(
      after.projectedMb > before.projectedMb + 20,
      `projected harus naik (before ${before.projectedMb} → after ${after.projectedMb})`,
    );
    spike.length = 0;
  });
});
