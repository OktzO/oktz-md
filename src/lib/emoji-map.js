import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from "fs";
import path from "path";

// emoji-apple.json (~27MB) parse SEKALI per proses. Dulu brat.js dan
// bratvid2.js masing-masing readFileSync+JSON.parse -> dua objek 27MB hidup
// bareng di module singleton. Shared module ini nyimpan satu instans; semua
// consumer (ourin-brat, bratvid2) dapat referensi sama. Download divalidasi +
// self-heal (port dari ourin-brat); gagal -> fallback {} (emoji digambar via
// fillText, teks tetap jalan).
// ponytail: plugins/canvas/iqcpink.js masih download+parse sendiri (3rd copy)
// — pindahkan ke loader ini kalau plugin itu mau di-opt.

const EMOJI_JSON_URL =
  process.env.BRAT_EMOJI_URL ||
  "https://media.githubusercontent.com/media/Ditzzx-vibecoder/entahlah/main/emoji-apple.json";

function emojiTmp() {
  const d = path.join(process.cwd(), "tmp");
  if (!existsSync(d)) mkdirSync(d, { recursive: true });
  return d;
}

function emojiJsonPath() {
  return path.join(emojiTmp(), "emoji-apple.json");
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

async function downloadValidated(url, dest, check) {
  const res = await fetch(url, { signal: AbortSignal.timeout(30000) });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  if (!check(buf)) throw new Error("payload invalid (bukan JSON — kena intercept proxy?)");
  writeFileSync(dest, buf);
  return buf;
}

let emojiMap = null;
let emojiMapPromise = null;
let emojiMapParseCount = 0;

export async function loadEmojiMap() {
  if (emojiMap) return emojiMap;
  if (!emojiMapPromise) {
    emojiMapPromise = (async () => {
      try {
        const dest = emojiJsonPath();
        if (!validCachedFile(dest, isJsonBuffer)) {
          await downloadValidated(EMOJI_JSON_URL, dest, isJsonBuffer);
        }
        emojiMap = JSON.parse(readFileSync(dest, "utf-8"));
        emojiMapParseCount += 1;
      } catch {
        emojiMap = {};
      }
    })().finally(() => {
      emojiMapPromise = null;
    });
  }
  return emojiMapPromise.then(() => emojiMap);
}

export { emojiMapParseCount };