import { logger } from "./ourin-logger.js";
import config from "../../config.js";
const RSS_LIMIT = 550 * 1024 * 1024;
const GC_RSS_THRESHOLD = 380 * 1024 * 1024;
const GC_HEAP_THRESHOLD = 250 * 1024 * 1024;
const CHECK_INTERVAL = 2 * 60 * 1000;
const OVER_LIMIT_CHECKS = 3;

let monitorTimer = null;
let overLimitStreak = 0;
let warnedOverLimit = false;

function formatMB(bytes) {
  return (bytes / 1024 / 1024).toFixed(1) + "MB";
}

// ponytail: tak ada restart path aman — index.js cuma gracefulShutdown
// (save DB + process.exit) saat sinyal, connection.js cuma reconnect baileys.
// Jadi RSS > limit 3 cek beruntun = warn owner via WA, restart manual.
function evaluateOverLimit(rss) {
  overLimitStreak = rss > RSS_LIMIT ? overLimitStreak + 1 : 0;
  return overLimitStreak >= OVER_LIMIT_CHECKS;
}

async function notifyOwner(rss) {
  try {
    const { getSocket } = await import("../connection.js");
    const sock = getSocket();
    if (!sock) return;
    const ownerNumbers = config.owner?.number || [];
    if (ownerNumbers.length === 0) return;
    const ownerNumber = String(ownerNumbers[0]).replace(/[^0-9]/g, "");
    if (!ownerNumber) return;
    await sock.sendMessage(`${ownerNumber}@s.whatsapp.net`, {
      text:
        `⚠️ *ᴘᴇʀɪɴɢᴀᴛᴀɴ ʀᴀᴍ*\n\n` +
        `> RSS ${formatMB(rss)} > limit ${formatMB(RSS_LIMIT)} selama ${OVER_LIMIT_CHECKS} cek beruntun.\n` +
        `> Bot tidak punya restart otomatis — butuh investigasi + restart manual.`,
    });
  } catch {}
}

function startMemoryMonitor() {
  if (monitorTimer) return;

  monitorTimer = setInterval(() => {
    const mem = process.memoryUsage();

    if (global.gc) {
      if (mem.rss > GC_RSS_THRESHOLD || mem.heapUsed > GC_HEAP_THRESHOLD) {
        logger.warn(
          "memory",
          `heap tinggi · rss ${formatMB(mem.rss)} · heap ${formatMB(mem.heapUsed)} — menjalankan global.gc()`,
        );
        global.gc();
      }
    }

    if (evaluateOverLimit(mem.rss)) {
      if (!warnedOverLimit) {
        warnedOverLimit = true;
        logger.warn(
          "memory",
          `RSS ${formatMB(mem.rss)} melebihi ${formatMB(RSS_LIMIT)} — owner diberitahu`,
        );
        notifyOwner(mem.rss);
      }
    } else {
      warnedOverLimit = false;
    }

    logger.system(
      "memory",
      `rss ${formatMB(mem.rss)} · heap ${formatMB(mem.heapUsed)}/${formatMB(mem.heapTotal)}`,
    );
  }, CHECK_INTERVAL);

  if (monitorTimer.unref) monitorTimer.unref();
  logger.success(
    "memory",
    `Pantau RAM aktif, limit ${formatMB(RSS_LIMIT)}, cek tiap ${CHECK_INTERVAL / 60000} menit`,
  );
}

function stopMemoryMonitor() {
  if (monitorTimer) {
    clearInterval(monitorTimer);
    monitorTimer = null;
  }
}

export { startMemoryMonitor, stopMemoryMonitor, evaluateOverLimit };
