import { drawBrat } from '../../src/lib/ourin-brat.js'
import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'

const pluginConfig = {
    name: 'bratcewek',
    alias: ['cewekbrat', 'bratperempuan', 'bratgirl'],
    category: 'sticker',
    description: 'Membuat sticker brat',
    usage: '.bratcewek <text>',
    example: '.bratcewek Hai semua',
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
        return m.reply(`🖼️ *ʙʀᴀᴛ cᴇᴡᴇᴋ sᴛɪᴄᴋᴇʀ*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}bratcewek Hai semua\``)
    }
    
    m.react('🕕')
    
    try {
        const buffer = await drawBrat({
            text,
            bgColor: "#FFC0CB",
            width: 512,
            height: 512,
            maxWidth: 450,
            maxHeight: 450,
            centerX: 256,
            centerY: 256,
            maxFontSize: 130,
            fontDecrement: 5,
            lineHeightMult: 1.1,
            textColor: "#000000"
        })
        await sock.sendImageAsSticker(m.chat, buffer, m, {
            packname: config.sticker.packname,
            author: config.sticker.author
        })
        m.react('✅')
    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }