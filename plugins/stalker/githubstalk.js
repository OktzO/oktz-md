import te from '../../src/lib/error.js'
import { fetchGithubProfile } from '../../src/lib/stalker-fallback.js'

const pluginConfig = {
    name: 'githubstalk',
    alias: ['ghstalk', 'stalkgh'],
    category: 'stalker',
    description: 'Stalk akun GitHub',
    usage: '.githubstalk <username>',
    example: '.githubstalk torvalds',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 10,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const username = m.args[0]
    
    if (!username) {
        return m.reply(`🐙 *ɢɪᴛʜᴜʙ sᴛᴀʟᴋ*\n\n> Masukkan username GitHub\n\n\`Contoh: ${m.prefix}githubstalk torvalds\``)
    }
    
    m.react('🔍')
    
    try {
        const { value: d } = await fetchGithubProfile(username)

        const caption = `🐙 *ɢɪᴛʜᴜʙ sᴛᴀʟᴋ*\n\n` +
            `👤 *Username:* ${d.username}\n` +
            `📛 *Nama:* ${d.name || '-'}\n` +
            `🏢 *Company:* ${d.company || '-'}\n` +
            `📍 *Location:* ${d.location || '-'}\n\n` +
            `📦 *Public Repos:* ${d.publicRepos ?? 0}\n` +
            `👥 *Followers:* ${d.followers ?? 0}\n` +
            `👤 *Following:* ${d.following ?? 0}\n\n` +
            `📝 *Bio:*\n${d.bio || '-'}\n\n` +
            `🔗 ${d.url}`

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