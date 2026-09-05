import * as botmodePlugin from "../group/botmode.js";
import { generateWAMessageFromContent, prepareWAMessageMedia, proto } from "ourin";
import config from "../../config.js";
import axios from "axios";
import {
  getTimeGreeting,
} from "../../src/lib/ourin-formatter.js";
import fs from "fs"
import {
  getCommandsByCategory,
  getCategories,
  getPluginCount,
  getPlugin,
  getPluginsByCategory,
} from "../../src/lib/ourin-plugins.js";
import { getCasesByCategory, getCaseCount } from "../../case/ourin.js";
import { getAssetBuffer } from "../../src/lib/ourin-asset-manager.js";
import { getFfmpegPath } from "../../src/lib/ourin-ffmpeg.js";

let _sharp;
async function getSharp() {
  if (!_sharp) _sharp = (await import("sharp")).default;
  return _sharp;
}
const pluginConfig = {
  name: "allmenu",
  alias: ["fullmenu", "am", "allcommand", "semua"],
  category: "main",
  description: "Menampilkan semua command lengkap per kategori",
  usage: ".allmenu",
  example: ".allmenu",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 5,
  energi: 0,
  isEnabled: true,
};
const CATEGORY_EMOJIS = {
  owner: "👑",
  main: "🏠",
  utility: "🔧",
  fun: "🎮",
  group: "👥",
  download: "📥",
  search: "🔍",
  tools: "🛠️",
  sticker: "🖼️",
  ai: "🤖",
  game: "🎯",
  media: "🎬",
  info: "ℹ️",
  religi: "☪️",
  panel: "🖥️",
  user: "📊",
  linode: "☁️",
  random: "🎲",
  canvas: "🎨",
  vps: "🌊",
  store: "🏪",
  premium: "💎",
  convert: "🔄",
  economy: "💰",
  cek: "📋",
  ephoto: "🎨",
  jpm: "📢",
  pushkontak: "📱",
};

function createBracketBox(emoji, title, lines = []) {
  let text = `╭➤--「${title} MENU 」\n`;
  for (const line of lines) {
    text += `┆ ⇝ ${line}\n`;
  }
  text += `╰➤-------------------------------\n\n`;
  return text;
}
function getCommandSymbols(cmdName) {
  const plugin = getPlugin(cmdName);
  if (!plugin || !plugin.config) return "";
  const symbols = [];
  if (plugin.config.isOwner) symbols.push("🅞");
  if (plugin.config.isPremium) symbols.push("🅟");
  if (plugin.config.limit && plugin.config.limit > 0) symbols.push("🅛");
  if (plugin.config.isAdmin) symbols.push("🅐");
  if (plugin.config.isGroup) symbols.push("🅖");
  if (plugin.config.isPrivate) symbols.push("🅟🅡");
  return symbols.length > 0 ? " " + symbols.join(" ") : "";
}
function getContextInfo(botConfig, m, thumbBuffer) {
  const saluranId = botConfig.saluran?.id || "120363400911374213@newsletter";
  const saluranName =
    botConfig.saluran?.name || botConfig.bot?.name || "Ourin-AI";
  const saluranLink = botConfig.saluran?.link || "";
  return {
    mentionedJid: [m.sender],
    forwardingScore: 9999,
    isForwarded: true,
    forwardedNewsletterMessageInfo: {
      newsletterJid: saluranId,
      newsletterName: saluranName,
      serverMessageId: 127,
    },
  };
}
async function handler(m, { sock, config: botConfig, db, uptime }) {
  const prefix = botConfig.command?.prefix || ".";
  const user = db.getUser(m.sender);
  const groupData = m.isGroup ? db.getGroup(m.chat) || {} : {};
  const botMode = groupData.botMode || "md";
  const categories = getCategories();
  const commandsByCategory = getCommandsByCategory();
  const casesByCategory = getCasesByCategory();
  let totalCommands = 0;
  for (const category of categories) {
    totalCommands += (commandsByCategory[category] || []).length;
  }
  const totalCases = getCaseCount();
  const totalFeatures = totalCommands + totalCases;
  let userRole = "User",
    roleEmoji = "👤";
  if (m.isOwner) {
    userRole = "Owner";
    roleEmoji = "👑";
  } else if (m.isPremium) {
    userRole = "Premium";
    roleEmoji = "💎";
  }
  const greeting = getTimeGreeting();
  let txt = ``;

  let imageBuffer = null;
  let thumbBuffer = null;
  try {
    imageBuffer = getAssetBuffer("ourin");
    thumbBuffer = getAssetBuffer("ourin2");
  } catch (e) { }

  const weatherCodeMap = {
    0: "☀️ Cerah", 1: "🌤️ Cerah Berawan", 2: "⛅ Berawan", 3: "☁️ Mendung", 45: "🌫️ Berkabut", 48: "🌫️ Kabut Tebal", 51: "🌦️ Gerimis", 61: "🌧️ Hujan Ringan", 63: "🌧️ Hujan", 65: "⛈️ Hujan Lebat", 80: "🌦️ Hujan Lokal", 95: "⛈️ Badai Petir"
  };

  let weatherText = "Merangin Cerah Berawan 22°C ☀️";
  try {
    const geo = await axios.get(`https://geocoding-api.open-meteo.com/v1/search?name=Merangin&count=1`);
    const loc = geo.data.results?.[0];
    if (loc) {
      const res = await axios.get(`https://api.open-meteo.com/v1/forecast?latitude=${loc.latitude}&longitude=${loc.longitude}&current=temperature_2m,weather_code`);
      const current = res.data.current;
      const kondisi = weatherCodeMap[current.weather_code] || "Cerah Berawan";
      weatherText = `Merangin ${kondisi} ${Math.round(current.temperature_2m)}°C`;
    }
  } catch (e) { }

  const userLimit = (m.isPremium || m.isOwner) ? "∞ Unlimited" : (user?.limit || 0);
  const botName = botConfig.bot?.name || "Remi AI";
  const devName = botConfig.bot?.developer || "Ell";
  const botVersion = botConfig.bot?.version || "11.0.0";
  const pushName = m.pushName || "User";
  const timeNow = new Date().toLocaleTimeString('id-ID', { timeZone: 'Asia/Jakarta' }).replace(/\./g, ':');

  txt += `✦ *Informasi User*\n`;
  txt += `• Nama   : ${pushName}\n`;
  txt += `• Status : ${userRole}\n`;
  txt += `• Limit  : ${userLimit}\n\n`;
  txt += `✦ *Informasi Bot*\n`;
  txt += `• Nama   : ${botName}\n`;
  txt += `• Fitur  : ${totalFeatures} fitur\n`;
  txt += `• Versi  : ${botVersion}\n`;
  txt += `• Waktu  : ${timeNow} WIB\n\n`;
  txt += `✦ *Kelebihan Bot Ini*\n`;
  txt += `• Fast response\n`;
  txt += `• Stable system\n`;
  txt += `• Multi feature\n`;
  txt += `• User friendly\n\n`;
  txt += `Silakan pilih menu di bawah ini.\n`;
  txt += `Gunakan sesuai kebutuhan dan jangan lupa gunakan dengan bijak.\n\n`;
  const categoryOrder = [
    "owner",
    "main",
    "utility",
    "tools",
    "fun",
    "game",
    "download",
    "search",
    "sticker",
    "media",
    "ai",
    "group",
    "religi",
    "info",
    "cek",
    "economy",
    "user",
    "canvas",
    "random",
    "premium"
  ];
  const sortedCategories = [...categories].sort((a, b) => {
    const indexA = categoryOrder.indexOf(a);
    const indexB = categoryOrder.indexOf(b);
    return (indexA === -1 ? 999 : indexA) - (indexB === -1 ? 999 : indexB);
  });
  let modeAllowedMap = {
    md: null,
    cpanel: ["main", "group", "sticker", "owner", "tools", "panel"],
    store: ["main", "group", "sticker", "owner", "store"],
    pushkontak: ["main", "group", "sticker", "owner", "pushkontak"],
  };
  let modeExcludeMap = {
    md: ["panel", "pushkontak", "store"],
    cpanel: null,
    store: null,
    pushkontak: null,
  };
  try {
    if (botmodePlugin && botmodePlugin.MODES) {
      const modes = botmodePlugin.MODES;
      modeAllowedMap = {};
      modeExcludeMap = {};
      for (const [key, val] of Object.entries(modes)) {
        modeAllowedMap[key] = val.allowedCategories;
        modeExcludeMap[key] = val.excludeCategories;
      }
    }
  } catch (e) { }
  const allowedCategories = modeAllowedMap[botMode];
  const excludeCategories = modeExcludeMap[botMode] || [];
  for (const category of sortedCategories) {
    if (category === "owner" && !m.isOwner) continue;
    if (
      allowedCategories &&
      !allowedCategories.includes(category.toLowerCase())
    )
      continue;
    if (excludeCategories && excludeCategories.includes(category.toLowerCase()))
      continue;
    const pluginCmds = commandsByCategory[category] || [];
    const caseCmds = casesByCategory[category] || [];
    const allCmds = [...pluginCmds, ...caseCmds];
    if (allCmds.length === 0) continue;
    const emoji = CATEGORY_EMOJIS[category] || "📋";
    const categoryName = category.toUpperCase();
    const commandLines = allCmds.map((cmd) => {
      const symbols = getCommandSymbols(cmd);
      return `${prefix}${cmd}${symbols}`;
    });
    txt += createBracketBox(emoji, categoryName, commandLines);
  }
  const savedVariant = db.setting("allmenuVariant");
  const allmenuVariant = savedVariant || botConfig.ui?.allmenuVariant || 2;
  try {
    switch (allmenuVariant) {
      case 1:
        await m.reply(txt);
        break;
      case 2:
        const media = await prepareWAMessageMedia({
          image: getAssetBuffer("ourin")
        }, { upload: sock.waUploadToServer })
        await sock.relayMessage(
          m.chat,
          {
            messageContextInfo: {},
            interactiveMessage: {
              header: {
                title: "",
                subtitle: "",
                hasMediaAttachment: true,
                imageMessage: media.imageMessage
              },
              body: {
                text: `> Halo *${pushName}* 👋 Perkenalkan aku *${botName}*, buatan *${devName}* Saya siap membantu kebutuhanmu, mulai dari download video, main game, tanya jawab, nyari info/sesuatu, bikin sticker, dan lain-lain.\n\n`,
              },
              footer: {
                text: txt
              },
              contextInfo: {
                isForwarded: true,
                fprwardingScore: 9,
                participant: "0@s.whatsapp.net",
                quotedMessage: {
                  conversation: `${config.bot?.name}`
                },
                mentionedJid: [
                  `${m.sender}`
                ]
              },
              nativeFlowMessage: {
                messageParamsJson: JSON.stringify({
                  limited_time_offer: {
                    text: `${greeting}`,
                    url: "Hai",
                    copy_code: "Dibuat oleh " + config.bot?.developer,
                    expiration_time: Date.now() + 1000000,
                  },
                }),
                buttons: [
                  {
                    name: "quick_reply",
                    buttonParamsJson: JSON.stringify({
                      display_text: "🍅 Kembali Ke Menu Utama",
                      id: m.prefix + "menu"
                    })
                  }
                ]
              }
            }
          },
          {}
        )
        break;
      case 5: {
        function runtime(seconds) {
          seconds = Number(seconds);
          const d = Math.floor(seconds / (3600 * 24));
          const h = Math.floor(seconds % (3600 * 24) / 3600);
          const m = Math.floor(seconds % 3600 / 60);
          const s = Math.floor(seconds % 60);
          return `${d} Jam ${m} Menit ${s} Detik`;
        }

        const weatherCode = {
          0: "☀️ Cerah", 1: "🌤️ Cerah Berawan", 2: "⛅ Berawan", 3: "☁️ Mendung", 45: "🌫️ Berkabut", 48: "🌫️ Kabut Tebal", 51: "🌦️ Gerimis", 61: "🌧️ Hujan Ringan", 63: "🌧️ Hujan", 65: "⛈️ Hujan Lebat", 80: "🌦️ Hujan Lokal", 95: "⛈️ Badai Petir"
        }

        async function weatherMenu(city = "Jakarta") {
          try {
            const geo = await axios.get(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1`)
            const loc = geo.data.results?.[0]
            if (!loc) return "Cuaca tidak tersedia"
            const res = await axios.get(`https://api.open-meteo.com/v1/forecast?latitude=${loc.latitude}&longitude=${loc.longitude}&current=temperature_2m,weather_code`)
            const current = res.data.current
            const kondisi = weatherCode[current.weather_code] || "🌍 Tidak diketahui"
            return `${kondisi} | 🌡️ ${Math.round(current.temperature_2m)}°C\n📍 ${loc.name}`
          } catch {
            return "Cuaca tidak tersedia"
          }
        }

        const thumbnail = await (await getSharp())(getAssetBuffer("ourin")).resize(300, 300).toBuffer()
        const qOrder = {
          key: { fromMe: false, participant: '0@s.whatsapp.net', remoteJid: m.sender },
          message: { locationMessage: { degreesLatitude: 0, degreesLongitude: 0, name: await weatherMenu(), jpegThumbnail: thumbnail } }
        }
        const media4 = await prepareWAMessageMedia({ video: fs.readFileSync(config.assets["ourin-mp4"]), gifPlayback: true }, { upload: sock.waUploadToServer });
        const msg4 = generateWAMessageFromContent(m.chat, {
          messageContextInfo: {},
          interactiveMessage: {
            header: { title: "", subtitle: "", hasMediaAttachment: true, videoMessage: media4.videoMessage },
            footer: { text: `Please select the button in below` },
            body: { text: txt },
            contextInfo: {
              mentionedJid: [m.sender],
              isForwarded: true,
              forwardingScore: 9,
              forwardedNewsletterMessageInfo: {
                newsletterJid: config.saluran?.id || "120363400911374213@newsletter",
                newsletterName: config.saluran?.name || config.bot?.name || "Ourin-AI",
                serverMessageId: 127,
              },
            },
            nativeFlowMessage: {
              messageParamsJson: JSON.stringify({
                limited_time_offer: { text: `${greeting}`, url: "Hai", expiration_time: Date.now() + 10000 },
                bottom_sheet: { in_thread_buttons_limit: 2, divider_indices: [1, 2, 3, 4, 5, 999], list_title: "Please select the menu", button_title: "🍙 See Category" },
                tap_target_configuration: { title: " X ", description: "bomboclard", canonical_url: "https://ourin.site", domain: "shop.example.com", button_index: 0 },
              }),
              buttons: [
                { name: "", buttonParamsJson: "" },
                {
                  name: "quick_reply",
                  buttonParamsJson: JSON.stringify({ display_text: "📂 Kembali Ke Daftar Kategori", id: m.prefix + "menucat" })
                },
                {
                  name: "quick_reply",
                  buttonParamsJson: JSON.stringify({ display_text: "🍅 Kembali Ke Menu Utama", id: m.prefix + "menu" })
                },
              ]
            }
          }
        }, { quoted: qOrder, userJid: sock.user?.id });

        await sock.relayMessage(m.chat, msg4.message, { messageId: msg4.key.id });
        break;
      }
      case 6: {
        const weatherCode = {
          0: "☀️ Cerah", 1: "🌤️ Cerah Berawan", 2: "⛅ Berawan", 3: "☁️ Mendung", 45: "🌫️ Berkabut", 48: "🌫️ Kabut Tebal", 51: "🌦️ Gerimis", 61: "🌧️ Hujan Ringan", 63: "🌧️ Hujan", 65: "⛈️ Hujan Lebat", 80: "🌦️ Hujan Lokal", 95: "⛈️ Badai Petir"
        }

        async function weatherMenu(city = "Jakarta") {
          try {
            const geo = await axios.get(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1`)
            const loc = geo.data.results?.[0]
            if (!loc) return "Cuaca tidak tersedia"
            const res = await axios.get(`https://api.open-meteo.com/v1/forecast?latitude=${loc.latitude}&longitude=${loc.longitude}&current=temperature_2m,weather_code`)
            const current = res.data.current
            const kondisi = weatherCode[current.weather_code] || "🌍 Tidak diketahui"
            return `${kondisi} | 🌡️ ${Math.round(current.temperature_2m)}°C\n📍 ${loc.name}`
          } catch {
            return "Cuaca tidak tersedia"
          }
        }

        const thumbnail = await (await getSharp())(getAssetBuffer("ourin")).resize(300, 300).toBuffer()

        const msg6 = generateWAMessageFromContent(m.chat, {
          messageContextInfo: {},
          interactiveMessage: {
            header: {
              hasMediaAttachment: true,
              locationMessage: {
                degreesLatitude: 0,
                degreesLongitude: 0,
                name: config.bot?.name || "Ourin-AI",
                address: await weatherMenu(),
                jpegThumbnail: thumbnail
              }
            },
            body: {
              text: txt,
            },
            contextInfo: {
              mentionedJid: [m.sender],
              isForwarded: true,
              forwardingScore: 9,
              forwardedNewsletterMessageInfo: {
                newsletterJid: config.saluran?.id || "120363400911374213@newsletter",
                newsletterName: config.saluran?.name || config.bot?.name || "Ourin-AI",
                serverMessageId: 127,
              },
            },
            nativeFlowMessage: {
              buttons: []
            }
          }
        }, { quoted: m, userJid: sock.user?.id });

        await sock.relayMessage(m.chat, msg6.message, { messageId: msg6.key.id });
        break;
      }
      default:
        break
    }
    const audioEnabled = db.setting("audioMenu") !== false;
    if (audioEnabled) {
      const audioUrl = botConfig.assets["ourin-mp3"];
      const audioVariant = db.setting("allmenuAudioStyle") || 1;
      try {
        const fs = (await import("fs")).default;
        const path = (await import("path")).default;
        const axios = (await import("axios")).default;

        switch (audioVariant) {
          case 1:
            try {
              const oggPath = await (async () => {
                const tempDir = path.join(process.cwd(), "temp");
                if (!fs.existsSync(tempDir)) fs.mkdirSync(tempDir, { recursive: true });
                const destPath = path.join(tempDir, "allmenu_audio_opus.ogg");
                if (fs.existsSync(destPath)) return destPath;
                const mp3Path = path.join(tempDir, "allmenu_audio.mp3");
                const res = await axios.get(audioUrl, { responseType: "arraybuffer" });
                fs.writeFileSync(mp3Path, Buffer.from(res.data));
                const { spawn } = await import("child_process");
                return new Promise((resolve, reject) => {
                  const ffmpeg = spawn(getFfmpegPath() ?? "ffmpeg", ["-y", "-i", mp3Path, "-c:a", "libopus", "-b:a", "48k", "-vbr", "on", destPath]);
                  ffmpeg.on("close", (code) => {
                    if (fs.existsSync(mp3Path)) fs.unlinkSync(mp3Path);
                    if (code === 0) resolve(destPath);
                    else reject(new Error("FFmpeg error"));
                  });
                  ffmpeg.on("error", (err) => {
                    if (fs.existsSync(mp3Path)) fs.unlinkSync(mp3Path);
                    reject(err);
                  });
                });
              })();
              await sock.sendMessage(m.chat, {
                audio: { url: oggPath },
                mimetype: "audio/ogg; codecs=opus",
                ptt: true,
              }, { quoted: m });
            } catch (err) {
              await sock.sendMessage(m.chat, {
                audio: { url: audioUrl },
                mimetype: "audio/mpeg",
                ptt: false,
              }, { quoted: m });
            }
            break;
          case 2: {
            const qpoll = {
              key: { participant: "0@s.whatsapp.net" },
              message: {
                pollCreationMessage: {
                  name: config.bot.name
                }
              }
            };
            try {
              const oggPath = await (async () => {
                const tempDir = path.join(process.cwd(), "temp");
                if (!fs.existsSync(tempDir)) fs.mkdirSync(tempDir, { recursive: true });
                const destPath = path.join(tempDir, "allmenu_audio_opus.ogg");
                if (fs.existsSync(destPath)) return destPath;
                const mp3Path = path.join(tempDir, "allmenu_audio.mp3");
                const res = await axios.get(audioUrl, { responseType: "arraybuffer" });
                fs.writeFileSync(mp3Path, Buffer.from(res.data));
                const { spawn } = await import("child_process");
                return new Promise((resolve, reject) => {
                  const ffmpeg = spawn(getFfmpegPath() ?? "ffmpeg", ["-y", "-i", mp3Path, "-c:a", "libopus", "-b:a", "48k", "-vbr", "on", destPath]);
                  ffmpeg.on("close", (code) => {
                    if (fs.existsSync(mp3Path)) fs.unlinkSync(mp3Path);
                    if (code === 0) resolve(destPath);
                    else reject(new Error("FFmpeg error"));
                  });
                  ffmpeg.on("error", (err) => {
                    if (fs.existsSync(mp3Path)) fs.unlinkSync(mp3Path);
                    reject(err);
                  });
                });
              })();
              await sock.sendMessage(m.chat, {
                audio: { url: oggPath },
                mimetype: "audio/ogg; codecs=opus",
                ptt: true,
              }, { quoted: qpoll });
            } catch (err) {
              await sock.sendMessage(m.chat, {
                audio: { url: audioUrl },
                mimetype: "audio/mpeg",
                ptt: false,
              }, { quoted: qpoll });
            }
            break;
          }
          case 3: {
            const qtext = {
              key: {
                fromMe: false,
                participant: m.sender,
              },
              message: {
                conversation: "setelin musiknya nya bang"
              }
            };
            await sock.sendMessage(m.chat, {
              audio: fs.readFileSync(config.assets["ourin-mp3"]),
              mimetype: "audio/mpeg",
              ptt: false,
            }, { quoted: qtext });
            break;
          }
          default: {
            const ftroliQuoted = {
              key: {
                fromMe: false,
                participant: "0@s.whatsapp.net",
                remoteJid: "status@broadcast",
              },
              message: {
                orderMessage: {
                  orderId: "44444444444444",
                  thumbnail:
                    (thumbBuffer || imageBuffer ? await (await getSharp())(thumbBuffer || imageBuffer)
                      .resize({ width: 300, height: 300 })
                      .toBuffer() : null),
                  itemCount: totalCmds,
                  status: "INQUIRY",
                  surface: "CATALOG",
                  message: `★ ${config.bot.name}`,
                  orderTitle: `📋 ${totalCmds} Commands`,
                  sellerJid: botConfig.botNumber
                    ? `${botConfig.botNumber}@s.whatsapp.net`
                    : m.sender,
                  token: "ourin-menu-v8",
                  totalAmount1000: 3333333,
                  totalCurrencyCode: "IDR",
                  contextInfo: {
                    isForwarded: true,
                    forwardingScore: 9,
                    forwardedNewsletterMessageInfo: {
                      newsletterJid: "120363351980387532@newsletter",
                      newsletterName: "Ourin Bot",
                      serverMessageId: 127,
                    },
                  },
                },
              },
            };
            try {
              await sock.sendMessage(
                m.chat,
                {
                  audio: fs.readFileSync(config.assets["ourin-mp3"]),
                  mimetype: "audio/mpeg",
                },
                { quoted: ftroliQuoted },
              );
            } catch (ffmpegErr) {
              await sock.sendMessage(
                m.chat,
                {
                  audio: fs.readFileSync(config.assets["ourin-mp3"]),
                  mimetype: "audio/mpeg",
                },
                { quoted: m },
              );
            }
            break;
          }
        }
      } catch (e) {
        console.error("[AllMenu] Error sending dynamic audio:", e.message);
      }
    }
  } catch (error) {
    console.error("[AllMenu] Error:", error.message);
    if (imageBuffer) {
      await sock.sendMessage(
        m.chat,
        {
          image: imageBuffer,
          caption: txt,
          contextInfo: getContextInfo(botConfig, m),
        },
        { quoted: m },
      );
    } else {
      await m.reply(txt);
    }
  }
}
export { pluginConfig as config, handler };
