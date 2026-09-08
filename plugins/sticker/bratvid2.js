import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'
import fs from 'fs'
import path from 'path'
import os from 'os'
import { ensureFfmpegOnPath } from '../../src/lib/ourin-ffmpeg.js'

// RAM: brat-canvas/video mem-parse ~80MB emoji JSON saat import (+200MB RSS).
// Lazy-load hanya saat command dijalankan.
let _bratVid = null
async function getBratVid() {
    if (!_bratVid) {
        _bratVid = (await import('brat-canvas/video')).bratVid
    }
    return _bratVid
}

const pluginConfig = {
    name: 'bratvid2',
    alias: ['bratv2'],
    category: 'sticker',
    description: 'Generate brat video v2',
    usage: '.bratvid2 <text>',
    example: '.bratvid2 hello world',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const text = m.args.join(' ')
    
    if (!text) {
        return m.reply(`🎬 *ʙʀᴀᴛ ᴠɪᴅᴇᴏ ᴠ2*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}bratvid2 hello world\``)
    }
    
    m.react('🕕')

    try {
        // brat-canvas menjalankan `spawn("ffmpeg")` dari PATH.
        // Tanpa ini, ffmpeg tidak ketemu => ENOENT => command gagal diam-diam.
        ensureFfmpegOnPath()

        const tempFile = path.join(os.tmpdir(), `brat2-${Date.now()}.webp`)
        const buffer = await (await getBratVid())(text, {
            outputFormat: 'mp4',
        })
        await fs.promises.writeFile(tempFile, buffer)
        await sock.sendVideoAsSticker(m.chat, tempFile, m, {
            packname: config.sticker.packname,
            author: config.sticker.author
        })
        await fs.promises.unlink(tempFile)
        m.react('✅')
    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }