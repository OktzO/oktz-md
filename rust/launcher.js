#!/usr/bin/env node
// Launcher Node.js untuk binary Rust `oktz` (panel egg Node.js).
// Kalau bin/oktz belum ada, binary diunduh dari GitHub Release repo ini.
// Path dihitung dari lokasi berkas ini, bukan cwd panel.
// Argumen diteruskan; tanpa argumen = mode `run`. Sinyal dan exit code
// diteruskan ke/dari anak supaya panel bisa membedakan stop vs crash.
//
//   node launcher.js [argumen oktz...]
//   OKTZ_BIN=/path/ke/oktz node launcher.js
import { spawn } from "node:child_process";
import {
  accessSync,
  chmodSync,
  constants,
  existsSync,
  mkdirSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const AKAR = path.dirname(fileURLToPath(import.meta.url));
const REPO = "OktzO/oktz-rs";
const ASET = "oktz-linux-x64";
const KANDIDAT = ["bin/oktz", "target/release/oktz"].map((p) => path.join(AKAR, p));
const BIN = process.env.OKTZ_BIN
  ? path.resolve(process.env.OKTZ_BIN)
  : KANDIDAT.find((p) => existsSync(p)) || KANDIDAT[0];
const ARGS = process.argv.slice(2);
const CHILD_ARGS = ARGS.length > 0 ? ARGS : ["run"];

if (!existsSync(BIN)) {
  if (process.env.OKTZ_BIN) {
    console.error(`binary ${BIN} tidak ada (dari OKTZ_BIN)`);
    process.exit(127);
  }
  try {
    await unduh();
  } catch (e) {
    console.error(`[launcher] gagal unduh binary: ${e.message}`);
    console.error(
      `[launcher] unduh manual: https://github.com/${REPO}/releases/latest (aset ${ASET}), ` +
        `simpan sebagai ${BIN}`
    );
    process.exit(127);
  }
}

// Panel kadang menyimpan berkas tanpa bit eksekusi.
try {
  accessSync(BIN, constants.X_OK);
} catch {
  chmodSync(BIN, 0o755);
}

console.log(`[launcher] jalankan ${BIN} ${CHILD_ARGS.join(" ")}`);
// cwd = akar repo supaya config.json, database/, dan storage/ selalu relatif
// ke sini, bukan ke direktori kerja panel.
const child = spawn(BIN, CHILD_ARGS, { stdio: "inherit", env: process.env, cwd: AKAR });

// Panel mengirim SIGTERM saat stop; teruskan supaya bot menutup socket rapi.
for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => child.kill(sig));
}

child.on("error", (e) => {
  console.error(`[launcher] gagal menjalankan ${BIN}: ${e.message}`);
  if (e.code === "ENOENT") {
    console.error(
      `[launcher] berkas ada tapi loader tidak ketemu — panel harus Linux x86_64 dengan glibc >= 2.34 ` +
        `(Debian 12/Ubuntu 22.04+). Arsitektur panel: ${process.platform}/${process.arch}`
    );
  }
  process.exit(127);
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.log(`[launcher] anak berhenti karena ${signal}`);
    process.exit(0);
  }
  process.exit(code ?? 1);
});

async function unduh() {
  console.log(`[launcher] binary belum ada, unduh dari GitHub Release ${REPO}`);
  const r = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { "User-Agent": "oktz-rs-launcher" },
  });
  if (!r.ok) throw new Error(`HTTP ${r.status} saat cek release terbaru`);
  const rilis = await r.json();
  const aset = (rilis.assets || []).find((a) => a.name === ASET);
  if (!aset) throw new Error(`release ${rilis.tag_name} tidak punya aset ${ASET}`);
  const b = await fetch(aset.browser_download_url);
  if (!b.ok) throw new Error(`HTTP ${b.status} saat unduh ${ASET}`);
  const isi = Buffer.from(await b.arrayBuffer());
  mkdirSync(path.dirname(BIN), { recursive: true });
  writeFileSync(BIN, isi, { mode: 0o755 });
  console.log(`[launcher] tersimpan ${BIN} (${isi.length} byte, ${rilis.tag_name})`);
}
