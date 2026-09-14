import { createCanvas, loadImage, GlobalFonts } from "@napi-rs/canvas";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "fs";
import path from "path";
import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "brattheme",
  alias: ["bratblack", "bratijo", "bratblur"],
  category: "canvas",
  description: "Generator brat tema (black/white/green), emoji apple, blur",
  usage: ".brattheme <theme> <teks> [-blur 1-3]",
  example: ".brattheme green Halo Guys Nama Saya",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

const TMP_DIR = path.join(process.cwd(), "tmp");
const FONT_PATH = path.join(TMP_DIR, "ARIALN.ttf");
const EMOJI_JSON_PATH = path.join(TMP_DIR, "emoji-apple.json");

const THEMES = {
  black: { bg: "#000000", text: "#ffffff" },
  white: { bg: "#ffffff", text: "#000000" },
  green: { bg: "#8ace00", text: "#000000" },
};

// Satu regex utk split (global), satu utk cek per-token (non-global).
// Versi asli pakai satu global + reset lastIndex -> rapuh. Dipisah = benar.
const EMOJI_SPLIT_RE = /(\p{Emoji_Presentation}\uFE0F?|\p{Emoji}\uFE0F|[\u{1F1E0}-\u{1F1FF}]{2}|\p{Extended_Pictographic}\uFE0F?)/gu;
const EMOJI_TEST_RE = /\p{Emoji_Presentation}\uFE0F?|\p{Emoji}\uFE0F|[\u{1F1E0}-\u{1F1FF}]{2}|\p{Extended_Pictographic}\uFE0F?/u;

let emojiMap = null;
let fontReady = false;
let fetchingFont = null;
let fetchingEmoji = null;
// Cache image emoji dibatasi (LRU-ish) supaya memori stabil di RAM kecil.
const EMOJI_CACHE_MAX = 128;
const emojiImageCache = new Map();

function ensureTmpDir() {
  if (!existsSync(TMP_DIR)) mkdirSync(TMP_DIR, { recursive: true });
}

// Unduh sekali lalu cache ke disk. Fallback URL aset via env.
async function downloadFile(url, dest) {
  ensureTmpDir();
  const res = await fetch(url);
  if (!res.ok) throw new Error(`download ${url}: HTTP ${res.status}`);
  writeFileSync(dest, Buffer.from(await res.arrayBuffer()));
}

async function ensureFont() {
  if (fontReady) return;
  // Dedupe: concurrent call cuma satu unduhan.
  if (!fetchingFont) {
    fetchingFont = (async () => {
      if (!existsSync(FONT_PATH)) await downloadFile(process.env.BRAT_FONT_URL || "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/Font/ARIALN.ttf", FONT_PATH);
      GlobalFonts.registerFromPath(FONT_PATH, "ArialNarrow");
      fontReady = true;
    })().finally(() => { fetchingFont = null; });
  }
  return fetchingFont;
}

async function loadEmojiMap() {
  if (emojiMap) return emojiMap;
  if (!fetchingEmoji) {
    fetchingEmoji = (async () => {
      if (!existsSync(EMOJI_JSON_PATH)) await downloadFile(process.env.BRAT_EMOJI_URL || "https://media.githubusercontent.com/media/Ditzzx-vibecoder/entahlah/main/emoji-apple.json", EMOJI_JSON_PATH);
      emojiMap = JSON.parse(readFileSync(EMOJI_JSON_PATH, "utf-8"));
    })().finally(() => { fetchingEmoji = null; });
  }
  return fetchingEmoji;
}

function emojiToUnicode(emoji) {
  return [...emoji].map((c) => c.codePointAt(0).toString(16).padStart(4, "0")).join("-");
}

async function getEmojiImage(emoji) {
  if (emojiImageCache.has(emoji)) return emojiImageCache.get(emoji);
  const map = await loadEmojiMap();
  const base = emojiToUnicode(emoji);
  const noVs = base.replace(/-fe0f/gi, "");
  const variants = [
    base, noVs, `${noVs}-fe0f`, base.toUpperCase(),
    noVs.toUpperCase(), `${noVs.toUpperCase()}-FE0F`,
  ];
  let b64 = null;
  for (const v of variants) if (map[v]) { b64 = map[v]; break; }
  if (!b64) return null;
  const img = await loadImage(Buffer.from(b64, "base64"));
  if (emojiImageCache.size >= EMOJI_CACHE_MAX) emojiImageCache.delete(emojiImageCache.keys().next().value);
  emojiImageCache.set(emoji, img);
  return img;
}

async function drawAppleEmoji(ctx, emoji, x, y, size) {
  const img = await getEmojiImage(emoji);
  if (!img) { ctx.fillText(emoji, x, y); return; }
  ctx.drawImage(img, x, y, size, size);
}

function measureTextCustom(ctx, text, fontSize) {
  let w = 0;
  for (const part of text.split(EMOJI_SPLIT_RE)) {
    if (!part) continue;
    w += EMOJI_TEST_RE.test(part) ? fontSize : ctx.measureText(part).width;
  }
  return w;
}

async function drawTextWithEmojis(ctx, text, x, y, fontSize) {
  let curX = x;
  for (const part of text.split(EMOJI_SPLIT_RE)) {
    if (!part) continue;
    if (EMOJI_TEST_RE.test(part)) {
      await drawAppleEmoji(ctx, part, curX, y, fontSize);
      curX += fontSize;
    } else {
      ctx.fillText(part, curX, y);
      curX += ctx.measureText(part).width;
    }
  }
}

function wrapText(ctx, text, maxWidth, fontSize) {
  ctx.font = `${fontSize}px ArialNarrow`;
  const words = text.split(" ");
  const lines = [];
  let cur = "";
  for (const word of words) {
    const test = cur ? `${cur} ${word}` : word;
    if (measureTextCustom(ctx, test, fontSize) > maxWidth && cur) {
      lines.push(cur);
      cur = word;
    } else {
      cur = test;
    }
  }
  if (cur) lines.push(cur);
  return lines;
}

function fitsAt(ctx, text, fontSize, maxWidth, maxHeight, lineGap, lines) {
  lines = lines || wrapText(ctx, text, maxWidth, fontSize);
  let longestWord = 0;
  for (const w of text.split(" ")) {
    const lw = measureTextCustom(ctx, w, fontSize);
    if (lw > longestWord) longestWord = lw;
  }
  const totalHeight = lines.length * (fontSize + lineGap) - lineGap;
  return longestWord <= maxWidth && totalHeight <= maxHeight;
}

function findBestFontSize(ctx, text, maxWidth, maxHeight, lineGap) {
  let lo = 10, hi = 700, best = 10;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    // Wrap sekali per percobaan, reuse utk cek tinggi -> potong measureText dobel.
    const lines = wrapText(ctx, text, maxWidth, mid);
    if (fitsAt(ctx, text, mid, maxWidth, maxHeight, lineGap, lines)) {
      best = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return best;
}

export async function generateBrat({ text = "Halo Guys Nama Saya", theme = "white", blur = 0 } = {}) {
  const selectedTheme = THEMES[theme] || THEMES.white;
  const blurAmount = [0, 1, 2, 3].includes(blur) ? blur : 0;

  const size = 1000;
  const padding = 80;
  const lineGap = 20;
  const maxWidth = size - padding * 2;
  const maxHeight = size - padding * 2;

  await ensureFont();
  // Emoji JSON besar -> hanya dimuat kalau teks memang ada emoji (hemat memori).
  const hasEmoji = EMOJI_TEST_RE.test(text);
  if (hasEmoji) await loadEmojiMap();

  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext("2d");

  const fontSize = findBestFontSize(ctx, text, maxWidth, maxHeight, lineGap);
  const lines = wrapText(ctx, text, maxWidth, fontSize);

  ctx.fillStyle = selectedTheme.bg;
  ctx.fillRect(0, 0, size, size);

  ctx.fillStyle = selectedTheme.text;
  ctx.font = `${fontSize}px ArialNarrow`;
  ctx.textAlign = "left";
  ctx.textBaseline = "top";

  const totalTextHeight = lines.length * (fontSize + lineGap) - lineGap;
  let y = (size - totalTextHeight) / 2;
  for (const line of lines) {
    await drawTextWithEmojis(ctx, line, padding, y, fontSize);
    y += fontSize + lineGap;
  }

  let buf = await canvas.encode("png");

  // @napi-rs/canvas TIDAK menerapkan ctx.filter (no-op) -> kode asli mati.
  // Blur betulan pakai sharp (C++, murah). Default blur=0: nol overhead.
  if (blurAmount > 0) {
    const { default: sharp } = await import("sharp");
    buf = await sharp(buf).blur(blurAmount).png().toBuffer();
  }

  return buf;
}

// ".brattheme green halo -blur 2" -> theme=green, blur=2, text="halo"
function parseArgs(input) {
  let text = input.trim();
  let theme = "white";
  let blur = 0;
  const bm = text.match(/-blur\s*([0-3])/);
  if (bm) { blur = Number(bm[1]); text = text.replace(bm[0], "").trim(); }
  const first = text.split(/\s+/)[0]?.toLowerCase();
  if (first && THEMES[first]) { theme = first; text = text.split(/\s+/).slice(1).join(" ").trim(); }
  return { text, theme, blur };
}

async function handler(m, { sock }) {
  const { text, theme, blur } = parseArgs(m.args.join(" ") || "");

  if (!text) {
    return m.reply(
      `🖼️ *ʙʀᴀᴛ ᴛʜᴇᴍᴇ*\n\n> Teks kosong.\n\n\`Tema: black | white | green\`\n\`Blur: -blur 1..3\`\n\n\`Contoh: ${m.prefix}brattheme green Halo Guys 👑\``,
    );
  }

  m.react("🕕");

  try {
    const buffer = await generateBrat({ text, theme, blur });
    await sock.sendImageAsSticker(m.chat, buffer, m, {
      packname: config.sticker.packname,
      author: config.sticker.author,
    });
    m.react("✅");
  } catch (error) {
    m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
