import axios from 'axios'
import config from '../../config.js'
import te from '../../src/lib/ourin-error.js'
const pluginConfig = {
    name: 'lahelu',
    alias: ['randommeme'],
    category: 'random',
    description: 'Random gambar lahelu',
    usage: '.lahelu',
    example: '.lahelu',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    if (!config.APIkey?.cuki) {
        return m.reply(`❌ *API Key belum diset!*\n\n> Isi \`APIKEY_CUKI\` di file \`.env\`, lalu restart bot`)
    }
    const api = `https://api.cuki.biz.id/api/random/lahelu?apikey=${encodeURIComponent(config.APIkey.cuki)}`
    await m.react('🕕')
    
    try {
        const res = (await axios.get(api)).data
        const random = res.data[Math.floor(Math.random() * res.data.length)]
        if(random.media.includes('.mp4')) {
            await sock.sendMedia(m.chat, random.media, random.title, m, {
                type: 'video'
            })
        } else {
            await sock.sendMedia(m.chat, random.media, random.title, m, {
                type: 'image'
            })
        }
        await m.react('✅')
    } catch (e) {
        await m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }