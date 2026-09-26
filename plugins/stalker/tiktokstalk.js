import te from '../../src/lib/error.js'
import { fetchTiktokProfile } from '../../src/lib/stalker-fallback.js'

const pluginConfig = {
    name: 'tiktokstalk',
    alias: ['ttstalk', 'stalktt'],
    category: 'stalker',
    description: 'Stalk akun TikTok',
    usage: '.tiktokstalk <username>',
    example: '.tiktokstalk mrbeast',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

function shortNum(num) {
    if (!num) return '0'
    num = parseInt(num)
    if (num >= 1_000_000_000) return (num / 1_000_000_000).toFixed(1).replace('.0', '') + 'B'
    if (num >= 1_000_000) return (num / 1_000_000).toFixed(1).replace('.0', '') + 'M'
    if (num >= 1_000) return (num / 1_000).toFixed(1).replace('.0', '') + 'K'
    return num.toString()
}

async function handler(m, { sock }) {
    const username = m.args[0]?.replace('@', '')
    
    if (!username) {
        return m.reply(`🎵 *ᴛɪᴋᴛᴏᴋ sᴛᴀʟᴋ*\n\n> Masukkan username TikTok\n\n\`Contoh: ${m.prefix}tiktokstalk mrbeast\``)
    }
    
    m.react('🔍')
    
    try {
        const { value: d } = await fetchTiktokProfile(username)

        const caption = `🎵 *ᴛɪᴋᴛᴏᴋ sᴛᴀʟᴋ*\n\n` +
            `👤 *Username:* @${d.username}\n` +
            `📛 *Nama:* ${d.name || '-'}\n` +
            `✅ *Verified:* ${d.verified ? 'Ya' : 'Tidak'}\n` +
            `🔒 *Private:* ${d.private ? 'Ya' : 'Tidak'}\n\n` +
            `👥 *Followers:* ${shortNum(d.followers)}\n` +
            `👤 *Following:* ${shortNum(d.following)}\n` +
            `❤️ *Likes:* ${shortNum(d.likes)}\n` +
            `🎬 *Videos:* ${shortNum(d.videos)}\n\n` +
            `📝 *Bio:*\n${d.bio || '-'}\n\n` +
            `🔗 ${d.link}`

        m.react('✅')

        if (d.avatar) {
            await sock.sendMessage(m.chat, {
                image: { url: d.avatar },
                caption
            }, { quoted: m })
        } else {
            await m.reply(caption)
        }

    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }