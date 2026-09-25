import * as timeHelper from '../../src/lib/time.js'
const pluginConfig = {
    name: 'absen',
    alias: ['hadir', 'present'],
    category: 'group',
    description: 'Tandai kehadiran di sesi absen',
    usage: '.absen',
    example: '.absen',
    isOwner: false,
    isPremium: false,
    isGroup: true,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true
}
if (!global.absensi) global.absensi = {}
const ABSENSI_MAX_CHATS = 500
const ABSENSI_TTL_MS = 24 * 60 * 60 * 1000

function pruneAbsensi() {
    const chats = Object.keys(global.absensi)
    if (chats.length <= ABSENSI_MAX_CHATS) return
    const cutoff = Date.now() - ABSENSI_TTL_MS
    for (const chatId of chats) {
        const entry = global.absensi[chatId]
        let created = (entry && (entry.createdAt || entry.lastUpdate || entry.lastActive)) || 0
        if (typeof created === 'string') created = new Date(created).getTime() || 0
        if (typeof created === 'number' && created && created < cutoff) delete global.absensi[chatId]
    }
}
async function handler(m, { sock }) {
    pruneAbsensi()
    const chatId = m.chat
    if (!global.absensi[chatId]) {
        return m.reply(
            `❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ᴀʙsᴇɴ*\n\n` +
            `> Belum ada sesi absen di grup ini!\n\n` +
            `> Admin dapat memulai dengan\n` +
            `> *.mulaiabsen [keterangan]*`
        )
    }
    const absen = global.absensi[chatId]
    if (absen.peserta.includes(m.sender)) {
        return m.reply(`❌ Kamu sudah absen!`)
    }
    absen.peserta.push(m.sender)
    const dateStr = timeHelper.formatNow('D MMMM YYYY')
    const list = absen.peserta
        .map((jid, i) => `┃ ${i + 1}. @${jid.split('@')[0]}`)
        .join('\n')
    await m.reply(`✅ *MANTAP, @${m.sender.split('@')[0]} HADIRR*\n` +
            `TUJUAN ABSEN: ${absen.keterangan}\n` +
            `╭┈┈⬡「 📋 INFO LAIN 」\n` +
            `┃ 📅 ${dateStr}\n` +
            `┃ 👥 Total: ${absen.peserta.length}\n` +
            `├┈┈⬡「 📝 *ᴅᴀғᴛᴀʀ ʜᴀᴅɪʀ* 」\n` +
            `${list}\n` +
            `╰┈┈┈┈┈┈┈┈⬡\n\n` +
            `> _Ketik *${m.prefix}absen* untuk hadir_\n` +
            `> _Ketik *${m.prefix}cekabsen* untuk melihat daftar_`,
            { mentions: absen.peserta })
}
export { pluginConfig as config, handler, pruneAbsensi }