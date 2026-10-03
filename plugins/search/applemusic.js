import { resolver } from '../../src/lib/resolve.js'
import te from '../../src/lib/error.js'
const pluginConfig = {
    name: 'applemusic',
    alias: ['amusic', 'am'],
    category: 'search',
    description: 'Cari lagu di Apple Music',
    usage: '.applemusic <query>',
    example: '.applemusic Best Friend',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}

async function handler(m, { sock }) {
    const query = m.text?.trim()

    if (!query) {
        return m.reply(
            `⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n` +
            `> \`${m.prefix}applemusic <query>\`\n\n` +
            `> Contoh:\n` +
            `> \`${m.prefix}applemusic Best Friend\``
        )
    }

    try {
        // Backend lokal (ytmusic-api) dicoba lebih dulu dan agregator menjadi
        // cadangan; `resolve()` yang memutuskan, bukan plugin ini.
        // Daftar kosong itu jawaban yang sah dari host, jadi penentuannya tetap
        // di sini seperti sebelumnya.
        const { data } = await resolver.resolve('ytmusic', { q: query })
        const tracks = data?.tracks ?? []

        if (!tracks.length) {
            return m.reply(`❌ Tidak ditemukan hasil untuk: ${query}`)
        }

        let txt = `🍎 *ᴀᴘᴘʟᴇ ᴍᴜsɪᴄ sᴇᴀʀᴄʜ*\n\n`
        txt += `> Query: *${query}*\n\n`

        tracks.slice(0, 5).forEach((t, i) => {
            txt += `*${i + 1}.* \`\`\`${t.title}\`\`\`\n`
            txt += `   ├ 📀 \`${t.artist || 'Unknown'}\`\n`
            txt += `   └ 🔗 \`${t.url}\`\n\n`
        })

        return m.reply(txt.trim())

    } catch (err) {
        return m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }