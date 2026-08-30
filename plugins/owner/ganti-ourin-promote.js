import fs from 'fs'
import path from 'path'
import te from '../../src/lib/ourin-error.js'
import { updateAssetUrl } from '../../src/lib/ourin-uploader.js'
const pluginConfig = {
    name: 'ganti-ourin-promote.jpg',
    alias: ['gantiourinpromote', 'setourinpromote'],
    category: 'owner',
    description: 'Ganti gambar ourin-promote.jpg',
    usage: '.ganti-ourin-promote.jpg (reply/kirim gambar)',
    example: '.ganti-ourin-promote.jpg',
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
    if (!isImage) return m.reply(`🖼️ *ɢᴀɴᴛɪ OURIN-PROMOTE.JPG*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/ourin-promote.webp`)
    try {
        let buffer = m.quoted && m.quoted.isMedia ? await m.quoted.download() : await m.download()
        if (!buffer) return m.reply('❌ Gagal mendownload gambar')
        await m.reply(`⏳ Sedang mengupload gambar...`)
        try {
            const newUrl = await updateAssetUrl('ourin-promote', buffer, 'ourin-promote.webp')
            m.reply(`✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Gambar ourin-promote.webp telah diganti ke URL baru:\n> ${newUrl}\n> Config telah diupdate secara realtime!`)
        } catch (e) {
            m.reply(`❌ Gagal mengupload gambar: ${e.message}`)
        }
    } catch (error) {
        await m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }