import te from '../../src/lib/error.js'
import { updateAssetUrl } from '../../src/lib/uploader.js'
const pluginConfig = {
    name: 'ganti-demote.jpg',
    alias: ['gantidemote', 'setdemote'],
    category: 'owner',
    description: 'Ganti gambar demote.jpg',
    usage: '.ganti-demote.jpg (reply/kirim gambar)',
    example: '.ganti-demote.jpg',
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
    if (!isImage) return m.reply(`🖼️ *ɢᴀɴᴛɪ ᴅᴇᴍᴏᴛᴇ.ᴊᴘɢ*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/demote.jpeg`)
    try {
        let buffer = m.quoted && m.quoted.isMedia ? await m.quoted.download() : await m.download()
        if (!buffer) return m.reply('❌ Gagal mendownload gambar')
        await m.reply(`⏳ Sedang mengupload gambar...`)
        try {
            const newUrl = await updateAssetUrl('demote', buffer, 'demote.jpeg')
            m.reply(`✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Gambar demote.jpeg telah diganti ke URL baru:\n> ${newUrl}\n> Config telah diupdate secara realtime!`)
        } catch (e) {
            m.reply(`❌ Gagal mengupload gambar: ${e.message}`)
        }
    } catch (error) {
        await m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }