import fs from "fs";
import path from "path";
import { logger } from "./ourin-logger.js";
import config from "../../config.js";

const assetCache = {};
const LARGE_MEDIA_EXTENSIONS = [".mp4", ".mp3", ".m4a", ".wav", ".avi", ".mkv", ".ogg"];
let _sharp = null;
async function _getSharp() {
  if (!_sharp) _sharp = (await import("sharp")).default;
  return _sharp;
}

function isLargeMedia(filepath) {
  if (!filepath || typeof filepath !== "string") return false;
  const lower = filepath.toLowerCase();
  return LARGE_MEDIA_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

async function _toJpegIfWebp(buf, filepath) {
  if (!filepath.toLowerCase().endsWith(".webp")) return buf;
  try {
    const sharp = await _getSharp();
    return await sharp(buf).jpeg({ quality: 80 }).toBuffer();
  } catch {
    return buf;
  }
}

export async function preloadAssets(configAssets) {
  if (!configAssets) return;
  for (const [key, filepath] of Object.entries(configAssets)) {
    try {
      if (typeof filepath === "string" && !filepath.startsWith("http")) {
        if (isLargeMedia(filepath)) continue;
        const fullPath = path.resolve(process.cwd(), filepath);
        if (fs.existsSync(fullPath)) {
          const raw = fs.readFileSync(fullPath);
          assetCache[key] = await _toJpegIfWebp(raw, filepath);
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

export async function updateAssetAndSave(key, buffer, filepath) {
  let cacheBuf = buffer;
  if (filepath && filepath.toLowerCase().endsWith(".webp")) {
    cacheBuf = await _toJpegIfWebp(buffer, filepath);
  }
  if (!isLargeMedia(filepath)) {
    assetCache[key] = cacheBuf;
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
