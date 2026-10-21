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
const KANDIDAT = ["bin/oktz", "target/release/oktz"].map((p) => path.join(AKAR, p));
const BIN = process.env.OKTZ_BIN
  ? path.resolve(process.env.OKTZ_BIN)
  : KANDIDAT.find((p) => existsSync(p)) || KANDIDAT[0];
const ARGS = process.argv.slice(2);
// Default binary `run` membaca `../storage/session/creds.json`, path layout
// dev (cwd = rust/). Di panel cwd = folder launcher, jadi creds ada di
// `storage/session/creds.json`. Kirim path eksplisit kalau ketemu, supaya
// tidak bergantung pada layout.
const KANDIDAT_CREDS = [
  path.join(AKAR, "storage/session/creds.json"),
  path.join(AKAR, "../storage/session/creds.json"),
];
const CREDS = KANDIDAT_CREDS.find((p) => existsSync(p));
const CHILD_ARGS = ARGS.length > 0 ? ARGS : CREDS ? ["run", CREDS] : ["run"];

function siap_dipakai(p) {
  if (!existsSync(p)) return false;
  try {
    const s = statSync(p);
    return s.isFile() && s.size > 0;
  } catch {
    return false;
  }
}

if (!siap_dipakai(BIN)) {
  if (process.env.OKTZ_BIN) {
    console.error(`binary ${BIN} tidak ada/kosong (dari OKTZ_BIN)`);
    process.exit(127);
  }
  if (existsSync(BIN)) {
    console.error(`[launcher] ${BIN} kosong atau bukan berkas, unduh ulang`);
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

// Path absolut: spawn dengan path relatif bisa ENOENT kalau cwd panel beda
// dari folder launcher.
const BIN_ABS = path.resolve(BIN);
let anak = null;
let sudahUnduhUlang = false;

function jalankan() {
  console.log(`[launcher] jalankan ${BIN_ABS} ${CHILD_ARGS.join(" ")}`);
  // cwd = akar repo supaya config.json, database/, dan storage/ selalu relatif
  // ke sini, bukan ke direktori kerja panel.
  anak = spawn(BIN_ABS, CHILD_ARGS, {
    stdio: "inherit",
    env: process.env,
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
        jalankan();
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

// Panel mengirim SIGTERM saat stop; teruskan supaya bot menutup socket rapi.
for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => anak && anak.kill(sig));
}

jalankan();

async function unduh(nama = ASET) {
  console.log(`[launcher] binary belum ada, unduh ${nama} dari GitHub Release ${REPO}`);
  const r = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { "User-Agent": "oktz-rs-launcher" },
  });
  if (!r.ok) throw new Error(`HTTP ${r.status} saat cek release terbaru`);
  const rilis = await r.json();
  let aset = (rilis.assets || []).find((a) => a.name === nama);
  if (!aset && nama === ASET_STATIS) {
    console.error(
      `[launcher] rilis ${rilis.tag_name} belum punya ${ASET_STATIS}, pakai ${ASET_GLIBC}`
    );
    nama = ASET_GLIBC;
    aset = (rilis.assets || []).find((a) => a.name === nama);
  }
  if (!aset) throw new Error(`release ${rilis.tag_name} tidak punya aset ${nama}`);
  const b = await fetch(aset.browser_download_url);
  if (!b.ok) throw new Error(`HTTP ${b.status} saat unduh ${nama}`);
  const isi = Buffer.from(await b.arrayBuffer());
  mkdirSync(path.dirname(BIN), { recursive: true });
  writeFileSync(BIN, isi, { mode: 0o755 });
  console.log(`[launcher] tersimpan ${BIN} (${isi.length} byte, ${rilis.tag_name})`);
}
