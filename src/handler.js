import config from "../config.js";
import { isSelf } from "../config.js";
import { generateWAMessageFromContent, prepareWAMessageMedia } from "ourin";
import { serialize, getCachedThumb } from "./lib/ourin-serialize.js";
import { saluranCtx } from "./lib/ourin-context.js";
import {
  getPlugin,
  getPluginCount,
  getAllPlugins,
  pluginStore,
  getAllCommandNames,
} from "./lib/ourin-plugins.js";
import {
  findSimilarCommands,
  formatSuggestionMessage,
} from "./lib/ourin-similarity.js";
import { getDatabase } from "./lib/ourin-database.js";
import {
  formatUptime,
  createWaitMessage,
  createErrorMessage,
} from "./lib/ourin-formatter.js";
import { getUptime } from "./connection.js";
import { logger, logMessage, c } from "./lib/ourin-logger.js";
import {
  isLid,
  isLidConverted,
  lidToJid,
  convertLidArray,
  resolveAnyLidToJid,
  cacheParticipantLids,
  savePersistentCache,
  getLidCacheSize,
} from "./lib/ourin-lid.js";
import { hasActiveSession, getSession } from "./lib/ourin-game-data.js";
import {
  levenshtein,
  formatAfkDuration,
  checkPermission,
  checkMode,
} from "./lib/ourin-middleware.js";
import {
  handleAntilink,
  handleAntiJudol,
  handleAntiPhising,
  handleAntiCustom,
  handleAntiRemove,
  handleAntiRemoveFromUpsert,
  cacheMessageForAntiRemove,
  handleAntilinkGc,
  handleAntilinkAll,
  handleAntiHidetag,
  handleAntiSwGc,
} from "./lib/ourin-group-protection.js";
import {
  debounceMessage,
  getCachedUser,
  getCachedGroup,
  getCachedSetting,
} from "./lib/ourin-performance.js";
import {
  isJadibotOwner,
  isJadibotPremium,
  loadJadibotDb,
} from "./lib/ourin-jadibot-database.js";
import { getActiveJadibots } from "./lib/ourin-jadibot-manager.js";
import { handleCommand as handleCaseCommand } from "../case/ourin.js";
import { RateLimiterMemory } from "rate-limiter-flexible";
import { games as ourinGames } from "./lib/ourin-games.js";
import fs from "fs";
import path from "path";
import { exec } from "child_process";
import axios from "axios";
import * as timeHelper from "./lib/ourin-time.js";
import sharp from "sharp";
const safe = (fn) => {
  try {
    return fn();
  } catch {
    return null;
  }
};

let FormData,
  levelHelper,
  handleBuyerDone,
  registrationAnswerHandler,
  dungeonAnswerHandler,
  kyubigameAnswerHandler,
  family100AnswerHandler,
  pushkontakAnswerHandler,
  anticustomReplyHandler,
  dafontAnswerHandler,
  gantiAssetAnswerHandler,
  srtAnswerHandler,
  checkAfk,
  isMuted,
  detectBot,
  autoStickerHandler,
  autoMediaHandler,
  checkAntidocument,
  checkAntisticker,
  checkAntimedia,
  ytmp4Plugin,
  confessPlugin,
  sulapPlugin,
  handleAutoAI,
  handleAutoDownload,
  checkStickerCommand,
  handleStickerReply,
  sendWelcomeMessage,
  sendGoodbyeMessage,
  autoJoinDetector,
  isMutedMember,
  isMutegc,
  resepAnswerHandler,
  stickerlyAnswerHandler,
  caiAnswerHandler,
  caiChatHandler,
  comparationAnswerHandler;

try {
  FormData = (await import("form-data")).default || (await import("form-data"));
} catch { }
try {
  levelHelper = await import("./lib/ourin-level.js");
} catch { }
try {
  handleBuyerDone = (await import("../plugins/store/done.js")).handleBuyerDone;
} catch { }
try {
  registrationAnswerHandler = (await import("../plugins/user/daftar.js"))
    .registrationAnswerHandler;
} catch { }
try {
  dungeonAnswerHandler = (await import("../plugins/game/dungeon.js"))
    .dungeonAnswerHandler;
} catch { }
try {
  kyubigameAnswerHandler = (await import("../plugins/game/kyubigame.js"))
    .kyubigameAnswerHandler;
} catch { }
try {
  family100AnswerHandler = (await import("../plugins/game/family100.js"))
    .family100AnswerHandler;
} catch { }
try {
  pushkontakAnswerHandler = (
    await import("../plugins/pushkontak/pushkontak.js")
  ).pushkontakAnswerHandler;
} catch { }
try {
  anticustomReplyHandler = (await import("../plugins/group/anticustom.js"))
    .replyHandler;
} catch { }
try {
  dafontAnswerHandler = (await import("../plugins/tools/dafont.js"))
    .dafontAnswerHandler;
} catch { }
try {
  gantiAssetAnswerHandler = (await import("../plugins/owner/ganti-asset.js"))
    .gantiAssetAnswerHandler;
  srtAnswerHandler = (await import("../plugins/owner/srt.js"))
    .srtAnswerHandler;
} catch (e) { }
try {
  resepAnswerHandler = (await import("../plugins/search/resep.js"))
    .resepAnswerHandler;
} catch { }
try {
  stickerlyAnswerHandler = (await import("../plugins/sticker/stickerly.js"))
    .stickerlyAnswerHandler;
} catch { }
try {
  const caiPlugin = await import("../plugins/ai/characterai.js");
  caiAnswerHandler = caiPlugin.caiAnswerHandler;
  caiChatHandler = caiPlugin.caiChatHandler;
} catch { }
try {
  comparationAnswerHandler = (await import("../plugins/search/comparation.js"))
    .comparationAnswerHandler;
} catch { }
try {
  checkAfk = (await import("../plugins/group/afk.js")).checkAfk;
} catch { }
try {
  isMuted = (await import("../plugins/group/mute.js")).isMuted;
} catch { }
try {
  isMutedMember = (await import("../plugins/group/mutemember.js"))
    .isMutedMember;
} catch { }
try {
  isMutegc = (await import("../plugins/group/mutegc.js")).isMutegc;
} catch { }
try {
  detectBot = (await import("../plugins/group/antibot.js")).detectBot;
} catch { }
try {
  autoStickerHandler = (await import("../plugins/group/autosticker.js"))
    .autoStickerHandler;
} catch { }
try {
  autoMediaHandler = (await import("../plugins/group/automedia.js"))
    .autoMediaHandler;
} catch { }
try {
  checkAntidocument = (await import("../plugins/group/antidocument.js"))
    .checkAntidocument;
} catch { }
try {
  checkAntisticker = (await import("../plugins/group/antisticker.js"))
    .checkAntisticker;
} catch { }
try {
  checkAntimedia = (await import("../plugins/group/antimedia.js"))
    .checkAntimedia;
} catch { }
try {
  ytmp4Plugin = await import("../plugins/download/ytmp4.js");
} catch { }
try {
  confessPlugin = await import("../plugins/fun/confess.js");
} catch { }
try {
  sulapPlugin = await import("../plugins/fun/sulap.js");
} catch { }
try {
  handleAutoAI = (await import("./lib/ourin-auto-ai.js")).handleAutoAI;
} catch { }
try {
  handleAutoDownload = (await import("./lib/ourin-auto-download.js"))
    .handleAutoDownload;
} catch { }
try {
  checkStickerCommand = (await import("./lib/ourin-sticker-command.js"))
    .checkStickerCommand;
} catch { }
try {
  handleStickerReply = (await import("./lib/ourin-sticker-reply.js"))
    .handleStickerReply;
} catch { }
try {
  sendWelcomeMessage = (await import("../plugins/group/welcome.js"))
    .sendWelcomeMessage;
} catch { }
try {
  sendGoodbyeMessage = (await import("../plugins/group/goodbye.js"))
    .sendGoodbyeMessage;
} catch { }
try {
  autoJoinDetector = (await import("../plugins/owner/autojoingc.js"))
    .autoJoinDetector;
} catch { }

let checkSpam = null,
  handleSpamAction = null;
try {
  const m = await import("../plugins/group/antispam.js");
  checkSpam = m.checkSpam;
  handleSpamAction = m.handleSpamAction;
} catch { }

let checkSlowmode = null,
  incrementChatCount = null;
try {
  checkSlowmode = (await import("../plugins/group/slowmode.js")).checkSlowmode;
} catch { }
try {
  incrementChatCount = (await import("../plugins/group/topchat.js"))
    .incrementChatCount;
} catch { }

let isToxic = null,
  handleToxicMessage = null;
try {
  const m = await import("../plugins/group/antitoxic.js");
  isToxic = m.isToxic;
  handleToxicMessage = m.handleToxicMessage;
} catch { }

const spamDelayTracker = new Map();
setInterval(() => {
  const now = Date.now();
  for (const [k, v] of spamDelayTracker) {
    if (now - v > 15000) spamDelayTracker.delete(k);
  }
}, 30000);

let _smartTriggerThumb = undefined;
async function getSmartTriggerThumb() {
  if (_smartTriggerThumb !== undefined) return _smartTriggerThumb;
  try {
    const url = config.assets["ourin2"];
    if (url) {
      _smartTriggerThumb = fs.readFileSync(url)
    } else {
      _smartTriggerThumb = null;
    }
  } catch {
    _smartTriggerThumb = null;
  }
  return _smartTriggerThumb;
}

const globalRateLimiter = new RateLimiterMemory({
  points: 8,
  duration: 3,
  blockDuration: 2,
});

const cachedGamePlugins = new Map();

try {
  const gameDir = path.join(process.cwd(), "plugins", "game");
  const gameFiles = fs
    .readdirSync(gameDir)
    .filter((f) => f.endsWith(".js") && !f.startsWith("_"));
  for (const file of gameFiles) {
    try {
      const plugin = await import(`../plugins/game/${file}`);
      const name = file.replace(".js", "");
      if (plugin.answerHandler) cachedGamePlugins.set(name, plugin);
    } catch { }
  }
} catch { }

async function handleGameAnswer(m, sock) {
  try {
    if (sulapPlugin?.answerHandler) {
      const handled = await sulapPlugin.answerHandler(m, sock);
      if (handled) return true;
    }

    const utPlugin = cachedGamePlugins.get('ulartangga');
    if (utPlugin?.answerHandler) {
      if (await utPlugin.answerHandler(m, sock)) return true;
    }

    const tttPlugin = cachedGamePlugins.get('tictactoe');
    if (tttPlugin?.answerHandler) {
      if (await tttPlugin.answerHandler(m, sock)) return true;
    }

    const suitPlugin = cachedGamePlugins.get('suitpvp');
    if (suitPlugin?.answerHandler) {
      if (await suitPlugin.answerHandler(m, sock)) return true;
    }

    if (!hasActiveSession(m.chat)) return false;

    const session = getSession(m.chat);
    if (!session) return false;

    const targeted = cachedGamePlugins.get(session.gameType);
    if (targeted) {
      const handled = await targeted.answerHandler(m, sock);
      if (handled) return true;
    }
  } catch { }
  return false;
}

async function handleSmartTriggers(m, sock, db) {
  if (!m.body) return false;

  const text = m.body.trim().toLowerCase();

  const firstWord = text.split(" ")[0];
  if (
    /^[\.\/\!\#\-]?(autoreply|ar|smarttrigger|smarttriggers)$/.test(firstWord)
  ) {
    return false;
  }

  if (text === "done") {
    const sessions = db.setting("transactionSessions") || {};
    if (sessions[m.sender]) {
      try {
        if (handleBuyerDone) {
          const session = sessions[m.sender];
          await handleBuyerDone(m, sock, session);
          delete sessions[m.sender];
          db.setting("transactionSessions", sessions);
          await db.save();
          return true;
        }
      } catch (e) {
        console.error("[Handler] Done trigger error:", e.message);
      }
    }
  }

  const globalSmartTriggers =
    db.setting("smartTriggers") ?? config.features?.smartTriggers ?? false;

  try {
    const saluranId = config.saluran?.id || "120363400911374213@newsletter";
    const saluranName = config.saluran?.name || config.bot?.name || "Ourin-AI";
    const botName = config.bot?.name || "Ourin-AI";

    let isAutoreplyEnabled = globalSmartTriggers;

    const processCustomReply = async (replyItem) => {
      let replyText = (replyItem.reply || "")
        .replace(/{name}/g, m.pushName || "User")
        .replace(/{tag}/g, `@${m.sender.split("@")[0]}`)
        .replace(/{sender}/g, m.sender.split("@")[0])
        .replace(/{botname}/g, config.bot?.name || "Bot")
        .replace(/{time}/g, timeHelper.formatTime("HH:mm:ss"))
        .replace(/{date}/g, timeHelper.formatDate("DD MMMM YYYY"));

      const mentions = replyText.includes(`@${m.sender.split("@")[0]}`)
        ? [m.sender]
        : [];

      if (replyItem.image && fs.existsSync(replyItem.image)) {
        const imageBuffer = fs.readFileSync(replyItem.image);
        await sock.sendMedia(m.chat, imageBuffer, replyText, m, {
          mentions: mentions,
          type: "image",
        });
      } else {
        await m.reply(replyText, { mentions: mentions });
      }
      return true;
    };

    if (m.isGroup) {
      const groupData = db.getGroup(m.chat) || {};
      isAutoreplyEnabled = groupData.autoreply ?? globalSmartTriggers;

      if (isAutoreplyEnabled) {
        let customReplies = groupData.customReplies || [];
        if (!Array.isArray(customReplies)) {
          customReplies = [];
          db.setGroup(m.chat, { customReplies });
        }
        for (const replyItem of customReplies) {
          if (!replyItem?.trigger) continue;
          if (text === replyItem.trigger || text.includes(replyItem.trigger)) {
            return await processCustomReply(replyItem);
          }
        }

        const globalCustomReplies = db.setting("globalCustomReplies") || [];
        for (const replyItem of globalCustomReplies) {
          if (!replyItem?.trigger) continue;
          if (text === replyItem.trigger || text.includes(replyItem.trigger)) {
            return await processCustomReply(replyItem);
          }
        }
      }
    } else {
      const privateAutoreply = db.setting("autoreplyPrivate") ?? false;
      if (!privateAutoreply && !globalSmartTriggers) return false;
      isAutoreplyEnabled = privateAutoreply || globalSmartTriggers;

      if (isAutoreplyEnabled) {
        const globalCustomReplies = db.setting("globalCustomReplies") || [];
        for (const replyItem of globalCustomReplies) {
          if (!replyItem?.trigger) continue;
          if (text === replyItem.trigger || text.includes(replyItem.trigger)) {
            return await processCustomReply(replyItem);
          }
        }
      }
    }

    if (!isAutoreplyEnabled) return false;

    const botJid = sock.user?.id;
    const isMentioned = m.mentionedJid?.some(
      (jid) => jid === botJid || jid?.includes(sock.user?.id?.split(":")[0]),
    );

    const thumbBuffer = await getSmartTriggerThumb();

    const contextInfos = saluranCtx();

    if (isMentioned) {
      await m.reply(
        `Ada yang manggil ${botName}?
        
Ada apa manggil aku @${m.sender.split("@")[0]}?`,
        { mentions: [m.sender] },
      );
      return true;
    }

    if (text?.toLowerCase() === "p") {
      await m.reply(
        `Hai @${m.sender.split("@")[0]}, utamakan salam dulu yahh`,
        { mentions: [m.sender] },
      );
      return true;
    }

    if (text?.toLowerCase() === "bot") {
      await m.reply(`Hai @${m.sender.split("@")[0]}, ${botName} Aktif ✅`, {
        mentions: [m.sender],
      });
      return true;
    }

    if (text?.toLowerCase()?.includes("assalamualaikum")) {
      await m.reply(`Waaalaikumssalam @${m.sender.split("@")[0]}`, {
        mentions: [m.sender],
      });
      return true;
    }

    if (text?.toLowerCase()?.includes("hallo")) {
      await m.reply(`Halo juga kak @${m.sender.split("@")[0]}`, {
        mentions: [m.sender],
      });
      return true;
    }
  } catch (error) {
    console.error("[SmartTriggers] Error:", error.message);
  }

  return false;
}

/**
 * Cek apakah user sedang spam
 * @param {string} jid - JID user
 * @returns {boolean} True jika sedang spam
 */
async function isSpamming(jid) {
  if (!config.features?.antiSpam) return false;

  try {
    await globalRateLimiter.consume(jid);
    return false;
  } catch {
    return true;
  }
}

/**
 * Handler utama untuk memproses pesan
 * @param {Object} msg - Raw message dari Baileys
 * @param {Object} sock - Socket connection
 * @returns {Promise<void>}
 * @example
 * sock.ev.on('messages.upsert', async ({ messages }) => {
 *   await messageHandler(messages[0], sock);
 * });
 */
async function messageHandler(msg, sock, options = {}) {
  const isJadibot = options.isJadibot || false;
  try {
    let m;
    try {
      m = await serialize(sock, msg);
    } catch (serializeErr) {
      return;
    }

    if (!m) return;
    if (!m.message) return;
    if (!m.sender) m.sender = m.chat || "";

    if (global.giveawaySessions?.has(m.sender)) {
      try {
        const { handleSession: gaHandler } =
          await import("../plugins/group/giveaway.js");
        const gaHandled = await gaHandler(m, sock);
        if (gaHandled) return;
      } catch (e) { }
    }

    if (m.message?.stickerPackMessage && sock.saveStickerPack) {
      try {
        const packMsg = m.message.stickerPackMessage;
        const packId = packMsg.stickerPackId || m.id;
        const packName = packMsg.name || "Unknown Pack";
        sock.saveStickerPack(packId, { stickerPackMessage: packMsg }, packName);
      } catch (e) { }
    }

    const db = getDatabase();
    if (!db?.ready) {
      return;
    }

    if (!m.isBot && m.sender && m.isGroup) {
      let contacts = db.setting("contacts") || {};
      const currentName = m.pushName;
      if (currentName && currentName !== "User" && currentName !== "Unknown" && currentName !== "~ User") {
        if (!contacts[m.sender] || contacts[m.sender].name !== currentName) {
          contacts[m.sender] = {
            jid: m.sender,
            name: currentName
          };
          db.setting("contacts", contacts);
        }
      }

      const ownerNumbers = (global.owner || []).map(o => typeof o === "string" ? o.replace(/[^0-9]/g, "") : String(o));
      const senderNumber = m.sender.replace(/[^0-9]/g, "");
      const isOwnerUser = m.isOwner || ownerNumbers.includes(senderNumber);

      if (isOwnerUser) {
        const groupData = db.getGroup(m.chat);
        if (groupData && groupData.autoSambut && groupData.autoSambut.enabled) {
          const ownerId = m.sender;
          const lastChat = groupData.autoSambut.lastChats?.[ownerId] || 0;
          const delayMs = groupData.autoSambut.delayMs || (groupData.autoSambut.delay ? groupData.autoSambut.delay * 3600000 : 7200000);
          const now = Date.now();

          if (lastChat > 0 && (now - lastChat >= delayMs)) {
            if (groupData.autoSambut.pesan !== undefined && !groupData.autoSambut.pesanList) {
              groupData.autoSambut.pesanList = [groupData.autoSambut.pesan];
            }

            const pList = groupData.autoSambut.pesanList || ["Halo {user}! Selamat datang kembali 🙇‍♂️"];
            const randomMsg = pList[Math.floor(Math.random() * pList.length)];

            let sambutan = randomMsg
              .replace(/{name}/gi, m.pushName || "Owner")
              .replace(/{user}/gi, `@${ownerId.split('@')[0]}`);

            m.reply(sambutan, { mentions: [ownerId] }).catch(() => { });
          }

          if (!groupData.autoSambut.lastChats) groupData.autoSambut.lastChats = {};
          groupData.autoSambut.lastChats[ownerId] = now;
          db.setGroup(m.chat, { autoSambut: groupData.autoSambut });
        }
      }
    }

    const jadibotId = options.jadibotId || null;
    if (isJadibot && jadibotId) {
      const botJid = sock.user?.id?.split(":")[0] + "@s.whatsapp.net";
      const senderNum = m.sender?.replace(/[^0-9]/g, "") || "";
      const botNum = botJid.replace(/[^0-9]/g, "");
      m.isOwner = isJadibotOwner(jadibotId, m.sender) || senderNum === botNum;
      m.isPremium = isJadibotPremium(jadibotId, m.sender) || m.isOwner;
    }

    if (config.features?.logMessage) {
      let groupName = "PRIVATE";
      if (m.isGroup) {
        const groupData = db.getGroup(m.chat);
        groupName = groupData?.name || "Unknown Group";
        if (groupName === "Unknown Group" || groupName === "Unknown") {
          sock
            .groupMetadata(m.chat)
            .then((meta) => {
              if (meta?.subject) db.setGroup(m.chat, { name: meta.subject });
            })
            .catch(() => { });
        }
      }

      if (!isJadibot) {
        const deviceHint =
          m.key?.id?.length > 22
            ? "Android"
            : m.key?.id?.startsWith("3EB0")
              ? "iPhone"
              : m.key?.id?.startsWith("BAE5")
                ? "Web"
                : null;
        logMessage({
          chatType: m.isNewsletter
            ? "newsletter"
            : m.isGroup
              ? "group"
              : "private",
          groupName: m.isNewsletter ? "Channel" : groupName,
          pushName: m.pushName,
          sender: m.sender,
          message: m.body,
          messageType: m.type,
          device: deviceHint || "Unknown",
          isForwarded: m.message?.[m.type]?.contextInfo?.isForwarded || false,
          isNewsletter:
            m.isNewsletter ||
            !!m.message?.[m.type]?.contextInfo?.forwardedNewsletterMessageInfo,
          isOwner: m.isOwner,
          isPremium: m.isPremium,
          isPartner: m.isPartner,
          isAdmin: m.isAdmin,
          device: deviceHint,
        });
      }
    }

    if (checkAfk) {
      checkAfk(m, sock).catch(() => { });
    }

    if (m.isGroup && !m.isNewsletter) {
      cacheMessageForAntiRemove(m, sock, db);

      const antiJudolTriggered = await handleAntiJudol(m, sock, db);
      if (antiJudolTriggered) return;

      const antiPhisingTriggered = await handleAntiPhising(m, sock, db);
      if (antiPhisingTriggered) return;

      const antiCustomTriggered = await handleAntiCustom(m, sock, db);
      if (antiCustomTriggered) return;

      const antilinkTriggered = await handleAntilink(m, sock, db);
      if (antilinkTriggered) return;

      const antilinkGcTriggered = await handleAntilinkGc(m, sock, db);
      if (antilinkGcTriggered) return;

      const antilinkAllTriggered = await handleAntilinkAll(m, sock, db);
      if (antilinkAllTriggered) return;

      const antiHidetagTriggered = await handleAntiHidetag(m, sock, db);
      if (antiHidetagTriggered) return;

      const antiSwGcTriggered = await handleAntiSwGc(m, sock, db);
      if (antiSwGcTriggered) return;

      if (checkAntidocument) {
        const isAntidocument = await checkAntidocument(m, sock, db);
        if (isAntidocument) return;
      }

      if (detectBot && !m.isOwner && !m.isAdmin) {
        try {
          const botDetected = await detectBot(m, sock);
          if (botDetected) return;
        } catch (e) { }
      }

      if (isMuted && !m.isAdmin && !m.isOwner) {
        try {
          if (isMuted(m.chat, db)) {
            if (m.isBotAdmin) await sock.sendMessage(m.chat, { delete: m.key });
            return;
          }
        } catch (e) { }
      }

      if (isMutedMember && !m.isAdmin && !m.isOwner) {
        try {
          if (isMutedMember(m.chat, m.sender, db)) {
            if (m.isBotAdmin) await sock.sendMessage(m.chat, { delete: m.key });
            return;
          }
        } catch (e) { }
      }

      if (isMutegc && !m.isAdmin && !m.isOwner && !m.isPartner) {
        try {
          if (isMutegc(m.chat, db)) return;
        } catch (e) { }
      }

      if (checkSpam && handleSpamAction && !m.isAdmin) {
        try {
          const isSpam = await checkSpam(m, sock, db);
          if (isSpam) {
            const delayKey = `${m.chat}_${m.sender}`;
            spamDelayTracker.set(delayKey, Date.now());
            await handleSpamAction(m, sock, db);
          }
        } catch (e) { }
      }

      if (checkSlowmode && !m.isAdmin && !m.isOwner) {
        try {
          const slowResult = checkSlowmode(m, sock, db);
          if (slowResult) {
            if (slowResult.mode === "onlycommand") {
              if (m.isCommand) return;
            } else {
              await sock.sendMessage(m.chat, { delete: m.key });
              return;
            }
          }
        } catch (e) { }
      }

      if (isToxic && handleToxicMessage) {
        try {
          const groupData = db.getGroup(m.chat) || {};
          if (groupData.antitoxic && !m.isAdmin && !m.isOwner) {
            const toxicWords = groupData.toxicWords || [];
            const result = isToxic(m.body, toxicWords);
            if (result.toxic) {
              await handleToxicMessage(m, sock, db, result.word);
              return;
            }
          }
        } catch (e) { }
      }
    }

    if (m.isGroup && incrementChatCount) {
      try {
        incrementChatCount(m.chat, m.sender, db, m.pushName);
      } catch (e) { }
    }

    const modeCheck = checkMode(m, getActiveJadibots);
    if (!modeCheck.allowed) {
      if (modeCheck.isAfk && m.isCommand) {
        await m.reply(modeCheck.afkMessage);
      } else if (modeCheck.hasJadibots && m.isCommand && !isJadibot) {
        await sock.sendMessage(
          m.chat,
          {
            text: modeCheck.jadibotMessage,
            contextInfo: {
              ...saluranCtx(),
              mentionedJid: modeCheck.jadibotMentions,
            },
          },
          { quoted: m },
        );
      } else if (modeCheck.isOnlyThisGroup && m.isCommand) {
        await m.reply(modeCheck.onlyThisGroupMessage);
      }
      return;
    }

    if (m.isBanned) {
      if (m.isCommand) {
        await m
          .reply(
            config.messages?.banned ||
            "🚫 *Kamu dibanned dari menggunakan bot ini.*",
          )
          .catch(() => { });
      }
      logger.warn("Banned user", m.sender);
      return;
    }

    if (m.isGroup && m.isCommand && !m.isOwner) {
      const groupData = db.getGroup(m.chat) || {};
      if (groupData.isBanned) {
        // kalau mau nambih text juga boleh bang, pake m.reply atau sendMessage
        return;
      }
    }

    const botId = sock.user?.id?.split(":")[0] || "unknown";
    const msgKey = `${botId}_${m.chat}_${m.sender}_${m.id}`;
    if (debounceMessage(msgKey)) {
      return;
    }

    if (db.setting("autoRead") ?? config.features?.autoRead) {
      sock.readMessages([m.key]).catch(() => { });
    }
    if (!m.pushName || m.pushName === "Unknown" || m.pushName.trim() === "") {
      if (!m.isCommand && !m.isBot && !m.fromMe && !m.isNewsletter) {
        return;
      }
      m.pushName = m.isNewsletter
        ? "Channel"
        : m.sender?.split("@")[0] || "User";
    }

    const registrationNameRequired =
      db.setting("registrationRequired") ??
      config.registration?.enabled ??
      false;
    const registrationNameUser = db.getUser(m.sender);
    if (
      registrationNameRequired &&
      registrationNameUser?.isRegistered &&
      registrationNameUser?.regName
    ) {
      m.originalPushName = m.pushName;
      m.pushName = registrationNameUser.regName;
    }

    if (m.isCommand) {
      db.setUser(m.sender, {
        name: m.originalPushName || m.pushName,
        lastSeen: new Date().toISOString(),
      });
    }

    const cmdVnEnabled = db.setting("cmdVn") || false;
    if (
      cmdVnEnabled &&
      m.type === "audioMessage" &&
      !m.isCommand &&
      config.APIkey?.groq
    ) {
      let inputFile = null;
      let wavFile = null;
      const cleanupCmdVnFiles = () => {
        [inputFile, wavFile].forEach((filePath) => {
          if (!filePath) return;
          try {
            if (fs.existsSync(filePath)) fs.unlinkSync(filePath);
          } catch { }
        });
      };
      try {
        const audioMsg = m.message?.audioMessage;
        const maxSize = 500 * 1024;
        if (
          audioMsg &&
          (!audioMsg.fileLength || audioMsg.fileLength <= maxSize)
        ) {
          const buffer = await m.download();
          if (buffer && buffer.length > 1000) {
            const tmpDir = path.join(process.cwd(), "tmp");
            if (!fs.existsSync(tmpDir))
              fs.mkdirSync(tmpDir, { recursive: true });

            inputFile = path.join(tmpDir, `vncmd_${Date.now()}.ogg`);
            wavFile = path.join(tmpDir, `vncmd_${Date.now()}.wav`);

            fs.writeFileSync(inputFile, buffer);

            await new Promise((resolve, reject) => {
              exec(
                `ffmpeg -y -i "${inputFile}" -ar 16000 -ac 1 -f wav "${wavFile}"`,
                { timeout: 15000 },
                (err) => (err ? reject(err) : resolve()),
              );
            });

            const wavBuffer = fs.readFileSync(wavFile);
            const form = new FormData();
            form.append("file", wavBuffer, {
              filename: "audio.wav",
              contentType: "audio/wav",
            });
            form.append("model", "whisper-large-v3");
            form.append("language", "id");
            form.append("response_format", "json");

            const { data } = await axios.post(
              "https://api.groq.com/openai/v1/audio/transcriptions",
              form,
              {
                headers: {
                  ...form.getHeaders(),
                  Authorization: `Bearer ${config.APIkey.groq}`,
                },
                timeout: 30000,
                maxContentLength: Infinity,
              },
            );

            cleanupCmdVnFiles();
            inputFile = null;
            wavFile = null;

            const transcript = (data.text || "")
              .trim()
              .toLowerCase()
              .replace(/[.,!?;:'"]/g, "")
              .trim();

            if (transcript) {
              const words = transcript.split(/\s+/);
              const rawWord = words[0];
              const prefix = config.command?.prefix || ".";

              const allPlugins = getAllPlugins();
              const allNames = [];
              for (const p of allPlugins) {
                if (p.config?.name && typeof p.config.name === "string")
                  allNames.push(p.config.name.toLowerCase());
                if (Array.isArray(p.config?.alias)) {
                  for (const a of p.config.alias) {
                    if (a && typeof a === "string")
                      allNames.push(a.toLowerCase());
                  }
                }
              }

              let bestMatch = null;
              let bestScore = Infinity;

              for (const cmd of allNames) {
                if (cmd === rawWord) {
                  bestMatch = cmd;
                  bestScore = 0;
                  break;
                }
                if (rawWord.startsWith(cmd) && cmd.length >= 3) {
                  const score = rawWord.length - cmd.length;
                  if (score < bestScore) {
                    bestScore = score;
                    bestMatch = cmd;
                  }
                }
                const dist = levenshtein(rawWord, cmd);
                if (dist <= 3 && dist < bestScore) {
                  bestScore = dist;
                  bestMatch = cmd;
                }
              }

              if (bestMatch) {
                const commandArgs = words.slice(1).join(" ");
                m.body = `${prefix}${bestMatch}${commandArgs ? " " + commandArgs : ""}`;
                const { parseCommand } =
                  await import("./lib/ourin-serialize.js");
                const parsed = parseCommand(m.body, prefix);
                m.isCommand = parsed.isCommand;
                m.command = parsed.command;
                m.args = parsed.args;
                m.prefix = parsed.prefix;
                m.isVnCommand = true;
              }
            }
          }
        }
      } catch (e) {
        cleanupCmdVnFiles();
        console.error("[CMD VN] Error:", e.message);
      }
    }

    if (m.body) {
      try {
        const userObj = db.getUser(m.sender) || db.setUser(m.sender);

        if (levelHelper && levelHelper.addExpWithLevelCheck) {
          await levelHelper.addExpWithLevelCheck(sock, m, db, userObj, 15);
        }
      } catch (e) {
        console.error("[Level System] Error:", e.message);
      }
    }

    if (handleAutoAI && m.isGroup && !m.isCommand) {
      try {
        const aiHandled = await handleAutoAI(m, sock);
        if (aiHandled) return;
      } catch (e) { }
    }

    if (caiChatHandler && !m.isCommand && m.body) {
      try {
        const caiHandled = await caiChatHandler(m, sock);
        if (caiHandled) return;
      } catch (e) { }
    }

    if (handleAutoDownload && m.body) {
      try {
        handleAutoDownload(m, sock, m.body);
      } catch (e) { }
    }

    if (autoJoinDetector && m.body) {
      try {
        const joined = await autoJoinDetector(m, sock);
        if (joined) return;
      } catch (e) { }
    }

    if (m.body?.startsWith(">>") && m.isOwner) {
      const code = m.body.slice(2).trim();
      if (!code) return;

      try {
        const AsyncFunction = Object.getPrototypeOf(
          async function () { },
        ).constructor;

        const execCode = new AsyncFunction(
          "m",
          "sock",
          "db",
          "config",
          "getDatabase",
          "console",
          `
          const { default: axios } = await import('axios')
          const { default: fs } = await import('fs')
          const { default: path } = await import('path')
          const { default: os } = await import('os')
          const { promisify } = await import('util')
          const { generateWAMessage, getBuffer, generateWAMessageFromContent, proto, generateMessageID } = await import('ourin')
          const { exec: childExec } = await import('child_process')
          const { VERSION, Button, ButtonV2, Carousel, AIRich, } = await import('./lib/ourin-builder.js')
          const exec = promisify(childExec)
          
          ${code}
          `,
        );

        const result = await execCode(
          m,
          sock,
          db,
          config,
          getDatabase,
          console,
        );

        if (result !== undefined && result !== null) {
          const output =
            typeof result === "object"
              ? JSON.stringify(result, null, 2)
              : String(result);

          if (output.length > 0) {
            await m.reply(
              `✅ *ᴇxᴇᴄ ʀᴇsᴜʟᴛ*\n\n\`\`\`\n${output.substring(0, 4000)}\n\`\`\``,
            );
          }
        }
      } catch (execError) {
        await m.reply(
          `❌ *ᴇxᴇᴄ ᴇʀʀᴏʀ*\n\n\`\`\`\n${execError.message}\n\nStack:\n${execError.stack?.substring(0, 1000) || "N/A"}\n\`\`\``,
        );
      }
      return;
    }

    if (m.body?.startsWith("!!") && m.isOwner) {
      const expr = m.body.slice(2).trim();
      if (!expr) return;

      try {
        const AsyncFunction = Object.getPrototypeOf(
          async function () { },
        ).constructor;

        const inspectCode = new AsyncFunction(
          "m",
          "sock",
          "db",
          "config",
          "getDatabase",
          "console",
          `
          const util = await import('util')
          const { default: axios } = await import('axios')
          const { default: fs } = await import('fs')
          const { default: path } = await import('path')
          const { default: os } = await import('os')
          const { generateWAMessage, getBuffer, generateWAMessageFromContent, proto, generateMessageID } = await import('ourin')

          const result = await ${expr}
          if (result === undefined) return 'undefined'
          if (result === null) return 'null'
          if (typeof result === 'string') return result
          try {
            return util.inspect.default(result, { depth: 4, maxArrayLength: 30, maxStringLength: 300, breakLength: 60, compact: false })
          } catch {
            try { return JSON.stringify(result, null, 2) } catch { return String(result) }
          }
          `,
        );

        const output = await inspectCode(
          m,
          sock,
          db,
          config,
          getDatabase,
          console,
        );

        if (output && String(output).length > 0) {
          const str = String(output);
          await m.reply(
            `${str}`,
          );
        }
      } catch (inspectError) {
        await m.reply(
          `${inspectError.message}\n\`\`\``,
        );
      }
      return;
    }

    const hasSuitGame =
      global.suitGames &&
      Object.values(global.suitGames).some(
        (r) =>
          (r.chat === m.chat || !m.isGroup) && [r.p, r.p2].includes(m.sender),
      );

    const hasTTTGame =
      global.tictactoeGames &&
      Object.values(global.tictactoeGames).some(
        (r) =>
          r.state === "PLAYING" &&
          r.chat === m.chat &&
          [r.game.playerX, r.game.playerO].filter(Boolean).includes(m.sender),
      );

    const hasUTGame = global.ulartanggaGames?.[m.chat]?.status === "PLAYING";

    let gameEvaluated = false;
    if (
      (hasActiveSession(m.chat) && m.quoted) ||
      hasSuitGame ||
      hasTTTGame ||
      hasUTGame
    ) {
      gameEvaluated = true;
      const gameHandled = await handleGameAnswer(m, sock);
      if (gameHandled) return;
    }

    try {
      if (registrationAnswerHandler) {
        const handled = await registrationAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Registration answer error:", e.message);
    }

    try {
      if (dungeonAnswerHandler) {
        const handled = await dungeonAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Dungeon answer error:", e.message);
    }

    try {
      if (resepAnswerHandler) {
        const handled = await resepAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Resep answer error:", e.message);
    }

    try {
      if (stickerlyAnswerHandler) {
        const handled = await stickerlyAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Stickerly answer error:", e.message);
    }

    try {
      if (caiAnswerHandler) {
        const handled = await caiAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] CAI answer error:", e.message);
    }

    try {
      if (comparationAnswerHandler) {
        const handled = await comparationAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Comparation answer error:", e.message);
    }

    try {
      if (kyubigameAnswerHandler) {
        const handled = await kyubigameAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Kyubigame answer error:", e.message);
    }

    try {
      if (family100AnswerHandler) {
        const handled = await family100AnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Family100 answer error:", e.message);
    }

    try {
      if (pushkontakAnswerHandler) {
        const handled = await pushkontakAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Pushkontak answer error:", e.message);
    }

    try {
      if (dafontAnswerHandler) {
        const handled = await dafontAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] Dafont answer error:", e.message);
    }

    try {
      if (gantiAssetAnswerHandler) {
        const handled = await gantiAssetAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] GantiAsset answer error:", e.message);
    }

    try {
      if (srtAnswerHandler) {
        const handled = await srtAnswerHandler(m, sock);
        if (handled) return;
      }
    } catch (e) {
      console.error("[Handler] SRT answer error:", e.message);
    }

    if (!m.isCommand) {
      if (hasActiveSession(m.chat) && !gameEvaluated) {
        gameEvaluated = true;
        const gameHandled = await handleGameAnswer(m, sock);
        if (gameHandled) return;
      }

      const smartHandled = await handleSmartTriggers(m, sock, db);
      if (smartHandled) return;

      if (m.quoted?.id || m.quoted?.key?.id) {
        try {
          if (anticustomReplyHandler) {
            const handled = await anticustomReplyHandler(m, { sock });
            if (handled) return;
          }
          if (
            global.confessData?.has(m.quoted.id) &&
            confessPlugin?.replyHandler
          ) {
            const handled = await confessPlugin.replyHandler(m, { sock });
            if (handled) return;
          }
          if (
            global.sulapSessions?.has(m.quoted.id) &&
            sulapPlugin?.replyHandler
          ) {
            const handled = await sulapPlugin.replyHandler(m, sock);
            if (handled) return;
          }
        } catch { }
      }

      if (autoStickerHandler && m.isGroup) {
        autoStickerHandler(m, sock).catch(() => { });
      }

      if (autoMediaHandler && m.isGroup) {
        autoMediaHandler(m, sock).catch(() => { });
      }

      if (checkAntisticker && m.isGroup) {
        const stickerHandled = await checkAntisticker(m, sock, db);
        if (stickerHandled) return;
      }

      if (checkAntimedia && m.isGroup) {
        const mediaHandled = await checkAntimedia(m, sock, db);
        if (mediaHandled) return;
      }

      if (handleStickerReply && m.isGroup) {
        const stickerReplyHandled = await handleStickerReply(m, sock);
        if (stickerReplyHandled) return;
      }

      if (checkStickerCommand) {
        try {
          const stickerCmd = checkStickerCommand(m);
          if (stickerCmd) {
            const prefix = m.prefix || config.command?.prefix || ".";
            m.body = `${prefix}${stickerCmd}`;
            const { parseCommand } = await import("./lib/ourin-serialize.js");
            const parsed = parseCommand(m.body, prefix);
            m.isCommand = parsed.isCommand;
            m.command = parsed.command;
            m.args = parsed.args;
            m.prefix = parsed.prefix;
          }
        } catch (e) {
          console.error("[Handler] Sticker command error:", e.message);
        }
      }

      if (!m.isCommand) return;
    }

    const delayKey = `${m.chat}_${m.sender}`;
    if (!m.isOwner && !m.isPremium) {
      const lastSpamDetect = spamDelayTracker.get(delayKey);
      if (lastSpamDetect) {
        const elapsed = Date.now() - lastSpamDetect;
        if (elapsed < 10000) {
          await new Promise((r) => setTimeout(r, 500));
        } else {
          spamDelayTracker.delete(delayKey);
        }
      }
    }

    const spamKey = `${botId}_${m.sender}`;
    if (!m.isOwner && !m.isPremium && (await isSpamming(spamKey))) {
      return;
    }

    const storeData = db.setting("storeList") || {};
    const storeCommand = storeData[m.command.toLowerCase()];

    if (m.isGroup) {
      const groupData = db.getGroup(m.chat) || {};
      const botMode = groupData.botMode || "all";

      if (botMode === "store" && storeCommand) {
        storeData[m.command.toLowerCase()].views =
          (storeCommand.views || 0) + 1;
        db.setting("storeList", storeData);

        const caption =
          `📦 *${m.command.toUpperCase()}*\n\n` +
          `${storeCommand.content}\n\n` +
          `───────────────\n` +
          `> 👁️ Views: ${storeData[m.command.toLowerCase()].views}\n` +
          `> 💳 Ketik \`${m.prefix}payment\` untuk bayar`;

        if (storeCommand.hasImage && storeCommand.imagePath) {
          try {
            const imageBuffer = await getCachedThumb(storeCommand.imagePath);
            if (imageBuffer) {
              await sock.sendMessage(
                m.chat,
                {
                  image: imageBuffer,
                  caption: caption,
                },
                { quoted: m },
              );
              return;
            }
          } catch (e) {
            console.error("Gagal load image store:", e.message);
          }
        }

        await m.reply(caption);
        return;
      }
    }

    try {
      const caseResult = await handleCaseCommand(m, sock);
      if (caseResult && caseResult.handled) {
        if (config.dev?.debugLog) {
          logger.success("Case", `Handled: ${m.command}`);
        }
        return;
      }
    } catch (caseError) {
      logger.error("Case System", caseError.message);
      if (config.dev?.debugLog) {
        console.error("[CaseSystem] Stack:", caseError.stack);
      }
    }

    let plugin = getPlugin(m.command);

    if (!plugin) {
      if (storeCommand) {
        storeData[m.command.toLowerCase()].views =
          (storeCommand.views || 0) + 1;
        db.setting("storeList", storeData);

        const caption =
          `📦 *${m.command.toUpperCase()}*\n\n` +
          `${storeCommand.content}\n\n` +
          `───────────────\n` +
          `> 👁️ Views: ${storeData[m.command.toLowerCase()].views}\n` +
          `> 💳 Ketik \`${m.prefix}payment\` untuk bayar`;

        if (storeCommand.hasImage && storeCommand.imagePath) {
          try {
            const imageBuffer = await getCachedThumb(storeCommand.imagePath);
            if (imageBuffer) {
              await sock.sendMessage(
                m.chat,
                {
                  image: imageBuffer,
                  caption: caption,
                },
                { quoted: m },
              );
              return;
            }
          } catch (e) {
            console.error("Gagal load image store:", e.message);
          }
        }

        await m.reply(caption);
        return;
      }

      const storeCommands = Object.keys(storeData);
      const allCommands = [...getAllCommandNames(), ...storeCommands];

      const similarityEnabled = db.setting("similarity") !== false;

      if (similarityEnabled) {
        const suggestions = findSimilarCommands(m.command, allCommands, {
          maxResults: 1,
          minSimilarity: 0.8,
          maxDistance: 2,
        });

        if (suggestions.length > 0) {
          const suggestedCommand = m.prefix + suggestions[0].command;
          const simpleMessage = `I'm sorry, but I don't have that command :(\n\nDid you mean this command?\n⇒ *${suggestedCommand}*`;

          try {
            await sock.sendPreview(
              m.chat,
              {
                caption: config.info.website + "\n" + simpleMessage,
                url: config.info.website,
                title: "Command not found",
                description: `Suggestions Command | ${config.bot.name}`,
                jpegThumbnail: await sharp(fs.readFileSync(config.assets["ourin2"])).resize(300, 300).toBuffer(),
                previewType: 1,
              },
              { quoted: m }
            );
          } catch (err) {
            console.error("[Similarity] Gagal mengirim pesan similarity preview:", err.message);
          }
        }
      }

      return;
    }

    if (!plugin.config.isEnabled) {
      return;
    }

    if (m.isGroup) {
      const groupData = db.getGroup(m.chat) || {};
      let botMode = groupData.botMode || "all";
      const pluginCategory = plugin.config.category?.toLowerCase();
      const baseAllowed = ["main", "group", "sticker", "owner"];

      if (isJadibot) {
        botMode = "all";

        const jadibotBlockedCategories = [
          "owner",
          "sewa",
          "panel",
          "store",
          "pushkontak",
        ];
        const jadibotBlockedCommands = [
          "sewa",
          "sewabot",
          "sewalist",
          "listsewa",
          "addsewa",
          "delsewa",
          "extendsewa",
          "checksewa",
          "sewainfo",
          "sewagroup",
          "stopsewa",
          "jadibot",
          "listjadibot",
          "addowner",
          "delowner",
          "ownerlist",
          "listowner",
          "self",
          "public",
          "botmode",
          "restart",
          "shutdown",
        ];

        if (
          jadibotBlockedCategories.includes(pluginCategory) ||
          jadibotBlockedCommands.includes(m.command.toLowerCase())
        ) {
          return m.reply(
            `⚠️ *ᴀᴋsᴇs ᴛᴇʀʙᴀᴛᴀs*\n\n` +
            `Fitur ini hanya tersedia di bot utama.\n` +
            `Jadibot tidak dapat mengakses fitur ini.\n\n` +
            `> Hubungi owner bot utama untuk informasi lebih lanjut.`,
          );
        }
      }

      const modeConfig = {
        all: { allowed: null, excluded: null, name: "All Features" },
        md: {
          allowed: null,
          excluded: ["pushkontak", "store", "panel", "otp"],
          name: "Multi Device",
        },
        cpanel: { allowed: [...baseAllowed, "tools", "panel"], name: "CPanel" },
        pushkontak: {
          allowed: [...baseAllowed, "pushkontak"],
          name: "Push Kontak",
        },
        store: { allowed: [...baseAllowed, "store"], name: "Store" },
        otp: { allowed: [...baseAllowed, "otp"], name: "OTP" },
      };

      const categoryModeMap = {
        download: "md",
        search: "md",
        ai: "md",
        fun: "md",
        game: "md",
        media: "md",
        utility: "md",
        tools: "md",
        ephoto: "md",
        religi: "md",
        info: "md",
        panel: "cpanel",
        pushkontak: "pushkontak",
        store: "store",
        otp: "otp",
        jpm: "md",
      };

      const currentConfig = modeConfig[botMode] || modeConfig.all;

      if (
        m.command !== "botmode" &&
        m.command !== "menu" &&
        m.command !== "menucat"
      ) {
        let isBlocked = false;

        if (
          currentConfig.allowed &&
          !currentConfig.allowed.includes(pluginCategory)
        ) {
          isBlocked = true;
        }
        if (
          currentConfig.excluded &&
          currentConfig.excluded.includes(pluginCategory)
        ) {
          isBlocked = true;
        }

        if (isBlocked) {
          const suggestedMode = categoryModeMap[pluginCategory] || "all";
          const suggestedModeName =
            modeConfig[suggestedMode]?.name || "Multi Device";

          await m.reply(
            `🔒 *ᴄᴏᴍᴍᴀɴᴅ ᴛɪᴅᴀᴋ ᴛᴇʀsᴇᴅɪᴀ*\n\n` +
            `> Bot sedang dalam mode *${currentConfig.name}*\n` +
            `> Command \`${m.prefix}${m.command}\` tersedia di mode *${suggestedModeName}*\n\n` +
            `💡 Hubungi admin grup untuk mengganti mode:\n` +
            `\`${m.prefix}botmode ${suggestedMode}\``,
          );
          return;
        }
      }

      if (!m.isAdmin && !m.isOwner) {
        const pluginCat = plugin.config.category?.toLowerCase();
        const grpData = db.getGroup(m.chat) || {};

        if (pluginCat === "game" && grpData.game === false) {
          await m.reply(
            `🎮 *FITUR GAME NONAKTIF*\n\nFitur game sedang dinonaktifkan di grup ini oleh admin.\nMinta admin untuk mengaktifkannya kembali dengan perintah *${m.prefix}game on*`
          );
          return;
        }

        if (pluginCat === "rpg" && grpData.rpg === false) {
          await m.reply(
            `⚔️ *FITUR RPG NONAKTIF*\n\nFitur RPG sedang dinonaktifkan di grup ini oleh admin.\nMinta admin untuk mengaktifkannya kembali dengan perintah *${m.prefix}rpg on*`
          );
          return;
        }
      }
    }

    const permission = checkPermission(m, plugin.config);
    if (!permission.allowed) {
      await m.reply(permission.reason);
      return;
    }

    const registrationRequired =
      db.setting("registrationRequired") ??
      config.registration?.enabled ??
      false;
    if (registrationRequired && !plugin.config.skipRegistration) {
      const user = db.getUser(m.sender);
      if (!m.isOwner && !m.isPartner && !m.isPremium && !user?.isRegistered) {
        await m.reply(
          `📝 *ᴡᴀᴊɪʙ ᴅᴀꜰᴛᴀʀ*\n\n` +
          `Kamu harus daftar terlebih dahulu!\n\n` +
          `> Ketik: \`${m.prefix}daftar\`\n\n` +
          `*Lalu reply pertanyaan bot sampai selesai*`,
        );
        return;
      }
    }

    const user = db.getUser(m.sender);

    if (!m.isOwner && !m.isPartner && plugin.config.cooldown > 0) {
      const cooldownRemaining = db.checkCooldown(
        m.sender,
        m.command,
        plugin.config.cooldown,
      );
      if (cooldownRemaining) {
        m.react("⏱️").catch(() => { });
        return;
      }
    }

    const energiEnabled =
      db.setting("energi") !== undefined
        ? db.setting("energi")
        : config.energi?.enabled !== false;

    const capEnergiOverrides = db.setting("capenergi") || {};
    const pluginEnergiCost = capEnergiOverrides[plugin.config.name] !== undefined ? capEnergiOverrides[plugin.config.name] : (plugin.config.energi || 0);

    if (energiEnabled && pluginEnergiCost > 0) {
      const ownerEnergi = config.energi?.owner ?? -1;
      const premiumEnergi = config.energi?.premium ?? -1;
      const defaultEnergi = config.energi?.default ?? 0;

      let currentEnergi;
      if (
        (m.isOwner || m.isPartner) &&
        (ownerEnergi === -1 || user?.energi === -1)
      ) {
      } else if (m.isPremium && (premiumEnergi === -1 || user?.energi === -1)) {
      } else {
        currentEnergi =
          user?.energi ??
          (m.isOwner || m.isPartner
            ? ownerEnergi
            : m.isPremium
              ? premiumEnergi
              : defaultEnergi);
        if (currentEnergi < pluginEnergiCost) {
          await m.reply(config.messages?.energiExceeded || "⚡ Energi habis!");
          return;
        }
        db.updateEnergi(m.sender, -pluginEnergiCost);

        if (db.setting("notiflimit")) {
          let limitMsg = config.messages?.limitDeducted || "🔋 Limit kau berkurang sebanyak {amount}. Sisa limit: {sisa}";
          limitMsg = limitMsg.replace("{amount}", pluginEnergiCost.toString()).replace("{sisa}", (currentEnergi - pluginEnergiCost).toString());
          await m.reply(limitMsg);
        }
      }
    }

    if (db.setting("autoTyping") ?? config.features?.autoTyping) {
      sock.sendPresenceUpdate("composing", m.chat).catch(() => { });
    }

    const context = {
      sock,
      m,
      args: m.args || [],
      text: m.text || "",
      command: m.command || "",
      prefix: m.prefix || config.command?.prefix || ".",
      config,
      db,
      uptime: getUptime(),
      plugins: {
        count: getPluginCount(),
      },
      jadibotId: jadibotId,
      isJadibot: isJadibot,
    };

    await plugin.handler(m, context);

    if (!m.isOwner && !m.isPartner && plugin.config.cooldown > 0) {
      db.setCooldown(m.sender, m.command, plugin.config.cooldown);
    }

    db.incrementStat("commandsExecuted");
    db.incrementStat(`command_${m.command}`);

    if (user) {
      const todayStr = new Date().toLocaleDateString('id-ID', { timeZone: 'Asia/Jakarta' });
      if (user.lastActiveDate !== todayStr) {
        const newActiveDays = (user.activeDays || 0) + 1;
        db.setUser(m.sender, { lastActiveDate: todayStr, activeDays: newActiveDays });
      }
    }

    if (db.setting("autoTyping") ?? config.features?.autoTyping) {
      sock.sendPresenceUpdate("paused", m.chat).catch(() => { });
    }
  } catch (error) {
    logger.error("handler", `${error.message}`);
    console.error("[Handler Stack]", error.stack);

    try {
      const db = getDatabase();
      if (db) {
        db.incrementStat("commandErrors");
        const errorLog = db.setting("errorLog") || [];
        errorLog.unshift({
          cmd: "unknown",
          err: error.message?.substring(0, 200),
          at: Date.now(),
        });
        if (errorLog.length > 50) errorLog.splice(50);
        db.setting("errorLog", errorLog);
      }
    } catch { }

    try {
      const m = await serialize(sock, msg);
      if (m) {
        await m.reply(`Sepertinya ada kendala, coba hubungi owner`);
      }
    } catch {
      logger.error("Failed to send error message");
    }
  }
}

/**
 * Handler untuk update group participants
 * @param {Object} update - Update data
 * @param {Object} sock - Socket connection
 * @returns {Promise<void>}
 */
async function groupHandler(update, sock) {
  try {
    if (global.sewaLeaving) return;

    const { id: groupJid, participants, action } = update;

    if (
      !participants ||
      !Array.isArray(participants) ||
      participants.length === 0
    ) {
      return;
    }

    const db = getDatabase();

    let groupData = db.getGroup(groupJid);
    if (!groupData) {
      db.setGroup(groupJid, {
        welcome: config.welcome?.defaultEnabled ?? true,
        goodbye: config.goodbye?.defaultEnabled ?? true,
        leave: config.goodbye?.defaultEnabled ?? true,
      });
      groupData = db.getGroup(groupJid);
    }

    let groupMeta;
    try {
      const cached = global.groupMetadataCache?.get(groupJid);
      if (cached && Date.now() - (cached._ts || 0) < 30000) {
        groupMeta = cached;
      } else {
        groupMeta = await sock.groupMetadata(groupJid);
        if (global.groupMetadataCache) {
          groupMeta._ts = Date.now();
          global.groupMetadataCache.set(groupJid, groupMeta);
        }
      }

      if (groupMeta?.participants) {
        cacheParticipantLids(groupMeta.participants);
      }
    } catch (e) {
      if (
        e.message?.includes("forbidden") ||
        e.message?.includes("401") ||
        e.message?.includes("403")
      ) {
        return;
      }
      if (
        e.message?.includes("rate-overlimit") ||
        e?.output?.statusCode === 429
      ) {
        logger.warn("GroupHandler", "rate-limited, skipping event");
        return;
      }
      throw e;
    }

    for (let participant of participants) {
      let participantJid;

      if (typeof participant === "object" && participant !== null) {
        participantJid =
          participant.jid || participant.id || participant.lid || "";
      } else {
        participantJid = participant;
      }

      if (!participantJid || typeof participantJid !== "string") continue;

      if (isLid(participantJid) || isLidConverted(participantJid)) {
        const found = groupMeta.participants?.find(
          (p) =>
            p.id === participantJid ||
            p.lid === participantJid ||
            p.lid === participantJid.replace("@s.whatsapp.net", "@lid"),
        );
        if (found) {
          participantJid =
            found.jid &&
              !found.jid.endsWith("@lid") &&
              !isLidConverted(found.jid)
              ? found.jid
              : found.id &&
                !found.id.endsWith("@lid") &&
                !isLidConverted(found.id)
                ? found.id
                : lidToJid(participantJid);
        } else {
          participantJid = lidToJid(participantJid);
        }
      }

      participant = participantJid;

      if (action === "add" && sendWelcomeMessage) {
        await sendWelcomeMessage(sock, groupJid, participant, groupMeta);
      }

      if (action === "remove" && sendGoodbyeMessage) {
        await sendGoodbyeMessage(sock, groupJid, participant, groupMeta);
      }

      const saluranId = config.saluran?.id || "120363400911374213@newsletter";
      const saluranName =
        config.saluran?.name || config.bot?.name || "Ourin-AI";

      let groupPpUrl = null;
      try {
        groupPpUrl = await sock.profilePictureUrl(groupJid, "image");
      } catch { }

      const rankActions = {
        promote: {
          notifKey: "notifPromote",
          imgKey: "_promoteImg",
          imgPath: config.assets["ourin-promote"],
          emoji: "🎉",
          label: "PROMOTE",
          text: (p, a) =>
            `🌿 @${p} sekarang menjadi admin baru 💕\nPromoted by: @${a}`,
        },
        demote: {
          notifKey: "notifDemote",
          imgKey: "_demoteImg",
          imgPath: config.assets["ourin-demote"],
          emoji: "📉",
          label: "DEMOTE",
          text: (p, a) =>
            `🌿 @${p} sudah tidak menjadi admin lagi.\nDemoted by: @${a}`,
        },
      };

      const rankCfg = rankActions[action];
      if (rankCfg && groupData[rankCfg.notifKey] === true) {
        const author = update.author || null;
        if (!groupHandler[rankCfg.imgKey]) {
          try {
            groupHandler[rankCfg.imgKey] = fs.readFileSync(rankCfg.imgPath);
          } catch {
            groupHandler[rankCfg.imgKey] = null;
          }
        }
        if (groupHandler[rankCfg.imgKey]) {
          const pNum = participant.split("@")[0];
          const aNum = author?.split("@")[0] || "Unknown";
          const mentions = author ? [participant, author] : [participant];

          const fakeQuoted = {
            key: {
              fromMe: false,
              participant: participant,
              remoteJid: participant
            },
            message: {
              conversation: action === "promote"
                ? `Halo semua, aku sekarang admin disini`
                : `Yahhh, aku udah bukan admin lagi 😔`
            }
          };

          const media4 = await prepareWAMessageMedia({ image: groupHandler[rankCfg.imgKey] }, { upload: sock.waUploadToServer });

          const promoteButtons = [
            {
              name: "cta_url",
              buttonParamsJson: JSON.stringify({
                display_text: "🍙 Cara menjadi admin",
                url: "https://www.whatsapp.com/communities/learning/beingagoodadmin?lang=id",
                merchant_url: "https://www.whatsapp.com/communities/learning/beingagoodadmin?lang=id"
              })
            },
            {
              name: "quick_reply",
              buttonParamsJson: JSON.stringify({
                display_text: "🖐 Halo admin baru",
                id: ""
              })
            }
          ];

          const demoteButtons = [
            {
              name: "quick_reply",
              buttonParamsJson: JSON.stringify({
                display_text: "Tetap semangat ya! 💪",
                id: ""
              })
            }
          ];

          const msg4 = generateWAMessageFromContent(groupJid, {
            viewOnceMessage: {
              message: {
                messageContextInfo: {},
                interactiveMessage: {
                  header: { title: "", subtitle: "", hasMediaAttachment: true, imageMessage: media4.imageMessage },
                  footer: { text: config.bot?.name || "Ourin-AI" },
                  body: { text: rankCfg.text(pNum, aNum) },
                  contextInfo: {
                    mentionedJid: mentions,
                    isForwarded: true,
                    forwardingScore: 9999,
                    forwardedNewsletterMessageInfo: {
                      newsletterJid: config.saluran?.id || "120363400911374213@newsletter",
                      newsletterName: config.saluran?.name || config.bot?.name || "Ourin-AI",
                      serverMessageId: 127,
                    },
                  },
                  nativeFlowMessage: {
                    messageParamsJson: JSON.stringify({
                      limited_time_offer: {
                        text: action === "promote" ? `Selamat yahh 🎉` : `Tetap Semangat 📉`,
                        url: "Hai",
                        expiration_time: Date.now() + 1000000
                      },
                      bottom_sheet: { in_thread_buttons_limit: 2, divider_indices: [1, 2], list_title: "Opsi", button_title: "🍙 Lihat Opsi" },
                      tap_target_configuration: { title: " X ", description: "bomboclard", canonical_url: "https://ourin.site", domain: "shop.example.com", button_index: 0 },
                    }),
                    buttons: action === "promote" ? promoteButtons : demoteButtons
                  }
                }
              }
            }
          }, { quoted: fakeQuoted, userJid: sock.user.jid });

          await sock.relayMessage(groupJid, msg4.message, { messageId: msg4.key.id });
        }
      }
    }
  } catch (error) {
    console.error("[GroupHandler] Error:", error.message);
  }
}

async function messageUpdateHandler(updates, sock) {
  const db = getDatabase();

  for (const update of updates) {
    try {
      await handleAntiRemove(update, sock, db);
    } catch (error) {
      continue;
    }

    try {
      const editedMsg = update.update?.message?.editedMessage?.message;
      const regularMsg = update.update?.message;

      const resolvedMessage =
        editedMsg ||
        (regularMsg && !regularMsg.protocolMessage ? regularMsg : null);

      if (!resolvedMessage) continue;

      const newMsg = {
        key: update.key,
        message: editedMsg ? { ...resolvedMessage } : regularMsg,
        messageTimestamp:
          update.messageTimestamp || Math.floor(Date.now() / 1000),
        pushName: update.pushName || "User",
      };

      await messageHandler(newMsg, sock);
    } catch (error) {
      console.error("[MsgUpdate] Error:", error.message);
    }
  }
}

/**
 * Cache untuk menyimpan state terakhir grup
 * Format: { groupId: { announce: boolean, restrict: boolean, lastUpdate: timestamp } }
 */
const groupSettingsCache = new Map();

/**
 * Debounce cooldown untuk mencegah spam (dalam ms)
 */
const GROUP_SETTINGS_COOLDOWN = 1000;

async function groupSettingsHandler(update, sock) {
  try {
    if (global.sewaLeaving) return;
    if (global.isFetchingGroups) return;

    const groupId = update.id;
    if (!groupId || !groupId.endsWith("@g.us")) return;

    if (update.announce === undefined && update.restrict === undefined) {
      return;
    }

    const cached = groupSettingsCache.get(groupId) || {};
    const now = Date.now();

    if (
      cached.lastUpdate &&
      now - cached.lastUpdate < GROUP_SETTINGS_COOLDOWN
    ) {
      return;
    }

    let hasRealChange = false;

    let groupName = groupId;
    let groupPpUrl = null;
    try {
      const meta = await sock.groupMetadata(groupId);
      groupName = meta?.subject || groupId;
    } catch { }
    try {
      groupPpUrl = await sock.profilePictureUrl(groupId, "image");
    } catch { }

    const db = getDatabase();
    const groupData = db.getGroup(groupId) || {};

    const zannContext = {
      contextInfo: saluranCtx(),
    };

    if (update.announce !== undefined) {
      if (cached.announce === undefined) {
        cached.announce = update.announce;
      } else if (cached.announce !== update.announce) {
        hasRealChange = true;

        if (update.announce === true && groupData.notifCloseGroup === true) {
          await sock.sendText(
            groupId,
            `🥗 Grup *${groupName}* di tutup oleh admin`,
            null,
            zannContext,
          );
        }

        if (update.announce === false && groupData.notifOpenGroup === true) {
          await sock.sendText(
            groupId,
            `🎃 Grup *${groupName}* telah di buka kembali oleh admin`,
            null,
            zannContext,
          );
        }

        cached.announce = update.announce;
      }
    }

    if (update.restrict !== undefined) {
      if (cached.restrict === undefined) {
        cached.restrict = update.restrict;
      } else if (cached.restrict !== update.restrict) {
        hasRealChange = true;

        if (update.restrict === true) {
          await sock.sendText(
            groupId,
            `🥗 Info Grup *${groupName}* terbatas !\nHanya admin yang dapat mengedit grup`,
            null,
            zannContext,
          );
        } else {
          await sock.sendText(
            groupId,
            `🥗 Info Grup *${groupName}* terbuka !\nSemua member dapat mengedit grup`,
            null,
            zannContext,
          );
        }
        cached.restrict = update.restrict;
      }
    }
    if (hasRealChange) {
      cached.lastUpdate = now;
    }
    if (cached.announce !== undefined || cached.restrict !== undefined) {
      groupSettingsCache.set(groupId, cached);
    }
  } catch (error) {
    console.error("[GroupSettings] Error:", error.message);
  }
}

export {
  messageHandler,
  groupHandler,
  messageUpdateHandler,
  groupSettingsHandler,
  checkPermission,
  checkMode,
  isSpamming,
  handleAntiRemoveFromUpsert,
};
