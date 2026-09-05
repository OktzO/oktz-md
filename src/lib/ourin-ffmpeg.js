/**
 * Utilitas resolusi path binary ffmpeg/ffprobe.
 *
 * Root cause: banyak modul (brat-canvas, plugin yang memanggil execFile("ffmpeg"))
 * menjalankan ffmpeg berdasarkan PATH. Di environment produksi ffmpeg tidak pernah
 * terpasang di PATH, sehingga spawn gagal dengan `ENOENT` dan command berakhir
 * diam-diam di blok catch.
 *
 * Modul ini menjadi SATU sumber kebenaran:
 *   1. hormati FFMPEG_PATH / FFPROBE_PATH bila diset operator
 *   2. pakai ffmpeg yang sudah ada di PATH
 *   3. fallback ke binary bawaan @ffmpeg-installer
 *
 * Selain mengembalikan path absolut, ia juga mendaftarkan direktori binary tersebut
 * ke process.env.PATH. Ini penting karena library pihak ketiga (mis. brat-canvas)
 * memanggil `spawn("ffmpeg")` dan tidak bisa diinstruksikan memakai path absolut.
 */

import fs from "fs";
import path from "path";
import { createRequire } from "module";

let cachedFfmpeg = null;
let cachedFfprobe = null;
let pathInjected = false;

function isExecutable(candidate) {
  if (!candidate || typeof candidate !== "string") return false;
  try {
    fs.accessSync(candidate, fs.constants.X_OK);
    return fs.statSync(candidate).isFile();
  } catch {
    return false;
  }
}

function resolveInstallerBinary() {
  try {
    const require = createRequire(import.meta.url);
    const installer = require("@ffmpeg-installer/ffmpeg");
    return installer?.path ?? null;
  } catch {
    return null;
  }
}

function findOnPath(binaryName) {
  const dirs = (process.env.PATH || "").split(path.delimiter).filter(Boolean);
  for (const dir of dirs) {
    const candidate = path.join(dir, binaryName);
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

function resolveBinary(binaryName, envKey) {
  // 1. override eksplisit dari operator
  const fromEnv = process.env[envKey];
  if (isExecutable(fromEnv)) return fromEnv;

  // 2. binary yang memang sudah tersedia di PATH
  const onPath = findOnPath(binaryName);
  if (onPath) return onPath;

  // 3. fallback ke binary bawaan @ffmpeg-installer
  const installerPath = resolveInstallerBinary();
  if (installerPath) {
    const sibling =
      binaryName === "ffprobe"
        ? installerPath.replace(/ffmpeg(\.exe)?$/, "ffprobe$1")
        : installerPath;
    if (isExecutable(sibling)) return sibling;
  }

  return null;
}

/**
 * Tambahkan direktori yang berisi binary ffmpeg/ffprobe ke process.env.PATH.
 * Aman dipanggil berulang kali (idempotent).
 */
export function ensureFfmpegOnPath() {
  if (pathInjected) return process.env.PATH || "";

  const dirsToAdd = [];
  const ffmpegPath = getFfmpegPath();
  const ffprobePath = getFfprobePath();

  for (const bin of [ffmpegPath, ffprobePath]) {
    if (!bin) continue;
    const dir = path.dirname(bin);
    if (!dirsToAdd.includes(dir)) dirsToAdd.push(dir);
  }

  const current = process.env.PATH || "";
  const existing = current.split(path.delimiter).filter(Boolean);
  const missing = dirsToAdd.filter((d) => !existing.includes(d));

  if (missing.length > 0) {
    process.env.PATH = [...missing, ...existing].join(path.delimiter);
  }

  pathInjected = true;
  return process.env.PATH;
}

/**
 * Path absolut ke binary ffmpeg, atau null bila tidak ditemukan.
 */
export function getFfmpegPath() {
  if (cachedFfmpeg === null) {
    cachedFfmpeg = resolveBinary("ffmpeg", "FFMPEG_PATH") ?? "";
  }
  return cachedFfmpeg || null;
}

/**
 * Path absolut ke binary ffprobe, atau null bila tidak ditemukan.
 */
export function getFfprobePath() {
  if (cachedFfprobe === null) {
    cachedFfprobe =
      resolveBinary("ffprobe", "FFPROBE_PATH") ??
      getFfmpegPath()?.replace(/ffmpeg(\.exe)?$/, "ffprobe$1") ??
      "";
    if (cachedFfprobe && !isExecutable(cachedFfprobe)) cachedFfprobe = "";
  }
  return cachedFfprobe || null;
}

/**
 * Helper untuk spawn/exec: hasilkan perintah yang pasti bisa dieksekusi.
 * Mengembalikan { command, args } sehingga pemanggil tidak perlu menggabungkan
 * string berisi spasi (sumber bug quoting).
 */
export function buildFfmpegCommand(args = [], binary = "ffmpeg") {
  const bin = binary === "ffprobe" ? getFfprobePath() : getFfmpegPath();
  if (!bin) {
    throw new Error(
      `Binary ${binary} tidak ditemukan. Install ffmpeg atau set env FFMPEG_PATH.`,
    );
  }
  return { command: bin, args };
}

export default {
  getFfmpegPath,
  getFfprobePath,
  ensureFfmpegOnPath,
  buildFfmpegCommand,
};
