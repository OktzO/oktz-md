import { sessionKeyStats } from '../../src/lib/turso-session.js'

const pluginConfig = {
    name: 'sessionstats',
    alias: ['sessionstat', 'statkeys'],
    category: 'owner',
    description: 'Menampilkan ukuran session_keys per kategori',
    usage: '.sessionstats',
    example: '.sessionstats',
    isOwner: true,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 30,
    energi: 0,
    isEnabled: true
}

// Bytes asli harus ikut tampil. Kalau ini jadi bacaan lokal saja, angka
// Turso yang monitore jadi tidak bisa dipercaya justru saat yang paling
// perlu dipercaya.
function fmtSize(bytes) {
    if (!bytes) return '0 B'
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

async function handler(m) {
    await m.react('📊')

    let stats
    try {
        stats = await sessionKeyStats()
    } catch (error) {
        await m.reply(`❌ Gagal membaca statistik session: ${error.message}`)
        return
    }

    if (!stats.byCategory.length) {
        const reason = stats.error
            ? `\n> _${stats.error}_`
            : '\n> _Turso tidak aktif atau belum ada key._'
        await m.reply(
            `╭┈┈⬡「 📊 *sᴇssɪᴏɴ sᴛᴀᴛs* 」\n┃\n┃ ɴᴏ ᴋᴇʏ ᴛᴇʀsɪᴍᴘᴀɴ${reason}\n╰┈┈⬡`
        )
        return
    }

    const lines = stats.byCategory.map((c) => {
        const marker = c.category === 'pre-key' ? ' ⤵️' : ''
        return `┃ ▸ \`${c.category}\` — ${c.keys} key · ${fmtSize(c.bytes)}${marker}`
    })

    const preKeys = stats.byCategory.find((c) => c.category === 'pre-key')
    const preLine = preKeys
        ? `┃\n┃ ⤵️ *pre-key* = ${preKeys.keys} key / ${fmtSize(preKeys.bytes)}\n┃ _sweep tiap 3 hari, hapus yang >30 hari_`
        : ''

    await m.reply(
        `╭┈┈⬡「 📊 *sᴇssɪᴏɴ sᴛᴀᴛs* 」\n┃\n` +
        lines.join('\n') +
        `\n┃\n┃ *total* — ${stats.total} key · ${fmtSize(stats.bytes)}` +
        preLine +
        `\n╰┈┈⬡`
    )
}

export { pluginConfig as config, handler }