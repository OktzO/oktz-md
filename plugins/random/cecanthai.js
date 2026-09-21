import te from '../../src/lib/ourin-error.js'
const pluginConfig = {
    name: 'cecanthai',
    alias: ['cewekthai', 'cewekthailand'],
    category: 'cecan',
    description: 'Random gambar cewek cantik Thailand',
    usage: '.cecanthai',
    example: '.cecanthai',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 1,
    isEnabled: true
}

async function handler(m, { sock }) {
    const api = 'https://api.nexray.web.id/random/cecan/thailand'
    await m.react('🇹🇭')
    try {
        await sock.sendMedia(m.chat, api, null, m, {
            type: 'image'
        })
        await m.react('✅')
    } catch (e) {
        await m.react('☢')
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }