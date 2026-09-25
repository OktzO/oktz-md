import { findParticipantByNumber } from '../../src/lib/lid.js'
import te from '../../src/lib/error.js'
const pluginConfig = {
    name: 'demote',
    alias: ['unadmin', 'turunkan'],
    category: 'group',
    description: 'Turunkan admin menjadi member biasa',
    usage: '.demote @user',
    example: '.demote @user',
    isOwner: false,
    isPremium: false,
    isGroup: true,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true,
    isAdmin: true,
    isBotAdmin: true
}

async function handler(m, { sock }) {
    let target = null

    if (m.quoted) {
        target = m.quoted.sender
    } else if (m.mentionedJid && m.mentionedJid.length > 0) {
        target = m.mentionedJid[0]
    }

    if (!target) {
        await m.reply(
            `❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n` +
            `> Reply pesan user atau mention!\n` +
            `> Contoh: \`${m.prefix}demote @user\``
        )
        return
    }

    try {
        const groupMeta = m.groupMetadata
        // findParticipantByNumber: di grup addressing_mode=lid ourin mengirim
        // { id: LID, phoneNumber: PN }, jadi perbandingan dengan target (PN hasil
        // resolve) tidak pernah ketemu -> "User tidak ditemukan".
        const participant = findParticipantByNumber(groupMeta.participants, target)

        if (!participant) {
            await m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di grup!`)
            return
        }

        if (!participant.admin) {
            await m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> User bukan admin!`)
            return
        }

        if (participant.admin === 'superadmin') {
            await m.reply(`❌ *ɢᴀɢᴀʟ*\n\n> Tidak bisa demote owner grup!`)
            return
        }

        // operasi member wajib memakai addressing asli server (p.id), bukan PN
        // hasil resolve - sama seperti kick.js.
        await sock.groupParticipantsUpdate(m.chat, [participant.id || target], 'demote')

        await m.reply(
            `@${target.split('@')[0]} sekarang bukan admin lagi.`,
            { mentions: [target] }
        )

    } catch (error) {
        m.reply(te(m.prefix, m.command, m.pushName))
    }
}

export { pluginConfig as config, handler }