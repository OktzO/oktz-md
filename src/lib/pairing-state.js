import fs from "node:fs";
import path from "node:path";

const COOLDOWN_FILE = ".pairing-cooldown";

function marker(creds) {
  if (!creds.additionalData || typeof creds.additionalData !== "object") {
    creds.additionalData = {};
  }
  return creds.additionalData;
}

export function isPairingPending(creds) {
  return creds?.additionalData?.pairingPending === true;
}

export function markPairingPending(creds) {
  marker(creds).pairingPending = true;
}

export function clearPairingPending(creds) {
  if (creds?.additionalData && typeof creds.additionalData === "object") {
    creds.additionalData.pairingPending = undefined;
  }
}

export function resetPairingCreds(creds) {
  if (!creds) return;
  creds.me = undefined;
  creds.pairingCode = undefined;
  creds.account = undefined;
  creds.signalIdentities = [];
  creds.registered = false;
  clearPairingPending(creds);
}

export function isAuthenticated(creds) {
  if (!creds) return false;
  // QR pairing writes `account` via configureSuccessfulPairing; pairing code
  // additionally flips `registered` at the companion-finish step. Either one
  // proves the device really got linked, so a 401 there is a real logout.
  return Boolean(creds.account) || creds.registered === true;
}

export function classifyClose(statusCode, creds) {
  if (statusCode === 401) {
    if (isPairingPending(creds)) return "reset-pairing";
    return isAuthenticated(creds) ? "purge-session" : "reset-pairing";
  }
  return "reconnect";
}

// WA kadang kirim 401 palsu tepat setelah handshake. Percaya langsung = hapus
// sesi sah. Referensi: zaileys retry 2x sebelum believe a logout.
export const FALSE_LOGOUT_GRACE_MS = 60 * 1000;
export const FALSE_LOGOUT_RETRIES = 2;

/**
 * Apakah 401 ini boleh dipercaya sebagai logout asli?
 * @returns {"retry"|"trust"}
 */
export function classifyLogout401({
  statusCode,
  creds,
  connectedAt,
  now = Date.now(),
  retries = 0,
  graceMs = FALSE_LOGOUT_GRACE_MS,
  maxRetries = FALSE_LOGOUT_RETRIES,
} = {}) {
  if (statusCode !== 401) return "trust";
  if (!isAuthenticated(creds)) return "trust";
  // baru saja konek → sangat mungkin 401 palsu
  if (connectedAt && now - connectedAt < graceMs && retries < maxRetries) {
    return "retry";
  }
  return "trust";
}

// berapa folder .broken-* yang disimpan; sebelumnya semua dihapus sebelum
// rename, jadi tiap 401 menghapus satu-satunya salinan sesi lama.
export const BROKEN_SESSION_KEEP = 3;

export function pruneBrokenSessions(storageRoot, sessionBaseName, keep = BROKEN_SESSION_KEEP) {
  let removed = 0;
  try {
    const prefix = `${sessionBaseName}.broken-`;
    const entries = fs
      .readdirSync(storageRoot)
      .filter((f) => f.startsWith(prefix))
      // timestamp di nama → urut kronologis
      .sort();
    const excess = entries.length - Math.max(0, keep);
    for (let i = 0; i < excess; i++) {
      try {
        fs.rmSync(path.join(storageRoot, entries[i]), { recursive: true, force: true });
        removed++;
      } catch { }
    }
  } catch { }
  return removed;
}

export function isRateLimitError(error) {
  if (!error) return false;
  const message = typeof error.message === "string" ? error.message : "";
  if (message.includes("rate-overlimit") || message.includes("429")) return true;
  if (error.output?.statusCode === 429) return true;
  if (error.data === 429) return true;
  if (error.data?.statusCode === 429) return true;
  return false;
}

function cooldownPath(dir) {
  return path.join(dir, COOLDOWN_FILE);
}

export function readCooldown(dir) {
  try {
    const raw = fs.readFileSync(cooldownPath(dir), "utf8");
    const parsed = JSON.parse(raw);
    return Number.isFinite(parsed?.until) ? parsed.until : 0;
  } catch {
    return 0;
  }
}

export function writeCooldown(dir, until) {
  try {
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(cooldownPath(dir), JSON.stringify({ until }));
  } catch { }
}

export function clearCooldown(dir) {
  try {
    fs.rmSync(cooldownPath(dir), { force: true });
  } catch { }
}

export function getCooldownRemainingMs(dir, now = Date.now()) {
  const until = readCooldown(dir);
  if (!until || until <= now) {
    clearCooldown(dir);
    return 0;
  }
  return until - now;
}

export function normalizePairingNumber(input) {
  if (typeof input !== "string") return null;
  const digits = input.replace(/[^0-9]/g, "");
  return digits.length > 0 ? digits : null;
}
