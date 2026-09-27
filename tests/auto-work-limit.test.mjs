// B3 — auto-work (auto-AI, CAI chat, auto-download, auto-join, CMD-VN Groq
// transcription, smart triggers) jalan di pesan NON-command apa pun, sedangkan
// limiter global hanya dipakai di `if (!m.isCommand) return;` — jadi stranger
// di mode public bisa membakar kuota Groq/Gemini/unduhan tanpa batas.
//
// Test ini mengunci dua hal:
//  1. Kuota auto-work: batas per-pengirim, owner/premium bebas, energi dipotong,
//     dan satu pesan hanya charged SATU kali (anti double-charge).
//  2. URUTAN: guard harus ada DI DEPAN tiap titik kerja mahal, dan limiter
//     command tetap persis di belakang command gate (semantik command tidak
//     berubah).
//
// Catatan: bagian (1) jalan di child process. Import src/handler.js menyisakan
// timer hidup (profile-picture.js:13, game-data.js:267, node-cache di
// connection.js) sehingga proses test tidak akan pernah exit sendiri kalau
// handler.js di-import langsung di runner.
import { describe, it, before } from "node:test";
import assert from "node:assert";
import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

const REPO = new URL("..", import.meta.url).pathname;
const HANDLER = path.join(REPO, "src", "handler.js");

function fakeDb(over = {}) {
  const state = { updates: [], settingEnergi: over.settingEnergi, users: {} };
  if (over.energi !== undefined) {
    state.users["6281234567890@s.whatsapp.net"] = { energi: over.energi };
  }
  return {
    state,
    setting(k) {
      return k === "energi" ? state.settingEnergi : undefined;
    },
    getUser(jid) {
      return state.users[jid] || null;
    },
    updateEnergi(jid, delta) {
      state.updates.push({ jid, delta });
      const cur = state.users[jid]?.energi ?? 99999;
      state.users[jid] = { ...(state.users[jid] || {}), energi: cur + delta };
      return true;
    },
  };
}

function fakeM(over = {}) {
  return {
    sender: "6281234567890@s.whatsapp.net",
    isOwner: false,
    isPremium: false,
    isPartner: false,
    ...over,
  };
}

// ── child script ────────────────────────────────────────────────────────────
const CHILD = (repo) => `
import path from "node:path";
import { pathToFileURL } from "node:url";
const repo = ${JSON.stringify(repo)};
const mod = await import(pathToFileURL(path.join(repo, "src/handler.js")).href);
const config = (await import(pathToFileURL(path.join(repo, "config.js")).href)).default;
const POINTS = config.autoWork?.points ?? 10;
const ENERGI = config.autoWork?.energi ?? 1;

const fakeDb = (over = {}) => {
  const state = { updates: [], settingEnergi: over.settingEnergi, users: {} };
  if (over.energi !== undefined) state.users["6281234567890@s.whatsapp.net"] = { energi: over.energi };
  return {
    state,
    setting(k) { return k === "energi" ? state.settingEnergi : undefined; },
    getUser(jid) { return state.users[jid] || null; },
    updateEnergi(jid, delta) {
      state.updates.push({ jid, delta });
      const cur = state.users[jid]?.energi ?? 99999;
      state.users[jid] = { ...(state.users[jid] || {}), energi: cur + delta };
      return true;
    },
  };
};
const fakeM = (over = {}) => ({
  sender: "6281234567890@s.whatsapp.net",
  isOwner: false, isPremium: false, isPartner: false, ...over,
});

const out = { points: POINTS, energi: ENERGI, cases: [] };
const rec = (name, ok, detail = "") => out.cases.push({ name, ok, detail });

// 1. ekspor tersedia
rec(
  "exports",
  typeof mod.consumeAutoWorkQuota === "function" && typeof mod.createAutoWorkGuard === "function",
  "consumeAutoWorkQuota + createAutoWorkGuard harus diekspor"
);

// 2. batas per-pengirim
{
  const db = fakeDb(); const m = fakeM();
  let firstDenied = -1;
  for (let i = 0; i < POINTS + 5; i++) {
    if (!(await mod.consumeAutoWorkQuota(m, db, "b3-limit-a"))) { firstDenied = i; break; }
  }
  rec("limit", firstDenied === POINTS, "harus boleh tepat POINTSx lalu deny, deny di index " + firstDenied);
}

// 3. owner & premium bebas
{
  const db = fakeDb();
  let ownerOk = true, premiumOk = true;
  for (let i = 0; i < POINTS * 3; i++) {
    ownerOk = ownerOk && (await mod.consumeAutoWorkQuota(fakeM({ isOwner: true }), db, "b3-owner"));
    premiumOk = premiumOk && (await mod.consumeAutoWorkQuota(fakeM({ isPremium: true }), db, "b3-premium"));
  }
  rec("owner-premium", ownerOk && premiumOk && db.state.updates.length === 0,
    "owner/premium harus selalu boleh & tidak dipotong energi (updates=" + db.state.updates.length + ")");
}

// 4. energi dipotong
{
  const db = fakeDb(); const m = fakeM();
  const allowed = await mod.consumeAutoWorkQuota(m, db, "b3-energi");
  rec("energi-dipotong", allowed && db.state.updates.length === 1 && db.state.updates[0].delta === -ENERGI,
    "energi harus dipotong " + ENERGI + ", dapat " + JSON.stringify(db.state.updates));
}

// 5. energi habis → deny
{
  const db = fakeDb({ energi: 0 });
  const allowed = await mod.consumeAutoWorkQuota(fakeM(), db, "b3-energi-habis");
  rec("energi-habis", allowed === false && db.state.updates.length === 0,
    "energi 0 harus deny dan tidak boleh dipotong jadi negatif");
}

// 6. satu pesan = satu charge (memoized)
{
  const db = fakeDb(); const m = fakeM();
  let allOk = true;
  for (let i = 0; i < 3; i++) {
    const allow = mod.createAutoWorkGuard(m, db, "b3-once");
    for (let j = 0; j < 4; j++) allOk = allOk && (await allow());
  }
  // 12 pemanggilan = 3 pesan; sisa kuota harus POINTS - 3
  let used = 0;
  for (let i = 0; i < POINTS - 3; i++) {
    const allow = mod.createAutoWorkGuard(m, db, "b3-once");
    if (await allow()) used++;
  }
  const extra = mod.createAutoWorkGuard(m, db, "b3-once");
  const denied = (await extra()) === false;
  rec("satu-charge", allOk && used === POINTS - 3 && denied,
    "12x allow() harus hanya memakai 3 kuota; terpakai=" + (3 + used) + " dari " + POINTS + ", deny terakhir=" + denied);
}

// 7. kill switch
{
  const db = fakeDb();
  const before = config.autoWork?.enabled;
  config.autoWork = { ...(config.autoWork || {}), enabled: false };
  let ok = true;
  try {
    for (let i = 0; i < POINTS * 3; i++) ok = ok && (await mod.consumeAutoWorkQuota(fakeM(), db, "b3-killswitch"));
  } finally {
    config.autoWork = { ...(config.autoWork || {}), enabled: before };
  }
  rec("kill-switch", ok, "enabled=false harus mematikan gate sepenuhnya");
}

process.stdout.write(JSON.stringify(out));
process.exit(0);
`;

function runChild() {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      ["--input-type=module", "-e", CHILD(REPO)],
      { cwd: REPO, timeout: 120000, maxBuffer: 4 * 1024 * 1024 },
      (err, stdout, stderr) => {
        resolve({ err, stdout, stderr });
      },
    );
  });
}

describe("kuota auto-work (rate limit + energi)", () => {
  let result;

  before(async () => {
    const { err, stdout, stderr } = await runChild();
    assert.ifError(
      err && !stdout,
      `child gagal: ${err?.message}\n${stderr}`,
    );
    result = JSON.parse(stdout);
  });

  it("child harness dieksekusi di repo yang benar", () => {
    assert.ok(result, "harus ada hasil child");
    assert.ok(result.points > 0, `config.autoWork.points harus > 0, dapat ${result.points}`);
  });

  const CHECKS = [
    ["exports", "consumeAutoWorkQuota + createAutoWorkGuard harus ada"],
    ["limit", "kuota auto-work tidak membatasi pengirim"],
    ["owner-premium", "owner/premium ikut dibatasi"],
    ["energi-dipotong", "energi tidak dipotong per kerja auto"],
    ["energi-habis", "energi habis tidak menolak"],
    ["satu-charge", "satu pesan charged lebih dari sekali"],
    ["kill-switch", "kill switch config tidak berfungsi"],
  ];

  for (const [name, msg] of CHECKS) {
    it(`${name}: ${msg}`, () => {
      const c = result.cases.find((x) => x.name === name);
      assert.ok(c, `kasus ${name} tidak dijalankan di child`);
      assert.strictEqual(c.ok, true, c.detail || `${name} gagal`);
    });
  }
});

describe("handler.js — guard auto-work harus mendahului kerja mahal", () => {
  let src;
  before(async () => {
    src = await readFile(HANDLER, "utf8");
  });

  const SITES = [
    ["transkripsi Groq (CMD VN)", "const cmdVnEnabled"],
    ["auto-AI", "await handleAutoAI("],
    ["CAI chat", "await caiChatHandler("],
    ["auto-download", "handleAutoDownload(m, sock, m.body)"],
    ["auto-join", "await autoJoinDetector("],
    ["smart trigger", "await handleSmartTriggers("],
  ];

  for (const [label, needle] of SITES) {
    it(`${label} berada DI BELAH guard kuota`, () => {
      const guardIdx = src.indexOf("createAutoWorkGuard(");
      assert.ok(guardIdx > -1, "guard kuota harus dibuat di messageHandler");
      const siteIdx = src.indexOf(needle);
      assert.ok(siteIdx > -1, `${needle} tidak ditemukan — struktur berubah?`);
      assert.ok(
        guardIdx < siteIdx,
        `${label} jalan sebelum guard dibuat — biaya API bisa dibakar tanpa batas`,
      );
      const around = src.slice(Math.max(0, siteIdx - 700), siteIdx + 200);
      assert.match(
        around,
        /allowAutoWork\(\)/,
        `${label} harus dikunci allowAutoWork() (bukan cuma limiter global yang ada setelah command gate)`,
      );
    });
  }

  it("limiter command tetap di belakang command gate (semantik command tidak berubah)", () => {
    const gateIdx = src.indexOf("if (!m.isCommand) return;");
    const limiterIdx = src.indexOf("await isSpamming(");
    assert.ok(gateIdx > -1 && limiterIdx > -1, "command gate / limiter tidak ditemukan");
    assert.ok(
      gateIdx < limiterIdx,
      "limiter global harus tetap setelah command gate",
    );
    assert.strictEqual(
      src.split("await isSpamming(").length - 1,
      1,
      "limiter global hanya boleh dipakai 1x — kalau dobel, command asli jadi double-charge",
    );
  });
});
