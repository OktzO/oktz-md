import config from '../../config.js'
import { f } from './../../src/lib/http.js'
import te from '../../src/lib/error.js'
const pluginConfig = {
    name: 'text2img3',
    alias: [],
    category: 'ai',
    description: 'Generate gambar dari teks dengan AI',
    usage: '.txt2img <prompt> | <style>',
    example: '.txt2img beautiful sunset | anime',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 30,
    energi: 1,
    isEnabled: true
}

const STYLES = ['photorealistic', 'digital-art', 'impressionist', 'anime', 'fantasy', 'sci-fi', 'vintage']

async function handler(m, { sock }) {
    const input = m.args.join(' ')
    if (!input) {
        return m.reply(
            `🎨 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*\n\n` +
            `> Generate gambar dari teks dengan AI\n\n` +
            `\`Contoh: ${m.prefix}txt2img beautiful sunset | anime\`\n\n` +
            `🎭 *sᴛʏʟᴇs*\n` +
            `> \`${STYLES.join(', ')}\``
        )
    }

    const [prompt] = input.split('|').map(s => s.trim())

    m.react('🕕')

    try {
        const { data } = await f(`https://api.neoxr.eu/api/stablediff?prompt=${encodeURIComponent(prompt)}&model=default&orientation=potrait&apikey=${config.APIkey.neoxr}`)

        if (!data?.url) throw new Error('Hasil kosong dari provider')

        await sock.sendMedia(m.chat, data?.url, null, m, {
            type: 'image'
        })

    } catch (error) {
        m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }