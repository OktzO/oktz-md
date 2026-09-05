/**
 * Utilitas terpusat untuk ffmpeg: resolusi path binary + antrian eksekusi.
 *
 * Bagian 1 — Resolusi path (getFfmpegPath / getFfprobePath / ensureFfmpegOnPath):
 *   Root cause yang diperbaiki: banyak modul (brat-canvas, plugin yang memanggil
 *   execFile("ffmpeg")) menjalankan ffmpeg berdasarkan PATH. Di environment
 *   produksi ffmpeg tidak terpasang di PATH, sehingga spawn gagal dengan
 *   `ENOENT` dan command berakhir diam-diam di blok catch.
 *
 *   Urutan resolusi (satu sumber kebenaran):
 *     1. hormati FFMPEG_PATH / FFPROBE_PATH bila diset operator
 *     2. pakai ffmpeg yang sudah ada di PATH
 *     3. fallback ke binary bawaan @ffmpeg-installer
 *
 * Bagian 2 — queueFFmpeg (dipertahankan dari implementasi sebelumnya):
 *   Antrian eksekusi ffmpeg dengan batas konkurensi, timeout, dan penanganan
 *   stderr. Dipakai oleh plugin convert/tools (audiofx, toaudio, tovn, dll).
 */

import fs from "fs";
import path from "path";
import { createRequire } from "module";
import { cpus } from "os";
import { exec } from "child_process";
import { logger } from "./ourin-logger.js";

/* ================================================================
 * Bagian 1: Resolusi path ffmpeg/ffprobe
 * ================================================================ */

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
 * Aman dipanggil berulang kali (idempotent). Penting untuk library pihak
 * ketiga (mis. brat-canvas) yang memanggil spawn("ffmpeg") dari PATH.
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

/** Path absolut ke binary ffmpeg, atau null bila tidak ditemukan. */
export function getFfmpegPath() {
  if (cachedFfmpeg === null) {
    cachedFfmpeg = resolveBinary("ffmpeg", "FFMPEG_PATH") ?? "";
  }
  return cachedFfmpeg || null;
}

/** Path absolut ke binary ffprobe, atau null bila tidak ditemukan. */
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

/* ================================================================
 * Bagian 2: Antrian eksekusi ffmpeg (implementasi asli, dipertahankan)
 * ================================================================ */

const CONCURRENCY = Math.max(2, cpus().length);
const TIMEOUT = 60_000;

let queue = [];
let running = 0;

function runNext() {
  while (running < CONCURRENCY && queue.length > 0) {
    const task = queue.shift();
    running++;
    task
      .execute()
      .then(task.resolve)
      .catch(task.reject)
      .finally(() => {
        running--;
        runNext();
      });
  }
}

/**
 * Ganti token `ffmpeg` / `ffprobe` di awal command string menjadi path absolut.
 * Contoh: "ffmpeg -y -i a.mp4 out.mp3" -> "/path/ke/ffmpeg -y -i a.mp4 out.mp3".
 * Bila path absolut tidak ditemukan, kembalikan command apa adanya (biar
 * error ENOENT-nya jelas di sisi pemanggil).
 */
function resolveQueueCommand(command) {
  if (typeof command !== "string") return command;
  const bin = command.match(/^\s*(ffmpeg|ffprobe)\b/);
  if (!bin) return command;
  const absolute =
    bin[1] === "ffprobe" ? getFfprobePath() : getFfmpegPath();
  if (!absolute) return command;
  return absolute + command.slice(bin[0].length);
}

function queueFFmpeg(command) {
  return new Promise((resolve, reject) => {
    const execute = () =>
      new Promise((res, rej) => {
        // Panggilan plugin berbentuk string: `ffmpeg -y -i ...`.
        // Di environment produksi `ffmpeg` tidak selalu ada di PATH,
        // jadi awalan command diganti dengan path absolut hasil resolusi.
        const resolvedCommand = resolveQueueCommand(command);
        const child = exec(resolvedCommand, { maxBuffer: 50 * 1024 * 1024 });
        let timedOut = false;
        let stderr = "";

        const timer = setTimeout(() => {
          timedOut = true;
          child.kill("SIGKILL");
        }, TIMEOUT);

        child.stderr?.on("data", (chunk) => {
          stderr += chunk;
          if (stderr.length > 2000) stderr = stderr.slice(-2000);
        });

        child.on("close", (code) => {
          clearTimeout(timer);
          if (timedOut)
            return rej(new Error(`FFmpeg timeout (${TIMEOUT / 1000}s)`));
          if (code !== 0)
            return rej(
              new Error(
                `FFmpeg exit code ${code}: ${stderr.split("\n").pop()}`,
              ),
            );
          res();
        });

        child.on("error", (err) => {
          clearTimeout(timer);
          rej(err);
        });
      });

    queue.push({ execute, resolve, reject });
    runNext();
  });
}

function getQueueStats() {
  return {
    running,
    queued: queue.length,
    concurrency: CONCURRENCY,
  };
}

export { queueFFmpeg, getQueueStats, CONCURRENCY };
export default {
  getFfmpegPath,
  getFfprobePath,
  ensureFfmpegOnPath,
  buildFfmpegCommand,
  queueFFmpeg,
  getQueueStats,
};
