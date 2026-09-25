import te from '../../src/lib/error.js'
import { updateAssetUrl } from '../../src/lib/uploader.js'
const pluginConfig = {
    name: 'ganti-ourin-rpg.jpg',
    alias: ['gantirpg', 'setourinrpg'],
    category: 'owner',
    description: 'Ganti gambar rpg.jpg (thumbnail rpg)',
    usage: '.ganti-ourin-rpg.jpg (reply/kirim gambar)',
    example: '.ganti-ourin-rpg.jpg',
    isOwner: true,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

async function handler(m, { sock }) {
    const isImage = m.isImage || (m.quoted && m.quoted.type === 'imageMessage')
    
    if (!isImage) {
        return m.reply(`🖼️ *ɢᴀɴᴛɪ ᴏᴜʀɪɴ-ʀᴘɢ.ᴊᴘɢ*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/rpg.jpeg`)
    }
    
    try {
        let buffer
        if (m.quoted && m.quoted.isMedia) {
            buffer = await m.quoted.download()
        } else if (m.isMedia) {
            buffer = await m.download()
        }
        
        if (!buffer) {
            return m.reply(`❌ Gagal mendownload gambar`)
        }
        
        await m.reply(`⏳ Sedang mengupload gambar...`)
        try {
            const newUrl = await updateAssetUrl('rpg', buffer, 'rpg.jpeg')
            m.reply(`✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Gambar rpg.jpeg telah diganti ke URL baru:\n> ${newUrl}\n> Config telah diupdate secara realtime!`)
        } catch (e) {
            m.reply(`❌ Gagal mengupload gambar: ${e.message}`)
        }
    } catch (error) {
        await m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }