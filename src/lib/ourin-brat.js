import { createCanvas, loadImage, GlobalFonts } from "@napi-rs/canvas";
import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from "fs";
import path from "path";

function getTokenWidth(ctx, token, fontSize) {
  if (token.type === "space") return ctx.measureText(" ").width;
  if (token.type === "emoji") return fontSize * 1.15;
  return ctx.measureText(token.value || "").width;
}

function buildLines(ctx, tokens, fontSize, maxW, family) {
  ctx.font = `bold ${fontSize}px ${family}`;
  const lines = [];
  let line = [];
  let lineW = 0;

  for (const token of tokens) {
    const w = getTokenWidth(ctx, token, fontSize);

    if (token.type === "space") {
      if (line.length > 0) {
        line.push({ ...token, w });
        lineW += w;
      }
      continue;
    }

    if (line.length > 0 && lineW + w > maxW) {
      while (line.length > 0 && line[line.length - 1].type === "space") {
        lineW -= line[line.length - 1].w;
        line.pop();
      }
      lines.push({ items: line, width: lineW });
      line = [{ ...token, w }];
      lineW = w;
    } else {
      line.push({ ...token, w });
      lineW += w;
    }
  }

  if (line.length > 0) {
    while (line.length > 0 && line[line.length - 1].type === "space") {
      lineW -= line[line.length - 1].w;
      line.pop();
    }
    lines.push({ items: line, width: lineW });
  }

  return lines;
}

export function tokenize(text) {
  const emojiRegex = /\p{Emoji_Presentation}|\p{Extended_Pictographic}/gu;
  const raw = [];
  let lastIndex = 0;
  let match;

  while ((match = emojiRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      raw.push({ type: "text", value: text.slice(lastIndex, match.index) });
    }
    raw.push({ type: "emoji", value: match[0] });
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    raw.push({ type: "text", value: text.slice(lastIndex) });
  }

  const tokens = [];
  for (const seg of raw) {
    if (seg.type === "emoji") {
      if (tokens.length > 0) tokens.push({ type: "space" });
      tokens.push({ type: "emoji", value: seg.value });
    } else {
      const words = seg.value.split(/\s+/).filter(w => w.length > 0);
      words.forEach(w => {
        if (tokens.length > 0) tokens.push({ type: "space" });
        tokens.push({ type: "text", value: w });
      });
    }
  }
  return tokens;
}

export async function drawBrat({
  text,
  bgUrl,
  bgColor,
  width,
  height,
  centerX,
  centerY,
  maxWidth,
  maxHeight,
  rotationAngle = 0,
  maxFontSize = 130,
  minFontSize = 10,
  fontDecrement = 2,
  lineHeightMult = 1.2,
  textColor = "#000000",
  align = "center",
  textBaseline = "middle"
}) {
  let bg = null;
  const family = await loadBratFont();
  if (bgUrl) {
    bg = await loadImage(bgUrl);
    width = width || bg.width;
    height = height || bg.height;
  }
  
  const canvas = createCanvas(width || 512, height || 512);
  const ctx = canvas.getContext("2d");

  if (bg) {
    ctx.drawImage(bg, 0, 0, width, height);
  } else if (bgColor) {
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);
  }

  const tokens = tokenize(text);
  let fontSize = maxFontSize;
  let lines = buildLines(ctx, tokens, fontSize, maxWidth, family);

  while (fontSize > minFontSize) {
    lines = buildLines(ctx, tokens, fontSize, maxWidth, family);
    const totalH = lines.length * fontSize * lineHeightMult;
    if (totalH <= maxHeight) break;
    fontSize -= fontDecrement;
  }

  const lineHeight = fontSize * lineHeightMult;
  const totalHeight = lines.length * lineHeight;

  const cx = typeof centerX === 'function' ? centerX(width, height) : (centerX || width / 2);
  const cy = typeof centerY === 'function' ? centerY(width, height) : (centerY || height / 2);

  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate(rotationAngle);
  ctx.fillStyle = textColor;
  ctx.textBaseline = textBaseline;

  let startY = -(totalHeight / 2) + (lineHeight / 2);

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const y = startY + i * lineHeight;
    ctx.font = `bold ${fontSize}px ${family}`;

    let currentX = align === "center" ? -line.width / 2 : -(maxWidth / 2);

    for (const token of line.items) {
      if (token.type === "text") {
        ctx.fillText(token.value, currentX, y);
        currentX += token.w;
      } else if (token.type === "space") {
        currentX += token.w;
      } else if (token.type === "emoji") {
        ctx.fillText(token.value, currentX, y);
        currentX += token.w;
      }
    }
  }

  ctx.restore();
  return canvas.encode("png");
}

// ─── Brat tema (white/black/green) + emoji apple + blur ───
// Port dari plugin brattheme dengan perbaikan:
// - FONT BLANK FIX: aset TTF/JSON divalidasi dulu (cek magic-byte/JSON) sebelum
//   cache ke disk; cache korup self-heal; register gagal -> fallback ke font
//   lokal repo (assets/ourin-font.ttf). Dulu: HTML dari proxy (HTTP 200) ke-cache
//   sebagai .ttf -> register gagal diam -> VPS tanpa font sistem = teks tak
//   ter-render -> sticker putih polos "blank" tanpa error.
// - JSON emoji besar hanya dimuat saat teks benar-benar ada emoji (hemat RAM).
// - ctx.filter blur di @napi-rs/canvas itu no-op -> blur asli pakai sharp,
//   dan hanya diimpor saat blur > 0 (nol overhead di jalur default).

const BRAT_THEMES = {
  black: { bg: "#000000", text: "#ffffff" },
  white: { bg: "#ffffff", text: "#000000" },
  green: { bg: "#8ace00", text: "#000000" },
};

const EMOJI_SPLIT_RE = /(\p{Emoji_Presentation}\uFE0F?|\p{Emoji}\uFE0F|[\u{1F1E0}-\u{1F1FF}]{2}|\p{Extended_Pictographic}\uFE0F?)/gu;
const EMOJI_TEST_RE = /\p{Emoji_Presentation}\uFE0F?|\p{Emoji}\uFE0F|[\u{1F1E0}-\u{1F1FF}]{2}|\p{Extended_Pictographic}\uFE0F?/u;

const bratTmp = () => {
  const d = path.join(process.cwd(), "tmp");
  if (!existsSync(d)) mkdirSync(d, { recursive: true });
  return d;
};

let bratFontFamily = null;
let bratFontPromise = null;
let bratEmojiMap = null;
let bratEmojiPromise = null;
const BRAT_EMOJI_CACHE_MAX = 128;
const bratEmojiCache = new Map();

function isTrueTypeBuffer(b) {
  if (!b || b.length < 1024) return false;
  const magic = b.readUInt32BE(0);
  return magic === 0x00010000 || magic === 0x4f54544f; // \0\1 TrueType | 'OTTO'
}

function isJsonBuffer(b) {
  if (!b || b.length < 16) return false;
  const c = b[0];
  return c === 0x7b || c === 0x5b; // '{' | '['
}

function validCachedFile(p, check) {
  if (!existsSync(p)) return false;
  try {
    const fd = readFileSync(p);
    if (check(fd)) return true;
    unlinkSync(p); // self-heal cache korup
  } catch {}
  return false;
}

async function bratDownloadValidated(url, dest, check) {
  const res = await fetch(url, { signal: AbortSignal.timeout(30000) });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  if (!check(buf)) throw new Error("payload invalid (bukan font/JSON — kena intercept proxy?)");
  writeFileSync(dest, buf);
  return buf;
}

async function loadBratFont() {
  if (bratFontFamily) return bratFontFamily;
  if (!bratFontPromise) {
    bratFontPromise = (async () => {
      const remote = path.join(bratTmp(), "ARIALN.ttf");
      const local = path.join(process.cwd(), "assets", "ourin-font.ttf");
      const candidates = [];
      try {
        if (!validCachedFile(remote, isTrueTypeBuffer)) {
          await bratDownloadValidated(
            process.env.BRAT_FONT_URL || "https://raw.githubusercontent.com/Ditzzx-vibecoder/Assets/main/Font/ARIALN.ttf",
            remote,
            isTrueTypeBuffer,
          ).catch(() => {});
        }
        if (validCachedFile(remote, isTrueTypeBuffer)) candidates.push(remote);
      } catch {}
      if (validCachedFile(local, isTrueTypeBuffer)) candidates.push(local);

      for (const p of candidates) {
        try {
          if (GlobalFonts.registerFromPath(p, "ArialNarrow")) {
            bratFontFamily = "ArialNarrow";
            return bratFontFamily;
          }
        } catch {}
      }
      bratFontFamily = "sans-serif";
      return bratFontFamily;
    })().finally(() => { bratFontPromise = null; });
  }
  return bratFontPromise;
}

// Dipakai drawBrat (varian lama) + generateBrat: pastikan font nyata ke-register.
export const ensureBratFont = loadBratFont;

async function loadBratEmojiMap() {
  if (bratEmojiMap) return bratEmojiMap;
  if (!bratEmojiPromise) {
    bratEmojiPromise = (async () => {
      const dest = path.join(bratTmp(), "emoji-apple.json");
      try {
        if (!validCachedFile(dest, isJsonBuffer)) {
          await bratDownloadValidated(
            process.env.BRAT_EMOJI_URL || "https://media.githubusercontent.com/media/Ditzzx-vibecoder/entahlah/main/emoji-apple.json",
            dest,
            isJsonBuffer,
          );
        }
        bratEmojiMap = JSON.parse(readFileSync(dest, "utf-8"));
      } catch {
        bratEmojiMap = {}; // emoji map gagal -> emoji digambar via fillText, teks tetap jalan
      }
    })().finally(() => { bratEmojiPromise = null; });
  }
  return bratEmojiPromise;
}

function emojiToUnicode(emoji) {
  return [...emoji].map((c) => c.codePointAt(0).toString(16).padStart(4, "0")).join("-");
}

async function getBratEmojiImage(emoji) {
  if (bratEmojiCache.has(emoji)) return bratEmojiCache.get(emoji);
  const map = await loadBratEmojiMap();
  const base = emojiToUnicode(emoji);
  const noVs = base.replace(/-fe0f/gi, "");
  const variants = [base, noVs, `${noVs}-fe0f`, base.toUpperCase(), noVs.toUpperCase(), `${noVs.toUpperCase()}-FE0F`];
  let b64 = null;
  for (const v of variants) if (map[v]) { b64 = map[v]; break; }
  if (!b64) return null;
  const img = await loadImage(Buffer.from(b64, "base64"));
  if (bratEmojiCache.size >= BRAT_EMOJI_CACHE_MAX) bratEmojiCache.delete(bratEmojiCache.keys().next().value);
  bratEmojiCache.set(emoji, img);
  return img;
}

function bratMeasure(ctx, text, fontSize, family) {
  let w = 0;
  for (const part of text.split(EMOJI_SPLIT_RE)) {
    if (!part) continue;
    w += EMOJI_TEST_RE.test(part) ? fontSize : ctx.measureText(part).width;
  }
  return w;
}

function bratWrap(ctx, text, maxWidth, fontSize, family) {
  ctx.font = `${fontSize}px ${family}`;
  const lines = [];
  let cur = "";
  for (const word of text.split(" ")) {
    const test = cur ? `${cur} ${word}` : word;
    if (bratMeasure(ctx, test, fontSize, family) > maxWidth && cur) {
      lines.push(cur);
      cur = word;
    } else {
      cur = test;
    }
  }
  if (cur) lines.push(cur);
  return lines;
}

function bratFits(ctx, text, fontSize, maxWidth, maxHeight, lineGap, family) {
  const lines = bratWrap(ctx, text, maxWidth, fontSize, family);
  let longestWord = 0;
  for (const w of text.split(" ")) {
    const lw = bratMeasure(ctx, w, fontSize, family);
    if (lw > longestWord) longestWord = lw;
  }
  const totalHeight = lines.length * (fontSize + lineGap) - lineGap;
  return { ok: longestWord <= maxWidth && totalHeight <= maxHeight, lines };
}

function bratBestFontSize(ctx, text, maxWidth, maxHeight, lineGap, family) {
  let lo = 10, hi = 700, best = 10;
  let cached = null;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const r = bratFits(ctx, text, mid, maxWidth, maxHeight, lineGap, family);
    if (r.ok) { best = mid; cached = { size: mid, lines: r.lines }; lo = mid + 1; }
    else hi = mid - 1;
  }
  return cached || { size: best, lines: bratWrap(ctx, text, maxWidth, best, family) };
}

async function generateBratWithBratCanvas({ text = "Halo Guys Nama Saya", theme = "white", blur = 0, bgColor, textColor } = {}) {
  const t = BRAT_THEMES[theme] || BRAT_THEMES.white;
  const selectedTheme = { bg: bgColor || t.bg, text: textColor || t.text };
  const blurAmount = [1, 2, 3].includes(blur) ? blur : 0;
  
  const bgMap = { "#ffffff": "white", "#000000": "black", "#8ace00": "green" };
  const textMap = { "#ffffff": "white", "#000000": "black" };
  
  const { bratGen } = await import("brat-canvas");
  const buf = await bratGen({
    text,
    background: bgMap[selectedTheme.bg] || "white",
    color: textMap[selectedTheme.text] || "black",
  });
  
  if (blurAmount > 0) {
    const { default: sharp } = await import("sharp");
    return sharp(buf).blur(blurAmount).png().toBuffer();
  }
  return buf;
}

async function isBlankImage(buf) {
  if (!buf || buf.length < 100) return true;
  try {
    const { default: sharp } = await import("sharp");
    const { data } = await sharp(buf)
      .resize(50, 50, { fit: "fill" })
      .raw()
      .toBuffer({ resolveWithObject: true });
    const firstR = data[0], firstG = data[1], firstB = data[2], firstA = data[3];
    for (let i = 4; i < data.length; i += 4) {
      if (data[i] !== firstR || data[i+1] !== firstG || data[i+2] !== firstB || data[i+3] !== firstA) {
        return false;
      }
    }
    return true;
  } catch {
    return true; // bukan gambar valid → anggap blank/rusak
  }
}

export { isBlankImage }

export async function generateBrat({ text = "Halo Guys Nama Saya", theme = "white", blur = 0, bgColor, textColor } = {}) {
  const t = BRAT_THEMES[theme] || BRAT_THEMES.white;
  const selectedTheme = { bg: bgColor || t.bg, text: textColor || t.text };
  const blurAmount = [1, 2, 3].includes(blur) ? blur : 0;

  const size = 1000;
  const padding = 80;
  const lineGap = 20;
  const maxWidth = size - padding * 2;
  const maxHeight = size - padding * 2;

  const family = await loadBratFont();
  if (EMOJI_TEST_RE.test(text)) await loadBratEmojiMap();

  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext("2d");

  const { size: fontSize, lines } = bratBestFontSize(ctx, text, maxWidth, maxHeight, lineGap, family);

  ctx.fillStyle = selectedTheme.bg;
  ctx.fillRect(0, 0, size, size);
  ctx.fillStyle = selectedTheme.text;
  ctx.font = `${fontSize}px ${family}`;
  ctx.textAlign = "left";
  ctx.textBaseline = "top";

  const totalTextHeight = lines.length * (fontSize + lineGap) - lineGap;
  let y = (size - totalTextHeight) / 2;
  for (const line of lines) {
    let curX = padding;
    for (const part of line.split(EMOJI_SPLIT_RE)) {
      if (!part) continue;
      if (EMOJI_TEST_RE.test(part)) {
        const img = await getBratEmojiImage(part);
        if (img) ctx.drawImage(img, curX, y, fontSize, fontSize);
        else ctx.fillText(part, curX, y);
        curX += fontSize;
      } else {
        ctx.fillText(part, curX, y);
        curX += ctx.measureText(part).width;
      }
    }
    y += fontSize + lineGap;
  }

  let buf = await canvas.encode("png");
  
  if (await isBlankImage(buf)) {
    console.warn("[generateBrat] Native canvas produced blank image, falling back to brat-canvas");
    const fallbackBuf = await generateBratWithBratCanvas({ text, theme, blur, bgColor, textColor });
    if (await isBlankImage(fallbackBuf)) {
      throw new Error("brat renderer failed: native + brat-canvas both blank");
    }
    return fallbackBuf;
  }
  
  if (blurAmount > 0) {
    const { default: sharp } = await import("sharp");
    buf = await sharp(buf).blur(blurAmount).png().toBuffer();
  }
  return buf;
}

export async function fetchBratFromAPI(text, timeout = 15000) {
  const url = `https://api.theresav.eu/api/maker/brat?text=${encodeURIComponent(text)}`;
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), timeout);

  try {
    const res = await fetch(url, { signal: controller.signal });
    clearTimeout(id);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const arrayBuffer = await res.arrayBuffer();
    return Buffer.from(arrayBuffer);
  } catch (err) {
    clearTimeout(id);
    if (err.name === "AbortError") throw new Error("timeout");
    throw err;
  }
}

// ".bratimg halo -blur 2" -> { text: "halo", blur: 2 }. Flag tak dikenal dibiarkan sbg teks.
export function parseBratArgs(input = "") {
  let text = String(input).trim();
  let blur = 0;
  const m = text.match(/-blur\s*([0-3])/);
  if (m) { blur = Number(m[1]); text = text.replace(m[0], "").trim(); }
  return { text, blur };
}
