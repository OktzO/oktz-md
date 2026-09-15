import { generateBrat, parseBratArgs } from '../../src/lib/ourin-brat.js'
import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'

const pluginConfig = {
    name: 'bratanime',
    alias: ['animebrat'],
    category: 'sticker',
    description: 'Membuat sticker brat',
    usage: '.animebrat <text>',
    example: '.animebrat Hai semua',
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
        return m.reply(`🖼️ *ʙʀᴀᴛ ᴀɴɪᴍᴇ sᴛɪᴄᴋᴇʀ*\n\n> Masukkan teks\n\n\`Contoh: ${m.prefix}animebrat Hai semua\``)
    }
    
    m.react('🕕')
    
    try {
        const { text: parsed, blur } = parseBratArgs(text)
        const buffer = await generateBrat({ text: parsed, theme: 'white', bgColor: '#FF69B4', blur })
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