/**
 * Credits & Thanks to
 * Developer = Lucky Archz ( Zann )
 * Lead owner = HyuuSATAN
 * Owner = Keisya
 * Designer = Danzzz
 * Wileys = Penyedia baileys
 * Penyedia APIs
 * Penyedia Scrapers
 * 
 * JANGAN HAPUS/GANTI CREDITS & THANKS TO
 * JANGAN DIJUAL YA MEK
 * 
 * Saluran Resmi Foto:
 * https://whatsapp.com/channel/0029VbB37bgBfxoAmAlsgE0t 
 * 
 */
import fs from 'fs'
import path from 'path'
import Crypto from 'crypto'
import ffmpegInstaller from '@ffmpeg-installer/ffmpeg'
import ffmpeg from 'fluent-ffmpeg'
ffmpeg.setFfmpegPath(ffmpegInstaller.path);

function getTempDir() {
    const tmpDir = path.join(process.cwd(), 'tmp');
    if (!fs.existsSync(tmpDir)) {
        fs.mkdirSync(tmpDir, { recursive: true });
    }
    return tmpDir;
}

const DEFAULT_METADATA = {
    packname: 'Sticker',
    author: 'Bot',
    packId: 'com.foto.sticker',
    emojis: ['🤖']
};

function createExif(options = {}) {
    const packname = options.packname ?? DEFAULT_METADATA.packname;
    const author = options.author ?? DEFAULT_METADATA.author;
    const packId = options.packId ?? DEFAULT_METADATA.packId;
    const emojis = options.emojis ?? DEFAULT_METADATA.emojis;
    
    const json = {
        'sticker-pack-id': packId,
        'sticker-pack-name': packname,
        'sticker-pack-publisher': author,
        'emojis': emojis,
        'is-avatar-sticker': 0,
        'android-app-store-link': '',
        'ios-app-store-link': ''
    };
    
    let exifAttr = Buffer.from([
        0x49, 0x49, 0x2A, 0x00,
        0x08, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x41, 0x57,
        0x07, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x16, 0x00,
        0x00, 0x00
    ]);
    
    let jsonBuffer = Buffer.from(JSON.stringify(json), 'utf8');
    let exif = Buffer.concat([exifAttr, jsonBuffer]);
    
    exif.writeUIntLE(jsonBuffer.length, 14, 4);
    
    return exif;
}

function imageToWebpFFmpeg(buffer) {
    return new Promise((resolve, reject) => {
        const tmpDir = getTempDir();
        const inputPath = path.join(tmpDir, `img_${Date.now()}_${Crypto.randomBytes(4).toString('hex')}.png`);
        const outputPath = path.join(tmpDir, `sticker_${Date.now()}_${Crypto.randomBytes(4).toString('hex')}.webp`);
        
        fs.writeFileSync(inputPath, buffer);
        
        ffmpeg(inputPath)
            .outputOptions([
                '-vcodec', 'libwebp',
                '-vf', "scale=512:512:force_original_aspect_ratio=decrease,pad=512:512:(ow-iw)/2:(oh-ih)/2:color=0x00000000,setsar=1",
                '-loop', '0',
                '-preset', 'default',
                '-an',
                '-vsync', '0',
                '-quality', '80'
            ])
            .toFormat('webp')
            .on('end', () => {
                try {
                    const webpBuffer = fs.readFileSync(outputPath);
                    if (fs.existsSync(inputPath)) fs.unlinkSync(inputPath);
                    if (fs.existsSync(outputPath)) fs.unlinkSync(outputPath);
                    resolve(webpBuffer);
                } catch (err) {
                    reject(err);
                }
            })
            .on('error', (err) => {
                if (fs.existsSync(inputPath)) fs.unlinkSync(inputPath);
                if (fs.existsSync(outputPath)) fs.unlinkSync(outputPath);
                reject(err);
            })
            .save(outputPath);
    });
}

function videoToWebpFFmpeg(buffer, options = {}) {
    return new Promise((resolve, reject) => {
        const tmpDir = getTempDir();
        const isGif = buffer.slice(0, 4).toString("hex") === "47494638";
        const ext = isGif ? "gif" : "mp4";
        const inputPath = path.join(tmpDir, `vid_${Date.now()}_${Crypto.randomBytes(4).toString('hex')}.${ext}`);
        const outputPath = path.join(tmpDir, `animated_${Date.now()}_${Crypto.randomBytes(4).toString('hex')}.webp`);
        
        fs.writeFileSync(inputPath, buffer);
        
        const duration = options.duration || 5;
        const fps = options.fps || 15;
        
        ffmpeg(inputPath)
            .inputOptions(['-y'])
            .outputOptions([
                '-vcodec', 'libwebp',
                '-vf', `fps=${fps},scale=512:512:force_original_aspect_ratio=decrease,pad=512:512:(ow-iw)/2:(oh-ih)/2:color=0x00000000,setsar=1`,
                '-loop', '0',
                '-ss', '0',
                '-t', String(duration),
                '-preset', 'default',
                '-an',
                '-vsync', '0'
            ])
            .toFormat('webp')
            .on('end', () => {
                try {
                    const webpBuffer = fs.readFileSync(outputPath);
                    if (fs.existsSync(inputPath)) fs.unlinkSync(inputPath);
                    if (fs.existsSync(outputPath)) fs.unlinkSync(outputPath);
                    resolve(webpBuffer);
                } catch (err) {
                    reject(err);
                }
            })
            .on('error', (err) => {
                if (fs.existsSync(inputPath)) fs.unlinkSync(inputPath);
                if (fs.existsSync(outputPath)) fs.unlinkSync(outputPath);
                reject(err);
            })
            .save(outputPath);
    });
}

// ponytail: node-webpmux dropped. Pure-Buffer RIFF/EXIF writer below is byte-identical
// (verified static + animated WebP, same chunk layout & size). Ceiling: it appends/replaces
// the EXIF chunk only; if a future need requires re-encoding pixels or rewriting other chunks,
// reintroduce a real muxer (node-webpmux or a Rust/native one).
async function addExifToWebp(webpBuffer, options = {}) {
    return addExifToWebpFallback(webpBuffer, options);
}

async function createStickerFromImage(imageBuffer, options = {}) {
    const isWebp = imageBuffer.slice(0, 4).toString("hex") === "52494646";
    const webpBuffer = isWebp ? imageBuffer : await imageToWebpFFmpeg(imageBuffer);
    return await addExifToWebp(webpBuffer, options);
}

async function createStickerFromVideo(videoBuffer, options = {}) {
    const isWebp = videoBuffer.slice(0, 4).toString("hex") === "52494646";
    let webpBuffer;
    
    if (isWebp) {
        webpBuffer = videoBuffer;
    } else {
        webpBuffer = await videoToWebpFFmpeg(videoBuffer, {
            duration: options.duration || 5,
            fps: options.fps || 15
        });
    }
    return await addExifToWebp(webpBuffer, {
        packname: options.packname,
        author: options.author,
        emojis: options.emojis
    });
}

function addExifToWebpFallback(webpBuffer, options = {}) {
    const exif = createExif(options);
    const hasExif = webpBuffer.indexOf(Buffer.from('EXIF')) !== -1;

    const extended = ensureExtendedContainer(webpBuffer);
    
    if (hasExif) {
        return replaceExifInWebp(extended, exif);
    }
    return appendExifToWebp(extended, exif);
}

// WhatsApp menolak webp kontainer basic (VP8/VP8L) yang diberi chunk metadata:
// per spec WebP, EXIF/ICCP/XMP butuh file extended (wajib ada VP8X). Sharp
// menghasilkan static basic, sedangkan ffmpeg-anim (bratvid2) menghasilkan
// extended — itulah beda bratimg (blank) vs bratvid2 (normal) di WhatsApp.
function ensureExtendedContainer(webpBuffer) {
    if (!isValidWebp(webpBuffer)) return webpBuffer;

    // buang slack trailing (sharp kasih byte sisa di luar chunk terakhir;
    // wa menolak "garbage bytes at end", github.com/WhatsApp/stickers#606)
    let end = 12;
    const riffSize = webpBuffer.readUInt32LE(4);
    const limit = Math.min(12 + riffSize, webpBuffer.length);
    while (end + 8 <= limit) {
        const size = webpBuffer.readUInt32LE(end + 4);
        if (end + 8 + size > limit) break;
        end += 8 + size + (size % 2);
    }
    let clean = webpBuffer;
    if (end < webpBuffer.length) {
        clean = webpBuffer.slice(0, end);
        clean.writeUInt32LE(end - 8, 4);
    }

    if (clean.indexOf(Buffer.from('VP8X')) !== -1) return clean;

    const dims = getWebpDimensions(clean);
    if (!dims) return clean;

    const hasAlpha = clean.indexOf(Buffer.from('ALPH')) !== -1;
    const hasAnim = clean.indexOf(Buffer.from('ANIM')) !== -1 || clean.indexOf(Buffer.from('ANMF')) !== -1;

    // flags VP8X: bit1=alpha, bit2=EXIF, bit4=animation
    const flags = 0x04 | (hasAlpha ? 0x02 : 0) | (hasAnim ? 0x10 : 0);
    const canvasW = Buffer.alloc(3);
    canvasW.writeUIntLE(Math.max(1, dims.width - 1), 0, 3);
    const canvasH = Buffer.alloc(3);
    canvasH.writeUIntLE(Math.max(1, dims.height - 1), 0, 3);

    const vp8xPayload = Buffer.concat([Buffer.from([flags]), Buffer.alloc(3), canvasW, canvasH]);
    const vp8xSize = Buffer.alloc(4);
    vp8xSize.writeUInt32LE(vp8xPayload.length);
    const vp8xChunk = Buffer.concat([Buffer.from('VP8X'), vp8xSize, vp8xPayload]);

    // VP8X wajib diposisikan tepat setelah signature 'WEBP'.
    return Buffer.concat([clean.slice(0, 12), vp8xChunk, clean.slice(12)]);
}

function replaceExifInWebp(webpBuffer, newExif) {
    const exifIndex = webpBuffer.indexOf(Buffer.from('EXIF'));
    
    if (exifIndex === -1 || exifIndex + 8 > webpBuffer.length) {
        return appendExifToWebp(webpBuffer, newExif);
    }
    
    const oldExifSize = webpBuffer.readUInt32LE(exifIndex + 4);
    
    if (exifIndex + 8 + oldExifSize > webpBuffer.length) {
        return appendExifToWebp(webpBuffer, newExif);
    }
    const padding = oldExifSize % 2 === 1 ? 1 : 0;
    
    const beforeExif = webpBuffer.slice(0, exifIndex);
    const afterExif = webpBuffer.slice(exifIndex + 8 + oldExifSize + padding);
    
    const exifChunkId = Buffer.from('EXIF');
    const exifSize = Buffer.alloc(4);
    exifSize.writeUInt32LE(newExif.length);
    
    const newPadding = newExif.length % 2 === 1 ? Buffer.from([0x00]) : Buffer.alloc(0);
    
    const newWebp = Buffer.concat([beforeExif, exifChunkId, exifSize, newExif, newPadding, afterExif]);
    
    const newFileSize = newWebp.length - 8;
    newWebp.writeUInt32LE(newFileSize, 4);
    
    return newWebp;
}

function appendExifToWebp(webpBuffer, exif) {
    const riffHeader = webpBuffer.slice(0, 4);
    const webpSignature = webpBuffer.slice(8, 12);
    const webpData = webpBuffer.slice(12);
    const exifChunkId = Buffer.from('EXIF');
    const exifSize = Buffer.alloc(4);
    exifSize.writeUInt32LE(exif.length);
    
    const exifChunk = Buffer.concat([exifChunkId, exifSize, exif]);
    const padding = exif.length % 2 === 1 ? Buffer.from([0x00]) : Buffer.alloc(0);
    const newWebpData = Buffer.concat([webpData, exifChunk, padding]);
    const newFileSize = Buffer.alloc(4);
    newFileSize.writeUInt32LE(4 + newWebpData.length);
    
    return Buffer.concat([riffHeader, newFileSize, webpSignature, newWebpData]);
}

function readExifFromWebp(webpBuffer) {
    const exifIndex = webpBuffer.indexOf(Buffer.from('EXIF'));
    
    if (exifIndex === -1) {
        return null;
    }
    
    const exifSize = webpBuffer.readUInt32LE(exifIndex + 4);
    const exifData = webpBuffer.slice(exifIndex + 8, exifIndex + 8 + exifSize);
    
    try {
        const jsonStart = 22;
        const jsonData = exifData.slice(jsonStart);
        return JSON.parse(jsonData.toString('utf8'));
    } catch {
        return null;
    }
}

function isValidWebp(buffer) {
    if (!Buffer.isBuffer(buffer) || buffer.length < 12) {
        return false;
    }
    
    const riff = buffer.slice(0, 4).toString('ascii');
    const webp = buffer.slice(8, 12).toString('ascii');
    
    return riff === 'RIFF' && webp === 'WEBP';
}

async function isAnimatedWebp(buffer) {
    if (!isValidWebp(buffer)) {
        return false;
    }
    
    return buffer.indexOf(Buffer.from('ANIM')) !== -1 || 
           buffer.indexOf(Buffer.from('ANMF')) !== -1;
}

function getWebpDimensions(buffer) {
    if (!isValidWebp(buffer)) {
        return null;
    }
    
    try {
        const vp8Index = buffer.indexOf(Buffer.from('VP8 '));
        if (vp8Index !== -1) {
            const width = buffer.readUInt16LE(vp8Index + 14) & 0x3FFF;
            const height = buffer.readUInt16LE(vp8Index + 16) & 0x3FFF;
            return { width, height };
        }
        const vp8lIndex = buffer.indexOf(Buffer.from('VP8L'));
        if (vp8lIndex !== -1) {
            const bits = buffer.readUInt32LE(vp8lIndex + 9);
            const width = (bits & 0x3FFF) + 1;
            const height = ((bits >> 14) & 0x3FFF) + 1;
            return { width, height };
        }
        const vp8xIndex = buffer.indexOf(Buffer.from('VP8X'));
        if (vp8xIndex !== -1) {
            const width = (buffer.readUIntLE(vp8xIndex + 12, 3) + 1);
            const height = (buffer.readUIntLE(vp8xIndex + 15, 3) + 1);
            return { width, height };
        }
        
        return null;
    } catch {
        return null;
    }
}

function generateStickerId() {
    return `sticker_${Date.now()}_${Crypto.randomBytes(4).toString('hex')}`;
}

function cleanTempFiles(maxAge = 3600000) {
    const tmpDir = path.join(process.cwd(), 'tmp');
    
    if (!fs.existsSync(tmpDir)) {
        return;
    }
    
    const now = Date.now();
    const files = fs.readdirSync(tmpDir);
    
    for (const file of files) {
        if (file.startsWith('exif_') || file.startsWith('input_') || 
            file.startsWith('output_') || file.startsWith('img_') || 
            file.startsWith('vid_') || file.startsWith('sticker_') ||
            file.startsWith('animated_')) {
            const filePath = path.join(tmpDir, file);
            try {
                const stat = fs.statSync(filePath);
                if (now - stat.mtimeMs > maxAge) {
                    fs.unlinkSync(filePath);
                }
            } catch {
            }
        }
    }
}

const PRESETS = {
    default: {
        packname: 'Sticker',
        author: 'Bot',
        emojis: ['🤖']
    },
    meme: {
        packname: 'Meme Pack',
        author: 'Bot',
        emojis: ['😂', '🤣']
    },
    love: {
        packname: 'Love Pack',
        author: 'Bot',
        emojis: ['❤️', '💕', '💖']
    },
    sad: {
        packname: 'Sad Pack',
        author: 'Bot',
        emojis: ['😢', '😭', '💔']
    },
    angry: {
        packname: 'Angry Pack',
        author: 'Bot',
        emojis: ['😠', '😡', '💢']
    }
};

export {
    createExif, addExifToWebp, addExifToWebpFallback, replaceExifInWebp, readExifFromWebp,
    imageToWebpFFmpeg, videoToWebpFFmpeg,
    createStickerFromImage, createStickerFromVideo,
    isValidWebp, isAnimatedWebp, getWebpDimensions,
    generateStickerId, cleanTempFiles, getTempDir,
    ffmpeg,
    DEFAULT_METADATA, PRESETS
}