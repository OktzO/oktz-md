import fs from "node:fs";
import path from "node:path";

const QR_FILE_PREFIX = "qr-";
const QR_FILE_SUFFIX = ".png";

/**
 * pairing code kena rate-limit → QR接管 pada socket yang sama.
 * Data QR tetap dikirim library di kedua mode, jadi tidak perlu socket baru.
 */
export function shouldUseQrFallback({
  usePairingCode,
  fallbackEnabled = true,
  rateLimited = false,
}) {
  if (!usePairingCode) return true;
  return Boolean(fallbackEnabled && rateLimited);
}

/**
 * Library memancarkan ref QR baru tiap ~20 detik. Tanpa ini terminal akan
 * dicetak ulang nonstop dan memenuhi layar.
 */
export function shouldRenderQr({ qr, lastRendered }) {
  if (typeof qr !== "string" || qr.length === 0) return false;
  if (qr === lastRendered) return false;
  return true;
}

export function canRenderMore({ renderedCount, maxPrints = Infinity }) {
  return renderedCount < maxPrints;
}

export function buildQrFilePath(dir, stamp = Date.now()) {
  return path.join(dir, `${QR_FILE_PREFIX}${stamp}${QR_FILE_SUFFIX}`);
}

/**
 * Hanya file QR buatan kita yang dihapus; file lain di folder itu tidak boleh
 * ikut hilang.
 */
export function pruneOldQrFiles(dir, keep = 3) {
  let names;
  try {
    names = fs.readdirSync(dir);
  } catch {
    return;
  }

  const qrFiles = [];
  for (const name of names) {
    if (!name.startsWith(QR_FILE_PREFIX) || !name.endsWith(QR_FILE_SUFFIX)) {
      continue;
    }
    const full = path.join(dir, name);
    let mtimeMs;
    try {
      mtimeMs = fs.statSync(full).mtimeMs;
    } catch {
      continue;
    }
    qrFiles.push({ full, mtimeMs });
  }

  if (qrFiles.length <= keep) return;

  qrFiles.sort((a, b) => b.mtimeMs - a.mtimeMs);
  for (const { full } of qrFiles.slice(keep)) {
    try {
      fs.rmSync(full, { force: true });
    } catch { }
  }
}
