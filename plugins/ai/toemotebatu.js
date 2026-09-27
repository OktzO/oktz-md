import { uploadImage } from '../../src/lib/uploader.js'
import { applyImageEffect } from '../../src/lib/effect-api.js'
import te from '../../src/lib/error.js'
const pluginConfig = {
    name: 'toemotebatu',
    alias: ['emotebatu', 'moai', 'tomoai'],
    category: 'ai',
    description: 'Ubah gambar ke emote batu 🗿',
    usage: '.toemotebatu (reply gambar)',
    example: '.toemotebatu',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 30,
    energi: 2,
    isEnabled: true
}

async function handler(m, { sock }) {
    const isImage = m.isImage || (m.quoted && m.quoted.type === 'imageMessage')
    
    if (!isImage) {
        return m.reply(`🗿 *ᴇᴍᴏᴛᴇ ʙᴀᴛᴜ*\n\n> Kirim/reply gambar\n\n\`${m.prefix}toemotebatu\``)
    }
    
    m.react('🕕')
    
    try {
        let buffer
        if (m.quoted && m.quoted.isMedia) {
            buffer = await m.quoted.download()
        } else if (m.isMedia) {
            buffer = await m.download()
        }
        
        if (!buffer) {
            m.react('❌')
            return m.reply(`❌ Gagal mendownload gambar`)
        }
        
        const imageUrl = await uploadImage(buffer, 'image.jpg')
        
        const res = await applyImageEffect('tomoai', imageUrl)
        
        m.react('✅')
        
        await sock.sendMedia(m.chat, res, null, m, {
            type: 'image'
        })
        
    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }