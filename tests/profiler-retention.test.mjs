// C5 — profiler menulis file baru tiap kali dan tidak pernah menghapus yang lama.
//
// cpu-*.cpuprofile ditulis dengan fs.writeFileSync dan heap-*.heapsnapshot lewat
// v8.writeHeapSnapshot; tidak ada satu pun jalur yang menghapus. Satu disk
// penampung Pterodactyl yang kecil akan penuh dari artefak profiling, dan karena
// storage/ tidak ikut auto-backup (gitignored) file-file ini menumpuk diam-diam
// sampai bot berhenti bisa menulis.
//
// Pengamatan: 25 file, 135MB, tertua sudah sebulan — dan itu di mesin yang
// tidak pernah menjalankan profiler lewat command.
import { describe, it } from "node:test";
import assert from "node:assert";
import * as mod from "../src/lib/profiler.js";

const { planRetention, RETENTION_MAX_FILES, RETENTION_MAX_BYTES, RETENTION_MAX_AGE_MS } = mod;

const DAY = 24 * 60 * 60 * 1000;
const now = Date.UTC(2026, 8, 30);
const file = (name, bytes, mtimeMs) => ({ name, bytes, mtimeMs });

describe("retensi artefak profiler", () => {
  it("konstanta retensi diekspor dan punya nilai yang masuk akal", () => {
    assert.equal(typeof RETENTION_MAX_FILES, "number");
    assert.equal(typeof RETENTION_MAX_BYTES, "number");
    assert.equal(typeof RETENTION_MAX_AGE_MS, "number");
    assert.ok(RETENTION_MAX_FILES > 0, "harus ada batas jumlah file");
    assert.ok(RETENTION_MAX_BYTES > 0, "harus ada batas total ukuran");
    assert.ok(RETENTION_MAX_AGE_MS >= DAY, "umur minimal satu hari");
  });

  it("file yang belum lewat umur tidak dihapus", () => {
    const files = [
      file("cpu-1.cpuprofile", 1000, now - 1000),
      file("heap-1.heapsnapshot", 2000, now - 2000),
    ];
    assert.deepEqual(planRetention(files, { now }).remove, []);
  });

  it("file yang lebih tua dari batas umur dihapus", () => {
    const files = [
      file("cpu-old.cpuprofile", 1000, now - (RETENTION_MAX_AGE_MS + DAY)),
      file("cpu-new.cpuprofile", 1000, now - 1000),
    ];
    const plan = planRetention(files, { now });
    assert.deepEqual(plan.remove.map((f) => f.name), ["cpu-old.cpuprofile"]);
    assert.equal(plan.bytesFreed, 1000);
  });

  it("jumlah file dibatasi, yang tertua lebih dulu", () => {
    // Semua mtime di masa lalu, cpu-000 yang paling tua, dan file di urutan
    // belakang sengaja dibuat lebih besar supaya batas jumlah harus menang
    // sebelum batas ukuran.
    const total = RETENTION_MAX_FILES + 5;
    const files = Array.from({ length: total }, (_, i) =>
      file(
        `cpu-${String(i).padStart(3, "0")}.cpuprofile`,
        10 + (total - i) * 10,
        now - (total - i) * 1000,
      )
    );
    const plan = planRetention(files, { now });
    assert.equal(plan.remove.length, 5, "kelebihan jumlah harus dihapus");
    for (const f of plan.remove) {
      assert.ok(f.mtimeMs < now - (RETENTION_MAX_FILES - 1) * 1000, `${f.name} harus yang tertua`);
    }
    // Lima yang tertua harus yang hilang, dan yang terbaru harus selamat.
    assert.ok(plan.remove.some((f) => f.name === "cpu-000.cpuprofile"));
    assert.ok(!plan.remove.some((f) => f.name === `cpu-${String(total - 1).padStart(3, "0")}.cpuprofile`));
  });

  it("total ukuran dibatasi, file terbesar lebih dulu", () => {
    const small = 1024;
    const files = [
      file("small-old.cpuprofile", small, now - 3000),
      file("big-mid.cpuprofile", small * 40, now - 2000),
      file("mid-new.cpuprofile", small * 20, now - 1000),
    ];
    const limit = small * 45;
    const plan = planRetention(files, { now, maxBytes: limit });
    assert.ok(plan.bytesFreed >= small * 30, "harus membebaskan cukup");
    assert.ok(
      plan.remove.some((f) => f.name === "small-old.cpuprofile") ||
      plan.remove.some((f) => f.name === "big-mid.cpuprofile"),
      "file terbesar atau tertua lebih dulu"
    );
    const kept = plan.keep.map((f) => f.name);
    assert.ok(kept.includes("mid-new.cpuprofile"), "file terbaru wajib dipertahankan");
  });

  it("file yang baru ditulis tidak ikut terhapus", () => {
    // Profil yang sedang disimap harus selamat dari resonate-nya sendiri.
    const justWritten = file("cpu-just-written.cpuprofile", 10, now);
    const plan = planRetention([justWritten], { now });
    assert.ok(!plan.remove.some((f) => f.name === justWritten.name));
  });

  it("daftar kosong tidak menghasilkan removal", () => {
    const plan = planRetention([], { now });
    assert.deepEqual(plan.remove, []);
    assert.deepEqual(plan.keep, []);
    assert.equal(plan.bytesFreed, 0);
  });
});
