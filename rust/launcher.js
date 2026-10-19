#!/usr/bin/env node
// Launcher Node.js untuk binary Rust `oktz` (panel egg Node.js).
// Argumen diteruskan; tanpa argumen = mode `run`. Sinyal dan exit code
// diteruskan ke/dari anak supaya panel bisa membedakan stop vs crash.
//
//   node launcher.js [argumen oktz...]
//   OKTZ_BIN=/path/ke/oktz node launcher.js
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

const BIN = process.env.OKTZ_BIN || path.join("target", "release", "oktz");
const ARGS = process.argv.slice(2);
const CHILD_ARGS = ARGS.length > 0 ? ARGS : ["run"];

if (!existsSync(BIN)) {
  console.error(`binary ${BIN} tidak ada. Build dulu: cargo build --release`);
  process.exit(127);
}

console.log(`[launcher] jalankan ${BIN} ${CHILD_ARGS.join(" ")}`);
const child = spawn(BIN, CHILD_ARGS, { stdio: "inherit", env: process.env });

// Panel mengirim SIGTERM saat stop; teruskan supaya bot menutup socket rapi.
for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => child.kill(sig));
}

child.on("error", (e) => {
  console.error(`[launcher] gagal menjalankan ${BIN}: ${e.message}`);
  process.exit(127);
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.log(`[launcher] anak berhenti karena ${signal}`);
    process.exit(0);
  }
  process.exit(code ?? 1);
});
