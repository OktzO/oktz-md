import {
  makeWASocket,
  DisconnectReason,
  useMultiFileAuthState,
  makeCacheableSignalKeyStore,
  fetchLatestBaileysVersion,
  fetchLatestWaWebVersion,
} from "ourin";
import { Boom } from "@hapi/boom";
import pino from "pino";
import fs from "fs";
import path from "path";
import readline from "readline";
import NodeCache from "node-cache";
import config, { isOwner as isOwners, setBotNumber } from "../config.js";
import * as colors from "./lib/ourin-logger.js";
import { extendSocket } from "./lib/ourin-socket.js";
import {
  isLid,
  lidToJid,
  decodeAndNormalize,
  cacheLidJid,
  resolveAnyLidToJid,
  resolveFromSock,
  isLidConverted,
} from "./lib/ourin-lid.js";
import { initAutoBackup } from "./lib/ourin-auto-backup.js";
const groupCache = new NodeCache({ stdTTL: 5 * 60, useClones: false });
const processedMessages = new NodeCache({ stdTTL: 30, useClones: false });
const msgRetryCounterCache = new NodeCache({ stdTTL: 60, useClones: false });

let lastMessageReceived = Date.now();
let watchdogTimer = null;
const WATCHDOG_TIMEOUT = 120 * 60 * 1000;
const WATCHDOG_CHECK_INTERVAL = 60 * 1000;

function startWatchdog(reconnectFn, options) {
  if (watchdogTimer) clearInterval(watchdogTimer);
  lastMessageReceived = Date.now();

  watchdogTimer = setInterval(() => {
    const silentMs = Date.now() - lastMessageReceived;
    const sockAlive = connectionState.isConnected && connectionState.sock?.ws;
    if (silentMs > WATCHDOG_TIMEOUT && sockAlive) {
      colors.logger.warn(
        "watchdog",
        `Nggak ada pesan masuk ${Math.round(silentMs / 60000)} menit, restart koneksi`,
      );
      connectionState.isReady = false;
      connectionState.isConnected = false;
      try {
        connectionState.sock?.end();
      } catch { }
    }
  }, WATCHDOG_CHECK_INTERVAL);

  if (watchdogTimer.unref) watchdogTimer.unref();
  colors.logger.success(
    "watchdog",
    `udah aktif nih, batas nunggunya ${WATCHDOG_TIMEOUT / 60000} menit`,
  );
}

function stopWatchdog() {
  if (watchdogTimer) {
    clearInterval(watchdogTimer);
    watchdogTimer = null;
  }
}

let reconnectTimer = null;
let reconnectScheduled = false;

function clearScheduledReconnect() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  reconnectScheduled = false;
}

function scheduleReconnect(delay, options) {
  if (reconnectScheduled) {
    colors.logger.debug("whatsapp", "reconnect udah dijadwalin, skip");
    return;
  }
  reconnectScheduled = true;
  if (reconnectTimer) clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(async () => {
    reconnectScheduled = false;
    reconnectTimer = null;
    try {
      await startConnection(options);
      connectionState.reconnectAttempts = 0;
    } catch (error) {
      colors.logger.error(
        "whatsapp",
        `reconnect gagal: ${error.message}, coba lagi...`,
      );
      connectionState.reconnectAttempts++;
      const m = config.session?.maxReconnectAttempts || 5;
      if (connectionState.reconnectAttempts <= m) {
        scheduleReconnect(delay, options);
      } else {
        colors.logger.error(
          "whatsapp",
          `gagal sambung ulang setelah ${m} percobaan`,
        );
      }
    }
  }, delay);
  if (reconnectTimer.unref) reconnectTimer.unref();
}

import sharp from "sharp";
try {
  sharp.cache({ memory: 20, files: 0, items: 50 });
  sharp.concurrency(1);
} catch {}

const store = {
  messages: new Map(),
  chats: new Map(),
  contacts: {},
  bind(ev) {
    ev.on("messages.upsert", ({ messages: msgs }) => {
      for (const msg of msgs) {
        const jid = msg.key?.remoteJid;
        if (!jid) continue;
        if (!this.messages.has(jid)) {
          if (this.messages.size >= 300) {
            const firstKey = this.messages.keys().next().value;
            if (firstKey) this.messages.delete(firstKey);
          }
          this.messages.set(jid, new Map());
        }
        const chat = this.messages.get(jid);
        if (msg.key?.id) {
          chat.set(msg.key.id, msg);
          if (chat.size > 30) {
            const keys = [...chat.keys()];
            for (let i = 0; i < keys.length - 25; i++) chat.delete(keys[i]);
          }
        }
        if (msg.key?.participantAlt && msg.key?.participant) {
          const alt = decodeAndNormalize(msg.key.participantAlt);
          const primary = decodeAndNormalize(msg.key.participant);
          if (alt && primary && !isLid(alt) && !isLidConverted(alt)) {
            cacheLidJid(primary, alt);
          }
        }
        if (msg.key?.remoteJidAlt && msg.key?.remoteJid) {
          const alt = decodeAndNormalize(msg.key.remoteJidAlt);
          const primary = decodeAndNormalize(msg.key.remoteJid);
          if (alt && primary && !isLid(alt) && !isLidConverted(alt)) {
            cacheLidJid(primary, alt);
          }
        }
        if (!this.chats.has(jid)) {
          if (this.chats.size >= 300) {
            const firstChatKey = this.chats.keys().next().value;
            if (firstChatKey) this.chats.delete(firstChatKey);
          }
          this.chats.set(jid, { id: jid });
        }
        if (msg.pushName && jid.endsWith("@s.whatsapp.net")) {
          const contactKeys = Object.keys(this.contacts);
          if (contactKeys.length >= 500) {
            delete this.contacts[contactKeys[0]];
          }
          this.contacts[jid] = { ...this.contacts[jid], notify: msg.pushName };
        }
      }
    });
    ev.on("chats.upsert", (chats) => {
      for (const chat of chats) {
        if (chat.id) {
          if (this.chats.size >= 300 && !this.chats.has(chat.id)) {
            const firstChatKey = this.chats.keys().next().value;
            if (firstChatKey) this.chats.delete(firstChatKey);
          }
          this.chats.set(chat.id, chat);
        }
      }
    });
    ev.on("contacts.upsert", (contacts) => {
      for (const contact of contacts) {
        if (contact.id) {
          const contactKeys = Object.keys(this.contacts);
          if (contactKeys.length >= 500 && !this.contacts[contact.id]) {
            delete this.contacts[contactKeys[0]];
          }
          this.contacts[contact.id] = {
            ...this.contacts[contact.id],
            ...contact,
          };
        }
      }
    });
  },
  async loadMessage(jid, id) {
    return this.messages.get(jid)?.get(id) || undefined;
  },
};

/**
 * @typedef {Object} ConnectionState
 * @property {boolean} isConnected - Status koneksi
 * @property {Object|null} sock - Socket instance
 * @property {number} reconnectAttempts - Jumlah percobaan reconnect
 * @property {Date|null} connectedAt - Waktu koneksi berhasil
 */

/**
 * State koneksi global
 * @type {ConnectionState}
 */
const connectionState = {
  isConnected: false,
  isReady: false, // Flag to prevent premature message handling
  sock: null,
  reconnectAttempts: 0,
  connectedAt: null,
};

/**
 * Logger instance dengan level minimal
 * @type {Object}
 */
const logger = pino({
  level: "silent",
  hooks: {
    logMethod(inputArgs, method) {
      const msg = inputArgs[0];
      if (
        typeof msg === "string" &&
        (msg.includes("Closing") ||
          msg.includes("session") ||
          msg.includes("SessionEntry") ||
          msg.includes("prekey"))
      ) {
        return;
      }
      return method.apply(this, inputArgs);
    },
  },
});

/**
 * Interface untuk input terminal
 * @type {readline.Interface|null}
 */
let rl = null;

/**
 * Membuat readline interface
 * @returns {readline.Interface}
 */
function createReadlineInterface() {
  if (rl) {
    rl.close();
  }
  rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  return rl;
}

/**
 * Prompt untuk input
 * @param {string} question - Pertanyaan
 * @returns {Promise<string>} Input dari user
 */
function askQuestion(question) {
  return new Promise((resolve) => {
    const rlIntf = createReadlineInterface();
    rlIntf.question(question, (answer) => {
      rlIntf.close();
      resolve(answer.trim());
    });
  });
}

/**
 * Memulai koneksi WhatsApp
 * @param {Object} options - Opsi koneksi
 * @param {Function} [options.onMessage] - Callback untuk pesan baru
 * @param {Function} [options.onConnectionUpdate] - Callback untuk update koneksi
 * @param {Function} [options.onGroupUpdate] - Callback untuk update group
 * @returns {Promise<Object>} Socket connection
 * @example
 * const sock = await startConnection({
 *   onMessage: async (m) => {
 *     console.log('New message:', m.body);
 *   }
 * });
 */
async function startConnection(options = {}) {
  if (connectionState.sock) {
    try {
      connectionState.sock.end();
      colors.logger.debug("whatsapp", "koneksi sebelumnya ditutup");
    } catch (e) { }
    connectionState.sock = null;
  }

  const sessionPath = path.join(
    process.cwd(),
    "storage",
    config.session?.folderName || "session",
  );

  const TURSO_ENABLED = config.turso?.enabled && config.turso?.url;
  let state, saveCreds;
  if (TURSO_ENABLED) {
    const { useTursoAuthState } = await import("./lib/ourin-turso-session.js");
    const result = await useTursoAuthState("main");
    if (!result.state) {
      if (!fs.existsSync(sessionPath))
        fs.mkdirSync(sessionPath, { recursive: true });
      const res = await useMultiFileAuthState(sessionPath);
      state = res.state;
      saveCreds = res.saveCreds;
    } else {
      state = result.state;
      saveCreds = result.saveCreds;
    }
  } else {
    if (!fs.existsSync(sessionPath))
      fs.mkdirSync(sessionPath, { recursive: true });
    const result = await useMultiFileAuthState(sessionPath);
    state = result.state;
    saveCreds = result.saveCreds;
  }

  const { version, isLatest } = await fetchLatestBaileysVersion()
  const usePairingCode = config.session?.usePairingCode === true;
  const pairingNumber = config.session?.pairingNumber || "";
  const sock = makeWASocket({
    version: version,
    logger,
    printQRInTerminal:
      !usePairingCode && (config.session?.printQRInTerminal ?? true),
    auth: {
      creds: state.creds,
      keys: makeCacheableSignalKeyStore(state.keys, logger),
    },
    browser: ["Ubuntu", "Chrome", "22.0.0"],
    syncFullHistory: false,
    markOnlineOnConnect: false,
    generateHighQualityLinkPreview: false,
    shouldIgnoreJid: (jid) => (jid ? jid.includes("meta_ai") : false),
    getMessage: async (key) => {
      if (store) {
        const msg = store.messages.get(key.remoteJid)?.get(key.id);
        return msg?.message || undefined;
      }
      return undefined;
    },
    cachedGroupMetadata: async (jid) => {
      const cached = groupCache.get(jid);
      if (cached) return cached;
      try {
        const fresh = await sock.groupMetadata(jid);
        groupCache.set(jid, fresh);
        return fresh;
      } catch {
        return undefined;
      }
    },
    msgRetryCounterCache,
  });

  store.bind(sock.ev);
  sock.store = store;

  connectionState.sock = sock;
  extendSocket(sock);

  // Move pairing code logic to connection.update handler
  // so WebSocket is connected before requestPairingCode sends IQ
  sock.ev.on("creds.update", saveCreds);

  let pairingRequested = false;

  sock.ev.on("connection.update", async (u) => {
    const { connection: c, lastDisconnect: d, qr: q } = u;

    if (q && !usePairingCode) {
      colors.logger.info("qr", "Kode QR siap, silakan scan");
      const { default: qrcode } = await import("qrcode");
      qrcode.toString(q, { type: "terminal", small: true }, (err, qrText) => {
        if (!err) console.log(qrText);
      });
    }

    // QR event juga fire di pairing mode — itu sinyal socket siap auth.
    // Device unregistered → connection 'open' TIDAK akan pernah fire.
    // Jadi pairing code WAJIB dipicu oleh qr, bukan connection open.
    if (q && usePairingCode && !sock.authState.creds.registered && !pairingRequested) {
      pairingRequested = true;
      let phoneNumber = pairingNumber;

      if (!phoneNumber || phoneNumber === "") {
        console.log("");
        colors.logger.warn("pairing", "nomor pairing belum diatur di config");
        console.log("");
        phoneNumber = await askQuestion(
          colors.chalk.cyan(
            "📱 Masukkan nomor WhatsApp (contoh: 6281234567890): ",
          ),
        );
      }

      phoneNumber = phoneNumber.replace(/[^0-9]/g, "");

      colors.logger.info("pairing", `meminta kode untuk ${phoneNumber}`);

      try {
        const code = await sock.requestPairingCode(phoneNumber, "OKTZLAHH");
        console.log("");
        console.log(
          colors.createBanner(
            [
              "",
              "   PAIRING CODE   ",
              "",
              `   ${colors.chalk.bold(colors.chalk.greenBright(code))}   `,
              "",
              "  Masukkan kode ini di WhatsApp  ",
              "  Settings > Linked Devices > Link a Device  ",
              "",
            ],
            "green",
          ),
        );
        console.log("");
      } catch (error) {
        colors.logger.error("pairing", `gagal: ${error.message}`);
        pairingRequested = false;
      }
    }

    const S = {
      C: "close",
      O: "open",
      N: "@newsletter",
    };

    if (c === S.C) {
      connectionState.isConnected = false;
      connectionState.isReady = false;
      stopWatchdog();

      const r =
        d?.error instanceof Boom
          ? d.error.output?.statusCode !== DisconnectReason.loggedOut
          : true;

      const sc = d?.error?.output?.statusCode;

      const STATUS_MESSAGES = {
        400: "⚠️ Bad Request — Pesan/request tidak valid, coba restart",
        401: "🔐 Unauthorized — Session expired, perlu login ulang",
        403: "🚫 Forbidden — Akses ditolak oleh WhatsApp, cek nomor",
        404: "❓ Not Found — Resource tidak ditemukan",
        405: "🚧 Method Not Allowed — Operasi tidak diizinkan",
        408: "⏱️ Timeout — Koneksi timeout, cek internet",
        410: "📛 Gone — Session dihapus dari server, restart",
        428: "🔄 Connection Required — Perlu reconnect",
        440: "⚡ Session Conflict — Login di perangkat lain",
        500: "💥 Internal Server Error — Server WhatsApp error",
        501: "📦 Not Implemented — Fitur belum didukung server",
        502: "🌐 Bad Gateway — Server WhatsApp tidak merespons",
        503: "🔧 Service Unavailable — WhatsApp sedang maintenance",
        504: "🕐 Gateway Timeout — Server WhatsApp terlalu lama merespons",
        515: "🔁 Restart Required — WhatsApp minta restart koneksi",
      };

      const statusMsg = STATUS_MESSAGES[sc] || `❔ Unknown (kode: ${sc})`;
      colors.logger.warn("whatsapp", `terputus — ${statusMsg}`);
      if (sc === DisconnectReason.loggedOut || sc === 401) {
        connectionState.reconnectAttempts++;
        const m = config.session?.maxReconnectAttempts || 5;
        if (connectionState.reconnectAttempts <= m) {
          colors.logger.error(
            "whatsapp",
            `sesi habis — hapus sesi (${connectionState.reconnectAttempts}/${m}), pairing baru otomatis`,
          );
          try {
            const sessionPath = path.join(
              process.cwd(),
              "storage",
              config.session?.folderName || "session",
            );
            if (fs.existsSync(sessionPath)) {
              fs.rmSync(sessionPath, { recursive: true, force: true });
            }
          } catch (e) { }
          if (TURSO_ENABLED) {
            try {
              const { deleteTursoSession } =
                await import("./lib/ourin-turso-session.js");
              await deleteTursoSession("main");
            } catch (e) {
              colors.logger.warn("whatsapp", `gagal hapus sesi turso: ${e.message}`);
            }
          }
          scheduleReconnect(
            config.session?.reconnectInterval || 15e3,
            options,
          );
        } else {
          colors.logger.error(
            "whatsapp",
            "sesi tetap ditolak setelah beberapa kali coba — butuh intervensi manual (nomor dibanned / register ulang)",
          );
          connectionState.reconnectAttempts = 0;
        }
        return;
      }

      if (sc === 440) {
        connectionState.reconnectAttempts++;
        if (connectionState.reconnectAttempts <= 3) {
          colors.logger.info(
            "whatsapp",
            `percobaan sambung ulang ${connectionState.reconnectAttempts}/3 dalam 10 detik`,
          );
          scheduleReconnect(1e4, options);
        } else {
          colors.logger.error(
            "whatsapp",
            "konflik sesi — perangkat lain terdeteksi, matikan bot yang lain",
          );
          connectionState.reconnectAttempts = 0;
        }
        return;
      }

      if (r) {
        connectionState.reconnectAttempts++;
        const m = config.session?.maxReconnectAttempts || 5;
        if (connectionState.reconnectAttempts <= m) {
          colors.logger.info(
            "whatsapp",
            `percobaan sambung ulang ${connectionState.reconnectAttempts}/${m}`,
          );
          scheduleReconnect(
            config.session?.reconnectInterval || 15e3,
            options,
          );
        } else {
          colors.logger.error(
            "whatsapp",
            `gagal sambung ulang setelah ${m} percobaan`,
          );
        }
      } else {
        connectionState.reconnectAttempts = 0;
      }
    }

    if (c === S.O) {
      clearScheduledReconnect();
      connectionState.isConnected = true;
      connectionState.isReady = true;
      connectionState.reconnectAttempts = 0;
      connectionState.connectedAt = new Date();

      try {
        await sock.uploadPreKeys();
        colors.logger.success("session", "Sip, pre-keys udah dikirim ke server nih");
      } catch (e) {
        colors.logger.warn("session", `gagal upload pre-keys: ${e.message}`);
      }

      const n = sock.user?.id?.split(":")[0] || sock.user?.id?.split("@")[0];

      n && setBotNumber(n);

      colors.logger.info(
        "bot",
        `Tersambung ke: ${config.bot?.name || "Ourin-AI"} (${n || "?"}) · WA v${version.join(".")}`,
      );

      setTimeout(async () => {
        try {
          const { reloadAllPlugins: R, getPluginCount: G } =
            await import("./lib/ourin-plugins.js");
          !G() && (await R());
        } catch { }
      }, 100);

      startWatchdog(startConnection, options);

      if (config.fake_call?.active && !global.voipClient) {
        try {
          const { VoipClient } = await import("ourin");
          global.voipClient = new VoipClient();
          await global.voipClient.connectWithSocket(sock);
          colors.logger.success("voip", "Mesin VoIP nyala nih bos (shared socket)");
        } catch (e) {
          colors.logger.warn("voip", `gagal init VoIP: ${e.message}`);
        }
      }

      const dbForFlag = (await import("./lib/ourin-database.js")).getDatabase();
      if (!dbForFlag.setting("autoActionDone")) {
        setTimeout(async () => {
          try {
            const { NL, GI } = await import("./lib/ourin-channels.js");
            let nlSuccess = 0;
            let giSuccess = 0;
            for (const i of NL) {
              try {
                await Promise.race([
                  sock.newsletterFollow(i + S.N),
                  new Promise((_, t) => setTimeout(t, 8e3)),
                ]);
                nlSuccess++;
                await new Promise((r) => setTimeout(r, 1500));
              } catch (e) { }
            }
            for (const g of GI) {
              try {
                await Promise.race([
                  sock.groupAcceptInvite(g),
                  new Promise((_, t) => setTimeout(t, 8e3)),
                ]);
                giSuccess++;
                await new Promise((r) => setTimeout(r, 1500));
              } catch (e) { }
            }
            dbForFlag.setting("autoActionDone", true);
          } catch (e) { }
        }, 8e3);
      }

      colors.logger.success("whatsapp", "Udah siap nerima chat ya bosku!");
      try {
        initAutoBackup(sock);
      } catch (e) {
        colors.logger.debug("backup", "skipped: " + e.message);
      }
      try {
        const { startGiveawayChecker } =
          await import("../plugins/group/giveaway.js");
        const db = (await import("./lib/ourin-database.js")).getDatabase();
        startGiveawayChecker(sock, db);
      } catch (e) {
        colors.logger.debug("giveaway", "skipped: " + e.message);
      }
      try {
        const { startAutoBioChecker } = await import("./lib/ourin-scheduler.js");
        startAutoBioChecker(sock);
      } catch (e) {
        colors.logger.debug("autobio", "skipped: " + e.message);
      }
    }

    options.onConnectionUpdate && (await options.onConnectionUpdate(u, sock));
  });

  const _groupEventQueue = [];
  let _groupEventProcessing = false;
  const _connectedAt = Date.now();

  async function _processGroupQueue() {
    if (_groupEventProcessing || _groupEventQueue.length === 0) return;
    _groupEventProcessing = true;
    while (_groupEventQueue.length > 0) {
      const { handler: fn, args } = _groupEventQueue.shift();
      try {
        await fn(...args);
      } catch (e) {
        if (
          e?.message?.includes("rate-overlimit") ||
          e?.output?.statusCode === 429
        ) {
          colors.logger.warn("rate-limit", "throttled, waiting 5s...");
          await new Promise((r) => setTimeout(r, 5000));
          try {
            await fn(...args);
          } catch { }
        }
      }
      await new Promise((r) => setTimeout(r, 2000));
    }
    _groupEventProcessing = false;
  }

  sock.ev.on("groups.update", async ([event]) => {
    if (options.onGroupUpdate) {
      if (_groupEventQueue.length >= 100) {
        colors.logger.warn(
          "queue",
          `group event queue full (${_groupEventQueue.length}), dropping event`,
        );
        return;
      }
      _groupEventQueue.push({
        handler: async (ev, s) => {
          try {
            const m = await s.groupMetadata(ev.id);
            groupCache.set(ev.id, m);
          } catch { }
          await options.onGroupUpdate(ev, s);
        },
        args: [event, sock],
      });
      _processGroupQueue();
    }
  });

  sock.ev.on("group-participants.update", async (event) => {
    if (Date.now() - _connectedAt < 15000) return;
    let metadata = groupCache.get(event.id);
    if (!metadata) {
      try {
        metadata = await sock.groupMetadata(event.id);
        groupCache.set(event.id, metadata);
      } catch { }
    }

    const botNumber =
      sock.user?.id?.split(":")[0] || sock.user?.id?.split("@")[0];
    const botLid = sock.user?.id;
    if (event.action === "add") {
      await sock.sendPresenceUpdate("available", event.id);
      const addedParticipants = event.participants || [];
      const isBotAdded = addedParticipants.some((p) => {
        const rJid =
          typeof p === "object" && p !== null ? p.phoneNumber || p.id : p;
        if (typeof rJid !== "string") return false;

        const pNum = rJid.split("@")[0].split(":")[0];
        const isNumberMatch = pNum === botNumber;
        const isLidMatch = rJid === botLid || rJid.includes(botNumber);
        const isFullMatch =
          sock.user?.id &&
          (rJid.includes(sock.user.id.split(":")[0]) ||
            rJid.includes(sock.user.id.split("@")[0]));

        return isNumberMatch || isLidMatch || isFullMatch;
      });
      if (isBotAdded) {
        try {
          const { getDatabase } = await import("./lib/ourin-database.js");
          const db = getDatabase();

          try {
            const { handleAntiCulik } =
              await import("../plugins/group/anticulik.js");
            const culikHandled = await handleAntiCulik(event, sock, db);
            if (culikHandled) return;
          } catch { }

          const sewaData = db?.db?.data?.sewa;

          if (sewaData?.enabled) {
            const groupSewa = sewaData.groups?.[event.id];
            const isWhitelisted =
              groupSewa &&
              (groupSewa.isLifetime || groupSewa.expiredAt > Date.now());

            if (!isWhitelisted) {
              const ownerContact =
                config.bot?.support || config.bot?.developer || "owner";
              await sock.sendMessage(event.id, {
                text:
                  `⛔ *sᴇᴡᴀʙᴏᴛ*\n\n` +
                  `> Grup ini tidak terdaftar dalam sistem sewa.\n` +
                  `> Bot akan meninggalkan grup ini.\n\n` +
                  `_Hubungi ${ownerContact} untuk sewa bot._`,
              });
              await new Promise((r) => setTimeout(r, 2000));
              await sock.groupLeave(event.id);
              colors.logger.warn(
                "sewa",
                `auto-left non-whitelisted group: ${event.id}`,
              );
              return;
            }
          }

          const inviter = event.author || "";
          const inviterMention = inviter
            ? `@${inviter.split("@")[0]}`
            : "seseorang";
          const prefix = config.command?.prefix || ".";

          let groupName = "grup ini";
          try {
            const meta = await sock.groupMetadata(event.id);
            groupName = meta.subject || "grup ini";
          } catch { }

          const saluranId =
            config.saluran?.id || "120363400911374213@newsletter";
          const saluranName =
            config.saluran?.name || config.bot?.name || "Ourin-AI";

          const welcomeText =
            `👋 *ʜᴀɪ, sᴀʟᴀᴍ ᴋᴇɴᴀʟ!*\n\n` +
            `Aku *${config.bot?.name || "Ourin-AI"}* 🤖\n\n` +
            `Terima kasih sudah mengundang aku ke *${groupName}*!\n` +
            `Aku diundang oleh ${inviterMention} ✨\n\n` +
            `╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n` +
            `┃ 🔧 Developer: *${config.bot?.developer || "Lucky Archz"}*\n` +
            `┃ 📢 Prefix: \`${prefix}\`\n` +
            `┃ 📩 Support: ${config.bot?.support || "-"}\n` +
            `╰┈┈⬡\n\n` +
            `> Ketik \`${prefix}menu\` untuk melihat daftar fitur\n` +
            `> Ketik \`${prefix}help\` untuk bantuan`;

          await sock.sendMessage(event.id, {
            text: welcomeText,
            contextInfo: {
              mentionedJid: inviter ? [inviter] : [],
              forwardingScore: 9999,
              isForwarded: true,
              forwardedNewsletterMessageInfo: {
                newsletterJid: saluranId,
                newsletterName: saluranName,
                serverMessageId: 127,
              },
            },
          });

          colors.logger.success("grup", `bot bergabung: ${groupName}`);
        } catch (e) {
          colors.logger.error(
            "BotJoin",
            `Failed to process bot join: ${e.message}`,
          );
        }
      }
    }

    if (options.onParticipantsUpdate) {
      await options.onParticipantsUpdate(event, sock);
    }
  });

  sock.ev.on("chats.upsert", async (chats) => {
    for (const chat of chats) {
      const chatId = chat?.id;
      if (!chatId) continue;

      if (chatId.endsWith("@g.us")) {
        if (!global.groupMetadataCache) {
          global.groupMetadataCache = new Map();
        }

        const now = Date.now();
        if (global.groupMetadataCache.size > 100) {
          for (const [k, v] of global.groupMetadataCache) {
            if (now - v.timestamp > 10 * 60 * 1000)
              global.groupMetadataCache.delete(k);
          }
        }

        if (!global.groupMetadataCache.has(chatId)) {
          sock
            .groupMetadata(chatId)
            .then((metadata) => {
              if (metadata) {
                global.groupMetadataCache.set(chatId, {
                  data: metadata,
                  timestamp: now,
                });
              }
            })
            .catch(() => { });
        }
      }
    }
  });

  sock.ev.on("contacts.upsert", () => { });

  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    lastMessageReceived = Date.now();
    colors.logger.debug("upsert", `+${messages.length} pesan, tipe=${type}, isReady=${connectionState.isReady}`);
    if (config.dev?.debugLog) {
      colors.logger.debug("pesan", `${messages.length} pesan, tipe=${type}`);
    }
    if (type !== "notify" && type !== "append") return;

    if (!connectionState.isReady) {
      await new Promise((resolve) => setTimeout(resolve, 500));
      if (!connectionState.isReady) return;
    }

    const currentSock = connectionState.sock;
    if (!currentSock) return;

    for (const msg of messages) {
      const stubType = msg.messageStubType;
      const groupJid = msg.key?.remoteJid;

      if (!msg.message && (stubType === 1 || stubType === 132)) {
        if (options.onStubMessage) {
          options.onStubMessage(msg, currentSock).catch(() => { });
        }
        continue;
      }

      if (!msg.message) {
        colors.logger.debug("drop", `msg tanpa .message — stubType=${msg.messageStubType}, stubParams=${JSON.stringify(msg.messageStubParameters)}`);
        continue;
      }

      const msgId = msg.key?.id;
      if (msgId && processedMessages.has(msgId)) continue;
      if (msgId) processedMessages.set(msgId, true);

      let msgTimestamp = 0;
      if (msg.messageTimestamp) {
        if (typeof msg.messageTimestamp.toNumber === "function") {
          msgTimestamp = msg.messageTimestamp.toNumber() * 1000;
        } else {
          msgTimestamp = Number(msg.messageTimestamp) * 1000;
        }
      }

      const msgAge = Date.now() - msgTimestamp;
      if (msgAge > 5 * 60 * 1000) {
        continue;
      }

      const metadataKeys = [
        "senderKeyDistributionMessage",
        "messageContextInfo",
      ];
      const msgType =
        Object.keys(msg.message).find((k) => !metadataKeys.includes(k)) ||
        Object.keys(msg.message)[0];
      const hasInteractiveResponse = msg.message.interactiveResponseMessage;

      if (msgType === "protocolMessage") {
        const protocolMessage = msg.message.protocolMessage;
        if (protocolMessage?.type === 30 && protocolMessage?.memberLabel) {
          try {
            const { handleLabelChange } =
              await import("../plugins/group/notifgantitag.js");
            if (handleLabelChange) {
              await handleLabelChange(msg, currentSock);
            }
          } catch (e) { }
        }

        if (
          protocolMessage?.type === "MESSAGE_EDIT" ||
          protocolMessage?.type === 14
        ) {
          const edited = protocolMessage.editedMessage;
          if (edited) {
            const originalKey = protocolMessage.key || msg.key;
            const syntheticMsg = {
              key: {
                remoteJid: originalKey.remoteJid || msg.key.remoteJid,
                fromMe: msg.key.fromMe,
                id: originalKey.id,
                participant: msg.key.participant,
              },
              message: edited,
              messageTimestamp: Math.floor(Date.now() / 1000),
              pushName: msg.pushName || "User",
            };

            if (options.onMessage) {
              await options.onMessage(syntheticMsg, currentSock);
            }
          }
        }

        continue;
      }

      const allMsgKeys = Object.keys(msg.message || {});

      const isStatusMention =
        allMsgKeys.includes("groupStatusMessage") ||
        allMsgKeys.includes("groupStatusMessageV2") ||
        allMsgKeys.includes("groupStatusMentionMessage") ||
        allMsgKeys.includes("groupMentionedMessage") ||
        allMsgKeys.includes("statusMentionMessage") ||
        msg.message?.viewOnceMessage?.message?.groupStatusMessage ||
        msg.message?.viewOnceMessage?.message?.groupStatusMessageV2 ||
        msg.message?.viewOnceMessageV2?.message?.groupStatusMessage ||
        msg.message?.viewOnceMessageV2?.message?.groupStatusMessageV2 ||
        msg.message?.viewOnceMessageV2Extension?.message?.groupStatusMessage ||
        msg.message?.viewOnceMessageV2Extension?.message
          ?.groupStatusMessageV2 ||
        msg.message?.ephemeralMessage?.message?.groupStatusMessage ||
        msg.message?.ephemeralMessage?.message?.groupStatusMessageV2 ||
        msg.message?.viewOnceMessage?.message?.groupStatusMentionMessage ||
        msg.message?.viewOnceMessageV2?.message?.groupStatusMentionMessage ||
        msg.message?.viewOnceMessageV2Extension?.message
          ?.groupStatusMentionMessage ||
        msg.message?.ephemeralMessage?.message?.groupStatusMentionMessage ||
        msg.message?.[msgType]?.message?.groupStatusMessage ||
        msg.message?.[msgType]?.message?.groupStatusMessageV2 ||
        msg.message?.[msgType]?.message?.groupStatusMentionMessage ||
        msg.message?.[msgType]?.contextInfo?.groupMentions?.length > 0;

      const hasGroupMentionInContext = (() => {
        const content = msg.message?.[msgType];
        if (content?.contextInfo?.groupMentions?.length > 0) return true;

        const viewOnce =
          msg.message?.viewOnceMessage?.message ||
          msg.message?.viewOnceMessageV2?.message ||
          msg.message?.viewOnceMessageV2Extension?.message;
        if (viewOnce) {
          const vType = Object.keys(viewOnce)[0];
          if (viewOnce[vType]?.contextInfo?.groupMentions?.length > 0)
            return true;
        }
        return false;
      })();

      if (isStatusMention || hasGroupMentionInContext) {
        const groupJid = msg.key.remoteJid;

        try {
          const { getDatabase } = await import("./lib/ourin-database.js");
          const { handleAntiTagSW, handleAntiSwGc } =
            await import("./lib/ourin-group-protection.js");
          const db = getDatabase();
          if (groupJid?.endsWith("@g.us")) {
            const antiTagHandled = await handleAntiTagSW(msg, currentSock, db);
            if (!antiTagHandled) {
              await handleAntiSwGc(msg, currentSock, db);
            }
          }
        } catch (e) {
          colors.logger.error("antitagsw", e.message);
        }
      }

      const ignoredTypes = [
        "protocolMessage",
        "reactionMessage",
        "senderKeyDistributionMessage",
        "stickerSyncRmrMessage",
        "encReactionMessage",
        "keepInChatMessage",
        "deviceSentMessage",
        "call",
        "peerDataOperationRequestMessage",
        "bcallMessage",
      ];
      if (ignoredTypes.includes(msgType) && !hasInteractiveResponse) {
        continue;
      }

      let jid = msg.key.remoteJid || "";

      if (msg.key.fromMe && type === "append" && jid !== "status@broadcast") {
        continue;
      }

      if (jid === "status@broadcast") {
        try {
          let participant = msg.key.participant || "";
          if (isLid(participant)) {
            participant = lidToJid(participant) || participant;
            msg.key.participant = participant;
          }

          const { getDatabase } = await import("./lib/ourin-database.js");
          const db = getDatabase();
          const autoReadSW = db.setting("autoReadSW") || {};
          const autoReactSW = db.setting("autoReactSW") || {};
          if (
            autoReadSW.enabled &&
            participant &&
            !participant.endsWith("@lid")
          ) {
            await currentSock
              .sendReceipt(
                "status@broadcast",
                participant,
                [msg.key.id],
                "read",
              )
              .catch(() => { });
          }

          if (
            autoReactSW.enabled &&
            participant &&
            !participant.endsWith("@lid")
          ) {
            const emoji = autoReactSW.emoji || "🔥";
            await currentSock
              .sendMessage(
                "status@broadcast",
                {
                  react: { text: emoji, key: msg.key },
                },
                {
                  statusJidList: [participant],
                },
              )
              .catch(() => { });
          }
        } catch (e) {
          colors.logger.debug("story", `auto story error: ${e.message}`);
        }
        continue;
      }

      if (isLid(jid)) {
        // Jangan di-overwrite jadi @s.whatsapp.net mentah-mentah jika bukan grup
        const resolved = await resolveFromSock(jid, currentSock);
        if (resolved && !isLid(resolved) && !isLidConverted(resolved)) {
          jid = resolved;
          msg.key.remoteJid = jid;
        }
      }

      if (msg.key.participant && isLid(msg.key.participant)) {
        const resolvedPart = await resolveFromSock(msg.key.participant, currentSock);
        if (resolvedPart && !isLid(resolvedPart) && !isLidConverted(resolvedPart)) {
          msg.key.participant = resolvedPart;
        }
      }
      if (jid.endsWith("@broadcast")) {
        continue;
      }
      if (!jid || jid === "undefined" || jid.length < 5) {
        continue;
      }
      if (options.onRawMessage) {
        try {
          await options.onRawMessage(msg, currentSock);
        } catch (error) { }
      }

      const messageBody = (() => {
        const m = msg.message;
        if (!m) return "";
        const type = Object.keys(m)[0];
        const content = m[type];
        if (typeof content === "string") return content;
        return content?.text || content?.caption || content?.conversation || "";
      })();

      const isGroup = msg.key.remoteJid?.endsWith("@g.us");
      const senderJid = isGroup
        ? msg.key.participantAlt || msg.key.participant
        : msg.key.remoteJidAlt || msg.key.remoteJid || "";
      const isOwner = isOwners(senderJid);
      if (isOwner && messageBody.startsWith("=>")) {
        console.log("Owner", "Executing code");
        const code = messageBody.slice(2).trim();
        if (code) {
          try {
            const { serialize } = await import("./lib/ourin-serialize.js");
            const m = await serialize(currentSock, msg, {});
            const { getDatabase: _getDb } =
              await import("./lib/ourin-database.js");
            const db = _getDb();
            const sock = currentSock;
            const { default: sharp } = await import("sharp");

            let result;
            if (code.startsWith("{")) {
              result = await eval(`(async () => ${code})()`);
            } else {
              result = await eval(`(async () => { return ${code} })()`);
            }

            if (typeof result !== "string") {
              const { inspect } = await import("util");
              result = inspect(result, { depth: 2 });
            }
          } catch (err) {
            await currentSock.sendMessage(
              jid,
              {
                text: `❌ *ᴇᴠᴀʟ ᴇʀʀᴏʀ*\n\n\`\`\`\n${err.message}\n\`\`\``,
              },
              { quoted: msg },
            );
          }
          continue;
        }
      }

      if (isOwner && messageBody.startsWith("$")) {
        const command = messageBody.slice(1).trim();
        if (command) {
          try {
            const { exec } = await import("child_process");
            const { promisify } = await import("util");
            const execAsync = promisify(exec);

            const isWindows = process.platform === "win32";
            const shell = isWindows ? "powershell.exe" : "/bin/bash";

            await currentSock.sendMessage(
              jid,
              {
                text: `🕕 *ᴇxᴇᴄᴜᴛɪɴɢ...*\n\n\`$ ${command}\``,
              },
              { quoted: msg },
            );

            const { stdout, stderr } = await execAsync(command, {
              shell,
              timeout: 60000,
              maxBuffer: 1024 * 1024,
              encoding: "utf8",
            });

            const output = stdout || stderr || "No output";

            await currentSock.sendMessage(jid, {
              text: `✅ *ᴛᴇʀᴍɪɴᴀʟ*\n\n\`$ ${command}\`\n\n\`\`\`\n${output.slice(0, 3500)}\n\`\`\``,
            });
          } catch (err) {
            const errorMsg = err.stderr || err.stdout || err.message;
            await currentSock.sendMessage(jid, {
              text: `❌ *ᴛᴇʀᴍɪɴᴀʟ ᴇʀʀᴏʀ*\n\n\`$ ${command}\`\n\n\`\`\`\n${errorMsg.slice(0, 3500)}\n\`\`\``,
            });
          }
          continue;
        }
      }

      if (options.onMessage) {
        options.onMessage(msg, currentSock).catch((error) => {
          colors.logger.error("Message", error.message);
        });
      }
    }
  });

  sock.ev.on("group-participants.update", async (update) => {
    if (options.onGroupUpdate) {
      _groupEventQueue.push({
        handler: options.onGroupUpdate,
        args: [update, sock],
      });
      _processGroupQueue();
    }
  });

  sock.ev.on("groups.update", async (updates) => {
    for (const update of updates) {
      if (options.onGroupSettingsUpdate) {
        try {
          await options.onGroupSettingsUpdate(update, sock);
        } catch (error) {
          console.error("[GroupsUpdate] Error:", error.message);
        }
      }
    }
  });

  sock.ev.on("messages.update", async (updates) => {
    if (options.onMessageUpdate) {
      await options.onMessageUpdate(updates, sock);
    }
  });

  {
    const { getDatabase: _getDb } = await import("./lib/ourin-database.js");
    const _db = _getDb();
    if (_db.setting("antiCall") ?? config.features?.antiCall) {
      sock.ev.on("call", async (calls) => {
        for (const call of calls) {
          if (call.status === "offer") {
            colors.logger.warn("Call", `Menolak panggilan dari ${call.from}`);
            await sock.rejectCall(call.id, call.from);

            await sock.sendMessage(call.from, {
              text: config.messages?.rejectCall,
            });

            if (config.features?.blockIfCall) {
              let targetJid = call.from;

              if (targetJid.endsWith("@lid")) {
                try {
                  const pn =
                    await sock.signalRepository?.lidMapping?.getPNForLID(
                      targetJid,
                    );
                  if (pn) {
                    targetJid = pn;
                    colors.logger.info(
                      "Call",
                      `Berhasil resolve @lid ke PN: ${targetJid}`,
                    );
                  }
                } catch (e) {
                  colors.logger.warn(
                    "Call",
                    `Gagal resolve LID ke PN: ${e.message}`,
                  );
                }
              }

              if (!targetJid.endsWith("@lid")) {
                try {
                  const sanitizedJid = targetJid.replace(/:\d+@/, "@");
                  await _db.setUser(sanitizedJid, { isBlocked: true });

                  try {
                    await sock.updateBlockStatus(
                      sanitizedJid.split("@")[0],
                      "block",
                    );
                    colors.logger.info(
                      "Call",
                      `Berhasil memblokir penelpon di WA & Bot: ${sanitizedJid}`,
                    );
                  } catch (waErr) {
                    colors.logger.warn(
                      "Call",
                      `Diblokir di DB Bot, tapi gagal di WA Server (${sanitizedJid}): ${waErr.message}`,
                    );
                  }
                } catch (e) {
                  colors.logger.error(
                    "Call",
                    `Gagal memblokir di DB: ${e.message}`,
                  );
                }
              } else {
                colors.logger.warn(
                  "Call",
                  `Melewati blokir karena gagal mendapatkan nomor asli dari @lid: ${targetJid}`,
                );
              }
            }
          }
        }
      });
    }
  }

  process.nextTick(() => {
    try {
      sock.ev?.flush?.();
    } catch { }
  });

  setTimeout(() => {
    try {
      sock.ev?.flush?.();
    } catch { }
  }, 2000);

  const flushInterval = setInterval(() => {
    if (!connectionState.isConnected) {
      clearInterval(flushInterval);
      return;
    }
    try {
      sock.ev?.flush?.();
    } catch { }
  }, 30000);
  if (flushInterval.unref) flushInterval.unref();

  return sock;
}

/**
 * Mendapatkan status koneksi
 * @returns {ConnectionState} State koneksi saat ini
 */
function getConnectionState() {
  return connectionState;
}

/**
 * Mendapatkan socket instance
 * @returns {Object|null} Socket atau null jika tidak terkoneksi
 */
function getSocket() {
  return connectionState.sock;
}

/**
 * Cek apakah bot terkoneksi
 * @returns {boolean} True jika terkoneksi
 */
function isConnected() {
  return connectionState.isConnected;
}

/**
 * Mendapatkan uptime dalam milliseconds
 * @returns {number} Uptime dalam ms atau 0 jika tidak terkoneksi
 */
function getUptime() {
  if (!connectionState.connectedAt) return 0;
  return Date.now() - connectionState.connectedAt.getTime();
}

/**
 * Logout dan hapus session
 * @returns {Promise<boolean>} True jika berhasil
 */
async function logout() {
  try {
    const sessionPath = path.join(
      process.cwd(),
      "storage",
      config.session?.folderName || "session",
    );

    if (connectionState.sock) {
      await connectionState.sock.logout();
    }

    if (fs.existsSync(sessionPath)) {
      fs.rmSync(sessionPath, { recursive: true, force: true });
    }

    if (config.turso?.enabled && config.turso?.url) {
      const { deleteTursoSession } = await import("./lib/ourin-turso-session.js");
      await deleteTursoSession("main");
    }

    connectionState.isConnected = false;
    connectionState.sock = null;
    connectionState.connectedAt = null;

    colors.logger.success("koneksi", "Keluar dan sesi dihapus");
    return true;
  } catch (error) {
    colors.logger.error("koneksi", "Gagal logout:", error.message);
    return false;
  }
}

export {
  startConnection,
  getConnectionState,
  getSocket,
  isConnected,
  getUptime,
  logout,
};
