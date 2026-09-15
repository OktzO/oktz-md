import config from "../../config.js";
import te from "../../src/lib/ourin-error.js";
import fs from "fs";
import path from "path";
import os from "os";
import { spawn } from "child_process";
import { createCanvas, loadImage, GlobalFonts } from "@napi-rs/canvas";
import {
  ensureFfmpegOnPath,
  buildFfmpegCommand,
} from "../../src/lib/ourin-ffmpeg.js";

const FONT_URL =
  "https://cdn.jsdelivr.net/gh/Napoleon-Fibonacci/assets@main/font/impact.ttf";
const EMOJI_JSON_URL =
  "https://media.githubusercontent.com/media/Ditzzx-vibecoder/entahlah/main/emoji-apple.json";

const TMP_DIR = path.join(process.cwd(), "tmp");
const FONT_PATH = path.join(TMP_DIR, "impact.ttf");
const EMOJI_JSON_PATH = path.join(TMP_DIR, "emoji-apple.json");

const THEMES = {
  black: { bg: "#000000", text: "#ffffff" },
  white: { bg: "#ffffff", text: "#000000" },
  green: { bg: "#8ace00", text: "#000000" },
};

function ensureTmpDir() {
  if (!fs.existsSync(TMP_DIR)) {
    fs.mkdirSync(TMP_DIR, { recursive: true });
  }
}

async function downloadFile(url, dest) {
  ensureTmpDir();
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download gagal (${res.status})`);
  const buf = Buffer.from(await res.arrayBuffer());
  fs.writeFileSync(dest, buf);
  return buf;
}

async function ensureFont() {
  if (!fs.existsSync(FONT_PATH)) await downloadFile(FONT_URL, FONT_PATH);
  GlobalFonts.registerFromPath(FONT_PATH, "Impact");
}

let emojiMap = null;
const emojiImageCache = new Map();

function emojiToUnicode(emoji) {
  return [...emoji]
    .map((c) => c.codePointAt(0).toString(16).padStart(4, "0"))
    .join("-");
}

async function loadEmojiMap() {
  if (emojiMap) return emojiMap;
  if (!fs.existsSync(EMOJI_JSON_PATH))
    await downloadFile(EMOJI_JSON_URL, EMOJI_JSON_PATH);
  emojiMap = JSON.parse(fs.readFileSync(EMOJI_JSON_PATH, "utf-8"));
  return emojiMap;
}

async function getEmojiImage(emoji) {
  if (emojiImageCache.has(emoji)) return emojiImageCache.get(emoji);
  const map = await loadEmojiMap();
  const base = emojiToUnicode(emoji);
  const variants = [
    base,
    base.replace(/-fe0f/gi, ""),
    `${base.replace(/-fe0f/gi, "")}-fe0f`,
    base.toUpperCase(),
    base.replace(/-fe0f/gi, "").toUpperCase(),
    base.replace(/-fe0f/gi, "").toUpperCase() + "-FE0F",
  ];
  let b64 = null;
  for (const v of variants) {
    if (map[v]) {
      b64 = map[v];
      break;
    }
  }
  if (!b64) return null;
  const img = await loadImage(Buffer.from(b64, "base64"));
  emojiImageCache.set(emoji, img);
  return img;
}

async function drawAppleEmoji(ctx, emoji, x, y, size) {
  const img = await getEmojiImage(emoji);
  if (!img) {
    ctx.fillText(emoji, x, y);
    return;
  }
  ctx.drawImage(img, x, y, size, size);
}

const EMOJI_REGEX =
  /(\p{Emoji_Modifier_Base}\p{Emoji_Modifier}|\p{Emoji_Presentation}\uFE0F?|\p{Emoji}\uFE0F|[\u{1F1E0}-\u{1F1FF}]{2}|\p{Extended_Pictographic}\uFE0F?)/gu;

function measureTextCustom(ctx, text, fontSize) {
  const parts = text.split(EMOJI_REGEX);
  let w = 0;
  for (const part of parts) {
    if (!part) continue;
    EMOJI_REGEX.lastIndex = 0;
    if (EMOJI_REGEX.test(part)) w += fontSize;
    else w += ctx.measureText(part).width;
    EMOJI_REGEX.lastIndex = 0;
  }
  return w;
}

async function drawTextWithEmojis(ctx, text, x, y, fontSize) {
  const parts = text.split(EMOJI_REGEX);
  let curX = x;
  for (const part of parts) {
    if (!part) continue;
    EMOJI_REGEX.lastIndex = 0;
    if (EMOJI_REGEX.test(part)) {
      await drawAppleEmoji(ctx, part, curX, y, fontSize);
      curX += fontSize;
    } else {
      ctx.fillText(part, curX, y);
      ctx.measureText(part);
      curX += ctx.measureText(part).width;
    }
    EMOJI_REGEX.lastIndex = 0;
  }
}

function wrapText(ctx, text, maxWidth, fontSize) {
  ctx.font = `${fontSize}px Impact`;
  const words = text.split(" ");
  const lines = [];
  let cur = "";
  for (const word of words) {
    const test = cur ? cur + " " + word : word;
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

function fitsAt(ctx, text, fontSize, maxWidth, maxHeight, lineGap) {
  const lines = wrapText(ctx, text, maxWidth, fontSize);
  const words = text.split(" ").filter(Boolean);
  if (!words.length) return false;
  const longestWord = Math.max(
    ...words.map((w) => measureTextCustom(ctx, w, fontSize)),
  );
  const totalHeight = lines.length * (fontSize + lineGap) - lineGap;
  return longestWord <= maxWidth && totalHeight <= maxHeight;
}

function findBestFontSize(ctx, text, maxWidth, maxHeight, lineGap) {
  let lo = 10;
  let hi = 700;
  let best = lo;

  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2);
    if (fitsAt(ctx, text, mid, maxWidth, maxHeight, lineGap)) {
      best = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return best;
}

function easeOutBack(x) {
  const c1 = 1.4;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2);
}

function calculateWordLayout(
  ctx,
  fullText,
  maxWidth,
  maxHeight,
  lineGap,
  margin,
  padding,
  boxSize,
) {
  const fontSize = findBestFontSize(ctx, fullText, maxWidth, maxHeight, lineGap);
  ctx.font = `${fontSize}px Impact`;
  const defaultSpaceWidth = ctx.measureText(" ").width;

  const fullLines = wrapText(ctx, fullText, maxWidth, fontSize);
  const totalTextHeight = fullLines.length * (fontSize + lineGap) - lineGap;
  const startY = margin + (boxSize - totalTextHeight) / 2;

  const wordLayouts = [];
  let currentY = startY;

  for (let l = 0; l < fullLines.length; l++) {
    const line = fullLines[l];
    const lineWords = line.split(" ").filter(Boolean);
    const isLastLine = l === fullLines.length - 1;

    const totalWordsW = lineWords.reduce(
      (acc, w) => acc + measureTextCustom(ctx, w, fontSize),
      0,
    );

    let spaceBetween = defaultSpaceWidth;
    if (!isLastLine && lineWords.length > 1) {
      spaceBetween = (maxWidth - totalWordsW) / (lineWords.length - 1);
    }

    let currentX = margin + padding;

    for (const word of lineWords) {
      const wordW = measureTextCustom(ctx, word, fontSize);
      wordLayouts.push({
        text: word,
        x: currentX,
        y: currentY,
        w: wordW,
        h: fontSize,
      });
      currentX += wordW + spaceBetween;
    }
    currentY += fontSize + lineGap;
  }

  return { fontSize, wordLayouts };
}

async function renderCanvas({
  wordLayouts,
  fontSize,
  wordStates,
  theme,
  blurAmount,
  highlightProgress = 0,
  format = "mp4",
  margin = 70,
}) {
  const selectedTheme = THEMES[theme] || THEMES.white;
  const size = 1000;
  const boxSize = size - margin * 2;
  const x = margin;
  const y = margin;
  const w = boxSize;
  const h = boxSize;

  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext("2d");

  if (format !== "gif") {
    ctx.fillStyle = selectedTheme.bg;
    ctx.fillRect(0, 0, size, size);
  } else {
    ctx.clearRect(0, 0, size, size);
    ctx.fillStyle = selectedTheme.bg;
    ctx.fillRect(x, y, w, h);
  }

  if (!wordLayouts || wordLayouts.length === 0) return canvas;

  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();

  ctx.fillStyle = selectedTheme.text;
  ctx.font = `${fontSize}px Impact`;
  ctx.textAlign = "left";
  ctx.textBaseline = "top";

  if (blurAmount > 0) ctx.filter = `blur(${blurAmount}px)`;

  for (let idx = 0; idx < wordLayouts.length; idx++) {
    const item = wordLayouts[idx];
    const state = wordStates[idx] || { scale: 0, alpha: 0, visible: false };

    if (!state.visible) continue;

    const centerX = item.x + item.w / 2;
    const centerY = item.y + fontSize / 2;

    ctx.save();
    ctx.globalAlpha = Math.max(0, Math.min(1, state.alpha));

    if (state.scale !== 1.0) {
      ctx.translate(centerX, centerY);
      ctx.scale(state.scale, state.scale);
      ctx.translate(-centerX, -centerY);
    }

    await drawTextWithEmojis(ctx, item.text, item.x, item.y, fontSize);
    ctx.restore();
  }

  if (highlightProgress > 0 && highlightProgress <= 1) {
    const totalDist = boxSize * 2.8;
    const curr = margin - boxSize * 1.0 + highlightProgress * totalDist;
    const sweepW = boxSize * 0.95;

    const grad = ctx.createLinearGradient(curr, curr, curr + sweepW, curr + sweepW);

    grad.addColorStop(0.0, "rgba(255, 255, 255, 0)");
    grad.addColorStop(0.1, "rgba(255, 255, 255, 0.35)");
    grad.addColorStop(0.25, "rgba(255, 255, 255, 0.95)");
    grad.addColorStop(0.38, "rgba(255, 255, 255, 0.35)");

    grad.addColorStop(0.45, "rgba(255, 255, 255, 0.05)");
    grad.addColorStop(0.52, "rgba(255, 255, 255, 0.05)");

    grad.addColorStop(0.6, "rgba(255, 255, 255, 0.35)");
    grad.addColorStop(0.75, "rgba(255, 255, 255, 0.95)");
    grad.addColorStop(0.88, "rgba(255, 255, 255, 0.35)");
    grad.addColorStop(1.0, "rgba(255, 255, 255, 0)");

    ctx.fillStyle = grad;
    ctx.fillRect(margin, margin, boxSize, boxSize);
  }

  ctx.restore();

  return canvas;
}

async function generateBratVideo({
  text = "Halo Guys",
  theme = "white",
  blur = 0,
  format = "mp4",
  holdDuration = 1.5,
}) {
  const blurAmount = [0, 1, 2, 3].includes(blur) ? blur : 0;

  ensureTmpDir();
  await ensureFont();
  await loadEmojiMap();

  if (!text.trim()) throw new Error("Teks kosong");

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "brat-"));

  try {
    const FPS = 60;
    const frameStepTime = 1 / FPS;
    const tasks = [];

    const size = 1000;
    const margin = 70;
    const padding = 40;
    const boxSize = size - margin * 2;
    const lineGap = 15;
    const maxWidth = boxSize - padding * 2;
    const maxHeight = boxSize - padding * 2;

    const dummyCanvas = createCanvas(size, size);
    const dummyCtx = dummyCanvas.getContext("2d");

    const { fontSize, wordLayouts } = calculateWordLayout(
      dummyCtx,
      text,
      maxWidth,
      maxHeight,
      lineGap,
      margin,
      padding,
      boxSize,
    );

    const totalWords = wordLayouts.length;

    tasks.push({
      wordStates: wordLayouts.map(() => ({ scale: 0, alpha: 0, visible: false })),
      highlightProgress: 0,
      duration: 0.15,
    });

    const staggerFrames = 5;
    const bounceFramesCount = 28;
    const totalBounceFrames =
      (totalWords - 1) * staggerFrames + bounceFramesCount;

    for (let f = 0; f < totalBounceFrames; f++) {
      const wordStates = wordLayouts.map((_, i) => {
        const startFrame = i * staggerFrames;
        const currentFrame = f - startFrame;

        if (currentFrame < 0) {
          return { scale: 0, alpha: 0, visible: false };
        } else if (currentFrame >= bounceFramesCount) {
          return { scale: 1.0, alpha: 1.0, visible: true };
        } else {
          const prog = currentFrame / (bounceFramesCount - 1);
          const bounceFactor = easeOutBack(prog);
          const scale = 0.2 + (1.0 - 0.2) * bounceFactor;
          const alpha = Math.min(1.0, prog * 1.8);
          return { scale, alpha, visible: true };
        }
      });

      const highlightProgress = (f + 1) / totalBounceFrames;

      tasks.push({
        wordStates,
        highlightProgress,
        duration: frameStepTime,
      });
    }

    const secondHighlightFrames = 38;
    const allVisibleStates = wordLayouts.map(() => ({
      scale: 1.0,
      alpha: 1.0,
      visible: true,
    }));

    for (let hf = 0; hf < secondHighlightFrames; hf++) {
      const highlightProgress = (hf + 1) / secondHighlightFrames;
      tasks.push({
        wordStates: allVisibleStates,
        highlightProgress,
        duration: frameStepTime,
      });
    }

    tasks.push({
      wordStates: allVisibleStates,
      highlightProgress: 0,
      duration: holdDuration,
    });

    // Root cause CPU: 108x PNG encode lossless 1000x1000 (11.7s) lalu ffmpeg
    // dekode PNG-nya lagi. Draw aslinya cuma 0.2s. Solusi: alirkan pixel RGBA
    // langsung ke stdin ffmpeg (rawvideo) — tidak ada PNG sama sekali.
    // Durasi per task (0.15s intro, 1.5s hold) dikonversi jadi frame duplikat
    // karena rawvideo selalu CFR. Render tetap sekuensial (bukan Promise.all)
    // — heap bot 512MB, 100+ canvas 1000x1000 paralel bikin OOM.
    // ponytail: masih render 60fps di 1000x1000. Upgrade kalau masih berat:
    // FPS=30 (set stagger/bounce/secondHighlight ikut dibagi 2) atau
    // tambah "-vf scale=512:512" — hemat ~2s CPU lagi, kualitas sticker tetap
    // aman karena WhatsApp merender sticker <=512px.
    const outPath = path.join(tmpDir, "output.mp4");

    const { command, args } = buildFfmpegCommand([
      "-y",
      "-f",
      "rawvideo",
      "-pix_fmt",
      "rgba",
      "-s",
      `${size}x${size}`,
      "-r",
      String(FPS),
      "-i",
      "pipe:0",
      "-c:v",
      "libx264",
      "-preset",
      "veryfast",
      "-crf",
      "23",
      "-pix_fmt",
      "yuv420p",
      "-movflags",
      "+faststart",
      outPath,
    ]);

    const child = spawn(command, args, { stdio: ["pipe", "ignore", "pipe"] });
    let stderr = "";
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
      if (stderr.length > 8000) stderr = stderr.slice(-8000);
    });
    child.stdin.on("error", () => {});

    const timeout = setTimeout(() => child.kill("SIGKILL"), 120_000);
    const closed = new Promise((resolve, reject) => {
      child.on("error", reject);
      child.on("close", (code) =>
        code === 0
          ? resolve()
          : reject(
              new Error(
                `FFmpeg exit code ${code}: ${stderr.split("\n").pop()}`,
              ),
            ),
      );
    });

    let writeErr = null;
    const alive = () => child.exitCode === null && !child.stdin.destroyed;
    try {
      for (let i = 0; i < tasks.length && alive(); i++) {
        const task = tasks[i];
        const canvas = await renderCanvas({
          wordLayouts,
          fontSize,
          wordStates: task.wordStates,
          theme,
          blurAmount,
          highlightProgress: task.highlightProgress,
          format,
          margin,
        });
        const rgba = canvas.data();
        canvas.dispose?.();
        const repeats = Math.max(1, Math.round(task.duration * FPS));
        for (let r = 0; r < repeats && alive(); r++) {
          // Cek exitCode SEBELUM pasang listener: kalau child sudah mati,
          // event 'close' sudah lewat dan 'drain' tidak akan pernah datang.
          if (!child.stdin.write(rgba) && alive()) {
            await new Promise((done) => {
              child.stdin.once("drain", done);
              child.once("close", done);
            });
          }
        }
      }
    } catch (err) {
      writeErr = err;
    } finally {
      child.stdin.end();
    }

    // Selalu await closed: kalau tidak, promise-nya bisa reject tanpa
    // handler (unhandled rejection) saat render loop gagal.
    const closeErr = await closed.catch((err) => err);
    clearTimeout(timeout);
    if (writeErr) throw writeErr;
    if (closeErr) throw closeErr;

    return fs.readFileSync(outPath);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

const pluginConfig = {
  name: "bratvid2",
  alias: ["bratv2"],
  category: "sticker",
  description: "Generate brat video v2 (animasi per-kata + emoji Apple)",
  usage: ".bratvid2 <text>",
  example: ".bratvid2 hello world",
  isOwner: false,
  isPremium: false,
  isGroup: false,
  isPrivate: false,
  cooldown: 10,
  energi: 1,
  isEnabled: true,
};

async function handler(m, { sock }) {
  const text = m.args.join(" ");

  if (!text) {
    return m.reply(
      `🎬 *ʙʀᴀᴛ ᴠɪᴅᴇᴏ ᴠ2*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}bratvid2 hello world\``,
    );
  }

  await m.react("🕕");

  try {
    // Library internal + plugin lain spawn ffmpeg dari PATH — inject dulu.
    ensureFfmpegOnPath();

    const buffer = await generateBratVideo({
      text,
      theme: "white",
      format: "mp4",
    });

    const tempFile = path.join(os.tmpdir(), `brat2-${Date.now()}.mp4`);
    await fs.promises.writeFile(tempFile, buffer);
    try {
      await sock.sendVideoAsSticker(m.chat, tempFile, m, {
        packname: config.sticker.packname,
        author: config.sticker.author,
      });
    } finally {
      await fs.promises.unlink(tempFile).catch(() => {});
    }
    await m.react("✅");
  } catch (error) {
    console.error("[BratVid2 Error]", error);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler, generateBratVideo };
