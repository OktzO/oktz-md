import fs from "fs";
import path from "path";
import sharp from "sharp";
import { logger } from "./logger.js";
import config from "../../config.js";

const assetCache = {};
const LARGE_MEDIA_EXTENSIONS = [".mp4", ".mp3", ".m4a", ".wav", ".avi", ".mkv", ".ogg"];

function isLargeMedia(filepath) {
  if (!filepath || typeof filepath !== "string") return false;
  const lower = filepath.toLowerCase();
  return LARGE_MEDIA_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

export function preloadAssets(configAssets) {
  if (!configAssets) return;
  for (const [key, filepath] of Object.entries(configAssets)) {
    try {
      if (typeof filepath === "string" && !filepath.startsWith("http")) {
        if (isLargeMedia(filepath)) continue;
        const fullPath = path.resolve(process.cwd(), filepath);
        if (fs.existsSync(fullPath)) {
          assetCache[key] = fs.readFileSync(fullPath);
          logger.system("CACHE", `Loaded: ${key}`);
        } else {
          logger.warn("CACHE", `File not found: ${fullPath}`);
        }
      }
    } catch (e) {
      logger.error("CACHE", `Failed to load ${key}: ${e.message}`);
    }
  }
}

export function getAssetBuffer(key, configAssets = null) {
  if (assetCache[key]) {
    return assetCache[key];
  }

  const assets = configAssets || config?.assets;
  if (assets && assets[key] && !assets[key].startsWith("http")) {
    try {
      const fullPath = path.resolve(process.cwd(), assets[key]);
      if (fs.existsSync(fullPath)) {
        const buf = fs.readFileSync(fullPath);
        if (!isLargeMedia(assets[key])) {
          assetCache[key] = buf;
        }
        return buf;
      }
    } catch (e) {
      console.error(`Failed to read ${key} from disk:`, e.message);
    }
  }

  return null;
}

/**
 * Resize asset jadi thumbnail, aman terhadap input yang tidak bisa
 * diproses sharp.
 *
 * Kenapa helper ini perlu: `getAssetBuffer()` kontraknya boleh `null`
 * (file hilang / gagal read). Tapi `sharp(null)` melempar error
 * SYNCHRONOUS — sehingga pola
 *
 *   await sharp(getAssetBuffer("foto")).resize(300, 300).toBuffer()
 *     .catch(() => null)
 *
 * tidak menangkap apa pun: `.catch()` menempel ke promise hasil
 * `toBuffer()`, sementara `sharp()` sudah lempar sebelum promise itu
 * ada. Akibatnya satu variant menu lempar dan seluruh `.menu` hilang.
 *
 * @param {Buffer|string|null|undefined} input
 * @param {number} size sisi thumbnail (piksel)
 * @returns {Promise<Buffer|null>} null kalau tidak bisa diproses
 */
export async function safeThumbnail(input, size = 300) {
  if (!input) return null;

  let pipeline;
  try {
    // sharp() melempar synchronus kalau input tidak valid — bungkus di
    // dalam try, bukan pakai .catch() di rantai promise.
    pipeline = sharp(input).resize(size, size);
  } catch {
    return null;
  }

  try {
    const buf = await pipeline.toBuffer();
    return buf && buf.length > 0 ? buf : null;
  } catch {
    return null;
  }
}

export function updateAssetAndSave(key, buffer, filepath) {
  if (!isLargeMedia(filepath)) {
    assetCache[key] = buffer;
  }
  if (filepath && !filepath.startsWith("http")) {
    try {
      const fullPath = path.resolve(process.cwd(), filepath);
      fs.writeFileSync(fullPath, buffer);
    } catch (e) {
      console.error(`Failed to write updated asset ${key} to disk:`, e.message);
    }
  }
}
