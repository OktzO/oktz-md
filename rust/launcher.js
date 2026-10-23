#!/usr/bin/env node
// Launcher Node.js untuk binary Rust `oktz` (panel egg Node.js).
// Kalau bin/oktz belum ada, binary diunduh dari GitHub Release repo ini.
// Path dihitung dari lokasi berkas ini, bukan cwd panel.
// Argumen diteruskan; tanpa argumen = mode `run`. Sinyal dan exit code
// diteruskan ke/dari anak supaya panel bisa membedakan stop vs crash.
//
// Tanpa argumen launcher ikut urutan bot JS lama:
//   1. creds.json ada dan sudah paired -> run
//   2. Turso aktif dan punya sesi      -> pulihkan creds + sesi ratchet, run
//   3. belum ada sesi                  -> pairing code / QR, lalu run
//
//   node launcher.js [argumen oktz...]
//   OKTZ_BIN=/path/ke/oktz node launcher.js
import { execFileSync, spawn } from "node:child_process";
import {
  accessSync,
  chmodSync,
  constants,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const AKAR = path.dirname(fileURLToPath(import.meta.url));
const REPO = "OktzO/oktz-rs";
// Statis dipilih lebih dulu: binary dinamis butuh loader glibc, dan panel
// Alpine/musl tidak punya loader itu sehingga exec gagal ENOENT walau
// berkasnya ada. Kalau rilis belum punya aset statis, pakai aset dinamis.
// `OKTZ_ASET` bisa memaksa nama aset tertentu.
const ASET_STATIS = "oktz-linux-x64-static";
const ASET_GLIBC = "oktz-linux-x64";
const ASET = process.env.OKTZ_ASET || ASET_STATIS;
// Naikkan tiap rilis: panel menyimpan bin/oktz lama, jadi tanpa penanda ini
// binary baru tidak pernah diunduh dan command baru (mis. menu) tetap hilang.
const VERSI_BIN = "v0.1.4";
const KANDIDAT = ["bin/oktz", "target/release/oktz"].map((p) => path.join(AKAR, p));
const BIN = process.env.OKTZ_BIN
  ? path.resolve(process.env.OKTZ_BIN)
  : KANDIDAT.find((p) => existsSync(p)) || KANDIDAT[0];
const ARGS = process.argv.slice(2);
const KANDIDAT_CREDS = [
  path.join(AKAR, "storage/session/creds.json"),
  path.join(AKAR, "../storage/session/creds.json"),
];

/** Lokasi creds yang dipakai binary: yang sudah ada, atau layout yang cocok. */
function pathCreds() {
  const ada = KANDIDAT_CREDS.find((p) => existsSync(p));
  if (ada) return ada;
  // Panel: storage/ di samping launcher. Dev (ourin-md): launcher di rust/,
  // storage/ di akar repo.
  return existsSync(path.join(AKAR, "../storage"))
    ? KANDIDAT_CREDS[1]
    : KANDIDAT_CREDS[0];
}

/** Sama dengan `looksPaired` di connection.js: registered atau punya account. */
function credsSiap(p) {
  if (!existsSync(p)) return false;
  try {
    const c = JSON.parse(readFileSync(p, "utf8"));
    return c?.registered === true || Boolean(c?.account);
  } catch {
    return false;
  }
}

/** config.js di akar repo (panel) atau satu tingkat di atas `rust/` (dev). */
function pathKonfig() {
  for (const p of [path.join(AKAR, "config.js"), path.join(AKAR, "../config.js")]) {
    if (existsSync(p)) return p;
  }
  return path.join(AKAR, "config.js");
}

/** config.json (milik binary Rust) di akar repo atau satu tingkat di atas. */
function pathKonfigJson() {
  for (const p of [path.join(AKAR, "config.json"), path.join(AKAR, "../config.json")]) {
    if (existsSync(p)) return p;
  }
  return path.join(AKAR, "config.json");
}

function bacaKonfigJson() {
  const p = pathKonfigJson();
  if (!existsSync(p)) return {};
  try {
    return JSON.parse(readFileSync(p, "utf8")) || {};
  } catch (e) {
    console.error(`[launcher] config.json tidak terbaca: ${e.message}`);
    return {};
  }
}

/** Baca nilai pairing dari config, tanpa meng-import config.js (ada efek samping). */
function bacaKonfig() {
  const j = bacaKonfigJson();
  if (j.session) {
    return {
      pairingNumber: j.session.pairingNumber,
      usePairingCode: j.session.usePairingCode,
    };
  }
  const p = pathKonfig();
  if (!existsSync(p)) return {};
  const t = readFileSync(p, "utf8");
  const nomor = /pairingNumber\s*:\s*["'`]([^"'`]*)["'`]/.exec(t)?.[1];
  const kode = /usePairingCode\s*:\s*(true|false)/.exec(t)?.[1];
  return {
    pairingNumber: nomor,
    usePairingCode: kode === undefined ? undefined : kode === "true",
  };
}

/** Nilai Turso: env menang, lalu config.json, lalu literal config.js. */
function bacaTurso() {
  const j = bacaKonfigJson().turso || {};
  const p = pathKonfig();
  let blok = "";
  if (existsSync(p)) {
    const t = readFileSync(p, "utf8");
    blok = /turso\s*:\s*\{([\s\S]*?)\n\s*\}/.exec(t)?.[1] || "";
  }
  const literal = (nama) => {
    const m = new RegExp(
      `${nama}\\s*:\\s*(?:process\\.env\\.[A-Z_]+\\s*\\|\\|\\s*)?["'\`]([^"'\`]*)["'\`]`
    ).exec(blok);
    return (m?.[1] || "").trim();
  };
  const url = (process.env.TURSO_URL || j.url || literal("url")).trim();
  const token = (process.env.TURSO_AUTH_TOKEN || j.authToken || literal("authToken")).trim();
  const mati = j.enabled === false || /enabled\s*:\s*false/.test(blok);
  return { url, token, aktif: !mati && url !== "" };
}

function tursoEndpoint(url) {
  let s = url.trim();
  if (s.startsWith("libsql://")) s = `https://${s.slice("libsql://".length)}`;
  else if (!/^https?:\/\//.test(s)) s = `https://${s}`;
  return `${s.replace(/\/+$/, "")}/v2/pipeline`;
}

/** Satu query Hrana lewat HTTP Turso, tanpa dependensi libsql. */
async function tursoQuery(endpoint, token, sql, args = []) {
  const body = {
    requests: [
      {
        type: "execute",
        stmt: {
          sql,
          args: args.map((v) => ({ type: "text", value: String(v) })),
        },
      },
      { type: "close" },
    ],
  };
  const r = await fetch(endpoint, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!r.ok) {
    const teks = await r.text().catch(() => "");
    throw new Error(`HTTP ${r.status}${teks ? `: ${teks.slice(0, 200)}` : ""}`);
  }
  const j = await r.json();
  const res = j.results?.[0];
  if (res?.type === "error") {
    throw new Error(res.error?.message || "pipeline Turso error");
  }
  if (res?.type !== "ok" || res.response?.type !== "execute") {
    throw new Error("respons pipeline Turso tidak dikenal");
  }
  const hasil = res.response.result || {};
  const cols = (hasil.cols || []).map((c) => c.name);
  return (hasil.rows || []).map((row) => {
    const o = {};
    row.forEach((cell, i) => {
      o[cols[i]] = cell?.type === "text" ? cell.value : null;
    });
    return o;
  });
}

/**
 * Pulihkan sesi dari Turso seperti useDurableAuthState di JS: creds hanya
 * dipakai kalau sesi lokal belum paired, keys mengisi ratchet yang hilang.
 *
 * @returns {Promise<string|null>} path fixture sesi, atau null kalau tanpa keys.
 */
async function pulihkanTurso(creds, turso) {
  const endpoint = tursoEndpoint(turso.url);
  const credsRows = await tursoQuery(
    endpoint,
    turso.token,
    "SELECT creds FROM session_creds WHERE scope = ?",
    ["main"]
  );
  if (credsRows.length === 0) throw new Error("session_creds kosong (scope main)");

  const raw = credsRows[0].creds;
  let c;
  try {
    c = JSON.parse(raw);
  } catch {
    throw new Error("creds Turso bukan JSON");
  }
  if (!(c?.registered === true || Boolean(c?.account))) {
    throw new Error("creds Turso belum paired");
  }

  mkdirSync(path.dirname(creds), { recursive: true });
  writeFileSync(creds, raw, { mode: 0o600 });

  const keys = await tursoQuery(
    endpoint,
    turso.token,
    "SELECT category, id, data FROM session_keys WHERE scope = ? AND category = ?",
    ["main", "session"]
  ).catch((e) => {
    // Creds sudah tersimpan; kegagalan keys jangan membatalkan restore —
    // jalankan tanpa fixture lebih baik daripada pairing ulang.
    console.error(`[launcher] query session_keys gagal: ${e.message}`);
    return [];
  });
  if (keys.length === 0) return null;

  const fixture = path.join(path.dirname(creds), "turso-session.json");
  const dump = {
    session: keys.map((r) => ({
      scope: "main",
      category: "session",
      id: r.id,
      data: r.data,
    })),
  };
  writeFileSync(fixture, JSON.stringify(dump), { mode: 0o600 });
  return fixture;
}

function siap_dipakai(p) {
  if (!existsSync(p)) return false;
  try {
    const s = statSync(p);
    return s.isFile() && s.size > 0;
  } catch {
    return false;
  }
}

// Hanya bin/oktz hasil unduhan yang dicek versinya; binary dev di
// target/release dan OKTZ_BIN tidak boleh dipaksa unduh.
const BIN_UNDUHAN = BIN === KANDIDAT[0] && !process.env.OKTZ_BIN;
const PENANDA = path.join(path.dirname(BIN), ".oktz-versi");
function versiCocok() {
  try {
    return readFileSync(PENANDA, "utf8").trim() === VERSI_BIN;
  } catch {
    return false;
  }
}

if (!siap_dipakai(BIN) || (BIN_UNDUHAN && !versiCocok())) {
  if (process.env.OKTZ_BIN && !siap_dipakai(BIN)) {
    console.error(`binary ${BIN} tidak ada/kosong (dari OKTZ_BIN)`);
    process.exit(127);
  }
  if (existsSync(BIN)) {
    console.error(`[launcher] ${BIN} kosong atau versi lama, unduh ulang`);
  }
  try {
    await unduh();
    if (BIN_UNDUHAN) writeFileSync(PENANDA, `${VERSI_BIN}\n`);
  } catch (e) {
    console.error(`[launcher] gagal unduh binary: ${e.message}`);
    if (!siap_dipakai(BIN)) {
      console.error(
        `[launcher] unduh manual: https://github.com/${REPO}/releases/latest (aset ${ASET}), ` +
          `simpan sebagai ${BIN}`
      );
      process.exit(127);
    }
    console.error(`[launcher] memakai binary lama di ${BIN}`);
  }
  if (!siap_dipakai(BIN)) {
    console.error(`[launcher] ${BIN} masih tidak bisa dipakai setelah unduh`);
    process.exit(127);
  }
}

// Panel kadang menyimpan berkas tanpa bit eksekusi.
try {
  accessSync(BIN, constants.X_OK);
} catch {
  try {
    chmodSync(BIN, 0o755);
  } catch (e) {
    console.error(`[launcher] tidak bisa chmod +x ${BIN}: ${e.message}`);
    process.exit(126);
  }
}

// Preflight registry: binari lama menjawab mode tak dikenal dengan JSON boot
// yang sama-sama punya kunci `plugins`, jadi cek ini aman untuk rilis lama.
function cekRegistry() {
  try {
    const out = execFileSync(path.resolve(BIN), ["plugins"], {
      cwd: AKAR,
      encoding: "utf8",
      timeout: 60_000,
      stdio: ["ignore", "pipe", "ignore"],
    });
    const baris = out
      .trim()
      .split("\n")
      .filter((l) => l.trim().startsWith("{"))
      .pop();
    const j = JSON.parse(baris);
    if (!Number.isInteger(j.plugins) || j.plugins <= 0) {
      console.error("[launcher] registry plugin kosong; bot tidak akan merespons command");
      process.exit(3);
    }
    console.log(`[launcher] plugin terdaftar: ${j.plugins}`);
  } catch (e) {
    console.error(`[launcher] gagal cek registry plugin: ${e.message}`);
    process.exit(3);
  }
}

// Path absolut: spawn dengan path relatif bisa ENOENT kalau cwd panel beda
// dari folder launcher.
const BIN_ABS = path.resolve(BIN);
cekRegistry();
let anak = null;
let sudahUnduhUlang = false;

function jalankan(args, extraEnv = {}) {
  console.log(`[launcher] jalankan ${BIN_ABS} ${args.join(" ")}`);
  // cwd = akar repo supaya config.json, database/, dan storage/ selalu relatif
  // ke sini, bukan ke direktori kerja panel.
  anak = spawn(BIN_ABS, args, {
    stdio: "inherit",
    env: { ...process.env, ...extraEnv },
    cwd: AKAR,
  });

  anak.on("error", async (e) => {
    console.error(`[launcher] gagal menjalankan ${BIN_ABS}: ${e.message}`);
    if (e.code === "ENOENT" && !sudahUnduhUlang && !process.env.OKTZ_BIN) {
      // Binary lama (dinamis) bisa tertinggal di bin/oktz; unduh ulang aset
      // yang dipilih lalu coba sekali lagi.
      sudahUnduhUlang = true;
      console.error(`[launcher] unduh ulang aset ${ASET} lalu coba lagi`);
      try {
        await unduh(ASET);
        chmodSync(BIN_ABS, 0o755);
        jalankan(args, extraEnv);
        return;
      } catch (e2) {
        console.error(`[launcher] unduh ulang gagal: ${e2.message}`);
      }
    }
    if (e.code === "ENOENT") {
      console.error(
        `[launcher] berkas ada tapi loader tidak ketemu. Aset: ${ASET}, ` +
          `panel: ${process.platform}/${process.arch}`
      );
      console.error(
        "[launcher] coba paksa aset lain: hapus bin/oktz lalu jalankan dengan OKTZ_ASET=" +
          ASET_GLIBC
      );
    }
    if (e.code === "EACCES") {
      console.error(`[launcher] tidak punya izin eksekusi: jalankan chmod +x ${BIN_ABS}`);
    }
    process.exit(127);
  });

  anak.on("exit", (code, signal) => {
    if (signal) {
      console.log(`[launcher] anak berhenti karena ${signal}`);
      process.exit(0);
    }
    process.exit(code ?? 1);
  });
}

/** Jalankan anak dan tunggu selesai (dipakai pairing, sebelum run). */
function spawnTunggu(args) {
  console.log(`[launcher] jalankan ${BIN_ABS} ${args.join(" ")}`);
  return new Promise((resolve) => {
    const c = spawn(BIN_ABS, args, {
      stdio: "inherit",
      env: process.env,
      cwd: AKAR,
    });
    anak = c;
    c.on("error", (e) => {
      console.error(`[launcher] gagal menjalankan ${BIN_ABS}: ${e.message}`);
      resolve(127);
    });
    c.on("exit", (code, signal) => resolve(signal ? 0 : (code ?? 1)));
  });
}

/** Pairing code/QR, lalu jalankan bot dengan sesi yang baru dibuat. */
async function pairing(creds) {
  const konf = bacaKonfig();
  const nomor = (process.env.PAIRING_NUMBER || konf.pairingNumber || "").replace(/\D/g, "");
  const pakaiKode =
    process.env.USE_PAIRING_CODE !== undefined
      ? process.env.USE_PAIRING_CODE !== "false"
      : konf.usePairingCode !== false;
  const db = path.resolve(
    AKAR,
    process.env.OKTZ_DB_DIR || "database/wa",
    "oktz-run.db"
  );

  const args = pakaiKode ? ["pair-code", nomor, db] : ["pair-qr", db];
  if (pakaiKode && !nomor) {
    console.error(
      "[launcher] belum ada sesi dan pairingNumber kosong. Isi config.js " +
        "session.pairingNumber atau env PAIRING_NUMBER."
    );
    process.exit(2);
  }
  console.log(
    `[launcher] belum ada sesi; mulai pairing ${pakaiKode ? `code ${nomor}` : "QR"}`
  );
  const rc = await spawnTunggu(args);
  if (rc !== 0) {
    console.error(`[launcher] pairing gagal (exit ${rc})`);
    process.exit(rc || 1);
  }
  if (!credsSiap(creds)) {
    console.error("[launcher] pairing selesai tapi creds.json tidak terbentuk");
    process.exit(1);
  }
  jalankan(["run", creds]);
}

async function boot() {
  const creds = pathCreds();

  if (credsSiap(creds)) {
    jalankan(["run", creds]);
    return;
  }

  const turso = bacaTurso();
  if (turso.aktif) {
    try {
      const fixture = await pulihkanTurso(creds, turso);
      console.log("[launcher] sesi dipulihkan dari Turso");
      jalankan(["run", creds], fixture ? { OKTZ_SESSION_FIXTURE: fixture } : {});
      return;
    } catch (e) {
      console.error(`[launcher] restore Turso gagal: ${e.message}`);
    }
  }

  // Creds ada tapi tidak sah (rusak/belum paired): karantina seperti JS,
  // supaya pairing tidak menimpa berkas yang mungkin masih bisa diselamatkan.
  if (existsSync(creds) && !credsSiap(creds)) {
    const tujuan = `${creds}.broken-${Date.now()}`;
    try {
      renameSync(creds, tujuan);
      console.error(`[launcher] creds tidak sah dipindah ke ${tujuan}`);
    } catch (e) {
      console.error(`[launcher] gagal memindahkan creds rusak: ${e.message}`);
    }
  }

  await pairing(creds);
}

// Panel mengirim SIGTERM saat stop; teruskan supaya bot menutup socket rapi.
for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => anak && anak.kill(sig));
}

if (ARGS.length > 0) {
  jalankan(ARGS);
} else {
  await boot();
}

async function unduh(nama = ASET) {
  console.log(`[launcher] binary belum ada, unduh ${nama} dari GitHub Release ${REPO}`);
  // URL unduh langsung, bukan API: IP panel sering kena rate limit 403.
  const dasar = `https://github.com/${REPO}/releases/latest/download`;
  let b = await fetch(`${dasar}/${nama}`);
  if (!b.ok && nama === ASET_STATIS) {
    console.error(
      `[launcher] rilis terbaru belum punya ${ASET_STATIS} (HTTP ${b.status}), pakai ${ASET_GLIBC}`
    );
    nama = ASET_GLIBC;
    b = await fetch(`${dasar}/${nama}`);
  }
  if (!b.ok) throw new Error(`HTTP ${b.status} saat unduh ${nama}`);
  const isi = Buffer.from(await b.arrayBuffer());
  if (isi.length === 0) throw new Error(`aset ${nama} kosong`);
  mkdirSync(path.dirname(BIN), { recursive: true });
  writeFileSync(BIN, isi, { mode: 0o755 });
  console.log(`[launcher] tersimpan ${BIN} (${isi.length} byte)`);
}
