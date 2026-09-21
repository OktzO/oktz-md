import { f } from '../../src/lib/ourin-http.js'
import te from '../../src/lib/ourin-error.js'
import config from '../../config.js'
const pluginConfig = {
    name: 'gita',
    alias: ['gitagpt', 'bhagavadgita'],
    category: 'ai',
    description: 'Chat dengan Gita GPT (Bhagavad Gita AI)',
    usage: '.gita <pertanyaan>',
    example: '.gita What is dharma?',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const text = m.args.join(' ')
    if (!text) {
        return m.reply(`📿 *ɢɪᴛᴀ ɢᴘᴛ*\n\n> Masukkan pertanyaan\n\n\`Contoh: ${m.prefix}gita What is dharma?\``)
    }

    m.react('🕕')

    if (!config.APIkey.cuki) {
        return m.reply(`❌ *API Key belum diset!*\n\n> Owner harus set \`APIKEY_CUKI\` di \`.env\``)
    }

    try {
        const url = `https://api.cuki.biz.id/api/ai/gita?apikey=${config.APIkey.cuki}&q=${encodeURIComponent(text)}`
        const data = await f(url)

        const content = data?.results
        if (!content) throw new Error('Respons Gita kosong')

        m.react('✅')
        await m.reply(`${content?.trim()}`)

    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }