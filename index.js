import path from "path";
import fs from "fs";
import config from "./config.js";

const _origInfo = console.info;
console.info = (...args) => {
  if (args[0] && typeof args[0] === "string" && args[0].startsWith("Closing session:")) return;
  return _origInfo(...args);
};
import { startConnection } from "./src/connection.js";
import {
  messageHandler,
  groupHandler,
  messageUpdateHandler,
  groupSettingsHandler,
  handleAntiRemoveFromUpsert,
} from "./src/handler.js";
import { loadPlugins, pluginStore } from "./src/lib/plugins.js";
import { initDatabase, getDatabase } from "./src/lib/database.js";
import {
  initScheduler,
  loadScheduledMessages,
  startGroupScheduleChecker,
  startSewaChecker,
} from "./src/lib/scheduler.js";
import { handleAntiTagSW } from "./src/lib/group-protection.js";
import { initSholatScheduler } from "./src/lib/sholat-scheduler.js";
import { initNotifScheduler } from "./src/lib/notif-scheduler.js";
import { initAutoJpmScheduler } from "./src/lib/auto-jpm.js";
import { startMemoryMonitor } from "./src/lib/memory-monitor.js";
import { evictOldestOverCap } from "./src/lib/cache-cap.js";
import { startTempCleaner } from "./src/lib/temp-cleaner.js";
import { startDailyPruner } from "./src/lib/data-pruner.js";
import { initAutoClearScheduler } from "./src/lib/chat-cleaner.js";
import { preloadAssets } from "./src/lib/asset-manager.js";
import {
  logger,
  c,
  playBootSequence,
  spinText,
  logConnection,
  logErrorBox,
  divider,
} from "./src/lib/logger.js";

await import("./src/lib/agent.js")
  .then((m) => m.initializeAgent())
  .catch(() => { });

const LOG_NOISE = new Set([
  "prekey",
  "_chains",
  "registrationId",
  "chainKey",
  "ephemeralKeyPair",
  "rootKey",
  "indexInfo",
  "pendingPreKey",
  "currentRatchet",
  "baseKey",
  "privKey",
  "Session already",
  "SessionEntry",
]);

function _isNoise(args) {
  const first = typeof args[0] === "string" ? args[0] : "";
  for (const noise of LOG_NOISE) {
    if (first.includes(noise)) return true;
  }
  return false;
}

const _log = console.log;
const _info = console.info;
const _warn = console.warn;

console.log = (...args) => {
  if (_isNoise(args)) return;
  _log.apply(console, args);
};

console.info = (...args) => {
  if (_isNoise(args)) return;
  _info.apply(console, args);
};

console.warn = (...args) => {
  if (_isNoise(args)) return;
  _warn.apply(console, args);
};

const startTime = Date.now();

let pluginWatcher = null;
const reloadDebounce = new Map();
const fileStatCache = new Map();
const FILESTAT_CACHE_CAP = 800;

function startDevWatcher(pluginsPath) {
  if (pluginWatcher) pluginWatcher.close();

  logger.system("dev", "Hot-Reload watcher active for plugins");

  pluginWatcher = fs.watch(
    pluginsPath,
    { recursive: true },
    (eventType, filename) => {
      if (!filename || !filename.endsWith(".js")) return;

      const existingTimeout = reloadDebounce.get(filename);
      if (existingTimeout) clearTimeout(existingTimeout);

      const timeout = setTimeout(async () => {
        reloadDebounce.delete(filename);
        const fullPath = path.join(pluginsPath, filename);

        if (!fs.existsSync(fullPath)) {
          fileStatCache.delete(fullPath);
          const pluginName = path.basename(filename, ".js");
          const { unloadPlugin } = await import("./src/lib/plugins.js");
          const result = unloadPlugin(pluginName);
          if (result.success) logger.warn("plugin", `removed ${filename}`);
          return;
        }

        try {
          const stats = fs.statSync(fullPath);
          const cached = fileStatCache.get(fullPath);
          const changed =
            !cached ||
            cached.mtimeMs !== stats.mtimeMs ||
            cached.size !== stats.size;
          if (!changed) return;

          fileStatCache.set(fullPath, {
            mtimeMs: stats.mtimeMs,
            size: stats.size,
          });
          evictOldestOverCap(fileStatCache, FILESTAT_CACHE_CAP);

          const { hotReloadPlugin } =
            await import("./src/lib/plugins.js");
          const result = await hotReloadPlugin(fullPath);
          if (!result.success) {
            logger.error(
              "plugin",
              `reload failed: ${filename}: ${result.error}`,
            );
          }
        } catch (error) {
          logger.error(
            "plugin",
            `reload failed: ${filename}: ${error.message}`,
          );
        }
      }, 500);

      reloadDebounce.set(filename, timeout);
    },
  );

  logger.debug("dev", `Monitoring directory: ${pluginsPath}`);
}

let srcWatcher = null;

function startSrcWatcher(srcPath) {
  if (srcWatcher) srcWatcher.close();

  logger.system("dev", "Hot-Reload watcher active for src");

  srcWatcher = fs.watch(srcPath, { recursive: true }, (eventType, filename) => {
    if (!filename || !filename.endsWith(".js")) return;

    const existingTimeout = reloadDebounce.get("src_" + filename);
    if (existingTimeout) clearTimeout(existingTimeout);

    const timeout = setTimeout(() => {
      reloadDebounce.delete("src_" + filename);
      const fullPath = path.join(srcPath, filename);
      if (!fs.existsSync(fullPath)) {
        logger.warn("dev", `src file removed: ${filename}`);
        return;
      }
      logger.success("dev", `src changed: ${filename}`);
    }, 500);

    reloadDebounce.set("src_" + filename, timeout);
  });

  logger.debug("dev", `Monitoring directory: ${srcPath}`);
}

// ---------------------------------------------------------------------------
// Fault handling (B5).
//
// Dua masalah lama di sini:
//
//  1. Handler-nya menelan SEMUA error lalu `return`, jadi process lanjut jalan
//     dalam keadaan tidak terdefinisi. Fault yang tidak dikenal dan berulang
//     (plugin cycle, socket lifecycle, timer yang salah) = loop diam yang tidak
//     pernah terlihat. Sekarang ada ambang: fault yang TIDAK termasuk
//     allow-list transien, sebanyak MAX_FAULTS_IN_WINDOW dalam FAULT_WINDOW_MS,
//     menghentikan engine. Fault pertama tetap hanya dilaporkan.
//  2. `unhandledRejection` mencetak nilai yang ditolak SECARA UTUH ke stdout.
//     Nilai itu bisa apa saja — objek dari wire yang berisi isi chat, Buffer,
//     atau objek yang menyimpan kredensial. Sekarang hanya ringkasan terikat
//     dan ter-redaksi yang dicetak (summarizeFault), dan argumen kedua
//     (nilai yang sudah settle) tidak pernah dicetak sama sekali.
// ---------------------------------------------------------------------------

const FAULT_SUMMARY_MAX = 400;
const FAULT_STACK_MAX = 1200;
const FAULT_KEYS_MAX = 24;
const MAX_FAULTS_IN_WINDOW = 5;
const FAULT_WINDOW_MS = 60_000;

// Noise jaringan. Bukan fault: tidak dihitung ke ambang, tidak dilaporkan.
const TRANSIENT_FAULT_CODES = [
  "write EOF",
  "ECONNRESET",
  "EPIPE",
  "ETIMEDOUT",
  "ENOTFOUND",
  "ECONNREFUSED",
  "read ECONNRESET",
];

// Pola kredensial yang harus hilang dari log sebelum ditulis.
const SECRET_PATTERNS = [
  /\b(?:gh[pousr]_|sk-(?:live|test)|xox[baprs]-|AKIA)[A-Za-z0-9_-]{8,}/g,
  /\bbearer\s+[A-Za-z0-9._-]{8,}/gi,
  /([?&](?:access_token|api_key|apikey|token|key|secret|password)=)[^&\s"']+/gi,
  /((?:password|passwd|pwd|secret|token|api[_-]?key|apikey|auth|session|cookie)\s*[:=]\s*)("[^"]*"|'[^']*'|[^\s,;&}]+)/gi,
];
const REDACTED = "[redacted]";

function redactSecrets(text) {
  let out = String(text ?? "");
  for (const pattern of SECRET_PATTERNS) {
    pattern.lastIndex = 0;
    out = out.replace(pattern, (match, prefix) =>
      prefix ? `${prefix}${REDACTED}` : REDACTED,
    );
  }
  return out;
}

/**
 * Ringkasan satu nilai fault: terikat panjangnya, ter-redaksi, dan aman
 * terhadap objek siklik / Proxy yang meledak saat diinspeksi.
 *
 * Untuk objek, yang dicetak hanya nama field — bukan nilainya. Objek dari wire
 * bisa berisi apa saja, dan "nilai objek" adalah tempat kebocoran paling sering
 * terjadi.
 */
function summarizeFault(value) {
  try {
    if (value === null) return "null";
    if (value === undefined) return "undefined";

    const type = typeof value;
    if (type === "string") {
      return redactSecrets(value).slice(0, FAULT_SUMMARY_MAX);
    }
    if (type === "number" || type === "boolean" || type === "bigint") {
      return `${type} ${String(value)}`;
    }
    if (type === "function") {
      return `function ${value.name || "(anonymous)"}`;
    }
    if (type === "symbol") return `symbol ${String(value)}`;

    if (Buffer.isBuffer(value)) {
      return `[Buffer ${value.length} bytes]`;
    }
    if (value instanceof Error) {
      const label = `${value.name || "Error"}${value.code ? ` [${String(value.code)}]` : ""}`;
      const detail = [
        value.message === undefined ? "" : String(value.message),
        typeof value.stack === "string" ? value.stack : "",
      ]
        .filter(Boolean)
        .join("\n");
      return redactSecrets(`${label}: ${detail}`).slice(0, FAULT_STACK_MAX);
    }
    if (Array.isArray(value)) {
      return `[Array ${value.length} items]`;
    }

    const keys = Object.keys(value);
    const shown = keys.slice(0, FAULT_KEYS_MAX).join(", ");
    const rest = keys.length - FAULT_KEYS_MAX;
    const more = rest > 0 ? `, +${rest} more` : "";
    const kind = value.constructor?.name || "Object";
    return `${kind} with ${keys.length} key(s): [${shown}${more}]`;
  } catch {
    return "[uninspectable fault value]";
  }
}

function isTransientFault(value) {
  if (!value) return false;
  if (typeof value === "string") {
    return TRANSIENT_FAULT_CODES.some((needle) => value.includes(needle));
  }
  if (typeof value.code === "string" && TRANSIENT_FAULT_CODES.includes(value.code)) {
    return true;
  }
  if (typeof value.message !== "string") return false;
  return TRANSIENT_FAULT_CODES.some((needle) => value.message.includes(needle));
}

function setupAntiCrash(options = {}) {
  const { onFatal = null } = options;

  // ponytail: guard 2 sinyal berturut-turut — SIGINT kedua = force exit,
  // supaya shutdown yang menggantung tetap bisa diinterupsi user.
  let shuttingDown = false;
  const gracefulShutdown = async (signal) => {
    if (shuttingDown) {
      logger.warn("system", "forced exit (second signal)");
      process.exit(1);
    }
    shuttingDown = true;
    console.log("");
    logger.system("system", `Received ${signal} signal`);
    logger.info("database", "Saving data to local storage...");
    try {
      const db = getDatabase();
      // WAJIB await: tanpa ini process.exit memotong flush Turso → boot
      // berikutnya data Turso stale MENIMPA file lokal yang lebih baru.
      await db.save();
      logger.success("database", "All data successfully saved");
    } catch (error) {
      logger.warn("database", `save failed: ${error.message}`);
    }
    logger.info("system", "Engine stopped safely");
    process.exit(0);
  };

  const faultTimestamps = [];
  let fatalFired = false;

  const stopEngine = (why) => {
    if (fatalFired) return;
    fatalFired = true;
    if (onFatal) {
      try {
        onFatal(why);
      } catch (error) {
        logger.error("system", `onFatal failed: ${error.message}`);
      }
      return;
    }
    logErrorBox(
      "too many faults",
      `stopping after ${faultTimestamps.length} faults in ${FAULT_WINDOW_MS / 1000}s — the state can no longer be trusted`,
    );
    gracefulShutdown(why).catch(() => process.exit(1));
  };

  // true kalau fault ini sudah melewati ambang.
  const countFault = () => {
    const now = Date.now();
    while (
      faultTimestamps.length &&
      now - faultTimestamps[0] >= FAULT_WINDOW_MS
    ) {
      faultTimestamps.shift();
    }
    faultTimestamps.push(now);
    return faultTimestamps.length >= MAX_FAULTS_IN_WINDOW;
  };

  function handleUncaughtException(error, origin) {
    if (isTransientFault(error)) return;

    logErrorBox(
      "uncaught exception",
      redactSecrets(error?.message ?? "(no message)").slice(0, FAULT_SUMMARY_MAX),
    );
    logger.error(
      "system",
      `uncaught exception (${origin || "unknown origin"}): ${summarizeFault(error)}`,
    );

    if (countFault()) {
      stopEngine("repeated uncaught exceptions");
      return;
    }
    logger.system("system", "Engine is still running");
  }

  function handleUnhandledRejection(value) {
    if (isTransientFault(value)) return;

    // Ringkasan saja. Nilai yang ditolak tidak pernah dicetak utuh, dan
    // argumen kedua (nilai yang sudah settle) sengaja diabaikan: justru
    // di situ payload/data user dan objek berkredensial berada.
    logErrorBox("unhandled rejection", summarizeFault(value));

    if (countFault()) {
      stopEngine("repeated unhandled rejections");
      return;
    }
    logger.system("system", "Engine is still running");
  }

  process.on("uncaughtException", handleUncaughtException);

  process.on("unhandledRejection", handleUnhandledRejection);

  process.on("warning", (warning) => {
    logger.warn("system", `${warning.name}: ${warning.message}`);
  });

  process.on("SIGINT", () => gracefulShutdown("STOP (SIGINT)"));
  process.on("SIGTERM", () => gracefulShutdown("TERMINATE (SIGTERM)"));

  logger.success("system", "Anti-Crash Protection is Active");

  return { handleUncaughtException, handleUnhandledRejection, stopEngine };
}

async function main() {
  await playBootSequence({
    name: config.bot?.name || "Foto-AI",
    version: config.bot?.version || "1.0.0",
    developer: config.bot?.developer || "Developer",
    mode: config.mode || "public",
  });
  setupAntiCrash();

  const dbPath = path.join(
    process.cwd(),
    config.database?.path || "./database/main",
  );
  await initDatabase(dbPath);
  const db = getDatabase();

  await spinText("system", "Starting local asset cache server...", { tone: "accent" });
  await preloadAssets(config.assets);

  const savedMode = db.setting("botMode");
  if (savedMode && (savedMode === "self" || savedMode === "public"))
    config.mode = savedMode;
  const savedPremium = db.setting("premiumUsers");
  if (Array.isArray(savedPremium)) config.premiumUsers = savedPremium;
  const savedBanned = db.setting("bannedUsers");
  if (Array.isArray(savedBanned)) config.bannedUsers = savedBanned;

  const pCount = Array.isArray(savedPremium) ? savedPremium.length : 0;
  const bCount = Array.isArray(savedBanned) ? savedBanned.length : 0;
  logger.success(
    "database",
    `Database initialized | Mode: ${config.mode} | Premium: ${pCount} | Banned: ${bCount}`,
  );

  const pluginsPath = path.join(process.cwd(), "plugins");
  const pluginCount = await loadPlugins(pluginsPath);
  logger.success("plugin", `${pluginCount} modules loaded successfully`);

  if (config.dev?.enabled && config.dev?.watchPlugins)
    startDevWatcher(pluginsPath);
  if (config.dev?.enabled && config.dev?.watchSrc) {
    const srcPath = path.join(process.cwd(), "src");
    startSrcWatcher(srcPath);
  }

  initScheduler(config);

  const bootTime = Date.now() - startTime;
  logger.success("boot", `System initialized in ${bootTime}ms`);
  divider();
  await spinText("network", "Opening WhatsApp connection tunnel...", {
    duration: 900,
    tone: "accent",
  });
  logConnection("connecting", "Establishing session and handshake protocol");
  console.log("");

  await startConnection({
    onRawMessage: async (msg, sock) => {
      try {
        const db = getDatabase();
        await handleAntiTagSW(msg, sock, db);
      } catch (error) { }
    },

    onMessage: async (msg, sock) => {
      let timeoutId;
      try {
        const handlerPromise = messageHandler(msg, sock);
        const timeoutPromise = new Promise((_, reject) => {
          timeoutId = setTimeout(
            () => reject(new Error("Handler timeout")),
            60000,
          );
        });
        await Promise.race([handlerPromise, timeoutPromise]);
      } catch (error) {
        if (error.message !== "Handler timeout") {
          logger.error("HANDLER", error.message);
          if (config.dev?.debugLog) console.error(c.gray(error.stack));
        }
      } finally {
        clearTimeout(timeoutId);
      }
    },

    onGroupUpdate: async (update, sock) => {
      try {
        await groupHandler(update, sock);
      } catch (error) {
        logger.error("GROUP", error.message);
      }
    },

    // Root cause welcome/goodbye mati: event 'group-participants.update'
    // (connection.js) memanggil options.onParticipantsUpdate, tapi callback
    // ini tidak pernah di-pass — groupHandler (welcome/goodbye/promote/dll)
    // tidak pernah menerima event add/remove.
    onParticipantsUpdate: async (update, sock) => {
      try {
        await groupHandler(update, sock);
      } catch (error) {
        logger.error("PARTICIPANTS", error.message);
      }
    },

    onMessageUpdate: async (updates, sock) => {
      try {
        await messageUpdateHandler(updates, sock);
      } catch (error) {
        logger.error("MSG", error.message);
      }
    },

    onGroupSettingsUpdate: async (update, sock) => {
      try {
        await groupSettingsHandler(update, sock);
      } catch (error) {
        logger.error("GROUP", error.message);
      }
    },

    onStubMessage: async (msg, sock) => {
      try {
        const db = getDatabase();
        await handleAntiRemoveFromUpsert(msg, sock, db);
      } catch (error) {
        logger.error("ANTIDELETE", error.message);
      }
    },

    onConnectionUpdate: async (update, sock) => {
      if (update.connection === "open") {
        logConnection("connected", sock.user?.name || "Bot");
        loadScheduledMessages(sock);
        startGroupScheduleChecker(sock);
        startSewaChecker(sock);
        initScheduler(config, sock);
        initAutoJpmScheduler(sock);
        initSholatScheduler(sock);
        initNotifScheduler(sock);
        initAutoClearScheduler(sock);

        try {
          const { getAllJadibotSessions, isJadibotActive, restartJadibotSession } =
            await import("./src/lib/jadibot-manager.js");
          const sessions = getAllJadibotSessions().filter(
            (s) => !isJadibotActive(s.jid),
          );
          if (sessions.length > 0) {
            logger.info("JADIBOT", `Restoring ${sessions.length} session(s)`);
            for (const session of sessions) {
              try {
                await restartJadibotSession(sock, session.id);
                await new Promise((r) => setTimeout(r, 3000));
              } catch (e) {
                logger.error(
                  "JADIBOT",
                  `Failed restore ${session.id}: ${e.message}`,
                );
              }
            }
          }
        } catch (e) {
          logger.error("JADIBOT", `Gagal memulihkan: ${e.message}`);
        }

        const devLabel = config.dev?.enabled ? ` ${c.yellow("• dev")}` : "";
        startMemoryMonitor();
        startTempCleaner();
        startDailyPruner();
        logger.success("ready", `All subsystems are fully operational${devLabel}`);
        divider();
      }
    },
  });
}

// `node --test` (dan harness yang meng-import file ini untuk menguji
// setupAntiCrash) mengisi NODE_TEST_CONTEXT di process-nya sendiri. Import
// index.js = import SEMUA modul boot bot; kalau main() ikut jalan di sana,
// harness-nya yang tersalut oleh koneksi WhatsApp sungguhan. Jadi boot hanya
// jalan di luar test runner.
if (!process.env.NODE_TEST_CONTEXT) {
  main().catch((error) => {
    logErrorBox("Fatal Error", error.message);
    console.error(c.gray(error.stack));
    process.exit(1);
  });
}

export { setupAntiCrash, summarizeFault, MAX_FAULTS_IN_WINDOW };
