import chalk from "chalk";
import * as timeHelper from "./ourin-time.js";
import { getCachedJid, isLidConverted } from "./ourin-lid.js";

// Mock gradient-string if any other file imports it from here
const gradientMock = (text) => text;
const gradient = () => gradientMock;

// 3 Main Colors
const cGreen = chalk.greenBright;
const cWhite = chalk.whiteBright;
const cGray = chalk.gray;

function makeTag(label, isSuccess = false, isError = false) {
  const l = label.toUpperCase().trim();
  let icon = "•";
  let colorFn = chalk.white;

  if (isSuccess || l === "OK" || l === "DONE") {
    icon = "✔";
    colorFn = chalk.green;
  } else if (isError || l === "FAIL" || l === "ERR" || l === "NO") {
    icon = "✖";
    colorFn = chalk.red;
  } else if (l === "WARN" || l === "WN") {
    icon = "⚠";
    colorFn = chalk.yellow;
  } else if (l === "INFO") {
    icon = "ℹ";
    colorFn = chalk.blue;
  } else if (l === "BOOT") {
    icon = "❖";
    colorFn = chalk.magenta;
  } else if (l === "SYS") {
    icon = "⚙";
    colorFn = chalk.cyan;
  } else if (l === "WAIT") {
    icon = "⟳";
    colorFn = chalk.yellow;
  } else if (l === "CMD") {
    icon = "❯";
    colorFn = chalk.magenta;
  } else if (l === "DBG") {
    icon = "🐛";
    colorFn = chalk.white;
  }

  const text = l.substring(0, 4).padEnd(4, " ");
  return `  ${colorFn(icon)}  ${colorFn(text)}`;
}

const SYM = {
  ok: makeTag("OK", true),
  no: makeTag("FAIL", false, true),
  wn: makeTag("WARN"),
  info: makeTag("INFO"),
  sys: makeTag("SYS"),
  dbg: makeTag("DBG"),
};

function writeLog(kind, label, detail = "") {
  const tags = {
    info: SYM.info,
    success: SYM.ok,
    warn: SYM.wn,
    error: SYM.no,
    system: SYM.sys,
    debug: SYM.dbg,
  };
  const tag = tags[kind] || SYM.info;

  // Format: [  OK  ] Started OKTZ AI
  const msg = `${tag} ${chalk.cyanBright(label)}${detail ? " " + cWhite(detail) : ""}`;
  console.log(msg);
}

const logger = {
  info: (label, detail = "") => writeLog("info", label, detail),
  success: (label, detail = "") => writeLog("success", label, detail),
  warn: (label, detail = "") => writeLog("warn", label, detail),
  error: (label, detail = "") => writeLog("error", label, detail),
  system: (label, detail = "") => writeLog("system", label, detail),
  debug: (label, detail = "") => writeLog("debug", label, detail),
  tag: (label, msg, detail = "") => {
    console.log(`${makeTag(label.substring(0, 4))} ${cWhite(msg)}${detail ? " " + cGray(detail) : ""}`);
  },
};

function createSpinner(label = "SYS", text = "loading", options = {}) {
  // Simplified spinner for linux style (just log the start)
  let active = false;
  return {
    start() {
      active = true;
      console.log(`${makeTag(label)} ${cWhite(text)}...`);
    },
    update(nextText) {
      if (active) console.log(`${makeTag(label)} ${cWhite(nextText)}...`);
    },
    stop() {
      active = false;
    },
    succeed(detail = text) {
      this.stop();
      logger.success(label, detail);
    },
    warn(detail = text) {
      this.stop();
      logger.warn(label, detail);
    },
    fail(detail = text) {
      this.stop();
      logger.error(label, detail);
    },
    isActive() {
      return active;
    }
  };
}

async function spinText(label, text, options = {}) {
  // Directly print success since we want a fast, simple boot
  console.log(`${makeTag("OK", true)} ${cWhite(text)}`);
}

async function typeLine(text, options = {}) {
  // Strip formatting from caller if it used old colors
  const clean = text.replace(/\x1B\[\d+m/g, "");
  console.log(`${makeTag("OK", true)} ${cWhite(clean)}`);
}

async function runLoader(text = "memuat", options = {}) {
  console.log(`${makeTag("OK", true)} ${cWhite(text)}`);
}

async function playBootSequence(info = {}) {
  const { name = "OKTZ", version = "3.3", mode = "public" } = info;
  console.log("");
  console.log(chalk.cyan(`
           ██████╗ ██╗   ██╗██████╗ ██╗███╗   ██╗
          ██╔═══██╗██║   ██║██╔══██╗██║████╗  ██║
          ██║   ██║██║   ██║██████╔╝██║██╔██╗ ██║
          ██║   ██║██║   ██║██╔══██╗██║██║╚██╗██║
          ╚██████╔╝╚██████╔╝██║  ██╗██║██║ ╚████║
           ╚═════╝  ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝
`));
  console.log(`         ${chalk.magenta.bold("►")} ${chalk.white("OKTZ MULTI-DEVICE BOT")} ${chalk.gray(`v${version}`)}`);
  console.log(`         ${chalk.magenta("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")}`);
  console.log("");
  console.log(`${makeTag("BOOT", true)} ${cWhite(`Memulai Sistem Utama...`)}`);
  console.log(`${makeTag("INFO")} ${cWhite(`Mode: ${chalk.cyan(mode)}`)}`);
}

function logCommand(info = {}) {
  const {
    prefix = ".", command, pushName, sender, chatType, groupName, messageType, device,
  } = info;
  if (!command) return;

  const time = timeHelper.formatTime("HH:mm:ss");
  const location = chatType === "group" || chatType === "newsletter"
    ? (groupName || "Group")
    : "Private";
  const senderName = pushName || "Pengguna";
  const typeTag = messageType || "Command";
  const msg = `${prefix}${command}`;

  console.log("");
  console.log(`  ${cWhite("╭─")} ${chalk.bgWhiteBright(" Command dieksekusi ")} ${cGray("•")} ${chatType === "private" ? chalk.yellow("Private") : chalk.whiteBright("Dari Grup") + " " + chalk.bgCyanBright(location)}`);
  console.log(`  ${cWhite("│")}  👤 ${chalk.greenBright(senderName)}`);
  console.log(`  ${cWhite("│")}  📱 ${chalk.yellowBright(device || "Unknown")} ${chalk.red(`• ${time} • ${typeTag}`)}`);
  console.log(`  ${cWhite("│")}  💬 ${chalk.whiteBright(msg)}`);
  console.log(`  ${cWhite("╰─")}`);
}
function logPlugin(name, category) {
  // Simple tree view for plugin
  console.log(`  ${cGray("├─")} ${cWhite(name)} ${cGray(`[${category}]`)}`);
}

function logConnection(status, info = "") {
  if (status === "connected") {
    console.log(`${makeTag("OK", true)} ${cWhite("Connected")} ${cGray(info ? `— ${info}` : "")}`);
  } else if (status === "connecting") {
    console.log(`${makeTag("WAIT")} ${cWhite("Connecting")} ${cGray(info ? `— ${info}` : "")}`);
  } else {
    console.log(`${makeTag("FAIL", false, true)} ${cWhite("Disconnected")} ${cGray(info ? `— ${info}` : "")}`);
  }
}

function logErrorBox(title, message) {
  console.log(`${makeTag("ERR", false, true)} ${cWhite(title)}: ${cGray(message)}`);
}

function printBanner(mini = false) {
  // No banner for linux style
}

function printStartup(info = {}) {
  // Already handled by boot sequence
}

const CODES = {
  reset: "", bold: "", dim: "", italic: "", underline: "",
  green: "", purple: "", white: "", gray: "", phantom: "",
  lime: "", silver: "", red: "", yellow: "", blue: "",
  cyan: "", magenta: "", bgBlack: "", bgGray: "",
};

// Map all colors to our 3 colors
const c = {
  green: cGreen,
  purple: cWhite,
  white: cWhite,
  gray: cGray,
  bold: (v) => v,
  dim: cGray,
  greenBold: cGreen,
  purpleBold: cWhite,
  whiteBold: cWhite,
  grayDim: cGray,
  red: cWhite,
  yellow: cWhite,
  cyan: cWhite,
  blue: cWhite,
  magenta: cWhite,
};

function divider() {
  // No divider for minimalism, or just a new line
  console.log("");
}

function createBanner(lines, color = "green") {
  return lines.map(l => `${cGray("│")} ${cWhite(l)}`).join("\n");
}

function getTimestamp() {
  return cGray(timeHelper.formatTime("HH:mm:ss"));
}

const theme = {
  primary: cWhite,
  secondary: cWhite,
  accent: cGreen,
  text: cWhite,
  dim: cGray,
  muted: cGray,
  success: cGreen,
  error: cWhite,
  warning: cWhite,
  info: cWhite,
  debug: cGray,
  border: cGray,
  tag: cWhite,
  pill: (t) => t,
  rainbow: gradientMock,
  borderFx: (t) => cGray(t),
  mintFx: (t) => cGreen(t),
  warmFx: (t) => cWhite(t),
  colorizeCategory: (t) => cWhite(t),
};

export {
  c,
  CODES,
  logger,
  createSpinner,
  spinText,
  typeLine,
  runLoader,
  playBootSequence,
  logCommand,
  logPlugin,
  logConnection,
  logErrorBox,
  printBanner,
  printStartup,
  createBanner,
  getTimestamp,
  divider,
  theme,
  chalk,
  gradient
};
