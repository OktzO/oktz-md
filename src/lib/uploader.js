import { createUploadProviders } from './upload-providers.js'

const PROVIDER_TIMEOUT_MS = 20000

// 9 host keyless, semua dicek hidup 2026-09-27. Sequential early-return, bukan
// fan-out: .tourl memang butuh semua host (user pilih), tapi upload internal
// cuma butuh SATU URL. Fan-out di sini bikin 9x bandwidth terbuang dan user
// nunggu host paling lambat (~20 detik) walau host pertama sudah jadi di 1
// detik. Kalau satu host mati, pindah saja ke berikutnya.
//
// Alias lama (uploadToTelegraph/uploadTo0x0/uploadToCatbox/uploadToTmpfiles/
// uploadToUguu) DIHAPUS: tidak pernah di-import, dan semuanya menunjuk ke satu
// fungsi yang sama jadi hanya berpura-pura punya redundancy.
export async function uploadImage(buffer, filename = 'image.jpg', opts = {}) {
  const providers = opts.providers ?? createUploadProviders();
  if (!providers.length) {
    throw new Error('Upload gagal: tidak ada host upload yang dikonfigurasi');
  }

  const perProviderMs = opts.perProviderMs ?? PROVIDER_TIMEOUT_MS;
  const errors = [];

  for (const provider of providers) {
    try {
      const result = await withTimeout(
        provider.run(buffer, filename),
        perProviderMs,
      );
      const url = result?.url;
      if (typeof url !== 'string' || !url) {
        throw new Error('respons tanpa URL');
      }
      return url;
    } catch (e) {
      errors.push(`${provider.name}: ${e.message}`);
    }
  }

  throw new Error(`Upload gagal di semua host — ${errors.join(' | ')}`);
}

function withTimeout(promise, ms) {
  let timer;
  return Promise.race([
    Promise.resolve(promise),
    new Promise((_, reject) => {
      timer = setTimeout(
        () => reject(new Error(`timeout ${ms}ms`)),
        ms,
      );
    }),
  ]).finally(() => clearTimeout(timer));
}

import fs from 'fs';
import path from 'path';
import { ImageUploadService } from 'node-upload-images';
import config from '../../config.js';

import { updateAssetAndSave } from './asset-manager.js';

export async function updateAssetUrl(assetKey, buffer, filename = 'image.jpg') {
  let localPath = config.assets?.[assetKey];

  if (!localPath || localPath.startsWith('http')) {
    let folder = 'image';
    if (filename.endsWith('.mp4')) folder = 'video';
    else if (filename.endsWith('.mp3')) folder = 'audio';

    localPath = `./assets/${folder}/${filename}`;

    if (!config.assets) config.assets = {};
    config.assets[assetKey] = localPath;

    const configPath = path.join(process.cwd(), 'config.js');
    let configContent = fs.readFileSync(configPath, 'utf8');

    const regex = new RegExp(`("${assetKey}"\\s*:\\s*)"([^"]+)"`);
    if (regex.test(configContent)) {
      configContent = configContent.replace(regex, `$1"${localPath}"`);
    } else {
      const assetsBlockRegex = /(assets\s*:\s*\{)([^}]*)(\})/;
      if (assetsBlockRegex.test(configContent)) {
        configContent = configContent.replace(assetsBlockRegex, (match, p1, p2, p3) => {
          let inner = p2.trim();
          if (inner.endsWith(',')) inner = inner.slice(0, -1);
          if (inner.length > 0) return `${p1}\n    ${inner},\n    "${assetKey}": "${localPath}"\n  ${p3}`;
          return `${p1}\n    "${assetKey}": "${localPath}"\n  ${p3}`;
        });
      }
    }
    fs.writeFileSync(configPath, configContent, 'utf8');
  }

  const fullPath = path.resolve(process.cwd(), localPath);
  const dir = path.dirname(fullPath);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

  let saveBuffer = buffer;
  if (localPath.toLowerCase().endsWith(".jpeg") || localPath.toLowerCase().endsWith(".jpg")) {
    try {
      const sharp = (await import("sharp")).default;
      saveBuffer = await sharp(buffer)
        .resize(1280, 720, { fit: "inside", withoutEnlargement: false })
        .jpeg({ quality: 80 })
        .toBuffer();
    } catch (e) {
      console.error(`Failed to convert ${assetKey} to jpeg:`, e.message);
    }
  }

  await updateAssetAndSave(assetKey, saveBuffer, localPath);

  return localPath;
}