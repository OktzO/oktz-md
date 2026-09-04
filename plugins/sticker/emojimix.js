import config from '../../config.js'
import { f } from './../../src/lib/ourin-http.js'
import te from '../../src/lib/ourin-error.js'
import { createCanvas } from '@napi-rs/canvas'

const pluginConfig = {
    name: 'emojimix',
    alias: ['mixemoji', 'emix'],
    category: 'sticker',
    description: 'Gabungkan 2 emoji menjadi 1',
    usage: '.emojimix <emoji1><emoji2>',
    example: '.emojimix 😂🔥',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const text = m.text?.trim()

    if (!text) {
        return m.reply(
            `🎭 *ᴇᴍᴏᴊɪ ᴍɪx*\n\n` +
            `> Gabungkan 2 emoji menjadi 1\n\n` +
            `> Contoh: \`${m.prefix}emojimix 😂🔥\``
        )
    }

    const emojiRegex = /\p{Extended_Pictographic}/gu
    const emojis = text.match(emojiRegex)

    if (!emojis || emojis.length < 2) {
        return m.reply(`❌ Masukkan minimal 2 emoji!\n\nContoh: ${m.prefix}emojimix 😂🔥`)
    }

    const emoji1 = emojis[0]
    const emoji2 = emojis[1]

    m.react('🕕')

    try {
        const apiUrl = `https://api.neoxr.eu/api/emoji?q=${encodeURIComponent(emoji1 + '_' + emoji2)}&apikey=${config.APIkey.neoxr}`

        const data = await f(apiUrl)

        if (data?.status && data?.data?.url) {
            const imageUrl = data.data.url
            await sock.sendImageAsSticker(m.chat, imageUrl, m, {
                packname: config.sticker.packname,
                author: config.sticker.author
            })
            m.react('✅')
            return
        }

        const canvas = createCanvas(256, 256)
        const ctx = canvas.getContext('2d')

        ctx.fillStyle = '#FFFFFF'
        ctx.fillRect(0, 0, 256, 256)

        ctx.textAlign = 'center'
        ctx.textBaseline = 'middle'
        ctx.font = '80px sans-serif'
        ctx.fillText(emoji1, 90, 128)
        ctx.fillText(emoji2, 166, 128)

        const buffer = await canvas.encode('png')

        await sock.sendImageAsSticker(m.chat, buffer, m, {
            packname: config.sticker.packname,
            author: config.sticker.author
        })

        m.react('✅')

    } catch (err) {
        console.log(err)
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }