// B5 — setupAntiCrash() menelan semua error lalu terus jalan dalam keadaan
// tidak terdefinisi, dan `console.error(c.gray("Promise:"), promise)`
// men-SERIALISASI SELURUH nilai yang ditolak ke stdout (bisa berisi data user,
// Buffer, atau objek berisi kredensial).
//
// Dua bagian:
// 1. Analisis sumber — selalu jalan. Import index.js akan menyalakan bot
//    (koneksi WA sungguhan), jadi bagian ini tidak boleh meng-import.
// 2. Harness child process —HANYA jalan kalau index.js punya boot guard untuk
//    test runner. Kalau belum ada, test ini di-skip (bukan lulus diam-diam).
import { describe, it, before } from "node:test";
import assert from "node:assert";
import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

const REPO = new URL("..", import.meta.url).pathname;
const INDEX = path.join(REPO, "index.js");
const MARKER = "__ANTI_CRASH_CASES__";

const CHILD = `
const REPO = ${JSON.stringify(REPO)};
const MARKER = ${JSON.stringify(MARKER)};
const mod = await import(REPO + "/index.js");
const { setupAntiCrash, summarizeFault, MAX_FAULTS_IN_WINDOW } = mod;
const cases = [];
const rec = (name, ok, detail = "") => cases.push({ name, ok, detail });

// 1. redaction + batas panjang
const SECRET = {
  password: "hunter2super",
  token: "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345",
  nested: { apiKey: "sk-live-XYZ123456789", url: "https://api.x.com/cb?access_token=abcdef123456&z=1" },
  big: "A".repeat(5000),
};
const s = summarizeFault(SECRET);
rec("no-secret-value",
  !s.includes("hunter2super") && !s.includes("ghp_ABCDEFGH") && !s.includes("sk-live-XYZ") && !s.includes("abcdef123456"),
  "nilai rahasia bocor ke log: " + s.slice(0, 200));
rec("bounded-summary", s.length <= 400, "panjang ringkasan = " + s.length);

const b = summarizeFault(Buffer.alloc(64, 7));
rec("buffer-tidak-didump", !b.includes("7,7,7") && b.length <= 200, "buffer: " + b);

const err = new Error("boom token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345");
err.stack = "Error: boom\\n" + "at x (y.js:1:1)\\n".repeat(400);
const e = summarizeFault(err);
rec("error-baca-pesan", e.includes("boom"), "pesan error hilang: " + e.slice(0, 120));
rec("error-batas-panjang", e.length <= 1200, "panjang ringkasan error = " + e.length);

const cyclic = { a: 1 };
cyclic.self = cyclic;
const c = summarizeFault(cyclic);
rec("cyclic-aman", typeof c === "string" && c.length <= 400, "cyclic: " + c);

// 2. jalur continue dibatasi
let fatals = 0;
const ctrl = setupAntiCrash({ onFatal: () => { fatals++; } });
for (let i = 0; i < MAX_FAULTS_IN_WINDOW - 1; i++) {
  ctrl.handleUncaughtException(new Error("fault-" + i), "test");
}
rec("continue-di-bawah-ambang", fatals === 0, "stop setelah " + (MAX_FAULTS_IN_WINDOW - 1) + " fault, fatals=" + fatals);
ctrl.handleUncaughtException(new Error("fault-ambang"), "test");
rec("berhenti-di-ambang", fatals === 1, "fault ke-" + MAX_FAULTS_IN_WINDOW + " harus memicu berhenti, fatals=" + fatals);

// 3. allow-list tetap diabaikan (tidak menghitung ke ambang)
let fatals2 = 0;
const ctrl2 = setupAntiCrash({ onFatal: () => { fatals2++; } });
for (let i = 0; i < MAX_FAULTS_IN_WINDOW * 3; i++) {
  ctrl2.handleUncaughtException(Object.assign(new Error("read ECONNRESET"), { code: "ECONNRESET" }), "test");
  ctrl2.handleUnhandledRejection(Object.assign(new Error("ETIMEDOUT"), { code: "ETIMEDOUT" }), Promise.resolve(1));
}
rec("allow-list-tidak-dihitung", fatals2 === 0, "error transien tidak boleh mematikan bot, fatals=" + fatals2);

// 4. jalur rejection tidak mencetak nilai promise
let logged = "";
const origErr = console.error;
const origLog = console.log;
console.error = (...a) => { logged += a.map((x) => (typeof x === "string" ? x : "[obj]")).join(" "); };
console.log = (...a) => { logged += a.map((x) => (typeof x === "string" ? x : "[obj]")).join(" "); };
const ctrl3 = setupAntiCrash({ onFatal: () => {} });
ctrl3.handleUnhandledRejection(
  { password: "hunter2super", token: "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345", buffer: Buffer.alloc(8, 9) },
  Promise.resolve(1),
);
console.error = origErr;
console.log = origLog;
rec("rejection-tidak-bocor",
  !logged.includes("hunter2super") && !logged.includes("ghp_ABCDEFGH") && logged.length > 0,
  "log rejection = " + logged.slice(0, 200));

// Harness ini SENGAJA memicu report fault (kasus 2: MAX_FAULTS_IN_WINDOW
// uncaughtException), jadi stdout berisi log kotak merah operator sebelum
// JSON hasil. Karena itu hasil ditulis setelah penanda, dan keluarannya
// di-flush dulu sebelum process.exit — kalau tidak, pipe bisa terpotong.
process.stdout.write("\\n" + MARKER + JSON.stringify({ cases }), () => process.exit(0));
`;

function hasBootGuard(src) {
  return /NODE_TEST_CONTEXT/.test(src);
}

function runChild() {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      ["--input-type=module", "-e", CHILD],
      {
        cwd: REPO,
        timeout: 120000,
        maxBuffer: 4 * 1024 * 1024,
        //_node --test_ hanya mengisi NODE_TEST_CONTEXT di child-nya sendiri;
        // kita set manual supaya index.js tidak menyalakan bot.
        env: { ...process.env, NODE_TEST_CONTEXT: "child-v8" },
      },
      (err, stdout, stderr) => resolve({ err, stdout, stderr }),
    );
  });
}

describe("index.js — setupAntiCrash", () => {
  let src;
  before(async () => {
    src = await readFile(INDEX, "utf8");
  });

  it("nilai yang ditolak tidak lagi diserialisasi penuh ke stdout", () => {
    assert.doesNotMatch(
      src,
      /console\.(error|log|warn)\([^)]*\bpromise\b/,
      'console.error(..., promise) mencetak SELURUH nilai yang ditolak (data user/Buffer/kredensial)',
    );
    assert.doesNotMatch(
      src,
      /String\(reason\)|reason\)\s*\)/,
      "nilai reason tidak boleh dicetak mentah-mentah",
    );
  });

  it("ada ringkasan terikat + redaksi untuk nilai yang ditolak", () => {
    assert.match(
      src,
      /function summarizeFault\(|const summarizeFault\s*=/,
      "butuh summarizeFault() untuk meringkas + meredaksi nilai yang ditolak",
    );
    assert.match(src, /redact\w*\(/, "butuh langkah redaksi (secret/token/password)");
    assert.match(
      src,
      /slice\(0,\s*[A-Z_]+\)|substring\(0,\s*[A-Z_]+\)/,
      "ringkasan harus dipotong batas panjang",
    );
  });

  it("fault tak dikenal dibatasi (tidak loop diam-diam selamanya)", () => {
    assert.match(src, /MAX_FAULTS_IN_WINDOW\s*=/, "butuh ambang fault");
    assert.match(src, /FAULT_WINDOW_MS\s*=/, "butuh jendela waktu fault");
    assert.match(
      src,
      /onFatal|shutdown/i,
      "saus ambang terlampaui harus ada tindakan (shutdown), bukan cuma log",
    );
  });

  it("allow-list error transien tetap ada", () => {
    for (const code of [
      "write EOF",
      "ECONNRESET",
      "EPIPE",
      "ETIMEDOUT",
      "ENOTFOUND",
      "ECONNREFUSED",
      "read ECONNRESET",
    ]) {
      assert.ok(src.includes(code), `kode transient ini harus tetap di-allow-list: ${code}`);
    }
  });

  it("UX kotak merah untuk operator tetap dipakai di kedua handler", () => {
    assert.match(src, /logErrorBox\(\s*"uncaught exception"/);
    assert.match(src, /logErrorBox\(\s*"unhandled rejection"/);
  });
});

describe("index.js — perilaku runtime setupAntiCrash", () => {
  it("harness hanya boleh jalan kalau index.js punya boot guard untuk test runner", async (t) => {
    const src = await readFile(INDEX, "utf8");
    if (!hasBootGuard(src)) {
      t.skip(
        "index.js belum punya boot guard — import akan menyalakan bot, harness dilewati",
      );
      return;
    }

    const { err, stdout, stderr } = await runChild();
    const at = stdout.indexOf(MARKER);
    assert.ok(
      at !== -1,
      `child tidak menghasilkan hasil: ${err?.message}\n${stdout.slice(-800)}\n${stderr?.slice(0, 800)}`,
    );
    const { cases } = JSON.parse(stdout.slice(at + MARKER.length));

    const details = {
      "no-secret-value": "nilai rahasia bocor ke ringkasan log",
      "bounded-summary": "ringkasan tidak dipotong batas panjang",
      "buffer-tidak-didump": "isi Buffer ikut dicetak",
      "error-baca-pesan": "pesan error hilang dari ringkasan",
      "error-batas-panjang": "stack error tidak dipotong",
      "cyclic-aman": "objek siklik membuat summarizeFault melempar",
      "continue-di-bawah-ambang": "bot berhenti sebelum ambang terlampaui",
      "berhenti-di-ambang": "tidak ada failover saat ambang terlampaui",
      "allow-list-tidak-dihitung": "error transien ikut mematikan bot",
      "rejection-tidak-bocor": "jalur unhandledRejection membocorkan nilai",
    };

    for (const [name, msg] of Object.entries(details)) {
      const c = cases.find((x) => x.name === name);
      assert.ok(c, `kasus ${name} tidak dijalankan — harness tidak lengkap`);
      assert.strictEqual(c.ok, true, `${msg}: ${c.detail}`);
    }
  });
});
