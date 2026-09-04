import { addExifToWebp } from '../../src/lib/ourin-exif.js'
import { createCanvas } from '@napi-rs/canvas'
import config from '../../config.js'
import { f } from '../../src/lib/ourin-http.js'
import te from '../../src/lib/ourin-error.js'
const NEOXR_APIKEY = config.APIkey?.neoxr || 'Milik-Bot-OurinMD'
const pluginConfig = {
    name: 'attp',
    alias: ['attp2', 'attp3'],
    category: 'sticker',
    description: 'Membuat sticker animated text',
    usage: '.attp <teks>',
    example: '.attp Hello World',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

async function localAnimatedText(text) {
    const frames = 8
    const colors = ['#FF5733', '#C70039', '#900C3F', '#581845', '#2E86AB', '#A23B72', '#F18F01', '#C73E1D']
    const canvas = createCanvas(512, 512)
    const ctx = canvas.getContext('2d')
    const buffers = []

    for (let i = 0; i < frames; i++) {
        ctx.clearRect(0, 0, 512, 512)
        ctx.fillStyle = '#000000'
        ctx.fillRect(0, 0, 512, 512)

        let fontSize = 50
        ctx.font = `bold ${fontSize}px sans-serif`
        let words = text.split(' ')
        let lines = []
        let currentLine = ''

        for (const word of words) {
            const test = currentLine ? currentLine + ' ' + word : word
            if (ctx.measureText(test).width > 460) {
                if (currentLine) lines.push(currentLine)
                currentLine = word
            } else {
                currentLine = test
            }
        }
        if (currentLine) lines.push(currentLine)

        const offsetY = Math.sin(i / frames * Math.PI * 2) * 10
        const lineHeight = fontSize * 1.3
        const totalH = lines.length * lineHeight
        let startY = (512 - totalH) / 2 + fontSize + offsetY

        ctx.textAlign = 'center'
        ctx.textBaseline = 'alphabetic'

        for (const line of lines) {
            ctx.font = `bold ${fontSize}px sans-serif`
            ctx.fillStyle = colors[i % colors.length]
            ctx.fillText(line, 256, startY)
            startY += lineHeight
        }

        buffers.push(await canvas.encode('png'))
    }

    return buffers
}

async function handler(m, { sock }) {
    let text = m.text?.trim()
    if (!text && m.quoted?.text) {
        text = m.quoted.text.trim()
    }
    if (!text) {
        return m.reply(
            `🎨 *ᴀɴɪᴍᴀᴛᴇᴅ ᴛᴇxᴛ sᴛɪᴄᴋᴇʀ*\n\n` +
            `> Masukkan teks untuk sticker\n\n` +
            `> Contoh: \`${m.prefix}attp Hello World\``
        )
    }
    if (text.length > 100) {
        return m.reply(`❌ Teks terlalu panjang! Maksimal 100 karakter.`)
    }
    m.react('🕕')
    try {
        const color = 'FF5733'
        const url = `https://api.neoxr.eu/api/attp3?text=${encodeURIComponent(text)}&color=${color}&apikey=${NEOXR_APIKEY}`
        const data = await f(url)
        if (data?.status && data?.data?.url) {
            const stickerUrl = data.data.url
            const stickerRes = await f(stickerUrl, 'buffer')
            if (stickerRes) {
                let finalSticker = stickerRes
                try {
                    finalSticker = await addExifToWebp(stickerRes, {
                        packname: config.sticker.packname,
                        author: config.sticker.author
                    })
                } catch (e) {
                    console.log('Exif error:', e)
                }
                await sock.sendMessage(m.chat, { sticker: finalSticker }, { quoted: m })
                m.react('✅')
                return
            }
        }
        throw new Error('API gagal, fallback ke local')
    } catch (err) {
        try {
            const frames = await localAnimatedText(text)
            const sent = []
            for (const buf of frames) {
                try {
                    const sticker = await addExifToWebp(buf, {
                        packname: config.sticker.packname,
                        author: config.sticker.author
                    })
                    await sock.sendMessage(m.chat, { sticker }, { quoted: m })
                    sent.push(true)
                    await new Promise(r => setTimeout(r, 300))
                } catch (e) { }
            }
            if (sent.length > 0) {
                m.react('✅')
                return
            }
        } catch (e2) { }
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}
export { pluginConfig as config, handler }